//! US1: independent worked examples for scene light, in counts at gain 0.
//! Samples are exact unless the scenario allows one or two counts, or noise.
use scope_node::imaging::{Exposure, ImagingCamera, ImagingFrame, ImagingSettings};
use simulator::{
    DustShadow, ExtendedSource, FlatPanel, LightPath, SatelliteStreak, Scene, SimImagingCamera,
    Simulator, Site, VirtualClock,
};
use std::{path::Path, sync::Arc};

fn rig() -> Simulator {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference-solutions/cedar-solve/tetra3/data/default_database.npz");
    let sim = Simulator::new(
        Arc::new(pattern_database::load_from_path(&path).expect("bundled cedar database")),
        Site::default(),
        VirtualClock::new(1_768_528_800.0),
    );
    sim.push_to(50.0, 120.0);
    sim
}

fn camera(sim: &Simulator) -> SimImagingCamera {
    SimImagingCamera::new(sim.clone())
        .with_noise(false)
        .with_stars(vec![])
}

async fn take(camera: &SimImagingCamera, ms: f64) -> ImagingFrame {
    camera
        .apply(&ImagingSettings {
            gain: Some(0.0),
            ..Default::default()
        })
        .await
        .unwrap();
    camera
        .expose(&Exposure {
            exposure_ms: ms,
            purpose: "sub".into(),
        })
        .await
        .unwrap()
}

fn sample(frame: &ImagingFrame, x: u32, y: u32) -> u16 {
    assert_eq!(frame.bits, 16);
    assert!(x < frame.width && y < frame.height);
    let index = 2 * (y as usize * frame.width as usize + x as usize);
    u16::from_le_bytes(frame.samples[index..index + 2].try_into().unwrap())
}

fn all_samples(frame: &ImagingFrame, expected: u16) {
    assert_eq!((frame.width, frame.height, frame.bits), (1236, 822, 16));
    assert_eq!(frame.samples.len(), 2 * 1236 * 822);
    assert!(frame
        .samples
        .as_chunks::<2>()
        .0
        .iter()
        .all(|bytes| u16::from_le_bytes(*bytes) == expected));
}

fn close(frame: &ImagingFrame, x: u32, y: u32, expected: f64, tolerance: f64) {
    let actual = f64::from(sample(frame, x, y));
    assert!(
        (actual - expected).abs() <= tolerance,
        "({x}, {y}): {actual} vs {expected}, tolerance {tolerance}"
    );
}

fn streak() -> SatelliteStreak {
    SatelliteStreak {
        frames: vec![1],
        from: (100.0, 200.5),
        to: (300.0, 200.5),
        electrons: 4000.0,
    }
}

fn source(sim: &Simulator, ms: f64) -> ExtendedSource {
    let truth = sim.imaging_truth_at(sim.clock().now() + ms / 2000.0);
    ExtendedSource {
        ra_deg: truth.ra_deg,
        dec_deg: truth.dec_deg,
        sigma_arcsec: 20.0 * truth.arcsec_per_px,
        electrons_per_s: 800.0,
        colour: [0.5, 1.0, 0.25],
    }
}

#[tokio::test(start_paused = true)]
async fn default_scene_keeps_every_sample_exactly_165() {
    let sim = rig();
    assert_eq!(sim.scene(), Scene::default());
    assert!(sim.scene().satellites.is_empty());
    assert!(sim.scene().sources.is_empty());
    let first = take(&camera(&sim), 1000.0).await;
    sim.set_scene(Scene::default());
    let second = take(&camera(&sim), 1000.0).await;
    // Offset 10 × 16 counts, and 20 sky electrons × 0.25 counts/electron.
    let expected = (10.0_f64 * 16.0 + 20.0 * 0.25).round() as u16;
    assert_eq!(expected, 165);
    all_samples(&first, expected);
    all_samples(&second, expected);
    assert_eq!(first.samples, second.samples);
}

