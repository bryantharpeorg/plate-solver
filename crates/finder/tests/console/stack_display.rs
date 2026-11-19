//! US7: worked sums, ramps and asinh bytes are independent display oracles.
//! JPEG colours allow six counts for compression; all tone bytes are exact.

use finder::imaging::{
    preview::{self, Rgb},
    stack::{
        display::{self, Plane, Reduced},
        service::StartRequest,
        Colour, Placement, Stack,
    },
};
use std::time::Duration;

use crate::stack_loop::{stand_in_rig, wait_stack, ScriptedSolver};

fn reduced(width: u32, height: u32, value: impl Fn(u32, u32) -> [f32; 3]) -> Reduced {
    let mut colour = Colour::new(width, height);
    for y in 0..height {
        for x in 0..width {
            for (channel, sample) in value(x, y).into_iter().enumerate() {
                colour.set(channel, x, y, sample);
            }
        }
    }
    Reduced {
        colour,
        covered: vec![true; (width * height) as usize],
    }
}

fn at(picture: &Rgb, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * picture.width + x) * 3) as usize;
    picture.pixels[i..i + 3].try_into().unwrap()
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} differs from {expected} by more than {tolerance}"
    );
}

/// US7-S1: whole blocks drop the tail, and divide summed sums by summed weights.
#[test]
fn reduce_whole_blocks_and_sample_weights() {
    for (width, height, expected) in [
        (1920, 1, (1920, 1)),
        (1921, 5, (960, 2)),
        (3841, 6, (1280, 2)),
    ] {
        let size = (width * height) as usize;
        let picture = display::reduce(&vec![7.0; size * 3], &vec![1.0; size], width, height, 1920);
        assert_eq!((picture.colour.width, picture.colour.height), expected);
        assert!(picture.covered.iter().all(|covered| *covered));
        assert!(picture.colour.planes.iter().all(|v| *v == 7.0));
    }
    let mut sums = [10.0, 20.0, 30.0, 0.0, 50.0, 60.0, 0.0, 0.0].repeat(3);
    let mut weights = vec![1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0];
    let picture = display::reduce(&sums, &weights, 4, 2, 2);
    assert_eq!((picture.colour.width, picture.colour.height), (2, 1));
    assert_eq!(picture.covered, [true, true]);
    assert_eq!(picture.colour.planes, [35.0, 30.0].repeat(3));
    for channel in 0..3 {
        sums[channel * 8 + 2] = 0.0;
    }
    weights[2] = 0.0;
    let picture = display::reduce(&sums, &weights, 4, 2, 2);
    assert_eq!(picture.covered, [true, false]);
    assert_eq!(picture.colour.planes, [35.0, 0.0].repeat(3));

    // 100/5 = 20; averaging the three pixel means would incorrectly give 26.667.
    let sums = [30.0, 20.0, 0.0, 50.0].repeat(3);
    let picture = display::reduce(&sums, &[3.0, 1.0, 0.0, 1.0], 2, 2, 1);
    assert_eq!((picture.colour.width, picture.colour.height), (1, 1));
    assert_eq!(picture.covered, [true]);
    assert_eq!(picture.colour.planes, [20.0; 3]);
}

fn ramp() -> Reduced {
    reduced(800, 600, |x, y| {
        let (x, y) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
        [
            (100.0 + 0.1 * x + 0.05 * y) as f32,
            (200.0 - 0.02 * x) as f32,
            50.0,
        ]
    })
}

/// US7-S2: fit the darker half of tile lower medians, with explicit fallbacks.
#[test]
fn background_ramps_bright_square_and_sparse_coverage() {
    let mut picture = ramp();
    let planes = display::background(&picture);
    for (plane, [a, b, c], c_tolerance) in [
        (planes[0], [0.1, 0.05, 100.0], 0.1),
        (planes[1], [-0.02, 0.0, 199.99], 0.002),
        (planes[2], [0.0, 0.0, 50.0], 0.00001),
    ] {
        close(plane.a, a, 0.0005);
        close(plane.b, b, 0.0005);
        close(plane.c, c, c_tolerance);
    }
    let bright = reduced(800, 600, |x, y| {
        [if (300..500).contains(&x) && (225..375).contains(&y) {
            550.0
        } else {
            50.0
        }; 3]
    });
    for plane in display::background(&bright) {
        close(plane.a, 0.0, 0.01);
        close(plane.b, 0.0, 0.01);
        close(plane.c, 50.0, 0.01);
    }
    for (i, covered) in picture.covered.iter_mut().enumerate() {
        *covered = i / 800 < 75;
    }
    for plane in display::background(&picture) {
        assert_eq!((plane.a, plane.b), (0.0, 0.0));
    }
    for (i, covered) in picture.covered.iter_mut().enumerate() {
        *covered = i / 800 < 37;
    }
    for (plane, level) in display::background(&picture)
        .into_iter()
        .zip([140.925, 191.99, 50.0])
    {
        assert_eq!((plane.a, plane.b), (0.0, 0.0));
        close(plane.c, level, 0.01);
    }
    for plane in display::background(&reduced(4, 4, |_, _| [7.0; 3])) {
        assert_eq!(
            plane,
            Plane {
                a: 0.0,
                b: 0.0,
                c: 7.0
            }
        );
    }
    picture.covered.fill(false);
    assert_eq!(
        display::background(&picture),
        [Plane {
            a: 0.0,
            b: 0.0,
            c: 0.0
        }; 3]
    );
}

