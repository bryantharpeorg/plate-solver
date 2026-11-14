//! Spec 020 US1: catalog membership comes from brightness, and detection from the
//! report's own centroids. The solver's matched set keeps its separate meaning.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use finder::samples::Samples;
use finder::sky::Frame;
use finder::solve::{self, SolveOptions, SolveReport, Status};
use finder::{router, AppState};
use pattern_database::{CatalogId, PatternDatabase, StarId};
use scope_link::proto::Centroid;
use serde_json::Value;
use tower::ServiceExt;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn samples_dir() -> PathBuf {
    repo().join("reference-solutions/cedar-solve/examples/data/medium_fov")
}

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = repo().join("reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).expect("the bundled database"))
});

/// The source detections stay in brightness order, including those the solver did
/// not use. Both comparisons use the default solve settings, changing only the list.
static CENTROID_REPORTS: LazyLock<(SolveReport, Frame, SolveReport)> = LazyLock::new(|| {
    let bytes = std::fs::read(samples_dir().join("orion2.jpg")).unwrap();
    let image = solve::decode_gray(&bytes).unwrap();
    let options = SolveOptions::default();
    let source = solve::solve_frame(&DB, &image, &options);
    assert_eq!(source.status, Status::MatchFound);
    let detections: Vec<_> = source
        .detected
        .iter()
        .map(|&[x, y]| Centroid {
            x,
            y,
            brightness: 0.0,
        })
        .collect();
    assert!(detections.len() > 30);
    let size = (image.width, image.height);
    let (full, frame) = solve::solve_centroids(&DB, &detections, size, &options);
    let (trimmed, _) = solve::solve_centroids(&DB, &detections[2..], size, &options);
    assert_eq!(full.status, Status::MatchFound);
    assert_eq!(trimmed.status, Status::MatchFound);
    (full, frame.expect("the first solved frame"), trimmed)
});

async fn api_report() -> Value {
    let state = AppState::shared(DB.clone(), Samples::scan(&samples_dir()).unwrap());
    let request = Request::post("/api/solve")
        .header("content-type", "multipart/form-data; boundary=star-test")
        .body(Body::from(
            "--star-test\r\nContent-Disposition: form-data; name=\"sample\"\r\n\r\n\
             orion2.jpg\r\n--star-test--\r\n",
        ))
        .unwrap();
    let response = router(state).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let report: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(report["status"], "match_found");
    report
}

/// Independent membership oracle: the bundled database is ordered by brightness.
/// Project catalog positions, retain those inside the solved frame, then take 60.
fn brightest_in_frame(frame: &Frame) -> Vec<(String, (f64, f64))> {
    let stars: Vec<_> = DB
        .nearby_stars(frame.center, frame.half_diagonal())
        .into_iter()
        .filter_map(|index| {
            let point = frame.project(DB.star_vector(StarId(index)).unwrap())?;
            frame.contains(point, 0.0).then(|| {
                let id = match DB.star_catalog_ids[index] {
                    CatalogId::Hip(n) => format!("HIP {n}"),
                    CatalogId::Bsc(n) => format!("HR {n}"),
                    CatalogId::Tyc(a, b, c) => format!("TYC {a}-{b}-{c}"),
                };
                (id, point)
            })
        })
        .take(60)
        .collect();
    assert_eq!(stars.len(), 60, "the fixture must exercise the full field");
    stars
}

/// US1-S1: the pixel-distance oracle has a 2.0 px inclusive radius. In particular,
/// detection includes stars beyond the solver's matched set, without false positives.
#[tokio::test]
async fn detected_marks_agree_with_all_reported_detections() {
    let report = api_report().await;
    let stars = report["stars"].as_array().unwrap();
    let detections = report["detected"].as_array().unwrap();
    assert!(!stars.is_empty());
    let mut unmatched_detected = 0;
    let mut undetected = 0;
    for star in stars {
        let matched = star["matched"].as_bool().unwrap();
        let detected = star["detected"].as_bool().expect("a boolean on every mark");
        let (x, y) = (star["x"].as_f64().unwrap(), star["y"].as_f64().unwrap());
        let nearby = detections
            .iter()
            .any(|d| (d[0].as_f64().unwrap() - x).hypot(d[1].as_f64().unwrap() - y) <= 2.0);
        assert!(!matched || detected, "matched star {}", star["id"]);
        assert_eq!(detected, nearby, "2 px detection for {}", star["id"]);
        unmatched_detected += usize::from(!matched && detected);
        undetected += usize::from(!detected);
    }
    assert!(
        unmatched_detected > 0,
        "detections beyond the solver's matches"
    );
    assert!(
        undetected > 0,
        "also exercise marks without a nearby detection"
    );
}

/// US1-S2: removing the two brightest detections changes the solver's selection,
/// but catalog membership stays. The oracle is the first frame's 60 brightest;
/// exclude its 2 px edge margin and allow strictly less than 1 px on each axis.
#[test]
fn field_stars_stay_when_the_two_brightest_detections_are_removed() {
    let (full, frame, trimmed) = &*CENTROID_REPORTS;
    let brightest = brightest_in_frame(frame);
    let interior: Vec<_> = brightest
        .iter()
        .filter(|(_, point)| frame.contains(*point, -2.0))
        .collect();
    assert!(!interior.is_empty());
    for (id, _) in interior {
        let a = full.stars.iter().find(|s| &s.id == id).expect(id);
        let b = trimmed.stars.iter().find(|s| &s.id == id).expect(id);
        assert!((a.x - b.x).abs() < 1.0, "{id}: x moved by >= 1 px");
        assert!((a.y - b.y).abs() < 1.0, "{id}: y moved by >= 1 px");
    }
}

/// US1-S3: exact existing solver counts, with no tolerance. Matched marks still
/// precede the additional field stars.
#[tokio::test]
async fn solver_matched_counts_remain_thirty() {
    let report = api_report().await;
    let stars = report["stars"].as_array().unwrap();
    assert_eq!(report["pointing"]["matched"], 30);
    assert_eq!(stars.iter().filter(|s| s["matched"] == true).count(), 30);
    assert!(stars[..30].iter().all(|s| s["matched"] == true));
    assert!(stars[30..].iter().all(|s| s["matched"] == false));
}

/// US1-S4: exact ordered catalog IDs, independently selected by brightness before
/// subtracting the API report's solver matches. No extra unmatched stars are allowed.
#[tokio::test]
async fn unmatched_marks_are_the_sixty_brightest_less_the_matches() {
    let report = api_report().await;
    let stars = report["stars"].as_array().unwrap();
    let matched: HashSet<_> = stars
        .iter()
        .filter(|s| s["matched"] == true)
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    let (_, frame, _) = &*CENTROID_REPORTS;
    let expected: Vec<_> = brightest_in_frame(frame)
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| !matched.contains(id.as_str()))
        .collect();
    let actual: Vec<_> = stars
        .iter()
        .filter(|s| s["matched"] == false)
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(actual, expected, "brightest first, exact membership");
}
