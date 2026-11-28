//! Nudges (055): one press of an arrow moves the telescope one step on the sky, the
//! way the arrow points in the view the observer picked. With Track off, a press is a
//! direct move of both axes through every guard a GoTo passes
//! (`goto::AxisMove::Offset`): each axis the step moves goes 0.5° past its end and
//! comes back, as a GoTo's approach does. A press writes nothing to the pointing model.

use crate::{
    api::{sim_json, ApiError, AppState, SessionWrite},
    goto::AxisMove,
    imaging::{solving::SolveView, ImagingView},
    mcp::guard,
    session::{PointingSource, SessionView, Site},
    sky::{self, Observer},
};
use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// A fine step on the sky, arcseconds (055 FR-002).
pub const FINE_ARCSEC: f64 = 30.0;
/// A coarse step on the sky, arcseconds (055 FR-002).
pub const COARSE_ARCSEC: f64 = 240.0;

const NO_LIVE_POINTING: &str = "the finder camera has no live pointing yet";
const NO_SITE: &str = "a site is needed";
const SOLVE_RUNNING: &str = "the imaging camera's solve is still running";
const SOLVE_FAILED: &str = "the imaging camera's latest frame did not solve";
const NOT_SOLVED: &str = "the imaging camera has not solved a frame yet";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Fine,
    Coarse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    Mount,
    FinderCamera,
    ImagingCamera,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NudgeAsk {
    pub direction: Direction,
    pub step: Step,
    pub view: View,
}

/// The step in the mount's view: `up_arcsec` the altitude, `right_arcsec` a clockwise
/// turn of azimuth (055 FR-001, FR-003).
#[derive(Debug, Serialize)]
pub struct NudgeView {
    pub up_arcsec: f64,
    pub right_arcsec: f64,
    pub moved: &'static str,
}

/// The imaging camera's frame on the axes, kept from the latest imaging solve that
/// solved until another solves (055 FR-005): `axes` are `[across, up]` rows, each over
/// the pixel's x and y columns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kept {
    pub frame_id: u64,
    pub axes: [[f64; 2]; 2],
}

/// `[up, right]` arcseconds in the mount's view for a press. The mount view's units are
/// exact. The finder camera's and the imaging camera's follow their frames (FR-004,
/// FR-005); `kept` holds the imaging camera's frame between solves.
pub fn offset(
    ask: &NudgeAsk,
    session: &SessionView,
    imaging: Option<&ImagingView>,
    kept: &Mutex<Option<Kept>>,
) -> Result<[f64; 2], String> {
    let size = match ask.step {
        Step::Fine => FINE_ARCSEC,
        Step::Coarse => COARSE_ARCSEC,
    };
    // The way of the press, clockwise of the view's up.
    let way = match ask.direction {
        Direction::Up => 0.0,
        Direction::Right => 90.0,
        Direction::Down => 180.0,
        Direction::Left => 270.0,
    };
    match ask.view {
        View::Mount => Ok(match ask.direction {
            Direction::Up => [size, 0.0],
            Direction::Right => [0.0, size],
            Direction::Down => [-size, 0.0],
            Direction::Left => [0.0, -size],
        }),
        View::FinderCamera => {
            let pointing = session
                .pointing
                .as_ref()
                .filter(|pointing| pointing.source == PointingSource::Live)
                .ok_or_else(|| NO_LIVE_POINTING.to_string())?;
            let zenith = pointing.up_deg.ok_or_else(|| NO_SITE.to_string())?;
            // The live frame, turned counter-clockwise by the chart rotation.
            let turn = way + session.chart_rotation_deg.unwrap_or(0.0) - zenith;
            let (sin, cos) = turn.to_radians().sin_cos();
            Ok([size * cos, size * sin])
        }
        View::ImagingCamera => imaging_offset(session, imaging, kept, size, way),
    }
}