fn tone_example() -> Reduced {
    reduced(3, 3, |x, y| {
        let i = (3 * y + x) as usize;
        [
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 300.0, 600.0, 800.0][i],
            0.0,
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 150.0, 300.0][i],
        ]
    })
}

/// US7-S3: span 600 gives round(255 asinh(5)/asinh(10)) = 197 at red 300.
#[test]
fn tone_gains_shared_span_noise_and_planes() {
    let flat = [Plane {
        a: 0.0,
        b: 0.0,
        c: 0.0,
    }; 3];
    let picture = tone_example();
    let expected = [
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
        [197, 0, 0],
        [255, 0, 140],
        [255, 0, 197],
    ]
    .concat();
    assert_eq!(
        display::tone(&picture, &flat, [1.0; 3], 10.0).pixels,
        expected
    );
    let gained = display::tone(&picture, &flat, [1.0, 1.0, 2.0], 10.0);
    assert_eq!(at(&gained, 1, 2), [255, 0, 197]);
    assert_eq!(at(&gained, 2, 2), [255, 0, 255]);
    assert_eq!(
        at(&display::tone(&picture, &flat, [1.0; 3], 1.0), 0, 2),
        [139, 0, 0]
    );
    let mut raised = picture.clone();
    for sample in &mut raised.colour.planes[..9] {
        *sample += 100.0;
    }
    assert_eq!(
        display::tone(&raised, &flat, [1.0; 3], 10.0).pixels,
        expected
    );
    let noisy = reduced(3, 3, |x, y| {
        [
            [0.0, 10.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 300.0][(3 * y + x) as usize],
            0.0,
            0.0,
        ]
    });
    assert_eq!(
        at(&display::tone(&noisy, &flat, [1.0; 3], 10.0), 2, 2),
        [249, 0, 0]
    );
    let mut tilted = picture.clone();
    for y in 0..3 {
        for x in 0..3 {
            tilted.colour.set(
                0,
                x,
                y,
                picture.colour.at(0, x, y) + 10.0 * (x as f32 + 0.5),
            );
        }
    }
    let mut planes = flat;
    planes[0].a = 10.0;
    assert_eq!(
        display::tone(&tilted, &planes, [1.0; 3], 10.0).pixels,
        expected
    );
    let mut uncovered = picture;
    uncovered.covered[0] = false;
    for channel in 0..3 {
        uncovered.colour.set(channel, 0, 0, 5000.0);
    }
    assert_eq!(
        display::tone(&uncovered, &flat, [1.0; 3], 10.0).pixels,
        expected
    );
    uncovered.covered.fill(false);
    assert_eq!(
        display::tone(&uncovered, &flat, [1.0; 3], 10.0).pixels,
        [0; 27]
    );
}

/// US7-S4: a known uniform band, independent JPEG decoding and block rounding.
#[test]
fn render_jpeg_and_shrink() {
    let sub = reduced(2400, 100, |x, _| {
        [if (1000..1400).contains(&x) {
            9000.0
        } else {
            1000.0
        }; 3]
    })
    .colour;
    let mut stack = Stack::new(2400, 100);
    stack.add(&sub, &Placement::identity());
    assert!(stack.weight().iter().all(|w| *w == 1.0));
    let sums = stack.sum().to_vec();
    let picture = display::render(&stack, [1.0; 3], 10.0, 1920);
    assert_eq!((picture.width, picture.height), (1200, 50));
    assert_eq!(at(&picture, 100, 25), [0; 3]);
    assert_eq!(at(&picture, 600, 25), [255; 3]);
    let decoded = image::load_from_memory(&preview::jpeg(&picture).unwrap())
        .unwrap()
        .to_rgb8();
    assert_eq!(decoded.dimensions(), (1200, 50));
    assert!(decoded.get_pixel(100, 25).0.into_iter().all(|v| v <= 6));
    assert!(decoded.get_pixel(600, 25).0.into_iter().all(|v| v >= 249));
    let small = display::shrink(&picture, 1024);
    assert_eq!((small.width, small.height), (600, 25));
    assert_eq!(at(&small, 50, 12), [0; 3]);
    assert_eq!(at(&small, 300, 12), [255; 3]);
    assert_eq!(display::shrink(&picture, 1920), picture);
    let tiny = Rgb {
        width: 2,
        height: 2,
        pixels: [0, 0, 0, 1, 2, 255].repeat(2),
    };
    assert_eq!(
        display::shrink(&tiny, 1),
        Rgb {
            width: 1,
            height: 1,
            pixels: vec![1, 1, 128]
        }
    );
    assert_eq!(stack.sum(), sums);
    assert!(stack.weight().iter().all(|w| *w == 1.0));
}

