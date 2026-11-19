//! The focus aid's arithmetic (033 US4): the median of the last three
//! readings since the last change, the change of more than 15% and the two
//! frames set aside after it, the trend against the reading that stood before,
//! the best of the last kept settled readings, and `found` as far, near, far,
//! near against the best of now. Oracles: the worked examples of plan.md § The
//! aid's arithmetic — its table is the first test's answer step by step, the
//! 39 samples through focus and back, the slow turn of the 5% rule, the
//! unmeasurable frames, the kept best and the history's length, and the three
//! sequences of `found` (far from the best 2.8 is 3.36 or more and near is
//! 3.08 or less). Every size is compared to within 1e-9.

use finder::imaging::focus::aid::{self, Aid, Phase, Sample, Trend};

/// One measurable sample (the shared setup): a reading of `px` among stars.
fn reading(px: f64) -> Sample {
    Sample::Reading { px, stars: 12 }
}

/// Every size agrees with its worked example to within 1e-9 (US4-S1).
fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1e-9
}

/// An optional size agrees with its worked example, or both say nothing.
fn assert_px(actual: Option<f64>, expected: Option<f64>, what: &str) {
    match (actual, expected) {
        (Some(actual), Some(expected)) => {
            assert!(
                close(actual, expected),
                "{what}: {actual} is not {expected}"
            );
        }
        (None, None) => {}
        (actual, expected) => panic!("{what}: {actual:?} is not {expected:?}"),
    }
}

