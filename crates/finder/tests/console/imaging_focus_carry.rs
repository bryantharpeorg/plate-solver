//! The record's altitude and the carry (054 US2): `focus::carry` and
//! `focus::judge` against the record carried to a reading's altitude, and
//! `Kept::note_at`, `Kept::reference_at` and `Kept::altitudes` over the records
//! of their files: a night and the night after it, a second Kept on the same
//! file, a night past, the foreign scale's rule, and a record written by hand
//! with no altitudes. Oracles: the airmass as the secant of the zenith distance,
//! written here as 1 / sin(altitude), the exponent 0.6 and the bounds 1 and 1.30,
//! written here too; the field night's numbers (4.6″ read at 74°, 6.1″ at 37°,
//! and the readings 6.4″, 6.7″, 7.3″ and 8.5″); and the altitudes and sizes the
//! scenarios state, with the file's keys as they are written.

use std::path::Path;

use finder::imaging::{
    focus::{self, Altitudes, Kept, Of, Verdict},
    relation::CameraId,
};
use serde_json::{json, Value};

use crate::support::Scratch;

/// The Unix moment the nights begin at (the shared setup).
const T: f64 = 1_768_528_800.0;

/// The camera the records belong to (the shared setup): the simulated imaging
/// camera.
fn camera() -> CameraId {
    CameraId {
        model: "simulated imaging camera".to_string(),
        serial: Some("SIM-0001".to_string()),
        width: 1236,
        height: 822,
        pixel_um: 19.12,
    }
}

/// The airmass of an altitude in degrees: the secant of the zenith distance,
/// written here and not taken from the code under test (US2-S1).
fn airmass(alt_deg: f64) -> f64 {
    1.0 / alt_deg.to_radians().sin()
}

/// The record of `px` read at `from_deg`, as it is carried to `to_deg` (US2-S1):
/// times the airmass ratio to the power 0.6, held between 1 and 1.30.
fn carried(px: f64, from_deg: f64, to_deg: f64) -> f64 {
    px * (airmass(to_deg) / airmass(from_deg))
        .powf(0.6)
        .clamp(1.0, 1.30)
}

