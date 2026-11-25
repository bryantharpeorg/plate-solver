//! US1-S1–S10. Oracles: the stand-in's properties and vector expectations,
//! independent FITS bytes, literal refusal texts, and the supplied clock.
//! Pixel, range, gain and paused-time comparisons have zero tolerance.
mod finder_script;
mod support;

use finder_script::{
    answers, camera, exposure, finder, flat, gain_step, light_request, ramp, reading, worked_frame,
    DEVICE,
};
use scope_node::{
    camera::Mode,
    config::CameraMode,
    imaging::{Exposure, ImagingCamera, Range},
    indi_finder::IndiFinder,
    Camera, CameraInfo, Settings,
};
use std::time::Duration;
use support::{colour, mono, Scripts, COLOUR_DEVICE, MONO_DEVICE};

/// US1-S1: all description fields come from the named camera's properties,
/// including the exposure answer's replacement range and positive gain floor.
#[tokio::test(start_paused = true)]
async fn info_describes_the_finder_camera() {
    let scripts = Scripts::new(vec![finder(0.0)]);
    let rig = IndiFinder::discover(camera(&scripts, DEVICE), CameraMode::Auto)
        .await
        .unwrap();
    let mode = Mode {
        width: 1280,
        height: 960,
        fps: 1.0,
    };
    assert_eq!(
        rig.info(),
        CameraInfo {
            model: "Finder CCD".to_string(),
            width: 1280,
            height: 960,
            modes: vec![mode],
            active_mode: Some(mode),
            exposure_min_ms: 0.032,
            exposure_max_ms: 2_000_000.0,
            gain_min: 1.0,
            gain_max: 100.0,
        }
    );
    scripts.finish().await;
}

/// US1-S2: default controls first raise gain 0 to 1, await the answer, and
/// pass the high byte of each worked 16-bit sample, with the Busy timestamp.
#[tokio::test(start_paused = true)]
async fn default_settings_take_the_high_bytes_at_the_answered_gain() {
    let script = finder(0.0) + &gain_step(1.0) + &exposure(1.0, &worked_frame());
    let scripts = Scripts::new(vec![script]);
    let mut rig = IndiFinder::discover(camera(&scripts, DEVICE), CameraMode::Auto)
        .await
        .unwrap();
    let frame = rig.next_frame().await.unwrap();
    assert_eq!(
        frame.pixels,
        [0x00, 0x00, 0x00, 0x01, 0x12, 0x80, 0xFF, 0xFF]
    );
    assert_eq!((frame.width, frame.height), (4, 2));
    assert_eq!(frame.exposure_ms, 1000.0);
    assert_eq!(frame.gain, 1.0);
    assert_eq!(frame.start_s, reading(400));
    assert_eq!(frame.end_s, reading(400) + 1.0);
    scripts.finish().await;
}

/// US1-S3: fixed controls hold across exposures, without a redundant gain
/// vector, and restart makes the next exposure send gain again.
#[tokio::test(start_paused = true)]
async fn fixed_settings_hold_until_restart_sends_the_gain_again() {
    let image = ramp();
    let script = finder(50.0)
        + &gain_step(30.0)
        + &exposure(0.5, &image)
        + &exposure(0.5, &image)
        + &gain_step(30.0)
        + &exposure(0.5, &image);
    let scripts = Scripts::new(vec![script]);
    let mut rig = IndiFinder::discover(camera(&scripts, DEVICE), CameraMode::Auto)
        .await
        .unwrap();
    rig.apply(&Settings {
        exposure_ms: Some(500.0),
        gain: Some(30.0),
        ..Settings::default()
    })
    .unwrap();
    for index in 0..3 {
        if index == 2 {
            rig.restart().unwrap();
        }
        let frame = rig.next_frame().await.unwrap();
        assert_eq!(frame.pixels, [0, 1, 2, 253, 254, 255]);
        assert_eq!(frame.exposure_ms, 500.0);
        assert_eq!(frame.gain, 30.0);
    }
    scripts.finish().await;
}

/// US1-S4: invalid edits fail locally; the script accepts no vector after
/// enableBLOB, proving these refusals send nothing to the camera.
#[tokio::test(start_paused = true)]
async fn out_of_range_settings_send_nothing() {
    let scripts = Scripts::new(vec![finder(0.0)]);
    let mut rig = IndiFinder::discover(camera(&scripts, DEVICE), CameraMode::Auto)
        .await
        .unwrap();
    for (settings, expected) in [
        (
            Settings {
                gain: Some(101.0),
                ..Settings::default()
            },
            "gain 101 is outside the camera's range 1–100",
        ),
        (
            Settings {
                gain: Some(0.5),
                ..Settings::default()
            },
            "gain 0.5 is outside the camera's range 1–100",
        ),
        (
            Settings {
                exposure_ms: Some(0.01),
                ..Settings::default()
            },
            "exposure_ms 0.01 ms is outside the camera's range 0.032–2000000 ms",
        ),
    ] {
        assert_eq!(rig.apply(&settings).unwrap_err().to_string(), expected);
    }
    scripts.finish().await;
}