/// The 39 samples through focus and back (US4-S1): the six readings of each
/// side of the pass, the change at each turn of the focuser, and the last
/// group that comes back near the best. The plan's table answers every step.
#[test]
fn through_focus_and_back() {
    let samples: [f64; 39] = [
        6.0, 6.1, 5.9, 6.0, // 1 to 4: the far side, settled at 3 and 4.
        4.5, 4.5, 4.4, 4.6, 4.5, // 5 to 9: a change at 5, settled at 9.
        3.2, 3.2, 3.3, 3.1, 3.2, // 10 to 14: a change at 10, settled at 14.
        2.8, 2.8, 2.9, 2.7, 2.8, // 15 to 19: no change, 12.5% from 3.2.
        3.3, 3.3, 3.2, 3.4, 3.3, // 20 to 24: a change at 20, settled at 24.
        4.0, 4.0, 4.1, 3.9, 4.0, // 25 to 29: a change at 25, settled at 29.
        3.3, 3.3, 3.3, 3.3, 3.3, // 30 to 34: a change at 30, settled at 34.
        2.85, 2.85, 2.8, 2.9, 2.85, // 35 to 39: 13.6% from 3.3, near the best.
    ];
    // The plan's table, one column a sample, oldest first: the phase, the
    // median of the readings held (its size and how many readings it is
    // from), the settled reading `push` answered, the trend, the best and
    // `found`.
    let phases: [Phase; 39] = [
        Phase::Reading,  // 1
        Phase::Reading,  // 2
        Phase::Reading,  // 3
        Phase::Reading,  // 4
        Phase::Settling, // 5
        Phase::Settling, // 6
        Phase::Reading,  // 7
        Phase::Reading,  // 8
        Phase::Reading,  // 9
        Phase::Settling, // 10
        Phase::Settling, // 11
        Phase::Reading,  // 12
        Phase::Reading,  // 13
        Phase::Reading,  // 14
        Phase::Reading,  // 15
        Phase::Reading,  // 16
        Phase::Reading,  // 17
        Phase::Reading,  // 18
        Phase::Reading,  // 19
        Phase::Settling, // 20
        Phase::Settling, // 21
        Phase::Reading,  // 22
        Phase::Reading,  // 23
        Phase::Reading,  // 24
        Phase::Settling, // 25
        Phase::Settling, // 26
        Phase::Reading,  // 27
        Phase::Reading,  // 28
        Phase::Reading,  // 29
        Phase::Settling, // 30
        Phase::Settling, // 31
        Phase::Reading,  // 32
        Phase::Reading,  // 33
        Phase::Reading,  // 34
        Phase::Reading,  // 35
        Phase::Reading,  // 36
        Phase::Reading,  // 37
        Phase::Reading,  // 38
        Phase::Reading,  // 39
    ];
    let medians: [Option<(f64, usize)>; 39] = [
        Some((6.0, 1)),  // 1
        Some((6.05, 2)), // 2
        Some((6.0, 3)),  // 3
        Some((6.0, 3)),  // 4
        None,            // 5
        None,            // 6
        Some((4.4, 1)),  // 7
        Some((4.5, 2)),  // 8
        Some((4.5, 3)),  // 9
        None,            // 10
        None,            // 11
        Some((3.3, 1)),  // 12
        Some((3.2, 2)),  // 13
        Some((3.2, 3)),  // 14
        Some((3.1, 3)),  // 15
        Some((2.8, 3)),  // 16
        Some((2.8, 3)),  // 17
        Some((2.8, 3)),  // 18
        Some((2.8, 3)),  // 19
        None,            // 20
        None,            // 21
        Some((3.2, 1)),  // 22
        Some((3.3, 2)),  // 23
        Some((3.3, 3)),  // 24
        None,            // 25
        None,            // 26
        Some((4.1, 1)),  // 27
        Some((4.0, 2)),  // 28
        Some((4.0, 3)),  // 29
        None,            // 30
        None,            // 31
        Some((3.3, 1)),  // 32
        Some((3.3, 2)),  // 33
        Some((3.3, 3)),  // 34
        Some((3.3, 3)),  // 35
        Some((2.85, 3)), // 36
        Some((2.85, 3)), // 37
        Some((2.85, 3)), // 38
        Some((2.85, 3)), // 39
    ];
    let settled: [Option<f64>; 39] = [
        None,       // 1
        None,       // 2
        Some(6.0),  // 3
        Some(6.0),  // 4
        None,       // 5
        None,       // 6
        None,       // 7
        None,       // 8
        Some(4.5),  // 9
        None,       // 10
        None,       // 11
        None,       // 12
        None,       // 13
        Some(3.2),  // 14
        Some(3.1),  // 15
        Some(2.8),  // 16
        Some(2.8),  // 17
        Some(2.8),  // 18
        Some(2.8),  // 19
        None,       // 20
        None,       // 21
        None,       // 22
        None,       // 23
        Some(3.3),  // 24
        None,       // 25
        None,       // 26
        None,       // 27
        None,       // 28
        Some(4.0),  // 29
        None,       // 30
        None,       // 31
        None,       // 32
        None,       // 33
        Some(3.3),  // 34
        Some(3.3),  // 35
        Some(2.85), // 36
        Some(2.85), // 37
        Some(2.85), // 38
        Some(2.85), // 39
    ];
    let trends: [Option<Trend>; 39] = [
        None,                // 1
        None,                // 2
        None,                // 3
        None,                // 4
        None,                // 5
        None,                // 6
        None,                // 7
        None,                // 8
        Some(Trend::Better), // 9
        Some(Trend::Better), // 10
        Some(Trend::Better), // 11
        Some(Trend::Better), // 12
        Some(Trend::Better), // 13
        Some(Trend::Better), // 14
        Some(Trend::Better), // 15
        Some(Trend::Better), // 16
        Some(Trend::Better), // 17
        Some(Trend::Better), // 18
        Some(Trend::Better), // 19
        Some(Trend::Better), // 20
        Some(Trend::Better), // 21
        Some(Trend::Better), // 22
        Some(Trend::Better), // 23
        Some(Trend::Worse),  // 24
        Some(Trend::Worse),  // 25
        Some(Trend::Worse),  // 26
        Some(Trend::Worse),  // 27
        Some(Trend::Worse),  // 28
        Some(Trend::Worse),  // 29
        Some(Trend::Worse),  // 30
        Some(Trend::Worse),  // 31
        Some(Trend::Worse),  // 32
        Some(Trend::Worse),  // 33
        Some(Trend::Better), // 34
        Some(Trend::Better), // 35
        Some(Trend::Better), // 36
        Some(Trend::Better), // 37
        Some(Trend::Better), // 38
        Some(Trend::Better), // 39
    ];
    let bests: [Option<f64>; 39] = [
        None,      // 1
        None,      // 2
        Some(6.0), // 3
        Some(6.0), // 4
        Some(6.0), // 5
        Some(6.0), // 6
        Some(6.0), // 7
        Some(6.0), // 8
        Some(4.5), // 9
        Some(4.5), // 10
        Some(4.5), // 11
        Some(4.5), // 12
        Some(4.5), // 13
        Some(3.2), // 14
        Some(3.1), // 15
        Some(2.8), // 16
        Some(2.8), // 17
        Some(2.8), // 18
        Some(2.8), // 19
        Some(2.8), // 20
        Some(2.8), // 21
        Some(2.8), // 22
        Some(2.8), // 23
        Some(2.8), // 24
        Some(2.8), // 25
        Some(2.8), // 26
        Some(2.8), // 27
        Some(2.8), // 28
        Some(2.8), // 29
        Some(2.8), // 30
        Some(2.8), // 31
        Some(2.8), // 32
        Some(2.8), // 33
        Some(2.8), // 34
        Some(2.8), // 35
        Some(2.8), // 36
        Some(2.8), // 37
        Some(2.8), // 38
        Some(2.8), // 39
    ];
    let founds: [bool; 39] = [
        false, // 1
        false, // 2
        false, // 3
        false, // 4
        false, // 5
        false, // 6
        false, // 7
        false, // 8
        false, // 9
        false, // 10
        false, // 11
        false, // 12
        false, // 13
        false, // 14
        false, // 15
        false, // 16
        false, // 17
        false, // 18
        false, // 19
        false, // 20
        false, // 21
        false, // 22
        false, // 23
        false, // 24
        false, // 25
        false, // 26
        false, // 27
        false, // 28
        false, // 29
        false, // 30
        false, // 31
        false, // 32
        false, // 33
        false, // 34
        false, // 35
        true,  // 36
        true,  // 37
        true,  // 38
        true,  // 39
    ];
    let mut aid = Aid::new();
    let mut settled_after: Vec<usize> = Vec::new();
    for n in 0..39 {
        let sample = n + 1;
        let step = aid.push(reading(samples[n]));
        assert_eq!(
            aid.phase(),
            Some(phases[n]),
            "the phase after sample {sample}"
        );
        if step.settled.is_some() {
            settled_after.push(sample);
        }
        match (aid.median().map(|m| (m.px, m.readings)), medians[n]) {
            (Some((actual, readings)), Some((expected, expected_readings))) => {
                assert_eq!(
                    readings, expected_readings,
                    "the median's readings after sample {sample}"
                );
                assert!(
                    close(actual, expected),
                    "the median after sample {sample}: {actual} is not {expected}"
                );
            }
            (None, None) => {}
            (actual, expected) => {
                panic!("the median after sample {sample}: {actual:?} is not {expected:?}")
            }
        }
        assert_px(
            step.settled,
            settled[n],
            &format!("the settled reading after sample {sample}"),
        );
        assert_eq!(aid.trend(), trends[n], "the trend after sample {sample}");
        assert_px(
            aid.best(),
            bests[n],
            &format!("the best after sample {sample}"),
        );
        assert_eq!(aid.found(), founds[n], "found after sample {sample}");
        assert_eq!(
            aid.readings(),
            sample as u64,
            "readings after sample {sample}"
        );
    }
    // `push` answered a settled reading after exactly these samples, and never
    // between them.
    assert_eq!(
        settled_after,
        [3, 4, 9, 14, 15, 16, 17, 18, 19, 24, 29, 34, 35, 36, 37, 38, 39]
    );
    // Far from the best 2.8 is 3.36 or more and near is 3.08 or less, the
    // worked example behind `found`: the settled readings run far (6.0, 4.5),
    // near (2.8), far (4.0), near (2.85).
    assert!(close(2.8 * (1.0 + aid::NEAR_FRACTION), 3.08));
    assert!(close(2.8 * (1.0 + aid::RISE_FRACTION), 3.36));
}