/// The file's JSON, whole (the shared setup).
fn read_file(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// US2-S1: `carry` leaves a record at the reading's own altitude as it is, carries
/// it to another altitude by the airmass ratio to the power 0.6 held between 1
/// and 1.30, and never makes it stricter. 4.6 read at 74° is 4.6 at 74°, lies
/// within 1e-12 of `carried` at 60° (strictly between 4.6 and 4.6 × 1.30), is
/// 4.6 × 1.30 at 20°; and 6.1 read at 37° is 6.1 at 70°. Against those records
/// the field night's readings judge: 6.7 and 6.4 are `clearly_out` against 4.6
/// (6.4 is 1.39 times it), 6.4 is `good` against 4.6 carried to 38°, 7.3 is
/// `slightly_soft` and 8.5 is `clearly_out` against 4.6 carried to 37°.
#[test]
fn the_record_is_carried_to_the_reading_altitude() {
    assert_eq!(focus::carry(4.6, 74.0, 74.0), 4.6);

    let at_60 = focus::carry(4.6, 74.0, 60.0);
    assert!((at_60 - carried(4.6, 74.0, 60.0)).abs() < 1e-12);
    assert!(4.6 < at_60 && at_60 < 4.6 * 1.30);

    assert!((focus::carry(4.6, 74.0, 20.0) - 4.6 * 1.30).abs() < 1e-12);
    assert_eq!(focus::carry(6.1, 37.0, 70.0), 6.1);

    assert_eq!(focus::judge(Ok(6.7), Some(4.6)), Verdict::ClearlyOut);
    assert_eq!(focus::judge(Ok(6.4), Some(4.6)), Verdict::ClearlyOut);
    assert_eq!(
        focus::judge(Ok(6.4), Some(focus::carry(4.6, 74.0, 38.0))),
        Verdict::Good
    );
    assert_eq!(
        focus::judge(Ok(7.3), Some(focus::carry(4.6, 74.0, 37.0))),
        Verdict::SlightlySoft
    );
    assert_eq!(
        focus::judge(Ok(8.5), Some(focus::carry(4.6, 74.0, 37.0))),
        Verdict::ClearlyOut
    );
}

/// US2-S2: the altitudes of the record over a night and the night after it. The
/// first reading of the night, 3.0 at 74°, is tonight's and is written with its
/// altitude; a smaller one at 60° replaces it and a larger one at 30° does not.
/// A second Kept on the same file reads 2.9 at 60°. A reading 43260 s on starts
/// a night and the old night's 2.9 joins the earlier nights' with its altitude,
/// 60°, which the file keeps as `earlier_alt_deg`, while tonight's record has no
/// altitude. A smaller reading, 2.5 at no altitude, then takes tonight. On a
/// fresh Kept, 3.0 at 74° read a night past is the earlier nights' record at 74°.
#[test]
fn the_altitudes_over_a_night_and_the_night_after() {
    let scratch = Scratch::new("imaging-focus-carry-nights");
    let path = scratch.0.join("focus.json");
    let mut kept = Kept::new(path.clone());
    kept.describe(camera());

    assert!(kept.note_at(3.0, T, None, false, Some(74.0)));
    assert_eq!(
        kept.reference_at(T + 1.0, None),
        Some((3.0, Of::Tonight, Some(74.0)))
    );
    assert_eq!(
        read_file(&path)["record"]["tonight"]["alt_deg"],
        json!(74.0)
    );

    assert!(kept.note_at(2.9, T + 60.0, None, false, Some(60.0)));
    assert!(!kept.note_at(3.1, T + 120.0, None, false, Some(30.0)));
    assert_eq!(kept.altitudes(T + 121.0, None).tonight, Some(60.0));

    // A second Kept on the same file reads the record as it was kept.
    let mut second = Kept::new(path.clone());
    second.describe(camera());
    assert_eq!(
        second.reference_at(T + 121.0, None),
        Some((2.9, Of::Tonight, Some(60.0)))
    );

    // A night later: the old night's 2.9 at 60° joins the earlier nights'.
    assert!(kept.note_at(3.2, T + 43_260.0, None, false, None));
    assert_eq!(
        kept.reference_at(T + 43_261.0, None),
        Some((2.9, Of::Earlier, Some(60.0)))
    );
    assert_eq!(
        kept.altitudes(T + 43_261.0, None),
        Altitudes {
            tonight: None,
            earlier: Some(60.0),
        }
    );
    let file = read_file(&path);
    assert_eq!(file["record"]["earlier_alt_deg"], json!(60.0));
    assert!(file["record"]["tonight"].get("alt_deg").is_none());

    // A smaller reading of tonight, with no altitude, takes tonight's place.
    assert!(kept.note(2.5, T + 43_300.0, None, false));
    assert_eq!(
        kept.reference_at(T + 43_301.0, None),
        Some((2.5, Of::Tonight, None))
    );

    // A fresh Kept on another file: 3.0 at 74° read a night past is the earlier
    // nights' record, and keeps its altitude.
    let mut past = Kept::new(scratch.0.join("focus-past.json"));
    past.describe(camera());
    assert!(past.note_at(3.0, T, None, false, Some(74.0)));
    assert_eq!(
        past.reference_at(T + 43_201.0, None),
        Some((3.0, Of::Earlier, Some(74.0)))
    );
}

/// US2-S3: the altitudes at a scale, and a foreign record's rule. A record made
/// at 0.55 keeps its altitude at 0.55 and shows none at 1.1; a reading at 1.1
/// drops it as another telescope's and starts a night with no altitude. On a
/// second file, a night and the night after it at 0.55 keep 60° for the earlier
/// part, and a reading at 1.1 drops that with the record, leaving tonight's 50°.
/// A file written by hand, of 033, with no altitudes reads as it did: 3.3 at
/// `T − 800`, with no altitude.
#[test]
fn the_altitudes_by_scale_and_a_foreign_record() {
    let scratch = Scratch::new("imaging-focus-carry-scaled");

    let mut scaled = Kept::new(scratch.0.join("focus-scaled.json"));
    scaled.describe(camera());
    assert!(scaled.note_at(3.0, T, Some(0.55), false, Some(74.0)));
    assert_eq!(scaled.altitudes(T + 1.0, Some(0.55)).tonight, Some(74.0));
    assert_eq!(scaled.altitudes(T + 1.0, Some(1.1)), Altitudes::default());
    assert!(scaled.note_at(5.0, T + 60.0, Some(1.1), false, None));
    assert_eq!(
        scaled.reference_at(T + 61.0, Some(1.1)),
        Some((5.0, Of::Tonight, None))
    );

    let mut foreign = Kept::new(scratch.0.join("focus-foreign.json"));
    foreign.describe(camera());
    assert!(foreign.note_at(3.0, T, Some(0.55), false, Some(60.0)));
    assert!(foreign.note_at(3.5, T + 43_260.0, Some(0.55), false, Some(70.0)));
    assert_eq!(
        foreign.altitudes(T + 43_261.0, Some(0.55)).earlier,
        Some(60.0)
    );
    assert!(foreign.note_at(4.0, T + 43_300.0, Some(1.1), false, Some(50.0)));
    assert_eq!(
        foreign.altitudes(T + 43_301.0, Some(1.1)),
        Altitudes {
            tonight: Some(50.0),
            earlier: None,
        }
    );

    // 033's own focus.json, written by hand: no altitudes anywhere.
    let by_hand = json!({
        "format": "imaging-focus/1",
        "camera": {
            "model": "simulated imaging camera",
            "serial": "SIM-0001",
            "width": 1236,
            "height": 822,
            "pixel_um": 19.12
        },
        "record": {
            "tonight": { "px": 3.3, "at": T - 800.0, "since": T - 800.0 }
        }
    });
    std::fs::write(scratch.0.join("focus.json"), by_hand.to_string()).unwrap();
    let mut old = Kept::new(scratch.0.join("focus.json"));
    old.describe(camera());
    assert_eq!(old.reference_at(T, None), Some((3.3, Of::Tonight, None)));
    assert_eq!(old.reference(T, None), Some((3.3, Of::Tonight)));
}
