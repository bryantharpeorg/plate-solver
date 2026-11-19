//! 035 US3: exact bilinear worked examples, then the simulator's truth and
//! first sub as independent oracles. Positions are pixel centres throughout.

use finder::imaging::{
    calibration::Mosaic,
    solver::ImagingSolve,
    stack::{colour, Added, Colour, Placement, Stack},
};
use scope_node::imaging::{Exposure, ImagingCamera, ImagingSettings};
use simulator::{ExtendedSource, ImagingTruth, Scene, SimImagingCamera, SimStar, Simulator};

use crate::support::live::sim;

fn ramp() -> Colour {
    let mut picture = Colour::new(8, 6);
    for y in 0_u32..6 {
        for x in 0_u32..8 {
            picture.set(0, x, y, (100 + 2 * x + 3 * y) as f32);
            picture.set(1, x, y, (50 + x) as f32);
            picture.set(2, x, y, 7.0);
        }
    }
    picture
}

fn shifted(t: [f64; 2]) -> Placement {
    Placement {
        t,
        ..Placement::identity()
    }
}

fn counts(covered: u64, rejected: u64) -> Added {
    Added {
        covered,
        added: covered - rejected,
        rejected,
        offsets: [0.0; 3],
    }
}

/// S1: exact integer samples, sums and weights, including every edge.
#[test]
fn identity_once_and_twice_preserves_every_channel() {
    let picture = ramp();
    let mut stack = Stack::new(8, 6);
    assert_eq!((stack.width(), stack.height(), stack.subs()), (8, 6, 0));
    assert_eq!(stack.sum(), &[0.0; 144]);
    assert_eq!(stack.weight(), &[0.0; 48]);
    assert_eq!(stack.mean(), Colour::new(8, 6));
    for n in 1..=2 {
        assert_eq!(stack.add(&picture, &Placement::identity()), counts(48, 0));
        assert_eq!(stack.subs(), n);
        assert_eq!(stack.weight(), &[n as f32; 48]);
        let sums: Vec<_> = picture.planes.iter().map(|v| v * n as f32).collect();
        assert_eq!(stack.sum(), sums);
        assert_eq!(stack.mean(), picture);
    }
}

/// S2: a bilinear rule reproduces linear ramps exactly. Coverage is bounded
/// by sample centres, and the divisor belongs to each pixel.
#[test]
fn shifts_interpolate_the_ramp_and_only_weight_pixels_the_sub_reaches() {
    let picture = ramp();
    for (t, covered) in [([2.0, 1.0], 30), ([0.5, 0.0], 42), ([100.0, 0.0], 0)] {
        let mut stack = Stack::new(8, 6);
        assert_eq!(stack.add(&picture, &shifted(t)), counts(covered, 0));
        assert_eq!(stack.subs(), 1);
        let mean = stack.mean();
        for y in 0_u32..6 {
            for x in 0_u32..8 {
                let reached = f64::from(x) + t[0] <= 7.0 && f64::from(y) + t[1] <= 5.0;
                let index = (y * 8 + x) as usize;
                assert_eq!(stack.weight()[index], if reached { 1.0 } else { 0.0 });
                let expected = if reached {
                    [
                        100.0 + 2.0 * (x as f32 + t[0] as f32) + 3.0 * (y as f32 + t[1] as f32),
                        50.0 + x as f32 + t[0] as f32,
                        7.0,
                    ]
                } else {
                    [0.0; 3]
                };
                for (channel, value) in expected.into_iter().enumerate() {
                    assert_eq!(mean.at(channel, x, y), value, "{t:?}, ({x}, {y})");
                    assert_eq!(stack.sum()[channel * 48 + index], value);
                }
            }
        }
    }
    let mut stack = Stack::new(8, 6);
    stack.add(&picture, &Placement::identity());
    stack.add(&picture, &shifted([2.0, 1.0]));
    assert_eq!(stack.subs(), 2);
    for y in 0_u32..6 {
        for x in 0_u32..8 {
            let reached = x <= 5 && y <= 4;
            let index = (y * 8 + x) as usize;
            assert_eq!(stack.weight()[index], if reached { 2.0 } else { 1.0 });
            assert_eq!(
                stack.mean().at(0, x, y),
                picture.at(0, x, y) + if reached { 3.5 } else { 0.0 }
            );
        }
    }
    assert_eq!(stack.mean().at(0, 0, 0), 103.5);
    assert_eq!(stack.mean().at(0, 7, 5), 129.0);
}