/// A slow turn with no change (US4-S2): a worked example of the 5% rule, where
/// the settled reading moves the trend once it is more than 5% from its level
/// — the level 6.0 stands until the settled 5.6, then 5.6 until the settled
/// 5.3, and the trend answers against the reading that stood before.
#[test]
fn a_slow_turn_moves_the_trend_at_5_percent_from_its_level() {
    let samples: [f64; 18] = [
        6.0, 6.0, 6.0, 5.9, 5.8, 5.6, 5.6, 5.6, 5.5, 5.4, 5.3, 5.3, 5.3, 5.4, 5.5, 5.6, 5.6, 5.6,
    ];
    let trends: [Option<Trend>; 18] = [
        // No settled reading has moved more than 5% from the level 6.0 yet.
        None,
        None,
        None,
        None,
        None,
        None,
        // 5.6 against 6.0.
        Some(Trend::Better),
        Some(Trend::Better),
        Some(Trend::Better),
        Some(Trend::Better),
        Some(Trend::Better),
        // 5.3 against the level 5.6.
        Some(Trend::Better),
        Some(Trend::Better),
        Some(Trend::Better),
        // 5.4 and 5.5 against 5.6.
        Some(Trend::Same),
        Some(Trend::Same),
        // 5.6 against the level 5.3.
        Some(Trend::Worse),
        Some(Trend::Worse),
    ];
    let mut aid = Aid::new();
    for (n, (&px, &trend)) in samples.iter().zip(trends.iter()).enumerate() {
        let n = n + 1;
        aid.push(reading(px));
        assert_eq!(
            aid.phase(),
            Some(Phase::Reading),
            "no sample of a slow turn is a change, after sample {n}"
        );
        assert_eq!(aid.trend(), trend, "the trend after sample {n}");
    }
    assert_px(aid.best(), Some(5.3), "the best at the end");
}

