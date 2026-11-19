//! US5: exact worked examples and the simulator's declared hot-pixel places.
//! Files are inspected directly; no product reader supplies a persistence oracle.

use std::{fs, io::ErrorKind};

use finder::imaging::calibration::{
    bad_pixel_map,
    library::{FrameKey, Kind, Library, MasterEntry},
    BadPixels, Mosaic, BAD_PIXEL_MAX_SHARE, BAD_PIXEL_SIGMAS,
};
use scope_node::imaging::{Exposure, ImagingCamera, ImagingSettings};
use serde_json::{json, Value};
use simulator::{
    HotPixel, LightPath, SensorPattern, SimImagingCamera, Simulator, Site, VirtualClock,
};

use crate::support::{live as support, Scratch};

fn worked_dark() -> Vec<f32> {
    let mut samples = vec![160.0; 400];
    samples[2 * 20 + 3] = 163.6;
    samples[7 * 20 + 7] = 163.4;
    samples[5 * 20 + 5] = 400.0;
    samples
}

/// US5-S1: median 160, spread floored to 1, strict line at 163.5.
#[test]
fn map_uses_three_and_a_half_spreads_and_row_order() {
    assert_eq!(BAD_PIXEL_SIGMAS, 3.5);
    assert_eq!(BAD_PIXEL_MAX_SHARE, 0.01);
    assert_eq!(
        bad_pixel_map(&worked_dark(), 20, 20),
        BadPixels {
            pixels: vec![(3, 2), (5, 5)],
            level: 160.0,
            spread: 1.0,
            too_many: false,
        }
    );
    let mut samples = vec![160.0; 400];
    samples[6 * 20 + 2] = 1000.0;
    samples[20 + 9] = 1000.0;
    samples[10] = 163.5;
    assert_eq!(bad_pixel_map(&samples, 20, 20).pixels, [(9, 1), (2, 6)]);
}

/// US5-S2: sorted indices 63 and 20 give 161 and 155; line 182.
#[test]
fn fixed_columns_widen_the_spread_without_being_mapped() {
    let mut samples: Vec<_> = (0..8)
        .flat_map(|_| (0..16).map(|x| 160.0 + 2.0 * (x % 8) as f32 - 7.0))
        .collect();
    samples[3 * 16 + 9] = 200.0;
    assert_eq!(
        bad_pixel_map(&samples, 16, 8),
        BadPixels {
            pixels: vec![(9, 3)],
            level: 161.0,
            spread: 6.0,
            too_many: false,
        }
    );
}

/// US5-S3: exactly one of 100 is allowed, two are not. Non-finite
/// samples are bad regardless of sign or comparison with the line.
#[test]
fn one_percent_is_inclusive_and_non_finite_samples_are_bad() {
    let mut samples = vec![160.0; 100];
    samples[23] = 1000.0;
    let map = bad_pixel_map(&samples, 10, 10);
    assert_eq!(map.pixels, [(3, 2)]);
    assert!(!map.too_many);
    samples[55] = 1000.0;
    let map = bad_pixel_map(&samples, 10, 10);
    assert!(map.pixels.is_empty());
    assert!(map.too_many);
    assert_eq!((map.level, map.spread), (160.0, 1.0));
    samples[55] = 160.0;
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        samples[23] = value;
        let map = bad_pixel_map(&samples, 10, 10);
        assert_eq!(map.pixels, [(3, 2)]);
        assert_eq!((map.level, map.spread), (160.0, 1.0));
        assert!(!map.too_many);
    }
}

