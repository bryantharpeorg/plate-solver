//! US3's disk and matching contract. Oracles are the specification's worked
//! examples and FITS bytes read independently here. Numbers are exact unless
//! an assertion explicitly allows the shared setup's 0.001 tolerance.

use std::{fs, io::ErrorKind};

use finder::imaging::{
    calibration::{
        library::{FrameKey, Kind, Library, MasterEntry},
        Mosaic,
    },
    fits::{self, CardValue as C, FitsImage, RealImage},
};
use serde_json::{json, Value};

use crate::support::Scratch;

/// Independent reader shared with the later stories that inspect saved files.
pub(crate) struct ReadFits {
    pub cards: Vec<(String, C)>,
    pub width: u32,
    pub height: u32,
    pub bitpix: i64,
    pub data_start: usize,
    pub samples: Vec<f32>,
}

/// Read 80-byte cards, their 2880-byte padding, and big-endian data. This calls
/// none of the product's FITS readers or header helpers.
pub(crate) fn read_fits(bytes: &[u8]) -> ReadFits {
    let mut cards = Vec::new();
    let mut end = None;
    for (i, card) in bytes.as_chunks::<80>().0.iter().enumerate() {
        let key = std::str::from_utf8(&card[..8]).unwrap().trim().to_owned();
        if key == "END" {
            end = Some((i + 1) * 80);
            break;
        }
        assert_eq!(&card[8..10], b"= ");
        let raw = std::str::from_utf8(&card[10..]).unwrap().trim();
        let value = if raw.starts_with('\'') {
            C::Text(raw[1..raw.len() - 1].trim_end().replace("''", "'"))
        } else {
            match raw {
                "T" => C::Logical(true),
                "F" => C::Logical(false),
                _ if raw.contains(['.', 'E', 'e']) => C::Real(raw.parse().unwrap()),
                _ => C::Int(raw.parse().unwrap()),
            }
        };
        cards.push((key, value));
    }
    let end = end.expect("END card");
    let data_start = end.div_ceil(2880) * 2880;
    assert!(bytes[end..data_start].iter().all(|&c| c == b' '));
    let integer = |key| match cards.iter().find(|(k, _)| k == key).unwrap().1 {
        C::Int(n) => n,
        _ => panic!("{key} is not an integer"),
    };
    let (width, height, bitpix) = (
        integer("NAXIS1") as u32,
        integer("NAXIS2") as u32,
        integer("BITPIX"),
    );
    let count = width as usize * height as usize;
    let data = &bytes[data_start..];
    let samples = match bitpix {
        -32 => data[..count * 4]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|v| f32::from_be_bytes(*v))
            .collect(),
        16 => {
            assert_eq!(integer("BZERO"), 32768);
            assert_eq!(integer("BSCALE"), 1);
            data[..count * 2]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|v| (i32::from(i16::from_be_bytes(*v)) + 32768) as f32)
                .collect()
        }
        _ => panic!("unexpected BITPIX {bitpix}"),
    };
    ReadFits {
        cards,
        width,
        height,
        bitpix,
        data_start,
        samples,
    }
}

fn text(value: &str) -> C {
    C::Text(value.into())
}

fn key() -> FrameKey {
    FrameKey {
        model: "simulated imaging camera".into(),
        serial: Some("SIM-0001".into()),
        width: 4,
        height: 2,
        origin_x: 0,
        origin_y: 0,
        bits: 16,
        bayer: Some("RGGB".into()),
        gain: Some(120.0),
        offset: Some(10.0),
        binning: 1,
        set_temp_c: Some(-10.0),
        temp_c: Some(-10.0),
        exposure_ms: 5000.0,
    }
}

