//! US6's scenarios, one test each. Oracle: INDI's documents, read through the
//! two hand-written scripts of the stand-in server (whose expectations check
//! what the adapter sent, and when), the worked FITS frames of plan.md §
//! Reading a FITS BLOB, and the saved capture with the values a separate
//! program read from it (US6-S8).

mod support;

use scope_node::{
    clock::NodeClock,
    imaging::{
        indi::{IndiCamera, IndiConnect},
        BayerPattern, Exposure, ImagingCamera, ImagingError, ImagingFrame, ImagingSettings, Roi,
    },
};
use std::{io, sync::Arc, task::Poll, time::Duration};
use support::{colour, fixture, mono, set_blob, set_number, Scripts, COLOUR_DEVICE};

/// The test clock of the spec's shared setup: 1000.0 plus the tokio time
/// elapsed since it was built, so paused time advances it exactly. The
/// adapter reads it for the exposure's times.
struct Clock(tokio::time::Instant);
impl NodeClock for Clock {
    fn now(&self) -> f64 {
        1000.0 + self.0.elapsed().as_secs_f64()
    }
}

fn clock() -> Arc<dyn NodeClock> {
    Arc::new(Clock(tokio::time::Instant::now()))
}

/// The test clock's reading `ms` milliseconds after it was built: the same
/// sum the clock itself makes, so the expected times are exact.
fn reading(ms: u64) -> f64 {
    1000.0 + Duration::from_millis(ms).as_secs_f64()
}

/// A camera over the stand-in server, named for the colour script's device,
/// with no neutral controls.
fn colour_camera(scripts: &Scripts) -> (Arc<dyn NodeClock>, IndiCamera) {
    let clock = clock();
    let camera = IndiCamera::new(
        scripts.connect(),
        "127.0.0.1:7624".to_string(),
        Some(COLOUR_DEVICE.to_string()),
        Vec::new(),
        clock.clone(),
    );
    (clock, camera)
}

/// Poll an exposure once, so its request is sent in the instant it is made:
/// the adapter sends nothing until its method is first polled, and the
/// scenarios time the call itself.
async fn begin(expose: &mut scope_node::imaging::ImagingFuture<'_, ImagingFrame>) {
    std::future::poll_fn(|cx| match expose.as_mut().poll(cx) {
        Poll::Pending => Poll::Ready(()),
        Poll::Ready(answer) => panic!("the exposure answered at once: {answer:?}"),
    })
    .await;
}

// The tests' own FITS builder (plan.md § Plan traps 6): the worked frames of
// plan.md § Reading a FITS BLOB, never through scope_node::imaging::fits.

/// One 80-byte card: the keyword in eight columns, `= ` in nine and ten, a
/// number right-justified to column 30, a text quoted from column 11 and
/// padded inside the quotes to eight characters.
fn card(keyword: &str, value: &str) -> [u8; 80] {
    let mut out = [b' '; 80];
    let head = format!("{keyword:<8}= ");
    out[..head.len()].copy_from_slice(head.as_bytes());
    let tail = match value
        .strip_prefix('\'')
        .and_then(|text| text.strip_suffix('\''))
    {
        Some(text) => format!("'{text:<8}'"),
        None => format!("{value:>width$}", value = value, width = 80 - head.len()),
    };
    out[head.len()..head.len() + tail.len()].copy_from_slice(tail.as_bytes());
    out
}

/// A whole FITS image: the cards, `END`, the header padded to 2880 bytes,
/// then the data padded to the same.
fn fits(cards: &[[u8; 80]], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for card in cards {
        out.extend_from_slice(card);
    }
    let mut end = [b' '; 80];
    end[..3].copy_from_slice(b"END");
    out.extend_from_slice(&end);
    while out.len() % 2880 != 0 {
        out.push(b' ');
    }
    out.extend_from_slice(data);
    while out.len() % 2880 != 0 {
        out.push(0);
    }
    out
}

