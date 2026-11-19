//! US1's oracles are the spec's worked examples and shared sensor formulas.
//! Counts are exact unless a scenario explicitly allows one count or noise.
use scope_node::imaging::{Exposure, ImagingCamera, ImagingFrame, ImagingSettings, Roi};
use simulator::{
    DustShadow, FlatPanel, HotPixel, LightPath, SensorPattern, SimImagingCamera, SimStar,
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

fn white_star(sim: &Simulator) -> SimStar {
    let truth = sim.imaging_truth();
    SimStar {
        ra_deg: truth.ra_deg,
        dec_deg: truth.dec_deg,
        magnitude: 5.0,
        colour: [1.0; 3],
    }
}

fn samples(frame: &ImagingFrame) -> impl Iterator<Item = u16> + '_ {
    assert_eq!(frame.bits, 16);
    assert_eq!(
        frame.samples.len(),
        frame.width as usize * frame.height as usize * 2
    );
    frame
        .samples
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| u16::from_le_bytes(*v))
}

fn sample(frame: &ImagingFrame, x: u32, y: u32) -> u16 {
    assert!(x < frame.width && y < frame.height);
    let index = 2 * (y as usize * frame.width as usize + x as usize);
    u16::from_le_bytes(frame.samples[index..index + 2].try_into().unwrap())
}

async fn take(camera: &SimImagingCamera, ms: f64, gain: f64, binning: u32) -> ImagingFrame {
    camera
        .apply(&ImagingSettings {
            gain: Some(gain),
            binning: Some(binning),
            ..Default::default()
        })
        .await
        .unwrap();
    camera
        .expose(&Exposure {
            exposure_ms: ms,
            purpose: "frame".into(),
        })
        .await
        .unwrap()
}

fn hot_pixels() -> SensorPattern {
    SensorPattern {
        hot_pixels: vec![
            HotPixel {
                x: 100,
                y: 50,
                electrons_per_s: 4000.0,
            },
            HotPixel {
                x: 101,
                y: 51,
                electrons_per_s: 2000.0,
            },
            HotPixel {
                x: 600,
                y: 400,
                electrons_per_s: 40000.0,
            },
        ],
        column_amplitude: 0.0,
    }
}

#[tokio::test(start_paused = true)]
async fn defaults_and_explicit_defaults_give_the_same_165_counts() {
    let sim = rig();
    assert_eq!(sim.sensor_pattern(), SensorPattern::default());
    assert!(sim.sensor_pattern().hot_pixels.is_empty());
    assert_eq!(sim.sensor_pattern().column_amplitude, 0.0);
    assert_eq!(sim.light_path(), LightPath::default());
    assert!(!sim.light_path().cover);
    assert_eq!(sim.light_path().flat_panel, None);
    assert_eq!(sim.light_path().corner_light, 1.0);
    assert!(sim.light_path().dust.is_empty());
    let first = take(&camera(&sim), 1000.0, 0.0, 1).await;
    sim.set_sensor_pattern(SensorPattern::default());
    sim.set_light_path(LightPath::default());
    let second = take(&camera(&sim), 1000.0, 0.0, 1).await;
    // 16 counts per offset unit, offset 10, plus 20 sky electrons at 0.25.
    assert!(samples(&first).all(|v| v == 165));
    assert!(samples(&second).all(|v| v == 165));
    assert_eq!(first.samples, second.samples);
}

#[tokio::test(start_paused = true)]
async fn cover_blocks_stars_and_sky_but_keeps_the_read_noise() {
    let sim = rig();
    sim.set_light_path(LightPath {
        cover: true,
        ..Default::default()
    });
    let starred = camera(&sim).with_stars(vec![white_star(&sim)]);
    assert!(samples(&take(&starred, 1000.0, 0.0, 1).await).all(|v| v == 160));
    let noisy = camera(&sim).with_noise(true);
    let frame = take(&noisy, 1000.0, 0.0, 1).await;
    let n = f64::from(frame.width) * f64::from(frame.height);
    let mean = samples(&frame).map(f64::from).sum::<f64>() / n;
    let sd = (samples(&frame)
        .map(|v| (f64::from(v) - mean).powi(2))
        .sum::<f64>()
        / n)
        .sqrt();
    assert!((mean - 160.0).abs() <= 0.05, "mean {mean}");
    // sqrt(0.75² + 1/12) = 0.804: read noise and sample rounding only.
    assert!((0.76..=0.85).contains(&sd), "deviation {sd}");
}

