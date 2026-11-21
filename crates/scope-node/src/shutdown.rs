//! What the node does with its mount when it is told to exit.
use crate::service::NodeService;
use std::{fmt, time::Duration};

/// How long the node waits, once told to exit, for the mount to confirm its
/// stop before it exits without that confirmation.
pub const STOP_BOUND: Duration = Duration::from_secs(8);

/// How the mount's stop ended when the node was told to exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shutdown {
    /// The node has no mount.
    NoMount,
    /// The mount confirmed the stop.
    Confirmed,
    /// The mount's adapter could not confirm the stop.
    NotConfirmed,
    /// The stop had not ended when the bound ran out.
    TimedOut,
}
impl fmt::Display for Shutdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMount => f.write_str("no mount to stop"),
            Self::Confirmed => f.write_str("the mount confirmed the stop"),
            Self::NotConfirmed => f.write_str("the mount did not confirm the stop"),
            Self::TimedOut => write!(
                f,
                "the mount did not confirm the stop within {} s",
                STOP_BOUND.as_secs()
            ),
        }
    }
}

impl NodeService {
    /// Refuse every later move for good, invalidate the queued and in-flight
    /// ones, and wait up to `STOP_BOUND` for the mount to confirm its stop.
    pub async fn shut_down(&self) -> Shutdown {
        let Some(queue) = self.mount_queue() else {
            return Shutdown::NoMount;
        };
        match tokio::time::timeout(STOP_BOUND, queue.close()).await {
            Ok(true) => Shutdown::Confirmed,
            Ok(false) => Shutdown::NotConfirmed,
            Err(_) => Shutdown::TimedOut,
        }
    }
}
