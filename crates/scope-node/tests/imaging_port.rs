//! US1's scenarios, one test each. Oracles: plan.md § The stand-in camera and
//! § The port, with the sample rule and every expected value computed here,
//! not read from the camera.
use scope_node::{
    clock::NodeClock,
    imaging::{
        check_settings, synthetic::SyntheticImagingCamera, BayerPattern, Exposure, ImagingCamera,
        ImagingError, ImagingInfo, ImagingSettings, ImagingStatus, Range, Roi,
    },
};
use std::{sync::Arc, time::Duration};

// The test clock of spec.md § User Scenarios: 1000.0 plus the tokio time
// elapsed since it was built, so paused time advances it exactly.
struct Clock(tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        1000.0 + self.0.elapsed().as_secs_f64()
    }
}
fn camera() -> (Arc<dyn NodeClock>, SyntheticImagingCamera) {
    let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
    (clock.clone(), SyntheticImagingCamera::new(clock))
}

/// The stand-in's description, from plan.md § The stand-in camera.
fn description() -> ImagingInfo {
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

/// The settings in force at the start: gain 100, offset 10, binning 1, the
/// whole sensor, cooler off, no set temperature.
fn initial() -> ImagingSettings {
    ImagingSettings {
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
    }
}

/// The sample rule, computed here from plan.md § The stand-in camera: with
/// binning `b`, region origin `(x0, y0)` and offset `o` in force, the sample at
/// column `c` and row `r` is `256 + 16·round(o) + ((x0/b + c + 3·(y0/b + r))
/// mod 251)`, little-endian, row by row from the top-left.
fn samples(roi: Roi, binning: u32, offset: f64, width: u32, height: u32) -> Vec<u8> {
    let base = 256.0 + 16.0 * offset.round();
    let mut bytes = Vec::with_capacity(width as usize * height as usize * 2);
    for row in 0..height {
        for column in 0..width {
            let index = (roi.x / binning + column + 3 * (roi.y / binning + row)) % 251;
            bytes.extend_from_slice(&((base + f64::from(index)) as u16).to_le_bytes());
        }
    }
    bytes
}

/// One refused `apply` answers `Invalid` with its text and changes nothing:
/// an empty `apply` after it answers the settings as they were.
async fn refuses(
    camera: &SyntheticImagingCamera,
    edit: &ImagingSettings,
    text: &str,
    in_force: &ImagingSettings,
) {
    assert_eq!(
        camera.apply(edit).await,
        Err(ImagingError::Invalid(text.to_string()))
    );
    assert_eq!(
        camera.apply(&ImagingSettings::default()).await,
        Ok(in_force.clone())
    );
}

#[tokio::test(start_paused = true)]
async fn the_stand_in_camera_describes_itself_and_its_status_exactly() {
    let (_, camera) = camera();
    assert_eq!(camera.describe().await, Ok(description()));
    assert_eq!(
        camera.status().await,
        Ok(ImagingStatus {
            temp_c: Some(20.0),
            cooler_power: Some(0.0),
            set_temp_c: None,
            exposing: false,
        })
    );
}

#[tokio::test(start_paused = true)]
async fn apply_answers_the_settings_in_force_and_changes_only_what_it_names() {
    let (_, camera) = camera();
    let first = camera.apply(&ImagingSettings::default()).await;
    assert_eq!(first, Ok(initial()));
    let second = camera
        .apply(&ImagingSettings {
            gain: Some(200.0),
            binning: Some(2),
            ..ImagingSettings::default()
        })
        .await;
    let third = camera.apply(&ImagingSettings::default()).await;
    assert_eq!(second, third);
    assert_eq!(
        second,
        Ok(ImagingSettings {
            gain: Some(200.0),
            binning: Some(2),
            ..initial()
        })
    );
    // The cooler's apply keeps gain 200 and binning 2 in force.
    let cooled = camera
        .apply(&ImagingSettings {
            cooler_on: Some(true),
            set_temp_c: Some(-10.0),
            ..ImagingSettings::default()
        })
        .await;
    assert_eq!(
        cooled,
        Ok(ImagingSettings {
            cooler_on: Some(true),
            set_temp_c: Some(-10.0),
            ..second.unwrap()
        })
    );
    // The cooler's rule: on with a set temperature T it reads T at power
    // clamp(2.5·(20 − T), 0, 100).
    assert_eq!(
        camera.status().await,
        Ok(ImagingStatus {
            temp_c: Some(-10.0),
            cooler_power: Some(75.0),
            set_temp_c: Some(-10.0),
            exposing: false,
        })
    );
}

#[tokio::test(start_paused = true)]
async fn a_500_ms_exposure_takes_exactly_500_ms_and_follows_the_sample_rule() {
    let (clock, camera) = camera();
    let at_call = clock.now();
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "frame".into(),
        })
        .await
        .unwrap();
    // Exactly 500 ms of virtual time, and the clock's readings at the start
    // and the end of the exposure.
    assert_eq!(clock.now(), at_call + 0.5);
    assert_eq!((frame.start_s, frame.end_s), (at_call, at_call + 0.5));
    assert_eq!(frame.exposure_ms, 500.0);
    assert_eq!((frame.width, frame.height, frame.bits), (64, 48, 16));
    assert_eq!(frame.bayer, Some(BayerPattern::Rggb));
    assert_eq!(frame.binning, 1);
    assert_eq!(frame.gain, Some(100.0));
    assert_eq!(frame.offset, Some(10.0));
    assert_eq!(frame.temp_c, Some(20.0));
    assert_eq!(frame.samples.len(), 6144);
    let roi = Roi {
        x: 0,
        y: 0,
        width: 64,
        height: 48,
    };
    assert_eq!(frame.roi, roi);
    assert_eq!(frame.samples, samples(roi, 1, 10.0, 64, 48));
    // The worked example of spec.md § User Scenarios: the sample at column 5,
    // row 2 is 427, the bytes `AB 01`.
    let at = 2 * (2 * 64 + 5);
    assert_eq!(&frame.samples[at..at + 2], &[0xAB, 0x01]);
}

