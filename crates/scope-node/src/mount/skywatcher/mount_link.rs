//! Connect a mount in the background while the scope node keeps serving its camera.
pub use super::protocol::{GotoPath, LineHealth, MountHealth};
use super::{protocol::wrapped_delta, transport::BaudStream, Parameters, SkyWatcher};
use crate::{
    clock::NodeClock,
    mount::{Axis, AxisInfo, AxisState, Mount, MountFuture, MountIdentity},
};
use std::{
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{sync::Notify, task::JoinHandle};

/// Opens a fresh byte stream to the board, e.g. the serial device.
pub type Opener<S> = Box<dyn Fn() -> io::Result<S> + Send + Sync>;

struct Snapshot<S> {
    board: Option<Arc<SkyWatcher<S>>>,
    // Keep the last successful stream for Stop even when telemetry is lost.
    stop_board: Option<Arc<SkyWatcher<S>>>,
    states: [AxisState; 2],
    resets: u64,
    baselines: [Option<(i32, f64)>; 2],
    seen_initialised: [bool; 2],
    failures: u8,
    pending_cause: Option<String>,
    health: MountHealth,
    retired_health: MountHealth,
    line_health: LineHealth,
    retired_line_health: LineHealth,
}
struct Inner<S> {
    open: Opener<S>,
    clock: Arc<dyn NodeClock>,
    snapshot: Mutex<Snapshot<S>>,
    io: tokio::sync::Mutex<()>,
    device: tokio::sync::Mutex<()>,
    wake: Notify,
    give_way: Notify,
    stopping: AtomicUsize,
}

// A dropped Stop future must also let future connect attempts proceed.
struct Stopping<'a>(&'a AtomicUsize);
impl<'a> Stopping<'a> {
    fn new(count: &'a AtomicUsize) -> Self {
        count.fetch_add(1, Ordering::AcqRel);
        Self(count)
    }
}
impl Drop for Stopping<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
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
                stop_board: None,
                states: [AxisState {
                    position: 0,
                    rate_deg_per_s: 0.0,
                    moving: true,
                }; 2],
                resets: 0,
                baselines: [None; 2],
                seen_initialised: [false; 2],
                failures: 0,
                pending_cause: None,
                health: MountHealth::default(),
                retired_health: MountHealth::default(),
                line_health: LineHealth::default(),
                retired_line_health: LineHealth::default(),
            }),
            io: tokio::sync::Mutex::new(()),
            device: tokio::sync::Mutex::new(()),
            wake: Notify::new(),
            give_way: Notify::new(),
            stopping: AtomicUsize::new(0),
        });
        // The task owns only Inner, so dropping the last link can abort it.
        let connect = tokio::spawn(connect(inner.clone()));
        Arc::new(Self { inner, connect })
    }
    /// Invalid replies over the link's lifetime and the last GoTo path per axis.
    pub fn health(&self) -> MountHealth {
        let snapshot = self.inner.snapshot.lock().unwrap();
        combined_health(&snapshot.retired_health, &snapshot.health)
    }
    /// Stale input over every board this link has had, keeping the latest bytes.
    pub fn line_health(&self) -> LineHealth {
        let snapshot = self.inner.snapshot.lock().unwrap();
        combined_line_health(&snapshot.retired_line_health, &snapshot.line_health)
    }
    async fn copy_health(&self, board: &Arc<SkyWatcher<S>>) {
        let health = board.health().await;
        let line_health = board.line_health().await;
        let mut snapshot = self.inner.snapshot.lock().unwrap();
        if snapshot
            .board
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, board))
        {
            snapshot.health = health;
            snapshot.line_health = line_health;
        }
    }
    fn board(&self) -> Option<Arc<SkyWatcher<S>>> {
        self.inner.snapshot.lock().unwrap().board.clone()
    }
    async fn stop(&self) -> bool {
        // Do not disturb discovery before a board has ever connected.
        if self.inner.snapshot.lock().unwrap().stop_board.is_none() {
            return false;
        }
        let _stopping = Stopping::new(&self.inner.stopping);
        self.inner.give_way.notify_waiters();
        let _device = self.inner.device.lock().await;
        let (board, stop_board) = {
            let snapshot = self.inner.snapshot.lock().unwrap();
            (snapshot.board.clone(), snapshot.stop_board.clone())
        };
        match (board, stop_board) {
            (Some(board), _) => board.stop_confirmed().await,
            (None, Some(board)) => board.halt_once().await,
            (None, None) => false,
        }
    }
}

fn combined_health(retired: &MountHealth, current: &MountHealth) -> MountHealth {
    MountHealth {
        invalid_replies: retired.invalid_replies + current.invalid_replies,
        last_invalid: current
            .last_invalid
            .clone()
            .or_else(|| retired.last_invalid.clone()),
        goto_path: std::array::from_fn(|i| current.goto_path[i].or(retired.goto_path[i])),
    }
}

fn combined_line_health(retired: &LineHealth, current: &LineHealth) -> LineHealth {
    LineHealth {
        stale_replies: retired.stale_replies + current.stale_replies,
        last_stale: current
            .last_stale
            .clone()
            .or_else(|| retired.last_stale.clone()),
    }
}