/// The worked FITS frame of plan.md § Reading a FITS BLOB: 4 × 2, `BITPIX`
/// 16, `BZERO` 32768, `BAYERPAT` RGGB, binning 1, −9.5 °C, the stored
/// samples `8000 8001 8002 8003 0000 1C40 7FFE 7FFF`.
fn worked_frame() -> Vec<u8> {
    fits(
        &[
            card("SIMPLE", "T"),
            card("BITPIX", "16"),
            card("NAXIS", "2"),
            card("NAXIS1", "4"),
            card("NAXIS2", "2"),
            card("BZERO", "32768"),
            card("BSCALE", "1"),
            card("XBINNING", "1"),
            card("YBINNING", "1"),
            card("CCD-TEMP", "-9.5"),
            card("BAYERPAT", "'RGGB'"),
        ],
        &[
            0x80, 0x00, 0x80, 0x01, 0x80, 0x02, 0x80, 0x03, 0x00, 0x00, 0x1C, 0x40, 0x7F, 0xFE,
            0x7F, 0xFF,
        ],
    )
}

/// The 3 × 2, 8-bit mono picture of US6-S2: the bytes as they are.
fn mono_frame() -> Vec<u8> {
    fits(
        &[
            card("SIMPLE", "T"),
            card("BITPIX", "8"),
            card("NAXIS", "2"),
            card("NAXIS1", "3"),
            card("NAXIS2", "2"),
        ],
        &[0x00, 0x01, 0x02, 0xFD, 0xFE, 0xFF],
    )
}

/// The standard base64 of a FITS image, the BLOB's text.
fn blob(fits: &[u8]) -> String {
    use base64::prelude::{Engine as _, BASE64_STANDARD};
    BASE64_STANDARD.encode(fits)
}

/// The answer the scenario's script gives to an armed exposure: `Busy` 200
/// ms after the request, the BLOB 500 ms after the `Busy`, then
/// `CCD_EXPOSURE` `Ok` — the spacing of US6-S1's answers.
fn frame_answers(fits: &[u8]) -> String {
    let blob = blob(fits);
    "\n@ send after 200\n".to_string()
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Busy",
            None,
            &[("CCD_EXPOSURE_VALUE", 0.5)],
        )
        + "\n@ send after 500\n"
        + &set_blob(
            COLOUR_DEVICE,
            "CCD1",
            "Ok",
            None,
            &[("CCD1", ".fits", blob.as_str())],
        )
        + "\n@ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Ok",
            None,
            &[("CCD_EXPOSURE_VALUE", 0.0)],
        )
        + "\n"
}

/// The exposure of 500 ms with the purpose `frame`, timed by the script's
/// `@ expect after 1000`: the call is made 1 s after `describe()` returned
/// (plan.md § Plan traps 20).
const LIGHT_500: &str = "@ expect after 1000 newSwitchVector CCD_FRAME_TYPE FRAME_LIGHT=On\n\
         @ expect after 0 newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.5\n";

/// The frame of the worked FITS BLOB, every field as US6-S1 states it: the
/// samples as numbers 0, 1, 2, 3, 32768, 40000, 65534, 65535 and as the
/// bytes of the little-endian answer.
fn worked_image(start_s: f64) -> ImagingFrame {
    ImagingFrame {
        samples: [0u16, 1, 2, 3, 32768, 40000, 65534, 65535]
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect(),
        width: 4,
        height: 2,
        bits: 16,
        bayer: Some(BayerPattern::Rggb),
        start_s,
        end_s: start_s + 0.5,
        exposure_ms: 500.0,
        gain: Some(90.0),
        offset: Some(20.0),
        binning: 1,
        temp_c: Some(-9.5),
        roi: Roi {
            x: 0,
            y: 0,
            width: 4944,
            height: 3284,
        },
    }
}

