//! Connect a mount in the background while the scope node keeps serving its camera.
use super::{transport::BaudStream, Parameters, SkyWatcher};
use crate::{
    clock::NodeClock,
    mount::{Axis, AxisInfo, AxisState, Mount, MountFuture, MountIdentity},
};
use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::task::JoinHandle;

/// Opens a fresh byte stream to the board, e.g. the serial device.
pub type Opener<S> = Box<dyn Fn() -> io::Result<S> + Send + Sync>;

struct Snapshot<S> {
    board: Option<Arc<SkyWatcher<S>>>,
    states: [AxisState; 2],
}
struct Inner<S> {
    open: Opener<S>,
    clock: Arc<dyn NodeClock>,
    snapshot: Mutex<Snapshot<S>>,
}

/// Owns a background connect task; synchronous mount queries read a snapshot.
pub struct MountLink<S: BaudStream + 'static> {
    inner: Arc<Inner<S>>,
    connect: JoinHandle<()>,
}
impl<S: BaudStream + 'static> Drop for MountLink<S> {
    fn drop(&mut self) {
        self.connect.abort();
    }
}
impl<S: BaudStream + 'static> MountLink<S> {
    /// Returns at once and connects in a background task.
    pub fn start(open: Opener<S>, clock: Arc<dyn NodeClock>) -> Arc<Self> {
        let inner = Arc::new(Inner {
            open,
            clock,
            snapshot: Mutex::new(Snapshot {
                board: None,
                states: [AxisState {
                    position: 0,
                    rate_deg_per_s: 0.0,
                    moving: true,
                }; 2],
            }),
        });
        // The task owns only Inner, so dropping the last link can abort it.
        let connect = tokio::spawn(connect(inner.clone()));
        Arc::new(Self { inner, connect })
    }
    fn board(&self) -> Option<Arc<SkyWatcher<S>>> {
        self.inner.snapshot.lock().unwrap().board.clone()
    }
}

async fn connect<S: BaudStream + 'static>(inner: Arc<Inner<S>>) {
    let mut last_error = None;
    loop {
        let result = match (inner.open)() {
            Ok(stream) => SkyWatcher::connect(stream, inner.clock.clone()).await,
            Err(error) => Err(error.into()),
        };
        match result {
            Ok(board) => {
                let report = board.report();
                eprintln!(
                    "mount connected: {} at {} baud; azimuth {}; altitude {}",
                    report.firmware,
                    report.baud,
                    axis_report(report.axes[0]),
                    axis_report(report.axes[1]),
                );
                for warning in &report.warnings {
                    eprintln!("mount warning: {warning}");
                }
                inner.snapshot.lock().unwrap().board = Some(Arc::new(board));
                return;
            }
            Err(error) => {
                let message = error.to_string();
                if last_error.as_ref() != Some(&message) {
                    eprintln!("mount not connected: {message}");
                    last_error = Some(message);
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

fn axis_report(axis: Parameters) -> String {
    format!(
        "{} counts/rev, timer {} Hz, high-speed ratio {}, {}",
        axis.cpr,
        axis.timer,
        axis.ratio,
        if axis.auxiliary_encoders {
            "auxiliary encoders, :W sent"
        } else {
            "no auxiliary encoders, no :W"
        },
    )
}

impl<S: BaudStream + 'static> Mount for MountLink<S> {
    fn connected(&self) -> bool {
        self.board().is_some()
    }
    fn identity(&self) -> Option<MountIdentity> {
        let board = self.board()?;
        let firmware = board.report().firmware;
        Some(MountIdentity {
            firmware: format!("{}.{:02}", firmware.major, firmware.minor),
            board: format!("0x{:02X}", firmware.board),
        })
    }
    fn axes(&self) -> [AxisInfo; 2] {
        self.board().map_or(
            [AxisInfo {
                counts_per_revolution: 0,
                top_rate_deg_per_s: 0.0,
                can_goto: false,
            }; 2],
            |board| board.axes(),
        )
    }
    fn state(&self, axis: Axis) -> MountFuture<'_, AxisState> {
        Box::pin(async move {
            if let Some(board) = self.board() {
                let state = board.state(axis).await;
                self.inner.snapshot.lock().unwrap().states[axis.index()] = state;
                state
            } else {
                AxisState {
                    moving: true,
                    ..self.inner.snapshot.lock().unwrap().states[axis.index()]
                }
            }
        })
    }
    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
        Box::pin(async move {
            if let Some(board) = self.board() {
                board.goto(axis, position).await;
            } else {
                eprintln!("mount not connected: move dropped");
            }
        })
    }
    fn rate(&self, axis: Axis, deg_per_s: f64) -> MountFuture<'_, ()> {
        Box::pin(async move {
            if let Some(board) = self.board() {
                board.rate(axis, deg_per_s).await;
            } else {
                eprintln!("mount not connected: move dropped");
            }
        })
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        Box::pin(async move {
            if let Some(board) = self.board() {
                board.stop_all().await;
            }
        })
    }
    fn stop_confirmed(&self) -> MountFuture<'_, bool> {
        Box::pin(async move {
            match self.board() {
                Some(board) => board.stop_confirmed().await,
                None => false,
            }
        })
    }
}
