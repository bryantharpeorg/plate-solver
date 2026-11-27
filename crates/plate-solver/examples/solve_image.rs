//! Solve one or more sky images from the command line.
//!
//! ```text
//! cargo run --release -p plate-solver --example solve_image -- \
//!     --db hip_30_60.npz [--fov 50] [--fov-max-error 10] [--distortion -0.08] \
//!     [--bin 2] [--sigma 8] [--noise-floor F] [--saturated] [--stars 30] [--dark dark.png] [--timeout 5000] [--json] \
//!     frame1.png [frame2.jpg ...]
//! ```
//!
//! * `--db` — a pattern database built for this camera's field of view (e.g. with
//!   `database-generation hip_main.dat out.npz --min-fov 30 --max-fov 60`), or a
//!   cedar/tetra3 `.npz`.
//! * `--fov` — horizontal FOV estimate in degrees. Leave it off for the first solve;
//!   a wrong hint with a tight `--fov-max-error` prevents solving.
//! * `--distortion` — radial distortion `k` (negative = barrel) measured from an
//!   earlier solve of the same lens. Speeds up and steadies wide-field solves.
//! * `--dark` — dark frame (lens capped, same exposure/gain) subtracted before
//!   detection. Removes hot pixels, which the binned detector does not reject.
//! * `--noise-floor` — the scope node's `noise_floor`, in 8-bit levels of the
//!   detection image. Leave it off for cedar's own 0.2.
//! * `--saturated` — keep the saturated stars cedar sets aside, by their brightness, as the
//!   scope node does (`star_detection::saturated`), and print how many regions were kept
//!   and set aside. It does not combine with `--dark`: the node subtracts no dark.
//!
//! Frames are read as cedar's pipeline reads them: color turned gray with PIL's weights
//! (`star_detection::gray`) and stars detected with cedar's settings
//! (`DetectParams::cedar()`: rows levelled, hot pixels rejected), as the finder console does.

use std::path::Path;
use std::time::Instant;

use plate_solver::{solve_from_centroids, DetectParams, SolveStatus};
use star_detection::saturated::{saturated_stars, with_saturated_stars, SetAside};
use star_detection::{detect_stars, detect_stars_with_noise_floor, gray};

struct Args {
    db: String,
    fov: Option<f64>,
    fov_max_error: f64,
    distortion: f64,
    binning: usize,
    sigma: f64,
    noise_floor: Option<f64>,
    saturated: bool,
    stars: usize,
    dark: Option<String>,
    timeout: u64,
    json: bool,
    images: Vec<String>,
}

