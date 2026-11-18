//! Taking imaging frames and saving them as FITS (031 US2): the route and the
//! frame view, the files, their cards and ids, the pace that leaves the link's
//! heartbeat alone, and the purposes `dark` and `flat`. Oracles: the stand-in
//! camera's sample rule (030 US1), worked FITS sizes, the times tokio's paused
//! clock measures, and owner decision 2's purposes.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::http::StatusCode;
use axum::Router;
use finder::{link::Transport, session::Session, AppState};
use scope_link::memory;
use scope_node::{
    clock::NodeClock,
    imaging::synthetic::SyntheticImagingCamera,
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator, VirtualClock};

use crate::support::live as support;

/// The console rig of the shared setup: the sim rig's node, whose imaging
/// camera is the rig's own, a paired session, and the imaging folder `R`.
struct Rig {
    app: Router,
    link: Arc<finder::link::LinkManager>,
    imaging: Arc<finder::imaging::Imaging>,
}

fn console_clock(sim: &Simulator) -> Arc<support::ConsoleClock> {
    Arc::new(support::ConsoleClock(sim.clock().clone()))
}

/// A node's NodeClock on the simulator's virtual clock, so the stand-in
/// camera's frames follow the rig's time.
struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

fn duplex(node: &Arc<SimNode>) -> Transport {
    let node = node.clone();
    Transport::connector(move || {
        let node = node.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            let service = node.service();
            tokio::spawn(async move {
                memory::serve_routes_on(server, service.routes())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    })
}

fn duplex_service(service: NodeService) -> Transport {
    Transport::connector(move || {
        let service = service.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                memory::serve_routes_on(server, service.routes())
                    .await
                    .unwrap();
            });
            Ok::<_, std::io::Error>(client)
        }
    })
}

fn state(
    sim: &Simulator,
    transport: Transport,
    root: &Path,
) -> (
    AppState,
    Arc<finder::link::LinkManager>,
    Arc<finder::imaging::Imaging>,
) {
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
        .with_clock(console_clock(sim))
        .with_link(transport);
    let link = state.link().unwrap();
    let state = state.with_live().with_imaging(root);
    let imaging = state.imaging().unwrap();
    (state, link, imaging)
}

fn sim_rig() -> Simulator {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}

/// The console rig: the node's imaging camera is the simulated one.
async fn console_rig(root: &Path) -> Rig {
    let sim = sim_rig();
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Settings::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
    let (state, link, imaging) = state(&sim, duplex(&node), root);
    Rig {
        app: finder::router(state),
        link,
        imaging,
    }
}

/// The stand-in console rig: the node rig's `SyntheticImagingCamera`, whose
/// clocks read the simulator's virtual clock, so its frames follow the sample
/// rule (the shared setup).
async fn stand_in_rig(root: &Path) -> Rig {
    let sim = sim_rig();
    let clock = Arc::new(NodeTime(sim.clock().clone()));
    let node = NodeService::with_controls(
        SyntheticCamera::new(clock.clone()),
        clock.clone(),
        "test-token",
        Settings::default(),
        NodeControls::default(),
    )
    .unwrap()
    .with_imaging(SyntheticImagingCamera::new(clock));
    node.controls()
        .set_frame_interval(Duration::from_millis(500));
    let (state, link, imaging) = state(&sim, duplex_service(node), root);
    Rig {
        app: finder::router(state),
        link,
        imaging,
    }
}

async fn get(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "GET", path, json!(null)).await
}

async fn put(app: &Router, body: Value) -> (StatusCode, Value) {
    support::request(app, "PUT", "/api/imaging/settings", body).await
}

async fn post_frame(app: &Router) -> (StatusCode, Value) {
    support::request(app, "POST", "/api/imaging/frame", json!(null)).await
}

