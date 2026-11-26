//! 051 US1: independent Gaussian and disc oracles, and the simulator's own sky.
use crate::support::live::{DB, START};
use finder::{finder_focus::measure, imaging::focus::aid::Sample};
use scope_link::proto::Centroid;
use scope_node::{
    detect::{self, DetectSettings},
    Camera, Settings,
};
use simulator::{
    render::{self, RenderStar},
    Optics, SimCamera, Simulator, Site, VirtualClock,
};
use std::time::Duration;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const PLACES: [(f64, f64); 6] = [
    (100.5, 100.5),
    (300.5, 100.5),
    (500.5, 100.5),
    (100.5, 300.5),
    (300.5, 300.5),
    (500.5, 300.5),
];

fn centroid(x: f64, y: f64, brightness: f64) -> Centroid {
    Centroid { x, y, brightness }
}

fn six(phase: f64) -> Vec<Centroid> {
    PLACES
        .iter()
        .map(|&(x, y)| centroid(x + phase, y + phase, 200.0))
        .collect()
}

// Composite Simpson integration over the pixel's unit interval. At sigma >= 0.5,
// 100 steps keep the error well below the specified 1e-6; the omitted tail beyond
// 12 sigma is below 1e-30. This integrates the fixture, independently of measure.
fn integral(pixel: u32, centre: f64, sigma: f64) -> f64 {
    let left = f64::from(pixel) - centre;
    if left > 12.0 * sigma || left + 1.0 < -12.0 * sigma {
        return 0.0;
    }
    let gaussian = |x: f64| (-x * x / (2.0 * sigma * sigma)).exp();
    let mut sum = gaussian(left) + gaussian(left + 1.0);
    for i in 1..100 {
        sum += if i % 2 == 0 { 2.0 } else { 4.0 } * gaussian(left + f64::from(i) / 100.0);
    }
    sum / 300.0
}

fn drawn(centroids: &[Centroid], sigmas: &[f64]) -> Vec<u8> {
    assert_eq!(centroids.len(), sigmas.len());
    let mut light = vec![10.0; (WIDTH * HEIGHT) as usize];
    for (star, &sigma) in centroids.iter().zip(sigmas) {
        let columns: Vec<_> = (0..WIDTH).map(|c| integral(c, star.x, sigma)).collect();
        let rows: Vec<_> = (0..HEIGHT).map(|r| integral(r, star.y, sigma)).collect();
        for (r, &vertical) in rows.iter().enumerate() {
            for (c, &horizontal) in columns.iter().enumerate() {
                light[r * WIDTH as usize + c] += star.brightness * horizontal * vertical;
            }
        }
    }
    light
        .into_iter()
        .map(|p| p.round().clamp(0.0, 255.0) as u8)
        .collect()
}

fn gaussian_diameter(sigma: f64) -> f64 {
    // Half the integral of a circular Gaussian lies inside sqrt(2 ln 2) sigma.
    2.0 * (2.0 * 2.0_f64.ln()).sqrt() * sigma
}

fn reading(sample: Sample, expected_stars: u32) -> f64 {
    match sample {
        Sample::Reading { px, stars } => {
            assert_eq!(stars, expected_stars);
            assert!(px.is_finite() && px > 0.0, "{sample:?}");
            px
        }
        _ => panic!("expected a reading of {expected_stars} stars, got {sample:?}"),
    }
}

fn relative(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance * expected,
        "{actual} versus {expected}, tolerance {tolerance}"
    );
}

/// US1-S1: Gaussian half-flux diameters across phases, the mean of the two
/// middle sizes, and the mean of the two middle box-edge values.
#[test]
fn drawn_gaussians_and_even_medians() {
    for phase in [0.0, 0.25, 0.5] {
        let stars = six(phase);
        for sigma in [1.2, 2.0, 3.0] {
            let pixels = drawn(&stars, &[sigma; 6]);
            let px = reading(measure(&pixels, WIDTH, HEIGHT, &stars), 6);
            relative(px, gaussian_diameter(sigma), 0.08);
        }
    }
    let stars = six(0.0);
    let mixed = drawn(&stars, &[1.2, 1.6, 2.0, 2.4, 2.8, 3.2]);
    let px = reading(measure(&mixed, WIDTH, HEIGHT, &stars), 6);
    relative(
        px,
        (gaussian_diameter(2.0) + gaussian_diameter(2.4)) / 2.0,
        0.03,
    );

    let mut pixels = drawn(&stars, &[3.0; 6]);
    let plain = reading(measure(&pixels, WIDTH, HEIGHT, &stars), 6);
    for r in 0..HEIGHT {
        for c in 0..WIDTH {
            if (c + r) % 2 == 1 {
                pixels[(r * WIDTH + c) as usize] += 2;
            }
        }
    }
    // Each 33-pixel box has 64 edge pixels at 10 and 64 at 12.
    for star in &stars {
        let (x, y) = (star.x.floor() as u32, star.y.floor() as u32);
        let edge: Vec<_> = (y - 16..=y + 16)
            .flat_map(|r| (x - 16..=x + 16).map(move |c| (c, r)))
            .filter(|&(c, r)| c.abs_diff(x) == 16 || r.abs_diff(y) == 16)
            .map(|(c, r)| pixels[(r * WIDTH + c) as usize])
            .collect();
        assert_eq!(edge.iter().filter(|&&p| p == 10).count(), 64);
        assert_eq!(edge.iter().filter(|&&p| p == 12).count(), 64);
    }
    relative(
        reading(measure(&pixels, WIDTH, HEIGHT, &stars), 6),
        plain,
        0.01,
    );
}

