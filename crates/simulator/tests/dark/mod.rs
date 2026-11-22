#![allow(dead_code)]
//! The exact seeded skies of spec 048, shared with the exposure story.
use math_core::{angular_distance, UnitVector};
use pattern_database::PatternDatabase;
use plate_solver::{solve_from_centroids, SolveStatus};
use scope_link::proto::Centroid;
use scope_node::{detect::DetectSettings, GrayFrame};
use simulator::{
    render::{RenderStar, PSF_SIGMA_PX, REFERENCE_MAGNITUDE},
    Optics, Simulator, Site, TrueDirection, VirtualClock,
};
use std::{
    f64::consts::TAU,
    path::Path,
    sync::{Arc, LazyLock},
};

pub const WIDTH: u32 = 1296;
pub const HEIGHT: u32 = 972;

static DB: LazyLock<Arc<PatternDatabase>> = LazyLock::new(|| {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    Arc::new(pattern_database::load_from_path(&path).expect("bundled cedar database"))
});

/// Levels at one second and unit gain; black and read do not scale.
#[derive(Clone, Copy)]
pub struct Sky {
    pub sky: f64,
    pub black: f64,
    pub read: f64,
    pub flux: f64,
}

pub const DARK: Sky = Sky {
    sky: 0.05,
    black: 2.0,
    read: 2.0,
    flux: 2.0,
};
pub const CITY: Sky = Sky {
    sky: 30.0,
    black: 0.0,
    read: 3.0,
    flux: 20.0,
};

pub struct Field {
    pub stars: Vec<RenderStar>,
    pub truth: TrueDirection,
}

pub fn field() -> Field {
    let sim = Simulator::new(
        DB.clone(),
        Site::default(),
        VirtualClock::new(1_768_528_800.0),
    );
    sim.set_optics(Optics {
        width: WIDTH,
        height: HEIGHT,
        fov_deg: 13.0,
        eyepiece_px: (648.0, 486.0),
    });
    sim.push_to(50.0, 120.0);
    Field {
        stars: sim
            .field_stars(2000.0, 16.0)
            .into_iter()
            .map(|(_, star)| star)
            .collect(),
        truth: sim.telescope().camera,
    }
}

pub fn peak(flux: f64, star: &RenderStar) -> f64 {
    flux * 10_f64.powf(-0.4 * (star.magnitude - REFERENCE_MAGNITUDE))
}

pub fn drawn(field: &Field, sky: Sky, scale: f64) -> Vec<RenderStar> {
    field
        .stars
        .iter()
        .copied()
        .filter(|star| peak(sky.flux * scale, star) >= 0.5)
        .collect()
}

// SplitMix64, with one Box-Muller cosine per normal. The simulator's production
// generator caches a sine and uses a different uniform; that changes this draw.
struct Random(u64);
impl Random {
    fn uniform(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut word = self.0;
        word = (word ^ (word >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        word = (word ^ (word >> 27)).wrapping_mul(0x94d049bb133111eb);
        word ^= word >> 31;
        ((word >> 11) as f64 + 0.5) / 9_007_199_254_740_992.0
    }

    fn normal(&mut self) -> f64 {
        let u1 = self.uniform();
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos()
    }
}

pub fn render(field: &Field, sky: Sky, scale: f64, seed: u64) -> Vec<u8> {
    let width = WIDTH as usize;
    let height = HEIGHT as usize;
    let mut light = vec![sky.sky * scale; width * height];
    for star in drawn(field, sky, scale) {
        let p = peak(sky.flux * scale, &star);
        for y in
            (star.y.floor() as i64 - 8).max(0)..=(star.y.floor() as i64 + 8).min(height as i64 - 1)
        {
            for x in (star.x.floor() as i64 - 8).max(0)
                ..=(star.x.floor() as i64 + 8).min(width as i64 - 1)
            {
                let r2 = (x as f64 + 0.5 - star.x).powi(2) + (y as f64 + 0.5 - star.y).powi(2);
                light[y as usize * width + x as usize] +=
                    p * (-r2 / (2.0 * PSF_SIGMA_PX.powi(2))).exp();
            }
        }
    }

    let mut random = Random(seed);
    let grid: Vec<_> = (0..(width + 1) * (height + 1))
        .map(|_| random.normal())
        .collect();
    light
        .into_iter()
        .enumerate()
        .map(|(index, light)| {
            let y = index / width;
            let x = index % width;
            let top = y * (width + 1) + x;
            let bottom = top + width + 1;
            let n = (grid[top] + grid[top + 1] + grid[bottom] + grid[bottom + 1]) / 2.0;
            (light - sky.black + sky.read * n).round().clamp(0.0, 255.0) as u8
        })
        .collect()
}

pub fn frame(pixels: Vec<u8>) -> GrayFrame {
    GrayFrame {
        pixels,
        width: WIDTH,
        height: HEIGHT,
        start_s: 0.0,
        end_s: 1.0,
        exposure_ms: 1000.0,
        gain: 1.0,
    }
}

pub fn floor(value: f64) -> DetectSettings {
    DetectSettings {
        noise_floor: Some(value),
        ..DetectSettings::cedar()
    }
}

pub fn true_stars(centroids: &[Centroid], stars: &[RenderStar]) -> usize {
    centroids
        .iter()
        .filter(|centroid| {
            stars
                .iter()
                .any(|star| (centroid.x - star.x).hypot(centroid.y - star.y) <= 1.5)
        })
        .count()
}

/// The console's defaults, with the scope link's (x, y) swapped at the boundary.
pub fn solve(field: &Field, centroids: &[Centroid]) -> Option<f64> {
    let coords: Vec<_> = centroids.iter().take(30).map(|s| (s.y, s.x)).collect();
    let solution = solve_from_centroids(
        &coords,
        (WIDTH as usize, HEIGHT as usize),
        None,
        10.0,
        0.01,
        1e-5,
        5000,
        0.0,
        0.002,
        &**DB,
    );
    if solution.status != Some(SolveStatus::MatchFound) {
        return None;
    }
    let solved = UnitVector::from_radec(solution.ra.unwrap(), solution.dec.unwrap());
    let truth = UnitVector::from_radec(field.truth.ra.to_radians(), field.truth.dec.to_radians());
    Some(angular_distance(solved, truth).to_degrees() * 3600.0)
}
