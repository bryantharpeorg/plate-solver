//! Pointing model and sync intake (TPoint §7.2, Meeus ch. 14 and 16).
//!
//! Signs follow the [TPoint manual](https://newapo.apo.nmsu.edu/static/pdf/tcc/tpoint.pdf)
//! and `docs/research/2026-10-21-pointing-model-and-tracking.md` §1.
//! Mechanical predictions are apparent directions: catalog altitude is converted
//! first, keeping atmospheric refraction out of the fitted mechanical terms.

use crate::{
    ephemeris,
    session::Aim,
    sky::{self, Frame, Observer},
};
use scope_link::proto::StarFrame;
use serde::{Deserialize, Serialize};

/// A solved aim point and interpolated axis positions, all at mid-exposure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sync {
    /// Console Unix seconds.
    pub time: f64,
    /// Azimuth and altitude axis positions, in counts (fractional at mid-exposure).
    pub axis: [f64; 2],
    /// Scale reported by the mount, azimuth then altitude.
    pub counts_per_revolution: [u32; 2],
    /// Azimuth north through east, degrees.
    pub az_deg: f64,
    /// Observed apparent altitude, degrees; refraction is accounted for before fitting.
    pub alt_deg: f64,
    /// Camera up relative to the horizon's altitude tangent, toward increasing azimuth.
    /// Derived from solve roll and the parallactic angle, in degrees.
    pub roll_deg: Option<f64>,
    /// Solve RMS in arcseconds, used as an inverse standard-deviation weight.
    pub rms_arcsec: f64,
    /// Index of the zero-point epoch; assigned at intake.
    pub epoch: usize,
}

/// The zero points for a stretch of the night. TPoint signs: A−IA, E+IE.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Epoch {
    /// Azimuth zero point in degrees.
    pub ia: f64,
    /// Altitude zero point in degrees.
    pub ie: f64,
}
/// Exact horizontal rotation, parameterised by TPoint AN and AW (degrees).
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BaseTilt {
    /// North term; lowers true altitude at azimuth zero.
    pub an: f64,
    /// West term; raises true altitude at azimuth 90°.
    pub aw: f64,
}
impl BaseTilt {
    /// Magnitude in degrees.
    pub fn deg(self) -> f64 {
        self.an.hypot(self.aw)
    }
    /// Azimuth toward which the base leans, in degrees.
    pub fn toward_deg(self) -> f64 {
        (-self.aw).atan2(self.an).to_degrees().rem_euclid(360.0)
    }
}
/// Hardware terms, in degrees, retained until explicitly reset.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FixedTerms {
    /// Collimation error.
    pub ca: f64,
    /// Non-perpendicularity of the axes.
    pub npae: f64,
    /// Tube flexure.
    pub tf: f64,
    /// Camera up's rotation toward increasing azimuth about the tube.
    pub camera: f64,
}
/// A prediction in the observed horizontal frame.
#[derive(Debug, Clone, Copy)]
pub struct Prediction {
    /// Azimuth in degrees.
    pub az_deg: f64,
    /// Apparent altitude in degrees.
    pub alt_deg: f64,
    /// Camera rotation relative to increasing altitude, degrees.
    pub roll_deg: f64,
}
/// Nightly syncs and zero points, with shared fixed terms.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// The retained, thinned syncs.
    pub syncs: Vec<Sync>,
    /// Zero points, oldest epoch first.
    pub epochs: Vec<Epoch>,
    /// Absent until two syncs in an epoch are at least 90° apart.
    pub base_tilt: Option<BaseTilt>,
    /// Absent until the count and spread requirements are met.
    pub fixed: Option<FixedTerms>,
    /// Next solar upper transit after the first sync, in console Unix seconds.
    pub expires_at: Option<f64>,
    /// Unweighted sky residual RMS in arcseconds.
    pub rms_arcsec: f64,
    #[serde(default)]
    fixed_since: Option<f64>,
}
/// Session API projection; absent until the model has syncs or fixed terms.
#[derive(Debug, Serialize)]
pub struct ModelView {
    /// Number of retained syncs.
    pub syncs: usize,
    /// Current zero-point epoch, numbered from one; zero before a nightly sync.
    pub epoch: usize,
    /// Azimuth zero point, degrees.
    pub ia: Option<f64>,
    /// Altitude zero point, degrees.
    pub ie: Option<f64>,
    /// Magnitude and direction of the fitted base tilt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_tilt: Option<BaseTiltView>,
    /// Shared hardware terms, degrees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed: Option<FixedTerms>,
    /// RMS on the sky, arcseconds.
    pub rms_arcsec: f64,
    /// Further syncs required; at least one when more sky coverage is needed.
    pub needs: usize,
}
/// Human-readable base tilt, in degrees.
#[derive(Debug, Serialize)]
pub struct BaseTiltView {
    /// Magnitude.
    pub deg: f64,
    /// Azimuth toward which the base leans.
    pub toward_deg: f64,
}