/// US5-S4: average the camera's eight ordinary, noisy, covered exposures
/// ourselves. H's three declared places are the oracle, with exact membership.
#[tokio::test(start_paused = true)]
async fn mean_of_eight_simulated_darks_maps_exactly_h() {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.push_to(50.0, 120.0);
    sim.set_sensor_pattern(SensorPattern {
        hot_pixels: [(100, 50, 4000.0), (101, 51, 2000.0), (600, 400, 40000.0)]
            .into_iter()
            .map(|(x, y, electrons_per_s)| HotPixel {
                x,
                y,
                electrons_per_s,
            })
            .collect(),
        column_amplitude: 0.0,
    });
    sim.set_light_path(LightPath {
        cover: true,
        ..Default::default()
    });
    let camera = SimImagingCamera::new(sim);
    camera
        .apply(&ImagingSettings {
            gain: Some(0.0),
            ..Default::default()
        })
        .await
        .unwrap();
    let mut sum = Vec::<u32>::new();
    let mut size = (0, 0);
    for _ in 0..8 {
        let frame = camera
            .expose(&Exposure {
                exposure_ms: 1000.0,
                purpose: "dark".into(),
            })
            .await
            .unwrap();
        assert_eq!(frame.bits, 16);
        assert_eq!((frame.width, frame.height), (1236, 822));
        size = (frame.width, frame.height);
        sum.resize((size.0 * size.1) as usize, 0);
        for (total, bytes) in sum.iter_mut().zip(frame.samples.as_chunks::<2>().0) {
            *total += u32::from(u16::from_le_bytes(*bytes));
        }
    }
    let mean: Vec<_> = sum.into_iter().map(|v| v as f32 / 8.0).collect();
    let map = bad_pixel_map(&mean, size.0, size.1);
    assert_eq!(map.pixels, [(100, 50), (101, 51), (600, 400)]);
    assert!(!map.too_many);
}

fn entry(id: &str, kind: Kind, size: u32) -> MasterEntry {
    MasterEntry {
        id: id.into(),
        kind,
        made_at: 1_768_528_803.0,
        frames: 8,
        checked: true,
        model: "simulated imaging camera".into(),
        serial: Some("SIM-0001".into()),
        width: size,
        height: size,
        origin_x: 0,
        origin_y: 0,
        bits: 16,
        bayer: Some("RGGB".into()),
        gain: Some(0.0),
        offset: Some(10.0),
        binning: 1,
        set_temp_c: None,
        temp_c: Some(20.0),
        exposure_ms: 1000.0,
        level: (kind == Kind::Dark).then_some(160.0),
        bad_pixels: None,
        label: (kind == Kind::Flat).then(|| "desk".into()),
        imaging_angle_deg: None,
        level_share: (kind == Kind::Flat).then_some(0.45),
        dark_frames: (kind == Kind::Flat).then_some(8),
        file: String::new(),
    }
}

fn sub(size: u32) -> Mosaic {
    let mut samples = vec![560.0; (size * size) as usize];
    samples[(2 * size + 3) as usize] = 5000.0;
    Mosaic {
        width: size,
        height: size,
        bayer: Some("RGGB".into()),
        samples,
    }
}

const DARK: &str = "dark-20260116T020003Z";