/// US6-S1: an exposure sets `CCD_FRAME_TYPE` and `CCD_EXPOSURE` in the
/// instant of the call, times `start_s` at the first of the Busy answers on
/// the node clock, and answers the worked frame with the region the camera
/// reported when the exposure was asked for, 700 ms after the call.
#[tokio::test(start_paused = true)]
async fn a_worked_exposure_answers_the_worked_frame() {
    let script = colour() + LIGHT_500
        // `Busy` 200 ms after the call, again 300 ms after that, the BLOB
        // 500 ms after the first `Busy`, then `Ok`.
        + "\n@ send after 200\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Busy",
            None,
            &[("CCD_EXPOSURE_VALUE", 0.5)],
        )
        + "\n@ send after 300\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Busy",
            None,
            &[("CCD_EXPOSURE_VALUE", 0.2)],
        )
        + "\n@ send after 200\n"
        + &set_blob(
            COLOUR_DEVICE,
            "CCD1",
            "Ok",
            None,
            &[("CCD1", ".fits", blob(&worked_frame()).as_str())],
        )
        + "\n@ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Ok",
            None,
            &[("CCD_EXPOSURE_VALUE", 0.0)],
        )
        + "\n";
    let scripts = Scripts::new(vec![script]);
    let (_, camera) = colour_camera(&scripts);
    camera.describe().await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let start = tokio::time::Instant::now();
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "frame".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(start.elapsed(), Duration::from_millis(700));
    // `start_s` is the clock's reading 200 ms after the call, `end_s` half
    // a second later.
    assert_eq!(frame, worked_image(reading(1400)));
    assert_eq!(scripts.connections(), 1);
    scripts.finish().await;
}

/// US6-S2: the purposes `dark` and `flat` choose their own frame types.
#[tokio::test(start_paused = true)]
async fn dark_and_flat_choose_their_frame_types() {
    let script = colour()
        + "@ expect newSwitchVector CCD_FRAME_TYPE FRAME_DARK=On\n\
           @ expect newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.5\n"
        + &frame_answers(&worked_frame())
        + "@ expect newSwitchVector CCD_FRAME_TYPE FRAME_FLAT=On\n\
           @ expect newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.5\n"
        + &frame_answers(&worked_frame());
    let scripts = Scripts::new(vec![script]);
    let (_, camera) = colour_camera(&scripts);
    camera.describe().await.unwrap();
    let dark = camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "dark".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(dark.width, 4);
    let flat = camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "flat".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(flat.width, 4);
    scripts.finish().await;
}

/// US6-S2: over the mono camera, whose BLOB is 3 × 2 with `BITPIX` 8, the
/// frame passes through with the gain and offset of `CCD_GAIN` and
/// `CCD_OFFSET` and the region of its `CCD_FRAME`, and nothing else: 6
/// bytes, the same bytes.
#[tokio::test(start_paused = true)]
async fn an_eight_bit_mono_frame_passes_through() {
    let script = mono()
        + "@ expect after 1000 newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.1\n\
           @ send after 200\n"
        + &set_number(
            support::MONO_DEVICE,
            "CCD_EXPOSURE",
            "Busy",
            None,
            &[("CCD_EXPOSURE_VALUE", 0.1)],
        )
        + "\n@ send after 200\n"
        + &set_blob(
            support::MONO_DEVICE,
            "CCD1",
            "Ok",
            None,
            &[("CCD1", ".fits", blob(&mono_frame()).as_str())],
        )
        + "\n@ send\n"
        + &set_number(
            support::MONO_DEVICE,
            "CCD_EXPOSURE",
            "Ok",
            None,
            &[("CCD_EXPOSURE_VALUE", 0.0)],
        )
        + "\n";
    let scripts = Scripts::new(vec![script]);
    let (_, camera) = camera_over(&scripts, None);
    camera.describe().await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 100.0,
            purpose: "frame".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(
        frame,
        ImagingFrame {
            samples: vec![0x00, 0x01, 0x02, 0xFD, 0xFE, 0xFF],
            width: 3,
            height: 2,
            bits: 8,
            bayer: None,
            start_s: reading(1400),
            end_s: reading(1400) + 0.1,
            exposure_ms: 100.0,
            gain: Some(90.0),
            offset: Some(0.0),
            binning: 1,
            temp_c: None,
            roi: Roi {
                x: 0,
                y: 0,
                width: 1280,
                height: 1024,
            },
        }
    );
    scripts.finish().await;
}

/// A camera over the stand-in server, the device named or picked by the
/// adapter's rule.
fn camera_over(scripts: &Scripts, device: Option<&str>) -> (Arc<dyn NodeClock>, IndiCamera) {
    let clock = clock();
    let camera = IndiCamera::new(
        scripts.connect(),
        "127.0.0.1:7624".to_string(),
        device.map(str::to_string),
        Vec::new(),
        clock.clone(),
    );
    (clock, camera)
}

