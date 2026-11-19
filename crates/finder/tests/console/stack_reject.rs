//! 035 US4: hand-worked medians and sample deviations, then a declared
//! simulator streak against a control with the same seeded noise.

use finder::imaging::{
    calibration::Mosaic,
    stack::{colour, levels, Added, Colour, Placement, Stack},
};
use scope_node::imaging::{Exposure, ImagingCamera, ImagingSettings};
use simulator::{SatelliteStreak, Scene, SimImagingCamera};

use crate::support::live::sim;

fn flat(width: u32, height: u32, values: [f32; 3]) -> Colour {
    let mut picture = Colour::new(width, height);
    let size = width as usize * height as usize;
    for (channel, value) in values.into_iter().enumerate() {
        picture.planes[channel * size..(channel + 1) * size].fill(value);
    }
    picture
}

fn counts(answer: Added, covered: u64, added: u64, rejected: u64) {
    assert_eq!(
        answer,
        Added {
            covered,
            added,
            rejected,
            offsets: [0.0; 3],
        }
    );
}

fn close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} vs {expected}, tolerance {tolerance}"
    );
}

/// S1: exact flat planes; ceil(1030 / 512) = 3 on either long side.
/// A full-plane median of the striped pictures would instead be 100.
#[test]
fn levels_samples_every_third_column_and_row_on_either_long_side() {
    assert_eq!(levels(&flat(8, 6, [5.0, 9.0, 2.0])), [5.0, 9.0, 2.0]);
    for (width, height) in [(1030, 4), (4, 1030)] {
        let mut picture = flat(width, height, [100.0, 9.0, 2.0]);
        for y in 0..height {
            for x in 0..width {
                if (width > height && x % 3 == 0) || (height > width && y % 3 == 0) {
                    picture.set(0, x, y, 1.0);
                }
            }
        }
        assert_eq!(levels(&picture), [1.0, 9.0, 2.0]);
    }
    // The shared percentile's lower median, not the average of two middles.
    let mut picture = flat(2, 2, [0.0, 9.0, 2.0]);
    picture.planes[..4].copy_from_slice(&[30.0, 10.0, 40.0, 20.0]);
    assert_eq!(levels(&picture), [20.0, 9.0, 2.0]);
}

/// S2: additive differences from the FIRST sub, independently per channel.
/// All samples and offsets here are exact binary-representable integers.
#[test]
fn background_match_keeps_the_first_subs_levels() {
    let first = flat(4, 4, [100.0, 200.0, 300.0]);
    let second = flat(4, 4, [110.0, 195.0, 300.0]);
    let identity = Placement::identity();
    let mut matched = Stack::new(4, 4).with_background_match();
    counts(matched.add(&first, &identity), 16, 16, 0);
    let answer = matched.add(&second, &identity);
    assert_eq!(answer.offsets, [-10.0, 5.0, 0.0]);
    assert_eq!((answer.covered, answer.added, answer.rejected), (16, 16, 0));
    assert_eq!(matched.mean(), first);
    assert_eq!(matched.weight(), &[2.0; 16]);

    let mut plain = Stack::new(4, 4);
    plain.add(&first, &identity);
    counts(plain.add(&second, &identity), 16, 16, 0);
    assert_eq!(plain.mean(), flat(4, 4, [105.0, 197.5, 300.0]));

    let third = flat(4, 4, [120.0, 190.0, 300.0]);
    assert_eq!(matched.add(&third, &identity).offsets, [-20.0, 10.0, 0.0]);
    assert_eq!(matched.mean(), first);
    assert_eq!(matched.subs(), 3);
}

fn learned(stack: Stack, samples: &[f32], placement: Placement) -> Stack {
    let mut stack = stack;
    for &sample in samples {
        let answer = stack.add(&flat(2, 2, [sample; 3]), &placement);
        assert_eq!(answer.added, answer.covered);
        assert_eq!(answer.rejected, 0);
    }
    stack
}

const ALTERNATING: [f32; 8] = [98.0, 102.0, 98.0, 102.0, 98.0, 102.0, 98.0, 102.0];

