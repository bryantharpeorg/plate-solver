//! US5's scenarios: `apply`'s vectors and answers, its refusals and timeouts,
//! and the neutral controls on each connection. Oracle: INDI's documents,
//! read through the hand-written scripts of the stand-in server, whose
//! expectations check what the adapter sent and when.
mod support;

use scope_node::{
    clock::NodeClock,
    imaging::{
        indi::{IndiCamera, NeutralControls},
        BayerPattern, ImagingCamera, ImagingError, ImagingInfo, ImagingSettings, ImagingStatus,
        Range, Roi,
    },
};
use std::{io, sync::Arc, time::Duration};
use support::{colour, set_number, set_switch, Scripts, COLOUR_DEVICE};

/// The test clock of the spec's shared setup: 1000.0 plus the tokio time
/// elapsed since it was built, so paused time advances it exactly. US5's
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

/// The camera of US4-S1, with the scenario's neutral controls: the
/// colour-camera script, a named device, an INDI server at 127.0.0.1:7624.
fn camera(scripts: &Scripts, neutral: NeutralControls) -> IndiCamera {
    IndiCamera::new(
        scripts.connect(),
        "127.0.0.1:7624".to_string(),
        Some(COLOUR_DEVICE.to_string()),
        neutral,
        clock(),
    )
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

/// The settings the camera reports, every value read from the script's
/// answers: the gain and binning the applies asked for, the offset and the
/// region as the camera answered them.
fn colour_settings(
    gain: f64,
    binning: u32,
    roi: Roi,
    cooler_on: bool,
    set_temp_c: Option<f64>,
) -> ImagingSettings {
    ImagingSettings {
        gain: Some(gain),
        offset: Some(20.0),
        binning: Some(binning),
        roi: Some(roi),
        cooler_on: Some(cooler_on),
        set_temp_c,
    }
}

/// US5-S1: three applies — gain and binning, the region, the cooler — send
/// their vectors in the plan's order, every element not asked for at the
/// value the device last reported, and answer the settings the camera
/// reports: the width the script's camera answered, the cooler on, the set
/// temperature remembered from a Busy answer.
#[tokio::test(start_paused = true)]
async fn the_applies_send_their_vectors_in_order_and_answer_the_camera() {
    let script = colour()
        + "@ expect newNumberVector CCD_BINNING HOR_BIN=2 VER_BIN=2\n\
           @ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_BINNING",
            "Ok",
            None,
            &[("HOR_BIN", 2.0), ("VER_BIN", 2.0)],
        )
        + "\n@ expect newNumberVector CCD_CONTROLS Gain=120 Offset=20 WB_R=52 WB_B=95\n\
           @ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_CONTROLS",
            "Ok",
            None,
            &[
                ("Gain", 120.0),
                ("Offset", 20.0),
                ("WB_R", 52.0),
                ("WB_B", 95.0),
            ],
        )
        + "\n@ expect newNumberVector CCD_FRAME X=1000 Y=800 WIDTH=1930 HEIGHT=1200\n\
           @ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_FRAME",
            "Ok",
            None,
            &[
                ("X", 1000.0),
                ("Y", 800.0),
                ("WIDTH", 1928.0),
                ("HEIGHT", 1200.0),
            ],
        )
        + "\n@ expect newSwitchVector CCD_COOLER COOLER_ON=On\n\
           @ send\n"
        + &set_switch(
            COLOUR_DEVICE,
            "CCD_COOLER",
            "Ok",
            None,
            &[("COOLER_ON", "On"), ("COOLER_OFF", "Off")],
        )
        + "\n@ expect newNumberVector CCD_TEMPERATURE CCD_TEMPERATURE_VALUE=-10\n\
           @ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_TEMPERATURE",
            "Busy",
            None,
            &[("CCD_TEMPERATURE_VALUE", 12.5)],
        )
        + "\n@ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_TEMPERATURE",
            "Ok",
            None,
            &[("CCD_TEMPERATURE_VALUE", 5.0)],
        )
        + "\n@ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_COOLER_POWER",
            "Ok",
            None,
            &[("CCD_COOLER_VALUE", 80.0)],
        )
        + "\n";
    let scripts = Scripts::new(vec![script]);
    let camera = camera(&scripts, Vec::new());
    assert_eq!(camera.describe().await.unwrap(), colour_info());

    // The first apply: binning first, then the gain, with every element of
    // `CCD_CONTROLS` at the value the device last reported.
    let first = camera
        .apply(&ImagingSettings {
            gain: Some(120.0),
            binning: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        first,
        colour_settings(
            120.0,
            2,
            Roi {
                x: 0,
                y: 0,
                width: 4944,
                height: 3284
            },
            false,
            None
        )
    );

    // The second: the region, answered with the width the camera answered.
    let second = camera
        .apply(&ImagingSettings {
            roi: Some(Roi {
                x: 1000,
                y: 800,
                width: 1930,
                height: 1200,
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        second,
        colour_settings(
            120.0,
            2,
            Roi {
                x: 1000,
                y: 800,
                width: 1928,
                height: 1200
            },
            false,
            None
        )
    );

    // The third: the cooler on, then the set temperature, remembered from
    // the camera's Busy answer.
    let third = camera
        .apply(&ImagingSettings {
            cooler_on: Some(true),
            set_temp_c: Some(-10.0),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        third,
        colour_settings(
            120.0,
            2,
            Roi {
                x: 1000,
                y: 800,
                width: 1928,
                height: 1200
            },
            true,
            Some(-10.0)
        )
    );

    // The script's later numbers reach the status: the sleep lets the
    // connection deliver them before it is read.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        camera.status().await.unwrap(),
        ImagingStatus {
            temp_c: Some(5.0),
            cooler_power: Some(80.0),
            set_temp_c: Some(-10.0),
            exposing: false,
        }
    );
    scripts.finish().await;
}

/// US5-S2: a gain outside the camera's range is `Invalid` with its text and
/// sends nothing — the script holds no expectation after `enableBLOB`, so
/// `finish` fails the test if the adapter sent one.
#[tokio::test(start_paused = true)]
async fn an_out_of_range_gain_is_invalid_and_sends_nothing() {
    let scripts = Scripts::new(vec![colour()]);
    let camera = camera(&scripts, Vec::new());
    assert_eq!(camera.describe().await.unwrap(), colour_info());
    match camera
        .apply(&ImagingSettings {
            gain: Some(300.0),
            ..Default::default()
        })
        .await
    {
        Err(ImagingError::Invalid(text)) => {
            assert_eq!(text, "gain 300 is outside the camera's range 0–240")
        }
        other => panic!("a gain out of range is Invalid, it answered {other:?}"),
    }
    scripts.finish().await;
}

/// US5-S2: an `Alert` answer fails the apply with the camera's own message.
#[tokio::test(start_paused = true)]
async fn an_alert_answer_fails_the_apply_with_the_camera_s_message() {
    let script = colour()
        + "@ expect newNumberVector CCD_BINNING HOR_BIN=3 VER_BIN=3\n\
           @ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_BINNING",
            "Alert",
            Some("Binning 3x3 is not supported."),
            &[("HOR_BIN", 1.0), ("VER_BIN", 1.0)],
        )
        + "\n";
    let scripts = Scripts::new(vec![script]);
    let camera = camera(&scripts, Vec::new());
    assert_eq!(camera.describe().await.unwrap(), colour_info());
    match camera
        .apply(&ImagingSettings {
            binning: Some(3),
            ..Default::default()
        })
        .await
    {
        Err(ImagingError::Failed(text)) => {
            assert_eq!(
                text,
                "the camera refused CCD_BINNING: Binning 3x3 is not supported."
            )
        }
        other => panic!("an Alert answer fails the apply, it answered {other:?}"),
    }
    scripts.finish().await;
}

/// US5-S2: a camera that never answers fails the apply exactly 5 s after the
/// call was made.
#[tokio::test(start_paused = true)]
async fn a_missing_answer_fails_the_apply_exactly_5s_late() {
    let script = colour() + "@ expect newNumberVector CCD_BINNING HOR_BIN=2 VER_BIN=2\n";
    let scripts = Scripts::new(vec![script]);
    let camera = camera(&scripts, Vec::new());
    assert_eq!(camera.describe().await.unwrap(), colour_info());
    let start = tokio::time::Instant::now();
    match camera
        .apply(&ImagingSettings {
            binning: Some(2),
            ..Default::default()
        })
        .await
    {
        Err(ImagingError::Failed(text)) => {
            assert_eq!(text, "the camera did not answer CCD_BINNING within 5 s")
        }
        other => panic!("a missing answer fails the apply, it answered {other:?}"),
    }
    assert_eq!(start.elapsed(), Duration::from_secs(5));
    scripts.finish().await;
}

/// US5-S5's first case's neutral controls: the white balance of plan.md's
/// worked example, as `ImagingConfig::neutral_controls` gives them.
fn white_balance() -> NeutralControls {
    vec![
        ("CCD_CONTROLS".to_string(), "WB_B".to_string(), 50.0),
        ("CCD_CONTROLS".to_string(), "WB_R".to_string(), 50.0),
    ]
}

/// The neutral vector the worked example asks for: every element of
/// `CCD_CONTROLS` in its defined order, the two named at 50 and `Gain` and
/// `Offset` at the device's reported values.
const NEUTRAL_EXPECT: &str =
    "@ expect newNumberVector CCD_CONTROLS Gain=90 Offset=20 WB_R=50 WB_B=50\n";

/// The script's answer to the neutral vector, in the state `Ok` with the same
/// four values, so they become the device's latest.
fn neutral_answer() -> String {
    "@ send\n".to_string()
        + &set_number(
            COLOUR_DEVICE,
            "CCD_CONTROLS",
            "Ok",
            None,
            &[
                ("Gain", 90.0),
                ("Offset", 20.0),
                ("WB_R", 50.0),
                ("WB_B", 50.0),
            ],
        )
        + "\n"
}

/// US5-S5: with neutral controls the adapter sends, after `enableBLOB`,
/// exactly one more element before `describe()` answers — one vector holding
/// every element of the property in its defined order, the named ones at
/// their values and the rest as reported — and a later apply keeps the
/// neutral values. A closed connection is set again by the next call, which
/// plays the second script to its end too.
#[tokio::test(start_paused = true)]
async fn the_neutral_controls_are_set_on_each_connection() {
    let neutral_part = colour() + NEUTRAL_EXPECT + &neutral_answer();
    // The first script ends after the apply's answer, closing the
    // connection; the second is played for the next one.
    let first = neutral_part.clone()
        + "@ expect newNumberVector CCD_CONTROLS Gain=120 Offset=20 WB_R=50 WB_B=50\n\
           @ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_CONTROLS",
            "Ok",
            None,
            &[
                ("Gain", 120.0),
                ("Offset", 20.0),
                ("WB_R", 50.0),
                ("WB_B", 50.0),
            ],
        )
        + "\n@ close\n";
    let second = neutral_part;
    let scripts = Scripts::new(vec![first, second]);
    let camera = camera(&scripts, white_balance());

    assert_eq!(camera.describe().await.unwrap(), colour_info());
    // The apply keeps the neutral values: every element at its latest value,
    // the white balance at the neutral 50 and 50.
    let applied = camera
        .apply(&ImagingSettings {
            gain: Some(120.0),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        applied,
        colour_settings(
            120.0,
            1,
            Roi {
                x: 0,
                y: 0,
                width: 4944,
                height: 3284
            },
            false,
            None
        )
    );
    // Let the script's @ close reach the adapter, so the next call connects
    // again and plays the second script to its end.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        camera.status().await.unwrap(),
        ImagingStatus {
            temp_c: Some(12.5),
            cooler_power: Some(0.0),
            set_temp_c: None,
            exposing: false,
        }
    );
    assert_eq!(scripts.connections(), 2);
    scripts.finish().await;
}

/// US5-S5: a control the camera does not define fails the connection with
/// its name, after `enableBLOB` and before any neutral vector is sent, and
/// every later call answers the same failure on a fresh connection: the
/// adapter sent `getProperties` twice and nothing after either `enableBLOB`,
/// which the scripts' own expectations prove.
#[tokio::test(start_paused = true)]
async fn a_control_the_camera_lacks_fails_the_connection() {
    let scripts = Scripts::new(vec![colour(), colour()]);
    let missing = camera(
        &scripts,
        vec![("CCD_CONTROLS".to_string(), "WB_G".to_string(), 50.0)],
    );
    for _ in 0..2 {
        match missing.describe().await {
            Err(ImagingError::Failed(text)) => {
                assert_eq!(text, "the camera has no control CCD_CONTROLS.WB_G")
            }
            other => panic!("a missing control fails the describe, it answered {other:?}"),
        }
    }
    assert_eq!(scripts.connections(), 2);
    scripts.finish().await;

    // A property the camera does not define at all, the same way.
    let scripts = Scripts::new(vec![colour()]);
    let no_such_property = camera(
        &scripts,
        vec![("CCD_WB".to_string(), "R".to_string(), 50.0)],
    );
    match no_such_property.describe().await {
        Err(ImagingError::Failed(text)) => assert_eq!(text, "the camera has no control CCD_WB.R"),
        other => panic!("a missing property fails the describe, it answered {other:?}"),
    }
    scripts.finish().await;
}

/// US5-S5: a camera that answers the neutral vector with `Alert` fails the
/// connection with the camera's message.
#[tokio::test(start_paused = true)]
async fn a_refused_neutral_control_fails_the_connection() {
    let script = colour()
        + NEUTRAL_EXPECT
        + "@ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_CONTROLS",
            "Alert",
            Some("no such control"),
            &[
                ("Gain", 90.0),
                ("Offset", 20.0),
                ("WB_R", 50.0),
                ("WB_B", 50.0),
            ],
        )
        + "\n";
    let scripts = Scripts::new(vec![script]);
    let camera = camera(&scripts, white_balance());
    match camera.describe().await {
        Err(ImagingError::Failed(text)) => {
            assert_eq!(text, "the camera refused CCD_CONTROLS: no such control")
        }
        other => panic!("a refused control fails the describe, it answered {other:?}"),
    }
    scripts.finish().await;
}

/// US4-S3's shape still holds for US5's adapter: a connector that answers an
/// I/O error fails the call with the server's address and the error's text.
#[tokio::test(start_paused = true)]
async fn an_unreachable_server_is_named_in_the_failure() {
    let connect: scope_node::imaging::indi::IndiConnect = Arc::new(|| {
        Box::pin(async {
            Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "the server is down",
            ))
        })
    });
    let unreachable = IndiCamera::new(
        connect,
        "127.0.0.1:7624".to_string(),
        Some(COLOUR_DEVICE.to_string()),
        Vec::new(),
        clock(),
    );
    match unreachable
        .apply(&ImagingSettings {
            gain: Some(120.0),
            ..Default::default()
        })
        .await
    {
        Err(ImagingError::Failed(text)) => assert!(
            text.starts_with("cannot reach the INDI server at 127.0.0.1:7624: "),
            "the failure names the server: {text}"
        ),
        other => panic!("an unreachable server fails the apply, it answered {other:?}"),
    }
}
