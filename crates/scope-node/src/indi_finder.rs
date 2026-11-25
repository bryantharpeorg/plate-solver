//! The finder camera over INDI (045): the camera port over the imaging
//! side's INDI client, so the finder camera and the imaging camera can share one
//! INDI server, each on its own device and connection.
use crate::{
    camera::{check_range, FrameFuture, Mode},
    config::CameraMode,
    imaging::{Exposure, ImagingCamera, ImagingError, ImagingFrame, ImagingSettings},
    Camera, CameraError, CameraInfo, GrayFrame, RangeError, Settings,
};

/// The finder camera's frame time in ms: the one mode's `fps` is `1000 / FRAME_MS`,
/// which caps automatic exposure, and the first automatic exposure is this
/// long.
pub const FRAME_MS: f64 = 1000.0;

/// The purpose every exposure of the finder camera carries: a light frame.
pub const PURPOSE: &str = "finder";

fn failed(error: ImagingError) -> CameraError {
    CameraError(error.to_string())
}

pub struct IndiFinder<C> {
    camera: C,
    info: CameraInfo,
    settings: Settings,
    /// Whether the camera has a gain control.
    has_gain: bool,
    /// The gain the camera last answered, at discovery or to an apply;
    /// `None` after a restart or a frame at another gain.
    reported_gain: Option<f64>,
}

impl<C: ImagingCamera> IndiFinder<C> {
    /// Describe the camera once, before the node serves (the port's `info`
    /// is synchronous): a failure ends the start, as rpicam's does.
    pub async fn discover(camera: C, mode: CameraMode) -> Result<Self, CameraError> {
        let described = camera.describe().await.map_err(failed)?;
        if let Some(bayer) = described.bayer {
            return Err(CameraError(format!(
                "the INDI camera {} is a colour camera ({}); the finder camera must be mono",
                described.model,
                bayer.as_str()
            )));
        }
        let size = Mode {
            width: described.width,
            height: described.height,
            fps: 1000.0 / FRAME_MS,
        };
        if let CameraMode::Size { width, height } = mode {
            if (width, height) != (size.width, size.height) {
                return Err(CameraError(format!(
                    "camera has no {width}x{height} mode; it lists {}x{}",
                    size.width, size.height
                )));
            }
        }
        // The node's gain is positive: a range from 0 starts at 1.
        let (gain_min, gain_max) = match described.gain {
            Some(range) => (range.min.max(1.0), range.max.max(1.0)),
            None => (1.0, 1.0),
        };
        for (name, min, max) in [
            (
                "exposure_ms",
                described.exposure_ms.min,
                described.exposure_ms.max,
            ),
            ("gain", gain_min, gain_max),
        ] {
            if !min.is_finite() || !max.is_finite() || max < min {
                return Err(CameraError(format!("camera has invalid {name} range")));
            }
        }
        let in_force = camera
            .apply(&ImagingSettings::default())
            .await
            .map_err(failed)?;
        Ok(Self {
            camera,
            info: CameraInfo {
                model: described.model,
                width: size.width,
                height: size.height,
                modes: vec![size],
                active_mode: Some(size),
                exposure_min_ms: described.exposure_ms.min,
                exposure_max_ms: described.exposure_ms.max,
                gain_min,
                gain_max,
            },
            settings: Settings::default(),
            has_gain: described.gain.is_some(),
            reported_gain: in_force.gain,
        })
    }
}

/// The 8-bit gray pixels of a mono frame: a 16-bit sample's high byte, an
/// 8-bit sample as it is.
pub fn gray(frame: &ImagingFrame) -> Result<Vec<u8>, CameraError> {
    let count = frame.width as usize * frame.height as usize;
    let pixels: Vec<u8> = match frame.bits {
        8 => frame.samples.clone(),
        16 => frame.samples.iter().skip(1).step_by(2).copied().collect(),
        bits => return Err(CameraError(format!("unsupported sample size {bits} bits"))),
    };
    if pixels.len() != count {
        return Err(CameraError(format!(
            "the INDI frame holds {} samples, not {} × {}",
            pixels.len(),
            frame.width,
            frame.height
        )));
    }
    Ok(pixels)
}

impl<C: ImagingCamera + 'static> Camera for IndiFinder<C> {
    fn info(&self) -> CameraInfo {
        self.info.clone()
    }
    fn apply(&mut self, settings: &Settings) -> Result<(), RangeError> {
        check_range(&self.info, settings)?;
        self.settings = settings.clone();
        Ok(())
    }
    fn next_frame(&mut self) -> FrameFuture<'_> {
        Box::pin(async move {
            if self.has_gain {
                // Automatic gain starts from the camera's own, inside the range.
                let wanted = self.settings.gain.unwrap_or_else(|| {
                    self.reported_gain
                        .unwrap_or(self.info.gain_min)
                        .clamp(self.info.gain_min, self.info.gain_max)
                });
                if self.reported_gain != Some(wanted) {
                    let edit = ImagingSettings {
                        gain: Some(wanted),
                        ..ImagingSettings::default()
                    };
                    let answer = self.camera.apply(&edit).await.map_err(failed)?;
                    self.reported_gain = answer.gain;
                }
            }
            let exposure = Exposure {
                exposure_ms: self.settings.exposure_ms.unwrap_or_else(|| {
                    FRAME_MS.clamp(self.info.exposure_min_ms, self.info.exposure_max_ms)
                }),
                purpose: PURPOSE.to_string(),
            };
            let frame = self.camera.expose(&exposure).await.map_err(failed)?;
            // No frame is passed on at a gain the adapter did not send: the
            // client's retry after a restart runs on a new connection, whose
            // camera has its own gain. The next exposure sends it again.
            if self.has_gain && frame.gain != self.reported_gain {
                let sent = self.reported_gain.take().unwrap_or_default();
                return Err(CameraError(format!(
                    "the finder camera exposed at gain {}, not at the gain {sent} sent",
                    frame.gain.unwrap_or_default()
                )));
            }
            Ok(GrayFrame {
                pixels: gray(&frame)?,
                width: frame.width,
                height: frame.height,
                start_s: frame.start_s,
                end_s: frame.end_s,
                exposure_ms: frame.exposure_ms,
                gain: frame.gain.unwrap_or(1.0),
            })
        })
    }
    /// After a failure the gain is sent again with the next exposure: a
    /// restarted server's camera may have its own.
    fn restart(&mut self) -> Result<(), CameraError> {
        self.reported_gain = None;
        Ok(())
    }
}
