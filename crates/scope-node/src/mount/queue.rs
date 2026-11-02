//! One bounded adapter queue. Stop invalidates its epoch and bypasses the queue.
use super::{Axis, Mount};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{oneshot, watch, Mutex as AsyncMutex, Notify},
    task::JoinHandle,
};

const CAPACITY: usize = 64;

#[derive(Clone, Copy)]
enum Move {
    Goto(Axis, i32),
    Rate(Axis, f64),
}
struct Command {
    epoch: u64,
    movement: Move,
    reply: oneshot::Sender<()>,
}
struct State {
    epoch: u64,
    stopping: bool,
    pending: VecDeque<Command>,
}
struct Shared {
    mount: Arc<dyn Mount>,
    state: Mutex<State>,
    changed: watch::Sender<u64>,
    ready: Notify,
    executing: AsyncMutex<()>,
    stopping: AsyncMutex<()>,
}

/// A rejected move was either invalidated by Stop or exceeded the queue bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueError {
    /// Stop discarded this request. A fresh request is required.
    Cancelled,
    /// The adapter already has the maximum number of waiting commands.
    Full,
}

/// Owns the single adapter worker; dropping it cancels outstanding operations.
pub struct MountQueue {
    shared: Arc<Shared>,
    worker: JoinHandle<()>,
}
impl Drop for MountQueue {
    fn drop(&mut self) {
        self.worker.abort();
    }
}
impl MountQueue {
    /// Start one worker for this mount. No move task is spawned per request.
    pub fn new(mount: Arc<dyn Mount>) -> Self {
        let shared = Arc::new(Shared {
            mount,
            state: Mutex::new(State {
                epoch: 0,
                stopping: false,
                pending: VecDeque::new(),
            }),
            changed: watch::channel(0).0,
            ready: Notify::new(),
            executing: AsyncMutex::new(()),
            stopping: AsyncMutex::new(()),
        });
        let worker = tokio::spawn(run(shared.clone()));
        Self { shared, worker }
    }
    /// Read-only adapter access for capabilities and exposure position samples.
    pub fn mount(&self) -> &Arc<dyn Mount> {
        &self.shared.mount
    }
    /// Number of commands waiting behind the in-flight operation.
    pub fn pending(&self) -> usize {
        self.shared.state.lock().unwrap().pending.len()
    }
    /// Request absolute positioning and await adapter acceptance.
    pub async fn goto(&self, axis: Axis, position: i32) -> Result<(), QueueError> {
        self.submit(Move::Goto(axis, position)).await
    }
    /// Request a signed rate and await adapter acceptance.
    pub async fn rate(&self, axis: Axis, rate: f64) -> Result<(), QueueError> {
        self.submit(Move::Rate(axis, rate)).await
    }
    async fn submit(&self, movement: Move) -> Result<(), QueueError> {
        let (reply, received) = oneshot::channel();
        {
            let mut state = self.shared.state.lock().unwrap();
            if state.stopping {
                return Err(QueueError::Cancelled);
            }
            if state.pending.len() >= CAPACITY {
                return Err(QueueError::Full);
            }
            let epoch = state.epoch;
            state.pending.push_back(Command {
                epoch,
                movement,
                reply,
            });
        }
        self.shared.ready.notify_one();
        received.await.map_err(|_| QueueError::Cancelled)
    }
    /// Invalidate queued and in-flight moves, then command both axes to stop.
    /// Cancellation releases the execution barrier before the adapter's stop,
    /// so an old move can never be polled after the stop has been answered.
    pub async fn stop_all(&self) {
        let _stop = self.shared.stopping.lock().await;
        {
            let mut state = self.shared.state.lock().unwrap();
            state.stopping = true;
            state.epoch = state.epoch.wrapping_add(1);
            state.pending.clear();
            self.shared.changed.send_replace(state.epoch);
        }
        let _executing = self.shared.executing.lock().await;
        self.shared.mount.stop_all().await;
        self.shared.state.lock().unwrap().stopping = false;
    }
}

async fn run(shared: Arc<Shared>) {
    let mut changed = shared.changed.subscribe();
    loop {
        shared.ready.notified().await;
        loop {
            let command = shared.state.lock().unwrap().pending.pop_front();
            let Some(command) = command else {
                break;
            };
            let _executing = shared.executing.lock().await;
            let epoch = *changed.borrow_and_update();
            if command.epoch != epoch {
                continue;
            }
            let operation = async {
                match command.movement {
                    Move::Goto(axis, position) => shared.mount.goto(axis, position).await,
                    Move::Rate(axis, rate) => shared.mount.rate(axis, rate).await,
                }
            };
            tokio::select! {
                biased;
                _ = changed.changed() => {},
                _ = operation => { let _ = command.reply.send(()); },
            }
        }
    }
}