fn entry(id: &str, kind: Kind) -> MasterEntry {
    let k = key();
    MasterEntry {
        id: id.into(),
        kind,
        made_at: 1_768_528_803.7,
        frames: 20,
        checked: true,
        model: k.model,
        serial: k.serial,
        width: k.width,
        height: k.height,
        origin_x: k.origin_x,
        origin_y: k.origin_y,
        bits: k.bits,
        bayer: k.bayer,
        gain: k.gain,
        offset: k.offset,
        binning: k.binning,
        set_temp_c: k.set_temp_c,
        temp_c: k.temp_c,
        exposure_ms: k.exposure_ms,
        level: (kind == Kind::Dark).then_some(160.0),
        bad_pixels: None,
        label: (kind == Kind::Flat).then(|| "desk".into()),
        imaging_angle_deg: (kind == Kind::Flat).then_some(17.0),
        level_share: (kind == Kind::Flat).then_some(0.45),
        dark_frames: (kind == Kind::Flat).then_some(20),
        file: String::new(),
    }
}

fn add(library: &Library, entry: MasterEntry) -> MasterEntry {
    let samples = vec![160.0; entry.width as usize * entry.height as usize];
    library.add(entry, &samples).unwrap()
}

/// US3-S1: the eight cards, size and byte sequence are worked FITS examples.
#[test]
fn real_fits_bytes_round_trip_and_refusals() {
    let image = RealImage {
        width: 2,
        height: 2,
        samples: vec![1.0, -2.5, 0.0, 160.25],
        cards: vec![
            ("IMAGETYP".into(), text("MASTER DARK")),
            ("NFRAMES".into(), C::Int(4)),
        ],
    };
    let bytes = fits::encode_real(&image);
    assert_eq!(bytes.len(), 5760);
    let read = read_fits(&bytes);
    assert_eq!(
        (read.width, read.height, read.bitpix, read.data_start),
        (2, 2, -32, 2880)
    );
    assert_eq!(
        read.cards,
        vec![
            ("SIMPLE".into(), C::Logical(true)),
            ("BITPIX".into(), C::Int(-32)),
            ("NAXIS".into(), C::Int(2)),
            ("NAXIS1".into(), C::Int(2)),
            ("NAXIS2".into(), C::Int(2)),
            ("IMAGETYP".into(), text("MASTER DARK")),
            ("NFRAMES".into(), C::Int(4)),
        ]
    );
    assert_eq!(&bytes[560..640], &[b"END".as_slice(), &[b' '; 77]].concat());
    assert_eq!(
        &bytes[2880..2896],
        &[0x3f, 0x80, 0, 0, 0xc0, 0x20, 0, 0, 0, 0, 0, 0, 0x43, 0x20, 0x40, 0]
    );
    assert!(bytes[2896..].iter().all(|&b| b == 0));
    assert_eq!(read.samples, image.samples);
    assert_eq!(fits::decode_real(&bytes).unwrap(), image);
    let integer = FitsImage {
        width: 2,
        height: 2,
        bits: 16,
        samples: [0u16, 1, 32768, 65535]
            .into_iter()
            .flat_map(u16::to_le_bytes)
            .collect(),
        cards: vec![],
    };
    let encoded = fits::encode(&integer);
    assert_eq!(
        read_fits(&encoded).samples,
        vec![0.0, 1.0, 32768.0, 65535.0]
    );
    assert_eq!(
        fits::decode_real(&encoded).unwrap_err().to_string(),
        "not a 32-bit real FITS image: BITPIX 16"
    );
    assert_eq!(
        fits::decode_real(&bytes[..2888]).unwrap_err().to_string(),
        "the FITS data is short: 8 of 16 bytes"
    );
    assert_eq!(
        fits::decode(&bytes).unwrap_err().to_string(),
        "unsupported FITS BITPIX -32"
    );
    assert_eq!(
        fits::decode_real(b"bad").unwrap_err().to_string(),
        "not a FITS image"
    );
    let mut bad = bytes.clone();
    bad[..6].copy_from_slice(b"BROKEN");
    assert_eq!(
        fits::decode_real(&bad).unwrap_err().to_string(),
        "not a FITS image"
    );
    bad = bytes.clone();
    bad[560..640].fill(b' ');
    assert_eq!(
        fits::decode_real(&bad).unwrap_err().to_string(),
        "not a FITS image"
    );
    bad = bytes.clone();
    bad[2 * 80 + 29] = b'3';
    bad[80 + 28..80 + 30].copy_from_slice(b"16");
    assert_eq!(
        fits::decode_real(&bad).unwrap_err().to_string(),
        "a 3-axis FITS file is not one image"
    );
}