/// The imaging camera's answer: the latest solve that solved replaces the kept frame,
/// and the press is read in that frame (FR-005).
fn imaging_offset(
    session: &SessionView,
    imaging: Option<&ImagingView>,
    keep: &Mutex<Option<Kept>>,
    size: f64,
    way: f64,
) -> Result<[f64; 2], String> {
    let mut kept = keep.lock().expect("nudge state");
    let latest = imaging.and_then(|view| view.solve.as_ref());
    if let Some(solve) = latest.filter(|solve| solve.state == "solved") {
        if (*kept).is_none_or(|known| known.frame_id != solve.frame_id) {
            let site = session.site.ok_or_else(|| NO_SITE.to_string())?;
            let time = mid_exposure(imaging, solve).unwrap_or(solve.requested_at);
            let axes = frame_axes(solve, site, time).ok_or_else(|| SOLVE_FAILED.to_string())?;
            *kept = Some(Kept {
                frame_id: solve.frame_id,
                axes,
            });
        }
    }
    let Some(frame) = *kept else {
        return Err(match latest.map(|solve| solve.state) {
            None => NOT_SOLVED,
            Some("solving") => SOLVE_RUNNING,
            Some(_) => SOLVE_FAILED,
        }
        .to_string());
    };
    if session.site.is_none() {
        return Err(NO_SITE.to_string());
    }
    // The press's pixel direction: `way` clockwise of the frame's up, which is towards
    // row 0 (`-y`), with right towards the last column (`+x`).
    let (sin, cos) = way.to_radians().sin_cos();
    let direction = [sin, -cos];
    let across = frame.axes[0][0] * direction[0] + frame.axes[0][1] * direction[1];
    let up = frame.axes[1][0] * direction[0] + frame.axes[1][1] * direction[1];
    let length = across.hypot(up);
    if length.is_nan() || length <= 0.0 {
        return Err(NOT_SOLVED.to_string());
    }
    Ok([size * up / length, size * across / length])
}

/// The frame's mid-exposure time while the imaging view still holds the solved frame
/// (its start plus half its exposure), else `None`.
fn mid_exposure(imaging: Option<&ImagingView>, solve: &SolveView) -> Option<f64> {
    let frame = imaging?
        .frame
        .as_ref()
        .filter(|frame| frame.id == solve.frame_id)?;
    Some(frame.start? + frame.exposure_ms / 2000.0)
}

/// The across and up axes of a solve's frame at a site and time: `[across, up]` rows over
/// the pixel's x (right) and y (down) columns. `cd_deg` gives degrees east (row 0) and
/// north (row 1) for one pixel right (column 0) and one pixel down (column 1); east and
/// north turn into across and up with the parallactic angle, as `tracking.rs` turns them.
fn frame_axes(solve: &SolveView, site: Site, time: f64) -> Option<[[f64; 2]; 2]> {
    let cd = solve.cd_deg?;
    let q = parallactic(solve.ra_deg?, solve.dec_deg?, site, time);
    let (sin, cos) = q.sin_cos();
    let across = [
        -cos * cd[0][0] + sin * cd[1][0],
        -cos * cd[0][1] + sin * cd[1][1],
    ];
    let up = [
        sin * cd[0][0] + cos * cd[1][0],
        sin * cd[0][1] + cos * cd[1][1],
    ];
    // Degenerate when the frame's axes fold onto a line: no press has a direction.
    let determinant = across[0] * up[1] - across[1] * up[0];
    (determinant.is_finite() && determinant != 0.0).then_some([across, up])
}

/// The parallactic angle of J2000 north at a place and time, in radians: the same turn
/// as `tracking.rs`'s `axis_frame` makes.
fn parallactic(ra_deg: f64, dec_deg: f64, site: Site, time: f64) -> f64 {
    let observer = Observer {
        lat: site.lat_deg,
        lon: site.lon_deg,
        time,
    };
    let (alt, az) = sky::alt_az(ra_deg, dec_deg, &observer);
    let (north_alt, north_az) = sky::alt_az(ra_deg, dec_deg + 1e-5, &observer);
    let cross = ((north_az - az + 180.0).rem_euclid(360.0) - 180.0) * alt.to_radians().cos();
    cross.atan2(north_alt - alt)
}