/// S3: mean 100, sample variance 32/7, 3 sigma = 6.414... .
/// The ninth sub's 106 is kept, 107 left out; mean tolerance 0.001 count.
#[test]
fn rejection_uses_each_pixels_sample_deviation_and_is_opt_in() {
    let mut ninth = flat(2, 2, [106.0; 3]);
    for channel in 0..3 {
        ninth.set(channel, 0, 0, 107.0);
    }
    for reject in [true, false] {
        let stack = Stack::new(2, 2);
        let mut stack = learned(
            if reject {
                stack.with_rejection()
            } else {
                stack
            },
            &ALTERNATING,
            Placement::identity(),
        );
        counts(
            stack.add(&ninth, &Placement::identity()),
            4,
            if reject { 3 } else { 4 },
            u64::from(reject),
        );
        assert_eq!(stack.subs(), 9);
        assert_eq!(
            stack.weight(),
            &[if reject { 8.0 } else { 9.0 }, 9.0, 9.0, 9.0]
        );
        let mean = stack.mean();
        for channel in 0..3 {
            close(
                mean.at(channel, 0, 0),
                if reject { 100.0 } else { 907.0 / 9.0 },
                0.001,
            );
            for (x, y) in [(1, 0), (0, 1), (1, 1)] {
                close(mean.at(channel, x, y), 100.667, 0.001);
            }
        }
    }
}

/// Dividing S by w instead of w-1 gives a limit of 6, wrongly rejecting 106.3.
#[test]
fn deviation_divides_by_weight_less_one() {
    let mut stack = learned(
        Stack::new(2, 2).with_rejection(),
        &ALTERNATING,
        Placement::identity(),
    );
    counts(
        stack.add(&flat(2, 2, [106.3; 3]), &Placement::identity()),
        4,
        4,
        0,
    );
    assert_eq!(stack.weight(), &[9.0; 4]);
}

/// Eight shifted subs reach only column 0; column 1 has just one sample
/// when the tenth sub arrives and must still accept a value of 200.
#[test]
fn learning_is_counted_per_pixel_instead_of_per_stack() {
    let mut stack = learned(
        Stack::new(2, 2).with_rejection(),
        &[100.0; 8],
        Placement {
            t: [1.0, 0.0],
            ..Placement::identity()
        },
    );
    assert_eq!(stack.weight(), &[8.0, 0.0, 8.0, 0.0]);
    counts(
        stack.add(&flat(2, 2, [100.0; 3]), &Placement::identity()),
        4,
        4,
        0,
    );
    counts(
        stack.add(&flat(2, 2, [200.0; 3]), &Placement::identity()),
        4,
        2,
        2,
    );
    assert_eq!(stack.weight(), &[9.0, 2.0, 9.0, 2.0]);
    assert_eq!(stack.mean(), {
        let mut expected = flat(2, 2, [100.0; 3]);
        for channel in 0..3 {
            expected.set(channel, 1, 0, 150.0);
            expected.set(channel, 1, 1, 150.0);
        }
        expected
    });
}

/// Zero observed variance still permits exactly three counts, but not four.
#[test]
fn deviation_has_a_one_count_floor_and_the_limit_is_strict() {
    for (value, added) in [(103.0, 4), (104.0, 0)] {
        let mut stack = learned(
            Stack::new(2, 2).with_rejection(),
            &[100.0; 8],
            Placement::identity(),
        );
        counts(
            stack.add(&flat(2, 2, [value; 3]), &Placement::identity()),
            4,
            added,
            4 - added,
        );
        assert_eq!(stack.weight(), &[if added == 4 { 9.0 } else { 8.0 }; 4]);
    }
}

/// A blue-only outlier must change neither the weight nor ANY channel's sum.
#[test]
fn a_failure_in_one_channel_leaves_the_whole_pixel_unchanged() {
    let mut stack = learned(
        Stack::new(2, 2).with_rejection(),
        &[100.0; 8],
        Placement::identity(),
    );
    let before = stack.sum().to_vec();
    let mut ninth = flat(2, 2, [100.0; 3]);
    ninth.set(2, 1, 1, 130.0);
    counts(stack.add(&ninth, &Placement::identity()), 4, 3, 1);
    assert_eq!(stack.weight(), &[9.0, 9.0, 9.0, 8.0]);
    for channel in 0..3 {
        for index in 0..4 {
            assert_eq!(
                stack.sum()[channel * 4 + index],
                before[channel * 4 + index] + if index == 3 { 0.0 } else { 100.0 }
            );
        }
    }
    // The rejected value must also leave the deviation unchanged.
    counts(
        stack.add(&flat(2, 2, [104.0; 3]), &Placement::identity()),
        4,
        0,
        4,
    );
    assert_eq!(stack.weight(), &[9.0, 9.0, 9.0, 8.0]);
}

#[test]
fn even_a_wild_eighth_sub_is_still_in_the_learning_set() {
    let mut stack = learned(
        Stack::new(2, 2).with_rejection(),
        &[100.0; 7],
        Placement::identity(),
    );
    counts(
        stack.add(&flat(2, 2, [10000.0; 3]), &Placement::identity()),
        4,
        4,
        0,
    );
    assert_eq!(stack.weight(), &[8.0; 4]);
    assert_eq!(stack.mean(), flat(2, 2, [1337.5; 3]));
}

