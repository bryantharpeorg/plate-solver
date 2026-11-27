use star_detection::saturated::{
    MAX_AXIS_RATIO, MAX_RING_SHARE, MAX_SATURATED_PIXELS, MIN_PIXELS, RING_MARGIN, SATURATED,
    TRAIL_MIN_REGIONS, TRAIL_TOLERANCE_DEG,
};

#[test]
fn readme_explains_saturated_stars_and_raw_frame_replay() {
    // Oracle: US3-S1's required words, with exact numeric agreement with the
    // detection module's public constants; only whitespace may differ.
    let readme = include_str!("../README.md");
    let (_, section) = readme
        .split_once("\n## Bright stars: saturated stars\n")
        .expect("scope node README needs its saturated stars section");
    let section = section.split("\n## ").next().unwrap();
    let section = section.split_whitespace().collect::<Vec<_>>().join(" ");
    for phrase in [
        format!("its brightest pixels read {SATURATED} or more"),
        format!("at least {MIN_PIXELS} pixels, tetra3's smallest spot"),
        format!("its length at most {MAX_AXIS_RATIO} times its width"),
        format!(
            "which is kept when at least {TRAIL_MIN_REGIONS} of the frame's long regions lie within {TRAIL_TOLERANCE_DEG} degrees of their common direction"
        ),
        format!(
            "the ring {RING_MARGIN} pixels out from it averaging less than {MAX_RING_SHARE} of the way from the frame's median to {SATURATED}"
        ),
        "whole, that ring inside the frame".into(),
        format!("A frame holding more than {MAX_SATURATED_PIXELS} saturated pixels"),
        "placed before the first of cedar-detect's stars that is fainter, so the list stays brightest first".into(),
        "the node keeps the first 200 as before".into(),
        "A star frame does not say which centroids are saturated, nor how many regions were set aside.".into(),
        "The console's Solve page detects an uploaded frame as the node does.".into(),
        "--saturated <frame>".into(),
        "It does not combine with `--dark`".into(),
    ] {
        assert!(section.contains(&phrase), "README section is missing: {phrase}");
    }

    let glossary = include_str!("../../../GLOSSARY.md");
    let definition = format!("A star whose brightest pixels read {SATURATED} or more");
    assert!(glossary
        .lines()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|lines| { lines[0] == "**Saturated star**:" && lines[1].starts_with(&definition) }));

    let example = include_str!("../../plate-solver/examples/solve_image.rs");
    assert!(example.contains("[--noise-floor F] [--saturated] [--stars N]"));
    assert!(example
        .lines()
        .any(|line| line.starts_with("//! * `--saturated` —")));
    assert!(example.contains("--saturated does not combine with --dark"));
}
