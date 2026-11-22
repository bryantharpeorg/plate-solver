//! US1-S4–S8. Oracles: cedar's metadata and sample, BSC5's count, the generated
//! ranges and explicitly supplied names. Metadata and statuses compare exactly.

use super::{databases, MID, NARROW, WIDE};
use crate::support::live::{request, DB};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use finder::{router, samples::Samples, AppState};
use serde_json::{json, Value};
use std::{io::Cursor, path::PathBuf};
use tower::ServiceExt;

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/examples/data/medium_fov")
}

fn state() -> AppState {
    AppState::shared(DB.clone(), Samples::scan(&samples_dir()).unwrap())
}

fn several() -> Router {
    router(state().with_databases(databases(&[("narrow", &NARROW), ("cedar", &DB)])))
}

async fn upload(app: &Router, name: &str, bytes: &[u8]) -> Value {
    let mut body = format!(
        "--database-upload\r\nContent-Disposition: form-data; name=\"image\"; \
         filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n--database-upload--\r\n");
    let response = app
        .clone()
        .oneshot(
            Request::post("/api/solve")
                .header(
                    "content-type",
                    "multipart/form-data; boundary=database-upload",
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

fn black() -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    image::GrayImage::new(64, 64)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

#[tokio::test]
async fn health_lists_every_database_in_given_order_and_keeps_the_first() {
    let app = router(state().with_databases(databases(&[
        ("narrow", &NARROW),
        ("cedar", &DB),
        ("mid", &MID),
        ("wide", &WIDE),
    ])));
    let (status, health) = request(&app, "GET", "/api/health", json!(null)).await;
    assert_eq!(status, StatusCode::OK);
    let entries = health["databases"].as_array().unwrap();
    assert_eq!(entries.len(), 4);
    for (entry, (name, catalog, min, max)) in entries.iter().zip([
        ("narrow", "hip_main", 4.5, 7.0),
        ("cedar", "hip_main", 10.0, 30.0),
        ("mid", "bsc5", 20.0, 30.0),
        ("wide", "bsc5", 35.0, 50.0),
    ]) {
        assert_eq!(entry["name"], name);
        assert_eq!(entry["catalog"], catalog);
        assert_eq!(entry["min_fov"], min);
        assert_eq!(entry["max_fov"], max);
    }
    assert_eq!(entries[1]["stars"], 42212);
    assert_eq!(entries[1]["patterns"], 1010981);
    assert_eq!(entries[2]["stars"], 9096);
    assert_eq!(health["database"]["min_fov"], 4.5);
    assert_eq!(health["database"]["max_fov"], 7.0);
}

#[tokio::test]
async fn upload_tries_the_next_database_until_it_solves() {
    let bytes = std::fs::read(samples_dir().join("orion_belt.jpg")).unwrap();
    let report = upload(&several(), "orion_belt.jpg", &bytes).await;
    assert_eq!(report["status"], "match_found");
    assert_eq!(report["database"], "cedar");
}

#[tokio::test]
async fn shared_without_a_list_names_the_star_catalog() {
    let bytes = std::fs::read(samples_dir().join("orion_belt.jpg")).unwrap();
    let report = upload(&router(state()), "orion_belt.jpg", &bytes).await;
    assert_eq!(report["status"], "match_found");
    assert_eq!(report["database"], "hip_main");
}

#[tokio::test]
async fn too_few_ends_the_tries_at_the_first_database() {
    let report = upload(&several(), "black.png", &black()).await;
    assert_eq!(report["status"], "too_few");
    assert_eq!(report["database"], "narrow");
}

#[tokio::test]
async fn a_solved_upload_does_not_change_the_database_in_use() {
    let app = several();
    let bytes = std::fs::read(samples_dir().join("orion_belt.jpg")).unwrap();
    let solved = upload(&app, "orion_belt.jpg", &bytes).await;
    assert_eq!(solved["status"], "match_found");
    assert_eq!(solved["database"], "cedar");
    let next = upload(&app, "black.png", &black()).await;
    assert_eq!(next["status"], "too_few");
    assert_eq!(next["database"], "narrow");
}
