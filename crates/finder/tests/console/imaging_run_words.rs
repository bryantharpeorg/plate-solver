//! US12: the run's public words and shapes, before a run exists. Oracles are
//! the literal lists, sentences and worked examples of US12-S1 and US12-S2;
//! comparisons are exact (no numerical tolerance or derived expected messages).

use std::{collections::BTreeSet, time::Duration};

use finder::imaging::run::{
    label, run_id, By, Ending, Failure, RunRequest, RunSettings, RunTarget, Unit, Verdict,
    DEFAULT_DURATION_S, ENDINGS, MAX_DURATION_S, MAX_SUBS, POLL, STATES,
};

#[tokio::test(start_paused = true)]
async fn every_ending_has_its_reason_and_literal_sentence() {
    assert_eq!(
        STATES,
        [
            "slewing",
            "tracking",
            "first_frame",
            "focus_check",
            "focus_pause",
            "solving",
            "centring",
            "stacking",
            "paused_by_tracker",
            "ended",
        ]
    );
    assert_eq!(
        ENDINGS,
        [
            "stopped",
            "count_limit",
            "duration_limit",
            "too_low",
            "sun_up",
            "link_lost",
            "mount_failed",
            "target_changed",
            "pushed_away",
            "camera_failed",
            "solver_unavailable",
            "solver_failing",
            "save_failed",
            "focus_timeout",
            "not_centred",
            "subs_failing",
            "stack_failed",
        ]
    );
    let settings = RunSettings {
        subs: Some(3),
        duration_s: 7200.0,
        keep_subs: true,
    };
    let cases = [
        (Ending::Stopped, "stopped", "Stopped: the mount was stopped."),
        (Ending::RunStopped, "stopped", "Imaging run stopped. Track is still on."),
        (Ending::StackStopped, "stopped", "Ended: the stack was stopped."),
        (Ending::CheckStopped, "stopped", "Ended: the focus check was stopped."),
        (Ending::CountLimit, "count_limit", "Finished: the 3 subs asked for are stacked."),
        (Ending::DurationLimit, "duration_limit", "Finished: the 120-minute limit was reached."),
        (Ending::TooLow, "too_low", "Ended: M42 is below the minimum altitude."),
        (Ending::SunUp, "sun_up", "Ended: the Sun is up."),
        (Ending::LinkLost, "link_lost", "Ended: the mount or the scope link stopped answering. Check the scope node and the mount's cable, then start again."),
        (Ending::MountFailed("failed after 5 corrections".into()), "mount_failed", "Ended: Track ended and the run with it: failed after 5 corrections. Check the mount, then start again."),
        (Ending::TargetChanged, "target_changed", "Ended: the target was changed. Start a new imaging run for the new target."),
        (Ending::PushedAway, "pushed_away", "Ended: the telescope was pushed away from M42. Track follows where it is left. Start a new imaging run to go back to M42."),
        (Ending::CameraFailed("boom".into()), "camera_failed", "Ended: the imaging camera failed: boom."),
        (Ending::SolverUnavailable("no star database".into()), "solver_unavailable", "Ended: the imaging solver cannot run: no star database."),
        (Ending::SolverFailing("no solution".into()), "solver_failing", "Ended: 3 imaging frames in a row could not be solved: no solution. Check for cloud, dew and focus."),
        (Ending::SaveFailed("disk".into()), "save_failed", "Ended: the console could not save to its imaging folder: disk. Check the disk."),
        (Ending::FocusTimeout, "focus_timeout", "Ended: focus was left paused for 15 minutes. Start again when ready."),
        (Ending::NotCentred { offset_arcsec: 400.0, measured: true }, "not_centred", "Ended: M42 is still 400″ from the sensor's centre after 3 tries. Check the aim point on the console page, then start again."),
        (Ending::NotCentred { offset_arcsec: 400.0, measured: false }, "not_centred", "Ended: M42 is 400″ from the sensor's centre and no new aim point could be measured."),
        (Ending::SubsFailing("not solved 7, too few stars 3".into()), "subs_failing", "Ended: no sub was stacked for 10 minutes (not solved 7, too few stars 3). Cloud, dew or a tree is the usual cause; check the sky and start again."),
        (Ending::StackFailed("the stack is busy".into()), "stack_failed", "Ended: the stack stopped: the stack is busy."),
    ];
    let mut reasons = BTreeSet::new();
    for (ending, reason, sentence) in cases {
        assert_eq!(ending.reason(), reason, "{ending:?}");
        assert_eq!(ending.message("M42", &settings), sentence, "{ending:?}");
        assert!(ENDINGS.contains(&ending.reason()));
        reasons.insert(ending.reason());
    }
    assert_eq!(reasons, ENDINGS.into_iter().collect());
}