/// US6-S2: the frame's `roi` is the device's reported region — the width
/// the camera answered, not the one asked for — and a region another client
/// sets while the exposure runs changes nothing.
#[tokio::test(start_paused = true)]
async fn the_roi_is_the_region_reported_when_the_exposure_was_asked_for() {
    // The region of US5-S1, answered with `WIDTH` 1928.
    let applied = colour()
        + "@ expect newNumberVector CCD_FRAME X=1000 Y=800 WIDTH=1930 HEIGHT=1200\n\
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
        + "\n"
        + LIGHT_500
        + &frame_answers(&worked_frame());
    let scripts = Scripts::new(vec![applied]);
    let (_, camera) = colour_camera(&scripts);
    camera.describe().await.unwrap();
    let answered = camera
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
        answered.roi,
        Some(Roi {
            x: 1000,
            y: 800,
            width: 1928,
            height: 1200,
        })
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "frame".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(
        frame.roi,
        Roi {
            x: 1000,
            y: 800,
            width: 1928,
            height: 1200,
        }
    );
    scripts.finish().await;

    // A region another client sets between the `Busy` and the BLOB changes
    // nothing: the exposure keeps the region it was asked for.
    let during = colour()
        + LIGHT_500
        + "\n@ send after 100\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Busy",
            None,
            &[("CCD_EXPOSURE_VALUE", 0.5)],
        )
        + "\n@ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_FRAME",
            "Ok",
            None,
            &[("X", 8.0), ("Y", 0.0), ("WIDTH", 64.0), ("HEIGHT", 64.0)],
        )
        + "\n@ send after 100\n"
        + &set_blob(
            COLOUR_DEVICE,
            "CCD1",
            "Ok",
            None,
            &[("CCD1", ".fits", blob(&worked_frame()).as_str())],
        )
        + "\n@ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Ok",
            None,
            &[("CCD_EXPOSURE_VALUE", 0.0)],
        )
        + "\n";
    let scripts = Scripts::new(vec![during]);
    let (_, camera) = colour_camera(&scripts);
    camera.describe().await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "frame".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(
        frame.roi,
        Roi {
            x: 0,
            y: 0,
            width: 4944,
            height: 3284,
        }
    );
    scripts.finish().await;
}

/// US6-S2: a colour frame whose FITS has no `BAYERPAT` and `XBINNING` 2, on
/// a camera whose `HOR_BIN` is 1, is RGGB from `CCD_CFA` with binning 2
/// from the header.
#[tokio::test(start_paused = true)]
async fn the_header_wins_over_the_device_s_properties() {
    let header_wins = fits(
        &[
            card("SIMPLE", "T"),
            card("BITPIX", "16"),
            card("NAXIS", "2"),
            card("NAXIS1", "4"),
            card("NAXIS2", "2"),
            card("BZERO", "32768"),
            card("BSCALE", "1"),
            card("XBINNING", "2"),
        ],
        &[
            0x80, 0x00, 0x80, 0x01, 0x80, 0x02, 0x80, 0x03, 0x00, 0x00, 0x1C, 0x40, 0x7F, 0xFE,
            0x7F, 0xFF,
        ],
    );
    let script = colour() + LIGHT_500 + &frame_answers(&header_wins);
    let scripts = Scripts::new(vec![script]);
    let (_, camera) = colour_camera(&scripts);
    camera.describe().await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "frame".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(frame.bayer, Some(BayerPattern::Rggb));
    assert_eq!(frame.binning, 2);
    // No `CCD-TEMP` in the header: the device's `CCD_TEMPERATURE` answers.
    assert_eq!(frame.temp_c, Some(12.5));
    scripts.finish().await;
}

