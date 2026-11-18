//! The ASTAP solver adapter (032): runs ASTAP's command-line program on a
//! prepared copy of the frame in a work folder, reads the `.ini` it leaves and
//! maps its exit codes. ASTAP itself is never built or linked here.

use std::{
    fs::File,
    path::Path,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

use super::{
    fits::{decode, encode, FitsImage},
    solver::{
        Answer, ImagingSolve, Prepared, Region, SolveError, SolveFrame, SolveHint, StarMeasure,
    },
};

/// For a solve whose hint has a place.
pub const SOLVE_LIMIT: Duration = Duration::from_secs(120);
/// For a solve whose hint has none: a search of the whole sky.
pub const WHOLE_SKY_LIMIT: Duration = Duration::from_secs(600);
pub const MEASURE_LIMIT: Duration = Duration::from_secs(30);
/// The signal-to-noise floor given to `-analyse`: the figure of the example on
/// ASTAP's own page (spec owner decision 24).
pub const ANALYSE_SNR: u32 = 30;
/// A prepared copy is halved while it has more rows than this.
pub const MAX_ROWS: u32 = 2500;

/// The ASTAP program and where its answers land.
#[derive(Debug, Clone)]
pub struct AstapConfig {
    pub program: std::path::PathBuf,
    pub database_name: String,
    pub database_dir: Option<std::path::PathBuf>,
    /// Each call makes, and removes, one folder in here.
    pub work_dir: std::path::PathBuf,
}

/// The ASTAP adapter: the program, the three limits and a count of its calls.
pub struct AstapSolver {
    config: AstapConfig,
    solve_limit: Duration,
    whole_sky_limit: Duration,
    measure_limit: Duration,
    calls: AtomicU64,
}

impl AstapSolver {
    /// With `SOLVE_LIMIT`, `WHOLE_SKY_LIMIT` and `MEASURE_LIMIT`.
    pub fn new(config: AstapConfig) -> Self {
        Self {
            config,
            solve_limit: SOLVE_LIMIT,
            whole_sky_limit: WHOLE_SKY_LIMIT,
            measure_limit: MEASURE_LIMIT,
            calls: AtomicU64::new(0),
        }
    }

    /// Other limits, for a test that runs in real time.
    pub fn with_limits(self, solve: Duration, whole_sky: Duration, measure: Duration) -> Self {
        Self {
            solve_limit: solve,
            whole_sky_limit: whole_sky,
            measure_limit: measure,
            ..self
        }
    }
}

impl super::solver::ImagingSolver for AstapSolver {
    fn name(&self) -> &'static str {
        "astap"
    }

    fn solve(&self, frame: SolveFrame, hint: SolveHint) -> super::solver::SolveFuture<'_> {
        let config = self.config.clone();
        let limit = if hint.position.is_none() {
            self.whole_sky_limit
        } else {
            self.solve_limit
        };
        let calls = &self.calls;
        Box::pin(async move {
            let n = calls.fetch_add(1, Ordering::Relaxed) + 1;
            let solved = tokio::task::spawn_blocking(move || {
                run(&config, n, &frame, None, Kind::Solve(hint), limit)
                    .and_then(|ran| read_solve(ran, &frame))
            })
            .await
            .unwrap_or_else(|_| {
                Err(SolveError::Failed(
                    "the imaging solver's call did not finish".to_string(),
                ))
            });
            solved
        })
    }

    fn measure(
        &self,
        frame: SolveFrame,
        region: Option<Region>,
    ) -> super::solver::MeasureFuture<'_> {
        let config = self.config.clone();
        let limit = self.measure_limit;
        let calls = &self.calls;
        Box::pin(async move {
            let n = calls.fetch_add(1, Ordering::Relaxed) + 1;
            let measured = tokio::task::spawn_blocking(move || {
                run(&config, n, &frame, region, Kind::Measure, limit).and_then(measure_call)
            })
            .await
            .unwrap_or_else(|_| {
                Err(SolveError::Failed(
                    "the imaging solver's call did not finish".to_string(),
                ))
            });
            measured
        })
    }
}

/// What one call runs: a solve with its hint, or a star measure.
enum Kind {
    Solve(SolveHint),
    Measure,
}

/// One call's outcome, before it is read: how the program ended, what it wrote,
/// the prepared copy it was given and how long the program took.
struct Ran {
    exit: Option<i32>,
    ini: Option<String>,
    stdout: String,
    prepared: Prepared,
    took_ms: f64,
}

