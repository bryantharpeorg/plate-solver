//! US2's oracles are the specified control path and the simulator's true direction.
//! Solves must agree within 30 arcseconds; exposure and gain are reported exactly.
mod dark;

use scope_link::{
    memory,
    proto::{self, scope_node_client::ScopeNodeClient},
    BearerToken,
};
use scope_node::{
    camera::{check_range, FrameFuture, Mode},
    clock::{NodeClock, SystemClock},
    detect::DetectSettings,
    service::NodeService,
    Camera, CameraInfo, GrayFrame, RangeError, Settings,
};
use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use tonic::{service::interceptor::InterceptedService, transport::Channel};

type Client = ScopeNodeClient<InterceptedService<Channel, BearerToken>>;

struct SkyCamera {
    field: Arc<dark::Field>,
    settings: Settings,
    automatic_gain: f64,
    frames: u64,
}

impl Camera for SkyCamera {
    fn info(&self) -> CameraInfo {
        // OV5647's 1296x972 mode and listed ExposureTime/AnalogueGain ranges:
        // fixtures/scope-node/list-cameras.txt, converted from microseconds to ms.
        let mode = Mode {
            width: dark::WIDTH,
            height: dark::HEIGHT,
            fps: 46.34,
        };
        CameraInfo {
            model: "drawn dark sky".into(),
            width: mode.width,
            height: mode.height,
            modes: vec![mode],
            active_mode: Some(mode),
            exposure_min_ms: 0.13,
            exposure_max_ms: 3066.985,
            gain_min: 1.0,
            gain_max: 63.9375,
        }
    }

    fn apply(&mut self, settings: &Settings) -> Result<(), RangeError> {
        check_range(&self.info(), settings)?;
        self.settings = settings.clone();
        Ok(())
    }

    fn next_frame(&mut self) -> FrameFuture<'_> {
        Box::pin(async move {
            let exposure_ms = self.settings.exposure_ms.unwrap_or(200.0);
            let gain = self.settings.gain.unwrap_or(self.automatic_gain);
            let start_s = SystemClock.now();
            tokio::time::sleep(Duration::from_secs_f64(exposure_ms / 1000.0)).await;
            self.frames += 1;
            Ok(GrayFrame {
                pixels: dark::render(
                    &self.field,
                    dark::DARK,
                    exposure_ms / 1000.0 * gain,
                    13 + self.frames,
                ),
                width: dark::WIDTH,
                height: dark::HEIGHT,
                start_s,
                end_s: start_s + exposure_ms / 1000.0,
                exposure_ms,
                gain,
            })
        })
    }
}

struct Rig {
    field: Arc<dark::Field>,
    client: Client,
    server: JoinHandle<()>,
}

impl Rig {
    async fn new(detection: DetectSettings, automatic_gain: f64) -> Self {
        let field = Arc::new(dark::field());
        let camera = SkyCamera {
            field: field.clone(),
            settings: Settings::default(),
            automatic_gain,
            frames: 0,
        };
        // Use the production constructor: its default 200 ms interval caps AE.
        let service = NodeService::new(
            camera,
            Arc::new(SystemClock),
            "test-token",
            Settings {
                exposure_ms: None,
                gain: None,
                detection,
            },
        )
        .unwrap();
        let (connect, io) = memory::channel_pair();
        let server = tokio::spawn(async move {
            memory::serve_on(io, service.into_server()).await.unwrap();
        });
        let client = ScopeNodeClient::with_interceptor(
            connect.await.unwrap(),
            BearerToken::new("test-token").unwrap(),
        );
        Self {
            field,
            client,
            server,
        }
    }

    async fn frames(&mut self) -> tonic::Streaming<proto::StarFrame> {
        self.client
            .frames(proto::FramesRequest {})
            .await
            .unwrap()
            .into_inner()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.server.abort();
    }
}

fn pairing_settings() -> proto::Settings {
    proto::Settings {
        exposure_ms: None,
        gain: None,
        detection: Some(proto::Detection {
            sigma: 8.0,
            binning: 2,
            normalize_rows: true,
            detect_hot_pixels: true,
        }),
    }
}