async fn connect<S: BaudStream + 'static>(inner: Arc<Inner<S>>) {
    let mut last_error = None;
    loop {
        match attempt(&inner).await {
            Ok(()) => {
                last_error = None;
                // notify_one retains a loss/reset wake that arrived before this wait.
                inner.wake.notified().await;
            }
            Err(message) => {
                if last_error.as_ref() != Some(&message) {
                    eprintln!("mount not connected: {message}");
                    last_error = Some(message);
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

async fn attempt<S: BaudStream + 'static>(inner: &Inner<S>) -> Result<(), String> {
    // Register before taking the device, including before the future is polled.
    // Serial descriptors share their input queue and baud, so Stop must cancel
    // this attempt and own device before reading the kept descriptor.
    let give_way = inner.give_way.notified();
    tokio::select! {
        biased;
        _ = give_way => Err("connect gave way to a stop".into()),
        result = async {
            let _device = inner.device.lock().await;
            // An attempt starting just after notify_waiters must give way too.
            if inner.stopping.load(Ordering::Acquire) > 0 {
                return Err("connect gave way to a stop".into());
            }
            let stream = (inner.open)().map_err(|error| error.to_string())?;
            let board = Arc::new(SkyWatcher::connect(stream, inner.clock.clone())
                .await.map_err(|error| error.to_string())?);
            let report = board.report();
            eprintln!(
                "mount connected: {} at {} baud; azimuth {}; altitude {}",
                report.firmware, report.baud,
                axis_report(report.axes[0]), axis_report(report.axes[1]),
            );
            for warning in &report.warnings {
                eprintln!("mount warning: {warning}");
            }
            let mut snapshot = inner.snapshot.lock().unwrap();
            if snapshot.stop_board.is_some() {
                let cause = snapshot.pending_cause.take().unwrap_or_else(|| "link lost".into());
                snapshot.resets += 1;
                eprintln!("mount reset ({cause}): re-initialised {} at {} baud",
                    report.firmware, report.baud);
            }
            snapshot.baselines = [None; 2];
            snapshot.seen_initialised = [false; 2];
            snapshot.failures = 0;
            snapshot.retired_health = combined_health(&snapshot.retired_health, &snapshot.health);
            snapshot.health = MountHealth::default();
            snapshot.retired_line_health = combined_line_health(
                &snapshot.retired_line_health, &snapshot.line_health,
            );
            snapshot.line_health = LineHealth::default();
            snapshot.stop_board = Some(board.clone());
            snapshot.board = Some(board);
            Ok(())
        } => result,
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
    fn resets(&self) -> u64 {
        self.inner.snapshot.lock().unwrap().resets
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
            let io = self.inner.io.lock().await;
            let Some(board) = self.board() else {
                return AxisState {
                    moving: true,
                    ..self.inner.snapshot.lock().unwrap().states[axis.index()]
                };
            };
            let state = board.state(axis).await;
            // Copy before either loss or reset removes this board, including the
            // wrong-length replies encountered by the read that found it.
            self.copy_health(&board).await;
            let initialised = board.initialised(axis).await;
            let now = self.inner.clock.now();
            let i = axis.index();
            let cause = {
                let mut snapshot = self.inner.snapshot.lock().unwrap();
                snapshot.states[i] = state;
                if let Some(initialised) = initialised {
                    snapshot.failures = 0;
                    let axis_info = board.axes()[i];
                    let jumped = snapshot.baselines[i].is_some_and(|(position, time)| {
                        let limit = 1.5
                            * axis_info.top_rate_deg_per_s
                            * (now - time)
                            * f64::from(axis_info.counts_per_revolution)
                            / 360.0
                            + 1000.0;
                        f64::from(wrapped_delta(state.position, position)).abs() > limit
                    });
                    let cause = if !initialised && snapshot.seen_initialised[i] {
                        Some("not initialised".to_string())
                    } else if jumped {
                        Some(format!("counts jumped on {axis:?}"))
                    } else {
                        None
                    };
                    snapshot.seen_initialised[i] |= initialised;
                    snapshot.baselines[i] = Some((state.position, now));
                    if let Some(cause) = &cause {
                        snapshot.board = None;
                        snapshot.pending_cause = Some(cause.clone());
                    }
                    cause
                } else {
                    snapshot.failures += 1;
                    if snapshot.failures == 3 {
                        snapshot.board = None;
                        eprintln!("mount link lost: 3 failed reads in a row");
                        self.inner.wake.notify_one();
                    }
                    None
                }
            };
            // Moves see disconnected before io is released. Never hold io across
            // discovery: moves must return immediately and Stop owns device only.
            drop(board);
            drop(io);
            if let Some(cause) = cause {
                let inner = self.inner.clone();
                // No await between removing the board and spawning: cancellation
                // of the finding read cannot strand a disconnected link.
                let reinitialise = tokio::spawn(async move {
                    if let Err(error) = attempt(&inner).await {
                        eprintln!("mount reset ({cause}): re-initialisation failed: {error}");
                        // pending_cause stays until a successful connect counts it.
                        inner.wake.notify_one();
                    }
                });
                let _ = reinitialise.await;
            }
            state
        })
    }
    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
        Box::pin(async move {
            let _io = self.inner.io.lock().await;
            if let Some(board) = self.board() {
                board.goto(axis, position).await;
                self.copy_health(&board).await;
            } else {
                eprintln!("mount not connected: move dropped");
            }
        })
    }
    fn rate(&self, axis: Axis, deg_per_s: f64) -> MountFuture<'_, ()> {
        Box::pin(async move {
            let _io = self.inner.io.lock().await;
            if let Some(board) = self.board() {
                board.rate(axis, deg_per_s).await;
                self.copy_health(&board).await;
            } else {
                eprintln!("mount not connected: move dropped");
            }
        })
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.stop().await;
        })
    }
    fn stop_confirmed(&self) -> MountFuture<'_, bool> {
        Box::pin(self.stop())
    }
}
