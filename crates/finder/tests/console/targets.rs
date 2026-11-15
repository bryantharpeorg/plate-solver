//! Target search through the public router, using the pinned OpenNGC extract as oracle.

use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use finder::samples::Samples;
use finder::{router, AppState};
use serde_json::Value;
use tower::ServiceExt;

static APP: LazyLock<Router> = LazyLock::new(|| {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let database = repo.join("reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    let samples = repo.join("reference-solutions/cedar-solve/examples/data/medium_fov");
    router(AppState::new(
        pattern_database::load_from_path(&database).expect("the vendored cedar database"),
        Samples::scan(&samples).expect("the sample frames"),
    ))
});

async fn get(uri: &str) -> (StatusCode, Value) {
    let response = APP
        .clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    assert_eq!(response.headers()["content-type"], "application/json");
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&body).expect("JSON response"),
    )
}

#[tokio::test]
async fn exact_ids_and_common_name_find_andromeda_first() {
    for query in ["m31", "ngc0224", "andromeda"] {
        let (status, body) = get(&format!("/api/targets?q={query}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body[0]["id"], "NGC 224", "{query}: {body}");
        assert_eq!(body[0]["messier"], "M31");
        assert_eq!(body[0]["common_names"][0], "Andromeda Galaxy");
        assert_eq!(body[0]["ra_deg"], 10.68479);
        assert_eq!(body[0]["dec_deg"], 41.26906);
    }
}

#[tokio::test]
async fn an_unknown_name_answers_an_empty_list() {
    let (status, body) = get("/api/targets?q=qqq").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::json!([]));
}

#[tokio::test]
async fn a_missing_empty_or_whitespace_query_is_a_json_bad_request() {
    for uri in ["/api/targets", "/api/targets?q=", "/api/targets?q=%20%20"] {
        let (status, body) = get(uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {body}");
        assert!(body["error"]
            .as_str()
            .is_some_and(|error| !error.is_empty()));
    }
}

#[tokio::test]
async fn default_and_clamped_limits_apply_to_more_than_fifty_matches() {
    // The pinned extract has 60 objects with "Nebula" as a common-name word.
    for (suffix, count) in [("", 20), ("&limit=0", 1), ("&limit=500", 50)] {
        let (status, body) = get(&format!("/api/targets?q=nebula{suffix}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_array().unwrap().len(), count);
        assert_eq!(body[0]["id"], "NGC 3372", "brightest matching nebula");
    }
}

#[tokio::test]
async fn a_malformed_limit_is_a_json_bad_request() {
    for limit in ["-1", "abc", ""] {
        let (status, body) = get(&format!("/api/targets?q=nebula&limit={limit}")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].is_string());
    }
}

#[tokio::test]
async fn search_works_from_an_empty_working_directory() {
    // A child test process changes cwd without changing it underneath parallel tests.
    const EXPECTED: &str = "FINDER_TARGETS_EXPECTED";
    const QUERY: &str = "/api/targets?q=andromeda";
    if let Ok(expected) = std::env::var(EXPECTED) {
        let (status, body) = get(QUERY).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, serde_json::from_str::<Value>(&expected).unwrap());
        return;
    }

    let (status, expected) = get(QUERY).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(expected[0]["id"], "NGC 224");
    let empty = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("targets-empty-cwd-{}", std::process::id()));
    std::fs::create_dir_all(&empty).unwrap();
    assert_eq!(std::fs::read_dir(&empty).unwrap().count(), 0);
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!(
                "{}::search_works_from_an_empty_working_directory",
                module_path!().split_once("::").unwrap().1
            ),
            "--nocapture",
        ])
        .current_dir(&empty)
        .env(EXPECTED, expected.to_string())
        .output()
        .expect("child test process");
    std::fs::remove_dir(&empty).unwrap();
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("test result: ok. 1 passed"),
        "the child ran no test: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