async fn next(stream: &mut tonic::Streaming<proto::StarFrame>, id: u64) -> proto::StarFrame {
    let frame = tokio::time::timeout(Duration::from_secs(10), stream.message())
        .await
        .expect("star frame within 10 seconds")
        .expect("scope link stays open")
        .expect("star frame stream continues");
    assert_eq!(frame.id, id, "read every exposure from the first");
    frame
}

// US2-S1: without a floor, noise drives the unchanged controller darker.
#[tokio::test(start_paused = true)]
async fn no_floor_lowers_gain_then_exposure_and_loses_the_solve() {
    let mut rig = Rig::new(DetectSettings::cedar(), 16.0).await;
    let mut stream = rig.frames().await;
    let first = next(&mut stream, 1).await;
    assert_eq!(first.centroids.len(), 200);
    assert_eq!((first.exposure_ms, first.gain), (200.0, 16.0));
    let error = dark::solve(&rig.field, &first.centroids).expect("first frame solves");
    assert!(error <= 30.0, "first solve error {error} arcseconds");

    let mut previous = first;
    for id in 2..=8 {
        let frame = next(&mut stream, id).await;
        assert!(frame.centroids.len() >= 100, "frame {id}");
        assert!(frame.gain <= previous.gain, "gain rose at frame {id}");
        assert!(
            frame.exposure_ms <= previous.exposure_ms,
            "exposure rose at frame {id}"
        );
        if frame.gain > 1.0 {
            assert_eq!(frame.exposure_ms, 200.0, "frame {id}");
        }
        if id >= 3 {
            assert!(
                dark::solve(&rig.field, &frame.centroids).is_none(),
                "unexpected solve at frame {id}"
            );
        }
        previous = frame;
    }
    assert_eq!(previous.gain, 1.0);
    assert!(previous.exposure_ms < 10.0);
}

// US2-S2: pairing must preserve the config's floor. Gain is noiseless in this
// drawn sky; these counts do not promise a real camera's high-gain performance.
#[tokio::test(start_paused = true)]
async fn pairing_keeps_the_floor_and_auto_exposure_finds_the_stars() {
    let mut rig = Rig::new(dark::floor(0.5), 1.0).await;
    let pairing = pairing_settings();
    let response = rig
        .client
        .configure(pairing.clone())
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response, pairing);
    let mut stream = rig.frames().await;
    let mut in_band = 0;
    for id in 1..=12 {
        let frame = next(&mut stream, id).await;
        assert_eq!(frame.settings.as_ref(), Some(&pairing), "frame {id}");
        assert_eq!(frame.exposure_ms, 200.0, "frame {id}");
        if id >= 4 {
            let error = dark::solve(&rig.field, &frame.centroids)
                .unwrap_or_else(|| panic!("frame {id} must solve"));
            assert!(error <= 30.0, "frame {id}: {error} arcseconds");
        }
        if id >= 6 {
            let count = frame.centroids.len();
            assert!((20..=50).contains(&count), "frame {id}: {count} stars");
            assert!(frame.gain > 8.0, "frame {id}: gain {}", frame.gain);
            in_band += usize::from((25..=40).contains(&count));
        }
    }
    assert!(in_band >= 5, "only {in_band} of the final seven in 25..=40");
}

// US2-S3: the four detection keys carried by the link still replace the config.
#[tokio::test(start_paused = true)]
async fn configure_takes_all_four_detection_keys_from_the_link() {
    let mut rig = Rig::new(dark::floor(0.5), 1.0).await;
    let requested = proto::Settings {
        exposure_ms: None,
        gain: None,
        detection: Some(proto::Detection {
            sigma: 20.0,
            binning: 4,
            normalize_rows: false,
            detect_hot_pixels: false,
        }),
    };
    let response = rig
        .client
        .configure(requested.clone())
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response, requested);
    let mut stream = rig.frames().await;
    next(&mut stream, 1).await;
    next(&mut stream, 2).await;
    assert_eq!(next(&mut stream, 3).await.settings, Some(requested));
}