/// US1-S2: reject edge, crowded and saturated stars, but ignore disconnected
/// bright pixels without centroids, even at the box's edge.
#[test]
fn only_isolated_unsaturated_stars_own_their_pixels() {
    let mut stars = six(0.0);
    stars.extend([
        centroid(10.5, 240.5, 200.0),
        centroid(624.5, 240.5, 200.0),
        centroid(400.5, 400.5, 200.0),
        centroid(410.5, 405.5, 200.0),
        centroid(300.5, 420.5, 250.0),
    ]);
    let pixels = drawn(&stars, &[2.0; 11]);
    assert_eq!(pixels[(420 * WIDTH + 300) as usize], 255);
    let plain = reading(measure(&pixels, WIDTH, HEIGHT, &stars), 6);
    for (right, below) in [(10, 10), (8, 16)] {
        let mut dirty = pixels.clone();
        for star in &stars[..6] {
            let (x, y) = (star.x.floor() as u32, star.y.floor() as u32);
            dirty[(y * WIDTH + x + right) as usize] = 200;
            dirty[((y + below) * WIDTH + x) as usize] = 255;
        }
        let px = reading(measure(&dirty, WIDTH, HEIGHT, &stars), 6);
        assert!((px - plain).abs() <= 1e-9, "{px} versus {plain}");
    }
}

/// US1-S3: fewer than five is unmeasurable; the half-flux diameter of a
/// uniform disc of radius 15 is 15 sqrt(2), beyond the 20-pixel limit.
#[test]
fn too_few_stars_and_far_out_discs() {
    let stars = six(0.0);
    let pixels = drawn(&stars[..4], &[2.0; 4]);
    assert_eq!(
        measure(&pixels, WIDTH, HEIGHT, &stars[..4]),
        Sample::TooFewStars { stars: 4 }
    );
    let mut discs = vec![10; (WIDTH * HEIGHT) as usize];
    assert_eq!(
        measure(&discs, WIDTH, HEIGHT, &[]),
        Sample::TooFewStars { stars: 0 }
    );
    for r in 0..HEIGHT {
        for c in 0..WIDTH {
            if stars[..5]
                .iter()
                .any(|s| (f64::from(c) + 0.5 - s.x).hypot(f64::from(r) + 0.5 - s.y) <= 15.0)
            {
                discs[(r * WIDTH + c) as usize] = 60;
            }
        }
    }
    match measure(&discs, WIDTH, HEIGHT, &stars[..5]) {
        Sample::FarOut { px, stars } => {
            assert_eq!(stars, 5);
            relative(px, 15.0 * 2.0_f64.sqrt(), 0.05);
        }
        other => panic!("expected far out, got {other:?}"),
    }
}

async fn simulated(camera: &mut SimCamera, exposure_ms: f64) -> Sample {
    camera
        .apply(&Settings {
            exposure_ms: Some(exposure_ms),
            gain: Some(1.0),
            ..Settings::default()
        })
        .unwrap();
    let frame = tokio::time::timeout(Duration::from_secs(120), camera.next_frame())
        .await
        .expect("a simulated exposure within 120 seconds")
        .unwrap();
    let stars = detect::detect(&frame, &DetectSettings::cedar());
    measure(&frame.pixels, frame.width, frame.height, &stars)
}

