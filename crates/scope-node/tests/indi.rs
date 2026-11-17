//! US4's scenarios, one test each. Oracle: INDI's documents, read through the
//! two hand-written scripts of the stand-in server, whose expectations check
//! what the adapter sent, when, and naming which device.
mod support;

use scope_node::{
    clock::NodeClock,
    imaging::{
        indi::{IndiCamera, IndiConnect},
        BayerPattern, ImagingCamera, ImagingError, ImagingInfo, ImagingStatus, Range,
    },
};
use std::{io, sync::Arc, time::Duration};
use support::{colour, def_switch, def_text, mono, Scripts, COLOUR_DEVICE, MOUNT_DEVICE};

/// The test clock of the spec's shared setup: 1000.0 plus the tokio time
/// elapsed since it was built, so paused time advances it exactly. US4's
/// adapter does not read it; US6's exposures will.
struct Clock(tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        1000.0 + self.0.elapsed().as_secs_f64()
    }
}

fn clock() -> Arc<dyn NodeClock> {
    Arc::new(Clock(tokio::time::Instant::now()))
}

/// A camera over the stand-in server, with no neutral controls (US5's).
fn camera(scripts: &Scripts, device: Option<&str>) -> (Arc<dyn NodeClock>, IndiCamera, String) {
    let clock = clock();
    let address = "127.0.0.1:7624".to_string();
    let camera = IndiCamera::new(
        scripts.connect(),
        address.clone(),
        device.map(str::to_string),
        Vec::new(),
        clock.clone(),
    );
    (clock, camera, address)
}

/// The colour script's description, from plan.md § The two worked scripts:
/// every value read from the script's table, none from the camera's name.
fn colour_info() -> ImagingInfo {
    ImagingInfo {
        model: "ZWO CCD ASI071MC Pro".to_string(),
        serial: Some("0a1b2c3d4e5f6071".to_string()),
        width: 4944,
        height: 3284,
        pixel_um: 4.78,
        bits: 16,
        bayer: Some(BayerPattern::Rggb),
        exposure_ms: Range {
            min: 0.064,
            max: 2_000_000.0,
        },
        gain: Some(Range {
            min: 0.0,
            max: 240.0,
        }),
        offset: Some(Range {
            min: 0.0,
            max: 100.0,
        }),
        binnings: vec![1, 2, 3, 4],
        cooler: true,
    }
}

/// The mono script's description: no serial, no CFA, no binning property and
/// no cooler, the gain and offset of `CCD_GAIN` and `CCD_OFFSET`.
fn mono_info() -> ImagingInfo {
    ImagingInfo {
        model: "CCD Simulator".to_string(),
        serial: None,
        width: 1280,
        height: 1024,
        pixel_um: 5.2,
        bits: 16,
        bayer: None,
        exposure_ms: Range {
            min: 10.0,
            max: 3_600_000.0,
        },
        gain: Some(Range {
            min: 0.0,
            max: 300.0,
        }),
        offset: Some(Range {
            min: 0.0,
            max: 6000.0,
        }),
        binnings: vec![1],
        cooler: false,
    }
}

/// US4-S1: over the colour-camera script the adapter sends `getProperties`
/// for protocol 1.7, connects the device, and `enableBLOB Also` exactly
/// 200 ms after the script's last definition — each element checked by the
/// script itself, which `finish` fails the test through — and describes and
/// reports the camera exactly as the script's table says.
#[tokio::test(start_paused = true)]
async fn the_named_camera_is_connected_described_and_reported() {
    let scripts = Scripts::new(vec![colour()]);
    let (_, camera, _) = camera(&scripts, Some(COLOUR_DEVICE));
    assert_eq!(camera.describe().await.unwrap(), colour_info());
    assert_eq!(
        camera.status().await.unwrap(),
        ImagingStatus {
            temp_c: Some(12.5),
            cooler_power: Some(0.0),
            set_temp_c: None,
            exposing: false,
        }
    );
    scripts.finish().await;
}

/// US4-S2: with no device named the adapter picks the device whose
/// `DRIVER_INTERFACE` has the camera bit, skips `CONNECT` on a camera that
/// is already connected, and switches compression off and upload to the
/// client — every element after the first naming `CCD Simulator`, and no
/// `CONNECTION`, both checked by the script's expectations.
#[tokio::test(start_paused = true)]
async fn the_first_camera_offered_is_chosen_and_switched() {
    let scripts = Scripts::new(vec![mono()]);
    let (_, camera, _) = camera(&scripts, None);
    assert_eq!(camera.describe().await.unwrap(), mono_info());
    assert_eq!(scripts.connections(), 1);
    scripts.finish().await;
}

/// US4-S3: a connector that answers an I/O error fails every call with the
/// server's address and the error's text.
#[tokio::test(start_paused = true)]
async fn an_unreachable_server_is_named_in_the_failure() {
    let connect: IndiConnect = Arc::new(|| {
        Box::pin(async {
            Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "the server is down",
            ))
        })
    });
    let camera = IndiCamera::new(
        connect,
        "127.0.0.1:7624".to_string(),
        None,
        Vec::new(),
        clock(),
    );
    match camera.describe().await {
        Err(ImagingError::Failed(text)) => assert!(
            text.starts_with("cannot reach the INDI server at 127.0.0.1:7624: "),
            "the failure names the server: {text}"
        ),
        other => panic!("describe fails on an unreachable server, it answered {other:?}"),
    }
}

