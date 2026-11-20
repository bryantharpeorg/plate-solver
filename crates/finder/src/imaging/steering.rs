//! Tracking steered by the subs (036): the tracking error a sub's imaging solve
//! gives and the one interface between the stack and a tracker. Sky terms only.

use std::{future::Future, pin::Pin};

use serde::Serialize;

use super::solver::{ImagingSolve, Region};

/// How far the sky at the set point is from the target.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TrackingError {
    /// Console Unix seconds: the sub's mid-exposure.
    pub time: f64,
    /// East (RA times cos Dec), in arcseconds.
    pub east_arcsec: f64,
    /// North, in arcseconds.
    pub north_arcsec: f64,
}

/// The sky at `set_point`, in the frame's pixels, less `target`, in J2000 degrees.
/// `None` when the target has no pixel in the solve or a number is not finite.
pub fn tracking_error(
    solve: &ImagingSolve,
    set_point: (f64, f64),
    target: (f64, f64),
    time: f64,
) -> Option<TrackingError> {
    if ![time, set_point.0, set_point.1, target.0, target.1]
        .into_iter()
        .all(f64::is_finite)
    {
        return None;
    }
    let pixel = solve.pixel_of(target.0, target.1)?;
    let (dx, dy) = (set_point.0 - pixel.0, set_point.1 - pixel.1);
    let cd = solve.cd_deg;
    let east_arcsec = 3600.0 * (cd[0][0] * dx + cd[0][1] * dy);
    let north_arcsec = 3600.0 * (cd[1][0] * dx + cd[1][1] * dy);
    (east_arcsec.is_finite() && north_arcsec.is_finite()).then_some(TrackingError {
        time,
        east_arcsec,
        north_arcsec,
    })
}

/// A set point given in sensor pixels, or the sensor's centre for `None`, in the
/// pixels of a frame that covers `roi` at `binning`.
pub fn set_point_in_frame(
    sensor: (u32, u32),
    set_point: Option<(f64, f64)>,
    roi: Region,
    binning: u32,
) -> (f64, f64) {
    let (sx, sy) = set_point.unwrap_or((f64::from(sensor.0) / 2.0, f64::from(sensor.1) / 2.0));
    (
        (sx - f64::from(roi.x)) / f64::from(binning),
        (sy - f64::from(roi.y)) / f64::from(binning),
    )
}

/// What a gap moved the telescope by, on the sky, in arcseconds.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stepped {
    pub east_arcsec: f64,
    pub north_arcsec: f64,
}

/// A gap's answer. It owns only the answer's receiver and borrows nothing of the
/// tracker, so awaiting it keeps no tracker alive.
pub type GapFuture = Pin<Box<dyn Future<Output = Stepped> + Send>>;

/// The one interface between the stack and whatever tracker is running.
pub trait Steered: Send + Sync {
    /// The J2000 place, in degrees, the tracker keeps at the set point.
    fn target(&self) -> Option<(f64, f64)>;
    /// Hand over a tracking error. It changes nothing until the next gap.
    fn steer(&self, error: TrackingError);
    /// The stack is between two exposures. The tracker makes its correction now
    /// and answers once its rates are acknowledged. The gap is asked for when
    /// this is called, not when the future is first polled.
    fn gap(&self) -> GapFuture;
    /// The stack no longer steers.
    fn release(&self);
    /// Whether tracking errors, rather than its own corrections, steer it now.
    fn steered_by_subs(&self) -> bool;
    /// The longest exposure allowed for `turn_rad`, in seconds; `None` when
    /// nothing limits it.
    fn longest_exposure_s(&self, _turn_rad: f64) -> Option<f64> {
        None
    }
    /// The same for an exposure decided now and held: the shortest that answer
    /// will be over the hour ahead.
    fn longest_exposure_ahead_s(&self, _turn_rad: f64) -> Option<f64> {
        None
    }
    /// Whether it is steady enough to expose.
    fn steady(&self) -> bool {
        true
    }
    /// Whether it was steady for all of a window of console Unix seconds.
    fn steady_between(&self, _start: f64, _end: f64) -> bool {
        true
    }
    /// Why it is not steady, in one word of its own for the page; `None` while
    /// it is steady or has nothing to say.
    fn pause(&self) -> Option<&'static str> {
        None
    }
    /// The share of a tracking error to take out as a step, and of a drift rate
    /// to take out of the rate, each from 0 to 1. A tracker with no such law
    /// ignores them.
    fn tune(&self, _step_share: f64, _rate_share: f64) {}
}