/// US1-S5: startup refuses a mosaic, an unavailable mode, a reversed
/// exposure range and an unreachable server with the prescribed text.
#[tokio::test(start_paused = true)]
async fn discovery_refuses_colour_wrong_size_invalid_range_and_no_server() {
    for (script, device, mode, expected) in [
        (colour(), COLOUR_DEVICE, CameraMode::Auto,
         "the INDI camera ZWO CCD ASI071MC Pro is a colour camera (RGGB); the finder camera must be mono"),
        (finder(0.0), DEVICE, CameraMode::Size { width: 640, height: 480 },
         "camera has no 640x480 mode; it lists 1280x960"),
        (finder(0.0).replace("min=\"3.2e-05\" max=\"2000\"", "min=\"3000\" max=\"2000\""),
         DEVICE, CameraMode::Auto, "camera has invalid exposure_ms range"),
    ] {
        let scripts = Scripts::new(vec![script]);
        let error = IndiFinder::discover(camera(&scripts, device), mode).await.err().unwrap();
        assert_eq!(error.to_string(), expected);
        scripts.finish().await;
    }
    let scripts = Scripts::new(Vec::new());
    let error = IndiFinder::discover(camera(&scripts, DEVICE), CameraMode::Auto)
        .await
        .err()
        .unwrap();
    assert_eq!(
        error.to_string(),
        "cannot reach the INDI server at 127.0.0.1:7624: no script left to play"
    );
    scripts.finish().await;
}

/// US1-S6: changing the expectation's device leaves the actual properties
/// naming Finder CCD, so the requested device is missing for exactly 5 s.
#[tokio::test(start_paused = true)]
async fn a_missing_named_device_fails_after_five_seconds() {
    let script = finder(0.0)
        .replace("@ device Finder CCD", "@ device Other CCD")
        .replace("@ expect newSwitchVector CONNECTION CONNECT=On\n", "")
        .replace("@ expect after 200 enableBLOB text=Also\n", "");
    let scripts = Scripts::new(vec![script]);
    let start = tokio::time::Instant::now();
    let error = IndiFinder::discover(camera(&scripts, "Other CCD"), CameraMode::Auto)
        .await
        .err()
        .unwrap();
    assert_eq!(start.elapsed(), Duration::from_secs(5));
    assert_eq!(
        error.to_string(),
        "the INDI server at 127.0.0.1:7624 has no device Other CCD"
    );
    scripts.finish().await;
}

/// US1-S7: the client's in-flight retry runs on a fresh device at gain 0.
/// That frame fails; the following exposure sends gain 30 again.
#[tokio::test(start_paused = true)]
async fn a_retried_frame_at_a_reset_gain_fails_then_the_gain_is_sent_again() {
    let image = worked_frame();
    let first =
        finder(0.0) + &gain_step(30.0) + &exposure(0.5, &image) + &light_request(0.5) + "@ close\n";
    let second = finder(0.0) + &exposure(0.5, &image) + &gain_step(30.0) + &exposure(0.5, &image);
    let scripts = Scripts::new(vec![first, second]);
    let mut rig = IndiFinder::discover(camera(&scripts, DEVICE), CameraMode::Auto)
        .await
        .unwrap();
    rig.apply(&Settings {
        exposure_ms: Some(500.0),
        gain: Some(30.0),
        ..Settings::default()
    })
    .unwrap();
    assert_eq!(rig.next_frame().await.unwrap().gain, 30.0);
    assert_eq!(
        rig.next_frame().await.unwrap_err().to_string(),
        "the finder camera exposed at gain 0, not at the gain 30 sent"
    );
    assert_eq!(rig.next_frame().await.unwrap().gain, 30.0);
    assert_eq!(scripts.connections(), 2);
    scripts.finish().await;
}