#[tokio::test(start_paused = true)]
async fn streak_uses_its_frame_number_distance_and_focus_width() {
    for focus in [0.0_f64, 2.0] {
        let sim = rig();
        sim.set_focus_error(focus);
        let scene = Scene {
            satellites: vec![streak()],
            sources: vec![],
        };
        sim.set_scene(scene.clone());
        assert_eq!(sim.scene(), scene);
        let camera = camera(&sim);
        let first = take(&camera, 1000.0).await;
        let second = take(&camera, 1000.0).await;
        let third = take(&camera, 1000.0).await;
        assert_eq!(camera.frames_taken(), 3);
        all_samples(&first, 165);
        all_samples(&third, 165);
        let variance = 1.2_f64.powi(2) + focus.powi(2);
        let expected = |d: f64| 160.0 + 5.0 + 0.25 * 4000.0 * (-d * d / (2.0 * variance)).exp();
        if focus == 0.0 {
            for (x, y, distance, counts) in [
                (150, 200, 0.0, 1165),
                (150, 201, 1.0, 872),
                (150, 199, 1.0, 872),
                (150, 203, 3.0, 209),
                (150, 204, 4.0, 169),
                (99, 200, 0.5, 1082),
            ] {
                assert_eq!(expected(distance).round() as u16, counts);
                close(&second, x, y, expected(distance), 1.0);
            }
            assert_eq!(sample(&second, 150, 207), 165);
            assert_eq!(sample(&second, 50, 200), 165);
            assert_eq!(sample(&second, 151, 200), sample(&second, 150, 200));
        } else {
            assert_eq!(expected(1.0).round() as u16, 1077);
            close(&second, 150, 201, expected(1.0), 1.0);
        }
    }
}

#[test]
#[should_panic]
fn a_source_with_zero_width_panics() {
    rig().set_scene(Scene {
        satellites: vec![],
        sources: vec![ExtendedSource {
            ra_deg: 0.0,
            dec_deg: 0.0,
            sigma_arcsec: 0.0,
            electrons_per_s: 800.0,
            colour: [1.0; 3],
        }],
    });
}

#[tokio::test(start_paused = true)]
async fn glow_uses_sky_position_colour_width_and_exposure() {
    let sim = rig();
    let camera = camera(&sim);
    for ms in [1000.0, 500.0] {
        let scene = Scene {
            satellites: vec![],
            sources: vec![source(&sim, ms)],
        };
        sim.set_scene(scene.clone());
        assert_eq!(sim.scene(), scene);
        let frame = take(&camera, ms).await;
        let seconds = ms / 1000.0;
        // Centre (618, 411); RGGB sample centres are at (x + 0.5, y + 0.5).
        for (x, y, colour, counts) in [
            (618, 411, 1.0, 365),
            (618, 410, 0.5, 265),
            (619, 411, 0.25, 215),
            (638, 411, 1.0, 283),
            (668, 411, 1.0, 173),
        ] {
            let d2 = (f64::from(x) + 0.5 - 618.0).powi(2) + (f64::from(y) + 0.5 - 411.0).powi(2);
            let expected = 160.0
                + 0.25 * seconds * (20.0 + 800.0 * colour * (-d2 / (2.0 * 20.0 * 20.0)).exp());
            if ms == 1000.0 {
                assert_eq!(expected.round() as u16, counts);
            } else if (x, y) == (618, 411) {
                assert_eq!(expected.round() as u16, 262);
            }
            close(&frame, x, y, expected, 2.0);
        }
        assert_eq!(sample(&frame, 0, 0), (160.0 + 5.0 * seconds).round() as u16);
    }
}

#[tokio::test(start_paused = true)]
async fn the_source_is_included_in_shot_noise() {
    let sim = rig();
    let quiet = camera(&sim);
    let noisy = SimImagingCamera::new(sim.clone()).with_stars(vec![]);
    let mut frames = Vec::new();
    for camera in [&quiet, &noisy] {
        sim.set_scene(Scene {
            satellites: vec![],
            sources: vec![ExtendedSource {
                electrons_per_s: 40000.0,
                colour: [1.0; 3],
                ..source(&sim, 1000.0)
            }],
        });
        frames.push(take(camera, 1000.0).await);
    }
    let mut differences = Vec::new();
    for y in 406..=416 {
        for x in 613..=623 {
            differences
                .push(f64::from(sample(&frames[1], x, y)) - f64::from(sample(&frames[0], x, y)));
        }
    }
    let mean = differences.iter().sum::<f64>() / 121.0;
    let deviation = (differences.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / 121.0).sqrt();
    // 0.25 × sqrt(40000 + 20 + 9) ≈ 50 counts; sky/read noise alone ≈ 1.35.
    assert!(deviation >= 20.0, "source shot noise: {deviation} counts");
}