/// The colour-camera script's first part: its `CONNECTION` and `DRIVER_INFO`
/// alone, the lines a device sends before the adapter can choose.
fn colour_first_part() -> String {
    format!(
        "@ expect getProperties @version=1.7\n\
         @ send\n{}\n{}\n",
        def_switch(
            COLOUR_DEVICE,
            "CONNECTION",
            &[("CONNECT", "Off"), ("DISCONNECT", "On")]
        ),
        def_text(
            COLOUR_DEVICE,
            "DRIVER_INFO",
            &[
                ("DRIVER_NAME", "ZWO CCD"),
                ("DRIVER_EXEC", "indi_asi_ccd"),
                ("DRIVER_VERSION", "2.8"),
                ("DRIVER_INTERFACE", "2"),
            ],
        ),
    )
}

/// US4-S3: a device the server never names fails the first call exactly 5 s
/// after it was made, and a server with no camera at all fails it the same
/// way.
#[tokio::test(start_paused = true)]
async fn a_missing_device_or_camera_fails_the_first_call_exactly_5s_late() {
    let scripts = Scripts::new(vec![colour_first_part()]);
    let (_, missing, _) = camera(&scripts, Some("No Such Camera"));
    let start = tokio::time::Instant::now();
    match missing.describe().await {
        Err(ImagingError::Failed(text)) => assert_eq!(
            text,
            "the INDI server at 127.0.0.1:7624 has no device No Such Camera"
        ),
        other => panic!("the missing device fails the call, it answered {other:?}"),
    }
    assert_eq!(start.elapsed(), Duration::from_secs(5));
    scripts.finish().await;

    let mount_only = format!(
        "@ expect getProperties @version=1.7\n\
         @ send\n{}\n{}\n",
        def_switch(MOUNT_DEVICE, "CONNECTION", &[("CONNECT", "Off")]),
        def_text(MOUNT_DEVICE, "DRIVER_INFO", &[("DRIVER_INTERFACE", "5")]),
    );
    let scripts = Scripts::new(vec![mount_only]);
    let (_, mount, _) = camera(&scripts, None);
    let start = tokio::time::Instant::now();
    match mount.describe().await {
        Err(ImagingError::Failed(text)) => {
            assert_eq!(text, "the INDI server at 127.0.0.1:7624 offers no camera")
        }
        other => panic!("the mount alone fails the call, it answered {other:?}"),
    }
    assert_eq!(start.elapsed(), Duration::from_secs(5));
    scripts.finish().await;
}

/// US4-S3: a `CCD_BITSPERPIXEL` other than 8 or 16 fails the describe.
#[tokio::test(start_paused = true)]
async fn an_unsupported_sample_size_fails_the_describe() {
    // The colour script with the sample size 12 in place of 16, the one
    // value the worked table gives in `CCD_INFO`'s last element.
    let twelve_bits = colour().replace(
        "name=\"CCD_BITSPERPIXEL\" format=\"%g\" min=\"16\" max=\"16\" step=\"1\">16",
        "name=\"CCD_BITSPERPIXEL\" format=\"%g\" min=\"12\" max=\"12\" step=\"1\">12",
    );
    assert!(
        twelve_bits.contains(">12</defNumber>"),
        "the worked script writes the sample size in one place only"
    );
    let scripts = Scripts::new(vec![twelve_bits]);
    let (_, camera, _) = camera(&scripts, Some(COLOUR_DEVICE));
    match camera.describe().await {
        Err(ImagingError::Failed(text)) => {
            assert_eq!(text, "unsupported sample size 12 bits")
        }
        other => panic!("a 12-bit camera is refused, it answered {other:?}"),
    }
    scripts.finish().await;
}

/// US4-S3: a lost connection is made again by the next call, which answers
/// from the new connection, and the adapter sent `getProperties` twice.
#[tokio::test(start_paused = true)]
async fn a_lost_connection_is_made_again_by_the_next_call() {
    // The second script answers a warmer camera, so a status read from the
    // first connection could not pass.
    let second = colour().replace(
        "name=\"CCD_TEMPERATURE_VALUE\" format=\"%g\" min=\"-50\" max=\"50\" step=\"1\">12.5",
        "name=\"CCD_TEMPERATURE_VALUE\" format=\"%g\" min=\"-50\" max=\"50\" step=\"1\">20.5",
    );
    assert!(second.contains(">20.5</defNumber>"));
    let first = colour() + "@ close\n";
    let scripts = Scripts::new(vec![first, second]);
    let (_, camera, _) = camera(&scripts, Some(COLOUR_DEVICE));
    assert_eq!(camera.describe().await.unwrap(), colour_info());
    assert_eq!(
        camera.status().await.unwrap(),
        ImagingStatus {
            temp_c: Some(20.5),
            cooler_power: Some(0.0),
            set_temp_c: None,
            exposing: false,
        }
    );
    assert_eq!(scripts.connections(), 2);
    scripts.finish().await;
}
