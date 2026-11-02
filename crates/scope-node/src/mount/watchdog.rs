//! Heartbeat silence is measured on the injected node clock, in 50 ms ticks.
use super::queue::MountQueue;
use crate::clock::NodeClock;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::task::JoinHandle;

struct State {
    last_heartbeat: f64,
    lost: bool,
}

/// Records heartbeat receipts and owns the node's independent stop timer.
pub struct Watchdog {
    state: Arc<Mutex<State>>,
    task: JoinHandle<()>,
}
impl Drop for Watchdog {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Watchdog {
    /// Arm immediately, including when the console never sends a first heartbeat.
    pub fn new(clock: Arc<dyn NodeClock>, queue: Arc<MountQueue>) -> Self {
        let state = Arc::new(Mutex::new(State {
            last_heartbeat: clock.now(),
            lost: false,
        }));
        let observed = state.clone();
        let task = tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(50));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tick.tick().await;
                let stop = {
                    let mut state = observed.lock().unwrap();
                    if !state.lost && clock.now() - state.last_heartbeat >= 1.0 {
                        state.lost = true;
                        true
                    } else {
                        false
                    }
                };
                if stop {
                    queue.stop_all().await;
                }
            }
        });
        Self { state, task }
    }
    /// A heartbeat only restores link health; it never restores a previous move.
    pub fn heartbeat(&self, now: f64) {
        let mut state = self.state.lock().unwrap();
        state.last_heartbeat = now;
        state.lost = false;
    }
    /// Whether one second has elapsed without an accepted heartbeat.
    pub fn lost(&self) -> bool {
        self.state.lock().unwrap().lost
    }
}
