//! The console's API end to end, on the real database and cedar's sample frames.

use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use axum::body::{to_bytes, Body, Bytes};
use axum::http::{Request, StatusCode};
use axum::response::Response;
use axum::Router;
use finder::samples::Samples;
use finder::stars::NAMED_STARS;
use finder::{router, AppState};
use futures_util::{stream, StreamExt};
use pattern_database::{CatalogId, PatternDatabase, StarId};
use serde_json::Value;
use tokio::sync::Notify;
use tower::ServiceExt;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn samples_dir() -> PathBuf {
    repo().join("reference-solutions/cedar-solve/examples/data/medium_fov")
}

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = repo().join("reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).expect("the vendored cedar database"))
});

fn state() -> AppState {
    AppState::shared(
        DB.clone(),
        Samples::scan(&samples_dir()).expect("the sample frames"),
    )
}

fn app() -> Router {
    router(state())
}

async fn send(request: Request<Body>) -> (StatusCode, String, Vec<u8>) {
    parts(app().oneshot(request).await.unwrap()).await
}

/// A response's status, content type and body.
async fn parts(response: Response) -> (StatusCode, String, Vec<u8>) {
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, content_type, body.to_vec())
}

async fn get(uri: &str) -> (StatusCode, String, Vec<u8>) {
    send(Request::get(uri).body(Body::empty()).unwrap()).await
}

async fn get_json(uri: &str) -> (StatusCode, Value) {
    let (status, _, body) = get(uri).await;
    (status, serde_json::from_slice(&body).expect("JSON body"))
}

/// Form text fields, `(name, value)`.
type Fields<'a> = &'a [(&'a str, &'a str)];
/// An uploaded frame, `(file name, bytes)`.
type Upload<'a> = Option<(&'a str, &'a [u8])>;

const BOUNDARY: &str = "finder-test-boundary";

/// A `/api/solve` request from text fields and at most one file.
fn form(fields: Fields, file: Upload) -> Request<Body> {
    solve_request(Body::from(form_body(fields, file)))
}

/// A `multipart/form-data` body from text fields and at most one file.
fn form_body(fields: Fields, file: Upload) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .bytes(),
        );
    }
    if let Some((file_name, bytes)) = file {
        body.extend(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"image\"; \
                 filename=\"{file_name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend(b"\r\n");
    }
    body.extend(format!("--{BOUNDARY}--\r\n").bytes());
    body
}

/// A `/api/solve` request with a multipart body.
fn solve_request(body: Body) -> Request<Body> {
    Request::post("/api/solve")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(body)
        .unwrap()
}

/// A request body that hands over `bytes` when the server first reads it and signals `read`;
/// with `stall`, it then sends nothing more, like a client whose upload has stopped.
fn watched_body(bytes: Vec<u8>, read: Arc<Notify>, stall: bool) -> Body {
    let first = stream::once(async move {
        read.notify_one();
        Ok::<_, Infallible>(Bytes::from(bytes))
    });
    if stall {
        Body::from_stream(first.chain(stream::pending()))
    } else {
        Body::from_stream(first)
    }
}

async fn solve(fields: Fields<'_>, file: Upload<'_>) -> (StatusCode, Value) {
    let (status, _, body) = send(form(fields, file)).await;
    (status, serde_json::from_slice(&body).expect("JSON body"))
}