/// One call, on the blocking pool in real time (plan.md § The ASTAP adapter):
/// make the work folder, read and prepare the frame, write the copy, run the
/// program to files in the folder, wait for it against a real-time deadline,
/// then remove the folder whatever happened.
fn run(
    config: &AstapConfig,
    n: u64,
    frame: &SolveFrame,
    region: Option<Region>,
    kind: Kind,
    limit: Duration,
) -> Result<Ran, SolveError> {
    let folder = config
        .work_dir
        .join(format!("solve-{}-{n}", std::process::id()));
    let answer = call(&folder, config, frame, region, kind, limit);
    let _ = std::fs::remove_dir_all(&folder);
    answer
}

/// One call's steps, inside the work folder.
fn call(
    folder: &Path,
    config: &AstapConfig,
    frame: &SolveFrame,
    region: Option<Region>,
    kind: Kind,
    limit: Duration,
) -> Result<Ran, SolveError> {
    std::fs::create_dir_all(folder).map_err(|error| {
        SolveError::Failed(format!(
            "the imaging solver's work folder could not be written: {error}"
        ))
    })?;
    let bytes = std::fs::read(&frame.path).map_err(|error| {
        SolveError::Failed(format!("the imaging frame could not be read: {error}"))
    })?;
    let image = decode(&bytes).map_err(|error| {
        SolveError::Failed(format!("the imaging frame could not be read: {error}"))
    })?;
    let (copy, prepared) =
        prepare(&image, frame.bayer.is_some(), region).map_err(SolveError::Failed)?;
    let input = folder.join("frame.fits");
    let base = folder.join("frame");
    std::fs::write(&input, encode(&copy)).map_err(|error| {
        SolveError::Failed(format!(
            "the imaging solver's work folder could not be written: {error}"
        ))
    })?;
    let args = match &kind {
        Kind::Solve(hint) => solve_args(
            &input,
            &base,
            hint,
            &config.database_name,
            config.database_dir.as_deref(),
        ),
        Kind::Measure => analyse_args(&input),
    };
    let stdout = folder.join("stdout.txt");
    let stderr = folder.join("stderr.txt");
    let stdout_file = File::create(&stdout).map_err(|error| {
        SolveError::Failed(format!(
            "the imaging solver's work folder could not be written: {error}"
        ))
    })?;
    let stderr_file = File::create(&stderr).map_err(|error| {
        SolveError::Failed(format!(
            "the imaging solver's work folder could not be written: {error}"
        ))
    })?;
    // The program's input is closed and its output goes to files in the folder,
    // so no pipe can fill.
    let started = Instant::now();
    let mut child = Command::new(&config.program)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .map_err(|error| {
            SolveError::Failed(format!("the imaging solver could not be started: {error}"))
        })?;
    let deadline = started + limit;
    let exit = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if Instant::now() >= deadline => {
                child.kill().ok();
                child.wait().ok();
                return Err(SolveError::TimedOut {
                    seconds: (limit.as_millis().div_ceil(1000)) as u64,
                });
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(_) => break None,
        }
    };
    let took_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(Ran {
        exit,
        ini: std::fs::read_to_string(folder.join("frame.ini")).ok(),
        stdout: std::fs::read_to_string(&stdout).unwrap_or_default(),
        prepared,
        took_ms,
    })
}

/// The solve an ended call answers, by the exit-code table of plan.md: 0 reads
/// the `.ini`, 1 is told apart by its `ERROR` line, 2 is too few stars, 16 an
/// unreadable frame, 32 and 33 a missing database, and everything else failed.
fn read_solve(ran: Ran, frame: &SolveFrame) -> Result<ImagingSolve, SolveError> {
    let detail = || ran.ini.as_deref().and_then(read_error);
    match ran.exit {
        Some(0) => match ran.ini.as_deref() {
            None => Err(SolveError::Failed(
                "the imaging solver's answer could not be read: no .ini file was written"
                    .to_string(),
            )),
            Some(ini) => match read_answer(ini) {
                Ok(answer) => {
                    let mut solve =
                        ImagingSolve::from_answer(&answer, ran.prepared, frame.width, frame.height);
                    solve.took_ms = ran.took_ms;
                    Ok(solve)
                }
                Err(why) => Err(SolveError::Failed(format!(
                    "the imaging solver's answer could not be read: {why}"
                ))),
            },
        },
        Some(1) => {
            let detail = detail();
            match detail.as_deref() {
                Some("Not enough stars.") => Err(SolveError::TooFewStars { detail }),
                Some("No star database found.") => Err(SolveError::NoDatabase { detail }),
                other => Err(SolveError::NotSolved {
                    detail: Some(other.unwrap_or("No solution found!").to_string()),
                }),
            }
        }
        Some(2) => Err(SolveError::TooFewStars {
            detail: detail().map(|text| text.to_string()),
        }),
        Some(16) => Err(SolveError::Failed(
            "the imaging solver could not read the imaging frame".to_string(),
        )),
        Some(32 | 33) => Err(SolveError::NoDatabase {
            detail: detail().map(|text| text.to_string()),
        }),
        Some(code) => Err(SolveError::Failed(format!(
            "the imaging solver ended with exit code {code}"
        ))),
        None => Err(SolveError::Failed(
            "the imaging solver ended without an exit code".to_string(),
        )),
    }
}

