//! Known finder optics and shared telescope state. Public angles are degrees.
use crate::{
    clock::VirtualClock,
    render::{self, RenderStar},
    sky::{self, Site},
};
use math_core::{PinholeCamera, UnitVector};
use pattern_database::PatternDatabase;
use std::sync::{Arc, Mutex};

/// Full-frame pixels use (x, y); the horizontal field is in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Optics {
    pub width: u32,
    pub height: u32,
    pub fov_deg: f64,
    pub eyepiece_px: (f64, f64),
}

impl Default for Optics {
    fn default() -> Self {
        Self {
            width: 1456,
            height: 1088,
            fov_deg: 12.0,
            eyepiece_px: (700.0, 420.0),
        }
    }
}

impl Optics {
    pub fn pinhole(self) -> PinholeCamera {
        assert!(self.width > 0 && self.height > 0);
        assert!(self.fov_deg > 0.0 && self.fov_deg < 180.0);
        PinholeCamera::new(
            self.width.into(),
            self.height.into(),
            self.fov_deg.to_radians(),
        )
    }

    fn eyepiece_ray(self) -> UnitVector {
        let (x, y) = self.eyepiece_px;
        assert!(x >= 0.0 && x < f64::from(self.width) && y >= 0.0 && y < f64::from(self.height));
        self.pinhole().unproject(&[(y, x)])[0].expect("finite eyepiece pixel")
    }
}

/// Ground truth, independent of detection or solving. RA/Dec are J2000 degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrueDirection {
    pub alt: f64,
    pub az: f64,
    pub ra: f64,
    pub dec: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrueDirections {
    pub camera: TrueDirection,
    pub eyepiece: TrueDirection,
}

#[derive(Clone, Copy)]
pub(crate) struct Pointing {
    pub optics: Optics,
    pub site: Site,
    pub alt: f64,
    pub az: f64,
}

/// Cloneable control handle shared with the camera. Alt/az denotes the camera centre.
#[derive(Clone)]
pub struct Simulator {
    pub(crate) db: Arc<PatternDatabase>,
    pub(crate) state: Arc<Mutex<Pointing>>,
    clock: VirtualClock,
}

impl Simulator {
    pub fn new(db: Arc<PatternDatabase>, site: Site, clock: VirtualClock) -> Self {
        assert!(site.lat_deg.is_finite() && site.lat_deg.abs() <= 90.0 && site.lon_deg.is_finite());
        Self {
            db,
            state: Arc::new(Mutex::new(Pointing {
                optics: Optics::default(),
                site,
                alt: 45.0,
                az: 180.0,
            })),
            clock,
        }
    }

    pub fn clock(&self) -> &VirtualClock {
        &self.clock
    }

    pub fn optics(&self) -> Optics {
        self.state.lock().unwrap().optics
    }

    pub fn set_optics(&self, optics: Optics) {
        optics.eyepiece_ray();
        self.state.lock().unwrap().optics = optics;
    }

    pub fn push_to(&self, alt: f64, az: f64) {
        assert!(alt.is_finite() && alt.abs() <= 90.0 && az.is_finite());
        let mut state = self.state.lock().unwrap();
        state.alt = alt;
        state.az = az.rem_euclid(360.0);
    }

    pub fn telescope(&self) -> TrueDirections {
        self.directions_at(self.clock.now())
    }

    /// Truth at the supplied exposure instant, rather than the later read time.
    pub fn directions_at(&self, unix_s: f64) -> TrueDirections {
        let p = *self.state.lock().unwrap();
        let rotation = sky::camera_rotation(p.alt, p.az, p.site, unix_s);
        let direction = |v: UnitVector| {
            let (ra, dec) = v.to_radec();
            let (alt, az) = sky::horizontal(v, p.site, unix_s);
            TrueDirection {
                alt,
                az,
                ra: ra.to_degrees(),
                dec: dec.to_degrees(),
            }
        };
        let mut camera = direction(sky::unrotate(
            rotation,
            UnitVector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        ));
        camera.alt = p.alt;
        camera.az = p.az;
        TrueDirections {
            camera,
            eyepiece: direction(sky::unrotate(rotation, p.optics.eyepiece_ray())),
        }
    }

    /// Place a J2000 direction at the eyepiece pixel, accounting for the tube's roll.
    /// Like alt-az pointing itself, this is singular at the zenith: a fixed lateral
    /// offset cannot reach the tiny polar cap inside that offset.
    pub fn point_eyepiece_at(&self, ra: f64, dec: f64) {
        assert!(ra.is_finite() && dec.is_finite() && dec.abs() <= 90.0);
        let mut p = self.state.lock().unwrap();
        let (alt, az) = sky::horizontal(
            UnitVector::from_radec(ra.to_radians(), dec.to_radians()),
            p.site,
            self.clock.now(),
        );
        let ray = p.optics.eyepiece_ray();
        let radius = ray.x.hypot(ray.z);
        let up = alt.to_radians().sin();
        assert!(
            up.abs() <= radius,
            "eyepiece direction is inside the zenith singularity"
        );
        let camera_alt = (up / radius).asin() - ray.z.atan2(ray.x);
        let along = ray.x * camera_alt.cos() - ray.z * camera_alt.sin();
        p.alt = camera_alt.to_degrees();
        p.az = (az + ray.y.atan2(along).to_degrees()).rem_euclid(360.0);
    }

    /// Database indices and projected PSFs in brightness order at the current time.
    pub fn field_stars(&self, exposure_ms: f64, gain: f64) -> Vec<(usize, RenderStar)> {
        let p = *self.state.lock().unwrap();
        render::field_stars(
            &self.db,
            p.optics,
            sky::camera_rotation(p.alt, p.az, p.site, self.clock.now()),
            exposure_ms,
            gain,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_optics_defaults_and_independent_controls() {
        let mut optics = Optics::default();
        assert_eq!((optics.width, optics.height), (1456, 1088));
        assert_eq!(optics.fov_deg, 12.0);
        assert_eq!(optics.eyepiece_px, (700.0, 420.0));
        optics.width = 800;
        optics.height = 600;
        optics.fov_deg = 20.0;
        optics.eyepiece_px = (350.0, 240.0);
        assert_eq!(optics.pinhole().width, 800.0);
        assert_eq!(optics.pinhole().height, 600.0);
        assert_eq!(optics.pinhole().fov, 20_f64.to_radians());
        assert_eq!(optics.eyepiece_px, (350.0, 240.0));
    }
}