/// US3-S2: the cards and JSON object are declared disk format, independently read.
#[test]
fn library_persists_cards_index_and_unique_ids_without_replacing() {
    let scratch = Scratch::new("calibration-library-files");
    let root = scratch.0.join("calibration");
    let library = Library::open(root.clone());
    assert!(library.masters().is_empty());
    assert_eq!(library.error(), None);
    let id = library.new_id(Kind::Dark, 1_768_528_803.7);
    assert_eq!(id, "dark-20260116T020003Z");
    let samples = [1.0, -2.5, 0.0, 160.25, 150.0, 160.0, 170.0, 190.0];
    let saved = library.add(entry(&id, Kind::Dark), &samples).unwrap();
    assert_eq!(saved.file, "darks/dark-20260116T020003Z.fits");
    assert!(saved.checked);
    assert_eq!(saved.bad_pixels, None);
    let bytes = fs::read(root.join(&saved.file)).unwrap();
    assert_eq!(bytes.len(), 5760);
    let read = read_fits(&bytes);
    assert_eq!((read.width, read.height), (4, 2));
    assert_eq!(read.samples, samples);
    assert_eq!(
        &read.cards[5..],
        &[
            ("IMAGETYP".into(), text("MASTER DARK")),
            ("CALID".into(), text(&id)),
            ("NFRAMES".into(), C::Int(20)),
            ("DARKTEST".into(), C::Logical(true)),
            ("EXPTIME".into(), C::Real(5.0)),
            ("GAIN".into(), C::Real(120.0)),
            ("OFFSET".into(), C::Real(10.0)),
            ("XBINNING".into(), C::Int(1)),
            ("YBINNING".into(), C::Int(1)),
            ("XORGSUBF".into(), C::Int(0)),
            ("YORGSUBF".into(), C::Int(0)),
            ("CCD-TEMP".into(), C::Real(-10.0)),
            ("SET-TEMP".into(), C::Real(-10.0)),
            ("BAYERPAT".into(), text("RGGB")),
            ("INSTRUME".into(), text("simulated imaging camera")),
            ("SERIALNO".into(), text("SIM-0001")),
            ("ROWORDER".into(), text("TOP-DOWN")),
            ("DATE".into(), text("2026-01-16T02:00:03.700")),
        ]
    );
    let index: Value = serde_json::from_slice(&fs::read(root.join("index.json")).unwrap()).unwrap();
    assert_eq!(index["format"], "finder-calibration/1");
    assert_eq!(
        index["masters"],
        json!([serde_json::to_value(&saved).unwrap()])
    );
    for folder in [&root, &root.join("darks")] {
        assert!(fs::read_dir(folder).unwrap().all(|f| !f
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".part")));
    }
    let reopened = Library::open(root.clone());
    assert_eq!(reopened.masters(), library.masters());
    assert_eq!(
        reopened.new_id(Kind::Dark, 1_768_528_803.2),
        "dark-20260116T020003Z-2"
    );
    assert_eq!(
        reopened
            .add(entry(&id, Kind::Dark), &[0.0; 8])
            .unwrap_err()
            .kind(),
        ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read(root.join(&saved.file)).unwrap(), bytes);
    assert_eq!(reopened.masters(), vec![saved]);
    // A leftover final file also reserves its id, even without an index entry.
    fs::write(root.join("darks/dark-20260116T020003Z-2.fits"), b"reserved").unwrap();
    assert_eq!(
        reopened.new_id(Kind::Dark, 1_768_528_803.999_9),
        "dark-20260116T020003Z-3"
    );
    let collision = reopened
        .add(entry("dark-20260116T020003Z-2", Kind::Dark), &samples)
        .unwrap_err();
    assert_eq!(collision.kind(), ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read(root.join("darks/dark-20260116T020003Z-2.fits")).unwrap(),
        b"reserved"
    );
    assert!(!root
        .join("darks/dark-20260116T020003Z-2.fits.part")
        .exists());
    assert_eq!(reopened.masters().len(), 1);
}

