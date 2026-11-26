//! The finder alignment (050): imaging frames under the imaging camera's
//! lease, each solved and paired with the finder camera's live solve nearest
//! its middle, and where the imaging camera's centre lies from the finder
//! camera's centre, as the finder camera sees it, off its frame too. It moves
//! nothing. Every solve is offered to 032 through `Imaging::note_solve`.

use math_core::{angular_distance, UnitVector};
use serde::Serialize;

use super::relation::FinderFix;

/// The middle of the finder camera's frame: within this share of its width,
/// and of its height, of its centre.
pub const MIDDLE_SHARE: f64 = 0.25;

/// Where the imaging camera's centre lies in the finder camera's frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Centre {
    /// The finder camera's pixels on the pinhole model, outside the frame
    /// when it falls outside.
    pub x: f64,
    pub y: f64,
    pub finder_width: f64,
    pub finder_height: f64,
    /// Right of the frame's centre along its rows (left when negative), and up
    /// along its columns (down when negative).
    pub right_deg: f64,
    pub up_deg: f64,
    /// The two centres' separation on the sky.
    pub separation_deg: f64,
    /// 032's own test (its FR-020): the place, distortion included, inside the frame.
    pub on_frame: bool,
    /// On the frame and within `MIDDLE_SHARE` of its width and height of its centre.
    pub middle: bool,
}

/// The imaging centre `ra_deg`, `dec_deg` in the fix's frame; `None` behind
/// the finder camera.
pub fn centre_of(fix: &FinderFix, ra_deg: f64, dec_deg: f64) -> Option<Centre> {
    let centre = UnitVector::from_radec(ra_deg.to_radians(), dec_deg.to_radians());
    let frame = &fix.frame;
    let (x, y) = frame.project_pinhole(centre)?;
    let (w, h) = frame.size();
    let focal = (w / 2.0) / (frame.fov_deg().to_radians() / 2.0).tan();
    let on_frame = frame
        .project(centre)
        .is_some_and(|place| frame.contains(place, 0.0));
    Some(Centre {
        x,
        y,
        finder_width: w,
        finder_height: h,
        right_deg: ((x - w / 2.0) / focal).atan().to_degrees(),
        up_deg: ((h / 2.0 - y) / focal).atan().to_degrees(),
        separation_deg: angular_distance(frame.center, centre).to_degrees(),
        on_frame,
        middle: on_frame
            && (x - w / 2.0).abs() <= MIDDLE_SHARE * w
            && (y - h / 2.0).abs() <= MIDDLE_SHARE * h,
    })
}
