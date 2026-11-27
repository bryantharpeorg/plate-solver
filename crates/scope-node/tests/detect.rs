use plate_solver::DetectParams;
use scope_node::{
    detect::{self, DetectSettings},
    GrayFrame,
};
use serde_json::Value;
use std::path::PathBuf;

#[test]
fn cedar_defaults_equal_every_console_field() {
    let node = DetectSettings::cedar();
    let console = DetectParams::cedar();
    assert_eq!(node.sigma, console.sigma);
    assert_eq!(node.noise_estimate, console.noise_estimate);
    assert_eq!(node.binning, console.binning);
    assert_eq!(node.normalize_rows, console.normalize_rows);
    assert_eq!(node.detect_hot_pixels, console.detect_hot_pixels);
    assert_eq!(node.return_binned, console.return_binned);
    assert_eq!(
        node.use_binned_for_star_candidates,
        console.use_binned_for_star_candidates
    );
}

#[test]
fn saved_finder_frame_keeps_its_saturated_star_first_then_the_consoles_centroids_in_order() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/scope-node");
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(root.join("fixtures.json")).unwrap()).unwrap();
    let frame = &fixture["finder_frame"];
    let image = image::open(root.join(frame["file"].as_str().unwrap()))
        .unwrap()
        .to_luma8();
    assert_eq!(image.width() as u64, frame["width"].as_u64().unwrap());
    assert_eq!(image.height() as u64, frame["height"].as_u64().unwrap());
    let gray = GrayFrame {
        width: image.width(),
        height: image.height(),
        pixels: image.into_raw(),
        start_s: 0.0,
        end_s: 0.02,
        exposure_ms: 20.0,
        gain: 1.0,
    };
    // Independent oracle: the saved frame's 29 saturated pixels, as (x, y).
    let saturated: Vec<_> = gray
        .pixels
        .iter()
        .enumerate()
        .filter(|(_, value)| **value >= 250)
        .map(|(i, _)| (i % gray.width as usize, i / gray.width as usize))
        .collect();
    let expected: Vec<_> = [
        (186, 20, 20),
        (187, 19, 24),
        (188, 19, 24),
        (189, 19, 25),
        (190, 20, 24),
        (191, 21, 24),
    ]
    .into_iter()
    .flat_map(|(y, left, right)| (left..=right).map(move |x| (x, y)))
    .collect();
    assert_eq!(saturated, expected);
    let settings = DetectParams::cedar();
    let console = star_detection::detect_stars(
        &gray.pixels,
        gray.width as usize,
        gray.height as usize,
        settings.sigma,
        settings.binning,
        settings.normalize_rows,
        settings.detect_hot_pixels,
    );
    let node = detect::detect(&gray, &DetectSettings::cedar());
    assert!(
        !node.is_empty(),
        "the saved finder frame must exercise detection"
    );
    assert!(
        !console.is_empty(),
        "cedar must still detect the saved frame"
    );
    assert!(console
        .iter()
        .all(|s| (s.x - 22.5).hypot(s.y - 189.5) >= 5.0));
    assert_eq!(node.len(), console.len() + 1);
    // Centre of mass of the 29 pixels above, with the pixel-centre convention.
    assert!((node[0].x - (632.0 / 29.0 + 0.5)).abs() < 1e-9);
    assert!((node[0].y - (5473.0 / 29.0 + 0.5)).abs() < 1e-9);
    for (a, b) in node.iter().skip(1).zip(console) {
        // Exact equality: cedar's centroids retain their values and order.
        assert_eq!((a.x, a.y, a.brightness), (b.x, b.y, b.brightness));
    }
}

#[test]
fn crowded_frame_keeps_the_detectors_brightest_two_hundred_in_order() {
    let mut frame = GrayFrame {
        width: 1600,
        height: 1200,
        pixels: vec![8; 1600 * 1200],
        start_s: 0.0,
        end_s: 0.02,
        exposure_ms: 20.0,
        gain: 1.0,
    };
    // Independently specified round PSFs on a separated grid, without a catalog.
    for y in (32..1200).step_by(64) {
        for x in (32..1600).step_by(64) {
            for dy in -5i32..=5 {
                for dx in -5i32..=5 {
                    frame.pixels[((y + dy) * 1600 + x + dx) as usize] =
                        (8.0 + 100.0 * (-f64::from(dx * dx + dy * dy) / 3.92).exp()) as u8;
                }
            }
        }
    }
    let all = star_detection::detect_stars(&frame.pixels, 1600, 1200, 8.0, 2, true, true);
    assert!(
        all.len() > 200,
        "only {} stars in the test input",
        all.len()
    );
    let node = detect::detect(&frame, &DetectSettings::cedar());
    assert_eq!(node.len(), 200);
    for (a, b) in node.iter().zip(all) {
        assert_eq!((a.x, a.y, a.brightness), (b.x, b.y, b.brightness));
    }
}
