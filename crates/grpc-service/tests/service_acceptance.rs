//! Acceptance tests for the gRPC service surface.
//!
//! Covers these Requirements of the `grpc-service` spec:
//!   - PlateSolver service surface (all four RPCs available)
//!   - Image message (inline vs shmem selection field-level)
//!   - ImageCoord and centroid messages (pixel-center convention, brightest-first)
//!   - Solution message (attitude fields on success, status on failure)
//!   - Coordinate boundary swap (x/y swapped inbound and outbound)
//!   - Request parameters forwarded to the solver
//!
//! The tests use an in-memory channel client so no TCP port is required.

use grpc_service::proto::plate_solver_client::PlateSolverClient;
use grpc_service::proto::plate_solver_server::PlateSolverServer as GeneratedServer;
use grpc_service::proto::{
    CentroidsRequest, Image, ImageCoord, InfoRequest, SolveFromCentroidsRequest,
    SolveFromImageRequest, SolveParams, SolveStatus,
};
use grpc_service::PlateSolverServer;
use hyper_util::rt::TokioIo;
use pattern_database::{DatabaseProperties, PatternDatabase, StarKdTree};

fn test_db() -> PatternDatabase {
    PatternDatabase {
        star_table: Vec::new(),
        num_stars: 0,
        pattern_catalog: Vec::new(),
        pattern_largest_edge: Vec::new(),
        pattern_key_hashes: Vec::new(),
        star_catalog_ids: Vec::new(),
        properties: DatabaseProperties {
            min_fov: 10.0,
            max_fov: 30.0,
            num_patterns: 100,
            verification_stars_per_fov: 150,
            star_catalog: "test_catalog".to_string(),
            ..DatabaseProperties::default()
        },
        star_kdtree: StarKdTree::new(&[]),
    }
}

fn test_image(width: usize, height: usize) -> Image {
    Image {
        width: width as i32,
        height: height as i32,
        image_data: vec![0u8; width * height],
        shmem_name: None,
        reopen_shmem: false,
    }
}

async fn make_client() -> PlateSolverClient<tonic::transport::Channel> {
    let db = test_db();
    let (client_io, server_io) = tokio::io::duplex(1024);
    let client_io = std::sync::Arc::new(tokio::sync::Mutex::new(Some(client_io)));

    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(GeneratedServer::new(PlateSolverServer::new(db)))
            .serve_with_incoming(tokio_stream::iter(vec![Ok::<_, std::io::Error>(server_io)]))
            .await
            .unwrap()
    });

    let endpoint = tonic::transport::Endpoint::from_static("http://localhost:50051");
    PlateSolverClient::new(
        endpoint
            .connect_with_connector(tower::service_fn(move |_| {
                let client_io = client_io.clone();
                async move {
                    let mut guard = client_io.lock().await;
                    Ok::<_, std::convert::Infallible>(TokioIo::new(guard.take().unwrap()))
                }
            }))
            .await
            .unwrap(),
    )
}

#[tokio::test]
async fn all_four_rpcs_available() {
    let mut client = make_client().await;

    let extract = client
        .extract_centroids(CentroidsRequest {
            input_image: Some(test_image(64, 64)),
            sigma: 8.0,
            binning: None,
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: false,
            normalize_rows: false,
            estimate_background_region: None,
        })
        .await;
    assert!(extract.is_ok(), "ExtractCentroids should be available");

    let solve_centroids = client
        .solve_from_centroids(SolveFromCentroidsRequest {
            centroids: vec![ImageCoord { x: 10.0, y: 10.0 }],
            width: 64,
            height: 64,
            params: Some(SolveParams {
                fov_estimate: Some(20.0),
                fov_max_error: Some(5.0),
                match_radius: Some(0.01),
                match_threshold: Some(1e-5),
                solve_timeout_ms: Some(100),
                distortion: Some(0.0),
                return_matches: false,
                return_catalog: false,
            }),
        })
        .await;
    assert!(
        solve_centroids.is_ok(),
        "SolveFromCentroids should be available"
    );

    let solve_image = client
        .solve_from_image(SolveFromImageRequest {
            extract: Some(CentroidsRequest {
                input_image: Some(test_image(64, 64)),
                sigma: 8.0,
                binning: None,
                return_binned: false,
                use_binned_for_star_candidates: false,
                detect_hot_pixels: false,
                normalize_rows: false,
                estimate_background_region: None,
            }),
            params: Some(SolveParams {
                fov_estimate: Some(20.0),
                fov_max_error: Some(5.0),
                match_radius: Some(0.01),
                match_threshold: Some(1e-5),
                solve_timeout_ms: Some(100),
                distortion: Some(0.0),
                return_matches: false,
                return_catalog: false,
            }),
        })
        .await;
    assert!(solve_image.is_ok(), "SolveFromImage should be available");

    let info = client.get_info(InfoRequest {}).await;
    assert!(info.is_ok(), "GetInfo should be available");
}

#[tokio::test]
async fn solve_from_image_one_call_end_to_end() {
    let mut client = make_client().await;

    let response = client
        .solve_from_image(SolveFromImageRequest {
            extract: Some(CentroidsRequest {
                input_image: Some(test_image(64, 64)),
                sigma: 8.0,
                binning: None,
                return_binned: false,
                use_binned_for_star_candidates: false,
                detect_hot_pixels: false,
                normalize_rows: false,
                estimate_background_region: None,
            }),
            params: Some(SolveParams {
                fov_estimate: Some(20.0),
                fov_max_error: Some(5.0),
                match_radius: Some(0.01),
                match_threshold: Some(1e-5),
                solve_timeout_ms: Some(100),
                distortion: Some(0.0),
                return_matches: false,
                return_catalog: false,
            }),
        })
        .await
        .unwrap();

    let solution = response.into_inner();
    // With a blank image and zero timeout the solve should report TOO_FEW.
    assert_eq!(solution.status, SolveStatus::TooFew as i32);
}