/// Interpolate both axes to mid-exposure, rejecting movement above 0.2°/s.
/// Differences are formed in f64 so signed count extremes cannot overflow.
pub fn mid_exposure(frame: &StarFrame, counts: [u32; 2]) -> Option<[f64; 2]> {
    let (start, end) = (frame.axis_start.as_ref()?, frame.axis_end.as_ref()?);
    let duration = frame.end_s - frame.start_s;
    if !duration.is_finite() || duration <= 0.0 || counts.contains(&0) {
        return None;
    }
    let start = [f64::from(start.azimuth), f64::from(start.altitude)];
    let end = [f64::from(end.azimuth), f64::from(end.altitude)];
    if (0..2).any(|i| (end[i] - start[i]).abs() * 360.0 / f64::from(counts[i]) / duration > 0.2) {
        return None;
    }
    Some([0, 1].map(|i| (start[i] + end[i]) / 2.0))
}
impl Sync {
    /// Build a sync from a scope-node solve, never from an uploaded image.
    /// Transform catalog directions to the observed horizontal frame before fitting,
    /// separating Meeus refraction from mechanical terms. Transforming camera up
    /// as well as the aim ray includes parallactic rotation and precession of north.
    pub fn from_frame(
        frame: &StarFrame,
        solved: Frame,
        aim: Option<Aim>,
        site: Observer,
        counts: [u32; 2],
        rms_arcsec: f64,
    ) -> Option<Self> {
        let axis = mid_exposure(frame, counts)?;
        let (w, h) = solved.size();
        let aim = aim.unwrap_or(Aim {
            x: w / 2.0,
            y: h / 2.0,
        });
        let ray = solved.unproject((aim.x, aim.y))?;
        let up = solved.unproject((aim.x, aim.y - 0.01))?;
        let horizontal = |ray: math_core::UnitVector| {
            let (ra, dec) = ray.to_radec();
            sky::alt_az(ra.to_degrees(), dec.to_degrees(), &site)
        };
        let (alt, az) = horizontal(ray);
        let (up_alt, up_az) = horizontal(up);
        let roll = (wrap(up_az - az) * alt.to_radians().cos())
            .atan2(up_alt - alt)
            .to_degrees();
        Some(Self {
            time: site.time,
            axis,
            counts_per_revolution: counts,
            az_deg: az,
            alt_deg: ephemeris::apparent_from_true(alt),
            roll_deg: Some(roll),
            rms_arcsec,
            epoch: 0,
        })
    }
}
fn wrap(deg: f64) -> f64 {
    (deg + 180.0).rem_euclid(360.0) - 180.0
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn vector(az: f64, alt: f64) -> [f64; 3] {
    let (a, e) = (az.to_radians(), alt.to_radians());
    [e.cos() * a.cos(), e.cos() * a.sin(), e.sin()]
}
/// Great-circle separation in degrees, using the chord half-angle formula.
pub fn separation(a: f64, e: f64, b: f64, f: f64) -> f64 {
    let (v, w) = (vector(a, e), vector(b, f));
    let chord = (0..3).map(|i| (v[i] - w[i]).powi(2)).sum::<f64>().sqrt();
    (2.0 * (chord / 2.0).clamp(0.0, 1.0).asin()).to_degrees()
}
fn rotate(v: [f64; 3], base: BaseTilt) -> [f64; 3] {
    let t = base.deg();
    if t < 1e-15 {
        return v;
    }
    let k = [base.aw / t, base.an / t, 0.0];
    let (s, c) = t.to_radians().sin_cos();
    let cross = [k[1] * v[2], -k[0] * v[2], k[0] * v[1] - k[1] * v[0]];
    [0, 1, 2].map(|i| v[i] * c + cross[i] * s + k[i] * dot(k, v) * (1.0 - c))
}
fn forward(axis: [f64; 2], epoch: Epoch, base: BaseTilt, fixed: FixedTerms) -> Prediction {
    let a = (axis[0] - epoch.ia).to_radians();
    let e = (axis[1] + epoch.ie).to_radians();
    let v = rotate([e.cos() * a.cos(), e.cos() * a.sin(), e.sin()], base);
    let up = rotate([-e.sin() * a.cos(), -e.sin() * a.sin(), e.cos()], base);
    let az = v[1].atan2(v[0]);
    let alt = v[2].atan2(v[0].hypot(v[1]));
    let horizontal = [-az.sin(), az.cos(), 0.0];
    let vertical = [-alt.sin() * az.cos(), -alt.sin() * az.sin(), alt.cos()];
    Prediction {
        az_deg: (az.to_degrees() - fixed.ca / alt.cos() - fixed.npae * alt.tan()).rem_euclid(360.0),
        alt_deg: alt.to_degrees() - fixed.tf * alt.cos(),
        roll_deg: dot(up, horizontal).atan2(dot(up, vertical)).to_degrees() + fixed.camera,
    }
}
impl Model {
    /// Current-epoch forward model, axis positions in counts.
    pub fn predict(&self, axis: [f64; 2], counts: [u32; 2]) -> Option<Prediction> {
        if counts.contains(&0) || axis.iter().any(|a| !a.is_finite()) {
            return None;
        }
        Some(forward(
            [0, 1].map(|i| axis[i] * 360.0 / f64::from(counts[i])),
            *self.epochs.last()?,
            self.base_tilt.unwrap_or_default(),
            self.fixed.unwrap_or_default(),
        ))
    }
    /// Newton inverse of the exact forward model, returning fractional counts.
    /// Starts from the uncorrected target with the current zero points applied.
    pub fn inverse(&self, az: f64, alt: f64, counts: [u32; 2]) -> Option<[f64; 2]> {
        let epoch = *self.epochs.last()?;
        if !az.is_finite() || !alt.is_finite() || counts.contains(&0) {
            return None;
        }
        let mut axis = [az + epoch.ia, alt - epoch.ie];
        let evaluate = |axis| {
            forward(
                axis,
                epoch,
                self.base_tilt.unwrap_or_default(),
                self.fixed.unwrap_or_default(),
            )
        };
        for _ in 0..15 {
            let p = evaluate(axis);
            let error = [wrap(az - p.az_deg), alt - p.alt_deg];
            if error[0].abs().max(error[1].abs()) < 1e-9 {
                return Some([0, 1].map(|i| axis[i] * f64::from(counts[i]) / 360.0));
            }
            let derivatives = [0, 1].map(|i| {
                let mut trial = axis;
                trial[i] += 1e-5;
                let q = evaluate(trial);
                [
                    wrap(q.az_deg - p.az_deg) / 1e-5,
                    (q.alt_deg - p.alt_deg) / 1e-5,
                ]
            });
            let det = derivatives[0][0] * derivatives[1][1] - derivatives[1][0] * derivatives[0][1];
            if det.abs() < 1e-10 {
                return None;
            }
            axis[0] += (error[0] * derivatives[1][1] - error[1] * derivatives[1][0]) / det;
            axis[1] += (error[1] * derivatives[0][0] - error[0] * derivatives[0][1]) / det;
        }
        None
    }
    /// Expire only the night's data. Fixed terms and their reset boundary survive.
    pub fn expire(&mut self, time: f64) -> bool {
        if self.expires_at.is_some_and(|noon| time >= noon) {
            self.reset_night();
            true
        } else {
            false
        }
    }
    /// Clear nightly syncs, zero points and base tilt, retaining fixed terms.
    pub fn reset_night(&mut self) {
        self.syncs.clear();
        self.epochs.clear();
        self.base_tilt = None;
        self.expires_at = None;
        self.rms_arcsec = 0.0;
    }
    /// Clear hardware calibration. Only subsequently acquired syncs can refit it.
    pub fn reset_fixed(&mut self) {
        self.fixed = None;
        self.fixed_since = self.syncs.iter().map(|s| s.time).max_by(f64::total_cmp);
    }
    fn eligible(&self) -> Vec<&Sync> {
        self.syncs
            .iter()
            .filter(|s| self.fixed_since.is_none_or(|t| s.time > t))
            .collect()
    }
    fn spread(&self) -> (usize, f64, f64) {
        let eligible = self.eligible();
        let mut az: Vec<_> = eligible
            .iter()
            .map(|s| s.az_deg.rem_euclid(360.0))
            .collect();
        az.sort_by(f64::total_cmp);
        if az.is_empty() {
            return (0, 0.0, 0.0);
        }
        let gap = az
            .windows(2)
            .map(|w| w[1] - w[0])
            .chain(std::iter::once(az[0] + 360.0 - az[az.len() - 1]))
            .fold(0.0, f64::max);
        let lo = eligible
            .iter()
            .map(|s| s.alt_deg)
            .fold(f64::INFINITY, f64::min);
        let hi = eligible
            .iter()
            .map(|s| s.alt_deg)
            .fold(f64::NEG_INFINITY, f64::max);
        (eligible.len(), 360.0 - gap, hi - lo)
    }
    /// Number of further syncs needed; a coverage deficit always requests more.
    pub fn needs(&self) -> usize {
        if self.fixed.is_some() {
            return 0;
        }
        let (n, az, alt) = self.spread();
        (20_usize.saturating_sub(n)).max(usize::from(az < 180.0 || alt < 30.0))
    }
    /// Add a qualified sync, assign its epoch, thin and refit. Invalid data is refused.
    pub fn add_sync(&mut self, mut sync: Sync) -> bool {
        if !sync.time.is_finite()
            || !sync.az_deg.is_finite()
            || !sync.alt_deg.is_finite()
            || sync.alt_deg.abs() >= 90.0
            || !sync.rms_arcsec.is_finite()
            || sync.rms_arcsec <= 0.0
            || sync.axis.iter().any(|a| !a.is_finite())
            || sync.counts_per_revolution.contains(&0)
            || sync.roll_deg.is_some_and(|r| !r.is_finite())
        {
            return false;
        }
        let slip = self
            .predict(sync.axis, sync.counts_per_revolution)
            .is_some_and(|p| {
                separation(p.az_deg, p.alt_deg, sync.az_deg, sync.alt_deg)
                    > if self.base_tilt.is_some() { 0.5 } else { 2.5 }
            });
        if self.epochs.is_empty() || slip {
            let axis =
                [0, 1].map(|i| sync.axis[i] * 360.0 / f64::from(sync.counts_per_revolution[i]));
            self.epochs.push(Epoch {
                ia: wrap(axis[0] - sync.az_deg),
                ie: sync.alt_deg - axis[1],
            });
        }
        sync.epoch = self.epochs.len() - 1;
        self.syncs.retain(|old| {
            old.epoch != sync.epoch
                || sync.time < old.time
                || sync.time - old.time >= 120.0
                || separation(old.az_deg, old.alt_deg, sync.az_deg, sync.alt_deg) >= 2.0
        });
        self.syncs.push(sync);
        let fit_base = !slip
            && self.syncs.iter().any(|a| {
                self.syncs
                    .iter()
                    .any(|b| a.epoch == b.epoch && wrap(a.az_deg - b.az_deg).abs() >= 90.0)
            });
        let (n, az, alt) = self.spread();
        let fit_fixed = !slip && n >= 20 && az >= 180.0 && alt >= 30.0;
        self.fit(fit_base, fit_fixed, slip);
        true
    }
    /// API summary, omitted for an entirely empty model.
    pub fn view(&self) -> Option<ModelView> {
        if self.syncs.is_empty() && self.fixed.is_none() {
            return None;
        }
        Some(ModelView {
            syncs: self.syncs.len(),
            epoch: self.epochs.len(),
            ia: self.epochs.last().map(|e| e.ia),
            ie: self.epochs.last().map(|e| e.ie),
            base_tilt: self.base_tilt.map(|b| BaseTiltView {
                deg: b.deg(),
                toward_deg: b.toward_deg(),
            }),
            fixed: self.fixed,
            rms_arcsec: self.rms_arcsec,
            needs: self.needs(),
        })
    }
    fn fit(&mut self, fit_base: bool, fit_fixed: bool, slip: bool) {
        // Each epoch has its own pair of zeros. Shared terms are solved with all
        // epochs, so a slip never makes observations from earlier in the night useless.
        let epochs = if slip {
            vec![self.epochs.len() - 1]
        } else {
            (0..self.epochs.len()).collect()
        };
        let mut p: Vec<f64> = epochs
            .iter()
            .flat_map(|&i| [self.epochs[i].ia, self.epochs[i].ie])
            .collect();
        let zeros = p.len();
        let base = self.base_tilt.unwrap_or_default();
        let fixed = self.fixed.unwrap_or_default();
        if fit_base || fit_fixed {
            p.extend([base.an, base.aw]);
        }
        if fit_fixed {
            p.extend([fixed.ca, fixed.npae, fixed.tf, fixed.camera]);
        }
        let evaluate = |p: &[f64], weighted: bool| {
            let base = if fit_base || fit_fixed {
                BaseTilt {
                    an: p[zeros],
                    aw: p[zeros + 1],
                }
            } else {
                base
            };
            let fixed = if fit_fixed {
                FixedTerms {
                    ca: p[zeros + 2],
                    npae: p[zeros + 3],
                    tf: p[zeros + 4],
                    camera: p[zeros + 5],
                }
            } else {
                fixed
            };
            let mut residuals = Vec::new();
            for s in &self.syncs {
                if fit_fixed && self.fixed_since.is_some_and(|time| s.time <= time) {
                    continue;
                }
                let Some(i) = epochs.iter().position(|&epoch| epoch == s.epoch) else {
                    continue;
                };
                let e = Epoch {
                    ia: p[2 * i],
                    ie: p[2 * i + 1],
                };
                let sky = forward(
                    [0, 1].map(|i| s.axis[i] * 360.0 / f64::from(s.counts_per_revolution[i])),
                    e,
                    base,
                    fixed,
                );
                let weight = if weighted { 1.0 / s.rms_arcsec } else { 1.0 };
                residuals.push(wrap(sky.az_deg - s.az_deg) * s.alt_deg.to_radians().cos() * weight);
                residuals.push((sky.alt_deg - s.alt_deg) * weight);
                if fit_fixed {
                    if let Some(roll) = s.roll_deg {
                        residuals.push(wrap(sky.roll_deg - roll) * weight);
                    }
                }
            }
            residuals
        };
        levenberg_marquardt(&mut p, zeros, |p| evaluate(p, true));
        let residual = evaluate(&p, false);
        let rms = (residual.iter().map(|v| v * v).sum::<f64>()
            / (residual.len().max(1) as f64 / 2.0))
            .sqrt()
            * 3600.0;
        for (i, &epoch) in epochs.iter().enumerate() {
            self.epochs[epoch] = Epoch {
                ia: p[2 * i],
                ie: p[2 * i + 1],
            };
        }
        if fit_base || fit_fixed {
            self.base_tilt = Some(BaseTilt {
                an: p[zeros],
                aw: p[zeros + 1],
            });
        }
        if fit_fixed {
            self.fixed = Some(FixedTerms {
                ca: p[zeros + 2],
                npae: p[zeros + 3],
                tf: p[zeros + 4],
                camera: p[zeros + 5],
            });
        }
        self.rms_arcsec = rms;
    }
}

// Damped normal equations, with central finite differences in degrees. At most
// eight physical parameters apply to any sync. Epoch zero blocks are eliminated
// before Cholesky, leaving at most six shared unknowns in the reduced system.
fn levenberg_marquardt(p: &mut Vec<f64>, zeros: usize, residual: impl Fn(&[f64]) -> Vec<f64>) {
    let mut lambda = 1e-3;
    let norm = |r: &[f64]| r.iter().map(|v| v * v).sum::<f64>();
    let mut r = residual(p);
    for _ in 0..100 {
        let n = p.len();
        let jac: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let mut q = p.clone();
                q[i] += 1e-5;
                let plus = residual(&q);
                q[i] -= 2e-5;
                let minus = residual(&q);
                plus.iter()
                    .zip(minus)
                    .map(|(a, b)| (a - b) / 2e-5)
                    .collect()
            })
            .collect();
        let mut a = vec![vec![0.0; n]; n];
        let mut b = vec![0.0; n];
        for i in 0..n {
            b[i] = -jac[i].iter().zip(&r).map(|(j, r)| j * r).sum::<f64>();
            for j in 0..n {
                a[i][j] = jac[i].iter().zip(&jac[j]).map(|(a, b)| a * b).sum();
            }
            a[i][i] += lambda * a[i][i].max(1e-8);
        }
        let Some(step) = epoch_normal_step(a, b, zeros) else {
            lambda *= 10.0;
            continue;
        };
        let next: Vec<_> = p.iter().zip(&step).map(|(p, s)| p + s).collect();
        let next_r = residual(&next);
        if norm(&next_r) < norm(&r) {
            *p = next;
            r = next_r;
            lambda = (lambda / 3.0).max(1e-12);
            if step.iter().all(|s| s.abs() < 1e-9) || norm(&r) < 1e-22 {
                break;
            }
        } else {
            lambda *= 10.0;
            if lambda > 1e15 {
                break;
            }
        }
    }
}
fn cholesky(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = b.len();
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let v = a[i][j] - (0..j).map(|k| l[i][k] * l[j][k]).sum::<f64>();
            if i == j {
                if v <= 0.0 || !v.is_finite() {
                    return None;
                }
                l[i][j] = v.sqrt();
            } else {
                l[i][j] = v / l[j][j];
            }
        }
    }
    let mut x = vec![0.0; n];
    for i in 0..n {
        x[i] = (b[i] - (0..i).map(|k| l[i][k] * x[k]).sum::<f64>()) / l[i][i];
    }
    for i in (0..n).rev() {
        x[i] = (x[i] - (i + 1..n).map(|k| l[k][i] * x[k]).sum::<f64>()) / l[i][i];
    }
    Some(x)
}
fn epoch_normal_step(a: Vec<Vec<f64>>, b: Vec<f64>, zeros: usize) -> Option<Vec<f64>> {
    let n = b.len();
    if n <= 8 {
        return cholesky(&a, &b);
    }
    let shared = n - zeros;
    let mut reduced: Vec<Vec<_>> = a[zeros..].iter().map(|row| row[zeros..].to_vec()).collect();
    let mut rhs = b[zeros..].to_vec();
    let mut blocks = Vec::new();
    for i in (0..zeros).step_by(2) {
        let block = vec![a[i][i..i + 2].to_vec(), a[i + 1][i..i + 2].to_vec()];
        let z = cholesky(&block, &b[i..i + 2])?;
        let columns: Vec<_> = (zeros..n)
            .map(|j| cholesky(&block, &[a[i][j], a[i + 1][j]]))
            .collect::<Option<_>>()?;
        for j in 0..shared {
            rhs[j] -= a[zeros + j][i] * z[0] + a[zeros + j][i + 1] * z[1];
            for k in 0..shared {
                reduced[j][k] -=
                    a[zeros + j][i] * columns[k][0] + a[zeros + j][i + 1] * columns[k][1];
            }
        }
        blocks.push((z, columns));
    }
    let common = cholesky(&reduced, &rhs)?;
    let mut step = vec![0.0; n];
    step[zeros..].copy_from_slice(&common);
    for (i, (z, columns)) in blocks.into_iter().enumerate() {
        for k in 0..2 {
            step[2 * i + k] = z[k] - (0..shared).map(|j| columns[j][k] * common[j]).sum::<f64>();
        }
    }
    Some(step)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Independent synthetic oracle: Rodrigues rotation of north/east/up vectors,
    // followed by the published TPoint small terms. Angles here are degrees.
    fn synthetic(axis: [f64; 2], p: [f64; 8], time: f64) -> Sync {
        let a = (axis[0] - p[0]).to_radians();
        let e = (axis[1] + p[1]).to_radians();
        let t = p[2].hypot(p[3]).to_radians();
        let k = if t == 0.0 {
            [0.0, 1.0, 0.0]
        } else {
            [p[3] / p[2].hypot(p[3]), p[2] / p[2].hypot(p[3]), 0.0]
        };
        let rotate = |v: [f64; 3]| {
            let cross = [k[1] * v[2], -k[0] * v[2], k[0] * v[1] - k[1] * v[0]];
            let dot = k[0] * v[0] + k[1] * v[1];
            [0, 1, 2].map(|i| v[i] * t.cos() + cross[i] * t.sin() + k[i] * dot * (1.0 - t.cos()))
        };
        let v = rotate([e.cos() * a.cos(), e.cos() * a.sin(), e.sin()]);
        let up = rotate([-e.sin() * a.cos(), -e.sin() * a.sin(), e.cos()]);
        let az = v[1].atan2(v[0]);
        let alt = v[2].asin();
        let east = [-az.sin(), az.cos(), 0.0];
        let vertical = [-alt.sin() * az.cos(), -alt.sin() * az.sin(), alt.cos()];
        let dot = |w: [f64; 3]| (0..3).map(|i| up[i] * w[i]).sum::<f64>();
        Sync {
            time,
            axis: axis.map(|d| d * 4500.0),
            counts_per_revolution: [1_620_000; 2],
            az_deg: (az.to_degrees() - p[4] / alt.cos() - p[5] * alt.tan()).rem_euclid(360.0),
            alt_deg: alt.to_degrees() - p[6] * alt.cos(),
            roll_deg: Some(dot(east).atan2(dot(vertical)).to_degrees() + p[7]),
            rms_arcsec: 1.0,
            epoch: 0,
        }
    }
    fn terms() -> [f64; 8] {
        [2.0, -0.5, 0.6, -0.8, 0.0, 0.0, 0.0, 0.0]
    }
    fn spread(p: [f64; 8], count: usize) -> Model {
        let mut model = Model::default();
        for i in 0..count {
            let a = 40.0 + 200.0 * i as f64 / (count - 1) as f64;
            let e = 25.0 + 40.0 * ((i * 7) % count) as f64 / (count - 1) as f64;
            model.add_sync(synthetic([a, e], p, i as f64 * 180.0));
        }
        model
    }
    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() * 3600.0 < 1.0,
            "{actual} != {expected}"
        );
    }
    #[test]
    fn mid_exposure_interpolates_counts_and_limits_both_rates() {
        use scope_link::proto::{AxisPositions, StarFrame};
        let mut frame = StarFrame {
            start_s: 10.0,
            end_s: 11.0,
            axis_start: Some(AxisPositions {
                azimuth: 1000,
                altitude: 20,
            }),
            axis_end: Some(AxisPositions {
                azimuth: 1060,
                altitude: 20,
            }),
            ..Default::default()
        };
        assert_eq!(mid_exposure(&frame, [1_620_000; 2]), Some([1030.0, 20.0]));
        frame.axis_end.as_mut().unwrap().altitude = 20 + 9000;
        assert_eq!(mid_exposure(&frame, [1_620_000; 2]), None);
        frame.end_s = frame.start_s;
        assert_eq!(mid_exposure(&frame, [1_620_000; 2]), None);
    }
    #[test]
    fn two_syncs_recover_zero_points_and_exact_base_rotation() {
        let mut model = Model::default();
        for (i, axis) in [[30.0, 35.0], [130.0, 55.0]].into_iter().enumerate() {
            model.add_sync(synthetic(axis, terms(), i as f64 * 180.0));
        }
        let epoch = model.epochs.last().unwrap();
        let base = model.base_tilt.unwrap();
        for (actual, expected) in [epoch.ia, epoch.ie, base.an, base.aw]
            .into_iter()
            .zip(terms())
        {
            close(actual, expected);
        }
        // Also exercise the Newton inverse away from the fitted directions.
        let sky = synthetic([351.0, 65.0], terms(), 0.0);
        let inverse = model
            .inverse(sky.az_deg, sky.alt_deg, [1_620_000; 2])
            .unwrap();
        close(inverse[0] / 4500.0, 351.0);
        close(inverse[1] / 4500.0, 65.0);
    }
    #[test]
    fn one_sync_opposite_a_one_degree_tilt_misses_by_twice_the_tilt() {
        let mut model = Model::default();
        model.add_sync(synthetic(
            [0.0, 45.0],
            [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            0.0,
        ));
        let truth = synthetic([180.0, 45.0], [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.0);
        let prediction = model
            .predict(truth.axis, truth.counts_per_revolution)
            .unwrap();
        assert!(
            (separation(
                prediction.az_deg,
                prediction.alt_deg,
                truth.az_deg,
                truth.alt_deg
            ) - 2.0)
                .abs()
                < 0.01
        );
    }
    #[test]
    fn slips_refit_only_epoch_zero_points_at_the_half_degree_threshold() {
        let mut p = terms();
        p[4] = 10.0 / 60.0;
        p[5] = 3.0 / 60.0;
        p[6] = 5.0 / 60.0;
        p[7] = 4.0;
        let original = spread(p, 24);
        for (error, epochs) in [(0.4, 1), (0.6, 2)] {
            let mut model = original.clone();
            let base = model.base_tilt;
            let fixed = model.fixed;
            let mut sync = synthetic([180.0, 50.0], p, 5000.0);
            sync.alt_deg += error;
            model.add_sync(sync.clone());
            assert_eq!(model.epochs.len(), epochs);
            if epochs == 2 {
                assert_eq!(model.base_tilt, base);
                assert_eq!(model.fixed, fixed);
                let predicted = model
                    .predict(sync.axis, sync.counts_per_revolution)
                    .unwrap();
                close(predicted.az_deg, sync.az_deg);
                close(predicted.alt_deg, sync.alt_deg);
                assert!((model.epochs[1].ie - model.epochs[0].ie).abs() > 0.5);
            }
        }
    }
    #[test]
    fn twenty_four_spread_syncs_recover_all_fixed_terms_and_camera_rotation() {
        let mut p = terms();
        p[4] = 10.0 / 60.0;
        p[5] = 3.0 / 60.0;
        p[6] = 5.0 / 60.0;
        p[7] = 4.0;
        let model = spread(p, 24);
        let fixed = model.fixed.unwrap();
        for (actual, expected) in [fixed.ca, fixed.npae, fixed.tf, fixed.camera]
            .into_iter()
            .zip(p[4..].iter().copied())
        {
            close(actual, expected);
        }
        assert!(model.rms_arcsec < 1.0);
        assert_eq!(model.needs(), 0);
    }
    #[test]
    fn fixed_terms_need_twenty_and_sky_spread_and_reset_requires_fresh_syncs() {
        let model = spread(terms(), 19);
        assert!(model.fixed.is_none());
        assert_eq!(model.needs(), 1);
        let mut model = spread(terms(), 24);
        assert!(model.fixed.is_some());
        model.reset_fixed();
        assert!(model.fixed.is_none());
        assert_eq!(model.needs(), 20);
        for i in 0..24 {
            model.add_sync(synthetic(
                [40.0 + i as f64 * 9.0, 25.0 + ((i * 7) % 24) as f64 * 2.0],
                terms(),
                6000.0 + i as f64 * 180.0,
            ));
        }
        assert!(model.fixed.is_some());
        let mut narrow = Model::default();
        for i in 0..24 {
            narrow.add_sync(synthetic(
                [359.0 + i as f64 / 24.0, 45.0],
                terms(),
                i as f64 * 180.0,
            ));
        }
        assert!(narrow.fixed.is_none());
        assert!(narrow.needs() > 0);
    }
    #[test]
    fn thinning_keeps_newest_but_preserves_older_and_distant_syncs() {
        let mut model = Model::default();
        for time in [0.0, 1.0, 2.0, 122.0] {
            model.add_sync(synthetic([30.0, 45.0], terms(), time));
        }
        assert_eq!(model.syncs.len(), 2);
        assert_eq!(model.syncs[0].time, 2.0);
        model.add_sync(synthetic([40.0, 45.0], terms(), 123.0));
        assert_eq!(model.syncs.len(), 3);
    }
    #[test]
    fn fixed_calibration_uses_fresh_syncs_after_reset_and_spans_epochs() {
        let mut p = terms();
        p[4] = 10.0 / 60.0;
        p[5] = 3.0 / 60.0;
        p[6] = 5.0 / 60.0;
        p[7] = 4.0;
        let mut model = spread(p, 24);
        model.reset_fixed();
        p[4] = 15.0 / 60.0;
        p[7] = -3.0;
        for i in 0..24 {
            // Three epochs, independently synthesised zero-point slips. Shared
            // hardware calibration must use this night's qualifying fresh syncs.
            let mut current = p;
            current[0] += 3.0 * (i / 8) as f64;
            model.add_sync(synthetic(
                [40.0 + i as f64 * 9.0, 25.0 + ((i * 7) % 24) as f64 * 2.0],
                current,
                6000.0 + i as f64 * 180.0,
            ));
        }
        assert_eq!(model.epochs.len(), 3);
        let fixed = model.fixed.unwrap();
        for (a, b) in [fixed.ca, fixed.npae, fixed.tf, fixed.camera]
            .into_iter()
            .zip(p[4..].iter().copied())
        {
            close(a, b);
        }
    }
    #[test]
    fn invalid_intake_is_refused_and_high_rms_has_less_weight() {
        let mut model = spread(terms(), 24);
        let before = model.clone();
        let mut invalid = synthetic([30.0, 40.0], terms(), 5000.0);
        invalid.rms_arcsec = f64::NAN;
        assert!(!model.add_sync(invalid));
        assert_eq!(model, before);
        assert!(Model::default()
            .inverse(0.0, 45.0, [1_620_000; 2])
            .is_none());
        assert!(model.inverse(f64::NAN, 45.0, [1_620_000; 2]).is_none());
        assert!(model.predict([0.0; 2], [0; 2]).is_none());
        let mut noisy = synthetic([130.0, 40.0], terms(), 5000.0);
        noisy.alt_deg += 0.1;
        noisy.rms_arcsec = 1e6;
        model.add_sync(noisy);
        close(model.epochs[0].ia, terms()[0]);
        close(model.epochs[0].ie, terms()[1]);
        assert!(cholesky(&[vec![-1.0]], &[1.0]).is_none());
    }
}
