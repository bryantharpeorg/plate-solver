//! Capture through the camera port; a failed adapter does not terminate the service.
use crate::mount::{Axis, Mount};
use crate::{detect, Camera, CameraError, GrayFrame, RangeError, Settings};
use scope_link::proto::{AxisPositions, Centroid};
use std::{sync::Arc, time::Duration};

#[derive(Debug)]
pub struct Captured {
    pub id: u64,
    pub frame: GrayFrame,
    pub centroids: Vec<Centroid>,
    pub axis_start: Option<AxisPositions>,
    pub axis_end: Option<AxisPositions>,
}

pub struct CaptureLoop<C> {
    camera: C,
    settings: Settings,
    id: u64,
    restart_at: Option<tokio::time::Instant>,
    restarts: u64,
    mount: Option<Arc<dyn Mount>>,
}
impl<C: Camera> CaptureLoop<C> {
    pub fn new(mut camera: C, settings: Settings) -> Result<Self, RangeError> {
        camera.apply(&settings)?;
        Ok(Self {
            camera,
            settings,
            id: 0,
            restart_at: None,
            restarts: 0,
            mount: None,
        })
    }
    pub fn restarts(&self) -> u64 {
        self.restarts
    }
    /// Attach exposure position sampling without changing the camera port.
    pub fn with_mount(mut self, mount: Arc<dyn Mount>) -> Self {
        self.mount = Some(mount);
        self
    }
    async fn positions(&mut self) -> Option<AxisPositions> {
        let mount = self.mount.as_ref()?;
        Some(AxisPositions {
            azimuth: mount.state(Axis::Azimuth).await.position,
            altitude: mount.state(Axis::Altitude).await.position,
        })
    }
    pub fn camera(&self) -> &C {
        &self.camera
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub fn apply(&mut self, settings: Settings) -> Result<(), RangeError> {
        self.camera.apply(&settings)?;
        self.settings = settings;
        Ok(())
    }
    /// An error is observable by the service; its next call retries after 1 s.
    pub async fn next(&mut self) -> Result<Captured, CameraError> {
        let result = self.capture().await;
        if let Err(error) = &result {
            eprintln!("camera capture failed: {error}");
            self.restart_at = Some(tokio::time::Instant::now() + Duration::from_secs(1));
        }
        result
    }
    async fn capture(&mut self) -> Result<Captured, CameraError> {
        if let Some(at) = self.restart_at.take() {
            tokio::time::sleep_until(at).await;
            self.camera.restart()?;
            self.restarts += 1;
        }
        let axis_start = self.positions().await;
        let frame = self.camera.next_frame().await?;
        let axis_end = self.positions().await;
        self.id += 1;
        let centroids = detect::detect(&frame, &self.settings.detection);
        Ok(Captured {
            id: self.id,
            frame,
            centroids,
            axis_start,
            axis_end,
        })
    }
}