/// `POST /api/mount/nudge`: a same-origin write, answered by `nudge` for the page.
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/mount/nudge", post(press))
}

async fn press(
    State(state): State<AppState>,
    _write: SessionWrite,
    ask: Result<Json<NudgeAsk>, JsonRejection>,
) -> Result<Json<NudgeView>, ApiError> {
    let ask = sim_json(ask)?;
    Ok(Json(nudge(&state, "page", &ask).await?))
}

/// One press, by `by` (`page` or `mcp`). Its refusals answer 409 with their words.
pub async fn nudge(
    state: &AppState,
    by: &'static str,
    ask: &NudgeAsk,
) -> Result<NudgeView, ApiError> {
    let session = state.session().snapshot(state.clock.now());
    let imaging = state.imaging().map(|imaging| imaging.view());
    let [up, right] = offset(ask, &session, imaging.as_ref(), &state.nudge_kept)
        .map_err(|reason| ApiError::new(StatusCode::CONFLICT, reason))?;
    guard::start_by(
        state,
        by,
        "nudge",
        AxisMove::Offset {
            up_arcsec: up,
            right_arcsec: right,
        },
    )
    .await?;
    Ok(NudgeView {
        up_arcsec: up,
        right_arcsec: right,
        moved: "axes",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        imaging::{frames::FrameView, solver::SolveHint, RoiView},
        session::{LatestPointing, Pointing, Session},
        sky::Frame,
    };

    const START: f64 = 1_768_528_800.0;
    /// A solved frame's centre, degrees.
    const RA: f64 = 83.82;
    const DEC: f64 = -5.39;
    /// One arcsecond a pixel, in degrees.
    const S: f64 = 1.0 / 3600.0;

    fn unmirrored() -> [[f64; 2]; 2] {
        [[-S, 0.0], [0.0, -S]]
    }

    fn mirrored() -> [[f64; 2]; 2] {
        [[S, 0.0], [0.0, -S]]
    }

    fn ask(direction: Direction, step: Step, view: View) -> NudgeAsk {
        NudgeAsk {
            direction,
            step,
            view,
        }
    }

    /// A solve of frame `frame_id` in `state`, asked for at `requested_at`.
    fn solve(
        frame_id: u64,
        state: &'static str,
        requested_at: f64,
        cd: [[f64; 2]; 2],
    ) -> SolveView {
        let solved = state == "solved";
        SolveView {
            frame_id,
            state,
            requested_at,
            solver: "simulated",
            hint: SolveHint::whole_sky(),
            ra_deg: solved.then_some(RA),
            dec_deg: solved.then_some(DEC),
            up_east_of_north_deg: solved.then_some(0.0),
            arcsec_per_px: solved.then_some(1.0),
            mirrored: solved.then_some(cd[0][0] * cd[1][1] - cd[0][1] * cd[1][0] < 0.0),
            cd_deg: solved.then_some(cd),
            stars: None,
            warning: None,
            took_ms: None,
            file: None,
            reason: None,
            error: None,
            detail: None,
        }
    }

    /// The imaging view of a ready console with `solve` and the frame `frame`.
    fn imaging(solve: Option<SolveView>, frame: Option<FrameView>) -> ImagingView {
        let mut view = ImagingView::off();
        view.state = "ready";
        view.solve = solve;
        view.frame = frame;
        view
    }

    /// A frame saved with id `id`, begun at `start` and exposed for `exposure_ms`.
    fn held(id: u64, start: f64, exposure_ms: f64) -> FrameView {
        FrameView {
            id,
            state: "saved",
            purpose: "imaging".into(),
            exposure_ms,
            requested_at: start,
            roi: RoiView {
                x: 0,
                y: 0,
                width: 1236,
                height: 822,
            },
            set_temp_c: None,
            width: None,
            height: None,
            bits: None,
            bayer: None,
            gain: None,
            offset: None,
            binning: None,
            temp_c: None,
            start: Some(start),
            date_obs: None,
            bytes: None,
            received: None,
            node_file: None,
            node_save_error: None,
            received_at: None,
            saved_at: None,
            file: None,
            preview: None,
            error: None,
        }
    }

    /// The session at 40° N 75° W with a live pointing whose `up_deg` is `up`, and the
    /// chart rotation `rotation`.
    fn live(up: Option<f64>, rotation: Option<f64>) -> SessionView {
        let mut view = Session::default().snapshot(START);
        view.site = Some(Site {
            lat_deg: 40.0,
            lon_deg: -75.0,
        });
        view.chart_rotation_deg = rotation;
        let frame = Frame::from_solution(&plate_solver::Solution {
            ra: Some(1.0),
            dec: Some(0.0),
            roll: Some(0.0),
            camera: Some(math_core::PinholeCamera::new(800.0, 600.0, 0.2)),
            ..Default::default()
        })
        .unwrap();
        view.pointing = Some(LatestPointing {
            pointing: Pointing {
                seq: 1,
                frame,
                ra_deg: 1.0_f64.to_degrees(),
                dec_deg: 0.0,
                alt_deg: Some(30.0),
                az_deg: Some(350.0),
                time: START,
            },
            source: PointingSource::Live,
            frame_id: Some(1),
            up_deg: up,
        });
        view
    }

    /// The smallest angle in degrees between two `[up, right]` answers.
    fn turned(a: [f64; 2], b: [f64; 2]) -> f64 {
        let (a, b) = (a[1].atan2(a[0]), b[1].atan2(b[0]));
        let turn = (a - b).to_degrees().rem_euclid(360.0);
        turn.min(360.0 - turn)
    }

    fn assert_same(a: [f64; 2], b: [f64; 2]) {
        assert!(
            (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9,
            "{a:?} != {b:?}"
        );
    }

    // US2-S2: the finder camera's offset turns with the zenith and the chart rotation.
    #[test]
    fn the_finder_camera_offset_turns_with_the_zenith_and_the_chart_rotation() {
        let kept = Mutex::new(None);
        for rotation in [0.0, 200.0] {
            let session = live(Some(40.0), Some(rotation));
            for (direction, way) in [
                (Direction::Up, 0.0),
                (Direction::Right, 90.0),
                (Direction::Down, 180.0),
                (Direction::Left, 270.0),
            ] {
                let [up, right] = offset(
                    &ask(direction, Step::Coarse, View::FinderCamera),
                    &session,
                    None,
                    &kept,
                )
                .unwrap();
                let a = (way + rotation - 40.0_f64).to_radians();
                assert!(
                    (up - 240.0 * a.cos()).abs() < 1e-9 && (right - 240.0 * a.sin()).abs() < 1e-9,
                    "{direction:?} at rotation {rotation}: [{up}, {right}]"
                );
            }
        }
    }

    #[test]
    fn the_finder_camera_refuses_without_a_live_pointing_or_a_site() {
        let kept = Mutex::new(None);
        let mut none = live(Some(40.0), None);
        none.pointing = None;
        let refusal = offset(
            &ask(Direction::Up, Step::Fine, View::FinderCamera),
            &none,
            None,
            &kept,
        );
        assert_eq!(refusal, Err(NO_LIVE_POINTING.to_string()));
        let without_site = live(None, None);
        let refusal = offset(
            &ask(Direction::Up, Step::Fine, View::FinderCamera),
            &without_site,
            None,
            &kept,
        );
        assert_eq!(refusal, Err(NO_SITE.to_string()));
    }

    // US2-S4: a mirrored frame's right is the unmirrored frame's left, and a solve asked
    // for while the frame it solved is still held is read at the frame's mid-exposure.
    #[test]
    fn a_mirrored_frame_swaps_right_and_left_and_a_held_frame_is_read_at_mid_exposure() {
        let session = live(None, None);
        let coarse = |direction: Direction, view: &ImagingView| {
            offset(
                &ask(direction, Step::Coarse, View::ImagingCamera),
                &session,
                Some(view),
                &Mutex::new(None),
            )
            .unwrap()
        };
        let plain = imaging(Some(solve(1, "solved", START, unmirrored())), None);
        let flipped = imaging(Some(solve(2, "solved", START, mirrored())), None);
        assert_same(
            coarse(Direction::Up, &flipped),
            coarse(Direction::Up, &plain),
        );
        assert_same(
            coarse(Direction::Down, &flipped),
            coarse(Direction::Down, &plain),
        );
        assert_same(
            coarse(Direction::Right, &flipped),
            coarse(Direction::Left, &plain),
        );
        assert_same(
            coarse(Direction::Left, &flipped),
            coarse(Direction::Right, &plain),
        );

        // Asked for at T while its frame, begun at T - 1,200 s for 2 s, is still held:
        // read at T - 1,199 s, as a solve asked for at T - 1,199 s is.
        let held_late = imaging(
            Some(solve(3, "solved", START, unmirrored())),
            Some(held(3, START - 1200.0, 2000.0)),
        );
        let early = imaging(Some(solve(3, "solved", START - 1199.0, unmirrored())), None);
        let at_t = imaging(Some(solve(3, "solved", START, unmirrored())), None);
        for direction in [
            Direction::Up,
            Direction::Right,
            Direction::Down,
            Direction::Left,
        ] {
            let late = coarse(direction, &held_late);
            assert_same(late, coarse(direction, &early));
            assert!(
                turned(late, coarse(direction, &at_t)) > 1.0,
                "{direction:?}: the mid-exposure answer is not more than 1° from the one at T"
            );
        }
    }

    // US2-S5: a later solve that does not solve keeps the frame; with none kept, the
    // three words; and without a site, `a site is needed`.
    #[test]
    fn the_kept_frame_outlasts_later_solves_that_do_not_solve_and_names_each_refusal() {
        let session = live(None, None);
        let right = |view: Option<&ImagingView>, kept: &Mutex<Option<Kept>>| {
            offset(
                &ask(Direction::Right, Step::Coarse, View::ImagingCamera),
                &session,
                view,
                kept,
            )
        };
        let kept = Mutex::new(None);
        let solved_at_t = imaging(Some(solve(1, "solved", START, unmirrored())), None);
        let answered = right(Some(&solved_at_t), &kept).unwrap();
        for state in ["solving", "failed"] {
            let later = imaging(Some(solve(1, state, START + 60.0, unmirrored())), None);
            assert_same(right(Some(&later), &kept).unwrap(), answered);
        }

        // With nothing kept, the latest solve's state names the refusal.
        let words = [("solving", SOLVE_RUNNING), ("failed", SOLVE_FAILED)];
        for (state, words) in words {
            let view = imaging(Some(solve(2, state, START + 60.0, unmirrored())), None);
            assert_eq!(
                right(Some(&view), &Mutex::new(None)),
                Err(words.to_string())
            );
        }
        assert_eq!(
            right(None, &Mutex::new(None)),
            Err("the imaging camera has not solved a frame yet".to_string())
        );

        // Without a site, a solved frame is not turned, kept or not.
        let mut without_site = live(None, None);
        without_site.site = None;
        let refusal = offset(
            &ask(Direction::Right, Step::Coarse, View::ImagingCamera),
            &without_site,
            Some(&solved_at_t),
            &Mutex::new(None),
        );
        assert_eq!(refusal, Err("a site is needed".to_string()));
        let refusal = offset(
            &ask(Direction::Right, Step::Coarse, View::ImagingCamera),
            &without_site,
            Some(&solved_at_t),
            &kept,
        );
        assert_eq!(refusal, Err("a site is needed".to_string()));
    }
}
