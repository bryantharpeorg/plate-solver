//! Focus (033): the focus reading, the focus region, the focus record and the
//! verdict.
use super::solver::{ImagingSolver, Region, SolveError, SolveFrame};

/// Fewer stars than this in a star measure are no reading.
pub const MIN_STARS: u32 = 5;
/// A star measure above this many frame pixels is "far out". 20 frame pixels is
/// ASTAP's limit on a mono frame, and half of it on a colour mosaic that the
/// adapter averages 2 × 2 (`docs/research/2026-11-15-imaging-camera.md` § 3.
/// ASTAP).
pub const FAR_OUT_PX: f64 = 20.0;

/// What one imaging frame, or a region of it, says about focus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusReading {
    pub frame_id: u64,
    /// The median half-flux diameter in sensor pixels.
    pub px: f64,
    /// The same in arcseconds, when a scale was given.
    pub arcsec: Option<f64>,
    pub stars: u32,
    /// The part of the sensor that was measured, in sensor pixels.
    pub region: Region,
    /// The middle of the exposure, console Unix seconds.
    pub at: f64,
}

/// Why a frame, or a region of it, says nothing about focus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unmeasurable {
    TooFewStars {
        stars: u32,
    },
    /// The size in sensor pixels, for showing only.
    FarOut {
        px: f64,
        stars: u32,
    },
}

pub type Read = Result<FocusReading, Unmeasurable>;

/// `region` is in the frame's pixels; `None` is the whole frame. `scale` is
/// arcseconds a sensor pixel.
pub async fn read(
    solver: &dyn ImagingSolver,
    frame: SolveFrame,
    frame_id: u64,
    region: Option<Region>,
    scale: Option<f64>,
) -> Result<Read, SolveError> {
    let binning = frame.binning;
    // The solver's own too-few-stars is a reading that says nothing, whatever
    // its words; any other error of the solver is `read`'s own, unchanged.
    let measure = match solver.measure(frame.clone(), region).await {
        Ok(measure) => measure,
        Err(SolveError::TooFewStars { .. }) => {
            return Ok(Err(Unmeasurable::TooFewStars { stars: 0 }));
        }
        Err(error) => return Err(error),
    };
    if measure.stars < MIN_STARS {
        return Ok(Err(Unmeasurable::TooFewStars {
            stars: measure.stars,
        }));
    }
    // The star measure is in the frame's own pixels, so the limit is read there
    // and the reading is the size in sensor pixels.
    if measure.half_flux_px > FAR_OUT_PX {
        return Ok(Err(Unmeasurable::FarOut {
            px: measure.half_flux_px * f64::from(binning),
            stars: measure.stars,
        }));
    }
    let r = region.unwrap_or(Region {
        x: 0,
        y: 0,
        width: frame.width,
        height: frame.height,
    });
    let px = measure.half_flux_px * f64::from(binning);
    Ok(Ok(FocusReading {
        frame_id,
        px,
        arcsec: scale.map(|scale| px * scale),
        stars: measure.stars,
        region: Region {
            x: frame.roi.x + r.x * binning,
            y: frame.roi.y + r.y * binning,
            width: r.width * binning,
            height: r.height * binning,
        },
        at: frame.start + frame.exposure_s / 2.0,
    }))
}