#[tokio::test(start_paused = true)]
async fn hot_charge_scales_with_time_survives_cover_and_averages_before_binning() {
    let sim = rig();
    sim.set_sensor_pattern(hot_pixels());
    sim.set_light_path(LightPath {
        cover: true,
        ..Default::default()
    });
    let camera = camera(&sim);
    for (ms, expected) in [(1000.0, [1160, 660, 10160]), (500.0, [660, 410, 5160])] {
        let frame = take(&camera, ms, 0.0, 1).await;
        for (hot, worked) in hot_pixels().hot_pixels.iter().zip(expected) {
            let charge = (160.0 + 0.25 * hot.electrons_per_s * ms / 1000.0).round() as u16;
            assert_eq!(charge, worked);
            assert_eq!(sample(&frame, hot.x, hot.y), worked);
        }
        assert_eq!(samples(&frame).filter(|&v| v != 160).count(), 3);
    }
    sim.set_light_path(LightPath {
        cover: true,
        corner_light: 0.5,
        ..Default::default()
    });
    let frame = take(&camera, 1000.0, 0.0, 1).await;
    for (hot, worked) in hot_pixels().hot_pixels.iter().zip([1160, 660, 10160]) {
        assert_eq!(sample(&frame, hot.x, hot.y), worked);
    }
    sim.set_light_path(LightPath::default());
    let frame = take(&camera, 1000.0, 0.0, 1).await;
    assert_eq!(sample(&frame, 100, 50), 1165);
    assert_eq!(sample(&frame, 0, 0), 165);
    sim.set_light_path(LightPath {
        cover: true,
        ..Default::default()
    });
    let frame = take(&camera, 1000.0, 0.0, 2).await;
    assert_eq!((frame.width, frame.height), (618, 411));
    for ((x, y, worked), hot) in [(50, 24, 410), (51, 25, 285), (300, 200, 2660)]
        .into_iter()
        .zip(hot_pixels().hot_pixels)
    {
        assert_eq!(160.0 + 0.25 * hot.electrons_per_s / 4.0, f64::from(worked));
        assert_eq!(sample(&frame, x, y), worked);
    }
    assert_eq!(samples(&frame).filter(|&v| v != 160).count(), 3);
}

/// The shared setup's V and D, evaluated independently at sensor sample centres.
fn transmission(sim: &Simulator, path: &LightPath, x: u32, y: u32) -> f64 {
    let optics = sim.imaging_optics();
    let (half_w, half_h) = (
        f64::from(optics.width) / 2.0,
        f64::from(optics.height) / 2.0,
    );
    let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
    let v = 1.0
        - (1.0 - path.corner_light) * ((px - half_w).powi(2) + (py - half_h).powi(2))
            / (half_w.powi(2) + half_h.powi(2));
    let d: f64 = path
        .dust
        .iter()
        .filter(|s| (px - s.x).powi(2) + (py - s.y).powi(2) <= s.radius_px.powi(2))
        .map(|s| 1.0 - s.depth)
        .product();
    v * d
}

fn panel_counts(sim: &Simulator, path: &LightPath, x: u32, y: u32, seconds: f64) -> f64 {
    let panel = path.flat_panel.unwrap();
    let colour = match (x % 2, y % 2) {
        (0, 0) => 0,
        (1, 1) => 2,
        _ => 1,
    };
    160.0
        + 0.25
            * panel.electrons_per_s
            * seconds
            * panel.colour[colour]
            * transmission(sim, path, x, y)
}

fn within_one(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1.0,
        "{actual} against {expected}"
    );
}

