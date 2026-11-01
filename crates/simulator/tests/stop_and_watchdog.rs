//! Oracles: recorded adapter calls on the injected node clock, and SimMount's mechanics.
mod support;
use scope_link::proto::{self, scope_node_mount_server::ScopeNodeMount};
use scope_node::mount::{Axis, AxisInfo, AxisState, Mount, MountFuture};
use simulator::{mount::SimMount, VirtualClock};
use std::{
    future::{pending, poll_fn, Future},
    sync::{Arc, Mutex},
    task::Poll,
};
use support::*;
use tokio::sync::Notify;
use tonic::Request;

struct Observed {
    sim: SimMount,
    clock: VirtualClock,
    stops: Mutex<Vec<f64>>,
    gotos: Mutex<Vec<i32>>,
    blocked: Notify,
}
impl Observed {
    fn new(clock: VirtualClock) -> Self {
        Self {
            sim: SimMount::new(clock.clone()),
            clock,
            stops: Mutex::new(vec![]),
            gotos: Mutex::new(vec![]),
            blocked: Notify::new(),
        }
    }
}
impl Mount for Observed {
    fn axes(&self) -> [AxisInfo; 2] {
        self.sim.axes()
    }
    fn state(&self, axis: Axis) -> MountFuture<'_, AxisState> {
        self.sim.state(axis)
    }
    fn goto(&self, axis: Axis, position: i32) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.gotos.lock().unwrap().push(position);
            self.sim.goto(axis, position).await;
        })
    }
    fn rate(&self, axis: Axis, rate: f64) -> MountFuture<'_, ()> {
        Box::pin(async move {
            if rate == 2.0 {
                self.blocked.notify_one();
                pending::<()>().await;
            }
            self.sim.rate(axis, rate).await;
        })
    }
    fn stop_all(&self) -> MountFuture<'_, ()> {
        Box::pin(async move {
            self.stops.lock().unwrap().push(self.clock.node_now());
            self.sim.stop_all().await;
        })
    }
}
async fn moving(rig: &mut Rig, sim: &SimMount) {
    for axis in [proto::Axis::Azimuth, proto::Axis::Altitude] {
        rig.mount
            .rate_axis(proto::RateAxisRequest {
                axis: axis as i32,
                deg_per_s: 0.6,
            })
            .await
            .unwrap();
    }
    advance(0.3).await;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        assert!(sim.state(axis).await.moving);
    }
}
async fn stopped(sim: &SimMount) {
    for axis in [Axis::Azimuth, Axis::Altitude] {
        let state = sim.state(axis).await;
        assert!(!state.moving);
        assert_eq!(state.rate_deg_per_s, 0.0);
    }
}