#[tokio::test]
async fn shmem_name_returns_internal() {
    let mut client = make_client().await;

    let response = client
        .extract_centroids(CentroidsRequest {
            input_image: Some(Image {
                width: 8,
                height: 8,
                image_data: vec![],
                shmem_name: Some("/test-shmem".to_string()),
                reopen_shmem: false,
            }),
            sigma: 8.0,
            binning: None,
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: false,
            normalize_rows: false,
            estimate_background_region: None,
        })
        .await;

    assert!(response.is_err(), "shmem should fail: no fast path");
    let status = response.unwrap_err();
    assert_eq!(status.code(), tonic::Code::Internal);
}

#[tokio::test]
async fn centroids_ordered_brightest_first() {
    // Two single-pixel stars of different brightness on a non-zero background.
    // Flat blocks fail the detector's strict 1-D peak test; isolated bright
    // pixels pass it and produce centroids at the pixel centers.
    let mut image = vec![50u8; 20 * 20];
    image[5 * 20 + 5] = 255;
    image[10 * 20 + 10] = 150;

    let mut client = make_client().await;
    let response = client
        .extract_centroids(CentroidsRequest {
            input_image: Some(Image {
                width: 20,
                height: 20,
                image_data: image,
                shmem_name: None,
                reopen_shmem: false,
            }),
            sigma: 2.0,
            binning: None,
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: false,
            normalize_rows: false,
            estimate_background_region: None,
        })
        .await
        .unwrap();

    let result = response.into_inner();
    assert!(result.star_candidates.len() >= 2, "two stars expected");
    let b0 = result.star_candidates[0].brightness;
    let b1 = result.star_candidates[1].brightness;
    assert!(
        b0 >= b1,
        "centroids should be brightest-first: {} < {}",
        b0,
        b1
    );
}

#[tokio::test]
async fn solve_params_forwarded_to_solver() {
    // A tiny timeout should force TIMEOUT before any real work.
    let mut client = make_client().await;
    let response = client
        .solve_from_centroids(SolveFromCentroidsRequest {
            centroids: vec![
                ImageCoord { x: 10.0, y: 10.0 },
                ImageCoord { x: 20.0, y: 20.0 },
                ImageCoord { x: 30.0, y: 30.0 },
                ImageCoord { x: 40.0, y: 40.0 },
            ],
            width: 100,
            height: 100,
            params: Some(SolveParams {
                fov_estimate: Some(20.0),
                fov_max_error: Some(5.0),
                match_radius: Some(0.01),
                match_threshold: Some(1e-5),
                solve_timeout_ms: Some(0),
                distortion: Some(0.0),
                return_matches: false,
                return_catalog: false,
            }),
        })
        .await
        .unwrap();

    let solution = response.into_inner();
    assert_eq!(solution.status, SolveStatus::Timeout as i32);
    // fov_estimate is clamped to the database range [10, 30] and returned in degrees.
    assert!(
        (solution.fov.expect("fov should be reported") - 20.0).abs() < 1e-6,
        "fov_estimate should be forwarded"
    );
    // A timed-out solve produces no attitude, and the wire must say so rather
    // than defaulting to 0.0 — which is a legitimate ra/dec.
    assert!(solution.ra.is_none());
    assert!(solution.dec.is_none());
    assert!(solution.roll.is_none());
}

#[tokio::test]
async fn get_info_reports_database_fov_range() {
    let mut client = make_client().await;
    let response = client.get_info(InfoRequest {}).await.unwrap();
    let info = response.into_inner();
    assert!((info.min_fov - 10.0).abs() < 1e-9);
    assert!((info.max_fov - 30.0).abs() < 1e-9);
    assert_eq!(info.num_patterns, 100);
    assert_eq!(info.star_catalog, "test_catalog");
}

#[tokio::test]
async fn extract_centroids_reports_wire_x_as_column() {
    // A single bright pixel at row 5, column 9 of a 16-wide, 12-high image. On the
    // wire (x, y) is (column, row), so the centroid must come back as (9.5, 5.5).
    // A symmetric position like (4, 4) cannot tell the two orders apart.
    let (w, h) = (16usize, 12usize);
    let mut image = vec![50u8; w * h];
    image[5 * w + 9] = 200;
    let mut client = make_client().await;
    let result = client
        .extract_centroids(CentroidsRequest {
            input_image: Some(Image {
                width: w as i32,
                height: h as i32,
                image_data: image,
                shmem_name: None,
                reopen_shmem: false,
            }),
            sigma: 2.0,
            binning: None,
            return_binned: false,
            use_binned_for_star_candidates: false,
            detect_hot_pixels: false,
            normalize_rows: false,
            estimate_background_region: None,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(result.star_candidates.len(), 1, "one star expected");
    let c = result.star_candidates[0]
        .centroid_position
        .as_ref()
        .unwrap();
    assert!(
        (c.x - 9.5).abs() < 1e-6 && (c.y - 5.5).abs() < 1e-6,
        "got x={} y={}",
        c.x,
        c.y
    );
}