/// S3: exact maps of pixel centres: (7-x, 5-y) and (5-y, x).
#[test]
fn half_and_quarter_turns_use_pixel_centres() {
    let mut stack = Stack::new(8, 6);
    let half = Placement {
        m: [[-1.0, 0.0], [0.0, -1.0]],
        t: [8.0, 6.0],
    };
    assert_eq!(stack.add(&ramp(), &half), counts(48, 0));
    let mean = stack.mean();
    for y in 0_u32..6 {
        for x in 0_u32..8 {
            assert_eq!(mean.at(0, x, y), (129 - 2 * x - 3 * y) as f32);
            assert_eq!(mean.at(1, x, y), (57 - x) as f32);
            assert_eq!(mean.at(2, x, y), 7.0);
        }
    }
    let mut q = Colour::new(6, 8);
    for y in 0_u32..8 {
        for x in 0_u32..6 {
            q.set(0, x, y, (10 * x + y) as f32);
        }
    }
    let mut stack = Stack::new(8, 6);
    let quarter = Placement {
        m: [[0.0, -1.0], [1.0, 0.0]],
        t: [6.0, 0.0],
    };
    assert_eq!(stack.add(&q, &quarter), counts(48, 0));
    let mean = stack.mean();
    for y in 0_u32..6 {
        for x in 0_u32..8 {
            assert_eq!(mean.at(0, x, y), (10 * (5 - y) + x) as f32);
        }
    }
}

#[test]
fn edge_slack_clamps_and_nonfinite_samples_reject_only_when_read() {
    let picture = ramp();
    let mut stack = Stack::new(8, 6);
    assert_eq!(
        stack.add(&picture, &shifted([-0.0000005, 0.0])),
        counts(48, 0)
    );
    assert_eq!(stack.mean(), picture);
    let mut stack = Stack::new(8, 6);
    assert_eq!(stack.add(&picture, &shifted([-0.01, 0.0])), counts(42, 0));

    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut picture = ramp();
        picture.set(2, 3, 3, bad);
        let mut stack = Stack::new(8, 6);
        assert_eq!(stack.add(&picture, &Placement::identity()), counts(48, 1));
        let mean = stack.mean();
        for y in 0_u32..6 {
            for x in 0_u32..8 {
                let rejected = (x, y) == (3, 3);
                assert_eq!(
                    stack.weight()[(y * 8 + x) as usize],
                    if rejected { 0.0 } else { 1.0 }
                );
                for c in 0..3 {
                    assert_eq!(
                        mean.at(c, x, y),
                        if rejected { 0.0 } else { picture.at(c, x, y) }
                    );
                }
            }
        }
        // All four shares are positive here, so the same bad sample rejects
        // exactly the four covered grid pixels whose interpolation reads it.
        let mut stack = Stack::new(8, 6);
        assert_eq!(stack.add(&picture, &shifted([0.5, 0.5])), counts(35, 4));
    }
}

#[test]
fn a_sub_under_two_wide_or_high_covers_nothing() {
    for (width, height) in [(0, 0), (1, 6), (8, 1)] {
        let mut stack = Stack::new(8, 6);
        assert_eq!(
            stack.add(&Colour::new(width, height), &Placement::identity()),
            counts(0, 0)
        );
        assert_eq!(stack.subs(), 1);
        assert_eq!(stack.weight(), &[0.0; 48]);
        assert_eq!(stack.mean(), Colour::new(8, 6));
    }
}