/// US1-S4: the pre-051 renderer's pinned pixels, and simulated Gaussian
/// H(e) = 2 sqrt(2 ln 2) sqrt(1.2^2 + e^2), measured through cedar detection.
#[tokio::test(start_paused = true)]
async fn simulated_finder_camera_focus_and_original_pixels() {
    let sim = Simulator::new(DB.clone(), Site::default(), VirtualClock::new(START));
    sim.push_to(50.0, 120.0);
    assert_eq!(sim.finder_focus_error(), 0.0);
    let mut camera = SimCamera::new(sim.clone());
    let stars: Vec<_> = sim.field_stars(1000.0, 1.0).iter().map(|s| s.1).collect();
    assert_eq!(stars.len(), 147);
    assert_eq!(camera.frames_taken(), 0);
    let pixels = render::render(&sim.optics(), &stars, 1000.0, 1.0, 13, 0);
    assert_eq!(pixels.len(), 1_584_128);
    // FNV-1a 64 of c75a6e76's own pixels, computed at c75a6e76.
    let hash = pixels.iter().fold(0xcbf29ce484222325_u64, |h, &p| {
        (h ^ u64::from(p)).wrapping_mul(0x100000001b3)
    });
    assert_eq!(hash, 0xb1d308b97a09eb40);
    assert_eq!(
        pixels,
        render::render_with_sigma(
            &sim.optics(),
            &stars,
            1000.0,
            1.0,
            13,
            0,
            render::PSF_SIGMA_PX
        )
    );
    assert_eq!(camera.frames_taken(), 0);

    let mut sizes = Vec::new();
    for error in [0.0_f64, 1.0, 2.0, 3.0] {
        sim.set_finder_focus_error(error);
        assert_eq!(sim.finder_focus_error(), error);
        let px = reading(simulated(&mut camera, 1000.0).await, 15);
        relative(px, gaussian_diameter(1.2_f64.hypot(error)), 0.10);
        if let Some(&previous) = sizes.last() {
            assert!(px > previous, "{px} must exceed {previous}");
        }
        sizes.push(px);
    }
    sim.set_finder_focus_error(-3.0);
    assert_eq!(sim.finder_focus_error(), -3.0);
    relative(
        reading(simulated(&mut camera, 1000.0).await, 15),
        sizes[3],
        0.05,
    );
    sim.set_finder_focus_error(2.0);
    assert_eq!(sim.finder_focus_error(), 2.0);
    relative(
        reading(simulated(&mut camera, 200.0).await, 15),
        sizes[2],
        0.04,
    );
    sim.set_finder_focus_error(8.0);
    assert_eq!(sim.finder_focus_error(), 8.0);
    match simulated(&mut camera, 1000.0).await {
        Sample::TooFewStars { stars } => assert!(stars < 5, "{stars} stars"),
        other => panic!("expected too few stars, got {other:?}"),
    }
    assert!(std::panic::catch_unwind(|| sim.set_finder_focus_error(f64::NAN)).is_err());
}

/// US1-S5: the integral of a star stays constant as its width grows. Average
/// paired star-minus-background sums over 64 frames, with a 5% flux tolerance.
#[test]
fn defocus_conserves_unsaturated_flux() {
    let optics = Optics {
        width: 64,
        height: 64,
        ..Optics::default()
    };
    let stars = [RenderStar {
        x: 32.5,
        y: 32.5,
        magnitude: 8.0,
    }];
    let mut means = Vec::new();
    for sigma in [render::PSF_SIGMA_PX, 3.0] {
        let mut sum = 0_i64;
        for frame in 0..64 {
            let pixels = render::render_with_sigma(&optics, &stars, 200.0, 1.0, 13, frame, sigma);
            let sky = render::render_with_sigma(&optics, &[], 200.0, 1.0, 13, frame, sigma);
            assert!(pixels.iter().all(|&p| p < 250));
            sum += pixels
                .iter()
                .zip(sky)
                .map(|(&p, b)| i64::from(p) - i64::from(b))
                .sum::<i64>();
        }
        means.push(sum as f64 / 64.0);
    }
    assert!(means[0] > 50.0, "{}", means[0]);
    relative(means[1], means[0], 0.05);
}

/// US1-S6: small stars grow with sigma at every phase; phase alone changes
/// sigma 0.5 by at most 1.32 times and sigma 0.8 by at most 1.1 times.
#[test]
fn pixel_sized_stars_grow_without_large_phase_jumps() {
    let mut phases = Vec::new();
    for phase in [0.0, 0.25, 0.5] {
        let stars = six(phase);
        let mut sizes = Vec::new();
        for sigma in [0.5, 0.8, 1.2] {
            let pixels = drawn(&stars, &[sigma; 6]);
            sizes.push(reading(measure(&pixels, WIDTH, HEIGHT, &stars), 6));
        }
        assert!(sizes[1] >= 1.1 * sizes[0], "{sizes:?}");
        assert!(sizes[2] >= 1.1 * sizes[1], "{sizes:?}");
        phases.push(sizes);
    }
    for (index, bound) in [(0, 1.32), (1, 1.1)] {
        let smallest = phases
            .iter()
            .map(|p| p[index])
            .fold(f64::INFINITY, f64::min);
        let largest = phases.iter().map(|p| p[index]).fold(0.0, f64::max);
        assert!(largest <= bound * smallest, "{phases:?}, bound {bound}");
    }
}