#[tokio::test(start_paused = true)]
async fn cover_panel_and_vignetting_act_on_scene_light() {
    for path in [
        LightPath {
            cover: true,
            ..Default::default()
        },
        LightPath {
            flat_panel: Some(FlatPanel {
                electrons_per_s: 2000.0,
                colour: [0.6, 1.0, 0.8],
            }),
            ..Default::default()
        },
        LightPath {
            corner_light: 0.5,
            ..Default::default()
        },
    ] {
        let sim = rig();
        sim.set_scene(Scene {
            satellites: vec![streak()],
            sources: vec![source(&sim, 1000.0)],
        });
        sim.set_light_path(path.clone());
        let camera = camera(&sim);
        take(&camera, 1000.0).await;
        let frame = take(&camera, 1000.0).await;
        if path.cover {
            all_samples(&frame, 160);
        } else if path.flat_panel.is_some() {
            sim.set_scene(Scene::default());
            let control = take(&camera, 1000.0).await;
            assert_eq!(frame.samples, control.samples);
        } else {
            // 034's V at the sample centre, independently of the renderer.
            let v = 1.0
                - 0.5 * ((150.5_f64 - 618.0).powi(2) + (200.5_f64 - 411.0).powi(2))
                    / (618.0_f64.powi(2) + 411.0_f64.powi(2));
            close(&frame, 150, 200, 160.0 + (5.0 + 1000.0) * v, 1.0);
        }
    }
}

#[test]
fn scene_validation_rejects_invalid_numbers_without_changing_the_scene() {
    let sim = rig();
    let valid = Scene {
        satellites: vec![streak()],
        sources: vec![source(&sim, 1000.0)],
    };
    sim.set_scene(valid.clone());
    // Every floating-point field rejects NaN and infinities.
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for field in 0..12 {
            let mut scene = valid.clone();
            let satellite = &mut scene.satellites[0];
            let source = &mut scene.sources[0];
            *match field {
                0 => &mut satellite.from.0,
                1 => &mut satellite.from.1,
                2 => &mut satellite.to.0,
                3 => &mut satellite.to.1,
                4 => &mut satellite.electrons,
                5 => &mut source.ra_deg,
                6 => &mut source.dec_deg,
                7 => &mut source.sigma_arcsec,
                8 => &mut source.electrons_per_s,
                c => &mut source.colour[c - 9],
            } = bad;
            assert!(std::panic::catch_unwind(|| sim.set_scene(scene)).is_err());
            assert_eq!(sim.scene(), valid);
        }
    }
    for field in 0..6 {
        let mut scene = valid.clone();
        *match field {
            0 => &mut scene.satellites[0].electrons,
            1 => &mut scene.sources[0].sigma_arcsec,
            2 => &mut scene.sources[0].electrons_per_s,
            c => &mut scene.sources[0].colour[c - 3],
        } = -1.0;
        assert!(std::panic::catch_unwind(|| sim.set_scene(scene)).is_err());
    }
    let mut copy = sim.scene();
    copy.satellites.clear();
    copy.sources.clear();
    assert_eq!(sim.scene(), valid);
    let mut zero_light = valid;
    zero_light.satellites[0].electrons = 0.0;
    zero_light.sources[0].electrons_per_s = 0.0;
    zero_light.sources[0].colour = [0.0; 3];
    sim.set_scene(zero_light.clone());
    assert_eq!(sim.clone().scene(), zero_light);
}