fn truth_solve(truth: ImagingTruth) -> ImagingSolve {
    let (sin, cos) = truth.up_east_of_north_deg.to_radians().sin_cos();
    let k = truth.arcsec_per_px / 3600.0;
    ImagingSolve::from_cd(
        truth.ra_deg,
        truth.dec_deg,
        [[-k * cos, -k * sin], [k * sin, -k * cos]],
        1236,
        822,
    )
}

async fn take(sim: &Simulator, camera: &SimImagingCamera, ms: f64) -> (Colour, ImagingTruth) {
    let frame = camera
        .expose(&Exposure {
            exposure_ms: ms,
            purpose: "sub".into(),
        })
        .await
        .unwrap();
    assert_eq!((frame.width, frame.height, frame.bits), (1236, 822, 16));
    let unix_offset = sim.clock().now() - sim.clock().node_now();
    let truth = sim.imaging_truth_at(unix_offset + (frame.start_s + frame.end_s) / 2.0);
    let picture = tokio::task::spawn_blocking(move || {
        colour(
            &Mosaic::from_samples(
                &frame.samples,
                frame.width,
                frame.height,
                frame.bits,
                frame.bayer.map(|b| b.as_str()),
            )
            .unwrap(),
        )
    })
    .await
    .unwrap();
    (picture, truth)
}

// The shared setup's tangent plane, written here without solver projections.
fn basis(ra: f64, dec: f64) -> [[f64; 3]; 3] {
    let (sa, ca) = ra.to_radians().sin_cos();
    let (sd, cd) = dec.to_radians().sin_cos();
    [
        [cd * ca, cd * sa, sd],
        [-sa, ca, 0.0],
        [-sd * ca, -sd * sa, cd],
    ]
}

fn sky_offset(centre: ImagingTruth, xi_deg: f64, eta_deg: f64) -> (f64, f64) {
    let [b, e, n] = basis(centre.ra_deg, centre.dec_deg);
    let v: [f64; 3] =
        std::array::from_fn(|i| b[i] + xi_deg.to_radians() * e[i] + eta_deg.to_radians() * n[i]);
    (
        v[1].atan2(v[0]).to_degrees().rem_euclid(360.0),
        v[2].atan2(v[0].hypot(v[1])).to_degrees(),
    )
}

fn pixel_by_hand(truth: ImagingTruth, star: SimStar) -> (f64, f64) {
    let [b, e, n] = basis(truth.ra_deg, truth.dec_deg);
    let v = basis(star.ra_deg, star.dec_deg)[0];
    let dot = |a: [f64; 3]| a.into_iter().zip(v).map(|(a, v)| a * v).sum::<f64>();
    let xi = dot(e) / dot(b);
    let eta = dot(n) / dot(b);
    let (sin, cos) = truth.up_east_of_north_deg.to_radians().sin_cos();
    let pixels = 206264.806 / truth.arcsec_per_px;
    (
        618.0 + (-xi * cos + eta * sin) * pixels,
        411.0 + (-xi * sin - eta * cos) * pixels,
    )
}

fn median(values: &[f32]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f32::total_cmp);
    f64::from(sorted[(sorted.len() - 1) / 2])
}

