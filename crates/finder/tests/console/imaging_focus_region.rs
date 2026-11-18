//! The focus region (033 US2): the nine tiles of a sensor, `focus::find_region`
//! over a scripted solver and over the simulated adapter's sharpest point, and
//! `focus::Kept` and its `focus.json`. Oracles: the worked examples of a tile a
//! third of the sensor each way rounded down to a multiple of twice the binning
//! (1236/3 = 412; 822/3 = 274, and 272 at binning 2; 4944/3 = 1648; 3284/3 =
//! 1094), the stated rules of the smallest reading and of the far-out tile named
//! only without one, the simulator's truth for the sharpest point, and, for
//! `Kept`, the stated keys and values, compared exactly.

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

use finder::imaging::{
    self,
    focus::{self, FocusReading, KeptRegion, Tile, Unmeasurable},
    relation::CameraId,
    solver::{
        ImagingSolver, MeasureFuture, Region, SolveError, SolveFrame, SolveFuture, SolveHint,
        StarMeasure,
    },
};
use serde_json::{json, Value};
use simulator::{Simulator, Site, VirtualClock};

use crate::support::live as support;

/// The focus rig's simulator (the shared setup): the test database at 40° N
/// 75° W on the virtual clock from Unix 1768528800, pointed at the celestial
/// pole, where the imaging field moves by under a pixel in three seconds. No
/// mount is connected.
fn sim() -> Simulator {
    let sim = Simulator::new(
        support::DB.clone(),
        Site::default(),
        VirtualClock::new(support::START),
    );
    sim.push_to(40.0, 0.0);
    sim
}

/// One imaging frame of the test's own description (the shared setup): no file
/// is read, the exposure starts at Unix 1768528810 and lasts 1 s.
fn frame(width: u32, height: u32, binning: u32, roi: Region) -> SolveFrame {
    SolveFrame {
        path: PathBuf::new(),
        width,
        height,
        bits: 16,
        bayer: None,
        binning,
        roi,
        start: 1_768_528_810.0,
        exposure_s: 1.0,
    }
}

/// The whole sensor's region, 1236 × 822.
fn whole() -> Region {
    Region {
        x: 0,
        y: 0,
        width: 1236,
        height: 822,
    }
}

/// One entry of the scripted solver's list (the shared setup): a star measure
/// of so many frame pixels and stars, or the error it answers.
#[derive(Clone)]
enum Entry {
    Stars(f64, u32),
    Error(SolveError),
}

/// The same star measure nine times over.
fn nine(px: f64, stars: u32) -> Vec<Entry> {
    vec![Entry::Stars(px, stars); 9]
}

/// What one `measure` call was given (the shared setup): the frame's region of
/// interest, binning and size, and the region asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Call {
    roi: Region,
    binning: u32,
    width: u32,
    height: u32,
    region: Option<Region>,
}

/// The scripted solver (the shared setup): `measure` keeps a note of each call
/// and answers the next entry of its list at once, repeating the last once the
/// list is used up; `solve` answers `Failed("not used")`.
struct Scripted {
    entries: Vec<Entry>,
    next: AtomicUsize,
    calls: Mutex<Vec<Call>>,
}

impl Scripted {
    fn new(entries: Vec<Entry>) -> Self {
        Self {
            entries,
            next: AtomicUsize::new(0),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// The `measure` calls the solver was given, in order.
    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    /// The next entry's answer, the last one repeating once the list is used
    /// up.
    fn outcome(&self) -> Result<StarMeasure, SolveError> {
        let n = self.next.fetch_add(1, Ordering::SeqCst);
        let last = self.entries.len() - 1;
        match &self.entries[n.min(last)] {
            Entry::Stars(px, stars) => Ok(StarMeasure {
                half_flux_px: *px,
                stars: *stars,
            }),
            Entry::Error(error) => Err(error.clone()),
        }
    }
}

impl ImagingSolver for Scripted {
    fn name(&self) -> &'static str {
        "scripted"
    }

    fn solve(&self, _frame: SolveFrame, _hint: SolveHint) -> SolveFuture<'_> {
        Box::pin(async { Err(SolveError::Failed("not used".to_string())) })
    }

