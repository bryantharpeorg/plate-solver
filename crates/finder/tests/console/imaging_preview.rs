//! The colour preview of each saved imaging frame (031 US6): `rgb`'s
//! superpixel colour, whole-factor box average and asinh stretch against the
//! scenarios' worked pictures, the preview route of the last eight saved
//! frames, and a simulated star's colour. Oracles: the worked examples of
//! plan.md § The preview, the simulator's star, and the JPEG the route
//! answers.

use std::{path::Path, sync::Arc, time::Duration};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{link::Transport, session::Session, AppState};
use image::GenericImageView;
use scope_link::memory;
use scope_node::{
    clock::NodeClock,
    imaging::{synthetic::SyntheticImagingCamera, Exposure, ImagingCamera},
    service::{NodeControls, NodeService},
    synthetic::SyntheticCamera,
    Settings,
};
use serde_json::{json, Value};
use simulator::{mount::SimMount, SimImagingCamera, SimStar, Simulator, VirtualClock};
use tower::ServiceExt;

use crate::support::live as support;

/// 16-bit little-endian samples, row by row from the top-left.
fn sixteen(rows: &[&[u16]]) -> Vec<u8> {
    rows.iter()
        .flat_map(|row| row.iter().flat_map(|sample| sample.to_le_bytes()))
        .collect()
}

/// The picture's pixel at (x, y) as red, green and blue.
fn pixel(picture: &finder::imaging::preview::Rgb, x: u32, y: u32) -> [u8; 3] {
    let index = (y * picture.width + x) as usize * 3;
    [
        picture.pixels[index],
        picture.pixels[index + 1],
        picture.pixels[index + 2],
    ]
}

/// US6-S1: the worked 6 × 6 RGGB frame gives the worked 3 × 3 picture — a
/// half of the span 600 maps to 197 and a quarter to 140, green the mean of
/// its two samples — and the same samples read as BGGR give every pixel with
/// its red and blue swapped (oracle: the worked example of plan.md § The
/// preview).
#[test]
fn the_worked_rggb_frame_gives_the_worked_picture() {
    let samples = sixteen(&[
        &[100, 100, 400, 400, 700, 100],
        &[100, 50, 100, 50, 100, 50],
        &[900, 100, 100, 100, 100, 100],
        &[100, 200, 100, 350, 100, 50],
        &[100, 100, 100, 100, 100, 100],
        &[100, 50, 100, 50, 100, 50],
    ]);
    let picture = finder::imaging::preview::rgb(&samples, 6, 6, 16, Some("RGGB"));
    assert_eq!(picture.width, 3);
    assert_eq!(picture.height, 3);
    assert_eq!(picture.pixels.len(), 27);
    let expected = [
        [0, 0, 0],
        [197, 140, 0],
        [255, 0, 0],
        [255, 0, 140],
        [0, 0, 197],
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
    ];
    for (index, want) in expected.iter().enumerate() {
        let (x, y) = (index % 3, index / 3);
        assert_eq!(&pixel(&picture, x as u32, y as u32), want, "pixel {index}");
    }

    // The same samples read as BGGR: every pixel's red and blue swap.
    let bggr = finder::imaging::preview::rgb(&samples, 6, 6, 16, Some("BGGR"));
    for index in 0..9 {
        let (x, y) = (index % 3, index / 3);
        let [red, green, blue] = pixel(&picture, x as u32, y as u32);
        assert_eq!(
            pixel(&bggr, x as u32, y as u32),
            [blue, green, red],
            "pixel {index}"
        );
    }
}