fn ids(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|o| o["id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn health_reports_the_database() {
    let (status, json) = get_json("/api/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "ok");
    assert_eq!(json["database"]["stars"], 42212);
    assert_eq!(json["database"]["min_fov"], 10.0);
    assert_eq!(json["database"]["max_fov"], 30.0);
    assert_eq!(json["samples"], 21);
}

#[tokio::test]
async fn samples_are_listed_served_and_previewed() {
    let (status, json) = get_json("/api/samples").await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 21, "the output/ folder is not a sample");
    assert!(names.contains(&"orion_belt.jpg"));
    assert!(json
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["label"] == "Orion belt"));

    let (status, content_type, body) = get("/api/samples/orion_belt.jpg").await;
    assert_eq!(
        (status, content_type.as_str()),
        (StatusCode::OK, "image/jpeg")
    );
    let original = std::fs::read(samples_dir().join("orion_belt.jpg")).unwrap();
    assert_eq!(body, original);

    let (status, content_type, thumb) = get("/api/samples/orion_belt.jpg/thumb").await;
    assert_eq!(
        (status, content_type.as_str()),
        (StatusCode::OK, "image/jpeg")
    );
    let preview = image::load_from_memory(&thumb).unwrap();
    // Whole-block shrinking: at most 320 pixels, and never less than half that.
    let side = preview.width().max(preview.height());
    assert!((160..=320).contains(&side), "{side}");

    for uri in [
        "/api/samples/missing.jpg",
        "/api/samples/..%2F..%2FCargo.toml",
        "/api/samples/missing.jpg/thumb",
        "/api/nothing-here",
    ] {
        let (status, json) = get_json(uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert!(json["error"].is_string(), "{uri}");
    }
}

/// Solving the orion2 sample gives the `solve_image` example's answer, and every matched
/// star's catalog position projects back onto the star it was matched to: the overlay the
/// page draws is the solution, not a picture of it.
#[tokio::test]
async fn a_sample_solves_and_its_overlay_lands_on_its_stars() {
    let (status, r) = solve(&[("sample", "orion2.jpg")], None).await;
    assert_eq!(status, StatusCode::OK, "{r}");
    assert_eq!(r["status"], "match_found");
    assert_eq!(r["name"], "orion2.jpg");
    assert_eq!(
        (r["width"].as_u64(), r["height"].as_u64()),
        (Some(507), Some(507))
    );
    assert!(r["hint"].is_null());

    let p = &r["pointing"];
    // `solve_image --json` on this frame: ra 85.190964, dec -4.246105, fov 10.181431.
    assert!(
        (p["ra_deg"].as_f64().unwrap() - 85.190_964).abs() < 1e-4,
        "{p}"
    );
    assert!(
        (p["dec_deg"].as_f64().unwrap() + 4.246_105).abs() < 1e-4,
        "{p}"
    );
    assert!(
        (p["fov_deg"].as_f64().unwrap() - 10.181_431).abs() < 1e-4,
        "{p}"
    );
    assert_eq!(p["ra_text"], "05h 40m 46s");
    assert_eq!(p["dec_text"], "\u{2212}04° 14′ 46″");
    assert_eq!(p["constellation"]["name"], "Orion");
    assert_eq!(p["matched"], 30);

    let stars = r["stars"].as_array().unwrap();
    let matched: Vec<&Value> = stars.iter().filter(|s| s["matched"] == true).collect();
    assert_eq!(matched.len(), 30);
    // Allow the solution's worst residual plus half a pixel of centroid noise.
    let max_px = p["max_arcsec"].as_f64().unwrap() / p["arcsec_per_px"].as_f64().unwrap() + 0.5;
    for s in &matched {
        let error = s["error_px"].as_f64().unwrap();
        assert!(
            error < max_px,
            "{} is {error:.2} px off (limit {max_px:.2})",
            s["id"]
        );
    }
    let names: Vec<&str> = stars.iter().filter_map(|s| s["name"].as_str()).collect();
    for star in ["Alnitak", "Alnilam", "Mintaka"] {
        assert!(names.contains(&star), "{star} not labelled: {names:?}");
    }
    let (w, h) = (507.0, 507.0);
    for s in stars.iter().filter(|s| s["matched"] == false) {
        let (x, y) = (s["x"].as_f64().unwrap(), s["y"].as_f64().unwrap());
        assert!(
            (0.0..w).contains(&x) && (0.0..h).contains(&y),
            "expected star off frame"
        );
    }

    let objects = ids(&r["objects"]);
    assert_eq!(objects, ["M42", "M78", "M43"], "brightest first");
    let m42 = &r["objects"][0];
    assert_eq!(m42["name"], "Great Orion Nebula");
    assert_eq!(m42["inside"], true);
    // 90′ across is a 45′ radius: 2700″ at 72″ a pixel.
    let radius = m42["radius_px"].as_f64().unwrap();
    assert!((36.0..39.0).contains(&radius), "M42 radius {radius} px");

    assert!(r["detected"].as_array().unwrap().len() >= 30);
    assert_eq!(r["used"], 30);
    let t = &r["timing_ms"];
    assert!(t["decode"].as_f64().unwrap() > 0.0 && t["solve"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn an_upload_solves_and_finds_what_is_in_it() {
    let bytes = std::fs::read(samples_dir().join("pleiades.jpg")).unwrap();
    let (status, r) = solve(
        &[("fov", "10"), ("fov_max_error", "2")],
        Some(("mine.jpg", &bytes)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{r}");
    assert_eq!(r["status"], "match_found");
    assert_eq!(r["name"], "mine.jpg");
    assert_eq!(r["pointing"]["constellation"]["abbr"], "Tau");
    assert!(ids(&r["objects"]).contains(&"M45".to_string()));
    let names: Vec<&str> = r["stars"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["name"].as_str())
        .collect();
    assert!(names.contains(&"Alcyone"), "{names:?}");
}

#[tokio::test]
async fn a_frame_that_does_not_solve_says_what_to_try() {
    let (status, r) = solve(&[("sample", "roof.jpg")], None).await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(r["status"], "match_found");
    assert!(r["hint"].as_str().unwrap().len() > 20);
    assert!(r["pointing"].is_null());
    assert!(r["stars"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn bad_requests_are_refused_with_reasons() {
    let cases: [(Fields, Upload, StatusCode); 6] = [
        (&[], None, StatusCode::BAD_REQUEST),
        (
            &[("sample", "orion2.jpg"), ("bin", "3")],
            None,
            StatusCode::BAD_REQUEST,
        ),
        (
            &[("sample", "orion2.jpg"), ("sigma", "lots")],
            None,
            StatusCode::BAD_REQUEST,
        ),
        (
            &[("sample", "orion2.jpg"), ("fov", "200")],
            None,
            StatusCode::BAD_REQUEST,
        ),
        (&[("sample", "nope.jpg")], None, StatusCode::NOT_FOUND),
        (
            &[],
            Some(("x.jpg", b"not a jpeg")),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
    ];
    for (fields, file, expected) in cases {
        let (status, json) = solve(fields, file).await;
        assert_eq!(status, expected, "{fields:?}: {json}");
        assert!(json["error"].as_str().is_some_and(|e| !e.is_empty()));
    }
}

#[tokio::test]
async fn similar_objects_rank_for_an_observer() {
    let (status, json) = get_json("/api/similar?id=m42").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["reference"]["id"], "M42");
    assert_eq!(json["family"], "diffuse nebulae");
    assert_eq!(json["suggestions"][0]["id"], "M78");

    // 40° N, 75° W at 9 pm on 2026-01-15.
    let (status, json) =
        get_json("/api/similar?id=M42&lat=40&lon=-75&time=1768528800&limit=3").await;
    assert_eq!(status, StatusCode::OK);
    let s = json["suggestions"].as_array().unwrap();
    assert_eq!(s.len(), 3);
    assert_eq!(
        (s[0]["id"].as_str(), s[0]["up"].as_bool()),
        (Some("M78"), Some(true))
    );
    assert!(s[0]["alt_deg"].as_f64().unwrap() > 40.0);
    assert_eq!(json["observer"]["lat"], 40.0);

    for (uri, expected) in [
        ("/api/similar?id=M200", StatusCode::NOT_FOUND),
        ("/api/similar?id=M42&lat=95&lon=0", StatusCode::BAD_REQUEST),
        ("/api/similar?id=M42&lat=40", StatusCode::BAD_REQUEST),
        ("/api/similar", StatusCode::BAD_REQUEST),
    ] {
        let (status, _, _) = get(uri).await;
        assert_eq!(status, expected, "{uri}");
    }
}

#[tokio::test]
async fn the_page_is_served_and_routes_fall_back_to_it() {
    for uri in ["/", "/index.html", "/some/route"] {
        let (status, content_type, body) = get(uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}");
        assert!(
            content_type.starts_with("text/html"),
            "{uri}: {content_type}"
        );
        assert!(String::from_utf8(body)
            .unwrap()
            .contains("<div id=\"root\">"));
    }
    let (status, _, _) = get("/assets/missing.js").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Every named star's HIP number is in the database at the position written beside it, so
/// a wrong number cannot put the wrong name on a frame.
#[test]
fn named_stars_match_the_database() {
    let db = &**DB;
    for &(hip, name, ra, dec) in NAMED_STARS {
        let index = db
            .star_catalog_ids
            .iter()
            .position(|id| *id == CatalogId::Hip(hip))
            .unwrap_or_else(|| panic!("{name} (HIP {hip}) is not in the database"));
        let star = db.star_vector(StarId(index)).unwrap();
        let listed = math_core::UnitVector::from_radec(ra.to_radians(), dec.to_radians());
        let off = math_core::angular_distance(star, listed).to_degrees();
        assert!(
            off < 0.05,
            "{name} (HIP {hip}) is {off:.3}° from its listed position"
        );
    }
}

/// Every sample frame lands where cedar put it (the plate-solver crate's goldens): the same
/// status, the same pointing and field, and cedar's matched stars matched again, within the
/// image tier's tolerances (10″, 15″ of field, 90% of cedar's stars). Measured: 19 of the 20
/// within 0.3″ and every one of cedar's stars; test2_g100_e50ms 4.2″, 9″ of field, 29 of 30.
/// The console decodes and detects as cedar does, so it shows cedar's answer.
#[tokio::test]
async fn every_sample_gives_cedars_answer() {
    let goldens: Value = serde_json::from_slice(
        &std::fs::read(repo().join("crates/plate-solver/tests/data/cedar_parity.json")).unwrap(),
    )
    .unwrap();
    let mut solved = 0;
    for case in goldens["cases"].as_array().unwrap() {
        let name = format!("{}.jpg", case["image"].as_str().unwrap());
        let cedar = &case["cedar"];
        let (status, r) = solve(&[("sample", &name)], None).await;
        assert_eq!(status, StatusCode::OK, "{name}: {r}");
        if cedar["status"] != "MATCH_FOUND" {
            assert_ne!(r["status"], "match_found", "{name} solved; cedar's did not");
            continue;
        }
        assert_eq!(r["status"], "match_found", "{name}: {r}");
        solved += 1;
        let p = &r["pointing"];
        let off = arcsec_between(
            (
                p["ra_deg"].as_f64().unwrap(),
                p["dec_deg"].as_f64().unwrap(),
            ),
            (
                cedar["ra"].as_f64().unwrap(),
                cedar["dec"].as_f64().unwrap(),
            ),
        );
        assert!(off < 10.0, "{name} points {off:.1}″ from cedar");
        let fov = (p["fov_deg"].as_f64().unwrap() - cedar["fov"].as_f64().unwrap()).abs() * 3600.0;
        assert!(fov < 15.0, "{name}'s field is {fov:.1}″ from cedar's");
        let matched: Vec<String> = r["stars"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["matched"] == true)
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect();
        let hip = cedar["hip"].as_array().unwrap();
        let again = hip
            .iter()
            .filter(|h| matched.contains(&format!("HIP {}", h.as_u64().unwrap())))
            .count();
        assert!(
            again * 10 >= hip.len() * 9,
            "{name}: {again} of cedar's {} stars matched",
            hip.len()
        );
    }
    assert_eq!(solved, 20, "cedar solves 20 of the 21 samples");
}

/// Angle between two J2000 positions in degrees, in arcseconds.
fn arcsec_between((ra1, dec1): (f64, f64), (ra2, dec2): (f64, f64)) -> f64 {
    let a = math_core::UnitVector::from_radec(ra1.to_radians(), dec1.to_radians());
    let b = math_core::UnitVector::from_radec(ra2.to_radians(), dec2.to_radians());
    math_core::angular_distance(a, b).to_degrees() * 3600.0
}

/// A body over the 32 MiB limit is refused before anything is decoded.
#[tokio::test]
async fn oversized_uploads_are_refused() {
    let big = vec![0u8; finder::api::SOLVE_BODY_LIMIT + 1];
    let (status, json) = solve(&[], Some(("huge.jpg", &big))).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(json["error"].as_str().is_some_and(|e| !e.is_empty()));
}

/// Two frames posted at once are both answered, each with its own solution: the solve gate
/// queues one behind the other rather than mixing or dropping them.
#[tokio::test]
async fn frames_posted_together_are_each_answered() {
    let ((s1, orion), (s2, pleiades)) = tokio::join!(
        solve(&[("sample", "orion2.jpg")], None),
        solve(&[("sample", "pleiades.jpg")], None),
    );
    assert_eq!((s1, s2), (StatusCode::OK, StatusCode::OK));
    assert_eq!(orion["pointing"]["constellation"]["abbr"], "Ori");
    assert_eq!(pleiades["pointing"]["constellation"]["abbr"], "Tau");
}

/// A request takes the solve gate before its form is read, so a queued upload stays on the
/// network; a client that stops sending while it holds the gate is answered 408 when the
/// form timeout runs out, and the queue moves on.
#[tokio::test]
async fn a_stalled_upload_times_out_and_the_queue_moves_on() {
    let timeout = Duration::from_secs(1);
    let app = router(state().with_form_timeout(timeout));
    let orion = form_body(&[("sample", "orion2.jpg")], None);

    let stalled_read = Arc::new(Notify::new());
    let half = orion[..orion.len() / 2].to_vec();
    let stalled = tokio::spawn(app.clone().oneshot(solve_request(watched_body(
        half,
        stalled_read.clone(),
        true,
    ))));
    // Its form is being read, so it holds the gate.
    stalled_read.notified().await;

    let queued_read = Arc::new(Notify::new());
    let queued = tokio::spawn(app.oneshot(solve_request(watched_body(
        orion,
        queued_read.clone(),
        false,
    ))));
    let unread = tokio::time::timeout(timeout / 4, queued_read.notified()).await;
    assert!(unread.is_err(), "the queued form was read before its turn");

    let (status, _, body) = parts(stalled.await.unwrap().unwrap()).await;
    assert_eq!(status, StatusCode::REQUEST_TIMEOUT);
    let json: Value = serde_json::from_slice(&body).expect("JSON body");
    assert!(json["error"].as_str().is_some_and(|e| !e.is_empty()));

    let (status, _, body) = parts(queued.await.unwrap().unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    let json: Value = serde_json::from_slice(&body).expect("JSON body");
    assert_eq!(json["pointing"]["constellation"]["abbr"], "Ori");
}

/// The objects just off the frame: none of them on it, nearest first, at most eight, each
/// with the way to turn.
#[tokio::test]
async fn objects_just_off_the_frame_are_listed_nearest_first() {
    let (_, r) = solve(&[("sample", "orion2.jpg")], None).await;
    let nearby = r["nearby"].as_array().unwrap();
    assert!(!nearby.is_empty() && nearby.len() <= 8, "{}", nearby.len());
    let on_frame = ids(&r["objects"]);
    let mut last = 0.0;
    for o in nearby {
        assert!(!on_frame.contains(&o["id"].as_str().unwrap().to_string()));
        let sep = o["separation_deg"].as_f64().unwrap();
        assert!(sep >= last, "not nearest first");
        last = sep;
        let dir = o["direction_deg"].as_f64().unwrap();
        assert!((0.0..360.0).contains(&dir));
    }
    // M79, in Lepus, lies about 20° south of Orion's belt.
    let m79 = nearby
        .iter()
        .find(|o| o["id"] == "M79")
        .expect("M79 nearby");
    assert!((15.0..25.0).contains(&m79["separation_deg"].as_f64().unwrap()));
}
