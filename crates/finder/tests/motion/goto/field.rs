//! Oracle: cedar's database range is 10°–30°; a 0.7-scale solved field
//! is about 8.4° and stays live pointing but must never enter the model.
use super::tilt::{latest_frame, rig_with};
use crate::support::goto::{self as api, Rig};
use finder::{
    live::{SolveFuture, Solver},
    sky::Frame,
    solve::{self, SolveOptions},
};
use math_core::PinholeCamera;
use scope_link::proto::StarFrame;
use simulator::mount::MountConfig;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Scale {
    window: Mutex<Option<(u64, Option<u64>)>>,
}
impl Solver for Scale {
    fn solve(&self, frame: StarFrame, options: SolveOptions) -> SolveFuture<'_> {
        Box::pin(async move {
            let (report, solved) = solve::solve_centroids(
                &api::DB,
                &frame.centroids,
                (frame.width as usize, frame.height as usize),
                &SolveOptions {
                    fov: None,
                    timeout_ms: u64::MAX,
                    ..options
                },
            );
            let scale = self.window.lock().unwrap().is_some_and(|(first, last)| {
                frame.id >= first && last.is_none_or(|last| frame.id <= last)
            });
            let solved = if scale {
                report.pointing.as_ref().and_then(|p| {
                    Frame::from_solution(&plate_solver::Solution {
                        ra: Some(p.ra_deg.to_radians()),
                        dec: Some(p.dec_deg.to_radians()),
                        roll: Some(p.roll_deg.to_radians()),
                        distortion: Some(p.distortion),
                        camera: Some(PinholeCamera::new(
                            f64::from(frame.width),
                            f64::from(frame.height),
                            (0.7 * p.fov_deg).to_radians(),
                        )),
                        ..Default::default()
                    })
                })
            } else {
                solved
            };
            (report, solved)
        })
    }
}
async fn field_rig() -> (Rig, Arc<Scale>) {
    let scale = Arc::new(Scale::default());
    let rig = rig_with(
        MountConfig {
            base_tilt_deg: 1.0,
            base_tilt_toward_deg: 130.0,
            zero_points_deg: [120.0, 50.0],
            backlash_deg: 0.3,
        },
        scale.clone(),
    )
    .await;
    rig.second().await;
    (rig, scale)
}

#[tokio::test(start_paused = true)]
async fn goto_passes_over_out_of_range_solves_and_uses_a_taken_sync() {
    let (rig, scale) = field_rig().await;
    let epochs = rig.state.session().model(rig.now()).epochs.len();
    assert_eq!(rig.call("/api/goto", false).await.0, 200);
    let mut latest = None;
    let mut first_error = None;
    let mut saw_small_field = false;
    let progress = rig
        .finish_with(Some(15.0), true, |p| {
            if latest.is_none() && p["state"] == "solving" {
                let id = latest_frame(&rig);
                *scale.window.lock().unwrap() = Some((id + 1, Some(id + 4)));
                latest = Some(id);
            }
            if let Some(error) = p["error_arcsec"].as_f64() {
                first_error.get_or_insert(error);
            }
            if let Some(field) = rig.state.session().snapshot(rig.now()).field {
                saw_small_field |= field.fov_deg.is_some_and(|fov| fov < 9.0);
            }
        })
        .await;
    let last = progress.last().unwrap();
    assert_eq!(last["state"], "done", "{progress:?}");
    assert!(last["frame_id"].as_u64().unwrap() > latest.unwrap() + 4);
    assert!(first_error.unwrap() < 120.0, "{progress:?}");
    assert!(last["error_arcsec"].as_f64().unwrap() < 60.0);
    assert!(rig.error_arcsec("M42") < 60.0);
    assert!(saw_small_field, "injected field must be observed live");
    assert_eq!(rig.state.session().model(rig.now()).epochs.len(), epochs);
}

#[tokio::test(start_paused = true)]
async fn goto_times_out_naming_the_last_solve_not_taken_as_a_sync() {
    let (rig, scale) = field_rig().await;
    let epochs = rig.state.session().model(rig.now()).epochs.len();
    assert_eq!(rig.call("/api/goto", false).await.0, 200);
    let mut window_start = None;
    let mut solving_at = None;
    let progress = rig
        .finish_with(Some(15.0), true, |p| {
            if window_start.is_none() && p["state"] == "solving" {
                let first = latest_frame(&rig) + 1;
                *scale.window.lock().unwrap() = Some((first, None));
                window_start = Some(first);
                solving_at = Some(rig.now());
            }
            assert!(p["error_arcsec"].is_null(), "{p}");
        })
        .await;
    let last = progress.last().unwrap();
    assert_eq!(last["state"], "failed", "{progress:?}");
    let id: u64 = last["reason"]
        .as_str()
        .unwrap()
        .strip_prefix("frame ")
        .unwrap()
        .strip_suffix(" solved but was not taken as a sync")
        .unwrap()
        .parse()
        .unwrap();
    assert!(id >= window_start.unwrap());
    let latest = latest_frame(&rig);
    assert!(
        id <= latest && latest - id <= 3,
        "named {id}, latest {latest}"
    );
    assert!(rig.now() - solving_at.unwrap() <= 35.0);
    assert!(last["error_arcsec"].is_null());
    assert_eq!(rig.state.session().model(rig.now()).epochs.len(), epochs);
}
