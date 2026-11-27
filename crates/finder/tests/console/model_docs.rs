//! US3-S1's oracle is 053's FR-012 and FR-013 documentation contract.
//! Wording is exact after joining whitespace; no numerical tolerance applies.

use std::{fs, path::Path};

#[test]
fn readme_and_glossary_explain_how_the_second_sync_fits_the_base_tilt() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme = fs::read_to_string(root.join("README.md"))
        .expect("the committed README must be readable")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let glossary = fs::read_to_string(root.join("../../GLOSSARY.md"))
        .expect("the committed glossary must be readable")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    for sentence in [
        "then for a solved exposure starting after that wait that the pointing model takes
         as a sync, and measures the error from that sync.",
        "`goto: {state, correction, error_arcsec, reason, frame_id, frame_time}` only after
         one starts, retaining its result until the next GoTo; `frame_id` is the star frame
         whose sync gave `error_arcsec` or ended the second sync, and `frame_time` its
         mid-exposure, so its age is the session's `time` less `frame_time`.",
        "When 30 s pass after settling with frames solved but none taken as a sync, the GoTo
         or second sync ends `failed` with `frame <id> solved but was not taken as a sync`,
         naming the last; with no frame solved, `no frame solved after settling`.",
        "It looks first between 30° and 60° up, as the model predicts the altitude, and only
         when no spot there has a safe path does it take the nearest outside it.",
        "When that sync is taken as a slip and no base tilt is fitted, it ends `failed` with
         `taken as a slip: no base tilt fitted`.",
        "Before the base tilt is fitted, a sync at least 90° on the azimuth axis from a sync
         of the current zero-point epoch joins that epoch and fits the base tilt when the
         tilt it fits is at most 6°, and is otherwise taken as a slip; the tilt alone decides
         it.",
        "opens a new zero-point epoch, as does the first solve after a push by hand while
         Track runs, at a tilt spot too.",
        "A live solve whose field lies more than 5% outside the range of the database in use
         is not taken as a sync.",
        "take the first sync between 30° and 60° up when the sky allows, and press the
         second sync at once, with the telescope untouched, before the first far GoTo.",
        "If a tree blocks the second sync's spot, a GoTo to a clear target at least 90°
         round on the azimuth axis fits the base tilt by the same rule.",
    ] {
        let sentence = sentence.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(readme.contains(&sentence), "README missing {sentence:?}");
    }

    for sentence in [
        "Fitted once two syncs of one zero-point epoch lie at least 90° apart on the azimuth
         axis, up to 6°.",
        "Before the base tilt is fitted, a sync a quarter turn round on the azimuth axis is
         one only when the tilt it fits is over 6°.",
    ] {
        let sentence = sentence.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            glossary.contains(&sentence),
            "glossary missing {sentence:?}"
        );
    }
}