fn change(k: &FrameKey, edit: impl FnOnce(&mut FrameKey)) -> FrameKey {
    let mut changed = k.clone();
    edit(&mut changed);
    changed
}

/// US3-S3: boundary values and nearest-temperature/newest tie breaks are the oracle.
#[test]
fn dark_matching_and_agreement_hold_the_declared_settings() {
    let scratch = Scratch::new("calibration-library-darks");
    let library = Library::open(scratch.0.join("calibration"));
    add(&library, entry("A", Kind::Dark));
    let k = key();
    let selected = library.select(&k, None, None);
    assert_eq!(selected.dark.as_deref(), Some("A"));
    assert_eq!(selected.flat, None);
    assert_eq!(selected.bad_pixels, None);
    assert_eq!(selected.missing, ["flat", "bad_pixels"]);
    assert_eq!(selected.frame, None);
    for changed in [
        change(&k, |k| k.set_temp_c = Some(-10.9)),
        change(&k, |k| k.exposure_ms = 5000.9),
        change(&k, |k| k.gain = Some(120.0005)),
        change(&k, |k| k.offset = Some(10.0005)),
    ] {
        assert_eq!(
            library.select(&changed, None, None).dark.as_deref(),
            Some("A")
        );
    }
    let incompatible = [
        change(&k, |k| k.set_temp_c = Some(-11.1)),
        change(&k, |k| k.set_temp_c = None),
        change(&k, |k| k.gain = Some(121.0)),
        change(&k, |k| k.gain = None),
        change(&k, |k| k.offset = Some(11.0)),
        change(&k, |k| k.binning = 2),
        change(&k, |k| k.exposure_ms = 5001.1),
        change(&k, |k| k.serial = None),
        change(&k, |k| k.serial = Some("SIM-0002".into())),
        change(&k, |k| k.model = "another camera".into()),
        change(&k, |k| k.width = 6),
        change(&k, |k| k.origin_x = 2),
        change(&k, |k| k.bayer = None),
        change(&k, |k| k.bits = 8),
        change(&k, |k| k.height = 4),
        change(&k, |k| k.origin_y = 2),
    ];
    for changed in &incompatible {
        let selected = library.select(changed, None, None);
        assert_eq!(selected.dark, None, "{changed:?}");
        assert_eq!(selected.missing, ["dark", "flat", "bad_pixels"]);
    }
    let mut b = entry("B", Kind::Dark);
    b.set_temp_c = None;
    b.temp_c = Some(20.0);
    add(&library, b);
    let warm = change(&k, |k| {
        k.set_temp_c = None;
        k.temp_c = Some(27.9);
    });
    assert_eq!(library.select(&warm, None, None).dark.as_deref(), Some("B"));
    assert_eq!(
        library
            .select(&change(&warm, |k| k.temp_c = Some(28.1)), None, None)
            .dark,
        None
    );
    assert_eq!(
        library
            .select(&change(&warm, |k| k.temp_c = None), None, None)
            .dark
            .as_deref(),
        Some("B")
    );
    let mut a2 = entry("A2", Kind::Dark);
    a2.set_temp_c = Some(-10.5);
    a2.made_at += 1000.0;
    add(&library, a2);
    let mut a3 = entry("A3", Kind::Dark);
    a3.made_at += 2000.0;
    add(&library, a3);
    assert_eq!(library.select(&k, None, None).dark.as_deref(), Some("A3"));
    assert_eq!(
        library
            .select(&change(&k, |k| k.set_temp_c = Some(-10.4)), None, None)
            .dark
            .as_deref(),
        Some("A2")
    );
    assert!(k.agrees(&k, 5000.0));
    assert!(change(&k, |k| k.temp_c = Some(3.0)).agrees(&k, 5000.0));
    assert!(k.agrees(&k, 5000.9));
    assert!(!k.agrees(&k, 5001.1));
    for changed in incompatible.iter().filter(|c| c.set_temp_c != Some(-11.1)) {
        assert!(!changed.agrees(&k, 5000.0), "{changed:?}");
    }
    assert!(!change(&k, |k| k.set_temp_c = Some(-10.5)).agrees(&k, 5000.0));
    let blank = FitsImage {
        width: 4,
        height: 2,
        bits: 16,
        samples: vec![0; 16],
        cards: vec![],
    };
    assert_eq!(
        FrameKey::from_image(&blank).unwrap_err(),
        "the imaging frame's file has no INSTRUME card"
    );
}