#[tokio::test(start_paused = true)]
async fn stop_bypasses_a_blocked_command_drops_older_goto_and_accepts_a_fresh_one() {
    let clock = VirtualClock::new(1_768_528_800.0);
    let observed = Arc::new(Observed::new(clock.clone()));
    let mut rig = rig(clock, Some(observed.clone())).await;
    moving(&mut rig, &observed.sim).await;
    beat(&mut rig).await;
    let service = rig.service.clone();
    let blocked = tokio::spawn(async move {
        service
            .rate_axis(Request::new(proto::RateAxisRequest {
                axis: 1,
                deg_per_s: 2.0,
            }))
            .await
    });
    observed.blocked.notified().await;
    // Poll the real handler until it queues the request behind the blocked adapter.
    let mut older = Box::pin(rig.service.goto_axis(Request::new(proto::GotoAxisRequest {
        axis: 1,
        position: 123_456,
    })));
    poll_fn(|cx| {
        assert!(older.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    tokio::time::timeout(
        std::time::Duration::from_millis(50),
        rig.stop.stop_all(proto::StopAllRequest {}),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(observed.stops.lock().unwrap().len(), 1);
    assert_eq!(older.await.unwrap_err().code(), tonic::Code::Cancelled);
    assert_eq!(
        blocked.await.unwrap().unwrap_err().code(),
        tonic::Code::Cancelled
    );
    advance(0.3).await;
    stopped(&observed.sim).await;
    assert!(observed.gotos.lock().unwrap().is_empty());
    rig.mount
        .goto_axis(proto::GotoAxisRequest {
            axis: 1,
            position: 2000,
        })
        .await
        .unwrap();
    for _ in 0..6 {
        advance(0.2).await;
        beat(&mut rig).await;
    }
    assert_eq!(*observed.gotos.lock().unwrap(), vec![2000]);
    assert_eq!(observed.sim.state(Axis::Azimuth).await.position, 2000);
}

#[tokio::test(start_paused = true)]
async fn silent_heartbeats_command_both_axes_to_stop_at_one_second_and_never_resume_a_rate() {
    let clock = VirtualClock::new(1_768_528_800.0);
    let observed = Arc::new(Observed::new(clock.clone()));
    let mut rig = rig(clock, Some(observed.clone())).await;
    moving(&mut rig, &observed.sim).await;
    let last = beat(&mut rig).await.t1;
    advance(0.89).await;
    assert!(observed.stops.lock().unwrap().is_empty());
    advance(0.21).await;
    let stops = observed.stops.lock().unwrap().clone();
    assert_eq!(stops.len(), 1);
    assert!((stops[0] - last - 1.0).abs() <= 0.1);
    advance(0.3).await;
    stopped(&observed.sim).await;
    // Lost links reject new motion until an authenticated heartbeat returns.
    assert_eq!(
        rig.mount
            .rate_axis(proto::RateAxisRequest {
                axis: 1,
                deg_per_s: 0.6
            })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::FailedPrecondition
    );
    let positions = [
        observed.sim.state(Axis::Azimuth).await.position,
        observed.sim.state(Axis::Altitude).await.position,
    ];
    for _ in 0..6 {
        beat(&mut rig).await;
        advance(0.2).await;
    }
    stopped(&observed.sim).await;
    assert_eq!(
        observed.sim.state(Axis::Azimuth).await.position,
        positions[0]
    );
    assert_eq!(
        observed.sim.state(Axis::Altitude).await.position,
        positions[1]
    );
    rig.mount
        .rate_axis(proto::RateAxisRequest {
            axis: 1,
            deg_per_s: -0.6,
        })
        .await
        .unwrap();
    advance(0.3).await;
    assert_eq!(observed.sim.state(Axis::Azimuth).await.rate_deg_per_s, -0.6);
    assert_ne!(
        observed.sim.state(Axis::Azimuth).await.position,
        positions[0]
    );
}

#[tokio::test(start_paused = true)]
async fn stop_cancels_an_active_goto_even_when_heartbeats_continue() {
    let clock = VirtualClock::new(1_768_528_800.0);
    let mount = Arc::new(SimMount::new(clock.clone()));
    let mut rig = rig(clock, Some(mount.clone())).await;
    rig.mount
        .goto_axis(proto::GotoAxisRequest {
            axis: proto::Axis::Azimuth as i32,
            position: 100_000,
        })
        .await
        .unwrap();
    advance(0.3).await;
    let moving = mount.state(Axis::Azimuth).await;
    assert!(moving.moving && moving.position > 0);
    assert!(moving.position < 100_000);
    rig.stop.stop_all(proto::StopAllRequest {}).await.unwrap();
    advance(0.4).await;
    stopped(&mount).await;
    let position = mount.state(Axis::Azimuth).await.position;
    assert!(position < 100_000);
    for _ in 0..6 {
        beat(&mut rig).await;
        advance(0.2).await;
    }
    assert_eq!(mount.state(Axis::Azimuth).await.position, position);
    stopped(&mount).await;
    // A new positioning request still reaches its target after the explicit stop.
    rig.mount
        .goto_axis(proto::GotoAxisRequest {
            axis: proto::Axis::Azimuth as i32,
            position: 2000,
        })
        .await
        .unwrap();
    for _ in 0..5 {
        beat(&mut rig).await;
        advance(0.2).await;
    }
    assert_eq!(mount.state(Axis::Azimuth).await.position, 2000);
    stopped(&mount).await;
}
