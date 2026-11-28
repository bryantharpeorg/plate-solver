//! The crop of a preview frame (058 US2): `preview::crop`'s brightest box, its
//! centre and its cells, one picture pixel a cell, on worked frames whose
//! samples are set by hand; and the crop route with the console's last eight
//! crops, on the simulated imaging camera. Oracles: the worked frames' boxes and
//! cells counted by hand (spec.md US2-S1), the scenarios' routes and words, and
//! the simulated imaging camera's 1236 × 822 RGGB sensor
//! (`crates/simulator/src/imaging.rs:145`).

use std::{path::Path, sync::Arc, time::Duration};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{
    imaging::preview::{self, Rgb},
    link::Transport,
    samples::Samples,
    session::Session,
    AppState,
};
use image::GenericImageView;
use scope_link::memory;
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimNode, Simulator};
use tower::ServiceExt;

use crate::support::{live as support, Scratch};

const WHITE: [u8; 3] = [255, 255, 255];
const RED: [u8; 3] = [255, 0, 0];
const BLACK: [u8; 3] = [0, 0, 0];

/// A worked frame whose samples are set by hand: `width` × `height` samples of
/// `background`, 8 or 16 bits, RGGB when `bayer` says so and mono otherwise.
struct Worked {
    width: u32,
    height: u32,
    bits: u32,
    bayer: Option<&'static str>,
    samples: Vec<u16>,
}

impl Worked {
    fn new(
        width: u32,
        height: u32,
        bits: u32,
        bayer: Option<&'static str>,
        background: u16,
    ) -> Self {
        Self {
            width,
            height,
            bits,
            bayer,
            samples: vec![background; (width * height) as usize],
        }
    }

    /// The sample at (x, y) is `value`.
    fn sample(&mut self, x: u32, y: u32, value: u16) {
        self.samples[(y * self.width + x) as usize] = value;
    }

    /// The samples from (x0, y0) to (x1, y1), both inclusive, are `value`.
    fn fill(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, value: u16) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.sample(x, y, value);
            }
        }
    }

    /// The 2 × 2 cell whose top-left sample is (x, y) is `value`.
    fn cell(&mut self, x: u32, y: u32, value: u16) {
        self.fill(x, y, x + 1, y + 1, value);
    }

    /// The 4 × 4 patch of samples whose top-left sample is (x, y) is `value`.
    fn patch(&mut self, x: u32, y: u32, value: u16) {
        self.fill(x, y, x + 3, y + 3, value);
    }

    /// The samples as the frame holds them: a byte each at 8 bits, else two,
    /// little-endian.
    fn bytes(&self) -> Vec<u8> {
        if self.bits == 8 {
            self.samples.iter().map(|&sample| sample as u8).collect()
        } else {
            self.samples
                .iter()
                .flat_map(|sample| sample.to_le_bytes())
                .collect()
        }
    }

    /// The frame's crop, as the preview frame's own crop is made.
    fn crop(&self) -> Rgb {
        preview::crop(
            &self.bytes(),
            self.width,
            self.height,
            self.bits,
            self.bayer,
        )
    }
}

/// The picture's pixel at (x, y), as red, green and blue.
fn pixel(picture: &Rgb, x: u32, y: u32) -> [u8; 3] {
    let index = (y * picture.width + x) as usize * 3;
    [
        picture.pixels[index],
        picture.pixels[index + 1],
        picture.pixels[index + 2],
    ]
}

/// The picture's pixels that are not black, as (x, y, colour), in row order.
fn lit(picture: &Rgb) -> Vec<(u32, u32, [u8; 3])> {
    let mut found = Vec::new();
    for y in 0..picture.height {
        for x in 0..picture.width {
            let colour = pixel(picture, x, y);
            if colour != BLACK {
                found.push((x, y, colour));
            }
        }
    }
    found
}

/// The picture's pixels that are not black are exactly `expected`, each in its
/// colour, whatever order they are named in.
fn assert_lit(picture: &Rgb, expected: &[(u32, u32, [u8; 3])]) {
    let mut want = expected.to_vec();
    want.sort_by_key(|&(x, y, _)| (y, x));
    assert_eq!(lit(picture), want);
}