/// Read `GET /api/imaging` every 100 ms until `check` holds, for at most 60 s.
async fn wait_until(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..600 {
        let (_, view) = get(app, "/api/imaging").await;
        if check(&view) {
            return view;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let (_, view) = get(app, "/api/imaging").await;
    panic!("imaging never reached the awaited state: {view}");
}

async fn ready(app: &Router) -> Value {
    wait_until(app, |view| view["state"] == "ready").await
}

/// A saved FITS file, read as the shared setup says and never through the
/// console's FITS code: the cards in order, a number as a number, then the
/// samples, each 16-bit sample the stored signed value plus `BZERO`.
struct Fits {
    cards: Vec<(String, Value)>,
    samples: Vec<u16>,
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
    let data = &bytes[offset.div_ceil(2880) * 2880..];
    let card = |name: &str| {
        cards
            .iter()
            .find(|(keyword, _)| keyword == name)
            .unwrap_or_else(|| panic!("no {name} card"))
            .1
            .clone()
    };
    let width = card("NAXIS1").as_i64().expect("NAXIS1") as usize;
    let height = card("NAXIS2").as_i64().expect("NAXIS2") as usize;
    let samples = if card("BITPIX") == json!(8) {
        data[..width * height]
            .iter()
            .map(|byte| u16::from(*byte))
            .collect()
    } else {
        (0..width * height)
            .map(|index| {
                let stored = i16::from_be_bytes([data[2 * index], data[2 * index + 1]]);
                (i32::from(stored) + 32768) as u16
            })
            .collect()
    };
    Fits { cards, samples }
}

/// One card of a saved file, by keyword.
fn card(cards: &[(String, Value)], name: &str) -> Value {
    cards
        .iter()
        .find(|(keyword, _)| keyword == name)
        .unwrap_or_else(|| panic!("no {name} card"))
        .1
        .clone()
}

/// The names of the `.part` files left in a folder (none, once a frame is saved).
fn part_files(folder: &Path) -> Vec<String> {
    std::fs::read_dir(folder)
        .unwrap()
        .filter_map(|entry| {
            let name = entry.unwrap().file_name();
            name.to_str()
                .filter(|name| name.ends_with(".part"))
                .map(String::from)
        })
        .collect()
}

/// The frame's folder under R: `frames/<UTC date>/`, the sim rig's start date.
fn frames(root: &Path) -> PathBuf {
    root.join("frames").join("2026-01-16")
}

/// The stand-in camera's sample rule with the settings it starts with (offset
/// 10, binning 1, the whole sensor): `256 + 16·10 + ((c + 3·r) mod 251)`.
fn stand_in_sample(column: u32, row: u32) -> u16 {
    (416 + ((column + 3 * row) % 251)) as u16
}

/// US2-S1: a frame from the stand-in camera is saved once, with the cards in
/// order, `DATE-OBS` on the console's clock and the samples of the sample rule.
#[tokio::test(start_paused = true)]
async fn a_frame_from_the_stand_in_camera_is_saved_as_fits() {
    let scratch = crate::support::Scratch::new("imaging-frames-fits");
    let root = scratch.0.join("imaging");
    let rig = stand_in_rig(&root).await;
    ready(&rig.app).await;
    put(&rig.app, json!({"exposure_ms":500})).await;
    let (_, session) = get(&rig.app, "/api/session").await;
    let time = session["time"].as_f64().unwrap();

    let (status, answer) = post_frame(&rig.app).await;
    assert_eq!(status, StatusCode::OK);
    let frame = &answer["frame"];
    assert_eq!(frame["id"], json!(1));
    assert_eq!(frame["state"], json!("exposing"));
    assert_eq!(frame["purpose"], json!("frame"));
    assert_eq!(frame["exposure_ms"], json!(500.0));
    assert_eq!(frame["roi"], json!({"x":0,"y":0,"width":64,"height":48}));

    let view = wait_until(&rig.app, |view| view["frame"]["state"] == "saved").await;
    let frame = &view["frame"];
    assert_eq!(frame["width"], json!(64));
    assert_eq!(frame["height"], json!(48));
    assert_eq!(frame["bits"], json!(16));
    assert_eq!(frame["bayer"], json!("RGGB"));
    assert_eq!(frame["gain"], json!(100.0));
    assert_eq!(frame["offset"], json!(10.0));
    assert_eq!(frame["binning"], json!(1));
    assert_eq!(frame["temp_c"], json!(20.0));
    assert_eq!(frame["bytes"], json!(6144));
    assert_eq!(frame["received"], json!(6144));
    assert_eq!(frame["roi"], json!({"x":0,"y":0,"width":64,"height":48}));
    assert_eq!(frame["file"], json!("frames/2026-01-16/000001.fits"));
    assert!(frame.get("error").is_none(), "{frame}");
    assert!(frame.get("set_temp_c").is_none(), "{frame}");
    let start = frame["start"].as_f64().unwrap();
    assert!(
        (start - time).abs() <= 0.001,
        "start {start} is not the session's time {time}"
    );
    let date_obs = frame["date_obs"].as_str().unwrap();
    let parsed = finder::simulate::parse_start(&format!("{date_obs}Z")).unwrap();
    assert!(
        (parsed - start).abs() <= 0.0006,
        "DATE-OBS {date_obs} is not the start {start}"
    );
    let received_at = frame["received_at"].as_f64().unwrap();
    let saved_at = frame["saved_at"].as_f64().unwrap();
    assert!(received_at - start >= 0.49 && received_at - start <= 1.5);
    assert!(saved_at - start >= 0.49 && saved_at - start <= 1.5);
    assert!(saved_at >= received_at);

    let file = frames(&root).join("000001.fits");
    assert_eq!(std::fs::metadata(&file).unwrap().len(), 11_520);
    assert!(part_files(&frames(&root)).is_empty());

    let fits = read_fits(&file);
    let expected = vec![
        ("SIMPLE".to_string(), json!(true)),
        ("BITPIX".to_string(), json!(16)),
        ("NAXIS".to_string(), json!(2)),
        ("NAXIS1".to_string(), json!(64)),
        ("NAXIS2".to_string(), json!(48)),
        ("BZERO".to_string(), json!(32768)),
        ("BSCALE".to_string(), json!(1)),
        ("DATE-OBS".to_string(), json!(date_obs)),
        ("EXPTIME".to_string(), json!(0.5)),
        ("GAIN".to_string(), json!(100.0)),
        ("OFFSET".to_string(), json!(10.0)),
        ("XBINNING".to_string(), json!(1)),
        ("YBINNING".to_string(), json!(1)),
        ("XORGSUBF".to_string(), json!(0)),
        ("YORGSUBF".to_string(), json!(0)),
        ("CCD-TEMP".to_string(), json!(20.0)),
        ("BAYERPAT".to_string(), json!("RGGB")),
        ("INSTRUME".to_string(), json!("synthetic imaging camera")),
        ("IMAGETYP".to_string(), json!("LIGHT")),
        ("ROWORDER".to_string(), json!("TOP-DOWN")),
        ("FRAMEID".to_string(), json!(1)),
        ("PURPOSE".to_string(), json!("frame")),
        ("END".to_string(), Value::Null),
    ];
    assert_eq!(fits.cards, expected);
    for row in 0..48u32 {
        for column in 0..64u32 {
            let sample = stand_in_sample(column, row);
            assert_eq!(fits.samples[(row * 64 + column) as usize], sample);
        }
    }
}

/// US2-S2: the file follows the camera's binning and region, the cooler's set
/// temperature is stamped while it is in force and the camera's serial with it
/// (worked examples: 2880 bytes of header and the samples padded to a multiple
/// of 2880; the origin cards are the header's region's origin over the
/// binning, 400/2 and 200/2).
#[tokio::test(start_paused = true)]
async fn frames_follow_binning_region_and_cooler() {
    let scratch = crate::support::Scratch::new("imaging-frames-cards");
    let root = scratch.0.join("imaging");
    let rig = console_rig(&root).await;
    ready(&rig.app).await;
    put(&rig.app, json!({"exposure_ms":200})).await;

    post_frame(&rig.app).await;
    let view = wait_until(&rig.app, |view| view["frame"]["state"] == "saved").await;
    assert!(view["frame"].get("set_temp_c").is_none(), "{view}");
    let first = frames(&root).join("000001.fits");
    assert_eq!(std::fs::metadata(&first).unwrap().len(), 2_036_160);
    let first = read_fits(&first);
    assert_eq!(card(&first.cards, "NAXIS1"), json!(1236));
    assert_eq!(card(&first.cards, "NAXIS2"), json!(822));
    assert_eq!(card(&first.cards, "XORGSUBF"), json!(0));
    assert_eq!(card(&first.cards, "YORGSUBF"), json!(0));
    assert_eq!(
        card(&first.cards, "INSTRUME"),
        json!("simulated imaging camera")
    );
    assert_eq!(card(&first.cards, "SERIALNO"), json!("SIM-0001"));
    assert!(first.cards.iter().all(|(keyword, _)| keyword != "SET-TEMP"));

    put(&rig.app, json!({"binning":2})).await;
    post_frame(&rig.app).await;
    wait_until(&rig.app, |view| view["frame"]["state"] == "saved").await;
    let second = frames(&root).join("000002.fits");
    assert_eq!(std::fs::metadata(&second).unwrap().len(), 512_640);
    let second = read_fits(&second);
    assert_eq!(card(&second.cards, "NAXIS1"), json!(618));
    assert_eq!(card(&second.cards, "NAXIS2"), json!(411));
    assert_eq!(card(&second.cards, "XBINNING"), json!(2));
    assert_eq!(card(&second.cards, "YBINNING"), json!(2));

    let region = json!({"x":400,"y":200,"width":400,"height":400});
    put(
        &rig.app,
        json!({"roi":region,"cooler_on":true,"set_temp_c":-10}),
    )
    .await;
    let (_, answer) = post_frame(&rig.app).await;
    assert_eq!(answer["frame"]["set_temp_c"], json!(-10.0));
    assert_eq!(answer["frame"]["roi"], region);
    let view = wait_until(&rig.app, |view| view["frame"]["state"] == "saved").await;
    assert_eq!(view["frame"]["set_temp_c"], json!(-10.0));
    assert_eq!(view["frame"]["roi"], region);
    let third = frames(&root).join("000003.fits");
    assert_eq!(std::fs::metadata(&third).unwrap().len(), 83_520);
    let third = read_fits(&third);
    assert_eq!(card(&third.cards, "NAXIS1"), json!(200));
    assert_eq!(card(&third.cards, "NAXIS2"), json!(200));
    assert_eq!(card(&third.cards, "XORGSUBF"), json!(200));
    assert_eq!(card(&third.cards, "YORGSUBF"), json!(100));
    assert_eq!(card(&third.cards, "CCD-TEMP"), json!(-10.0));
    assert_eq!(card(&third.cards, "SET-TEMP"), json!(-10.0));

    put(&rig.app, json!({"cooler_on":false})).await;
    post_frame(&rig.app).await;
    let view = wait_until(&rig.app, |view| view["frame"]["state"] == "saved").await;
    assert!(view["frame"].get("set_temp_c").is_none(), "{view}");
    let fourth = read_fits(&frames(&root).join("000004.fits"));
    assert!(fourth
        .cards
        .iter()
        .all(|(keyword, _)| keyword != "SET-TEMP"));
}

/// US2-S3: the console's ids start after the largest file of the date's folder
/// and rise with every frame, and a name already taken fails the frame and
/// leaves the file as it is.
#[tokio::test(start_paused = true)]
async fn ids_follow_the_folder_and_a_taken_name_fails_the_frame() {
    let scratch = crate::support::Scratch::new("imaging-frames-ids");
    let root = scratch.0.join("imaging");
    std::fs::create_dir_all(frames(&root)).unwrap();
    let seven = frames(&root).join("000007.fits");
    std::fs::write(&seven, [7u8; 128]).unwrap();
    let rig = stand_in_rig(&root).await;
    ready(&rig.app).await;
    put(&rig.app, json!({"exposure_ms":100})).await;

    let (status, answer) = post_frame(&rig.app).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(answer["frame"]["id"], json!(8));
    wait_until(&rig.app, |view| view["frame"]["state"] == "saved").await;
    assert!(frames(&root).join("000008.fits").exists());
    assert_eq!(std::fs::read(&seven).unwrap(), [7u8; 128]);

    let (_, answer) = post_frame(&rig.app).await;
    assert_eq!(answer["frame"]["id"], json!(9));
    wait_until(&rig.app, |view| view["frame"]["state"] == "saved").await;
    assert!(frames(&root).join("000009.fits").exists());
    assert_eq!(std::fs::read(&seven).unwrap(), [7u8; 128]);

    // A file under the next name fails the frame and is left as it is.
    let ten = frames(&root).join("000010.fits");
    std::fs::write(&ten, [10u8; 64]).unwrap();
    let (_, answer) = post_frame(&rig.app).await;
    assert_eq!(answer["frame"]["id"], json!(10));
    let view = wait_until(&rig.app, |view| view["frame"]["state"] == "failed").await;
    assert_eq!(
        view["frame"]["error"],
        json!("could not save the imaging frame: frames/2026-01-16/000010.fits already exists")
    );
    assert_eq!(std::fs::read(&ten).unwrap(), [10u8; 64]);
    assert!(part_files(&frames(&root)).is_empty());

    let (_, answer) = post_frame(&rig.app).await;
    assert_eq!(answer["frame"]["id"], json!(11));
    wait_until(&rig.app, |view| view["frame"]["state"] == "saved").await;
    assert!(frames(&root).join("000011.fits").exists());
}

/// US2-S4: a frame paced at 64 KiB/s is 28 s in flight (worked example: the
/// eighth and last chunk leaves when 7 × 262,144 bytes have gone, 28 s after
/// the first, at 65,536 bytes a second), the heartbeat keeps its period and the
/// link stays connected.
#[tokio::test(start_paused = true)]
async fn a_paced_frame_keeps_the_link_connected() {
    let scratch = crate::support::Scratch::new("imaging-frames-paced");
    let root = scratch.0.join("imaging");
    let rig = console_rig(&root).await;
    ready(&rig.app).await;
    put(&rig.app, json!({"exposure_ms":200,"forward_kib_per_s":64})).await;

    let status = rig.link.status();
    let heartbeats_before = status.borrow().heartbeats;
    post_frame(&rig.app).await;
    let mut receiving = None;
    let mut received_seen = Vec::new();
    let mut saved = None;
    for _ in 0..600 {
        let (_, imaging) = get(&rig.app, "/api/imaging").await;
        let (_, session) = get(&rig.app, "/api/session").await;
        assert_eq!(session["node"]["connected"], json!(true), "{session}");
        let frame = &imaging["frame"];
        if frame["state"] == "saved" {
            saved = Some(imaging);
            break;
        }
        assert!(
            frame["state"] == "exposing" || frame["state"] == "receiving",
            "{imaging}"
        );
        if frame["state"] == "receiving" {
            assert_eq!(frame["bytes"], json!(2_031_984), "{imaging}");
            received_seen.push(frame["received"].as_u64().unwrap());
            receiving.get_or_insert_with(tokio::time::Instant::now);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let view = saved.expect("the paced frame was never saved");
    let receiving = receiving.expect("a read while receiving");
    let elapsed = receiving.elapsed();
    assert!(
        elapsed >= Duration::from_secs(27) && elapsed <= Duration::from_secs(31),
        "receiving for {elapsed:?}"
    );
    assert!(
        received_seen.iter().any(|value| *value > received_seen[0]),
        "received never rose: {received_seen:?}"
    );
    let heartbeats = status.borrow().heartbeats - heartbeats_before;
    assert!(heartbeats >= 104, "only {heartbeats} heartbeats");
    let frame = &view["frame"];
    let start = frame["start"].as_f64().unwrap();
    let received_at = frame["received_at"].as_f64().unwrap();
    let saved_at = frame["saved_at"].as_f64().unwrap();
    let in_hand = received_at - start - 0.2;
    assert!((in_hand - 28.0).abs() <= 0.1, "{in_hand} s in hand");
    assert!(saved_at >= received_at && saved_at <= received_at + 1.0);
    let file = frames(&root).join("000001.fits");
    assert_eq!(std::fs::metadata(&file).unwrap().len(), 2_036_160);
}

/// US2-S6: the purposes `dark` and `flat` are the console's labels, and the
/// files carry them as `PURPOSE` and `IMAGETYP` (oracle: owner decision 2).
#[tokio::test(start_paused = true)]
async fn dark_and_flat_frames_carry_their_purposes() {
    let scratch = crate::support::Scratch::new("imaging-frames-purposes");
    let root = scratch.0.join("imaging");
    let rig = stand_in_rig(&root).await;
    ready(&rig.app).await;
    put(&rig.app, json!({"exposure_ms":100})).await;

    let view = rig.imaging.take("dark").await.unwrap();
    assert_eq!(view.frame.as_ref().unwrap().purpose, "dark");
    wait_until(&rig.app, |view| view["frame"]["state"] == "saved").await;
    let view = rig.imaging.take("flat").await.unwrap();
    assert_eq!(view.frame.as_ref().unwrap().purpose, "flat");
    wait_until(&rig.app, |view| view["frame"]["state"] == "saved").await;

    let dark = read_fits(&frames(&root).join("000001.fits"));
    assert_eq!(card(&dark.cards, "IMAGETYP"), json!("DARK"));
    assert_eq!(card(&dark.cards, "PURPOSE"), json!("dark"));
    let flat = read_fits(&frames(&root).join("000002.fits"));
    assert_eq!(card(&flat.cards, "IMAGETYP"), json!("FLAT"));
    assert_eq!(card(&flat.cards, "PURPOSE"), json!("flat"));
}