/// US6-S2: the box average's whole factor, a flat frame's black, a mono
/// frame's gray, a block's mean, and the span's floor of 1 (oracle: the
/// scenario's worked sizes and pictures).
#[test]
fn sizes_means_mono_and_the_span_floor() {
    // A mono 8-bit 5200 × 100 frame: the factor is ⌈5200/1280⌉ = 5.
    let samples: Vec<u8> = (0..5200 * 100).map(|i| (i % 251) as u8).collect();
    let picture = finder::imaging::preview::rgb(&samples, 5200, 100, 8, None);
    assert_eq!((picture.width, picture.height), (1040, 20));

    // A 16-bit RGGB 2600 × 1400 frame: 1300 × 700 superpixels, halved.
    let samples: Vec<u8> = (0..2600u32 * 1400)
        .flat_map(|i| ((i % 1000) as u16).to_le_bytes())
        .collect();
    let picture = finder::imaging::preview::rgb(&samples, 2600, 1400, 16, Some("RGGB"));
    assert_eq!((picture.width, picture.height), (650, 350));

    // A uniform frame gives a picture of all zeros.
    let samples: Vec<u8> = [500u16; 36]
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    let picture = finder::imaging::preview::rgb(&samples, 6, 6, 16, Some("RGGB"));
    assert_eq!((picture.width, picture.height), (3, 3));
    assert!(picture.pixels.iter().all(|&value| value == 0));

    // A mono frame gives equal red, green and blue in every pixel.
    let samples: Vec<u8> = (0..64).map(|i| (i * 4) as u8).collect();
    let picture = finder::imaging::preview::rgb(&samples, 8, 8, 8, None);
    assert!(picture
        .pixels
        .chunks(3)
        .all(|pixel| pixel[0] == pixel[1] && pixel[1] == pixel[2]));
    assert!(picture.pixels.iter().any(|&value| value != 0));

    // A mono 8-bit 2560 × 2 frame whose 2 × 2 blocks hold 0 at their top-left
    // and the block's column index mod 200 in their other three samples:
    // only the mean of a block, not its first sample, varies.
    let mut samples = vec![0u8; 2560 * 2];
    for block in 0..1280u32 {
        let value = (block % 200) as u8;
        samples[2 * block as usize + 1] = value;
        samples[2560 + 2 * block as usize] = value;
        samples[2560 + 2 * block as usize + 1] = value;
    }
    let picture = finder::imaging::preview::rgb(&samples, 2560, 2, 8, None);
    assert_eq!((picture.width, picture.height), (1280, 1));
    assert!(picture.pixels.iter().any(|&value| value != 0));

    // A 16-bit RGGB 6 × 2 frame whose greens are 100, 100 and 100.5: the span
    // is its floor, 1, so the third pixel is (0, 197, 0).
    let samples = sixteen(&[&[0, 100, 0, 100, 0, 101], &[100, 0, 100, 0, 100, 0]]);
    let picture = finder::imaging::preview::rgb(&samples, 6, 2, 16, Some("RGGB"));
    assert_eq!((picture.width, picture.height), (3, 1));
    assert_eq!(pixel(&picture, 0, 0), [0, 0, 0]);
    assert_eq!(pixel(&picture, 1, 0), [0, 0, 0]);
    assert_eq!(pixel(&picture, 2, 0), [0, 197, 0]);
}

/// A node's NodeClock on the simulator's virtual clock, so the stand-in
/// camera's frames follow the rig's time.
struct NodeTime(VirtualClock);
impl NodeClock for NodeTime {
    fn now(&self) -> f64 {
        self.0.node_now()
    }
}

/// The console rig of the shared setup: the sim rig's own node services, a
/// paired session and the imaging folder `root`.
fn state(sim: &Simulator, transport: Transport, root: &Path) -> AppState {
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
    AppState::shared(support::DB.clone(), finder::samples::Samples::default())
        .with_session(session)
        .with_clock(Arc::new(support::ConsoleClock(sim.clock().clone())))
        .with_link(transport)
        .with_live()
        .with_imaging(root)
}

fn sim_rig() -> Simulator {
    let sim = support::sim();
    sim.connect_mount(SimMount::new(sim.clock().clone()));
    sim.push_to(50.0, 120.0);
    sim
}

/// One fresh duplex for each connection, serving the node's own routes.
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

