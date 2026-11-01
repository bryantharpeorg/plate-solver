//! One adapter task. Stop invalidates queued epochs and cancels the active future.
use super::{Axis, AxisInfo, AxisState, Mount};
use std::{
    collections::VecDeque,
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{oneshot, watch, Notify},
    task::JoinHandle,
};
use tonic::Status;

enum Command {
    Goto(Axis, i32),
    Rate(Axis, f64),
}
struct Entry {
    epoch: u64,
    command: Command,
    reply: oneshot::Sender<Result<(), Status>>,
}
struct State {
    epoch: u64,
    pending: VecDeque<Entry>,
    stopping: usize,
    lost: bool,
}
struct Inner {
    mount: Arc<dyn Mount>,
    state: Mutex<State>,
    wake: Notify,
    epoch: watch::Sender<u64>,
    // Stop cancels the adapter future before taking this lock, so a blocked
    // command cannot hold Stop hostage or finish after the stop's reply.
    adapter: tokio::sync::Mutex<()>,
}
struct Worker(JoinHandle<()>);
impl Drop for Worker {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Shared mount command boundary; reads bypass the motion queue.
#[derive(Clone)]
pub struct MountQueue {
    inner: Arc<Inner>,
    _worker: Arc<Worker>,
}
impl MountQueue {
    pub fn new(mount: Arc<dyn Mount>) -> Self {
        let inner = Arc::new(Inner {
            mount,
            state: Mutex::new(State {
                epoch: 0,
                pending: VecDeque::new(),
                stopping: 0,
                lost: false,
            }),
            wake: Notify::new(),
            epoch: watch::channel(0).0,
            adapter: tokio::sync::Mutex::new(()),
        });
        let worker = tokio::spawn(run(inner.clone()));
        Self {
            inner,
            _worker: Arc::new(Worker(worker)),
        }
    }
    pub fn axes(&self) -> [AxisInfo; 2] {
        self.inner.mount.axes()
    }
    pub async fn state(&self, axis: Axis) -> AxisState {
        self.inner.mount.state(axis).await
    }
    pub async fn goto(&self, axis: Axis, position: i32) -> Result<(), Status> {
        self.send(Command::Goto(axis, position)).await
    }
    pub async fn rate(&self, axis: Axis, rate: f64) -> Result<(), Status> {
        self.send(Command::Rate(axis, rate)).await
    }
    async fn send(&self, command: Command) -> Result<(), Status> {
        let (reply, received) = oneshot::channel();
        {
            let mut state = self.inner.state.lock().unwrap();
            if state.lost {
                return Err(Status::failed_precondition(
                    "mount link lost; a fresh heartbeat is required",
                ));
            }
            let epoch = state.epoch;
            state.pending.push_back(Entry {
                epoch,
                command,
                reply,
            });
        }
        self.inner.wake.notify_one();
        received
            .await
            .unwrap_or_else(|_| Err(Status::cancelled("mount command discarded by stop")))
    }
    /// Invalidation happens synchronously, before the returned stop future waits.
    pub fn stop_all(&self) -> impl Future<Output = ()> + Send + 'static {
        self.stop(false)
    }
    pub(super) fn link_lost(&self) -> impl Future<Output = ()> + Send + 'static {
        self.stop(true)
    }
    pub(super) fn heartbeat(&self) {
        self.inner.state.lock().unwrap().lost = false;
    }
    fn stop(&self, lost: bool) -> impl Future<Output = ()> + Send + 'static {
        {
            let mut state = self.inner.state.lock().unwrap();
            state.epoch += 1;
            state.stopping += 1;
            state.lost |= lost;
            state.pending.clear();
            self.inner.epoch.send_replace(state.epoch);
        }
        let inner = self.inner.clone();
        // Own the safety action independently of a disconnected RPC's lifetime.
        let stop = tokio::spawn(async move {
            let _adapter = inner.adapter.lock().await;
            inner.mount.stop_all().await;
            inner.state.lock().unwrap().stopping -= 1;
            inner.wake.notify_one();
        });
        async move {
            let _ = stop.await;
        }
    }
}
async fn run(inner: Arc<Inner>) {
    let mut epochs = inner.epoch.subscribe();
    loop {
        inner.wake.notified().await;
        loop {
            let entry = {
                let mut state = inner.state.lock().unwrap();
                if state.stopping != 0 {
                    None
                } else {
                    state.pending.pop_front()
                }
            };
            let Some(entry) = entry else {
                break;
            };
            let _adapter = inner.adapter.lock().await;
            epochs.borrow_and_update();
            if inner.state.lock().unwrap().epoch != entry.epoch {
                continue;
            }
            let operation = async {
                match entry.command {
                    Command::Goto(axis, position) => inner.mount.goto(axis, position).await,
                    Command::Rate(axis, rate) => inner.mount.rate(axis, rate).await,
                }
            };
            let result = tokio::select! {
                biased;
                _ = epochs.changed() => Err(Status::cancelled("mount command cancelled by stop")),
                _ = operation => Ok(()),
            };
            let _ = entry.reply.send(result);
        }
    }
}