/// US3-S4: flats ignore the five photometric settings, with a strict one-degree
/// warning threshold and the shorter circular angle difference (0.001 tolerance).
#[test]
fn flat_matching_labels_and_camera_turns() {
    let scratch = Scratch::new("calibration-library-flats");
    let library = Library::open(scratch.0.join("calibration"));
    let f1 = add(&library, entry("F1", Kind::Flat));
    let k = key();
    for changed in [
        k.clone(),
        change(&k, |k| k.exposure_ms = 2.0),
        change(&k, |k| k.temp_c = Some(20.0)),
        change(&k, |k| k.gain = Some(121.0)),
        change(&k, |k| k.gain = None),
        change(&k, |k| k.offset = Some(11.0)),
        change(&k, |k| k.set_temp_c = Some(-11.1)),
        change(&k, |k| k.set_temp_c = None),
    ] {
        assert_eq!(
            library.select(&changed, None, None).flat.as_deref(),
            Some("F1")
        );
    }
    for changed in [
        change(&k, |k| k.binning = 2),
        change(&k, |k| k.serial = None),
        change(&k, |k| k.serial = Some("SIM-0002".into())),
        change(&k, |k| k.model = "another camera".into()),
        change(&k, |k| k.width = 6),
        change(&k, |k| k.height = 4),
        change(&k, |k| k.origin_x = 2),
        change(&k, |k| k.origin_y = 2),
        change(&k, |k| k.bayer = None),
        change(&k, |k| k.bits = 8),
    ] {
        assert_eq!(
            library.select(&changed, None, None).flat,
            None,
            "{changed:?}"
        );
    }
    for angle in [None, Some(17.9), Some(18.0)] {
        assert_eq!(library.select(&k, None, angle).flat_turned_deg, None);
    }
    let turned = library.select(&k, None, Some(19.5));
    assert_eq!(turned.flat.as_deref(), Some("F1"));
    assert_eq!(turned.flat_turned_deg, Some(2.5));
    let mut f2 = entry("F2", Kind::Flat);
    f2.label = Some("corrector".into());
    f2.gain = Some(0.0);
    f2.made_at += 1000.0;
    add(&library, f2);
    assert_eq!(library.select(&k, None, None).flat.as_deref(), Some("F2"));
    assert_eq!(
        library.select(&k, Some("desk"), None).flat.as_deref(),
        Some("F1")
    );
    assert_eq!(library.select(&k, Some("other"), None).flat, None);
    let library = Library::open(scratch.0.join("cooler-off"));
    let mut f3 = entry("F3", Kind::Flat);
    f3.set_temp_c = None;
    f3.temp_c = Some(20.0);
    f3.imaging_angle_deg = None;
    add(&library, f3);
    for changed in [
        k.clone(),
        change(&k, |k| {
            k.set_temp_c = None;
            k.temp_c = Some(35.0);
        }),
    ] {
        assert_eq!(
            library.select(&changed, None, Some(50.0)).flat.as_deref(),
            Some("F3")
        );
        assert_eq!(
            library.select(&changed, None, Some(50.0)).flat_turned_deg,
            None
        );
    }
    let library = Library::open(scratch.0.join("wrapped"));
    let mut wrapped = f1;
    wrapped.imaging_angle_deg = Some(179.5);
    add(&library, wrapped);
    assert_eq!(library.select(&k, None, Some(-179.6)).flat_turned_deg, None);
    assert!(
        (library
            .select(&k, None, Some(-178.4))
            .flat_turned_deg
            .unwrap()
            - 2.1)
            .abs()
            <= 0.001
    );
}

