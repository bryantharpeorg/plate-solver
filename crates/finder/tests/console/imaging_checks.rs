//! Imaging frames that break the rules, and an 8-bit mono one (031 US4): the
//! checks on what the scope node sends (FR-010) and the frame the console
//! saves exactly as the node sent it. Oracles: the scenario's words and
//! timings, the worked header wait, and the bytes the test's own node sends.

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{http::StatusCode, Router};
use finder::{link::Transport, session::Session, AppState};
use futures_util::{stream, StreamExt};
use scope_link::{
    memory, proto,
    proto::scope_node_imaging_server::{ScopeNodeImaging, ScopeNodeImagingServer},
    TokenInterceptor,
};
use scope_node::{
    clock::NodeClock,
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use serde_json::{json, Value};
use simulator::{mount::SimMount, VirtualClock};
use tonic::{Request, Response, Status};

use crate::support::live as support;

/// A node's NodeClock on the simulator's virtual clock, so the stand-in
/// camera's frames follow the rig's time.
struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

/// One exposure the test's own `ScopeNodeImaging` answers `Expose` with: the
/// events in order, then the end of the stream, or a stream that stays open
/// and silent.
#[derive(Clone)]
struct Script {
    events: Vec<Result<proto::ExposeEvent, Status>>,
    hold: bool,
}

/// The test's own `ScopeNodeImaging` (US4-S1): the four unary calls answered
/// from one description and one settings answer, and `Expose` from a script,
/// one exposure at a time, in order. The script is shared, so every
/// connection's clone answers from the same queue.
#[derive(Clone)]
struct ScriptedImaging {
    info: proto::ImagingInfo,
    settings: proto::ImagingSettings,
    script: Arc<Mutex<VecDeque<Script>>>,
}

/// The camera of US4-S2: a mono 8-bit camera with no controls but the
/// exposure, whose region in force is (0, 0, 3, 2).
fn mono_camera() -> proto::ImagingInfo {
    proto::ImagingInfo {
        model: "test mono camera".to_string(),
        serial: None,
        width: 16,
        height: 8,
        pixel_um: 1.0,
        bits: 8,
        bayer: String::new(),
        exposure_ms: Some(proto::ImagingRange {
            min: 1.0,
            max: 60000.0,
        }),
        gain: None,
        offset: None,
        binnings: vec![1],
        cooler: false,
    }
}

/// The settings in force the node answers `ConfigureImaging` with (US4-S2).
fn mono_settings() -> proto::ImagingSettings {
    proto::ImagingSettings {
        gain: None,
        offset: None,
        binning: Some(1),
        roi: Some(proto::ImagingRoi {
            x: 0,
            y: 0,
            width: 3,
            height: 2,
        }),
        cooler_on: None,
        set_temp_c: None,
    }
}

impl ScriptedImaging {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            info: mono_camera(),
            settings: mono_settings(),
            script: Arc::new(Mutex::new(VecDeque::new())),
        })
    }
}

#[tonic::async_trait]
impl ScopeNodeImaging for ScriptedImaging {
    async fn describe_imaging(
        &self,
        _: Request<proto::DescribeImagingRequest>,
    ) -> Result<Response<proto::ImagingInfo>, Status> {
        Ok(Response::new(self.info.clone()))
    }

    async fn imaging_status(
        &self,
        _: Request<proto::ImagingStatusRequest>,
    ) -> Result<Response<proto::ImagingStatusReply>, Status> {
        Ok(Response::new(proto::ImagingStatusReply::default()))
    }

    async fn configure_imaging(
        &self,
        _: Request<proto::ImagingSettings>,
    ) -> Result<Response<proto::ImagingSettings>, Status> {
        Ok(Response::new(self.settings))
    }

    type ExposeStream =
        Pin<Box<dyn futures_util::Stream<Item = Result<proto::ExposeEvent, Status>> + Send>>;
    async fn expose(
        &self,
        _: Request<proto::ExposeRequest>,
    ) -> Result<Response<Self::ExposeStream>, Status> {
        let script = self
            .script
            .lock()
            .unwrap()
            .pop_front()
            .expect("a scripted exposure");
        let events = stream::iter(script.events);
        Ok(Response::new(if script.hold {
            Box::pin(events.chain(stream::pending()))
        } else {
            Box::pin(events)
        }))
    }

    async fn abort_exposure(
        &self,
        _: Request<proto::AbortExposureRequest>,
    ) -> Result<Response<proto::AbortExposureReply>, Status> {
        Ok(Response::new(proto::AbortExposureReply::default()))
    }
}