/// US6-S3: the first `CCD_EXPOSURE` answers `Alert` with the camera's
/// message; the adapter sends the abort at once, and exactly 1 s later the
/// frame type and the exposure again, and the second try answers the frame
/// of US6-S1 1700 ms after the call.
#[tokio::test(start_paused = true)]
async fn an_alert_is_aborted_and_tried_once_more() {
    let script = colour()
        + LIGHT_500
        + "\n@ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Alert",
            Some("Exposure failed after 3 attempts."),
            &[("CCD_EXPOSURE_VALUE", 0.5)],
        )
        + "\n@ expect after 0 newSwitchVector CCD_ABORT_EXPOSURE ABORT=On\n\
           @ expect after 1000 newSwitchVector CCD_FRAME_TYPE FRAME_LIGHT=On\n\
           @ expect after 0 newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.5\n"
        + &frame_answers(&worked_frame());
    let scripts = Scripts::new(vec![script]);
    let (_, camera) = colour_camera(&scripts);
    camera.describe().await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let start = tokio::time::Instant::now();
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "frame".to_string(),
        })
        .await
        .unwrap();
    // 1000 of pause, 200 to `Busy`, 500 to the BLOB.
    assert_eq!(start.elapsed(), Duration::from_millis(1700));
    // The retry's `Busy` answers 1200 ms after the call.
    assert_eq!(frame, worked_image(reading(2400)));
    assert_eq!(scripts.connections(), 1);
    scripts.finish().await;
}

/// US6-S3: two `Alert`s answer `Failed` with the camera's message in the
/// failed-twice words, 1000 ms after the call, and the adapter sent the
/// five elements and one more abort.
#[tokio::test(start_paused = true)]
async fn two_alerts_fail_the_exposure() {
    let script = colour()
        + LIGHT_500
        + "\n@ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Alert",
            Some("Exposure failed after 3 attempts."),
            &[("CCD_EXPOSURE_VALUE", 0.5)],
        )
        + "\n@ expect after 0 newSwitchVector CCD_ABORT_EXPOSURE ABORT=On\n\
           @ expect after 1000 newSwitchVector CCD_FRAME_TYPE FRAME_LIGHT=On\n\
           @ expect after 0 newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.5\n\
           @ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Alert",
            Some("Exposure failed after 3 attempts."),
            &[("CCD_EXPOSURE_VALUE", 0.5)],
        )
        + "\n@ expect after 0 newSwitchVector CCD_ABORT_EXPOSURE ABORT=On\n";
    let scripts = Scripts::new(vec![script]);
    let (_, camera) = colour_camera(&scripts);
    camera.describe().await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let start = tokio::time::Instant::now();
    match camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "frame".to_string(),
        })
        .await
    {
        Err(ImagingError::Failed(text)) => assert_eq!(
            text,
            "the exposure failed twice: the camera reported a failed exposure: Exposure failed after 3 attempts."
        ),
        other => panic!("two Alerts fail the exposure, it answered {other:?}"),
    }
    assert_eq!(start.elapsed(), Duration::from_millis(1000));
    scripts.finish().await;
}

/// US6-S4: no frame within the exposure plus 60 s of the request, twice:
/// the aborts and the second try at the times the script states, counted
/// from the request and not from `Busy`, `Failed` exactly 122 s after the
/// call, and a `status()` 30 s in answered in that instant with `exposing`
/// true.
#[tokio::test(start_paused = true)]
async fn a_missing_frame_times_out_twice() {
    let script = colour()
        + LIGHT_500
        + "\n@ send after 300\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Busy",
            None,
            &[("CCD_EXPOSURE_VALUE", 0.5)],
        )
        // 60,200 ms after the `Busy`: the wait is counted from the request.
        + "\n@ expect after 60200 newSwitchVector CCD_ABORT_EXPOSURE ABORT=On\n\
           @ expect after 1000 newSwitchVector CCD_FRAME_TYPE FRAME_LIGHT=On\n\
           @ expect after 0 newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.5\n\
           @ send after 300\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Busy",
            None,
            &[("CCD_EXPOSURE_VALUE", 0.5)],
        )
        + "\n@ expect after 60200 newSwitchVector CCD_ABORT_EXPOSURE ABORT=On\n";
    let scripts = Scripts::new(vec![script]);
    let (_, camera) = colour_camera(&scripts);
    camera.describe().await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let call = tokio::time::Instant::now();
    let request = Exposure {
        exposure_ms: 500.0,
        purpose: "frame".to_string(),
    };
    let mut expose = camera.expose(&request);
    begin(&mut expose).await;
    // 30 s into the call, `status` answers in that instant.
    tokio::time::sleep(Duration::from_secs(30)).await;
    let asked = tokio::time::Instant::now();
    assert!(
        camera.status().await.unwrap().exposing,
        "the camera is exposing 30 s into the call"
    );
    assert_eq!(asked.elapsed(), Duration::ZERO);
    match expose.await {
        Err(ImagingError::Failed(text)) => assert_eq!(
            text,
            "the exposure failed twice: no imaging frame arrived within 60 s of the exposure's end"
        ),
        other => panic!("a missing frame fails the exposure, it answered {other:?}"),
    }
    assert_eq!(call.elapsed(), Duration::from_secs(122));
    assert_eq!(scripts.connections(), 1);
    scripts.finish().await;
}