/// US1-S8: the imaging client's own description takes parsed answer
/// ranges; an unparsed maximum preserves the definition's 3600 seconds.
#[tokio::test(start_paused = true)]
async fn set_number_ranges_replace_only_the_attributes_that_parse() {
    for (script, expected) in [
        (
            finder(0.0),
            Range {
                min: 0.032,
                max: 2_000_000.0,
            },
        ),
        (
            finder(0.0).replace("max=\"2000\"", "max=\"x\""),
            Range {
                min: 0.032,
                max: 3_600_000.0,
            },
        ),
    ] {
        let scripts = Scripts::new(vec![script]);
        let imaging = camera(&scripts, DEVICE);
        assert_eq!(imaging.describe().await.unwrap().exposure_ms, expected);
        scripts.finish().await;
    }
}

/// The XML lines of the first send block, before the next directive.
fn first_block(script: &str) -> &str {
    script
        .split_once("@ send\n")
        .unwrap()
        .1
        .split_once("\n@ ")
        .unwrap()
        .0
}

/// US1-S9: both servers advertise both devices, but each connection must
/// name its own. Their 100 ms Busy and 1 s exposures complete concurrently.
#[tokio::test(start_paused = true)]
async fn named_finder_and_imaging_cameras_expose_concurrently() {
    // mono() appends its device directive to the final definition. Put it
    // on its own line so the stand-in checks the device on every request.
    let mono_script = mono().replace("@ device CCD Simulator", "\n@ device CCD Simulator");
    let colour_script = colour();
    let finder_script = mono_script.replacen(
        "@ send\n",
        &format!("@ send\n{}\n", first_block(&colour_script)),
        1,
    ) + "@ expect newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=1\n"
        + &answers(MONO_DEVICE, 1.0, &flat(7));
    let imaging_script = colour_script.replacen(
        "@ send\n",
        &format!("@ send\n{}\n", first_block(&mono_script)),
        1,
    ) + &light_request(1.0)
        + &answers(COLOUR_DEVICE, 1.0, &flat(9));
    let finder_scripts = Scripts::new(vec![finder_script]);
    let imaging_scripts = Scripts::new(vec![imaging_script]);
    let mut rig = IndiFinder::discover(camera(&finder_scripts, MONO_DEVICE), CameraMode::Auto)
        .await
        .unwrap();
    let imaging = camera(&imaging_scripts, COLOUR_DEVICE);
    let description = imaging.describe().await.unwrap();
    assert_eq!(rig.info().model, "CCD Simulator");
    assert_eq!(description.model, "ZWO CCD ASI071MC Pro");
    let request = Exposure {
        exposure_ms: 1000.0,
        purpose: "light".to_string(),
    };
    let start = tokio::time::Instant::now();
    let (finder_frame, imaging_frame) = tokio::join!(
        async {
            let frame = rig.next_frame().await.unwrap();
            (frame, start.elapsed())
        },
        async {
            let frame = imaging.expose(&request).await.unwrap();
            (frame, start.elapsed())
        },
    );
    assert_eq!(finder_frame.1, Duration::from_millis(1100));
    assert_eq!(imaging_frame.1, Duration::from_millis(1100));
    assert_eq!(finder_frame.0.pixels, [7; 6]);
    assert_eq!(imaging_frame.0.samples, [9; 6]);
    finder_scripts.finish().await;
    imaging_scripts.finish().await;
}

/// US1-S10: no gain control sends no gain vector and reports gain 1;
/// both ends of a wholly sub-unit gain range are raised to 1.
#[tokio::test(start_paused = true)]
async fn absent_and_sub_unit_gain_ranges_report_one_to_one() {
    let gain_element =
        "<defNumber name=\"Gain\" format=\"%g\" min=\"0\" max=\"100\" step=\"1\">0</defNumber>";
    let without_gain = finder(0.0).replace(gain_element, "");
    assert!(!without_gain.contains("name=\"Gain\""));
    let scripts = Scripts::new(vec![without_gain + &exposure(1.0, &worked_frame())]);
    let mut rig = IndiFinder::discover(camera(&scripts, DEVICE), CameraMode::Auto)
        .await
        .unwrap();
    assert_eq!((rig.info().gain_min, rig.info().gain_max), (1.0, 1.0));
    assert_eq!(rig.next_frame().await.unwrap().gain, 1.0);
    scripts.finish().await;

    let sub_unit = finder(0.0).replace(
        gain_element,
        "<defNumber name=\"Gain\" format=\"%g\" min=\"0\" max=\"0.5\" step=\"1\">0</defNumber>",
    );
    assert!(sub_unit.contains("max=\"0.5\""));
    let scripts = Scripts::new(vec![sub_unit]);
    let rig = IndiFinder::discover(camera(&scripts, DEVICE), CameraMode::Auto)
        .await
        .unwrap();
    assert_eq!((rig.info().gain_min, rig.info().gain_max), (1.0, 1.0));
    scripts.finish().await;
}
