//! The newest-frame live solver, bounded preview history and live reports.
use crate::{
    link::LinkStatus,
    pushto::Guidance,
    session::{Clock, Session},
    sky::Frame,
    solve::{self, SolveOptions, SolveReport},
};
use pattern_database::PatternDatabase;
use scope_link::proto::StarFrame;
use serde::Serialize;
use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{broadcast, watch, Semaphore},
    task::JoinHandle,
};

/// An async solve and its projection.
pub type SolveFuture<'a> = Pin<Box<dyn Future<Output = (SolveReport, Option<Frame>)> + Send + 'a>>;
/// Injectable async solver.
pub trait Solver: Send + Sync {
    /// Solve with these options.
    fn solve(&self, frame: StarFrame, options: SolveOptions) -> SolveFuture<'_>;
}
/// Production solver on the blocking pool.
pub struct DbSolver(pub Arc<PatternDatabase>);
impl Solver for DbSolver {
    fn solve(&self, frame: StarFrame, options: SolveOptions) -> SolveFuture<'_> {
        let db = self.0.clone();
        Box::pin(async move {
            let size = (frame.width as usize, frame.height as usize);
            let fallback_db = db.clone();
            tokio::task::spawn_blocking(move || {
                solve::solve_centroids(&db, &frame.centroids, size, &options)
            })
            .await
            .unwrap_or_else(|_| {
                let (mut report, _) = solve::solve_centroids(&fallback_db, &[], size, &options);
                report.status = solve::Status::Cancelled;
                report.hint = report.status.hint();
                (report, None)
            })
        })
    }
}
/// The report consumed by the console's live stream.
#[derive(Debug, Clone, Serialize)]
pub struct LiveReport {
    /// Node exposure id.
    pub frame_id: u64,
    /// Mid-exposure in console Unix seconds.
    pub time: f64,
    /// Sequence if solved.
    pub pointing_seq: Option<u64>,
    /// Latest preview id.
    pub preview_seq: Option<u64>,
    /// Number of detected stars in this frame.
    pub stars: usize,
    /// Chart report, with no filename.
    pub report: SolveReport,
    /// Guidance at completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance: Option<Guidance>,
}
/// A preview and its exposure's centroids.
#[derive(Clone)]
pub struct Preview {
    /// Frame id on the scope link.
    pub frame_id: u64,
    /// The node's encoded JPEG.
    pub jpeg: Vec<u8>,
    /// Tap centroids.
    pub centroids: Vec<scope_link::proto::Centroid>,
}
/// Owns and cancels the live tasks.
pub struct Live {
    tasks: Vec<JoinHandle<()>>,
    reports: broadcast::Sender<LiveReport>,
    previews: Arc<Mutex<VecDeque<Preview>>>,
}
impl Drop for Live {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}
impl Live {
    /// Start alongside a link. The shared gate also serializes uploaded solves.
    pub fn start(
        session: Arc<Session>,
        clock: Arc<dyn Clock>,
        mut frames: watch::Receiver<Option<StarFrame>>,
        mut status: watch::Receiver<LinkStatus>,
        solver: Arc<dyn Solver>,
        gate: Arc<Semaphore>,
        db_range: (f64, f64),
    ) -> Self {
        let (reports, _) = broadcast::channel(16);
        let previews = Arc::new(Mutex::new(VecDeque::with_capacity(8)));
        let mut incoming = frames.clone();
        let history = previews.clone();
        let receive_session = session.clone();
        let receive_clock = clock.clone();
        // Receipt continues while solving, including previews on frames we drop.
        let receive = tokio::spawn(async move {
            loop {
                let frame = incoming.borrow_and_update().clone();
                if let Some(frame) = frame {
                    receive_session.receive_live_frame(
                        frame.centroids.len(),
                        receive_clock.now(),
                        db_range,
                    );
                    if let Some(jpeg) = frame.preview_jpeg {
                        let mut history = history.lock().expect("preview history");
                        if history.len() == 8 {
                            history.pop_front();
                        }
                        history.push_back(Preview {
                            frame_id: frame.id,
                            jpeg,
                            centroids: frame.centroids,
                        });
                    }
                }
                if incoming.changed().await.is_err() {
                    return;
                }
            }
        });
        let outgoing = reports.clone();
        let history = previews.clone();
        let solve = tokio::spawn(async move {
            let mut estimate = None;
            let mut failures = 0;
            loop {
                if frames.borrow().is_none() && frames.changed().await.is_err() {
                    return;
                }
                while !status.borrow().connected || status.borrow().offset_s.is_none() {
                    if status.changed().await.is_err() {
                        return;
                    }
                }
                let Ok(permit) = gate.acquire().await else {
                    return;
                };
                // A link may have dropped or been re-paired while an upload held the gate.
                let link = status.borrow().clone();
                let Some(offset) = link.offset_s.filter(|_| link.connected) else {
                    continue;
                };
                // Take the frame only once free, even if an upload held the gate.
                let Some(frame) = frames.borrow_and_update().clone() else {
                    continue;
                };
                let time = (frame.start_s + frame.end_s) / 2.0 - offset;
                let options = SolveOptions {
                    fov: estimate,
                    fov_max_error: estimate
                        .map_or(SolveOptions::default().fov_max_error, |fov| 0.05 * fov),
                    ..SolveOptions::default()
                };
                let (mut report, solved) = solver.solve(frame.clone(), options).await;
                report.name = None;
                let pointing_seq = solved.and_then(|solved| {
                    estimate = Some(solved.fov_deg());
                    session.record_live_pointing(solved, time, frame.id)
                });
                if pointing_seq.is_some() {
                    failures = 0;
                } else {
                    failures += 1;
                    if failures >= 3 {
                        estimate = None;
                    }
                }
                report.pointing_seq = pointing_seq;
                let preview_seq = history
                    .lock()
                    .expect("preview history")
                    .back()
                    .map(|p| p.frame_id);
                let guidance = session.snapshot(clock.now()).guidance;
                let _ = outgoing.send(LiveReport {
                    frame_id: frame.id,
                    time,
                    pointing_seq,
                    preview_seq,
                    stars: frame.centroids.len(),
                    report,
                    guidance,
                });
                drop(permit);
                if frames.changed().await.is_err() {
                    return;
                }
            }
        });
        Self {
            tasks: vec![receive, solve],
            reports,
            previews,
        }
    }
    /// Subscribe; receivers may lag beyond capacity 16.
    pub fn subscribe(&self) -> broadcast::Receiver<LiveReport> {
        self.reports.subscribe()
    }
    /// Read a retained preview.
    pub fn preview(&self, frame_id: u64) -> Option<Preview> {
        self.previews
            .lock()
            .expect("preview history")
            .iter()
            .find(|p| p.frame_id == frame_id)
            .cloned()
    }
}