/// US6-S5: a connection that closes 100 ms after `CCD_EXPOSURE` is lost, and
/// the second try is made on a new one and answers the frame.
#[tokio::test(start_paused = true)]
async fn a_closed_connection_is_retried_on_a_new_one() {
    let closing = colour()
        + LIGHT_500
        // An empty `@ send` only waits: `@ close` then closes 100 ms after
        // the exposure was asked for.
        + "\n@ send after 100\n@ close\n";
    let again = colour()
        + "@ expect after 0 newSwitchVector CCD_FRAME_TYPE FRAME_LIGHT=On\n\
           @ expect after 0 newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.5\n"
        + &frame_answers(&worked_frame());
    let scripts = Scripts::new(vec![closing, again]);
    let (_, camera) = colour_camera(&scripts);
    camera.describe().await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let start = tokio::time::Instant::now();
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "frame".to_string(),
        })
        .await
        .unwrap();
    // 100 to the close, 1000 of pause, 200 of settle, 200 to `Busy`, 500
    // to the BLOB.
    assert_eq!(start.elapsed(), Duration::from_millis(2000));
    // The retry's `Busy` answers 1500 ms after the call.
    assert_eq!(frame, worked_image(reading(2700)));
    assert_eq!(scripts.connections(), 2);
    scripts.finish().await;
}

/// US6-S5: two closed connections answer `Failed` with the loss in the
/// failed-twice words.
#[tokio::test(start_paused = true)]
async fn two_closed_connections_fail_the_exposure() {
    let closing = colour() + LIGHT_500 + "\n@ send after 100\n@ close\n";
    let closing_again = colour()
        + "@ expect after 0 newSwitchVector CCD_FRAME_TYPE FRAME_LIGHT=On\n\
           @ expect after 0 newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.5\n\
           @ send after 100\n@ close\n";
    let scripts = Scripts::new(vec![closing, closing_again]);
    let (_, camera) = colour_camera(&scripts);
    camera.describe().await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    match camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "frame".to_string(),
        })
        .await
    {
        Err(ImagingError::Failed(text)) => assert_eq!(
            text,
            "the exposure failed twice: the INDI server closed the connection"
        ),
        other => panic!("two closed connections fail the exposure, it answered {other:?}"),
    }
    assert_eq!(scripts.connections(), 2);
    scripts.finish().await;
}