/// A frame with too few stars and a far-out frame (US4-S3): both drop the
/// readings held and set nothing aside, the far-out frame keeps its size for
/// the history and the frame with too few stars keeps nothing, both are
/// counted in `readings`, and the best is left alone.
#[test]
fn an_unmeasurable_frame_drops_the_readings_held() {
    let mut aid = Aid::new();
    for (n, expected) in [(1, None), (2, None), (3, Some(3.0))] {
        aid.push(reading(3.0));
        assert_px(aid.best(), expected, &format!("the best after sample {n}"));
    }
    // A sample of 2 stars.
    aid.push(Sample::TooFewStars { stars: 2 });
    assert_eq!(aid.phase(), Some(Phase::TooFewStars), "the phase after 4");
    assert_eq!(aid.median(), None, "no median after 4");
    assert_px(aid.best(), Some(3.0), "the best after 4");
    aid.push(reading(3.1));
    let median = aid.median().expect("a median after 5");
    assert_eq!(median.readings, 1, "one reading held after 5");
    assert!(
        close(median.px, 3.1),
        "the median after 5: {} is not 3.1",
        median.px
    );
    aid.push(reading(3.1));
    assert_eq!(aid.trend(), None, "no trend after 6");
    aid.push(reading(3.1));
    assert_eq!(aid.trend(), Some(Trend::Same), "3.1 against 3.0 after 7");
    assert_px(aid.best(), Some(3.0), "the best after 7");
    // A far-out sample.
    aid.push(Sample::FarOut { px: 41.0, stars: 9 });
    assert_eq!(aid.phase(), Some(Phase::FarOut), "the phase after 8");
    assert_eq!(aid.median(), None, "no median after 8");
    assert_px(aid.best(), Some(3.0), "the best after 8");
    for _ in 9..=11 {
        aid.push(reading(3.6));
    }
    assert_eq!(aid.trend(), Some(Trend::Worse), "3.6 against 3.1 after 11");
    assert_px(aid.best(), Some(3.0), "the best at the end");
    assert_eq!(aid.readings(), 11, "every sample is counted");
    let history: [Option<f64>; 11] = [
        Some(3.0),
        Some(3.0),
        Some(3.0),
        None,
        Some(3.1),
        Some(3.1),
        Some(3.1),
        Some(41.0),
        Some(3.6),
        Some(3.6),
        Some(3.6),
    ];
    let kept = aid.history();
    assert_eq!(kept.len(), 11, "the history holds every sample");
    for (n, (actual, expected)) in kept.iter().zip(history.iter()).enumerate() {
        assert_px(
            *actual,
            *expected,
            &format!("the history's sample {}", n + 1),
        );
    }
}

