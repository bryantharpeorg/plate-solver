//! A seeded stand-in for `[imaging] adapter = "simulated"` and for tests: no sky.
use crate::{
    clock::NodeClock,
    imaging::{
        check_exposure, check_settings, BayerPattern, Exposure, ImagingCamera, ImagingError,
        ImagingFrame, ImagingFuture, ImagingInfo, ImagingSettings, ImagingStatus, Range, Roi,
    },
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Notify;

/// The stand-in's whole description, fixed at `new`.
fn info() -> ImagingInfo {
    ImagingInfo {
        model: "synthetic imaging camera".into(),
        serial: None,
        width: 64,
        height: 48,
        pixel_um: 4.78,
        bits: 16,
        bayer: Some(BayerPattern::Rggb),
        exposure_ms: Range {
            min: 1.0,
            max: 60_000.0,
        },
        gain: Some(Range {
            min: 0.0,
            max: 300.0,
        }),
        offset: Some(Range {
            min: 0.0,
            max: 100.0,
        }),
        binnings: vec![1, 2],
        cooler: true,
    }
}

/// The settings in force, whether an exposure runs, and how many aborts have
/// been asked for. The count keeps a stale permit left by an exposure that
/// finished between an abort and its own end from ending the next one.
struct State {
    settings: ImagingSettings,
    exposing: bool,
    aborts: u64,
}

/// The cooler's rule: with the cooler off, or on with no set temperature, the
/// sensor reads 20.0 °C at power 0.0; on with a set temperature `T`, it reads
/// `T` at power `clamp(2.5·(20 − T), 0, 100)`.
fn cooler(settings: &ImagingSettings) -> (f64, f64) {
    match (settings.cooler_on, settings.set_temp_c) {
        (Some(true), Some(temp)) => (temp, (2.5 * (20.0 - temp)).clamp(0.0, 100.0)),
        _ => (20.0, 0.0),
    }
}

/// One frame of the sample rule: with binning `b`, region origin `(x0, y0)` and
/// offset `o` in force, the sample at column `c` and row `r` is
/// `256 + 16·round(o) + ((x0/b + c + 3·(y0/b + r)) mod 251)`, little-endian,
/// row by row from the top-left. The delivered frame is the region's width and
/// height each divided by the binning, rounded down.
fn frame(settings: &ImagingSettings, exposure: &Exposure, start_s: f64) -> ImagingFrame {
    let binning = settings.binning.unwrap_or(1).max(1);
    let roi = settings.roi.unwrap_or_default();
    let width = roi.width / binning;
    let height = roi.height / binning;
    let base = 256.0 + 16.0 * settings.offset.unwrap_or(0.0).round();
    let mut samples = Vec::with_capacity(width as usize * height as usize * 2);
    for row in 0..height {
        for column in 0..width {
            let index = (roi.x / binning + column + 3 * (roi.y / binning + row)) % 251;
            samples.extend_from_slice(&((base + f64::from(index)) as u16).to_le_bytes());
        }
    }
    let (temp_c, _) = cooler(settings);
    ImagingFrame {
        samples,
        width,
        height,
        bits: 16,
        bayer: Some(BayerPattern::Rggb),
        start_s,
        end_s: start_s + exposure.exposure_ms / 1000.0,
        exposure_ms: exposure.exposure_ms,
        gain: settings.gain,
        offset: settings.offset,
        binning,
        temp_c: Some(temp_c),
        roi,
    }
}

/// Clears the exposing mark on every way out, a dropped future included.
struct ExposingGuard<'a>(&'a Mutex<State>);
impl Drop for ExposingGuard<'_> {
    fn drop(&mut self) {
        self.0.lock().unwrap().exposing = false;
    }
}

/// A seeded stand-in for `[imaging] adapter = "simulated"` and for tests: no
/// sky, one exposure at a time, and frames from the sample rule.
pub struct SyntheticImagingCamera {
    clock: Arc<dyn NodeClock>,
    state: Mutex<State>,
    abort: Notify,
}

impl SyntheticImagingCamera {
    pub fn new(clock: Arc<dyn NodeClock>) -> Self {
        Self {
            clock,
            state: Mutex::new(State {
                settings: ImagingSettings {
                    gain: Some(100.0),
                    offset: Some(10.0),
                    binning: Some(1),
                    roi: Some(Roi {
                        x: 0,
                        y: 0,
                        width: 64,
                        height: 48,
                    }),
                    cooler_on: Some(false),
                    set_temp_c: None,
                },
                exposing: false,
                aborts: 0,
            }),
            abort: Notify::new(),
        }
    }
}

impl ImagingCamera for SyntheticImagingCamera {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo> {
        Box::pin(async move { Ok(info()) })
    }

    fn status(&self) -> ImagingFuture<'_, ImagingStatus> {
        Box::pin(async move {
            let state = self.state.lock().unwrap();
            let (temp_c, cooler_power) = cooler(&state.settings);
            Ok(ImagingStatus {
                temp_c: Some(temp_c),
                cooler_power: Some(cooler_power),
                set_temp_c: state.settings.set_temp_c,
                exposing: state.exposing,
            })
        })
    }

    fn apply<'a>(&'a self, settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings> {
        Box::pin(async move {
            let mut state = self.state.lock().unwrap();
            // While an exposure runs, an empty edit answers the settings in
            // force and any other is Busy before any check, changing nothing.
            if state.exposing {
                if *settings == ImagingSettings::default() {
                    return Ok(state.settings.clone());
                }
                return Err(ImagingError::Busy);
            }
            let merged = check_settings(&info(), &state.settings, settings)?;
            state.settings = merged.clone();
            Ok(merged)
        })
    }

    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame> {
        Box::pin(async move {
            check_exposure(&info(), exposure)?;
            let (settings, aborts) = {
                let mut state = self.state.lock().unwrap();
                if state.exposing {
                    return Err(ImagingError::Busy);
                }
                state.exposing = true;
                (state.settings.clone(), state.aborts)
            };
            let _clear = ExposingGuard(&self.state);
            let start_s = self.clock.now();
            let wait = tokio::time::sleep(Duration::from_secs_f64(exposure.exposure_ms / 1000.0));
            tokio::pin!(wait);
            loop {
                tokio::select! {
                    _ = &mut wait => break,
                    _ = self.abort.notified() => {
                        // A permit an older exposure left behind is not ours.
                        if self.state.lock().unwrap().aborts != aborts {
                            return Err(ImagingError::Aborted);
                        }
                    }
                }
            }
            Ok(frame(&settings, exposure, start_s))
        })
    }

    fn abort(&self) -> ImagingFuture<'_, ()> {
        Box::pin(async move {
            let mut state = self.state.lock().unwrap();
            if state.exposing {
                state.aborts += 1;
                drop(state);
                self.abort.notify_one();
            }
            Ok(())
        })
    }
}