    fn measure(&self, frame: SolveFrame, region: Option<Region>) -> MeasureFuture<'_> {
        self.calls.lock().unwrap().push(Call {
            roi: frame.roi,
            binning: frame.binning,
            width: frame.width,
            height: frame.height,
            region,
        });
        let answer = self.outcome();
        Box::pin(async move { answer })
    }
}

/// US2-S1: a tile is a third of the sensor each way, rounded down to a
/// multiple of twice the binning, and its corner `col × width, row × height`
/// from the sensor's corner; a tile index of 3 in either place is refused
/// (oracle: the worked examples 1236/3 = 412; 822/3 = 274, and 272 at binning 2;
/// 4944/3 = 1648; 3284/3 = 1094.67, so 1094).
#[test]
fn the_tiles_of_a_sensor() {
    let tile = |col, row| Tile::new(col, row).expect("within the grid");
    assert_eq!(
        focus::tile_region(whole(), 1, tile(0, 0)),
        Region {
            x: 0,
            y: 0,
            width: 412,
            height: 274,
        }
    );
    assert_eq!(
        focus::tile_region(whole(), 1, tile(2, 0)),
        Region {
            x: 824,
            y: 0,
            width: 412,
            height: 274,
        }
    );
    assert_eq!(
        focus::tile_region(whole(), 1, tile(1, 1)),
        Region {
            x: 412,
            y: 274,
            width: 412,
            height: 274,
        }
    );
    assert_eq!(
        focus::tile_region(whole(), 1, tile(2, 2)),
        Region {
            x: 824,
            y: 548,
            width: 412,
            height: 274,
        }
    );
    // A large sensor: 4944/3 = 1648, 3284/3 = 1094.67, so 1094.
    let large = Region {
        x: 0,
        y: 0,
        width: 4944,
        height: 3284,
    };
    assert_eq!(
        focus::tile_region(large, 1, tile(2, 2)),
        Region {
            x: 3296,
            y: 2188,
            width: 1648,
            height: 1094,
        }
    );
    // At binning 2 the height rounds to 272, twice the binning twice.
    assert_eq!(
        focus::tile_region(whole(), 2, tile(1, 1)),
        Region {
            x: 412,
            y: 272,
            width: 412,
            height: 272,
        }
    );
    assert!(Tile::new(3, 0).is_none());
    assert!(Tile::new(0, 3).is_none());
    assert!(Tile::new(2, 2).is_some());
}