/// The best over the settled readings kept (US4-S4): with four kept, the
/// settled readings are those after samples 3, 4, 9, 14, 15, 16 and 17, so the
/// 3.0 of sample 9 stays the best until the 5.0 of sample 17 pushes it out.
/// And the history holds the last 120 samples of 130, while `readings` counts
/// them all.
#[test]
fn the_best_is_the_smallest_of_the_last_kept_settled_readings() {
    let samples: [f64; 17] = [
        5.0, 5.0, 5.0, 5.0, // 1 to 4.
        3.0, 3.0, 3.0, 3.0, 3.0, // 5 to 9.
        5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, // 10 to 17.
    ];
    let mut aid = Aid::new().with_best_kept(4);
    let mut settled_after: Vec<usize> = Vec::new();
    let mut best: Vec<Option<f64>> = Vec::new();
    for (n, &px) in samples.iter().enumerate() {
        if aid.push(reading(px)).settled.is_some() {
            settled_after.push(n + 1);
        }
        best.push(aid.best());
    }
    assert_eq!(
        settled_after,
        [3, 4, 9, 14, 15, 16, 17],
        "the settled readings"
    );
    assert_px(best[8], Some(3.0), "the best after 9");
    assert_px(best[15], Some(3.0), "the best after 16");
    assert_px(
        best[16],
        Some(5.0),
        "the best after 17, the last four settled readings all 5.0",
    );
    let mut aid = Aid::new();
    for _ in 0..130 {
        aid.push(reading(3.0));
    }
    assert_eq!(aid.readings(), 130, "readings counts every sample pushed");
    let history = aid.history();
    assert_eq!(history.len(), 120, "the history keeps the last 120 samples");
    assert!(
        history
            .iter()
            .all(|&px| px.is_some_and(|px| close(px, 3.0))),
        "the history holds the readings"
    );
}

/// The three sequences of `found` (US4-S5): a reading that only fell has not
/// been shown to be the lowest, one that rose and did not come back is not
/// back at the best, and far, near, far, near ends found — on a best slightly
/// lower than the earlier pass, which 2.8 is still within 10% of.
#[test]
fn found_needs_far_near_far_near_against_the_best() {
    // The reading only fell: 6.0 four times, 4.5 five times, 2.8 five times.
    let mut aid = Aid::new();
    for &(px, times) in &[(6.0, 4usize), (4.5, 5), (2.8, 5)] {
        for _ in 0..times {
            aid.push(reading(px));
        }
    }
    assert!(
        !aid.found(),
        "a reading that only fell has not been shown to be the lowest"
    );
    // It rose and did not come back: the same followed by 4.0 five times.
    for _ in 0..5 {
        aid.push(reading(4.0));
    }
    assert!(!aid.found(), "it rose and did not come back");
    // Far, near, far, near: 6.0 four times, 2.8 five times, 4.0 five times,
    // 2.7 five times.
    let mut aid = Aid::new();
    for &(px, times) in &[(6.0, 4usize), (2.8, 5), (4.0, 5), (2.7, 5)] {
        for _ in 0..times {
            aid.push(reading(px));
        }
    }
    assert!(
        aid.found(),
        "2.8 is within 10% of the new best 2.7, so the earlier pass counts"
    );
    assert_px(aid.best(), Some(2.7), "the best of the third sequence");
}
