//! rpicam-apps subprocess adapter. A private FIFO pairs metadata with stdout YUV.
pub mod listing;
pub mod metadata;
pub mod yuv;

use crate::{
    camera::{check_range, FrameFuture},
    config::CameraMode,
    Camera, CameraError, CameraInfo, RangeError, Settings,
};
use metadata::MetadataReader;
use std::{
    ffi::OsString,
    path::PathBuf,
    process::{ExitStatus, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::{
    io::AsyncReadExt,
    net::unix::pipe,
    process::{Child, ChildStdout, Command},
    task::JoinHandle,
};
use yuv::YuvReader;

/// Run the configured size. A size the camera does not list is refused, not rounded.
fn choose_mode(info: &mut CameraInfo, mode: CameraMode) -> Result<(), CameraError> {
    let CameraMode::Size { width, height } = mode else {
        return Ok(());
    };
    let found = info
        .modes
        .iter()
        .copied()
        .find(|m| m.width == width && m.height == height);
    if found.is_none() {
        let listed: Vec<_> = info
            .modes
            .iter()
            .map(|m| format!("{}x{}", m.width, m.height))
            .collect();
        return Err(CameraError(format!(
            "camera has no {width}x{height} mode; it lists {}",
            listed.join(", ")
        )));
    }
    info.active_mode = found;
    Ok(())
}

pub struct RpicamCamera {
    info: CameraInfo,
    settings: Settings,
    path: Option<OsString>,
    process: Option<Process>,
    reopen: bool,
}

impl RpicamCamera {
    /// Query the camera stack rather than inventing sensor capabilities.
    pub async fn discover(mode: CameraMode) -> Result<Self, CameraError> {
        let output = Command::new("rpicam-hello")
            .args(["--list-cameras", "-v"])
            .output()
            .await?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return Err(CameraError(format!(
                "rpicam-hello exited {}: {stderr}",
                output.status
            )));
        }
        let listing = format!("{}\n{stderr}", String::from_utf8_lossy(&output.stdout));
        let mut info = listing::parse(&listing)?.remove(0);
        choose_mode(&mut info, mode)?;
        Self::new(info)
    }

    pub fn new(mut info: CameraInfo) -> Result<Self, CameraError> {
        if info.active_mode.is_none() {
            info.active_mode = info.modes.first().copied();
        }
        let mode = info
            .active_mode
            .as_ref()
            .ok_or_else(|| CameraError("camera has no modes".into()))?;
        if mode.width == 0
            || mode.height == 0
            || !mode.width.is_multiple_of(2)
            || !mode.height.is_multiple_of(2)
            || !mode.fps.is_finite()
            || mode.fps <= 0.0
        {
            return Err(CameraError("camera has invalid YUV420 mode".into()));
        }
        for (name, min, max) in [
            ("exposure_ms", info.exposure_min_ms, info.exposure_max_ms),
            ("gain", info.gain_min, info.gain_max),
        ] {
            if !min.is_finite() || !max.is_finite() || min <= 0.0 || max < min {
                return Err(CameraError(format!("camera has invalid {name} range")));
            }
        }
        Ok(Self {
            info,
            settings: Settings::default(),
            path: None,
            process: None,
            reopen: false,
        })
    }

    /// Child-only PATH override, so fixture scripts do not change global test state.
    pub fn with_path(mut self, path: OsString) -> Self {
        self.path = Some(path);
        self
    }

    async fn start(&self) -> Result<Process, CameraError> {
        let directory = FifoDirectory::new()?;
        let fifo = directory.0.join("metadata.json");
        rustix::fs::mkfifoat(
            rustix::fs::CWD,
            &fifo,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        )
        .map_err(|e| CameraError(format!("metadata FIFO: {e}")))?;
        let receiver = pipe::OpenOptions::new().open_receiver(&fifo)?;
        // Keep reads pending before the child opens its writer. On child exit we
        // close this sender, letting buffered metadata drain to a genuine EOF.
        let keepalive = pipe::OpenOptions::new().open_sender(&fifo)?;
        let mode = self
            .info
            .active_mode
            .as_ref()
            .expect("validated camera mode");
        let mut command = Command::new("rpicam-vid");
        if let Some(path) = &self.path {
            command.env("PATH", path);
        }
        command
            .args(["--codec", "yuv420", "-o", "-", "--metadata"])
            .arg(&fifo)
            .args([
                "--metadata-format",
                "json",
                "--nopreview",
                "-t",
                "0",
                "--denoise",
                "off",
                "--sharpness",
                "0",
                "--contrast",
                "1",
                "--saturation",
                "1",
            ])
            .arg("--mode")
            .arg(format!("{}:{}", mode.width, mode.height))
            .arg("--width")
            .arg(mode.width.to_string())
            .arg("--height")
            .arg(mode.height.to_string());
        let mut fps = mode.fps.min(5.0);
        if let Some(exposure) = self.settings.exposure_ms {
            command
                .arg("--shutter")
                .arg((exposure * 1000.0).to_string());
            fps = fps.min(1000.0 / exposure);
        }
        if let Some(gain) = self.settings.gain {
            command.arg("--gain").arg(gain.to_string());
        }
        command
            .arg("--framerate")
            .arg(fps.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = command.spawn()?;
        let stdout = child.stdout.take().expect("piped stdout");
        let mut stderr = child.stderr.take().expect("piped stderr");
        let log = tokio::spawn(async move {
            let mut tail = Vec::new();
            let mut buffer = [0u8; 4096];
            while let Ok(n) = stderr.read(&mut buffer).await {
                if n == 0 {
                    break;
                }
                tail.extend_from_slice(&buffer[..n]);
                if tail.len() > 65536 {
                    tail.drain(..tail.len() - 65536);
                }
            }
            tail
        });
        Ok(Process {
            child,
            yuv: YuvReader::new(stdout, mode.width, mode.height, yuv::stride(mode.width))?,
            metadata: MetadataReader::new(receiver),
            keepalive: Some(keepalive),
            log,
            status: None,
            _directory: directory,
        })
    }
}

impl Camera for RpicamCamera {
    fn info(&self) -> CameraInfo {
        self.info.clone()
    }
    fn apply(&mut self, settings: &Settings) -> Result<(), RangeError> {
        check_range(&self.info, settings)?;
        if self.settings.exposure_ms != settings.exposure_ms || self.settings.gain != settings.gain
        {
            self.reopen = true;
        }
        self.settings = settings.clone();
        Ok(())
    }
    fn restart(&mut self) -> Result<(), CameraError> {
        self.reopen = true;
        Ok(())
    }
    fn next_frame(&mut self) -> FrameFuture<'_> {
        Box::pin(async move {
            if self.reopen {
                if let Some(mut process) = self.process.take() {
                    // Release the camera device before its replacement opens it.
                    let _ = process.child.kill().await;
                }
                self.reopen = false;
            }
            if self.process.is_none() {
                self.process = Some(self.start().await?);
            }
            let process = self.process.as_mut().expect("started camera");
            let result = process.read().await;
            match result {
                Ok((pixels, metadata)) => {
                    let mode = self.info.active_mode.as_ref().expect("validated mode");
                    Ok(metadata.frame(pixels, mode.width, mode.height))
                }
                Err(reason) => {
                    let error = process.failure(reason).await;
                    self.process = None;
                    Err(error)
                }
            }
        })
    }
}

struct Process {
    child: Child,
    yuv: YuvReader<ChildStdout>,
    metadata: MetadataReader<pipe::Receiver>,
    keepalive: Option<pipe::Sender>,
    log: JoinHandle<Vec<u8>>,
    status: Option<ExitStatus>,
    _directory: FifoDirectory,
}
impl Process {
    async fn read(&mut self) -> Result<(Vec<u8>, metadata::Metadata), CameraError> {
        let pair = async {
            let (pixels, metadata) = tokio::try_join!(
                async { self.yuv.next().await.map_err(CameraError::from) },
                self.metadata.next()
            )?;
            match (pixels, metadata) {
                (Some(pixels), Some(metadata)) => Ok((pixels, metadata)),
                _ => Err(CameraError("frame stream or metadata ended".into())),
            }
        };
        tokio::pin!(pair);
        if self.status.is_none() {
            tokio::select! {
                result = &mut pair => return result,
                status = self.child.wait() => {
                    self.status = Some(status?);
                    self.keepalive = None;
                }
            }
        }
        pair.await
    }
    async fn failure(&mut self, reason: CameraError) -> CameraError {
        let status = match self.status {
            Some(status) => Ok(status),
            None => {
                // Give an exiting process time to finish its stderr and status;
                // malformed live streams are stopped instead of hanging forever.
                match tokio::time::timeout(std::time::Duration::from_secs(1), self.child.wait())
                    .await
                {
                    Ok(status) => status,
                    Err(_) => {
                        let _ = self.child.start_kill();
                        self.child.wait().await
                    }
                }
            }
        };
        self.keepalive = None;
        let stderr = tokio::time::timeout(std::time::Duration::from_secs(1), &mut self.log)
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or_default();
        CameraError(format!(
            "rpicam-vid: {reason}; exit {status:?}; stderr: {}",
            String::from_utf8_lossy(&stderr)
        ))
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        self.log.abort();
    }
}

struct FifoDirectory(PathBuf);
impl FifoDirectory {
    fn new() -> Result<Self, CameraError> {
        use std::os::unix::fs::DirBuilderExt;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        loop {
            let path = std::env::temp_dir().join(format!(
                "scope-node-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }
}
impl Drop for FifoDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Mode;

    fn listed() -> CameraInfo {
        CameraInfo {
            modes: vec![
                Mode {
                    width: 640,
                    height: 480,
                    fps: 62.5,
                },
                Mode {
                    width: 1296,
                    height: 972,
                    fps: 46.34,
                },
            ],
            ..CameraInfo::default()
        }
    }

    #[test]
    fn auto_leaves_the_mode_unchosen() {
        let mut info = listed();
        choose_mode(&mut info, CameraMode::Auto).unwrap();
        assert!(info.active_mode.is_none());
    }

    #[test]
    fn a_listed_size_becomes_the_active_mode() {
        let mut info = listed();
        let size = CameraMode::Size {
            width: 1296,
            height: 972,
        };
        choose_mode(&mut info, size).unwrap();
        let mode = info.active_mode.unwrap();
        assert_eq!((mode.width, mode.height, mode.fps), (1296, 972, 46.34));
    }

    #[test]
    fn an_unlisted_size_is_refused_naming_what_is_listed() {
        let mut info = listed();
        let size = CameraMode::Size {
            width: 1920,
            height: 1080,
        };
        let error = choose_mode(&mut info, size).unwrap_err().to_string();
        assert!(error.contains("1920x1080"), "{error}");
        assert!(error.contains("640x480, 1296x972"), "{error}");
        assert!(info.active_mode.is_none());
    }
}
