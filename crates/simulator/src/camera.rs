//! Adapter from the catalog renderer to the scope node's camera port.
use crate::{render, sky, Simulator};
use scope_link::proto::{CameraInfo, Mode};
use scope_node::{
    camera::{check_range, FrameFuture},
    Camera, GrayFrame, RangeError, Settings,
};
use std::time::Duration;

pub struct SimCamera {
    simulator: Simulator,
    settings: Settings,
    exposure_range: (f64, f64),
    gain_range: (f64, f64),
    seed: u64,
    frames: u64,
}

impl SimCamera {
    pub fn new(simulator: Simulator) -> Self {
        Self {
            simulator,
            settings: Settings::default(),
            exposure_range: (1.0, 2000.0),
            gain_range: (1.0, 16.0),
            seed: 13,
            frames: 0,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Change both ranges atomically. Existing fixed controls must still fit.
    pub fn set_ranges(
        &mut self,
        exposure_ms: (f64, f64),
        gain: (f64, f64),
    ) -> Result<(), RangeError> {
        for (setting, (min, max), unit) in [("exposure_ms", exposure_ms, " ms"), ("gain", gain, "")]
        {
            if !min.is_finite() || !max.is_finite() || min <= 0.0 || max < min {
                return Err(RangeError {
                    setting,
                    value: min,
                    min,
                    max,
                    unit,
                });
            }
        }
        let mut info = self.info();
        info.exposure_min_ms = exposure_ms.0;
        info.exposure_max_ms = exposure_ms.1;
        info.gain_min = gain.0;
        info.gain_max = gain.1;
        check_range(&info, &self.settings)?;
        self.exposure_range = exposure_ms;
        self.gain_range = gain;
        Ok(())
    }

    pub fn frames_taken(&self) -> u64 {
        self.frames
    }
}

impl Camera for SimCamera {
    fn info(&self) -> CameraInfo {
        let optics = self.simulator.optics();
        let mode = Mode {
            width: optics.width,
            height: optics.height,
            fps: 5.0,
        };
        CameraInfo {
            model: "simulator".into(),
            width: optics.width,
            height: optics.height,
            modes: vec![mode],
            active_mode: Some(mode),
            exposure_min_ms: self.exposure_range.0,
            exposure_max_ms: self.exposure_range.1,
            gain_min: self.gain_range.0,
            gain_max: self.gain_range.1,
        }
    }

    fn apply(&mut self, settings: &Settings) -> Result<(), RangeError> {
        check_range(&self.info(), settings)?;
        self.settings = settings.clone();
        Ok(())
    }

    fn next_frame(&mut self) -> FrameFuture<'_> {
        Box::pin(async move {
            let exposure_ms = self.settings.exposure_ms.unwrap_or(
                render::REFERENCE_EXPOSURE_MS.clamp(self.exposure_range.0, self.exposure_range.1),
            );
            let gain = self.settings.gain.unwrap_or(self.gain_range.0);
            let start_s = self.simulator.clock().node_now();
            let exposure_s = exposure_ms / 1000.0;
            let time = self.simulator.clock().now() + exposure_s / 2.0;
            // Snapshot optics and pointing before awaiting: changes affect the next exposure.
            let p = *self.simulator.state.lock().unwrap();
            let rotation = sky::camera_rotation(p.alt, p.az, p.site, time);
            let stars =
                render::field_stars(&self.simulator.db, p.optics, rotation, exposure_ms, gain);
            let id = self.frames;
            self.frames = self.frames.wrapping_add(1);
            tokio::time::sleep(Duration::from_secs_f64(exposure_s)).await;
            let pixels = render::render(
                &p.optics,
                &stars.iter().map(|(_, s)| *s).collect::<Vec<_>>(),
                exposure_ms,
                gain,
                self.seed,
                id,
            );
            Ok(GrayFrame {
                pixels,
                width: p.optics.width,
                height: p.optics.height,
                start_s,
                end_s: start_s + exposure_s,
                exposure_ms,
                gain,
            })
        })
    }
}