/// US5-S5: the persisted JSON and repaired counts are independent worked
/// values. Removing the files after a first use also proves the read cache.
#[test]
fn library_attaches_persists_selects_uses_caches_and_removes_a_map() {
    let scratch = Scratch::new("bad-pixels-library");
    let library = Library::open(scratch.0.clone());
    let dark = library
        .add(entry(DARK, Kind::Dark, 20), &worked_dark())
        .unwrap();
    let key = FrameKey::from(&dark);
    let file = scratch.0.join(format!("darks/{DARK}.badpixels.json"));
    assert_eq!(dark.bad_pixels, None);
    assert!(!file.exists());
    let attached = library
        .attach_bad_pixels(DARK, &bad_pixel_map(&worked_dark(), 20, 20))
        .unwrap();
    assert_eq!(attached.bad_pixels, Some(2));
    assert_eq!(library.masters(), std::slice::from_ref(&attached));
    let index: Value =
        serde_json::from_slice(&fs::read(scratch.0.join("index.json")).unwrap()).unwrap();
    assert_eq!(index["masters"], json!([attached]));
    let bytes = fs::read(&file).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap(),
        json!({"format":"finder-bad-pixels/1", "width":20, "height":20,
            "level":160.0, "spread":1.0, "sigmas":3.5, "pixels":[[3,2],[5,5]]})
    );
    assert!(!file.with_extension("json.part").exists());
    let selection = library.select(&key, None, None);
    assert_eq!(selection.bad_pixels, Some(2));
    assert_eq!(selection.missing, ["flat"]);
    // Reopening proves that the index alone is sufficient to find the map.
    let library = Library::open(scratch.0.clone());
    assert_eq!(library.masters(), [attached]);
    for cached in [false, true] {
        if cached {
            fs::remove_file(&file).unwrap();
            fs::remove_file(scratch.0.join(&dark.file)).unwrap();
        }
        let mut sub = sub(20);
        let applied = library.calibrate(&key, &mut sub, None, None);
        assert_eq!(applied.selection, selection);
        assert!(applied.problems.is_empty());
        assert_eq!(applied.report.dark, "applied");
        assert_eq!(applied.report.bad_pixels, "map");
        assert_eq!(applied.report.repaired, 2);
        assert_eq!(sub.samples[2 * 20 + 3], 400.0);
        assert_eq!(sub.samples[5 * 20 + 5], 400.0);
    }
    fs::write(&file, bytes).unwrap();
    assert!(library.remove(DARK).unwrap());
    assert!(!file.exists());
    assert!(!scratch.0.join(dark.file).exists());
    assert!(library.masters().is_empty());
    let index: Value =
        serde_json::from_slice(&fs::read(scratch.0.join("index.json")).unwrap()).unwrap();
    assert_eq!(index["masters"], json!([]));
}

/// An empty map is still a map, so it suppresses the sub search. Attaching
/// another map invalidates that cached empty map; removal forgets it too.
#[test]
fn empty_maps_and_replacement_do_not_leave_stale_repairs() {
    let scratch = Scratch::new("bad-pixels-replacement");
    let library = Library::open(scratch.0.clone());
    let dark = library
        .add(entry(DARK, Kind::Dark, 20), &worked_dark())
        .unwrap();
    let key = FrameKey::from(&dark);
    let empty = bad_pixel_map(&[160.0; 400], 20, 20);
    let attached = library.attach_bad_pixels(DARK, &empty).unwrap();
    assert_eq!(attached.bad_pixels, Some(0));
    assert_eq!(library.select(&key, None, None).missing, ["flat"]);
    let mut unrepaired = sub(20);
    let applied = library.calibrate(&key, &mut unrepaired, None, None);
    assert_eq!(applied.report.bad_pixels, "map");
    assert_eq!(applied.report.repaired, 0);
    assert_eq!(unrepaired.samples[2 * 20 + 3], 5000.0 - 163.6);

    library
        .attach_bad_pixels(DARK, &bad_pixel_map(&worked_dark(), 20, 20))
        .unwrap();
    let applied = library.calibrate(&key, &mut sub(20), None, None);
    assert_eq!(applied.report.bad_pixels, "map");
    assert_eq!(applied.report.repaired, 2);
    assert!(library.remove(DARK).unwrap());
    library
        .add(entry(DARK, Kind::Dark, 20), &[160.0; 400])
        .unwrap();
    let applied = library.calibrate(&key, &mut sub(20), None, None);
    assert_eq!(applied.report.bad_pixels, "sub");
    assert_eq!(applied.report.repaired, 1);
    assert!(applied.problems.is_empty());
}

