//! Focus (033): the focus reading, the focus region, the focus record and the
//! verdict.
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{
    relation::CameraId,
    solver::{ImagingSolver, Region, SolveError, SolveFrame},
};

/// Fewer stars than this in a star measure are no reading.
pub const MIN_STARS: u32 = 5;
/// A star measure above this many frame pixels is "far out". 20 frame pixels is
/// ASTAP's limit on a mono frame, and half of it on a colour mosaic that the
/// adapter averages 2 × 2 (`docs/research/2026-11-15-imaging-camera.md` § 3.
/// ASTAP).
pub const FAR_OUT_PX: f64 = 20.0;

/// What one imaging frame, or a region of it, says about focus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusReading {
    pub frame_id: u64,
    /// The median half-flux diameter in sensor pixels.
    pub px: f64,
    /// The same in arcseconds, when a scale was given.
    pub arcsec: Option<f64>,
    pub stars: u32,
    /// The part of the sensor that was measured, in sensor pixels.
    pub region: Region,
    /// The middle of the exposure, console Unix seconds.
    pub at: f64,
}

/// Why a frame, or a region of it, says nothing about focus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unmeasurable {
    TooFewStars {
        stars: u32,
    },
    /// The size in sensor pixels, for showing only.
    FarOut {
        px: f64,
        stars: u32,
    },
}

pub type Read = Result<FocusReading, Unmeasurable>;

/// `region` is in the frame's pixels; `None` is the whole frame. `scale` is
/// arcseconds a sensor pixel.
pub async fn read(
    solver: &dyn ImagingSolver,
    frame: SolveFrame,
    frame_id: u64,
    region: Option<Region>,
    scale: Option<f64>,
) -> Result<Read, SolveError> {
    let binning = frame.binning;
    // The solver's own too-few-stars is a reading that says nothing, whatever
    // its words; any other error of the solver is `read`'s own, unchanged.
    let measure = match solver.measure(frame.clone(), region).await {
        Ok(measure) => measure,
        Err(SolveError::TooFewStars { .. }) => {
            return Ok(Err(Unmeasurable::TooFewStars { stars: 0 }));
        }
        Err(error) => return Err(error),
    };
    if measure.stars < MIN_STARS {
        return Ok(Err(Unmeasurable::TooFewStars {
            stars: measure.stars,
        }));
    }
    // The star measure is in the frame's own pixels, so the limit is read there
    // and the reading is the size in sensor pixels.
    if measure.half_flux_px > FAR_OUT_PX {
        return Ok(Err(Unmeasurable::FarOut {
            px: measure.half_flux_px * f64::from(binning),
            stars: measure.stars,
        }));
    }
    let r = region.unwrap_or(Region {
        x: 0,
        y: 0,
        width: frame.width,
        height: frame.height,
    });
    let px = measure.half_flux_px * f64::from(binning);
    Ok(Ok(FocusReading {
        frame_id,
        px,
        arcsec: scale.map(|scale| px * scale),
        stars: measure.stars,
        region: Region {
            x: frame.roi.x + r.x * binning,
            y: frame.roi.y + r.y * binning,
            width: r.width * binning,
            height: r.height * binning,
        },
        at: frame.start + frame.exposure_s / 2.0,
    }))
}

/// Tiles across and down (FR-006).
pub const GRID: u32 = 3;

/// One of the nine tiles of the sensor (FR-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub col: u32,
    pub row: u32,
}

impl Tile {
    /// `None` unless both are below `GRID`.
    pub fn new(col: u32, row: u32) -> Option<Self> {
        (col < GRID && row < GRID).then_some(Self { col, row })
    }
}