fn sub() -> Mosaic {
    let mut samples = vec![560.0; 16];
    samples[10] = 1560.0;
    Mosaic {
        width: 4,
        height: 4,
        bayer: Some("RGGB".into()),
        samples,
    }
}

/// US3-S5: (560 - 160) / 0.5 = 800, including the dark's hot sample.
/// Without masters, two red neighbours repair the isolated 1560 to 560.
#[test]
fn calibration_reads_masters_repairs_without_them_and_reports_unusable_files() {
    let scratch = Scratch::new("calibration-library-apply");
    let root = scratch.0.join("calibration");
    let library = Library::open(root.clone());
    let k = change(&key(), |k| k.height = 4);
    let mut dark = entry("A", Kind::Dark);
    dark.height = 4;
    let mut samples = vec![160.0; 16];
    samples[10] = 1160.0;
    let dark = library.add(dark, &samples).unwrap();
    let mut flat = entry("F1", Kind::Flat);
    flat.height = 4;
    library.add(flat, &[0.5; 16]).unwrap();
    let mut image = sub();
    let applied = library.calibrate(&k, &mut image, None, None);
    assert_eq!(image.samples, vec![800.0; 16]);
    assert_eq!(applied.selection.dark.as_deref(), Some("A"));
    assert_eq!(applied.selection.flat.as_deref(), Some("F1"));
    assert_eq!(
        (
            applied.report.dark,
            applied.report.flat,
            applied.report.bad_pixels,
            applied.report.repaired
        ),
        ("applied", "applied", "sub", 0)
    );
    assert!(applied.problems.is_empty());
    // Once read, the same masters serve a run without rereading their files.
    fs::remove_file(root.join(&dark.file)).unwrap();
    let mut image = sub();
    assert!(library
        .calibrate(&k, &mut image, None, None)
        .problems
        .is_empty());
    assert_eq!(image.samples, vec![800.0; 16]);
    let empty = Library::open(scratch.0.join("empty"));
    let mut image = sub();
    let applied = empty.calibrate(&k, &mut image, None, None);
    assert_eq!(image.samples, vec![560.0; 16]);
    assert_eq!(
        (
            applied.report.dark,
            applied.report.flat,
            applied.report.bad_pixels,
            applied.report.repaired
        ),
        ("missing", "missing", "sub", 1)
    );
    assert_eq!(applied.selection.missing, ["dark", "flat", "bad_pixels"]);
    assert!(applied.problems.is_empty());
    let missing = Library::open(root.clone());
    let mut image = sub();
    let applied = missing.calibrate(&k, &mut image, None, None);
    assert_eq!(
        (applied.report.dark, applied.report.flat),
        ("missing", "applied")
    );
    assert_eq!(applied.selection.dark.as_deref(), Some("A"));
    assert_eq!(applied.problems.len(), 1);
    assert!(applied.problems[0].starts_with("the master dark "));
    assert!(applied.problems[0].contains(" could not be used: "));
    fs::write(
        root.join(&dark.file),
        fits::encode_real(&RealImage {
            width: 2,
            height: 2,
            samples: vec![160.0; 4],
            cards: vec![],
        }),
    )
    .unwrap();
    let wrong_size = Library::open(root);
    let mut image = sub();
    let applied = wrong_size.calibrate(&k, &mut image, None, None);
    assert_eq!(
        (applied.report.dark, applied.report.flat),
        ("missing", "applied")
    );
    assert_eq!(
        applied.problems,
        ["the master dark A could not be used: it is 2 × 2 and the sub 4 × 4"]
    );
}

