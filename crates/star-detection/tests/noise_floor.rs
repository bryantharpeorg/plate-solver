//! US1-S1/S2: cedar parity and worked threshold arithmetic on clipped backgrounds.
use image::ImageReader;
use star_detection::{detect_stars, detect_stars_with_noise_floor};
use std::{collections::BTreeMap, path::Path};

#[test]
fn floors_at_or_below_cedars_floor_preserve_all_reference_stars_in_order() {
    // The existing parity test independently pins detect_stars to cedar's goldens.
    // Here every Star field and its position in the output must be unchanged.
    let images: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(include_str!("data/cedar_goldens_bin1_sigma8.json")).unwrap();
    assert_eq!(images.len(), 13);
    let data = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-detect/test_data");
    for name in images.keys() {
        let image = ImageReader::open(data.join(name))
            .unwrap()
            .decode()
            .unwrap()
            .to_luma8();
        let width = image.width() as usize;
        let height = image.height() as usize;
        for (binning, normalize_rows) in [(1, false), (2, true)] {
            let expected = detect_stars(&image, width, height, 8.0, binning, normalize_rows, true);
            assert!(!expected.is_empty(), "{name}, binning {binning}");
            for floor in [0.0, 0.1, 0.2] {
                let actual = detect_stars_with_noise_floor(
                    &image,
                    width,
                    height,
                    8.0,
                    binning,
                    normalize_rows,
                    true,
                    floor,
                );
                assert_eq!(actual, expected, "{name}, binning {binning}, floor {floor}");
            }
        }
    }
}

fn row_star(levels: [u8; 3]) -> Vec<u8> {
    let mut image = vec![0; 32 * 32];
    image[10 * 32 + 15..10 * 32 + 18].copy_from_slice(&levels);
    image
}

#[test]
fn raised_floor_rejects_faint_peaks_and_retains_the_brighter_centroid() {
    // Cedar's rounded 2*sigma*noise gate sees a doubled central pixel.
    // At 0.7 the gate is 11 and 2*6 = 12 passes; at 1.0 it is 16.
    let faint = row_star([3, 6, 3]);
    assert_eq!(detect_stars(&faint, 32, 32, 8.0, 1, false, false).len(), 1);
    for floor in [0.2, 0.7] {
        assert_eq!(
            detect_stars_with_noise_floor(&faint, 32, 32, 8.0, 1, false, false, floor).len(),
            1,
            "floor {floor}"
        );
    }
    assert!(detect_stars_with_noise_floor(&faint, 32, 32, 8.0, 1, false, false, 1.0).is_empty());

    let bright = row_star([9, 18, 9]);
    let stars = detect_stars_with_noise_floor(&bright, 32, 32, 8.0, 1, false, false, 1.0);
    assert_eq!(stars.len(), 1);
    // Symmetry fixes the centroid at the centre of pixel (16, 10), within 0.1 px.
    assert!((stars[0].x - 16.5).hypot(stars[0].y - 10.5) <= 0.1);
    assert!(detect_stars_with_noise_floor(&bright, 32, 32, 8.0, 1, false, false, 2.5).is_empty());

    // Pin cedar's own 0.2: 2*2 = 4 passes gate 3, but fails gate 5 at 0.3.
    let faintest = row_star([1, 2, 1]);
    assert_eq!(
        detect_stars(&faintest, 32, 32, 8.0, 1, false, false).len(),
        1
    );
    assert_eq!(
        detect_stars_with_noise_floor(&faintest, 32, 32, 8.0, 1, false, false, 0.2).len(),
        1
    );
    assert!(detect_stars_with_noise_floor(&faintest, 32, 32, 8.0, 1, false, false, 0.3).is_empty());
}
