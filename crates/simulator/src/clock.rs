//! The simulated UTC epoch follows tokio time, including paused test runtimes.
use tokio::time::Instant;

#[derive(Clone, Debug)]
pub struct VirtualClock {
    start_unix_s: f64,
    start: Instant,
}

impl VirtualClock {
    pub fn new(start_unix_s: f64) -> Self {
        assert!(start_unix_s.is_finite());
        Self {
            start_unix_s,
            start: Instant::now(),
        }
    }

    pub fn now(&self) -> f64 {
        self.start_unix_s + self.start.elapsed().as_secs_f64()
    }

    pub fn start_unix_s(&self) -> f64 {
        self.start_unix_s
    }

    /// Boot-relative exposure timestamps intentionally differ from console UTC.
    pub fn node_now(&self) -> f64 {
        1000.0 + self.start.elapsed().as_secs_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test(start_paused = true)]
    async fn virtual_clock_advances_only_with_tokio_time() {
        // Unix timestamp of 2026-01-16T02:00:00Z, independently specified by US3.
        let clock = VirtualClock::new(1_768_528_800.0);
        assert_eq!(clock.now(), 1_768_528_800.0);
        assert_eq!(clock.node_now(), 1000.0);
        let copy = clock.clone();
        tokio::time::advance(Duration::from_secs(10)).await;
        assert_eq!(clock.now(), 1_768_528_810.0);
        assert_eq!(copy.now(), 1_768_528_810.0);
        assert_eq!(clock.node_now(), 1010.0);
    }
}
