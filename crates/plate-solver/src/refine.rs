//! Refinement stage: re-fit attitude over all matches, extract RA/Dec/Roll,
//! refine FOV/distortion, compute residuals, and assemble the final solution.

use crate::status::{Solution, SolveStatus, VerificationOutcome};
use math_core::{
    attitude::{extract_radec_roll, solve_attitude},
    fov::{refine_fov, refine_fov_with_distortion},
    PinholeCamera, UnitVector,
};

/// Refine an accepted candidate into a fully populated `Solution`.
///
/// Steps:
/// 1. Re-fit attitude over all matched pairs with the SVD Wahba solver.
/// 2. Extract RA/Dec/Roll from the refined rotation matrix.
/// 3. Refine FOV (and optionally distortion) from the matched pairs.
/// 4. Recompute residuals (RMSE/P90E/MAXE) in arcseconds.
/// 5. Assemble the final `Solution` with camera, matches, and status.
pub fn refine_solution<D>(
    ctx: &crate::status::SolveContext<D>,
    candidate: &pattern_database::Candidate,
    outcome: &VerificationOutcome,
    prep: &crate::preparation::Prepared,
    width: f64,
    height: f64,
) -> Solution {
    let no_match = || Solution {
        status: Some(SolveStatus::NoMatch),
        fov_used: Some(ctx.fov_initial),
        ..Solution::default()
    };
    if !outcome.accepted || outcome.matched_image_vectors.is_empty() {
        return no_match();
    }

    // 1. Re-fit the attitude over all matches, image vectors at the coarse FOV.
    let Some((rotation, ra, dec, roll)) =
        fit_attitude(&outcome.matched_image_vectors, &outcome.matched_stars)
    else {
        return no_match();
    };

    // 2. FOV and distortion by least squares on (tan(theta_catalog), r_image), cedar's
    //    model r(1 - k r^2) = f tan(theta) with r in units of width/2. Catalog vectors
    //    are derotated into the camera frame first.
    let raw: Vec<(f64, f64)> = outcome
        .matched_image_indices
        .iter()
        .map(|&i| prep.raw_centroids[i])
        .collect();
    let (cy, cx) = (height / 2.0, width / 2.0);
    let (mut a00, mut a01, mut a11, mut b0, mut b1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (c, cat) in raw.iter().zip(outcome.matched_stars.iter()) {
        let d = rotation.rotate(*cat);
        let t = (d.y * d.y + d.z * d.z).sqrt() / d.x;
        let r = ((c.0 - cy).powi(2) + (c.1 - cx).powi(2)).sqrt() / width * 2.0;
        let r3 = r * r * r;
        a00 += t * t;
        a01 += t * r3;
        a11 += r3 * r3;
        b0 += t * r;
        b1 += r3 * r;
    }
    let det = a00 * a11 - a01 * a01;
    let (mut fov, mut k) = (outcome.coarse_fov, 0.0);
    if det.abs() > 1e-300 {
        let f = (a11 * b0 - a01 * b1) / det;
        let k_est = (a00 * b1 - a01 * b0) / det;
        let f = f / (1.0 - k_est);
        let fov_est = 2.0 * (1.0 / f).atan();
        if fov_est.is_finite() && fov_est > 0.0 {
            fov = fov_est;
            k = k_est;
        }
    }

    // PROTOTYPE (not cedar): iterate re-match + least squares + attitude re-fit with
    // the refined camera model. Cedar stops after one pass, which on wide, distorted
    // fields can leave the boresight off by many arcminutes. Opt-in for now.
    let mut raw = raw;
    let mut matched_stars = outcome.matched_stars.clone();
    let mut matched_ids = outcome.matched_catalog_ids.clone();
    let mut matched_cents = outcome.matched_centroids.clone();
    let (mut rotation, mut ra, mut dec, mut roll) = (rotation, ra, dec, roll);
    if std::env::var_os("PLATE_SOLVER_ITERATIVE_REFINE").is_some() {
        for _ in 0..3 {
            let Some(pass) = rematch(ctx, prep, &rotation, fov, k, width, height) else {
                break;
            };
            raw = pass.iter().map(|&(i, _)| prep.raw_centroids[i]).collect();
            matched_ids = pass.iter().map(|&(_, s)| s).collect();
            matched_stars = matched_ids
                .iter()
                .map(|&s| {
                    ctx.db
                        .star_vector(pattern_database::StarId(s))
                        .expect("valid")
                })
                .collect();
            if let Some((f, kk)) = fit_focal_and_k(&rotation, &raw, &matched_stars, width, height) {
                fov = f;
                k = kk;
            }
            let und = math_core::undistort_centroids(&raw, width, height, k);
            matched_cents = und.clone();
            let v: Vec<UnitVector> = PinholeCamera::new(width, height, fov)
                .unproject(&und)
                .into_iter()
                .map(|v| v.expect("never zero-length"))
                .collect();
            if let Some(fit) = fit_attitude(&v, &matched_stars) {
                (rotation, ra, dec, roll) = fit;
            }
        }
    }

    // 3. Residuals: matched centroids re-undistorted with k, unprojected at the
    //    refined FOV, rotated to the sky (R^T v), compared to the catalog vectors.
    let undist = math_core::undistort_centroids(&raw, width, height, k);
    let camera = PinholeCamera::new(width, height, fov);
    let r = &rotation.rows;
    let mut angles: Vec<f64> = camera
        .unproject(&undist)
        .into_iter()
        .zip(matched_stars.iter())
        .map(|(v, cat)| {
            let v = v.expect("never zero-length");
            let sky = UnitVector {
                x: r[0][0] * v.x + r[1][0] * v.y + r[2][0] * v.z,
                y: r[0][1] * v.x + r[1][1] * v.y + r[2][1] * v.z,
                z: r[0][2] * v.x + r[1][2] * v.y + r[2][2] * v.z,
            };
            math_core::angular_distance(sky, *cat)
        })
        .collect();
    angles.sort_by(|a, b| a.total_cmp(b));
    let to_arcsec = 180.0 / std::f64::consts::PI * 3600.0;
    let rmse = (angles.iter().map(|a| a * a).sum::<f64>() / angles.len() as f64).sqrt() * to_arcsec;
    let p90e = angles[((angles.len() - 1) as f64 * 0.9) as usize] * to_arcsec;
    let maxe = angles[angles.len() - 1] * to_arcsec;

    Solution {
        status: Some(SolveStatus::MatchFound),
        camera: Some(camera),
        matched_centroids: matched_cents,
        matched_stars,
        matched_catalog_ids: matched_ids,
        match_probability: outcome.match_probability,
        fov_used: Some(camera.fov),
        pattern_candidates: vec![*candidate],
        ra: Some(ra),
        dec: Some(dec),
        roll: Some(roll),
        rmse: Some(rmse),
        p90e: Some(p90e),
        maxe: Some(maxe),
        distortion: Some(k),
        noise_estimate: None,
    }
}

/// One re-match pass: undistort every verification centroid with `k`, project the
/// catalog with the current attitude and FOV, and pair each centroid with the
/// nearest unused catalog star within `match_radius * width`.
fn rematch(
    ctx: &crate::status::SolveContext,
    prep: &crate::preparation::Prepared,
    rotation: &math_core::attitude::RotationMatrix,
    fov: f64,
    k: f64,
    width: f64,
    height: f64,
) -> Option<Vec<(usize, usize)>> {
    let camera = PinholeCamera::new(width, height, fov);
    let undist = math_core::undistort_centroids(&prep.raw_centroids, width, height, k);
    let r = &rotation.rows;
    let bore = UnitVector {
        x: r[0][0],
        y: r[0][1],
        z: r[0][2],
    };
    let radius = math_core::fov::diagonal_fov(fov, width, height) / 2.0 * 1.05;
    let nearby = ctx.db.nearby_stars(bore, radius);
    let derot: Vec<UnitVector> = nearby
        .iter()
        .map(|&i| {
            rotation.rotate(
                ctx.db
                    .star_vector(pattern_database::StarId(i))
                    .expect("valid"),
            )
        })
        .collect();
    let (px, in_frame) = camera.project(&derot);
    let kept: Vec<usize> = in_frame.into_iter().take(2 * undist.len()).collect();
    let r2 = (ctx.match_radius * width).powi(2);
    let mut used = vec![false; kept.len()];
    let mut pairs = Vec::new();
    for (i, c) in undist.iter().enumerate() {
        let mut best: Option<usize> = None;
        let mut best_d2 = r2;
        for (q, &kk) in kept.iter().enumerate() {
            let d2 = (px[kk].0 - c.0).powi(2) + (px[kk].1 - c.1).powi(2);
            if !used[q] && d2 < best_d2 {
                best_d2 = d2;
                best = Some(q);
            }
        }
        if let Some(q) = best {
            used[q] = true;
            pairs.push((i, nearby[kept[q]]));
        }
    }
    (pairs.len() >= 4).then_some(pairs)
}

/// Cedar's least squares for focal length and distortion: `r(1 - k r^2) = f tan(theta)`
/// with `r` in units of width/2 and catalog vectors derotated into the camera frame.
/// Returns `(fov, k)`.
fn fit_focal_and_k(
    rotation: &math_core::attitude::RotationMatrix,
    raw: &[(f64, f64)],
    catalog: &[UnitVector],
    width: f64,
    height: f64,
) -> Option<(f64, f64)> {
    let (mut a00, mut a01, mut a11, mut b0, mut b1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (c, cat) in raw.iter().zip(catalog) {
        let cam = rotation.rotate(*cat);
        let t = (cam.y * cam.y + cam.z * cam.z).sqrt() / cam.x;
        let r = ((c.0 - height / 2.0).powi(2) + (c.1 - width / 2.0).powi(2)).sqrt() / width * 2.0;
        let r3 = r * r * r;
        a00 += t * t;
        a01 += t * r3;
        a11 += r3 * r3;
        b0 += t * r;
        b1 += r3 * r;
    }
    let det = a00 * a11 - a01 * a01;
    if det.abs() <= 1e-300 {
        return None;
    }
    let k = (a00 * b1 - a01 * b0) / det;
    let f = (a11 * b0 - a01 * b1) / det / (1.0 - k);
    let fov = 2.0 * (1.0 / f).atan();
    (fov.is_finite() && fov > 0.0).then_some((fov, k))
}

/// Solve attitude over all matched pairs and extract RA/Dec/Roll.
fn fit_attitude(
    image_vectors: &[UnitVector],
    catalog_vectors: &[UnitVector],
) -> Option<(math_core::attitude::RotationMatrix, f64, f64, f64)> {
    let rotation = solve_attitude(image_vectors, catalog_vectors)?;
    let (ra, dec, roll) = extract_radec_roll(&rotation);
    Some((rotation, ra, dec, roll))
}

/// Refine FOV from matched pairs, optionally estimating distortion.
#[allow(dead_code)]
fn refine_camera(
    fov: f64,
    width: f64,
    height: f64,
    image_vectors: &[UnitVector],
    catalog_vectors: &[UnitVector],
    estimate_distortion: bool,
) -> (PinholeCamera, Option<f64>) {
    if estimate_distortion {
        let (refined_fov, k) =
            refine_fov_with_distortion(fov, width, height, image_vectors, catalog_vectors);
        (PinholeCamera::new(width, height, refined_fov), Some(k))
    } else {
        let refined_fov = refine_fov(fov, width, height, image_vectors, catalog_vectors, None).0;
        (PinholeCamera::new(width, height, refined_fov), None)
    }
}