/// US2-S2: `find_region` measures the nine tiles of a whole frame in row-major
/// order, one after another, and names the tile with the smallest reading among
/// the tiles with five stars, passing over a smaller one with three (oracle:
/// the worked example: the smallest of the eight tiles with five stars is 2.6,
/// and the smaller 2.2 has only three stars).
#[tokio::test(start_paused = true)]
async fn the_scripted_search_and_its_calls() {
    let entries = || {
        vec![
            Entry::Stars(4.0, 12),
            Entry::Stars(3.5, 12),
            Entry::Stars(2.6, 12),
            Entry::Stars(3.6, 12),
            Entry::Stars(2.9, 12),
            Entry::Stars(3.4, 12),
            Entry::Stars(4.1, 12),
            Entry::Stars(2.2, 3),
            Entry::Stars(4.4, 12),
        ]
    };
    let solver = Scripted::new(entries());
    let search = focus::find_region(&solver, frame(1236, 822, 1, whole()), 1, None)
        .await
        .unwrap();

    // The best is the smallest of the tiles with five stars: 2.6 at (2, 0).
    assert_eq!(search.best, Some(Tile { col: 2, row: 0 }));
    assert_eq!(search.far_out, None);

    // Nine entries, in row-major order, the third a reading of 2.6 px and the
    // eighth unmeasurable with three stars.
    let tiles: Vec<Tile> = search.tiles.iter().map(|tile| tile.tile).collect();
    let row_major: Vec<Tile> = (0..3)
        .flat_map(|row| (0..3).map(move |col| Tile { col, row }))
        .collect();
    assert_eq!(tiles, row_major);
    let third = search.tiles[2].read.unwrap();
    assert!((third.px - 2.6).abs() < 1e-9, "{third:?}");
    assert_eq!(third.stars, 12);
    assert_eq!(
        search.tiles[7].read,
        Err(Unmeasurable::TooFewStars { stars: 3 })
    );

    // Nine calls, with the regions of the nine tiles in row-major order, the
    // tile's sensor pixels at the frame's binning.
    let calls = solver.calls();
    assert_eq!(calls.len(), 9);
    assert_eq!(
        calls[0],
        Call {
            roi: whole(),
            binning: 1,
            width: 1236,
            height: 822,
            region: Some(Region {
                x: 0,
                y: 0,
                width: 412,
                height: 274,
            }),
        }
    );
    let regions: Vec<Option<Region>> = (0..3)
        .flat_map(|row| {
            (0..3).map(move |col| {
                Some(Region {
                    x: 412 * col,
                    y: 274 * row,
                    width: 412,
                    height: 274,
                })
            })
        })
        .collect();
    assert_eq!(
        calls.iter().map(|call| call.region).collect::<Vec<_>>(),
        regions
    );

    // A whole frame at binning 2: the fifth call's region is the tile's sensor
    // pixels divided by the binning.
    let solver = Scripted::new(entries());
    focus::find_region(&solver, frame(618, 411, 2, whole()), 1, None)
        .await
        .unwrap();
    let calls = solver.calls();
    assert_eq!(calls.len(), 9);
    assert_eq!(
        calls[4].region,
        Some(Region {
            x: 206,
            y: 136,
            width: 206,
            height: 136,
        })
    );
}

async fn search(entries: Vec<Entry>) -> (Result<focus::Search, SolveError>, Vec<Call>) {
    let solver = Scripted::new(entries);
    let search = focus::find_region(&solver, frame(1236, 822, 1, whole()), 1, None).await;
    (search, solver.calls())
}

/// US2-S3: a tile that is unmeasurable is not a candidate; equal readings give
/// the first; a `TooFewStars` tile is left out and the best is still answered;
/// the far-out tile with the smallest size is named only when there is no best;
/// and another solver error ends the search at once (oracle: the stated
/// rules).
#[tokio::test(start_paused = true)]
async fn the_search_s_ties_errors_and_far_out_tiles() {
    // Nine tiles with two stars: no best and no far-out tile.
    let (answer, calls) = search(nine(3.0, 2)).await;
    let answer = answer.unwrap();
    assert_eq!(answer.best, None);
    assert_eq!(answer.far_out, None);
    assert_eq!(calls.len(), 9);

    // Nine equal readings: the first of the tied tiles, and no far-out tile.
    let (answer, _) = search(nine(3.0, 12)).await;
    let answer = answer.unwrap();
    assert_eq!(answer.best, Some(Tile { col: 0, row: 0 }));
    assert_eq!(answer.far_out, None);

    // A `TooFewStars` tile is left out and the best is still answered.
    let mut entries = vec![
        Entry::Stars(3.0, 12),
        Entry::Error(SolveError::TooFewStars { detail: None }),
    ];
    entries.extend((0..7).map(|_| Entry::Stars(3.0, 12)));
    let (answer, _) = search(entries).await;
    let answer = answer.unwrap();
    assert_eq!(answer.best, Some(Tile { col: 0, row: 0 }));
    assert_eq!(
        answer.tiles[1].read,
        Err(Unmeasurable::TooFewStars { stars: 0 })
    );

    // Nine far-out tiles: the first of the tied far-out tiles.
    let (answer, _) = search(nine(25.0, 12)).await;
    let answer = answer.unwrap();
    assert_eq!(answer.best, None);
    assert_eq!(answer.far_out, Some(Tile { col: 0, row: 0 }));

    // The smallest of the far-out tiles when one tile has too few stars.
    let mixed = |first: u32| {
        let mut entries = vec![Entry::Stars(3.0, first)];
        entries.push(Entry::Stars(25.0, 12));
        entries.push(Entry::Stars(25.0, 12));
        entries.push(Entry::Stars(25.0, 12));
        entries.push(Entry::Stars(22.0, 12));
        entries.extend((0..4).map(|_| Entry::Stars(25.0, 12)));
        entries
    };
    let (answer, _) = search(mixed(2)).await;
    let answer = answer.unwrap();
    assert_eq!(answer.best, None);
    assert_eq!(answer.far_out, Some(Tile { col: 1, row: 1 }));

    // The same nine with the first tile measurable: the best is answered and no
    // far-out tile is named.
    let (answer, _) = search(mixed(12)).await;
    let answer = answer.unwrap();
    assert_eq!(answer.best, Some(Tile { col: 0, row: 0 }));
    assert_eq!(answer.far_out, None);

    // Another solver error ends the search with that error, after exactly
    // three calls.
    let solver = Scripted::new(vec![
        Entry::Stars(3.0, 12),
        Entry::Stars(3.0, 12),
        Entry::Error(SolveError::Failed("x".to_string())),
    ]);
    assert_eq!(
        focus::find_region(&solver, frame(1236, 822, 1, whole()), 1, None).await,
        Err(SolveError::Failed("x".to_string()))
    );
    assert_eq!(solver.calls().len(), 3);
}