/// The star measure an ended call answers: the two printed lines are read
/// whatever the exit code, the diameter in the frame's own pixels.
fn measure_call(ran: Ran) -> Result<StarMeasure, SolveError> {
    match read_measure(&ran.stdout) {
        Ok(measure) if measure.stars > 0 && measure.half_flux_px > 0.0 => Ok(StarMeasure {
            half_flux_px: measure.half_flux_px * f64::from(ran.prepared.k),
            stars: measure.stars,
        }),
        Ok(_) => Err(SolveError::TooFewStars { detail: None }),
        Err(_) if ran.exit == Some(16) => Err(SolveError::Failed(
            "the imaging solver could not read the imaging frame".to_string(),
        )),
        Err(error) => Err(error),
    }
}

/// The arguments of a solve, in ASTAP's own order: the input and the output
/// base, the hint's place in hours and in declination plus 90°, its radius held
/// to 180°, its field height or `0` without one, `-z 1`, the database name and,
/// with one, the database folder.
pub fn solve_args(
    input: &Path,
    base: &Path,
    hint: &SolveHint,
    database_name: &str,
    database_dir: Option<&Path>,
) -> Vec<String> {
    // A hint with no place is sent as RA 0 h, Dec 0°.
    let (ra, dec) = hint.position.unwrap_or((0.0, 0.0));
    let mut args = vec![
        "-f".to_string(),
        input.display().to_string(),
        "-o".to_string(),
        base.display().to_string(),
        "-ra".to_string(),
        format!("{:.6}", ra / 15.0),
        "-spd".to_string(),
        format!("{:.5}", dec + 90.0),
        "-r".to_string(),
        format!("{:.2}", hint.radius_deg.min(180.0)),
        "-fov".to_string(),
    ];
    match hint.fov_deg {
        Some(fov_deg) => args.push(format!("{fov_deg:.4}")),
        None => args.push("0".to_string()),
    }
    for word in ["-z", "1", "-D", database_name] {
        args.push(word.to_string());
    }
    if let Some(dir) = database_dir {
        args.push("-d".to_string());
        args.push(dir.display().to_string());
    }
    args
}

/// The arguments of a star measure.
pub fn analyse_args(input: &Path) -> Vec<String> {
    vec![
        "-f".to_string(),
        input.display().to_string(),
        "-analyse".to_string(),
        ANALYSE_SNR.to_string(),
    ]
}

/// The `KEY=value` lines of an `.ini` or of `-analyse`'s output, both sides
/// trimmed, a line with no `=` skipped.
fn lines(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect()
}

/// The value of a key, when the text holds it.
fn value<'a>(lines: &'a [(String, String)], key: &str) -> Option<&'a str> {
    lines
        .iter()
        .find(|(found, _)| found == key)
        .map(|(_, value)| value.as_str())
}

/// The solve an `.ini` holds, or why it holds none.
pub fn read_answer(ini: &str) -> Result<Answer, String> {
    let lines = lines(ini);
    if value(&lines, "PLTSOLVD") != Some("T") {
        return Err("PLTSOLVD is not T".to_string());
    }
    let number = |key: &str| {
        value(&lines, key)
            .ok_or_else(|| format!("{key} is missing"))
            .and_then(|text| {
                text.parse::<f64>()
                    .map_err(|_| format!("{key} is not a number"))
            })
    };
    let mut answer = Answer {
        crpix: [0.0; 2],
        crval: [0.0; 2],
        cd: [[0.0; 2]; 2],
        warning: None,
    };
    answer.crpix[0] = number("CRPIX1")?;
    answer.crpix[1] = number("CRPIX2")?;
    answer.crval[0] = number("CRVAL1")?;
    answer.crval[1] = number("CRVAL2")?;
    answer.cd[0][0] = number("CD1_1")?;
    answer.cd[0][1] = number("CD1_2")?;
    answer.cd[1][0] = number("CD2_1")?;
    answer.cd[1][1] = number("CD2_2")?;
    if let Some(warning) = value(&lines, "WARNING").filter(|text| !text.is_empty()) {
        answer.warning = Some(warning.to_string());
    }
    Ok(answer)
}