/// The sensor pixels of a tile of `sensor`, for a camera at `binning` (FR-006):
/// a third of the sensor each way, rounded down to a multiple of twice the
/// binning — 030's rule for a region of interest, so every tile can be asked
/// of the camera as it is. A few pixels at the right and bottom edges belong
/// to no tile.
pub fn tile_region(sensor: Region, binning: u32, tile: Tile) -> Region {
    let unit = 2 * binning;
    let width = sensor.width / GRID / unit * unit;
    let height = sensor.height / GRID / unit * unit;
    Region {
        x: sensor.x + tile.col * width,
        y: sensor.y + tile.row * height,
        width,
        height,
    }
}

/// One tile of a frame and what it said (FR-007).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileRead {
    pub tile: Tile,
    pub read: Read,
}

/// The nine tiles of a frame, the sharpest one and, when none could be
/// measured, the far-out tile with the smallest size (FR-007).
#[derive(Debug, Clone, PartialEq)]
pub struct Search {
    /// All nine, in row-major order.
    pub tiles: Vec<TileRead>,
    /// The tile with the smallest reading; `None` when no tile could be
    /// measured.
    pub best: Option<Tile>,
    /// Only while `best` is `None`: the far-out tile with the smallest size.
    /// `None` when there is a `best` or no tile is far out.
    pub far_out: Option<Tile>,
}

/// Measure the nine tiles of an imaging frame, one after another, and name
/// the sharpest (FR-007). The tiles are those of the frame's region of
/// interest at its binning, and each is measured through `read` in the frame's
/// own pixels. An error of the solver other than `TooFewStars` — which `read`
/// has already turned into an unmeasurable tile — ends the search with that
/// error.
pub async fn find_region(
    solver: &dyn ImagingSolver,
    frame: SolveFrame,
    frame_id: u64,
    scale: Option<f64>,
) -> Result<Search, SolveError> {
    let binning = frame.binning;
    let mut tiles = Vec::new();
    let mut best: Option<(Tile, f64)> = None;
    let mut far: Option<(Tile, f64)> = None;
    for row in 0..GRID {
        for col in 0..GRID {
            let tile = Tile::new(col, row).expect("row and column within the grid");
            // The tile's sensor pixels, then the same in the frame's pixels.
            let sensor = tile_region(frame.roi, binning, tile);
            let region = Region {
                x: (sensor.x - frame.roi.x) / binning,
                y: (sensor.y - frame.roi.y) / binning,
                width: sensor.width / binning,
                height: sensor.height / binning,
            };
            let read = read(solver, frame.clone(), frame_id, Some(region), scale).await?;
            match read {
                // The smallest reading, compared with `<` so the first of
                // equal ones stays; an unmeasurable tile is never the best.
                Ok(reading) => {
                    if best.is_none_or(|(_, kept)| reading.px < kept) {
                        best = Some((tile, reading.px));
                    }
                }
                Err(Unmeasurable::FarOut { px, .. }) => {
                    if far.is_none_or(|(_, kept)| px < kept) {
                        far = Some((tile, px));
                    }
                }
                Err(Unmeasurable::TooFewStars { .. }) => {}
            }
            tiles.push(TileRead { tile, read });
        }
    }
    Ok(Search {
        tiles,
        best: best.map(|(tile, _)| tile),
        // A far-out tile is named only when no tile could be measured.
        far_out: if best.is_some() {
            None
        } else {
            far.map(|(tile, _)| tile)
        },
    })
}

/// The format line of `focus.json` (FR-008).
pub const FORMAT: &str = "imaging-focus/1";

/// How the focus region was chosen (FR-008, FR-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Found,
    Fixed,
}

/// The focus region as it is kept (FR-008, FR-009). `x` to `height` are sensor
/// pixels; a found one holds the reading it was found from, and a fixed one
/// holds none of the three.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KeptRegion {
    pub col: u32,
    pub row: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub source: Source,
    /// For a found one: the reading it was found from. Skipped while `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub px: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stars: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<u64>,
    /// Console Unix seconds.
    pub at: f64,
}