fn picture_number(name: &str) -> u64 {
    let number = name
        .strip_prefix("/api/imaging/stack/preview.jpg?n=")
        .unwrap();
    assert!(!number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()));
    number.parse().unwrap()
}

fn jpeg_size(bytes: &[u8]) -> (u32, u32) {
    assert_eq!(
        image::guess_format(bytes).unwrap(),
        image::ImageFormat::Jpeg
    );
    let picture = image::load_from_memory(bytes).unwrap();
    (picture.width(), picture.height())
}

/// US7-S5: kept JPEGs and picture counts, including colour-only requests and restart.
#[tokio::test(start_paused = true)]
async fn kept_picture_refresh_stop_and_restart() {
    let rig = stand_in_rig("stack-display", ScriptedSolver::new(), None).await;
    let stack = rig.state.stack().unwrap();
    assert!(stack.view().is_none());
    assert!(stack.preview().is_none());
    assert!(stack.preview_within(32).is_none());
    let started = stack.start(StartRequest::default(), None).await.unwrap();
    assert_eq!(started.stacked, 0);
    assert!(started.preview.is_none());
    assert!(serde_json::to_value(&started)
        .unwrap()
        .get("preview")
        .is_none());
    assert!(stack.preview().is_none());
    let view = wait_stack(&rig, |v| {
        v["stacked"].as_u64().unwrap() >= 1 && v["preview"].is_string()
    })
    .await;
    let number = picture_number(view["preview"].as_str().unwrap());
    assert!(number >= 1);
    assert_eq!(jpeg_size(&stack.preview().unwrap()), (64, 48));
    assert_eq!(jpeg_size(&stack.preview_within(32).unwrap()), (32, 24));
    let before_empty = stack.view().unwrap().preview;
    let unchanged = stack.start(StartRequest::default(), None).await.unwrap();
    assert_eq!(unchanged.preview, before_empty);
    let before = picture_number(stack.view().unwrap().preview.as_deref().unwrap());
    let changed = stack
        .start(
            StartRequest {
                gains: Some([2.0, 1.0, 1.0]),
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();
    assert!(picture_number(changed.preview.as_deref().unwrap()) > before);
    let original = stack.preview().unwrap();
    stack
        .start(
            StartRequest {
                gains: Some([1.0, 1.0, 0.5]),
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();
    let gained = stack.preview().unwrap();
    assert_ne!(gained, original);
    stack
        .start(
            StartRequest {
                stretch: Some(1.0),
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();
    assert_ne!(stack.preview().unwrap(), gained);
    let ended = stack.stop().await.unwrap();
    assert!(ended.preview.is_some());
    let kept = stack.preview().unwrap();
    assert_eq!(jpeg_size(&kept), (64, 48));
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(stack.preview().unwrap(), kept);
    assert_eq!(stack.view().unwrap().preview, ended.preview);
    let restarted = stack.start(StartRequest::default(), None).await.unwrap();
    assert_eq!(restarted.stacked, 0);
    assert!(restarted.preview.is_none());
    assert!(stack.preview().is_none());
    stack.stop().await;
}

/// US7-S5: queued subs defer the first picture to eight accepted subs, not eight taken.
#[tokio::test(start_paused = true)]
async fn busy_worker_renders_every_eighth_stacked_sub() {
    let rig = stand_in_rig(
        "stack-display-busy",
        ScriptedSolver::new().delay(Duration::from_secs(3)),
        None,
    )
    .await;
    let stack = rig.state.stack().unwrap();
    stack.start(StartRequest::default(), None).await.unwrap();
    for _ in 0..3000 {
        let view = stack.view().unwrap();
        if view.stacked < 8 {
            assert!(view.preview.is_none(), "{view:?}");
            assert!(stack.preview().is_none());
        }
        if view.stacked >= 9 {
            assert_eq!(view.stacked, 9);
            assert_eq!(
                view.preview.as_deref(),
                Some("/api/imaging/stack/preview.jpg?n=1")
            );
            assert_eq!(jpeg_size(&stack.preview().unwrap()), (64, 48));
            stack.stop().await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("the slow stack did not reach nine subs");
}