/// US2-S4: on the simulator's seeded field, with the stars smallest at a point
/// of the sensor, the search keeps the tile whose centre is nearest it, and
/// its reading is the point-spread size grown by that distance (oracle: the
/// simulator's truth, worked as 2·√(2·ln 2)·√(1.2² + (3.0·d)²): 2.864762 at
/// (0.8, 0.2) and 2.825784 at the middle, where the nearest other tile reads
/// 3.69 or more).
#[tokio::test(start_paused = true)]
async fn the_search_on_the_simulator_with_a_sharpest_point() {
    let sim = sim();
    let whole_b1 = || frame(1236, 822, 1, whole());
    let region = |col: u32, row: u32| Region {
        x: 412 * col,
        y: 274 * row,
        width: 412,
        height: 274,
    };

    // Sharpest at (0.8, 0.2): the top right tile, after the test's own count
    // of its stars.
    let sharp =
        imaging::simulated::SimulatedSolver::new(sim.clone()).with_sharpest_at(0.8, 0.2, 3.0);
    let measure = sharp.measure(whole_b1(), Some(region(2, 0))).await.unwrap();
    assert!(measure.stars >= 5, "{measure:?}");
    let search = focus::find_region(&sharp, whole_b1(), 1, None)
        .await
        .unwrap();
    let best = Tile { col: 2, row: 0 };
    assert_eq!(search.best, Some(best));
    let reading = search
        .tiles
        .iter()
        .find(|tile| tile.tile == best)
        .expect("the best tile")
        .read
        .unwrap();
    assert!((reading.px - 2.864762).abs() < 1e-5, "{reading:?}");
    // Every other measurable tile reads 3.69 or more: the choice is the
    // sharpest tile's, not noise's.
    for tile in &search.tiles {
        if tile.tile != best {
            if let Ok(other) = tile.read {
                assert!(other.px >= 3.69, "{tile:?}");
            }
        }
    }

    // Sharpest at (0.5, 0.5): the middle tile, after the same check.
    let sharp =
        imaging::simulated::SimulatedSolver::new(sim.clone()).with_sharpest_at(0.5, 0.5, 3.0);
    let measure = sharp.measure(whole_b1(), Some(region(1, 1))).await.unwrap();
    assert!(measure.stars >= 5, "{measure:?}");
    let search = focus::find_region(&sharp, whole_b1(), 1, None)
        .await
        .unwrap();
    let best = Tile { col: 1, row: 1 };
    assert_eq!(search.best, Some(best));
    let reading = search
        .tiles
        .iter()
        .find(|tile| tile.tile == best)
        .expect("the best tile")
        .read
        .unwrap();
    assert!((reading.px - 2.825784).abs() < 1e-5, "{reading:?}");
}