#[tokio::test(start_paused = true)]
async fn dust_dims_both_streak_and_glow_before_counts_are_added() {
    let sim = rig();
    sim.set_light_path(LightPath {
        dust: vec![
            DustShadow {
                x: 150.5,
                y: 200.5,
                radius_px: 5.0,
                depth: 0.5,
            },
            DustShadow {
                x: 618.0,
                y: 411.0,
                radius_px: 5.0,
                depth: 0.5,
            },
        ],
        ..Default::default()
    });
    let mut satellite = streak();
    satellite.frames = vec![0];
    sim.set_scene(Scene {
        satellites: vec![satellite],
        sources: vec![source(&sim, 1000.0)],
    });
    let frame = take(&camera(&sim), 1000.0).await;
    close(&frame, 150, 200, 160.0 + 0.5 * (5.0 + 1000.0), 1.0);
    close(
        &frame,
        618,
        411,
        160.0 + 0.5 * (5.0 + 200.0 * (-0.5_f64 / 800.0).exp()),
        2.0,
    );
}

#[tokio::test(start_paused = true)]
async fn scene_is_fixed_when_an_exposure_starts() {
    let sim = rig();
    let mut satellite = streak();
    satellite.frames = vec![0];
    sim.set_scene(Scene {
        satellites: vec![satellite],
        sources: vec![],
    });
    let camera = Arc::new(camera(&sim));
    let pending = {
        let camera = camera.clone();
        tokio::spawn(async move { take(&camera, 1000.0).await })
    };
    tokio::task::yield_now().await;
    assert!(camera.status().await.unwrap().exposing);
    sim.set_scene(Scene::default());
    close(&pending.await.unwrap(), 150, 200, 1165.0, 1.0);
    all_samples(&take(&camera, 1000.0).await, 165);
}

#[tokio::test(start_paused = true)]
async fn segment_caps_and_round_cutoffs_are_in_sensor_pixels() {
    let sim = rig();
    let glow = source(&sim, 1000.0);
    sim.set_scene(Scene {
        satellites: vec![
            SatelliteStreak {
                frames: vec![0],
                from: (203.5, 203.5),
                to: (200.5, 200.5),
                electrons: 4000.0,
            },
            SatelliteStreak {
                frames: vec![0],
                from: (300.5, 300.5),
                to: (300.5, 300.5),
                electrons: 1e9,
            },
        ],
        sources: vec![ExtendedSource {
            sigma_arcsec: glow.sigma_arcsec / 10.0,
            electrons_per_s: 1e9,
            colour: [1.0; 3],
            ..glow
        }],
    });
    let frame = take(&camera(&sim), 1000.0).await;
    // One sample off the diagonal is sqrt(0.5) from its line; past the end,
    // (199.5, 199.5) is sqrt(2) from the cap at (200.5, 200.5).
    close(
        &frame,
        201,
        202,
        165.0 + 1000.0 * (-0.5_f64 / 2.88).exp(),
        1.0,
    );
    close(
        &frame,
        199,
        199,
        165.0 + 1000.0 * (-2.0_f64 / 2.88).exp(),
        1.0,
    );
    close(
        &frame,
        306,
        300,
        165.0 + 0.25e9 * (-36.0_f64 / 2.88).exp(),
        1.0,
    );
    // High flux makes the cutoff observable: both points lie inside the
    // bounding square, but outside the circular reaches 6 and 9 respectively.
    assert_eq!(sample(&frame, 306, 301), 165);
    assert_eq!(sample(&frame, 626, 419), 165);
    assert!(sample(&frame, 626, 411) > 20_000);
    // An ROI keeps sensor positions and filter parity, rather than moving the scene.
    let cropped = camera(&sim);
    cropped
        .apply(&ImagingSettings {
            roi: Some(scope_node::imaging::Roi {
                x: 190,
                y: 190,
                width: 140,
                height: 140,
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    let crop = take(&cropped, 1000.0).await;
    for (x, y) in [(201, 202), (199, 199), (306, 300), (306, 301)] {
        assert_eq!(sample(&crop, x - 190, y - 190), sample(&frame, x, y));
    }
}