/// US3-S6: removal updates both files and index; corrupt index bytes survive
/// the next write verbatim, while a future format is explicitly refused.
#[test]
fn removal_and_unreadable_index_recovery() {
    let scratch = Scratch::new("calibration-library-index");
    let root = scratch.0.join("calibration");
    let library = Library::open(root.clone());
    let dark = add(&library, entry("A", Kind::Dark));
    let flat = add(&library, entry("F1", Kind::Flat));
    assert!(library.remove("A").unwrap());
    assert!(!root.join(&dark.file).exists());
    assert_eq!(library.masters(), vec![flat.clone()]);
    let index: Value = serde_json::from_slice(&fs::read(root.join("index.json")).unwrap()).unwrap();
    assert_eq!(index["masters"], json!([flat]));
    assert!(!library.remove("A").unwrap());
    fs::write(root.join("index.json"), "not json").unwrap();
    fs::write(root.join("index.json.unreadable"), "older unreadable index").unwrap();
    let library = Library::open(root.clone());
    assert!(library.masters().is_empty());
    assert!(library
        .error()
        .unwrap()
        .starts_with("the calibration index could not be read: "));
    add(&library, entry("replacement", Kind::Dark));
    assert_eq!(
        fs::read_to_string(root.join("index.json.unreadable")).unwrap(),
        "not json"
    );
    let index: Value = serde_json::from_slice(&fs::read(root.join("index.json")).unwrap()).unwrap();
    assert_eq!(index["masters"].as_array().unwrap().len(), 1);
    assert_eq!(library.error(), None);
    fs::write(
        root.join("index.json"),
        r#"{"format":"finder-calibration/2","masters":[]}"#,
    )
    .unwrap();
    let library = Library::open(root.clone());
    assert!(library.masters().is_empty());
    assert_eq!(
        library.error().as_deref(),
        Some("the calibration index could not be read: its format is finder-calibration/2")
    );
    fs::write(
        root.join("index.json"),
        r#"{"format":"finder-calibration/1","masters":[{}]}"#,
    )
    .unwrap();
    let library = Library::open(root);
    assert!(library.masters().is_empty());
    assert!(library.error().unwrap().contains("missing field"));
}

/// The required-card order is declared in the plan. Optional values stay absent;
/// EXPTIME is converted from seconds, and XBINNING is the single binning key.
#[test]
fn frame_key_reads_required_and_optional_cards() {
    let mut image = FitsImage {
        width: 4,
        height: 2,
        bits: 16,
        samples: vec![0; 16],
        cards: vec![],
    };
    for (name, value) in [
        ("INSTRUME", text("simulated imaging camera")),
        ("EXPTIME", C::Real(5.0)),
        ("XBINNING", C::Int(1)),
        ("XORGSUBF", C::Int(0)),
        ("YORGSUBF", C::Int(0)),
    ] {
        assert_eq!(
            FrameKey::from_image(&image).unwrap_err(),
            format!("the imaging frame's file has no {name} card")
        );
        image.cards.push((name.into(), value));
    }
    let expected = change(&key(), |k| {
        k.serial = None;
        k.bayer = None;
        k.gain = None;
        k.offset = None;
        k.temp_c = None;
        k.set_temp_c = None;
    });
    assert_eq!(FrameKey::from_image(&image).unwrap(), expected);
    image.cards.extend([
        ("SERIALNO".into(), text("SIM-0001")),
        ("GAIN".into(), C::Real(120.0)),
        ("OFFSET".into(), C::Int(10)),
        ("SET-TEMP".into(), C::Real(-10.0)),
        ("CCD-TEMP".into(), C::Real(-10.0)),
        ("BAYERPAT".into(), text("RGGB")),
    ]);
    assert_eq!(FrameKey::from_image(&image).unwrap(), key());
    assert_eq!(
        FrameKey::from_view(&finder::imaging::ImagingView::off()),
        None
    );
}