/// The stand-in console rig (the shared setup): the node rig's
/// `SyntheticImagingCamera`, whose clock reads the simulator's virtual clock.
async fn stand_in(root: &Path) -> Router {
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
    finder::router(state(&sim, duplex_service(node), root))
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

/// A GET whose body is bytes, with the answer's `Content-Type`.
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

/// US6-S3: a saved frame's view carries its preview route, the route answers
/// a 32 × 24 JPEG while the frame is among the last eight saved, an unknown
/// id answers 404, an id that is not a number 400, and a 64 × 1 mosaic, with
/// no whole 2 × 2 cell, is saved with no preview (oracles: the stand-in
/// camera's 64 × 48 sensor and the route's contract).
#[tokio::test(start_paused = true)]
async fn a_saved_frame_answers_a_jpeg_preview_of_the_last_eight() {
    let scratch = crate::support::Scratch::new("imaging-preview-route");
    let root = scratch.0.join("imaging");
    let app = stand_in(&root).await;
    wait_until(&app, |view| view["state"] == "ready").await;
    put(&app, json!({"exposure_ms":100})).await;

    let (status, _) = post_frame(&app).await;
    assert_eq!(status, StatusCode::OK);
    let view = wait_until(&app, |view| view["frame"]["state"] == "saved").await;
    assert_eq!(
        view["frame"]["preview"],
        json!("/api/imaging/frames/1/preview.jpg"),
        "{view}"
    );

    let (status, kind, body) = get_bytes(&app, "/api/imaging/frames/1/preview.jpg").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(kind.as_deref(), Some("image/jpeg"));
    let decoded = image::load_from_memory(&body).unwrap();
    assert_eq!(decoded.dimensions(), (32, 24));

    // An id the console no longer holds, and an id that is not a number.
    let (status, _, body) = get_bytes(&app, "/api/imaging/frames/999/preview.jpg").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let error: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        error["error"],
        json!("the console no longer holds that preview")
    );
    let (status, _, _) = get_bytes(&app, "/api/imaging/frames/abc/preview.jpg").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Eight more frames: the first falls out of the last eight, the ninth
    // answers.
    for id in 2..=9u64 {
        let (status, _) = post_frame(&app).await;
        assert_eq!(status, StatusCode::OK);
        wait_until(&app, |view| {
            view["frame"]["id"] == json!(id) && view["frame"]["state"] == "saved"
        })
        .await;
    }
    let (status, _, _) = get_bytes(&app, "/api/imaging/frames/1/preview.jpg").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = get_bytes(&app, "/api/imaging/frames/9/preview.jpg").await;
    assert_eq!(status, StatusCode::OK);

    // A 64 × 1 region: a mosaic with no whole 2 × 2 cell is saved with no
    // preview.
    put(&app, json!({"roi":{"x":0,"y":0,"width":64,"height":1}})).await;
    let (status, _) = post_frame(&app).await;
    assert_eq!(status, StatusCode::OK);
    let view = wait_until(&app, |view| {
        view["frame"]["id"] == json!(10) && view["frame"]["state"] == "saved"
    })
    .await;
    assert!(view["frame"].get("preview").is_none(), "{view}");
}

/// US6-S4: one star of magnitude 5 at the truth's centre for mid-exposure,
/// pure red and then pure blue, on the sim rig's camera with noise on: the
/// saturated pixels sit at the frame's centre, in the star's colour alone
/// (oracle: the simulator's star; a sample four times the sky's noise above
/// the median maps to 93).
#[tokio::test(start_paused = true)]
async fn a_simulated_star_shows_in_its_own_colour() {
    let sim = sim_rig();
    for colour in [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0]] {
        // The star sits at the truth's centre for mid-exposure: the exposure
        // begins the virtual instant after this (plan trap 8).
        let truth = sim.imaging_truth_at(sim.clock().now() + 0.010);
        let camera = SimImagingCamera::new(sim.clone())
            .with_stars(vec![SimStar {
                ra_deg: truth.ra_deg,
                dec_deg: truth.dec_deg,
                magnitude: 5.0,
                colour,
            }])
            .with_noise(true);
        let frame = camera
            .expose(&Exposure {
                exposure_ms: 20.0,
                purpose: "frame".to_string(),
            })
            .await
            .unwrap();
        let picture = finder::imaging::preview::rgb(
            &frame.samples,
            frame.width,
            frame.height,
            frame.bits,
            frame.bayer.map(|pattern| pattern.as_str()),
        );
        assert_eq!((picture.width, picture.height), (618, 411));
        let (star, others) = if colour == [1.0, 0.0, 0.0] {
            (0usize, [1usize, 2usize])
        } else {
            (2usize, [0usize, 1usize])
        };
        let mut saturated = 0;
        for y in 0..411 {
            for x in 0..618 {
                let pixel = pixel(&picture, x, y);
                if pixel[star] == 255 {
                    saturated += 1;
                    assert!(
                        (x as i32 - 309).abs() <= 2 && (y as i32 - 205).abs() <= 2,
                        "a saturated pixel at ({x}, {y})"
                    );
                    for other in others {
                        assert!(pixel[other] <= 100, "pixel at ({x}, {y}): {pixel:?}");
                    }
                }
            }
        }
        assert!(saturated > 0, "no pixel of the star showed at 255");
    }
}
