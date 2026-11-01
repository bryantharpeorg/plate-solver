//! A silent console loses motion authority after one second on the node clock.
use super::queue::MountQueue;
use crate::clock::NodeClock;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::task::JoinHandle;

struct Beat {
    last: f64,
    lost: bool,
}
struct Runtime {
    beat: Arc<Mutex<Beat>>,
    clock: Arc<dyn NodeClock>,
    queue: MountQueue,
    task: JoinHandle<()>,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.task.abort();
    }
}
#[derive(Clone)]
pub struct Watchdog(Arc<Runtime>);
impl Watchdog {
    pub fn new(clock: Arc<dyn NodeClock>, queue: MountQueue) -> Self {
        let beat = Arc::new(Mutex::new(Beat {
            last: clock.now(),
            lost: false,
        }));
        let task = tokio::spawn(run(clock.clone(), queue.clone(), beat.clone()));
        Self(Arc::new(Runtime {
            beat,
            clock,
            queue,
            task,
        }))
    }
    pub fn heartbeat(&self) {
        let mut beat = self.0.beat.lock().unwrap();
        beat.last = self.0.clock.now();
        beat.lost = false;
        self.0.queue.heartbeat();
    }
}
async fn run(clock: Arc<dyn NodeClock>, queue: MountQueue, beat: Arc<Mutex<Beat>>) {
    let mut tick = tokio::time::interval(Duration::from_millis(50));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tick.tick().await;
        let stop = {
            let mut beat = beat.lock().unwrap();
            if !beat.lost && clock.now() - beat.last >= 1.0 {
                beat.lost = true;
                Some(queue.link_lost())
            } else {
                None
            }
        };
        if let Some(stop) = stop {
            stop.await;
        }
    }
}
