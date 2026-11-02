//! Oracles: the simulator's mechanics and explicit command log; watchdog
//! deadline 1.0 ± 0.1 s, rest at exactly zero, targets within one count.
mod support;
use scope_link::proto;
use scope_node::mount::{Axis, Mount};
use std::time::Duration;
use support::*;

#[tokio::test(start_paused = true)]
async fn stop_cancels_inflight_and_queued_commands_but_accepts_a_fresh_goto() {
    let mut rig = Rig::new().await;
    rig.rate(proto::Axis::Azimuth, 1.0).await;
    rig.rate(proto::Axis::Altitude, -1.0).await;
    rig.step(0.5, true).await;
    assert!(rig.adapter.mount.state(Axis::Azimuth).await.moving);
    assert!(rig.adapter.mount.state(Axis::Altitude).await.moving);
    let mut client = rig.mount.clone();
    let blocked = tokio::spawn(async move {
        client
            .rate_axis(proto::RateAxisRequest {
                axis: proto::Axis::Azimuth as i32,
                deg_per_s: 0.123,
            })
            .await
    });
    rig.adapter.entered.notified().await;
    let mut client = rig.mount.clone();
    let queued = tokio::spawn(async move {
        client
            .goto_axis(proto::GotoAxisRequest {
                axis: proto::Axis::Azimuth as i32,
                position: 90_000,
            })
            .await
    });
    // Observe queue admission before Stop; scheduler timing is not the oracle.
    for _ in 0..100 {
        tokio::task::yield_now().await;
        if rig.service.mount_queue().unwrap().pending() == 1 {
            break;
        }
    }
    assert_eq!(rig.service.mount_queue().unwrap().pending(), 1);
    tokio::time::timeout(
        Duration::from_millis(100),
        rig.stop.stop_all(proto::StopAllRequest {}),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        blocked.await.unwrap().unwrap_err().code(),
        tonic::Code::Aborted
    );
    assert_eq!(
        queued.await.unwrap().unwrap_err().code(),
        tonic::Code::Aborted
    );
    rig.adapter.release.notify_one();
    rig.step(0.5, true).await;
    rig.at_rest().await;
    assert!(rig.adapter.gotos.lock().unwrap().is_empty());
    rig.mount
        .goto_axis(proto::GotoAxisRequest {
            axis: proto::Axis::Azimuth as i32,
            position: 4500,
        })
        .await
        .unwrap();
    rig.step(2.0, true).await;
    assert_eq!(*rig.adapter.gotos.lock().unwrap(), vec![4500]);
    assert!((rig.adapter.mount.state(Axis::Azimuth).await.position - 4500).abs() <= 1);
    rig.at_rest().await;
}

#[tokio::test(start_paused = true)]
async fn silent_link_stops_both_axes_once_at_one_second_and_never_resumes_a_rate() {
    let mut rig = Rig::new().await;
    rig.rate(proto::Axis::Azimuth, 1.5).await;
    rig.rate(proto::Axis::Altitude, -1.5).await;
    rig.step(0.6, true).await;
    for axis in [Axis::Azimuth, Axis::Altitude] {
        assert!(rig.adapter.mount.state(axis).await.moving);
    }
    let last = rig.heartbeat().await;
    rig.step(0.9, false).await;
    assert!(rig.adapter.stops.lock().unwrap().is_empty());
    rig.step(0.2, false).await;
    let stops = rig.adapter.stops.lock().unwrap().clone();
    assert_eq!(stops.len(), 1);
    assert!(
        (stops[0] - last - 1.0).abs() <= 0.1,
        "{stops:?}, last {last}"
    );
    rig.step(0.6, false).await;
    rig.at_rest().await;
    assert_eq!(
        rig.mount
            .rate_axis(proto::RateAxisRequest {
                axis: proto::Axis::Azimuth as i32,
                deg_per_s: 1.0,
            })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::FailedPrecondition
    );
    let position = rig.adapter.mount.state(Axis::Azimuth).await.position;
    rig.step(0.5, true).await;
    rig.at_rest().await;
    assert_eq!(
        rig.adapter.mount.state(Axis::Azimuth).await.position,
        position
    );
    rig.rate(proto::Axis::Azimuth, -1.0).await;
    rig.step(0.5, true).await;
    let state = rig.adapter.mount.state(Axis::Azimuth).await;
    assert!(state.moving && state.position < position);
    assert_eq!(state.rate_deg_per_s, -1.0);
}

#[tokio::test(start_paused = true)]
async fn a_full_adapter_queue_still_accepts_stop_and_discards_all_waiting_moves() {
    let mut rig = Rig::new().await;
    let mut client = rig.mount.clone();
    let blocked = tokio::spawn(async move {
        client
            .rate_axis(proto::RateAxisRequest {
                axis: proto::Axis::Azimuth as i32,
                deg_per_s: 0.123,
            })
            .await
    });
    rig.adapter.entered.notified().await;
    let mut waiting = Vec::new();
    for position in 1..=64 {
        let queue = rig.service.mount_queue().unwrap().clone();
        waiting.push(tokio::spawn(async move {
            queue.goto(Axis::Azimuth, position).await
        }));
    }
    for _ in 0..100 {
        if rig.service.mount_queue().unwrap().pending() == 64 {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(rig.service.mount_queue().unwrap().pending(), 64);
    assert_eq!(
        rig.mount
            .goto_axis(proto::GotoAxisRequest {
                axis: proto::Axis::Azimuth as i32,
                position: 9000,
            })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::ResourceExhausted
    );
    rig.stop.stop_all(proto::StopAllRequest {}).await.unwrap();
    assert_eq!(
        blocked.await.unwrap().unwrap_err().code(),
        tonic::Code::Aborted
    );
    for command in waiting {
        assert_eq!(
            command.await.unwrap(),
            Err(scope_node::mount::queue::QueueError::Cancelled)
        );
    }
    assert!(rig.adapter.gotos.lock().unwrap().is_empty());
    rig.at_rest().await;
}