/// US5-S5: unusable maps preserve the selection and explain the fallback
/// exactly. Invalid JSON and a missing file must also trigger the sub search.
#[test]
fn unusable_map_falls_back_to_sub_with_a_problem() {
    let scratch = Scratch::new("bad-pixels-fallback");
    let library = Library::open(scratch.0.clone());
    let dark = library
        .add(entry(DARK, Kind::Dark, 20), &worked_dark())
        .unwrap();
    let key = FrameKey::from(&dark);
    library
        .attach_bad_pixels(DARK, &bad_pixel_map(&worked_dark(), 20, 20))
        .unwrap();
    let file = scratch.0.join(format!("darks/{DARK}.badpixels.json"));
    let valid: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    for (field, value, why) in [
        (
            "format",
            json!("finder-bad-pixels/2"),
            "its format is finder-bad-pixels/2",
        ),
        ("width", json!(10), "it is 10 × 20 and the sub 20 × 20"),
        ("height", json!(10), "it is 20 × 10 and the sub 20 × 20"),
    ] {
        let mut invalid = valid.clone();
        invalid[field] = value;
        fs::write(&file, serde_json::to_vec(&invalid).unwrap()).unwrap();
        let library = Library::open(scratch.0.clone());
        let applied = library.calibrate(&key, &mut sub(20), None, None);
        assert_eq!(applied.report.bad_pixels, "sub");
        assert_eq!(applied.selection.bad_pixels, Some(2));
        assert_eq!(applied.report.repaired, 1);
        assert_eq!(
            applied.problems,
            [format!(
                "the bad-pixel map of {DARK} could not be used: {why}"
            )]
        );
    }
    for missing in [false, true] {
        if missing {
            fs::remove_file(&file).unwrap();
        } else {
            fs::write(&file, b"not JSON").unwrap();
        }
        let library = Library::open(scratch.0.clone());
        let applied = library.calibrate(&key, &mut sub(20), None, None);
        assert_eq!(applied.report.bad_pixels, "sub");
        assert_eq!(applied.problems.len(), 1);
        assert!(applied.problems[0]
            .starts_with(&format!("the bad-pixel map of {DARK} could not be used: ")));
    }
}

/// US5-S5: attachment validates the id before writing anything, including
/// for an oversized map; oversized maps and an ordinary add keep null.
#[test]
fn rejected_ids_and_oversized_maps_do_not_write_or_change_entries() {
    let scratch = Scratch::new("bad-pixels-rejected");
    let library = Library::open(scratch.0.clone());
    let flat = library
        .add(entry("flat-20260116T020003Z", Kind::Flat, 10), &[1.0; 100])
        .unwrap();
    let mut samples = vec![160.0; 100];
    samples[23] = 1000.0;
    samples[55] = 1000.0;
    let dark = library.add(entry(DARK, Kind::Dark, 10), &samples).unwrap();
    let key = FrameKey::from(&dark);
    let map = bad_pixel_map(&samples, 10, 10);
    assert!(map.too_many);
    let index = fs::read(scratch.0.join("index.json")).unwrap();
    assert_eq!(library.attach_bad_pixels(DARK, &map).unwrap(), dark);
    assert_eq!(fs::read(scratch.0.join("index.json")).unwrap(), index);
    let file = scratch.0.join(format!("darks/{DARK}.badpixels.json"));
    assert!(!file.exists());
    assert_eq!(
        library.select(&key, Some("absent"), None).missing,
        ["flat", "bad_pixels"]
    );
    assert_eq!(
        library
            .calibrate(&key, &mut sub(10), None, None)
            .report
            .bad_pixels,
        "sub"
    );
    let before = library.masters();
    for id in [flat.id.as_str(), "dark-20260116T020009Z"] {
        for map in [&map, &bad_pixel_map(&worked_dark(), 20, 20)] {
            assert_eq!(
                library.attach_bad_pixels(id, map).unwrap_err().kind(),
                ErrorKind::NotFound
            );
        }
        assert!(!scratch
            .0
            .join(format!("darks/{id}.badpixels.json"))
            .exists());
        assert!(!scratch
            .0
            .join(format!("flats/{id}.badpixels.json"))
            .exists());
    }
    assert_eq!(library.masters(), before);
    assert_eq!(fs::read(scratch.0.join("index.json")).unwrap(), index);
    assert_eq!(fs::read_dir(scratch.0.join("darks")).unwrap().count(), 1);
    assert_eq!(fs::read_dir(scratch.0.join("flats")).unwrap().count(), 1);
}