/// US6-S6: while an exposure waits, a second `expose` and an `apply` that
/// names a setting are `Busy` and send nothing, an empty `apply` answers
/// the settings in force, `abort` sends the abort switch at the time the
/// script states and ends the exposure with `Aborted` in the instant it was
/// called, a late BLOB is discarded, and a following exposure works.
#[tokio::test(start_paused = true)]
async fn abort_ends_the_exposure_and_the_rules_of_one_in_flight_hold() {
    let script = colour()
        + "@ expect after 1000 newSwitchVector CCD_FRAME_TYPE FRAME_LIGHT=On\n\
           @ expect after 0 newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=30\n\
           @ send\n"
        + &set_number(
            COLOUR_DEVICE,
            "CCD_EXPOSURE",
            "Busy",
            None,
            &[("CCD_EXPOSURE_VALUE", 30.0)],
        )
        // The abort, exactly 2000 ms after the `Busy` was sent.
        + "\n@ expect after 2000 newSwitchVector CCD_ABORT_EXPOSURE ABORT=On\n\
           @ send after 1000\n"
        // A BLOB of another picture, discarded: nothing waits for it.
        + &set_blob(
            COLOUR_DEVICE,
            "CCD1",
            "Ok",
            None,
            &[("CCD1", ".fits", blob(&mono_frame()).as_str())],
        )
        + "\n@ expect after 1000 newSwitchVector CCD_FRAME_TYPE FRAME_LIGHT=On\n\
           @ expect after 0 newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=0.1\n"
        + &frame_answers(&worked_frame());
    let scripts = Scripts::new(vec![script]);
    let (_, camera) = colour_camera(&scripts);
    camera.describe().await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;

    // The 30 s exposure, in flight from here.
    let exposure = Exposure {
        exposure_ms: 30_000.0,
        purpose: "frame".to_string(),
    };
    let mut first = camera.expose(&exposure);
    begin(&mut first).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // A second exposure is `Busy` and sends nothing.
    match camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "frame".to_string(),
        })
        .await
    {
        Err(ImagingError::Busy) => {}
        other => panic!("a second expose is Busy, it answered {other:?}"),
    }
    // An `apply` that names a setting is `Busy` before any check.
    match camera
        .apply(&ImagingSettings {
            gain: Some(100.0),
            ..Default::default()
        })
        .await
    {
        Err(ImagingError::Busy) => {}
        other => panic!("an apply that names a setting is Busy, it answered {other:?}"),
    }
    // An empty `apply` answers the settings in force and sends nothing.
    assert_eq!(
        camera.apply(&ImagingSettings::default()).await.unwrap(),
        ImagingSettings {
            gain: Some(90.0),
            offset: Some(20.0),
            binning: Some(1),
            roi: Some(Roi {
                x: 0,
                y: 0,
                width: 4944,
                height: 3284,
            }),
            cooler_on: Some(false),
            set_temp_c: None,
        }
    );
    // `abort` ends the exposure with `Aborted` in the instant it is called.
    let abort_at = tokio::time::Instant::now();
    camera.abort().await.unwrap();
    let answered = tokio::time::Instant::now();
    assert_eq!(answered, abort_at);
    match first.await {
        Err(ImagingError::Aborted) => {}
        other => panic!("the aborted exposure answers Aborted, it answered {other:?}"),
    }
    assert_eq!(tokio::time::Instant::now(), answered);

    // The BLOB of another picture, 1 s later, is discarded; an exposure
    // made after it has come answers the worked frame, not that one.
    tokio::time::sleep(Duration::from_secs(2)).await;
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 100.0,
            purpose: "frame".to_string(),
        })
        .await
        .unwrap();
    assert_eq!((frame.width, frame.height, frame.bits), (4, 2, 16));
    scripts.finish().await;
}

/// US6-S7: a BLOB the adapter cannot read fails the exposure at once, with
/// its own text, no abort and no second try — each script holds no
/// expectation after the BLOB, and `finish` fails if the adapter sent one.
#[tokio::test(start_paused = true)]
async fn blobs_the_camera_cannot_read_fail_at_once() {
    // A 3-axis header.
    let three_axes = fits(
        &[
            card("SIMPLE", "T"),
            card("BITPIX", "16"),
            card("NAXIS", "3"),
            card("NAXIS1", "4"),
            card("NAXIS2", "2"),
            card("NAXIS3", "2"),
        ],
        &[],
    );
    // A float header.
    let floating = fits(
        &[
            card("SIMPLE", "T"),
            card("BITPIX", "-32"),
            card("NAXIS", "2"),
            card("NAXIS1", "4"),
            card("NAXIS2", "2"),
        ],
        &[],
    );
    // A short one: the header promises 64 × 48 and the data is 16 bytes.
    let mut short = fits(
        &[
            card("SIMPLE", "T"),
            card("BITPIX", "16"),
            card("NAXIS", "2"),
            card("NAXIS1", "64"),
            card("NAXIS2", "48"),
            card("BZERO", "32768"),
        ],
        &[
            0x80, 0x00, 0x80, 0x01, 0x80, 0x02, 0x80, 0x03, 0x00, 0x00, 0x1C, 0x40, 0x7F, 0xFE,
            0x7F, 0xFF,
        ],
    );
    // The data block is padded to 2880; the header says 6144 bytes.
    short.truncate(short.len() - 1000);
    let cases: [(Vec<u8>, &str, &str); 4] = [
        (
            worked_frame(),
            ".fits.fz",
            "the camera sent .fits.fz, not .fits",
        ),
        (three_axes, ".fits", "the camera sent a 3-axis FITS picture"),
        (floating, ".fits", "unsupported FITS BITPIX -32"),
        (short, ".fits", "the camera's FITS data is short"),
    ];
    for (picture, format, text) in cases {
        let script = colour()
            + LIGHT_500
            // The BLOB at once, in the instant of the call.
            + "\n@ send\n"
            + &set_blob(
                COLOUR_DEVICE,
                "CCD1",
                "Ok",
                None,
                &[("CCD1", format, blob(&picture).as_str())],
            )
            + "\n";
        let scripts = Scripts::new(vec![script]);
        let (_, camera) = colour_camera(&scripts);
        camera.describe().await.unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
        let start = tokio::time::Instant::now();
        match camera
            .expose(&Exposure {
                exposure_ms: 500.0,
                purpose: "frame".to_string(),
            })
            .await
        {
            Err(ImagingError::Failed(answer)) => assert_eq!(answer, text),
            other => panic!("a BLOB of {format} fails the exposure, it answered {other:?}"),
        }
        assert_eq!(start.elapsed(), Duration::ZERO, "no second try");
        assert_eq!(scripts.connections(), 1);
        scripts.finish().await;
    }
}