/// US2-S5: `Kept` holds nothing before the camera is described and then writes
/// no file; `keep_region` writes `focus.json` whole under a `.part` name and
/// renames it, with the format, the camera and the found or fixed region; the
/// region is read back for the same camera and not for another serial, another
/// format or a file that is not JSON; and a file that is not JSON is replaced
/// by a readable one at the next `keep_region` (oracle: the stated keys and
/// values, compared exactly).
#[test]
fn the_kept_region_and_its_file() {
    let scratch = crate::support::Scratch::new("imaging-focus-region-kept");
    let path = scratch.0.join("focus.json");
    let camera = CameraId {
        model: "simulated imaging camera".to_string(),
        serial: Some("SIM-0001".to_string()),
        width: 1236,
        height: 822,
        pixel_um: 19.12,
    };
    let other = CameraId {
        serial: Some("SIM-0002".to_string()),
        ..camera.clone()
    };
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
        at: 1_768_528_803.5,
    };
    let found = KeptRegion::found(whole(), 1, Tile::new(2, 0).unwrap(), &reading);
    let read_file = || serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap();

    // Nothing before the camera is described, and no file.
    let mut kept = focus::Kept::new(path.clone());
    kept.describe(camera.clone());
    assert_eq!(kept.region(), None);
    assert!(!path.exists());

    // A region found from a reading, written whole with no `.part` left.
    kept.keep_region(found);
    let file = read_file();
    assert_eq!(file["format"], json!("imaging-focus/1"));
    assert_eq!(
        file["camera"],
        json!({
            "model": "simulated imaging camera",
            "serial": "SIM-0001",
            "width": 1236,
            "height": 822,
            "pixel_um": 19.12,
        })
    );
    assert_eq!(
        file["region"],
        json!({
            "col": 2, "row": 0, "x": 824, "y": 0, "width": 412, "height": 274,
            "source": "found", "px": 2.6, "stars": 12, "frame_id": 3,
            "at": 1_768_528_803.5,
        })
    );
    let parts: Vec<String> = std::fs::read_dir(&scratch.0)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".part"))
        .collect();
    assert_eq!(parts, Vec::<String>::new(), "the file was renamed");

    // A second `Kept` reads it back for the same camera.
    let mut again = focus::Kept::new(path.clone());
    again.describe(camera.clone());
    assert_eq!(again.region(), Some(found));

    // A tile fixed by hand: its rectangle, and none of a reading's keys.
    let fixed = KeptRegion::fixed(whole(), 1, Tile::new(1, 1).unwrap(), 1_768_528_810.0);
    kept.keep_region(fixed);
    let file = read_file();
    assert_eq!(file["region"]["source"], json!("fixed"));
    assert_eq!(file["region"]["x"], json!(412));
    assert_eq!(file["region"]["y"], json!(274));
    for key in ["px", "stars", "frame_id"] {
        assert!(file["region"].get(key).is_none(), "{file}");
    }

    // Another camera's file answers nothing.
    kept.describe(other);
    assert_eq!(kept.region(), None);

    // Another format answers nothing.
    let mut file = read_file();
    file["format"] = json!("imaging-focus/2");
    std::fs::write(&path, file.to_string()).unwrap();
    let mut kept = focus::Kept::new(path.clone());
    kept.describe(camera.clone());
    assert_eq!(kept.region(), None);

    // A `keep_region` before any `describe` writes no file.
    std::fs::remove_file(&path).unwrap();
    let mut kept = focus::Kept::new(path.clone());
    kept.keep_region(fixed);
    assert!(!path.exists());

    // A file that is not JSON answers nothing and is replaced by a readable
    // file at the next `keep_region`.
    std::fs::write(&path, "not json").unwrap();
    let mut kept = focus::Kept::new(path.clone());
    kept.describe(camera.clone());
    assert_eq!(kept.region(), None);
    kept.keep_region(fixed);
    let file = read_file();
    assert_eq!(file["format"], json!("imaging-focus/1"));
    assert_eq!(file["region"]["source"], json!("fixed"));
}