/// US2-S1: `preview::crop` of seven worked frames gives the worked pictures: the
/// brightest box's centre, the crop moved in to the frame, one picture pixel a
/// cell, stretched over its own samples. A frame with no whole box is centred
/// on itself, and a frame with no whole cell row has no JPEG.
#[test]
fn the_crop_is_centred_on_the_brightest_box_one_pixel_a_cell() {
    // 1000 × 900 RGGB of 100. The patch's box sums to 420,800 against the hot
    // sample's 167,835, so the centre is (688, 304) and the crop starts at
    // (488, 104); the cell at (486, 102) lies outside it.
    let mut frame = Worked::new(1000, 900, 16, Some("RGGB"), 100);
    frame.patch(700, 300, 20000);
    frame.sample(100, 800, 65535);
    frame.cell(488, 104, 1000);
    frame.cell(886, 502, 1000);
    frame.cell(486, 102, 1000);
    frame.sample(588, 304, 1000);
    let picture = frame.crop();
    assert_eq!((picture.width, picture.height), (200, 200));
    assert_lit(
        &picture,
        &[
            (0, 0, WHITE),
            (106, 98, WHITE),
            (107, 98, WHITE),
            (106, 99, WHITE),
            (107, 99, WHITE),
            (50, 100, RED),
            (199, 199, WHITE),
        ],
    );

    // 500 × 300 mono of 10. The patch's box sums to 14,080 against the sample's
    // 10,485, so the centre is (16, 272) and the crop starts at (0, 100); row 299
    // is in no whole box, and the sample of 200 at (200, 299) lies outside it.
    let mut frame = Worked::new(500, 300, 8, None, 10);
    frame.patch(4, 260, 250);
    frame.sample(400, 50, 255);
    frame.sample(0, 100, 200);
    frame.sample(199, 299, 200);
    frame.sample(200, 299, 200);
    let picture = frame.crop();
    assert_eq!((picture.width, picture.height), (200, 200));
    let mut expected = vec![(0, 0, WHITE), (199, 199, WHITE)];
    for y in 160..164 {
        for x in 4..8 {
            expected.push((x, y, WHITE));
        }
    }
    assert_lit(&picture, &expected);

    // 1236 × 822 RGGB of 100, the simulated camera's sensor. Two patches each
    // sum to 420,800: the tie goes to the first in row order, centre (1008, 112),
    // so the crop starts at (808, 0) and the other patch lies outside it.
    let mut frame = Worked::new(1236, 822, 16, Some("RGGB"), 100);
    frame.patch(1000, 100, 20000);
    frame.patch(200, 700, 20000);
    frame.cell(808, 0, 1000);
    frame.cell(1206, 398, 1000);
    let picture = frame.crop();
    assert_eq!((picture.width, picture.height), (200, 200));
    assert_lit(
        &picture,
        &[
            (0, 0, WHITE),
            (96, 50, WHITE),
            (97, 50, WHITE),
            (96, 51, WHITE),
            (97, 51, WHITE),
            (199, 199, WHITE),
        ],
    );

    // 1000 × 900 RGGB of 100 with the patch at (700, 300) and 300s over the block
    // from (0, 600) to (299, 899): those boxes sum to 307,200 at most, under the
    // patch's 420,800. The cell of 150 at (600, 300) is in the crop, and the
    // stretch over the crop's own samples draws it white.
    let mut frame = Worked::new(1000, 900, 16, Some("RGGB"), 100);
    frame.patch(700, 300, 20000);
    frame.fill(0, 600, 299, 899, 300);
    frame.cell(600, 300, 150);
    let picture = frame.crop();
    assert_eq!(pixel(&picture, 56, 98), WHITE);

    // 30 × 1000 mono of 10 has no whole box: the crop is centred on the frame,
    // (15, 500), and is its 30 columns by 200 rows from (0, 400).
    let mut frame = Worked::new(30, 1000, 8, None, 10);
    frame.sample(0, 400, 200);
    frame.sample(29, 599, 200);
    let picture = frame.crop();
    assert_eq!((picture.width, picture.height), (30, 200));
    assert_lit(&picture, &[(0, 0, WHITE), (29, 199, WHITE)]);

    // 30 × 1002 RGGB of 100 has no whole box either, centre (15, 501): the crop
    // is 30 columns by 400 rows from row 300, a whole cell's first row, so the
    // sample at (0, 302) is the red of cell (0, 1).
    let mut frame = Worked::new(30, 1002, 16, Some("RGGB"), 100);
    frame.sample(0, 302, 1000);
    let picture = frame.crop();
    assert_eq!((picture.width, picture.height), (15, 200));
    assert_lit(&picture, &[(0, 1, RED)]);

    // 64 × 1 RGGB has no whole cell row: its crop has no rows and no JPEG.
    let frame = Worked::new(64, 1, 16, Some("RGGB"), 100);
    assert!(preview::jpeg(&frame.crop()).is_none());
}

/// The simulator the crop rig's node serves: its imaging camera's sensor and a
/// mount, as the preview rig sets them.
fn sim_rig() -> Simulator {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}

/// One fresh duplex for each connection, serving the node's own routes.
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

