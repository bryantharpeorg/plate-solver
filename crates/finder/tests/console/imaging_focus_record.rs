//! The focus record and the verdict (033 US3): `focus::judge` against the
//! record, `Kept::note`, `Kept::record` and `Kept::reference` over two nights,
//! the record in `focus.json` beside the region, and the record's measured
//! scale. Oracles: the worked examples of the verdict's bands (3.29/3.00 =
//! 1.097, 3.31/3.00 = 1.103, 3.89/3.00 = 1.297 and 3.91/3.00 = 1.303, against
//! the plan's 10% and 30%), the worked example of the 12-hour night (43201 s
//! after a night's first settled reading the night is over, counted from the
//! night's start and not from its record's time; the smaller of 3.10 and 3.60
//! is 3.10; the smaller of 3.10 and 2.90 is the earlier nights' record once the
//! second night is over; a night whose record, 3.80, is larger than the
//! earlier nights' leaves them as they were), the stated keys and values of
//! the file, and the worked examples of the 5% rule (2.28/2.19 = 1.041;
//! 2.33/2.19 = 1.064; 1.10/2.19 = 0.502; 2.19/1.10 = 1.991; 1.12/1.10 = 1.018).

use std::path::Path;

use finder::imaging::{
    focus::{self, FocusReading, KeptRegion, Mark, Of, RecordView, Tile, Unmeasurable, Verdict},
    relation::CameraId,
    solver::Region,
};
use serde_json::{json, Value};

use crate::support::Scratch;

/// The test's Unix moment (the shared setup): a night begins here.
const T: f64 = 1_768_528_800.0;

/// The camera the record belongs to (the shared setup): the simulated imaging
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

/// Another camera of the same model, by serial (the shared setup).
fn other() -> CameraId {
    CameraId {
        serial: Some("SIM-0002".to_string()),
        ..camera()
    }
}

/// The whole sensor's region, 1236 × 822 (the shared setup).
fn whole() -> Region {
    Region {
        x: 0,
        y: 0,
        width: 1236,
        height: 822,
    }
}

/// One part of the record: a size and when it was read.
fn mark(px: f64, at: f64) -> Option<Mark> {
    Some(Mark { px, at })
}