/// Brightest pixel within 6 px of the independently projected position, then
/// the 25 × 25 centroid weighted by value less the plane's median.
fn centroid(picture: &Colour, near: (f64, f64), background: f64) -> ((f64, f64), f64) {
    let mut brightest = None;
    for y in (near.1 - 6.5).ceil() as u32..=(near.1 + 5.5).floor() as u32 {
        for x in (near.0 - 6.5).ceil() as u32..=(near.0 + 5.5).floor() as u32 {
            if (f64::from(x) + 0.5 - near.0).hypot(f64::from(y) + 0.5 - near.1) > 6.0 {
                continue;
            }
            let value = picture.at(1, x, y);
            if brightest.is_none_or(|(_, _, best)| value > best) {
                brightest = Some((x, y, value));
            }
        }
    }
    let (bx, by, peak) = brightest.expect("a pixel within 6 px of the star");
    assert!(bx >= 12 && bx + 12 < picture.width && by >= 12 && by + 12 < picture.height);
    let (mut sum, mut sx, mut sy) = (0.0, 0.0, 0.0);
    for y in by - 12..=by + 12 {
        for x in bx - 12..=bx + 12 {
            let weight = f64::from(picture.at(1, x, y)) - background;
            sum += weight;
            sx += weight * (f64::from(x) + 0.5);
            sy += weight * (f64::from(y) + 0.5);
        }
    }
    assert!(sum > 0.0);
    ((sx / sum, sy / sum), f64::from(peak) - background)
}

/// S4: simulator truth and first sub's rendering are the oracles. The 0.02 px
/// centroid tolerance detects the turn's sign and half-pixel convention errors.
#[tokio::test(start_paused = true)]
async fn three_attitudes_keep_five_stars_on_the_first_subs_grid() {
    let sim = sim();
    sim.set_imaging_optics(simulator::ImagingOptics {
        focus_error: 2.0,
        ..sim.imaging_optics()
    });
    let centre = sim.imaging_truth();
    let stars = [
        (0.0, 0.0),
        (0.15, 0.10),
        (0.15, -0.10),
        (-0.15, 0.10),
        (-0.15, -0.10),
    ]
    .map(|(xi, eta)| {
        let (ra_deg, dec_deg) = sky_offset(centre, xi, eta);
        SimStar {
            ra_deg,
            dec_deg,
            magnitude: 7.0,
            colour: [1.0; 3],
        }
    });
    let camera = SimImagingCamera::new(sim.clone())
        .with_noise(false)
        .with_stars(stars.to_vec());
    camera
        .apply(&ImagingSettings {
            gain: Some(0.0),
            ..Default::default()
        })
        .await
        .unwrap();
    let mut subs = Vec::new();
    for (angle_deg, xi, eta) in [(17.0, 0.0, 0.0), (22.0, 0.02, -0.01), (197.0, 0.0, 0.0)] {
        sim.set_imaging_optics(simulator::ImagingOptics {
            angle_deg,
            ..sim.imaging_optics()
        });
        let pointing = sky_offset(centre, xi, eta);
        sim.point_eyepiece_at(pointing.0, pointing.1);
        subs.push(take(&sim, &camera, 10.0).await);
    }
    tokio::task::spawn_blocking(move || {
        let reference = &subs[0].0;
        let first_truth = subs[0].1;
        let first_solve = truth_solve(first_truth);
        let mut stack = Stack::new(1236, 822);
        for (n, (picture, truth)) in subs.iter().enumerate() {
            let placement = Placement::between(&first_solve, &truth_solve(*truth)).unwrap();
            let expected_turn = (truth.up_east_of_north_deg - first_truth.up_east_of_north_deg
                + 180.0)
                .rem_euclid(360.0)
                - 180.0;
            let error = (placement.turn_deg() - expected_turn + 180.0).rem_euclid(360.0) - 180.0;
            assert!(error.abs() <= 0.02, "sub {n}: turn error {error}");
            if n == 1 {
                assert!((4.9..=5.1).contains(&placement.turn_deg().abs()));
            } else if n == 2 {
                assert!((placement.turn_deg().abs() - 180.0).abs() <= 0.02);
            }
            let added = stack.add(picture, &placement);
            assert_eq!(added.added, added.covered);
            assert_eq!(added.rejected, 0);
            let coverage = added.covered as f64 / 1_015_992.0;
            if n == 1 {
                assert!((0.85..=0.97).contains(&coverage), "coverage {coverage}");
            } else if n == 2 {
                assert!(coverage >= 0.99, "coverage {coverage}");
            }
        }
        assert_eq!(stack.subs(), 3);
        let mean = stack.mean();
        let first_background = median(reference.plane(1));
        let stack_background = median(mean.plane(1));
        for star in stars {
            let near = pixel_by_hand(first_truth, star);
            let (first, first_peak) = centroid(reference, near, first_background);
            let (stacked, peak) = centroid(&mean, near, stack_background);
            assert!(
                (stacked.0 - first.0).abs() <= 0.02,
                "x: {stacked:?} vs {first:?}"
            );
            assert!(
                (stacked.1 - first.1).abs() <= 0.02,
                "y: {stacked:?} vs {first:?}"
            );
            assert!(peak >= 0.85 * first_peak, "peak {peak} vs {first_peak}");
        }
    })
    .await
    .unwrap();
}