/// US6-S8: the saved capture of INDI's camera simulator plays to its end,
/// and the description and the frame equal the values a separate program
/// read from it when it was recorded.
#[tokio::test(start_paused = true)]
async fn the_saved_capture_plays_to_its_end() {
    let expected: serde_json::Value =
        serde_json::from_str(&fixture("ccd-simulator.expect.json")).unwrap();
    let scripts = Scripts::new(vec![fixture("ccd-simulator.script")]);
    let connect: IndiConnect = scripts.connect();
    let camera = IndiCamera::new(
        connect,
        "127.0.0.1:7624".to_string(),
        Some("CCD Simulator".to_string()),
        Vec::new(),
        clock(),
    );
    let info = camera.describe().await.unwrap();
    let number = |value: &serde_json::Value| value.as_f64().unwrap();
    let range = |value: &serde_json::Value| scope_node::imaging::Range {
        min: number(&value[0]),
        max: number(&value[1]),
    };
    assert_eq!(info.model, expected["model"].as_str().unwrap());
    assert_eq!(f64::from(info.width), number(&expected["width"]));
    assert_eq!(f64::from(info.height), number(&expected["height"]));
    assert_eq!(info.pixel_um, number(&expected["pixel_um"]));
    assert_eq!(f64::from(info.bits), number(&expected["bits"]));
    assert_eq!(info.exposure_ms, range(&expected["exposure_ms"]));
    assert_eq!(info.gain.unwrap(), range(&expected["gain"]));
    assert_eq!(info.offset.unwrap(), range(&expected["offset"]));
    assert_eq!(
        info.binnings,
        expected["binnings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| number(value) as u32)
            .collect::<Vec<_>>()
    );

    // The region of the recording, then the exposure of 1 s.
    camera
        .apply(&ImagingSettings {
            roi: Some(Roi {
                x: 0,
                y: 0,
                width: 64,
                height: 48,
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 1000.0,
            purpose: "frame".to_string(),
        })
        .await
        .unwrap();
    let saved = &expected["frame"];
    assert_eq!(f64::from(frame.width), number(&saved["width"]));
    assert_eq!(f64::from(frame.height), number(&saved["height"]));
    assert_eq!(f64::from(frame.bits), number(&saved["bits"]));
    let samples: Vec<u16> = frame
        .samples
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    assert_eq!(
        samples[..8],
        saved["first_samples"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| number(value) as u16)
            .collect::<Vec<_>>()[..]
    );
    let sum: u64 = samples.iter().map(|sample| u64::from(*sample)).sum();
    assert_eq!(sum, number(&saved["sample_sum"]) as u64);
    scripts.finish().await;
}

/// An unreachable server still fails an exposure with the server's address
/// and the error's text (US4-S3's shape, for `expose`).
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
        Some(COLOUR_DEVICE.to_string()),
        Vec::new(),
        clock(),
    );
    match camera
        .expose(&Exposure {
            exposure_ms: 500.0,
            purpose: "frame".to_string(),
        })
        .await
    {
        Err(ImagingError::Failed(text)) => assert!(
            text.starts_with("cannot reach the INDI server at 127.0.0.1:7624: "),
            "the failure names the server: {text}"
        ),
        other => panic!("an unreachable server fails the exposure, it answered {other:?}"),
    }
}