/// The crop rig: the preview rig with no node folder and no solver, the
/// simulated imaging camera at 500 ms a frame, a paired session and the
/// imaging folder `root`.
fn rig(root: &Path) -> Router {
    let sim = sim_rig();
    let node = Arc::new(
        SimNode::new(sim.clone(), "test-token", Default::default())
            .unwrap()
            .with_frame_interval(Duration::from_millis(500)),
    );
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
    let state = AppState::shared(support::DB.clone(), Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(duplex(&node))
        .with_live()
        .with_imaging(root);
    finder::router(state)
}

async fn get(app: &Router, path: &str) -> (StatusCode, Value) {
    support::request(app, "GET", path, json!(null)).await
}

async fn post(app: &Router, path: &str, body: Value) -> (StatusCode, Value) {
    support::request(app, "POST", path, body).await
}

async fn put(app: &Router, body: Value) -> (StatusCode, Value) {
    support::request(app, "PUT", "/api/imaging/settings", body).await
}

/// Read `GET /api/imaging` every 100 ms until `check` holds, for at most
/// 120 s of paused time.
async fn wait_until(app: &Router, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..1200 {
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

/// A GET whose body is bytes, with the answer's content type.
async fn get_bytes(app: &Router, path: &str) -> (StatusCode, Option<String>, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(path)
                .body(Body::from(""))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let kind = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, kind, body)
}

/// The crop route of frame `id` answers 200 with a JPEG of 200 × 200.
async fn assert_crop(app: &Router, id: u64) {
    let (status, kind, body) = get_bytes(app, &format!("/api/imaging/frames/{id}/crop.jpg")).await;
    assert_eq!(status, StatusCode::OK, "frame {id}");
    assert_eq!(kind.as_deref(), Some("image/jpeg"));
    assert_eq!(
        image::load_from_memory(&body).unwrap().dimensions(),
        (200, 200)
    );
}

/// The crop route of frame `id` answers 404 with the words of a crop the
/// console no longer holds.
async fn assert_no_crop(app: &Router, id: u64) {
    let (status, _, body) = get_bytes(app, &format!("/api/imaging/frames/{id}/crop.jpg")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "frame {id}");
    let answer: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        answer["error"],
        json!("the console no longer holds that crop"),
        "{answer}"
    );
}

/// US2-S2: a shown preview frame carries its crop's route, which answers the
/// crop as a 200 × 200 JPEG while the console holds it among the last eight. A
/// saved frame has no crop, an id that is not a number answers 400, and a frame
/// whose region has no whole cell row is shown with neither a preview nor a crop.
#[tokio::test(start_paused = true)]
async fn a_preview_frame_has_a_crop_while_the_console_holds_it() {
    let scratch = Scratch::new("imaging-preview-crop-route");
    let root = scratch.0.join("imaging");
    let app = rig(&root);
    ready(&app).await;
    let (status, answer) = put(&app, json!({"exposure_ms": 100})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    // Frame 1 is shown with its crop's route, which answers the crop.
    let (status, answer) = post(&app, "/api/imaging/preview", json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let view = wait_until(&app, |view| {
        view["frame"]["id"] == json!(1) && view["frame"]["state"] == "shown"
    })
    .await;
    assert_eq!(
        view["frame"]["crop"],
        json!("/api/imaging/frames/1/crop.jpg"),
        "{view}"
    );
    assert_crop(&app, 1).await;

    // Frame 2 is saved: it has no crop, and its crop route answers 404.
    let (status, answer) = post(&app, "/api/imaging/frame", json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let view = wait_until(&app, |view| {
        view["frame"]["id"] == json!(2) && view["frame"]["state"] == "saved"
    })
    .await;
    assert!(view["frame"].get("crop").is_none(), "{view}");
    assert_no_crop(&app, 2).await;

    // Eight more preview frames, each shown before the next: the console keeps
    // the last eight crops, so frame 1's is let go and frames 3 and 10 stay.
    for id in 3..=10 {
        let (status, answer) = post(&app, "/api/imaging/preview", json!(null)).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        wait_until(&app, |view| {
            view["frame"]["id"] == json!(id) && view["frame"]["state"] == "shown"
        })
        .await;
    }
    assert_no_crop(&app, 1).await;
    assert_crop(&app, 3).await;
    assert_crop(&app, 10).await;
    let (status, _, _) = get_bytes(&app, "/api/imaging/frames/abc/crop.jpg").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A region of 64 × 1 has no whole cell row: frame 11 is shown with neither
    // a preview nor a crop.
    let (status, answer) = put(
        &app,
        json!({"roi": {"x": 0, "y": 0, "width": 64, "height": 1}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let (status, answer) = post(&app, "/api/imaging/preview", json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let view = wait_until(&app, |view| {
        view["frame"]["id"] == json!(11) && view["frame"]["state"] == "shown"
    })
    .await;
    assert!(view["frame"].get("preview").is_none(), "{view}");
    assert!(view["frame"].get("crop").is_none(), "{view}");
}