#[tokio::test(start_paused = true)]
async fn counts_minutes_and_offsets_follow_the_written_rules() {
    let mut settings = RunSettings {
        subs: Some(1),
        duration_s: 5400.0,
        keep_subs: false,
    };
    assert_eq!(
        Ending::CountLimit.message("M42", &settings),
        "Finished: the 1 sub asked for is stacked."
    );
    for (duration_s, sentence) in [
        (5400.0, "Finished: the 90-minute limit was reached."),
        (90.0, "Finished: the 1.5-minute limit was reached."),
        (1.0, "Finished: the 0.1-minute limit was reached."),
    ] {
        settings.duration_s = duration_s;
        assert_eq!(Ending::DurationLimit.message("M42", &settings), sentence);
    }
    for (offset_arcsec, sentence) in [
        (599.0, "Ended: M42 is still 599″ from the sensor's centre after 3 tries. Check the aim point on the console page, then start again."),
        (600.0, "Ended: M42 is still 10.0′ from the sensor's centre after 3 tries. Check the aim point on the console page, then start again."),
        (3792.0, "Ended: M42 is still 63.2′ from the sensor's centre after 3 tries. Check the aim point on the console page, then start again."),
    ] {
        let ending = Ending::NotCentred { offset_arcsec, measured: true };
        assert_eq!(ending.message("M42", &settings), sentence);
    }
}

#[tokio::test(start_paused = true)]
async fn parts_answer_their_public_words() {
    assert_eq!(By::Page.as_str(), "page");
    assert_eq!(By::Mcp.as_str(), "mcp");
    for (verdict, word) in [
        (Verdict::Good, "good"),
        (Verdict::SlightlySoft, "slightly_soft"),
        (Verdict::ClearlyOut, "clearly_out"),
        (Verdict::Unmeasurable, "unmeasurable"),
        (Verdict::NoRecord, "no_record"),
    ] {
        assert_eq!(verdict.as_str(), word);
    }
    assert_eq!(Unit::Arcsec.as_str(), "arcsec");
    assert_eq!(Unit::Px.as_str(), "px");
    assert_eq!(Failure::Stopped.words(), "stopped");
    for failure in [
        Failure::Camera("boom".into()),
        Failure::Save("boom".into()),
        Failure::SolverUnavailable("boom".into()),
        Failure::NotSolved("boom".into()),
        Failure::Mount("boom".into()),
        Failure::Other("boom".into()),
    ] {
        assert_eq!(failure.words(), "boom");
    }
}

#[tokio::test(start_paused = true)]
async fn names_and_ids_match_the_worked_utc_example() {
    assert_eq!(label("M42"), "M42");
    assert_eq!(label("NGC 1976"), "NGC-1976");
    assert_eq!(run_id(1768528800.0, "M42"), "2026-01-16T02-00-00Z-M42");
    assert_eq!(
        run_id(1768528800.0, "NGC 1976"),
        "2026-01-16T02-00-00Z-NGC-1976"
    );
}

#[tokio::test(start_paused = true)]
async fn request_settings_target_and_constants_have_the_declared_shapes() {
    let empty = serde_json::from_str::<RunRequest>("{}").unwrap();
    assert_eq!(empty, RunRequest::default());
    assert_eq!(empty.subs, None);
    assert_eq!(empty.duration_s, None);
    assert_eq!(empty.keep_subs, None);
    assert_eq!(
        serde_json::from_str::<RunRequest>(r#"{"subs":3,"duration_s":60.0,"keep_subs":false}"#)
            .unwrap(),
        RunRequest {
            subs: Some(3),
            duration_s: Some(60.0),
            keep_subs: Some(false)
        }
    );
    assert!(serde_json::from_str::<RunRequest>(r#"{"exposure_ms":500}"#).is_err());
    assert_eq!(
        serde_json::to_string(&RunSettings {
            subs: None,
            duration_s: 7200.0,
            keep_subs: true
        })
        .unwrap(),
        r#"{"subs":null,"duration_s":7200.0,"keep_subs":true}"#
    );
    assert_eq!(
        serde_json::to_string(&RunTarget {
            id: "NGC 1976".into(),
            name: "M42".into(),
            ra_deg: 83.82,
            dec_deg: -5.39
        })
        .unwrap(),
        r#"{"id":"NGC 1976","name":"M42","ra_deg":83.82,"dec_deg":-5.39}"#
    );
    assert_eq!(POLL, Duration::from_millis(250));
    assert_eq!(DEFAULT_DURATION_S, 7200.0);
    assert_eq!(MAX_DURATION_S, 43200.0);
    assert_eq!(MAX_SUBS, 100000);
}
