//! Hardware-neutral two-axis mount port. Positions are counts, rates are degrees/s.
use std::{future::Future, pin::Pin};

pub mod queue;
pub mod skywatcher;
pub mod watchdog;

/// An axis in the mount's own frame, independent of its zero points on the sky.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Clockwise turn as seen from above.
    Azimuth,
    /// Upward tilt of the tube.
    Altitude,
}

impl Axis {
    /// Index in every two-axis array: azimuth first, altitude second.
    pub const fn index(self) -> usize {
        match self {
            Self::Azimuth => 0,
            Self::Altitude => 1,
        }
    }
}

/// Capabilities reported by an adapter, rather than assumptions about a mount.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisInfo {
    /// Encoder counts in one complete revolution.
    pub counts_per_revolution: u32,
    /// Maximum magnitude of a signed angular rate, in degrees/s.
    pub top_rate_deg_per_s: f64,
    /// Whether the adapter supports absolute positioning on this axis.
    pub can_goto: bool,
}

/// An axis reading; it is not the tube's true direction on the sky.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisState {
    /// Signed counts relative to the mount's power-on zero.
    pub position: i32,
    /// Signed angular rate in degrees/s.
    pub rate_deg_per_s: f64,
    /// True until the axis finishes its move or decelerating stop.
    pub moving: bool,
}

/// Object-safe asynchronous operation, usable by services through `dyn Mount`.
pub type MountFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The scope node drives both axes only through this port.
///
/// Commands return when accepted; poll `state` for completion. Adapters keep rates
/// within their reported top rate. Positive rates increase axis counts. A stop
/// cancels positioning and rate commands on both axes and may decelerate to rest.
pub trait Mount: Send + Sync {
    /// Capabilities in azimuth, altitude order.
    fn axes(&self) -> [AxisInfo; 2];
    /// Read position, rate and motion status on the axis's own clock.
    fn state(&self, axis: Axis) -> MountFuture<'_, AxisState>;
    /// Accept an absolute target in counts, for an axis advertising `can_goto`.
    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()>;
    /// Accept a finite signed rate in degrees/s; zero requests a stop on this axis.
    fn rate(&self, axis: Axis, deg_per_s: f64) -> MountFuture<'_, ()>;
    /// Cancel motion on both axes and begin stopping them.
    fn stop_all(&self) -> MountFuture<'_, ()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeMount(Mutex<[AxisState; 2]>);

    impl Mount for FakeMount {
        fn axes(&self) -> [AxisInfo; 2] {
            [
                AxisInfo {
                    counts_per_revolution: 360_000,
                    top_rate_deg_per_s: 2.0,
                    can_goto: true,
                },
                AxisInfo {
                    counts_per_revolution: 720_000,
                    top_rate_deg_per_s: 4.0,
                    can_goto: true,
                },
            ]
        }
        fn state(&self, axis: Axis) -> MountFuture<'_, AxisState> {
            Box::pin(async move { self.0.lock().unwrap()[axis.index()] })
        }
        fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
            Box::pin(async move {
                self.0.lock().unwrap()[axis.index()] = AxisState {
                    position,
                    rate_deg_per_s: 0.0,
                    moving: false,
                };
            })
        }
        fn rate(&self, axis: Axis, deg_per_s: f64) -> MountFuture<'_, ()> {
            Box::pin(async move {
                let mut states = self.0.lock().unwrap();
                states[axis.index()].rate_deg_per_s = deg_per_s;
                states[axis.index()].moving = deg_per_s != 0.0;
            })
        }
        fn stop_all(&self) -> MountFuture<'_, ()> {
            Box::pin(async move {
                for state in self.0.lock().unwrap().iter_mut() {
                    state.rate_deg_per_s = 0.0;
                    state.moving = false;
                }
            })
        }
    }

    #[tokio::test]
    async fn two_axes_expose_capabilities_positions_signed_rates_and_stop() {
        // Independent fake capabilities and commands, with exact count/rate equality.
        let mount: Box<dyn Mount> = Box::new(FakeMount(Mutex::new(
            [AxisState {
                position: 0,
                rate_deg_per_s: 0.0,
                moving: false,
            }; 2],
        )));
        for (axis, counts, top, position, rate) in [
            (Axis::Azimuth, 360_000, 2.0, -1234, -1.25),
            (Axis::Altitude, 720_000, 4.0, 5678, 2.5),
        ] {
            assert_eq!(
                mount.axes()[axis.index()],
                AxisInfo {
                    counts_per_revolution: counts,
                    top_rate_deg_per_s: top,
                    can_goto: true,
                }
            );
            mount.goto(axis, position).await;
            assert_eq!(mount.state(axis).await.position, position);
            mount.rate(axis, rate).await;
            let state = mount.state(axis).await;
            assert_eq!(state.rate_deg_per_s, rate);
            assert!(state.moving);
        }
        mount.stop_all().await;
        for axis in [Axis::Azimuth, Axis::Altitude] {
            let state = mount.state(axis).await;
            assert_eq!(state.rate_deg_per_s, 0.0);
            assert!(!state.moving);
            assert_ne!(state.position, 0);
        }
    }
}
