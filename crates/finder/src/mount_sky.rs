//! A drawn sky on the real mount (026): catalog stars at the pointing the encoders give.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use serde::Serialize;

use crate::{ephemeris, link::FrameFilter, model::Model, session::Session};
use pattern_database::PatternDatabase;
use scope_link::proto;

/// The finder's horizontal field, as measured on the sky on 2026-11-09.
pub const FOV_DEG: f64 = 13.0;
/// The azimuth the zero counts read as.
pub const ZERO_AZ_DEG: f64 = 180.0;
/// The most centroids a node frame carries.
pub const MAX_CENTROIDS: usize = 200;

/// The session's view of the drawn sky (FR-006).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MountSkyView {
    /// The tube's altitude when the console started, from a level.
    pub tube_alt_deg: f64,
    /// Counts `[azimuth, altitude]` that read as `ZERO_AZ_DEG` and `tube_alt_deg`;
    /// `null` until the first usable frame.
    pub zero_counts: Option<[i32; 2]>,
    /// True once the node's reset count changed after the zero was fixed (FR-014).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub power_cycled: bool,
}

/// Draws each node frame's stars from the mount's encoders. `zero` holds the zero
/// counts and the reset count the session had seen when they were fixed;
/// `power_cycled` is never cleared.
pub struct MountSky {
    db: Arc<PatternDatabase>,
    session: Arc<Session>,
    tube_alt_deg: f64,
    /// The fixed zero counts and the reset count seen when they were fixed.
    zero: Mutex<Option<([i32; 2], u64)>>,
    /// Latched when the node's reset count changes after the zero was fixed.
    power_cycled: AtomicBool,
}

impl MountSky {
    pub fn new(db: Arc<PatternDatabase>, session: Arc<Session>, tube_alt_deg: f64) -> Self {
        Self {
            db,
            session,
            tube_alt_deg,
            zero: Mutex::new(None),
            power_cycled: AtomicBool::new(false),
        }
    }

    pub fn view(&self) -> MountSkyView {
        MountSkyView {
            tube_alt_deg: self.tube_alt_deg,
            zero_counts: self
                .zero
                .lock()
                .expect("mount-sky zero")
                .map(|(fixed, _)| fixed),
            power_cycled: self.power_cycled.load(Ordering::SeqCst),
        }
    }
}

impl FrameFilter for MountSky {
    fn filter(&self, mut frame: proto::StarFrame, offset_s: Option<f64>) -> proto::StarFrame {
        let unusable = |frame: &mut proto::StarFrame| {
            frame.centroids.clear();
            frame.preview_jpeg = None;
        };
        // 1. Stopped for good once the node's reset count changed (FR-014).
        if self.power_cycled.load(Ordering::SeqCst) {
            unusable(&mut frame);
            return frame;
        }
        // 2. Counts at both ends of the exposure.
        let Some((start, end)) = frame.axis_start.as_ref().zip(frame.axis_end.as_ref()) else {
            unusable(&mut frame);
            return frame;
        };
        let mid = [
            (f64::from(start.azimuth) + f64::from(end.azimuth)) / 2.0,
            (f64::from(start.altitude) + f64::from(end.altitude)) / 2.0,
        ];
        // 3. Terms the pointing needs: a site, counts per revolution, a reset count
        //    the session has seen, a clock offset and a renderable frame.
        let (site, counts, resets) = self.session.mount_sky_terms();
        let (Some(site), Some(counts), Some(resets), Some(offset_s)) =
            (site, counts, resets, offset_s)
        else {
            unusable(&mut frame);
            return frame;
        };
        if counts.contains(&0)
            || frame.width == 0
            || frame.height == 0
            || !frame.exposure_ms.is_finite()
            || frame.exposure_ms <= 0.0
            || !frame.gain.is_finite()
            || frame.gain <= 0.0
        {
            unusable(&mut frame);
            return frame;
        }
        // 5. Zero counts, fixed by the first usable frame; a changed reset count
        //    stops the drawing for good.
        let seen = self
            .zero
            .lock()
            .expect("mount-sky zero")
            .map(|(fixed, seen)| (fixed, seen));
        let zero = match seen {
            Some((_fixed, seen)) if seen != resets => {
                self.power_cycled.store(true, Ordering::SeqCst);
                unusable(&mut frame);
                return frame;
            }
            Some((fixed, _)) => fixed,
            None => {
                let fixed = [mid[0].round() as i32, mid[1].round() as i32];
                *self.zero.lock().expect("mount-sky zero") = Some((fixed, resets));
                fixed
            }
        };
        // 6. The pointing the mid-exposure counts give: with no tilt and no fixed
        //    terms this is the spec's truth formula, in the model's own convention.
        let mut model = Model::default();
        model.load_known_terms(
            0.0,
            0.0,
            [
                ZERO_AZ_DEG - f64::from(zero[0]) * 360.0 / f64::from(counts[0]),
                self.tube_alt_deg - f64::from(zero[1]) * 360.0 / f64::from(counts[1]),
            ],
        );
        let Some(pointing) = model.predict(mid, counts) else {
            unusable(&mut frame);
            return frame;
        };
        // 7-8. The camera's attitude on the sky at mid-exposure console time.
        let rotation = simulator::sky::camera_rotation(
            ephemeris::true_from_apparent(pointing.alt_deg),
            pointing.az_deg,
            simulator::sky::Site {
                lat_deg: site.lat_deg,
                lon_deg: site.lon_deg,
            },
            (frame.start_s + frame.end_s) / 2.0 - offset_s,
        );
        let optics = simulator::optics::Optics {
            width: frame.width,
            height: frame.height,
            fov_deg: FOV_DEG,
            eyepiece_px: (f64::from(frame.width) / 2.0, f64::from(frame.height) / 2.0),
        };
        // 9. Brightest-first catalog stars, at most 200, and no preview.
        let mut stars: Vec<_> = simulator::render::field_stars(
            &self.db,
            optics,
            rotation,
            frame.exposure_ms,
            frame.gain,
        )
        .into_iter()
        .map(|(_, star)| proto::Centroid {
            x: star.x,
            y: star.y,
            brightness: 1e6 * 10f64.powf(-0.4 * star.magnitude),
        })
        .collect();
        stars.sort_by(|a, b| b.brightness.total_cmp(&a.brightness));
        stars.truncate(MAX_CENTROIDS);
        frame.centroids = stars;
        frame.preview_jpeg = None;
        frame
    }
}