async fn take(camera: &SimImagingCamera) -> Colour {
    let frame = camera
        .expose(&Exposure {
            exposure_ms: 1000.0,
            purpose: "sub".into(),
        })
        .await
        .unwrap();
    assert_eq!((frame.width, frame.height, frame.bits), (1236, 822, 16));
    tokio::task::spawn_blocking(move || {
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
    .unwrap()
}

/// Mean of green over L, defined independently from the simulator renderer:
/// pixel centres within 1 px of the segment and >=12 px from both endpoints.
fn along_streak(picture: &Colour, streak: &SatelliteStreak) -> f64 {
    let (dx, dy) = (streak.to.0 - streak.from.0, streak.to.1 - streak.from.1);
    let length_squared = dx * dx + dy * dy;
    let (mut sum, mut count) = (0.0, 0_u32);
    for y in 0..picture.height {
        for x in 0..picture.width {
            let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let (ax, ay) = (px - streak.from.0, py - streak.from.1);
            let t = ((ax * dx + ay * dy) / length_squared).clamp(0.0, 1.0);
            let distance = (ax - t * dx).hypot(ay - t * dy);
            if distance <= 1.0
                && ax.hypot(ay) >= 12.0
                && (px - streak.to.0).hypot(py - streak.to.1) >= 12.0
            {
                sum += f64::from(picture.at(1, x, y));
                count += 1;
            }
        }
    }
    assert!(
        count > 1000,
        "the declared segment must reach enough pixels"
    );
    sum / f64::from(count)
}

/// S4 oracle: the simulator's declared streak and a second camera with the
/// same noise seed, reset frame counter and empty scene. Tolerances are counts:
/// plain mean excess >=8; rejection/control difference <=0.5; offsets <=0.5.
#[tokio::test(start_paused = true)]
async fn one_satellite_sub_leaves_no_mark_against_the_noisy_control() {
    let sim = sim();
    let streak = SatelliteStreak {
        frames: vec![9],
        from: (200.0, 300.5),
        to: (1000.0, 500.5),
        electrons: 200.0,
    };
    sim.set_scene(Scene {
        satellites: vec![streak.clone()],
        sources: vec![],
    });
    let camera = SimImagingCamera::new(sim.clone())
        .with_noise(true)
        .with_stars(vec![]);
    camera.apply(&ImagingSettings::default()).await.unwrap();
    let mut x = Stack::new(1236, 822)
        .with_rejection()
        .with_background_match();
    let mut y = Stack::new(1236, 822);
    for n in 0..12 {
        let picture = take(&camera).await;
        (x, y) = tokio::task::spawn_blocking(move || {
            let answer = x.add(&picture, &Placement::identity());
            assert_eq!(answer.covered, 1_015_992);
            assert_eq!(answer.added + answer.rejected, answer.covered);
            if n < 8 {
                assert_eq!(answer.rejected, 0, "learning sub {n}");
            }
            if n == 9 {
                assert!(answer.rejected as f64 <= 0.2 * 1_015_992.0, "{answer:?}");
            }
            assert!(
                answer.offsets.iter().all(|offset| offset.abs() <= 0.5),
                "{answer:?}"
            );
            counts(
                y.add(&picture, &Placement::identity()),
                1_015_992,
                1_015_992,
                0,
            );
            (x, y)
        })
        .await
        .unwrap();
    }
    assert_eq!(camera.frames_taken(), 12);

    sim.set_scene(Scene::default());
    let control_camera = SimImagingCamera::new(sim.clone())
        .with_noise(true)
        .with_stars(vec![]);
    control_camera
        .apply(&ImagingSettings::default())
        .await
        .unwrap();
    let mut c = Stack::new(1236, 822)
        .with_rejection()
        .with_background_match();
    for _ in 0..12 {
        let picture = take(&control_camera).await;
        c = tokio::task::spawn_blocking(move || {
            c.add(&picture, &Placement::identity());
            c
        })
        .await
        .unwrap();
    }
    assert_eq!(control_camera.frames_taken(), 12);
    tokio::task::spawn_blocking(move || {
        assert_eq!((x.subs(), y.subs(), c.subs()), (12, 12, 12));
        let control = along_streak(&c.mean(), &streak);
        let plain = along_streak(&y.mean(), &streak);
        let rejected = along_streak(&x.mean(), &streak);
        assert!(plain - control >= 8.0, "plain {plain}, control {control}");
        assert!(
            (rejected - control).abs() <= 0.5,
            "rejected {rejected}, control {control}"
        );
    })
    .await
    .unwrap();
}