/// Mean green signal within 30 px, less the mean in the 150..200 px annulus;
/// noise is that annulus's population standard deviation, in f64.
fn signal_noise(picture: &Colour) -> (f64, f64) {
    let (mut disc, mut annulus) = (Vec::new(), Vec::new());
    for y in 0_u32..picture.height {
        for x in 0_u32..picture.width {
            let radius = (f64::from(x) + 0.5 - 618.0).hypot(f64::from(y) + 0.5 - 411.0);
            let value = f64::from(picture.at(1, x, y));
            if radius <= 30.0 {
                disc.push(value);
            }
            if (150.0..=200.0).contains(&radius) {
                annulus.push(value);
            }
        }
    }
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let background = mean(&annulus);
    let variance = annulus
        .iter()
        .map(|v| (v - background).powi(2))
        .sum::<f64>()
        / annulus.len() as f64;
    (mean(&disc) - background, variance.sqrt())
}

/// S5: declared source gives 0.787 × 3.98 counts over the disc. Four times
/// the subs should double signal/noise: allowed ratio 1.7..2.3.
#[tokio::test(start_paused = true)]
async fn sixteen_faint_glow_subs_double_signal_over_noise_of_four() {
    let sim = sim();
    let centre = sim.imaging_truth();
    sim.set_scene(Scene {
        satellites: vec![],
        sources: vec![ExtendedSource {
            ra_deg: centre.ra_deg,
            dec_deg: centre.dec_deg,
            sigma_arcsec: 30.0 * centre.arcsec_per_px,
            electrons_per_s: 40.0,
            colour: [1.0; 3],
        }],
    });
    let camera = SimImagingCamera::new(sim.clone())
        .with_noise(true)
        .with_stars(vec![]);
    assert_eq!(
        camera
            .apply(&ImagingSettings::default())
            .await
            .unwrap()
            .gain,
        Some(120.0)
    );
    let (mut sixteen, mut four) = (Stack::new(1236, 822), Stack::new(1236, 822));
    let mut reference = None;
    for n in 0..16 {
        sim.point_eyepiece_at(centre.ra_deg, centre.dec_deg);
        let (picture, truth) = take(&sim, &camera, 100.0).await;
        let solve = truth_solve(truth);
        let first_solve = reference.get_or_insert(solve.clone());
        let placement = Placement::between(first_solve, &solve).unwrap();
        (sixteen, four) = tokio::task::spawn_blocking(move || {
            sixteen.add(&picture, &placement);
            if n < 4 {
                four.add(&picture, &placement);
            }
            (sixteen, four)
        })
        .await
        .unwrap();
    }
    tokio::task::spawn_blocking(move || {
        assert_eq!((sixteen.subs(), four.subs()), (16, 4));
        let (s16, n16) = signal_noise(&sixteen.mean());
        let (s4, n4) = signal_noise(&four.mean());
        assert!((2.66..=3.60).contains(&s16), "sixteen-sub signal {s16}");
        assert!(n16 < 0.6 * n4, "noise: sixteen {n16}, four {n4}");
        let ratio = (s16 / n16) / (s4 / n4);
        assert!((1.7..=2.3).contains(&ratio), "signal/noise ratio {ratio}");
    })
    .await
    .unwrap();
}