#[tokio::test(start_paused = true)]
async fn binning_2_and_a_region_deliver_the_region_over_the_binning() {
    let (_, camera) = camera();
    camera
        .apply(&ImagingSettings {
            binning: Some(2),
            roi: Some(Roi {
                x: 8,
                y: 4,
                width: 16,
                height: 8,
            }),
            ..ImagingSettings::default()
        })
        .await
        .unwrap();
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 100.0,
            purpose: "frame".into(),
        })
        .await
        .unwrap();
    assert_eq!((frame.width, frame.height), (8, 4));
    assert_eq!(frame.binning, 2);
    // The region in force, in unbinned sensor pixels.
    let roi = Roi {
        x: 8,
        y: 4,
        width: 16,
        height: 8,
    };
    assert_eq!(frame.roi, roi);
    assert_eq!(frame.samples, samples(roi, 2, 10.0, 8, 4));
    assert_eq!(&frame.samples[..2], &426u16.to_le_bytes());
}

#[tokio::test(start_paused = true)]
async fn each_refused_setting_answers_invalid_with_its_text_and_changes_nothing() {
    let (_, camera) = camera();
    let mut in_force = initial();
    refuses(
        &camera,
        &ImagingSettings {
            gain: Some(301.0),
            ..ImagingSettings::default()
        },
        "gain 301 is outside the camera's range 0–300",
        &in_force,
    )
    .await;
    refuses(
        &camera,
        &ImagingSettings {
            offset: Some(-1.0),
            ..ImagingSettings::default()
        },
        "offset -1 is outside the camera's range 0–100",
        &in_force,
    )
    .await;
    refuses(
        &camera,
        &ImagingSettings {
            binning: Some(3),
            ..ImagingSettings::default()
        },
        "binning 3 is not one of the camera's binnings 1, 2",
        &in_force,
    )
    .await;
    refuses(
        &camera,
        &ImagingSettings {
            roi: Some(Roi {
                x: 2,
                y: 0,
                width: 64,
                height: 48,
            }),
            ..ImagingSettings::default()
        },
        "roi must lie inside the 64 × 48 sensor with x and y multiples of 2",
        &in_force,
    )
    .await;
    // The region (2, 2, 16, 8) is accepted at binning 1; binning 2 alone is
    // then refused for it, changing nothing.
    let accepted = camera
        .apply(&ImagingSettings {
            roi: Some(Roi {
                x: 2,
                y: 2,
                width: 16,
                height: 8,
            }),
            ..ImagingSettings::default()
        })
        .await
        .unwrap();
    in_force.roi = accepted.roi;
    refuses(
        &camera,
        &ImagingSettings {
            binning: Some(2),
            ..ImagingSettings::default()
        },
        "roi must lie inside the 64 × 48 sensor with x and y multiples of 4",
        &in_force,
    )
    .await;
    refuses(
        &camera,
        &ImagingSettings {
            roi: Some(Roi {
                x: 0,
                y: 0,
                width: 80,
                height: 48,
            }),
            ..ImagingSettings::default()
        },
        "roi must lie inside the 64 × 48 sensor with x and y multiples of 2",
        &in_force,
    )
    .await;
    // The cooler on, on a camera of the test's own description without one,
    // through `check_settings`.
    let no_cooler = ImagingInfo {
        cooler: false,
        ..description()
    };
    assert_eq!(
        check_settings(
            &no_cooler,
            &in_force,
            &ImagingSettings {
                cooler_on: Some(true),
                ..ImagingSettings::default()
            }
        ),
        Err(ImagingError::Invalid(
            "this camera has no cooler".to_string()
        ))
    );
    refuses(
        &camera,
        &ImagingSettings {
            set_temp_c: Some(60.0),
            ..ImagingSettings::default()
        },
        "set_temp_c must be between -50 and 50",
        &in_force,
    )
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_bad_exposure_or_purpose_is_refused_at_once() {
    let (clock, camera) = camera();
    let before = clock.now();
    for ms in [0.5, 60_001.0, f64::NAN] {
        assert_eq!(
            camera
                .expose(&Exposure {
                    exposure_ms: ms,
                    purpose: "frame".into(),
                })
                .await,
            Err(ImagingError::Invalid(format!(
                "exposure_ms {ms} is outside the camera's range 1–60000 ms"
            )))
        );
    }
    for purpose in ["", "Dark", "abcdefghijklmnopq"] {
        assert_eq!(
            camera
                .expose(&Exposure {
                    exposure_ms: 500.0,
                    purpose: purpose.into(),
                })
                .await,
            Err(ImagingError::Invalid(
                "purpose must be 1 to 16 lower-case letters".to_string()
            ))
        );
    }
    // Each refusal answered at once: no virtual time passed.
    assert_eq!(clock.now(), before);
}

#[tokio::test(start_paused = true)]
async fn a_second_expose_is_busy_and_abort_ends_the_first_at_once() {
    let clock: Arc<dyn NodeClock> = Arc::new(Clock(tokio::time::Instant::now()));
    let camera = Arc::new(SyntheticImagingCamera::new(clock.clone()));
    let first = {
        let camera = camera.clone();
        tokio::spawn(async move {
            camera
                .expose(&Exposure {
                    exposure_ms: 10_000.0,
                    purpose: "frame".into(),
                })
                .await
        })
    };
    tokio::time::sleep(Duration::from_secs(1)).await;
    // While one runs, status says so and an empty apply answers the settings
    // in force.
    assert!(camera.status().await.unwrap().exposing);
    assert_eq!(
        camera.apply(&ImagingSettings::default()).await,
        Ok(initial())
    );
    // An apply that names a setting is Busy and changes nothing.
    assert_eq!(
        camera
            .apply(&ImagingSettings {
                gain: Some(150.0),
                ..ImagingSettings::default()
            })
            .await,
        Err(ImagingError::Busy)
    );
    assert_eq!(
        camera.apply(&ImagingSettings::default()).await,
        Ok(initial())
    );
    // A second expose answers Busy at once.
    assert_eq!(
        camera
            .expose(&Exposure {
                exposure_ms: 100.0,
                purpose: "frame".into(),
            })
            .await,
        Err(ImagingError::Busy)
    );
    let at_abort = clock.now();
    camera.abort().await.unwrap();
    // The first exposure answers Aborted at that same virtual instant, and
    // the status follows.
    assert_eq!(first.await.unwrap(), Err(ImagingError::Aborted));
    assert_eq!(clock.now(), at_abort);
    assert!(!camera.status().await.unwrap().exposing);
    // Abort with nothing exposing answers Ok.
    camera.abort().await.unwrap();
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 100.0,
            purpose: "frame".into(),
        })
        .await
        .unwrap();
    assert_eq!((frame.width, frame.height), (64, 48));
}