/// The text of an `.ini`'s `ERROR` line, trimmed, when it has one that is not
/// empty. An exit code of 1 is told apart by it.
pub fn read_error(ini: &str) -> Option<String> {
    value(&lines(ini), "ERROR")
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// The two lines `-analyse` prints: the median half-flux diameter and the star
/// count, in the prepared copy's pixels.
pub fn read_measure(stdout: &str) -> Result<StarMeasure, SolveError> {
    let lines = lines(stdout);
    let diameter = value(&lines, "HFD_MEDIAN").and_then(|text| text.parse::<f64>().ok());
    let stars = value(&lines, "STARS").and_then(|text| text.parse::<u32>().ok());
    match (diameter, stars) {
        (Some(half_flux_px), Some(stars)) => Ok(StarMeasure {
            half_flux_px,
            stars,
        }),
        _ => Err(SolveError::Failed(
            "the imaging solver's measure could not be read".to_string(),
        )),
    }
}

/// The samples of an image as numbers: 16-bit little-endian pairs, or 8-bit
/// bytes.
fn numbers(image: &FitsImage) -> Vec<u32> {
    match image.bits {
        16 => image
            .samples
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u32::from(u16::from_le_bytes([pair[0], pair[1]])))
            .collect(),
        _ => image.samples.iter().map(|byte| u32::from(*byte)).collect(),
    }
}

/// One 2 × 2 cell's mean of four numbers, rounded half up.
fn mean(a: u32, b: u32, c: u32, d: u32) -> u32 {
    (a + b + c + d + 2) / 4
}

/// Halve a grid of samples 2 × 2, dropping an odd last row or column.
fn halve(grid: &[u32], width: usize, height: usize) -> (Vec<u32>, usize, usize) {
    let (w, h) = (width / 2, height / 2);
    let mut out = Vec::with_capacity(w * h);
    for row in 0..h {
        for column in 0..w {
            let i = row * 2 * width + column * 2;
            out.push(mean(
                grid[i],
                grid[i + 1],
                grid[i + width],
                grid[i + width + 1],
            ));
        }
    }
    (out, w, h)
}

/// The prepared copy of a frame, or of a region of it (FR-004): the region cut
/// first and moved inward to whole cells for a frame with a Bayer pattern, each
/// whole cell averaged into one sample, then halved 2 × 2 while it is more than
/// `MAX_ROWS` rows high. A 16-bit picture with the frame's top row first.
pub fn prepare(
    image: &FitsImage,
    bayer: bool,
    region: Option<Region>,
) -> Result<(FitsImage, Prepared), String> {
    let (width, height) = (image.width as usize, image.height as usize);
    let (mut x0, mut y0, mut w, mut h) = match region {
        None => (0, 0, width, height),
        Some(region) => (
            region.x as usize,
            region.y as usize,
            region.width as usize,
            region.height as usize,
        ),
    };
    if w == 0 || h == 0 || x0 + w > width || y0 + h > height {
        return Err("the region is outside the imaging frame".to_string());
    }
    let mut k = 1;
    if bayer {
        // The region moves inward to whole cells: its left and top edges up to
        // the next even number, its right and bottom edges down to the previous
        // even one.
        let x1 = (x0 + w) & !1;
        let y1 = (y0 + h) & !1;
        x0 = (x0 + 1) & !1;
        y0 = (y0 + 1) & !1;
        if x1 <= x0 || y1 <= y0 {
            return Err("the region is too small".to_string());
        }
        w = x1 - x0;
        h = y1 - y0;
    }
    let all = numbers(image);
    let mut grid: Vec<u32> = Vec::with_capacity(w * h);
    for row in y0..y0 + h {
        grid.extend(&all[row * width + x0..row * width + x0 + w]);
    }
    if bayer {
        let (halved, halved_w, halved_h) = halve(&grid, w, h);
        (grid, w, h) = (halved, halved_w, halved_h);
        k = 2;
    }
    while h > MAX_ROWS as usize {
        let (halved, halved_w, halved_h) = halve(&grid, w, h);
        (grid, w, h) = (halved, halved_w, halved_h);
        k *= 2;
    }
    let mut samples = Vec::with_capacity(w * h * 2);
    for number in &grid {
        samples.extend_from_slice(&(*number as u16).to_le_bytes());
    }
    Ok((
        FitsImage {
            width: w as u32,
            height: h as u32,
            bits: 16,
            samples,
            cards: Vec::new(),
        },
        Prepared {
            x0: x0 as u32,
            y0: y0 as u32,
            k,
            width: w as u32,
            height: h as u32,
        },
    ))
}