/// The file's JSON, whole (the shared setup).
fn read_file(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// A region found from a reading, as US2-S5 keeps one (the shared setup): the
/// top right tile, 2.6 px of 12 stars of frame 3.
fn found() -> KeptRegion {
    let reading = FocusReading {
        frame_id: 3,
        px: 2.6,
        arcsec: None,
        stars: 12,
        region: Region {
            x: 824,
            y: 0,
            width: 412,
            height: 274,
        },
        at: T + 3.5,
    };
    KeptRegion::found(whole(), 1, Tile::new(2, 0).unwrap(), &reading)
}

/// US3-S1: against the reference 3.00 a reading of 2.50 is good, 3.29 good,
/// 3.31 slightly soft, 3.89 slightly soft and 3.91 clearly out; with no
/// reference each of those is no record; too few stars is unmeasurable and far
/// out clearly out, whatever the record; and `Verdict::as_str` gives exactly
/// those five words (oracle: the worked examples 3.29/3.00 = 1.097, 3.31/3.00
/// = 1.103, 3.89/3.00 = 1.297 and 3.91/3.00 = 1.303, against the plan's 10%
/// and 30%).
#[test]
fn the_verdict_against_the_record() {
    let against = |px: f64| focus::judge(Ok(px), Some(3.00));
    assert_eq!(against(2.50), Verdict::Good);
    assert_eq!(against(3.29), Verdict::Good);
    assert_eq!(against(3.31), Verdict::SlightlySoft);
    assert_eq!(against(3.89), Verdict::SlightlySoft);
    assert_eq!(against(3.91), Verdict::ClearlyOut);

    // With no reference each of those is no record.
    for px in [2.50, 3.29, 3.31, 3.89, 3.91] {
        assert_eq!(focus::judge(Ok(px), None), Verdict::NoRecord);
    }

    // Whatever the record: too few stars is unmeasurable and far out is
    // clearly out.
    for reference in [Some(3.00), None] {
        assert_eq!(
            focus::judge(Err(Unmeasurable::TooFewStars { stars: 3 }), reference),
            Verdict::Unmeasurable
        );
        assert_eq!(
            focus::judge(
                Err(Unmeasurable::FarOut {
                    px: 40.0,
                    stars: 12
                }),
                reference
            ),
            Verdict::ClearlyOut
        );
    }

    // The five words, exactly.
    assert_eq!(Verdict::Good.as_str(), "good");
    assert_eq!(Verdict::SlightlySoft.as_str(), "slightly_soft");
    assert_eq!(Verdict::ClearlyOut.as_str(), "clearly_out");
    assert_eq!(Verdict::Unmeasurable.as_str(), "unmeasurable");
    assert_eq!(Verdict::NoRecord.as_str(), "no_record");
}

/// US3-S2: the record over two nights. Tonight's part holds the night's
/// smallest settled reading; a night is over 12 hours after it began, counted
/// from its start and not from its record's time; the old night's record joins
/// the earlier nights' only when smaller; the smaller part judges until a note
/// marks tonight's as passed and tonight's alone from then on; the mark stays
/// for the night, and a night's first reading carries the mark it is given
/// (oracle: the worked example of the 12-hour night and of the rule of owner
/// decision 8).
#[test]
fn the_record_over_two_nights() {
    let scratch = Scratch::new("imaging-focus-record-nights");
    let mut kept = focus::Kept::new(scratch.0.join("focus.json"));
    kept.describe(camera());

    // Before any note the record has neither part and judges nothing.
    assert_eq!(kept.record(T, None), RecordView::default());
    assert_eq!(kept.reference(T, None), None);

    // The night's readings: the smaller replaces tonight's record.
    assert!(kept.note(3.40, T, None, false));
    assert!(kept.note(3.10, T + 60.0, None, false));
    assert!(!kept.note(3.30, T + 120.0, None, false));
    assert_eq!(
        kept.record(T + 200.0, None),
        RecordView {
            tonight: mark(3.1, T + 60.0),
            earlier: None,
            passed: false,
        }
    );
    assert_eq!(kept.reference(T + 200.0, None), Some((3.1, Of::Tonight)));

    // 43201 s after the night began it is over: no tonight, and the earlier
    // nights' record is the night's reading.
    assert_eq!(
        kept.record(T + 43_201.0, None),
        RecordView {
            tonight: None,
            earlier: mark(3.1, T + 60.0),
            passed: false,
        }
    );
    assert_eq!(kept.reference(T + 43_201.0, None), Some((3.1, Of::Earlier)));

    // A second Kept, of its own file: the same three readings, but the first
    // with the mark. A night's first reading carries the mark it is given, and
    // the night began at T, more than 12 hours before its record was read.
    let scratch = Scratch::new("imaging-focus-record-mark");
    let mut second = focus::Kept::new(scratch.0.join("focus.json"));
    second.describe(camera());
    assert!(second.note(3.40, T, None, true));
    assert!(second.note(3.10, T + 60.0, None, false));
    assert!(!second.note(3.30, T + 120.0, None, false));
    assert!(second.record(T + 1.0, None).passed);
    assert!(second.note(3.50, T + 43_230.0, None, false));
    assert_eq!(
        second.record(T + 43_231.0, None),
        RecordView {
            tonight: mark(3.5, T + 43_230.0),
            earlier: mark(3.1, T + 60.0),
            passed: false,
        }
    );

    // Back on the first Kept: a new night. The old night's record joins the
    // earlier nights', and the smaller part judges, tonight's not having been
    // passed through.
    assert!(kept.note(3.60, T + 50_000.0, None, false));
    assert_eq!(
        kept.record(T + 50_001.0, None),
        RecordView {
            tonight: mark(3.6, T + 50_000.0),
            earlier: mark(3.1, T + 60.0),
            passed: false,
        }
    );
    assert_eq!(kept.reference(T + 50_001.0, None), Some((3.1, Of::Earlier)));

    // A note that passes through focus marks tonight's part, and tonight's
    // judges alone from then on.
    assert!(kept.note(3.70, T + 50_030.0, None, true));
    assert_eq!(
        kept.record(T + 50_031.0, None),
        RecordView {
            tonight: mark(3.6, T + 50_000.0),
            earlier: mark(3.1, T + 60.0),
            passed: true,
        }
    );
    assert_eq!(kept.reference(T + 50_031.0, None), Some((3.6, Of::Tonight)));

    // Nothing changed: the reading is not smaller and the mark is never set
    // back within a night.
    assert!(!kept.note(3.65, T + 50_040.0, None, true));

    // A smaller reading keeps the mark it already has.
    assert!(kept.note(2.90, T + 50_060.0, None, false));
    assert_eq!(
        kept.record(T + 50_061.0, None),
        RecordView {
            tonight: mark(2.9, T + 50_060.0),
            earlier: mark(3.1, T + 60.0),
            passed: true,
        }
    );
    assert_eq!(kept.reference(T + 50_061.0, None), Some((2.9, Of::Tonight)));

    // The second night over too: the smaller of 3.10 and 2.90 is the earlier
    // nights' record.
    assert_eq!(
        kept.record(T + 100_000.0, None),
        RecordView {
            tonight: None,
            earlier: mark(2.9, T + 50_060.0),
            passed: false,
        }
    );
    assert_eq!(
        kept.reference(T + 100_000.0, None),
        Some((2.9, Of::Earlier))
    );

    // A night whose record, 3.80, is larger than the earlier nights' does not
    // join them, and the next night's leaves them as they were.
    assert!(kept.note(3.80, T + 100_000.0, None, false));
    assert_eq!(
        kept.record(T + 100_001.0, None),
        RecordView {
            tonight: mark(3.8, T + 100_000.0),
            earlier: mark(2.9, T + 50_060.0),
            passed: false,
        }
    );
    kept.note(3.90, T + 150_000.0, None, false);
    assert_eq!(
        kept.record(T + 150_001.0, None).earlier,
        mark(2.9, T + 50_060.0)
    );
}

/// US3-S3: the record is written to `focus.json` beside the region, neither
/// write losing what the other wrote, and read back when the camera is
/// described; `passed` is in the file only once it is true (oracle: the stated
/// keys and values, compared exactly).
#[test]
fn the_record_in_the_file_beside_the_region() {
    let scratch = Scratch::new("imaging-focus-record-file");
    let path = scratch.0.join("focus.json");
    let mut kept = focus::Kept::new(path.clone());
    kept.describe(camera());

    // A region kept as in US2-S5, then the night's first three readings.
    kept.keep_region(found());
    assert!(kept.note(3.40, T, None, false));
    assert!(kept.note(3.10, T + 60.0, None, false));
    assert!(!kept.note(3.30, T + 120.0, None, false));

    let file = read_file(&path);
    assert_eq!(
        file["record"]["tonight"],
        json!({"px": 3.1, "at": T + 60.0, "since": T})
    );
    assert!(file["record"]["tonight"].get("passed").is_none(), "{file}");
    assert!(file["record"].get("earlier").is_none(), "{file}");
    assert!(file["record"].get("arcsec_per_px").is_none(), "{file}");
    // The region is still in the file.
    assert_eq!(file["region"]["col"], json!(2));
    assert_eq!(file["region"]["source"], json!("found"));

    // A second Kept answers the same record and the same region; another
    // camera's answers neither.
    let mut again = focus::Kept::new(path.clone());
    again.describe(camera());
    assert_eq!(again.record(T + 200.0, None), kept.record(T + 200.0, None));
    assert_eq!(again.region(), kept.region());
    again.describe(other());
    assert_eq!(again.record(T + 200.0, None), RecordView::default());
    assert_eq!(again.region(), None);

    // Keeping a region after the notes leaves the record in the file.
    kept.keep_region(KeptRegion::fixed(
        whole(),
        1,
        Tile::new(1, 1).unwrap(),
        T + 130.0,
    ));
    let file = read_file(&path);
    assert_eq!(file["record"]["tonight"]["px"], json!(3.1));
    assert_eq!(file["region"]["source"], json!("fixed"));

    // A note that passes through focus marks tonight's part in the file — its
    // smallest reading stays — and a third Kept reads the mark back.
    assert!(kept.note(3.20, T + 130.0, None, true));
    let file = read_file(&path);
    assert_eq!(file["record"]["tonight"]["passed"], json!(true));
    assert_eq!(file["record"]["tonight"]["px"], json!(3.1));
    let mut third = focus::Kept::new(path.clone());
    third.describe(camera());
    assert!(third.record(T + 200.0, None).passed);
}

/// US3-S4: the record is kept with the measured scale in force when it was
/// last noted. While both that scale and the measured scale now in force are
/// known and differ by more than 5%, the record judges nothing and shows
/// nothing, and the next note replaces it. A record noted while no measured
/// scale was known carries none, is never set aside by this rule, and takes
/// the first measured scale a later note that changes it brings (oracle: the
/// worked examples of the 5% rule: 2.28/2.19 = 1.041; 2.33/2.19 = 1.064;
/// 1.10/2.19 = 0.502; 2.19/1.10 = 1.991; 1.12/1.10 = 1.018).
#[test]
fn the_record_s_scale_and_other_telescopes() {
    let scratch = Scratch::new("imaging-focus-record-scale");
    let path = scratch.0.join("focus.json");
    let mut kept = focus::Kept::new(path.clone());
    kept.describe(camera());

    // A record made at a measured scale of 2.19.
    assert!(kept.note(3.00, T, Some(2.19), false));
    let file = read_file(&path);
    assert_eq!(file["record"]["arcsec_per_px"], json!(2.19));

    // A scale 4.1% away is this telescope's; 6.4% is another's, and half is
    // too: the record judges nothing and shows nothing there.
    assert_eq!(
        kept.reference(T + 10.0, Some(2.28)),
        Some((3.00, Of::Tonight))
    );
    assert_eq!(kept.reference(T + 10.0, Some(2.33)), None);
    assert_eq!(kept.record(T + 10.0, Some(2.33)), RecordView::default());
    assert_eq!(kept.reference(T + 10.0, None), Some((3.00, Of::Tonight)));
    assert_eq!(kept.reference(T + 10.0, Some(1.10)), None);
    assert_eq!(kept.record(T + 10.0, Some(1.10)), RecordView::default());

    // The next note replaces the record, and holds the scale it was made at.
    assert!(kept.note(4.00, T + 20.0, Some(1.10), false));
    assert_eq!(
        kept.record(T + 25.0, Some(1.10)),
        RecordView {
            tonight: mark(4.0, T + 20.0),
            earlier: None,
            passed: false,
        }
    );
    let file = read_file(&path);
    assert_eq!(file["record"]["arcsec_per_px"], json!(1.10));

    // A record made with no measured scale carries none, and is no
    // telescope's in particular.
    let bare_path = scratch.0.join("bare.json");
    let mut bare = focus::Kept::new(bare_path.clone());
    bare.describe(camera());
    assert!(bare.note(3.00, T, None, false));
    let file = read_file(&bare_path);
    assert!(file["record"].get("arcsec_per_px").is_none(), "{file}");
    assert_eq!(
        bare.reference(T + 10.0, Some(1.10)),
        Some((3.00, Of::Tonight))
    );

    // A note that changes nothing writes no scale into the file either.
    assert!(!bare.note(3.20, T + 20.0, Some(1.10), false));
    let file = read_file(&bare_path);
    assert!(file["record"].get("arcsec_per_px").is_none(), "{file}");

    // The next note that changes it brings its scale.
    assert!(bare.note(2.90, T + 30.0, Some(1.10), false));
    let file = read_file(&bare_path);
    assert_eq!(file["record"]["arcsec_per_px"], json!(1.10));

    // From 1.10, 2.19 is 99.1% away and 1.12 is 1.8%.
    assert_eq!(bare.reference(T + 40.0, Some(2.19)), None);
    assert_eq!(
        bare.reference(T + 40.0, Some(1.12)),
        Some((2.90, Of::Tonight))
    );
}