/// The console rig: the stand-in node's own services with the test's
/// `ScopeNodeImaging` in place of the node's, over in-memory streams, and the
/// imaging folder R.
fn rig(root: &Path, imaging: Arc<ScriptedImaging>) -> Router {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    let clock = Arc::new(NodeTime(sim.clock().clone()));
    let node = NodeService::with_controls(
        SyntheticCamera::new(clock.clone()),
        clock,
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap();
    node.controls()
        .set_frame_interval(Duration::from_millis(500));
    let transport = Transport::connector(move || {
        let node = node.clone();
        let imaging = imaging.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                let routes = tonic::service::Routes::new(node.into_server()).add_service(
                    ScopeNodeImagingServer::with_interceptor(
                        (*imaging).clone(),
                        TokenInterceptor::new("test-token"),
                    ),
                );
                memory::serve_routes_on(server, routes).await.unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    });
    let session = Session::default();
    session
        .edit(
            serde_json::from_value(json!({
                "node": {"address": "scope.local:1", "token": "test-token"},
                "site": {"lat_deg": 40, "lon_deg": -75}
            }))
            .unwrap(),
            support::START,
        )
        .unwrap();
    let state = AppState::shared(support::DB.clone(), finder::samples::Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live()
        .with_imaging(root);
    finder::router(state)
}

async fn get(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "GET", path, json!(null)).await
}

async fn post_frame(app: &Router) -> (StatusCode, Value) {
    support::request(app, "POST", "/api/imaging/frame", json!(null)).await
}

/// Read `GET /api/imaging` every 100 ms until the camera is ready, for at
/// most 60 s.
async fn ready(app: &Router) -> Value {
    for _ in 0..600 {
        let (_, view) = get(app, "/api/imaging").await;
        if view["state"] == "ready" {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("imaging never became ready");
}

/// Read the view until the frame `id` has failed with exactly `error`, for at
/// most 160 s: a 152 s header wait fits, where the shared setup's 60 s would
/// not (US4-S1).
async fn wait_failed(app: &Router, id: u64, error: &str) -> Value {
    for _ in 0..1600 {
        let (_, view) = get(app, "/api/imaging").await;
        let frame = &view["frame"];
        if frame["id"] == json!(id) && frame["state"] == json!("failed") {
            assert_eq!(frame["error"], json!(error), "{view}");
            return view;
        }
        assert!(
            frame["id"] == json!(id)
                && (frame["state"] == json!("exposing") || frame["state"] == json!("receiving")),
            "{view}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("frame {id} never failed");
}

/// Read the view until the frame `id` is saved, for at most 60 s.
async fn wait_saved(app: &Router, id: u64) -> Value {
    for _ in 0..600 {
        let (_, view) = get(app, "/api/imaging").await;
        if view["frame"]["id"] == json!(id) && view["frame"]["state"] == json!("saved") {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("frame {id} was never saved");
}

/// R holds no `.fits` and no `.part` file anywhere under it (US4-S1).
fn assert_no_frames(root: &Path, name: &str) {
    let mut stack = vec![root.to_path_buf()];
    while let Some(folder) = stack.pop() {
        for entry in std::fs::read_dir(&folder).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let file_name = entry.file_name();
                let file_name = file_name.to_str().unwrap();
                assert!(
                    !file_name.ends_with(".fits") && !file_name.ends_with(".part"),
                    "{name}: {file_name} left behind"
                );
            }
        }
    }
}

/// A header event, with the header's shape the checks read (FR-010).
fn header_event(
    width: u32,
    height: u32,
    bits: u32,
    byte_count: u64,
) -> Result<proto::ExposeEvent, Status> {
    Ok(proto::ExposeEvent {
        event: Some(proto::expose_event::Event::Header(
            proto::ImagingFrameHeader {
                width,
                height,
                bits,
                byte_count,
                binning: 1,
                exposure_ms: 100.0,
                purpose: "frame".to_string(),
                start_s: 1000.0,
                ..Default::default()
            },
        )),
    })
}

/// A chunk event, with the bytes the test's own node sends.
fn chunk_event(bytes: &[u8]) -> Result<proto::ExposeEvent, Status> {
    Ok(proto::ExposeEvent {
        event: Some(proto::expose_event::Event::Chunk(bytes.to_vec())),
    })
}

/// A saved FITS file, read with the test's own fixed-column reader, never
/// through the console's FITS code: the cards in order, then the bytes that
/// follow the header's blocks.
struct Fits {
    cards: Vec<(String, Value)>,
    data: Vec<u8>,
}

/// A card's number, as the shared setup compares it: whole and real kept apart.
fn number(text: &str) -> Value {
    if text.contains('.') || text.contains('E') || text.contains('e') {
        let value: f64 = text.parse().expect("a number card");
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .expect("a finite real")
    } else {
        Value::Number(text.parse::<i64>().expect("a whole number").into())
    }
}

fn read_fits(path: &Path) -> Fits {
    let bytes = std::fs::read(path).unwrap();
    let mut cards = Vec::new();
    let mut offset = 0;
    loop {
        let card = &bytes[offset..offset + 80];
        let keyword = String::from_utf8_lossy(&card[..8]).trim_end().to_string();
        if keyword == "END" {
            cards.push((keyword, Value::Null));
            offset += 80;
            break;
        }
        assert_eq!(
            &card[8..10],
            b"= ",
            "the {keyword} card is not a value card"
        );
        let value = String::from_utf8_lossy(&card[10..]).trim().to_string();
        let value = if let Some(text) = value.strip_prefix('\'') {
            Value::String(
                text.strip_suffix('\'')
                    .expect("a closing quote")
                    .replace("''", "'")
                    .trim_end()
                    .to_string(),
            )
        } else if value == "T" {
            Value::Bool(true)
        } else if value == "F" {
            Value::Bool(false)
        } else {
            number(&value)
        };
        cards.push((keyword, value));
        offset += 80;
    }
    let data = bytes[offset.div_ceil(2880) * 2880..].to_vec();
    Fits { cards, data }
}

/// A relative path the view names, under R.
fn under(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

/// US4-S1: an imaging frame that is short, too large, out of order or late
/// ends `failed` with exactly the words of the rule it broke (FR-010), and R
/// holds no `.fits` and no `.part` file after any of them. The stalled frame
/// fails 30 s after the 16 bytes were sent and the silent one 152 s after the
/// request (worked example: twice the console's starting 1000 ms exposure
/// plus 150 s).
#[tokio::test(start_paused = true)]
async fn broken_frames_fail_with_their_words_and_leave_no_file() {
    let scratch = crate::support::Scratch::new("imaging-checks-broken");
    let root = scratch.0.join("imaging");
    std::fs::create_dir_all(&root).unwrap();
    let node = ScriptedImaging::new();
    let app = rig(&root, node.clone());
    ready(&app).await;

    let sixteen = [0u8; 16];
    let cases: Vec<(&str, Script, &str, Option<f64>)> = vec![
        (
            "incomplete",
            Script {
                events: vec![header_event(4, 4, 16, 32), chunk_event(&sixteen)],
                hold: false,
            },
            "the imaging frame was incomplete: 16 of 32 bytes",
            None,
        ),
        (
            "a header that does not add up",
            Script {
                events: vec![header_event(4, 4, 16, 30)],
                hold: false,
            },
            "the imaging frame's header does not add up",
            None,
        ),
        (
            "a header above the limit",
            Script {
                events: vec![header_event(16384, 8193, 16, 268_468_224)],
                hold: false,
            },
            "the imaging frame's header does not add up",
            None,
        ),
        (
            "a chunk before any header",
            Script {
                events: vec![chunk_event(&[0, 0, 0, 0])],
                hold: false,
            },
            "the scope node sent no imaging frame header",
            None,
        ),
        (
            "more bytes than the header said",
            Script {
                events: vec![header_event(4, 4, 16, 32), chunk_event(&[0u8; 40])],
                hold: false,
            },
            "the scope node sent more bytes than the header said",
            None,
        ),
        (
            "a failed exposure",
            Script {
                events: vec![Err(Status::internal("the camera fell over"))],
                hold: false,
            },
            "the exposure failed: the camera fell over",
            None,
        ),
        (
            "a stalled frame",
            Script {
                events: vec![header_event(4, 4, 16, 32), chunk_event(&sixteen)],
                hold: true,
            },
            "the imaging frame stalled",
            Some(30.0),
        ),
        (
            "no header at all",
            Script {
                events: Vec::new(),
                hold: true,
            },
            "no imaging frame header arrived within 152 s",
            Some(152.0),
        ),
    ];
    for (name, script, error, after) in cases {
        node.script.lock().unwrap().push_back(script);
        let asked = tokio::time::Instant::now();
        let (status, answer) = post_frame(&app).await;
        assert_eq!(status, StatusCode::OK, "{name}: {answer}");
        let id = answer["frame"]["id"].as_u64().expect("a frame id");
        wait_failed(&app, id, error).await;
        if let Some(after) = after {
            let elapsed = asked.elapsed().as_secs_f64();
            assert!(
                elapsed >= after && elapsed <= after + 0.2,
                "{name}: failed {elapsed} s after the request, not {after} s"
            );
        }
        assert_no_frames(&root, name);
    }
}

/// US4-S2: a frame from an 8-bit mono camera, whose header carries a region
/// the console's settings did not hold (as when another INDI client has moved
/// it), is saved as the node sent it: 8 bits, no cards for what the camera did
/// not report, the header's region in the view and the origin cards, and its
/// bytes unchanged (oracle: the bytes the test's node sent).
#[tokio::test(start_paused = true)]
async fn an_eight_bit_mono_frame_is_saved_as_sent() {
    let scratch = crate::support::Scratch::new("imaging-checks-mono");
    let root = scratch.0.join("imaging");
    std::fs::create_dir_all(&root).unwrap();
    let node = ScriptedImaging::new();
    let app = rig(&root, node.clone());
    let view = ready(&app).await;
    assert_eq!(
        view["settings"]["roi"],
        json!({"x":0,"y":0,"width":3,"height":2})
    );

    node.script.lock().unwrap().push_back(Script {
        events: vec![
            Ok(proto::ExposeEvent {
                event: Some(proto::expose_event::Event::Header(
                    proto::ImagingFrameHeader {
                        width: 3,
                        height: 2,
                        bits: 8,
                        byte_count: 6,
                        binning: 1,
                        exposure_ms: 100.0,
                        purpose: "frame".to_string(),
                        start_s: 1000.0,
                        roi: Some(proto::ImagingRoi {
                            x: 8,
                            y: 4,
                            width: 3,
                            height: 2,
                        }),
                        ..Default::default()
                    },
                )),
            }),
            chunk_event(&[0x00, 0x01, 0x02, 0xFD, 0xFE, 0xFF]),
        ],
        hold: false,
    });

    // The frame is asked for with the settings' region.
    let (status, answer) = post_frame(&app).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        answer["frame"]["roi"],
        json!({"x":0,"y":0,"width":3,"height":2})
    );

    let view = wait_saved(&app, answer["frame"]["id"].as_u64().unwrap()).await;
    let frame = &view["frame"];
    // The header's region replaces the settings' in the frame.
    assert_eq!(frame["roi"], json!({"x":8,"y":4,"width":3,"height":2}));
    assert_eq!(frame["bits"], json!(8));
    assert!(frame.get("bayer").is_none(), "{view}");
    assert!(frame.get("gain").is_none(), "{view}");
    assert!(frame.get("offset").is_none(), "{view}");
    assert!(frame.get("temp_c").is_none(), "{view}");

    // The file: one header block and the samples padded to one, 5760 bytes.
    let file = under(&root, frame["file"].as_str().expect("a file name"));
    assert_eq!(std::fs::metadata(&file).unwrap().len(), 5760);
    let fits = read_fits(&file);
    let date_obs = frame["date_obs"].as_str().expect("a DATE-OBS");
    let expected: Vec<(String, Value)> = [
        ("SIMPLE", json!(true)),
        ("BITPIX", json!(8)),
        ("NAXIS", json!(2)),
        ("NAXIS1", json!(3)),
        ("NAXIS2", json!(2)),
        ("DATE-OBS", json!(date_obs)),
        ("EXPTIME", json!(0.1)),
        ("XBINNING", json!(1)),
        ("YBINNING", json!(1)),
        ("XORGSUBF", json!(8)),
        ("YORGSUBF", json!(4)),
        ("INSTRUME", json!("test mono camera")),
        ("IMAGETYP", json!("LIGHT")),
        ("ROWORDER", json!("TOP-DOWN")),
        ("FRAMEID", json!(1)),
        ("PURPOSE", json!("frame")),
        ("END", Value::Null),
    ]
    .into_iter()
    .map(|(keyword, value)| (keyword.to_string(), value))
    .collect();
    assert_eq!(fits.cards, expected);
    // The samples begin with the bytes the node sent, unchanged.
    assert_eq!(&fits.data[..6], &[0x00, 0x01, 0x02, 0xFD, 0xFE, 0xFF]);
}