#[tokio::test(start_paused = true)]
async fn columns_follow_sensor_coordinates_after_gain_and_before_binning() {
    let sim = rig();
    sim.set_sensor_pattern(SensorPattern {
        column_amplitude: 14.0,
        ..Default::default()
    });
    sim.set_light_path(LightPath {
        cover: true,
        ..Default::default()
    });
    let camera = camera(&sim);
    let first = take(&camera, 1000.0, 0.0, 1).await;
    let column_counts = |x: u32| 160.0 + 14.0 * (f64::from(x % 8) / 7.0 - 0.5);
    for (index, value) in samples(&first).enumerate() {
        let x = index as u32 % first.width;
        let worked = 160 + 2 * (x % 8) as i32 - 7;
        assert_eq!(column_counts(x).round() as i32, worked);
        assert_eq!(i32::from(value), worked);
    }
    assert_eq!(first.samples, take(&camera, 100.0, 200.0, 1).await.samples);
    let binned = take(&camera, 1000.0, 0.0, 2).await;
    for (x, worked) in [155, 157, 163, 165, 155].into_iter().enumerate() {
        let sensor_x = 2 * (x - x % 2) + x % 2;
        let mean = (column_counts(sensor_x as u32) + column_counts(sensor_x as u32 + 2)) / 2.0;
        assert_eq!(mean.round() as u16, worked);
        assert_eq!(sample(&binned, x as u32, 0), worked);
    }
    camera
        .apply(&ImagingSettings {
            binning: Some(1),
            roi: Some(Roi {
                x: 4,
                y: 0,
                width: 16,
                height: 2,
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    let region = take(&camera, 1000.0, 0.0, 1).await;
    assert_eq!(sample(&region, 0, 0), 161);
    for (index, value) in samples(&region).enumerate() {
        let sensor_x = index as u32 % region.width + 4;
        assert_eq!(f64::from(value), column_counts(sensor_x).round());
    }
}

#[tokio::test(start_paused = true)]
async fn coloured_panel_replaces_stars_and_sky_and_vignettes_in_sensor_coordinates() {
    let sim = rig();
    let mut path = LightPath {
        flat_panel: Some(FlatPanel {
            electrons_per_s: 40000.0,
            colour: [0.8, 1.0, 0.6],
        }),
        corner_light: 0.5,
        ..Default::default()
    };
    sim.set_light_path(path.clone());
    let camera = camera(&sim).with_stars(vec![white_star(&sim)]);
    let frame = take(&camera, 1000.0, 0.0, 1).await;
    for (x, y, worked) in [
        (618, 411, 10160),
        (618, 410, 8160),
        (619, 411, 6160),
        (0, 0, 4167),
        (1235, 821, 3166),
    ] {
        let expected = panel_counts(&sim, &path, x, y, 1.0);
        within_one(expected, f64::from(worked));
        within_one(f64::from(sample(&frame, x, y)), expected);
        within_one(f64::from(sample(&frame, x, y)), f64::from(worked));
    }
    let half = take(&camera, 500.0, 0.0, 1).await;
    within_one(f64::from(sample(&half, 618, 411)), 5160.0);
    within_one(
        f64::from(sample(&half, 618, 411)),
        panel_counts(&sim, &path, 618, 411, 0.5),
    );
    path.flat_panel.as_mut().unwrap().colour = [1.0; 3];
    sim.set_light_path(path.clone());
    camera
        .apply(&ImagingSettings {
            roi: Some(Roi {
                x: 600,
                y: 400,
                width: 40,
                height: 30,
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    let region = take(&camera, 1000.0, 0.0, 1).await;
    for (x, y, worked) in [(0, 0, 10156), (18, 11, 10160), (39, 29, 10153)] {
        let expected = panel_counts(&sim, &path, x + 600, y + 400, 1.0);
        within_one(expected, f64::from(worked));
        within_one(f64::from(sample(&region, x, y)), expected);
        within_one(f64::from(sample(&region, x, y)), f64::from(worked));
    }
}

#[tokio::test(start_paused = true)]
async fn dust_includes_its_rim_sky_vignettes_and_cover_blocks_a_panel() {
    let sim = rig();
    let mut path = LightPath {
        flat_panel: Some(FlatPanel {
            electrons_per_s: 40000.0,
            colour: [1.0; 3],
        }),
        dust: vec![DustShadow {
            x: 300.0,
            y: 200.0,
            radius_px: 10.0,
            depth: 0.3,
        }],
        ..Default::default()
    };
    let camera = camera(&sim);
    sim.set_light_path(path.clone());
    let frame = take(&camera, 1000.0, 0.0, 1).await;
    for (x, y, worked) in [
        (300, 200, 7160),
        (309, 200, 7160),
        (310, 200, 10160),
        (300, 215, 10160),
    ] {
        assert_eq!(panel_counts(&sim, &path, x, y, 1.0).round() as u16, worked);
        assert_eq!(sample(&frame, x, y), worked);
    }
    path.dust[0].x = 300.5;
    path.dust[0].y = 200.5;
    sim.set_light_path(path.clone());
    let rim = take(&camera, 1000.0, 0.0, 1).await;
    for (x, worked) in [(310, 7160), (311, 10160)] {
        assert_eq!(
            panel_counts(&sim, &path, x, 200, 1.0).round() as u16,
            worked
        );
        assert_eq!(sample(&rim, x, 200), worked);
    }
    let sky = LightPath {
        corner_light: 0.5,
        ..Default::default()
    };
    sim.set_light_path(sky.clone());
    let frame = take(&camera, 4000.0, 0.0, 1).await;
    for (x, y, worked) in [(618, 411, 180), (0, 0, 170)] {
        let expected = (160.0 + 0.25 * 20.0 * 4.0 * transmission(&sim, &sky, x, y)).round() as u16;
        assert_eq!(expected, worked);
        assert_eq!(sample(&frame, x, y), worked);
    }
    path.dust[0].x = 300.0;
    path.dust[0].y = 200.0;
    path.cover = true;
    sim.set_light_path(path);
    assert!(samples(&take(&camera, 1000.0, 0.0, 1).await).all(|v| v == 160));
}

#[tokio::test(start_paused = true)]
async fn star_light_takes_both_dust_factors_and_vignetting_before_hot_charge() {
    let plain_sim = rig();
    let sim = rig();
    // Magnitude 10 keeps the unattenuated comparison below full scale.
    let plain_camera = camera(&plain_sim).with_stars(vec![SimStar {
        magnitude: 10.0,
        ..white_star(&plain_sim)
    }]);
    let camera = camera(&sim).with_stars(vec![SimStar {
        magnitude: 10.0,
        ..white_star(&sim)
    }]);
    for camera in [&plain_camera, &camera] {
        camera
            .apply(&ImagingSettings {
                roi: Some(Roi {
                    x: 600,
                    y: 400,
                    width: 40,
                    height: 30,
                }),
                ..Default::default()
            })
            .await
            .unwrap();
    }
    let path = LightPath {
        corner_light: 0.5,
        dust: vec![
            DustShadow {
                x: 618.0,
                y: 411.0,
                radius_px: 20.0,
                depth: 0.3,
            },
            DustShadow {
                x: 618.0,
                y: 411.0,
                radius_px: 10.0,
                depth: 0.5,
            },
        ],
        ..Default::default()
    };
    sim.set_light_path(path.clone());
    sim.set_sensor_pattern(SensorPattern {
        hot_pixels: vec![HotPixel {
            x: 618,
            y: 411,
            electrons_per_s: 4000.0,
        }],
        ..Default::default()
    });
    // Both start at the same injected instant, so sidereal drift is identical.
    let (plain, patterned) = tokio::join!(
        take(&plain_camera, 1000.0, 0.0, 1),
        take(&camera, 1000.0, 0.0, 1)
    );
    for y in 0..30 {
        for x in 0..40 {
            let hot = if (x, y) == (18, 11) { 1000.0 } else { 0.0 };
            let expected = 160.0
                + (f64::from(sample(&plain, x, y)) - 160.0)
                    * transmission(&sim, &path, x + 600, y + 400)
                + hot;
            // Both frames round independently, so allow one count.
            within_one(f64::from(sample(&patterned, x, y)), expected);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn an_exposure_keeps_its_initial_pattern_and_light_path() {
    let sim = rig();
    sim.set_light_path(LightPath {
        cover: true,
        ..Default::default()
    });
    sim.set_sensor_pattern(hot_pixels());
    let camera = Arc::new(camera(&sim));
    camera
        .apply(&ImagingSettings {
            gain: Some(0.0),
            ..Default::default()
        })
        .await
        .unwrap();
    let task = tokio::spawn({
        let camera = camera.clone();
        async move {
            camera
                .expose(&Exposure {
                    exposure_ms: 1000.0,
                    purpose: "frame".into(),
                })
                .await
                .unwrap()
        }
    });
    tokio::task::yield_now().await;
    assert!(camera.status().await.unwrap().exposing);
    let clone = sim.clone();
    clone.set_sensor_pattern(SensorPattern::default());
    clone.set_light_path(LightPath::default());
    assert_eq!(sim.sensor_pattern(), SensorPattern::default());
    assert_eq!(sim.light_path(), LightPath::default());
    let frame = task.await.unwrap();
    assert_eq!(sample(&frame, 100, 50), 1160);
    assert_eq!(samples(&frame).filter(|&v| v != 160).count(), 3);
    assert!(samples(&take(&camera, 1000.0, 0.0, 1).await).all(|v| v == 165));
}

#[test]
fn setters_reject_invalid_inputs_without_changing_the_shared_state() {
    let sim = rig();
    for bad in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for pattern in [
            SensorPattern {
                column_amplitude: bad,
                ..Default::default()
            },
            SensorPattern {
                hot_pixels: vec![HotPixel {
                    x: 0,
                    y: 0,
                    electrons_per_s: bad,
                }],
                ..Default::default()
            },
        ] {
            assert!(std::panic::catch_unwind(|| sim.set_sensor_pattern(pattern)).is_err());
            assert_eq!(sim.sensor_pattern(), SensorPattern::default());
        }
    }
    let mut invalid = Vec::new();
    for corner_light in [0.0, -0.1, 1.1, f64::NAN, f64::INFINITY] {
        invalid.push(LightPath {
            corner_light,
            ..Default::default()
        });
    }
    for bad in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        invalid.push(LightPath {
            flat_panel: Some(FlatPanel {
                electrons_per_s: bad,
                colour: [1.0; 3],
            }),
            ..Default::default()
        });
        for colour in 0..3 {
            let mut panel = FlatPanel {
                electrons_per_s: 0.0,
                colour: [0.0; 3],
            };
            panel.colour[colour] = bad;
            invalid.push(LightPath {
                flat_panel: Some(panel),
                ..Default::default()
            });
        }
    }
    for (radius_px, depth) in [
        (0.0, 0.0),
        (-1.0, 0.0),
        (f64::NAN, 0.0),
        (10.0, -0.1),
        (10.0, 1.1),
        (10.0, f64::NAN),
        (10.0, f64::INFINITY),
    ] {
        invalid.push(LightPath {
            dust: vec![DustShadow {
                x: 0.0,
                y: 0.0,
                radius_px,
                depth,
            }],
            ..Default::default()
        });
    }
    for path in invalid {
        assert!(std::panic::catch_unwind(|| sim.set_light_path(path)).is_err());
        assert_eq!(sim.light_path(), LightPath::default());
    }
    let valid = LightPath {
        flat_panel: Some(FlatPanel {
            electrons_per_s: 0.0,
            colour: [0.0; 3],
        }),
        corner_light: f64::MIN_POSITIVE,
        dust: vec![
            DustShadow {
                x: 0.0,
                y: 0.0,
                radius_px: 1.0,
                depth: 0.0,
            },
            DustShadow {
                x: 0.0,
                y: 0.0,
                radius_px: 1.0,
                depth: 1.0,
            },
        ],
        ..Default::default()
    };
    sim.set_light_path(valid.clone());
    assert_eq!(sim.light_path(), valid);
    let mut copy = sim.light_path();
    copy.dust.clear();
    assert_eq!(sim.light_path(), valid);
    sim.set_sensor_pattern(hot_pixels());
    let mut copy = sim.sensor_pattern();
    copy.hot_pixels.clear();
    assert_eq!(sim.sensor_pattern(), hot_pixels());
}

#[tokio::test(start_paused = true)]
async fn hot_pixels_are_clipped_to_the_region_in_sensor_coordinates() {
    let sim = rig();
    sim.set_sensor_pattern(hot_pixels());
    sim.set_light_path(LightPath {
        cover: true,
        ..Default::default()
    });
    let camera = camera(&sim);
    camera
        .apply(&ImagingSettings {
            roi: Some(Roi {
                x: 96,
                y: 48,
                width: 16,
                height: 16,
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    let frame = take(&camera, 1000.0, 0.0, 1).await;
    assert_eq!(sample(&frame, 4, 2), 1160);
    assert_eq!(sample(&frame, 5, 3), 660);
    assert_eq!(samples(&frame).filter(|&v| v != 160).count(), 2);
}