fn parse_args() -> Args {
    let cedar = DetectParams::cedar();
    let mut a = Args {
        db: String::new(),
        fov: None,
        fov_max_error: 10.0,
        distortion: 0.0,
        binning: cedar.binning,
        sigma: cedar.sigma,
        noise_floor: None,
        saturated: false,
        stars: 30,
        dark: None,
        timeout: 5000,
        json: false,
        images: Vec::new(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut val = || {
            it.next()
                .unwrap_or_else(|| usage(&format!("{arg} needs a value")))
        };
        match arg.as_str() {
            "--db" => a.db = val(),
            "--fov" => a.fov = Some(val().parse().unwrap_or_else(|_| usage("bad --fov"))),
            "--fov-max-error" => {
                a.fov_max_error = val()
                    .parse()
                    .unwrap_or_else(|_| usage("bad --fov-max-error"))
            }
            "--distortion" => {
                a.distortion = val().parse().unwrap_or_else(|_| usage("bad --distortion"))
            }
            "--bin" => a.binning = val().parse().unwrap_or_else(|_| usage("bad --bin")),
            "--sigma" => a.sigma = val().parse().unwrap_or_else(|_| usage("bad --sigma")),
            "--noise-floor" => {
                a.noise_floor = Some(val().parse().unwrap_or_else(|_| usage("bad --noise-floor")))
            }
            "--saturated" => a.saturated = true,
            "--stars" => a.stars = val().parse().unwrap_or_else(|_| usage("bad --stars")),
            "--dark" => a.dark = Some(val()),
            "--timeout" => a.timeout = val().parse().unwrap_or_else(|_| usage("bad --timeout")),
            "--json" => a.json = true,
            "-h" | "--help" => usage(""),
            s if s.starts_with("--") => usage(&format!("unknown option {s}")),
            s => a.images.push(s.to_string()),
        }
    }
    if a.saturated && a.dark.is_some() {
        usage("--saturated does not combine with --dark: the scope node subtracts no dark");
    }
    if a.db.is_empty() || a.images.is_empty() {
        usage("need --db and at least one image");
    }
    a
}

fn usage(msg: &str) -> ! {
    if !msg.is_empty() {
        eprintln!("error: {msg}");
    }
    eprintln!("usage: solve_image --db DB.npz [--fov DEG] [--fov-max-error DEG] [--distortion K] [--bin 1|2|4|8] [--sigma S] [--noise-floor F] [--saturated] [--stars N] [--dark DARK] [--timeout MS] [--json] IMAGE...");
    std::process::exit(2)
}

fn load_gray(path: &str) -> (Vec<u8>, usize, usize) {
    let img = image::ImageReader::open(path)
        .and_then(|r| r.with_guessed_format())
        .unwrap_or_else(|e| usage(&format!("cannot open {path}: {e}")))
        .decode()
        .unwrap_or_else(|e| usage(&format!("cannot decode {path}: {e}")));
    let (w, h) = (img.width() as usize, img.height() as usize);
    // Color turns gray as cedar turns it (PIL's weights), not with `to_luma8`'s Rec. 709.
    let pixels = if img.color().has_color() {
        gray::from_rgb8(img.to_rgb8().as_raw())
    } else {
        img.to_luma8().into_raw()
    };
    (pixels, w, h)
}

fn main() {
    let args = parse_args();
    let t = Instant::now();
    let db = pattern_database::load_from_path(Path::new(&args.db))
        .unwrap_or_else(|e| usage(&format!("cannot load {}: {e}", args.db)));
    eprintln!(
        "database: {} stars, {} patterns, FOV {}-{} deg, loaded in {:.1} s",
        db.num_stars,
        db.properties.num_patterns,
        db.properties.min_fov,
        db.properties.max_fov,
        t.elapsed().as_secs_f64()
    );
    let dark = args.dark.as_deref().map(load_gray);
    let cedar = DetectParams::cedar();

    for path in &args.images {
        let (mut img, w, h) = load_gray(path);
        if let Some((d, dw, dh)) = &dark {
            if (*dw, *dh) != (w, h) {
                usage("dark frame size differs from the image");
            }
            for (p, q) in img.iter_mut().zip(d) {
                *p = p.saturating_sub(*q);
            }
        }

        let t = Instant::now();
        let stars = match args.noise_floor {
            None => detect_stars(
                &img,
                w,
                h,
                args.sigma,
                args.binning,
                cedar.normalize_rows,
                cedar.detect_hot_pixels,
            ),
            Some(floor) => detect_stars_with_noise_floor(
                &img,
                w,
                h,
                args.sigma,
                args.binning,
                cedar.normalize_rows,
                cedar.detect_hot_pixels,
                floor,
            ),
        };
        let mut saturated_kept = 0;
        let mut saturated_set_aside = 0;
        let stars = if args.saturated {
            let (kept, set_aside) = saturated_stars(&img, w, h, &stars);
            saturated_kept = kept.len();
            saturated_set_aside = set_aside.len();
            if !args.json {
                let count = |reason| set_aside.iter().filter(|&&r| r == reason).count();
                println!(
                    "{path}: saturated stars kept {saturated_kept}, set aside {saturated_set_aside} (small {}, edge {}, elongated {}, extended {})",
                    count(SetAside::Small), count(SetAside::Edge),
                    count(SetAside::Elongated), count(SetAside::Extended)
                );
            }
            with_saturated_stars(&img, w, h, stars)
        } else {
            stars
        };
        let t_detect = t.elapsed().as_secs_f64() * 1e3;
        // The solver wants (y, x); `Star` has named x (column) / y (row).
        let centroids: Vec<(f64, f64)> =
            stars.iter().take(args.stars).map(|s| (s.y, s.x)).collect();

        let t = Instant::now();
        let sol = solve_from_centroids(
            &centroids,
            (w, h),
            args.fov,
            args.fov_max_error,
            0.01,
            1e-5,
            args.timeout,
            args.distortion,
            0.002,
            &db,
        );
        let t_solve = t.elapsed().as_secs_f64() * 1e3;

        let status = match sol.status {
            Some(SolveStatus::MatchFound) => "MATCH_FOUND",
            Some(SolveStatus::NoMatch) => "NO_MATCH",
            Some(SolveStatus::Timeout) => "TIMEOUT",
            Some(SolveStatus::Cancelled) => "CANCELLED",
            Some(SolveStatus::TooFew) => "TOO_FEW",
            None => "UNKNOWN",
        };
        let deg = |v: Option<f64>| v.map(f64::to_degrees);
        let (ra, dec, roll) = (deg(sol.ra), deg(sol.dec), deg(sol.roll));
        let fov = sol.camera.map(|c| c.fov.to_degrees());

        if args.json {
            let f = |v: Option<f64>| v.map_or("null".to_string(), |x| format!("{x:.6}"));
            let saturated = if args.saturated {
                format!(",\"saturated_kept\":{saturated_kept},\"saturated_set_aside\":{saturated_set_aside}")
            } else {
                String::new()
            };
            println!(
                "{{\"image\":{path:?},\"status\":\"{status}\",\"ra\":{},\"dec\":{},\"roll\":{},\"fov\":{},\"distortion\":{},\"matches\":{},\"rmse_arcsec\":{},\"stars_detected\":{},\"t_detect_ms\":{t_detect:.1},\"t_solve_ms\":{t_solve:.1}{saturated}}}",
                f(ra), f(dec), f(roll), f(fov), f(sol.distortion), sol.matched_centroids.len(), f(sol.rmse), stars.len()
            );
        } else if let (Some(ra), Some(dec), Some(roll), Some(fov)) = (ra, dec, roll, fov) {
            println!(
                "{path}: {status}  RA {ra:.4}  Dec {dec:+.4}  Roll {roll:.2}  FOV {fov:.2} deg  k {:+.3}  matches {}  RMSE {:.0}\"  ({} stars, detect {t_detect:.0} ms, solve {t_solve:.0} ms)",
                sol.distortion.unwrap_or(0.0),
                sol.matched_centroids.len(),
                sol.rmse.unwrap_or(f64::NAN),
                stars.len()
            );
            println!(
                "  sky map: https://aladin.cds.unistra.fr/AladinLite/?target={ra:.4}%20{dec:+.4}&fov={:.0}",
                fov * 1.2
            );
        } else {
            println!("{path}: {status}  ({} stars detected, detect {t_detect:.0} ms, solve {t_solve:.0} ms)", stars.len());
        }
    }
}