impl KeptRegion {
    /// The region found from a reading: its rectangle from `tile_region`, its
    /// size, star count, frame and time from the reading (FR-007, FR-008).
    pub fn found(sensor: Region, binning: u32, tile: Tile, reading: &FocusReading) -> Self {
        let region = tile_region(sensor, binning, tile);
        Self {
            col: tile.col,
            row: tile.row,
            x: region.x,
            y: region.y,
            width: region.width,
            height: region.height,
            source: Source::Found,
            px: Some(reading.px),
            stars: Some(reading.stars),
            frame_id: Some(reading.frame_id),
            at: reading.at,
        }
    }

    /// A region fixed by hand: its rectangle and the time, and no reading
    /// (FR-009).
    pub fn fixed(sensor: Region, binning: u32, tile: Tile, at: f64) -> Self {
        let region = tile_region(sensor, binning, tile);
        Self {
            col: tile.col,
            row: tile.row,
            x: region.x,
            y: region.y,
            width: region.width,
            height: region.height,
            source: Source::Fixed,
            px: None,
            stars: None,
            frame_id: None,
            at,
        }
    }

    /// The tile that was kept.
    pub fn tile(&self) -> Tile {
        Tile {
            col: self.col,
            row: self.row,
        }
    }
}

/// `focus.json`: the camera and the focus region, each left out while there is
/// none, and so is every `None` inside the region (FR-008). US3 adds the
/// record beside the region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Stored {
    format: String,
    camera: CameraId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    region: Option<KeptRegion>,
}

/// What the console keeps about focus for one camera (FR-008): its private
/// fields are the file's path, the camera and the region; US3 adds the record.
#[derive(Debug)]
pub struct Kept {
    /// `<imaging folder>/focus.json`.
    path: PathBuf,
    /// The camera the region belongs to; `None` before the camera is described.
    camera: Option<CameraId>,
    region: Option<KeptRegion>,
}

impl Kept {
    /// Nothing known yet. `path` is `<imaging folder>/focus.json`; nothing is
    /// read (FR-008).
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            camera: None,
            region: None,
        }
    }

    /// The camera was described (FR-008). For the camera already held this
    /// does nothing. Otherwise the file is read and what it holds for this
    /// camera is taken — a file that is missing, unreadable, of another
    /// format or of another camera answers nothing.
    pub fn describe(&mut self, camera: CameraId) {
        if self.camera.as_ref() == Some(&camera) {
            return;
        }
        self.region = std::fs::read(&self.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Stored>(&bytes).ok())
            .filter(|stored| stored.format == FORMAT && stored.camera == camera)
            .and_then(|stored| stored.region);
        self.camera = Some(camera);
    }

    /// The focus region kept for the camera (FR-008).
    pub fn region(&self) -> Option<KeptRegion> {
        self.region
    }

    /// Keep a region in place of the last, and write the file (FR-008). Before
    /// the camera is described this does nothing. A failed write is printed
    /// to standard error and the region stays in memory.
    pub fn keep_region(&mut self, region: KeptRegion) {
        if self.camera.is_none() {
            return;
        }
        self.region = Some(region);
        self.save();
    }

    /// Write everything held to the file (FR-008): whole under a `.part`
    /// name and renamed, creating the imaging folder first when it is
    /// missing.
    fn save(&self) {
        let stored = Stored {
            format: FORMAT.to_string(),
            camera: self.camera.clone().expect("the camera is described"),
            region: self.region,
        };
        let bytes = serde_json::to_vec(&stored).expect("the focus region's numbers are finite");
        let part = self.path.with_extension("json.part");
        let written: std::io::Result<()> = (|| {
            if let Some(folder) = self.path.parent() {
                std::fs::create_dir_all(folder)?;
            }
            std::fs::write(&part, bytes)?;
            std::fs::rename(&part, &self.path)
        })();
        if let Err(error) = written {
            eprintln!("focus: could not write {}: {error}", self.path.display());
        }
    }
}