/// The disk contract also fixes flat-only cards, null keys, deterministic id
/// ties and the selected dark's map count, before maps are implemented in US5.
#[test]
fn flat_cards_optional_values_and_equal_time_ties() {
    let scratch = Scratch::new("calibration-library-ties");
    let library = Library::open(scratch.0.clone());
    assert_eq!(
        library.new_id(Kind::Flat, 1_768_528_803.7),
        "flat-20260116T020003Z"
    );
    let mut flat = entry("F2", Kind::Flat);
    flat.checked = false;
    flat.gain = None;
    flat.offset = None;
    flat.temp_c = None;
    flat.set_temp_c = None;
    flat.bayer = None;
    flat.serial = None;
    flat.imaging_angle_deg = None;
    let flat = add(&library, flat);
    let read = read_fits(&fs::read(scratch.0.join(&flat.file)).unwrap());
    assert_eq!(
        &read.cards[5..],
        &[
            ("IMAGETYP".into(), text("MASTER FLAT")),
            ("CALID".into(), text("F2")),
            ("NFRAMES".into(), C::Int(20)),
            ("NDARKS".into(), C::Int(20)),
            ("DARKTEST".into(), C::Logical(false)),
            ("EXPTIME".into(), C::Real(5.0)),
            ("XBINNING".into(), C::Int(1)),
            ("YBINNING".into(), C::Int(1)),
            ("XORGSUBF".into(), C::Int(0)),
            ("YORGSUBF".into(), C::Int(0)),
            ("INSTRUME".into(), text("simulated imaging camera")),
            ("ROWORDER".into(), text("TOP-DOWN")),
            ("DATE".into(), text("2026-01-16T02:00:03.700")),
            ("FLATLBL".into(), text("desk")),
        ]
    );
    let index: Value =
        serde_json::from_slice(&fs::read(scratch.0.join("index.json")).unwrap()).unwrap();
    let fields = index["masters"][0].as_object().unwrap();
    assert_eq!(fields.len(), 26);
    for field in [
        "gain",
        "offset",
        "temp_c",
        "set_temp_c",
        "bayer",
        "serial",
        "imaging_angle_deg",
        "level",
        "bad_pixels",
    ] {
        assert_eq!(fields.get(field), Some(&Value::Null), "{field}");
    }
    let k = change(&key(), |k| {
        k.serial = None;
        k.bayer = None;
    });
    let mut f1 = flat;
    f1.id = "F1".into();
    add(&library, f1);
    assert_eq!(library.select(&k, None, None).flat.as_deref(), Some("F2"));
    let mut a2 = entry("A2", Kind::Dark);
    a2.bad_pixels = Some(3);
    add(&library, a2);
    add(&library, entry("A1", Kind::Dark));
    let selected = library.select(&key(), None, None);
    assert_eq!(selected.dark.as_deref(), Some("A2"));
    assert_eq!(selected.bad_pixels, Some(3));
    assert_eq!(selected.missing, ["flat"]);
    assert_eq!(
        library
            .masters()
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        ["A1", "A2", "F1", "F2"]
    );
    let angle_flat = add(&library, entry("angled", Kind::Flat));
    let read = read_fits(&fs::read(scratch.0.join(angle_flat.file)).unwrap());
    assert_eq!(read.cards.last(), Some(&("IMGANGLE".into(), C::Real(17.0))));
}
