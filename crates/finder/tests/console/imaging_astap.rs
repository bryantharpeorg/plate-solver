//! The ASTAP solver adapter (032 US1): a worked answer read as an imaging solve,
//! the exact arguments, every exit code turned into its reason, the limits, the
//! prepared copy and the saved captures, all against the stub program of the
//! shared setup in real time. Oracles: the two worked answers and their
//! arithmetic, ASTAP's documents, the test frame's rule and the saved captures
//! under `fixtures/finder/astap/`.

use crate::support::Scratch;
use finder::imaging::{
    astap::{self, AstapConfig, AstapSolver},
    solver::{
        HintSource, ImagingSolve, ImagingSolver, Prepared, Region, SolveError, SolveFrame,
        SolveHint, StarMeasure,
    },
};
use math_core::{angular_distance, UnitVector};
use serde_json::Value;
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Worked answer A (the shared setup): the example `.ini` on ASTAP's own page,
/// for a picture 2328 × 1760.
const WORKED_A: &str = "PLTSOLVD=T
CRPIX1= 1.1645000000000000E+003
CRPIX2= 8.8050000000000000E+002
CRVAL1= 1.5463033992314939E+002
CRVAL2= 2.2039358425145043E+001
CDELT1=-7.4798001762187193E-004
CDELT2= 7.4845252983311850E-004
CROTA1=-1.1668387329628058E+000
CROTA2=-1.1900321176194073E+000
CD1_1=-7.4781868711882519E-004
CD1_2= 1.5241315209850368E-005
CD2_1= 1.5534412042060001E-005
CD2_2= 7.4829732842251226E-004
CMDLINE=astap_cli -f frame.fits
";

/// Worked answer B (the shared setup): hand-made for an imaging frame 1236 × 822
/// with the Bayer pattern `RGGB`, whose prepared copy is 618 × 411.
const WORKED_B: &str = "PLTSOLVD=T
CRPIX1= 3.0950000000000000E+002
CRPIX2= 2.0600000000000000E+002
CRVAL1= 8.3822100000000006E+001
CRVAL2=-5.3910999999999998E+000
CDELT1=-1.2000000000000000E-003
CDELT2= 1.2000000000000000E-003
CROTA1= 3.0000000000000000E+001
CROTA2= 3.0000000000000000E+001
CD1_1=-1.0392304845413263E-003
CD1_2=-6.0000000000000000E-004
CD2_1= 6.0000000000000000E-004
CD2_2=-1.0392304845413263E-003
CMDLINE=astap_cli -f frame.fits
WARNING=Warning scale was inaccurate! Set FOV=0.49d
";

/// The stub program of the shared setup, which stands in for `astap_cli`.
const STUB: &str = r#"#!/bin/sh
dir=$(dirname "$0")
: > "$dir/args"
base=
input=
while [ $# -gt 0 ]; do
  printf '%s\n' "$1" >> "$dir/args"
  case "$1" in
    -o) base=$2 ;;
    -f) input=$2 ;;
  esac
  shift
done
[ -n "$input" ] && cp "$input" "$dir/input.fits"
[ -n "$base" ] && [ -f "$dir/reply.ini" ] && cp "$dir/reply.ini" "$base.ini"
[ -n "$base" ] && [ -f "$dir/reply.wcs" ] && cp "$dir/reply.wcs" "$base.wcs"
[ -f "$dir/reply.stdout" ] && cat "$dir/reply.stdout"
exit "$(cat "$dir/code")"
"#;

/// The program of US1-S5: it writes its process id, then sleeps.
const SLEEPER: &str = "#!/bin/sh\necho $$ > \"$(dirname \"$0\")/pid\"\nexec sleep 30\n";

/// The stub adapter (the shared setup): the stub program, its `code` and
/// whichever `reply.*` files the scenario names beside it, and the work folder
/// `work` under the same scratch folder.
fn stub(scratch: &Path, code: &str, files: &[(&str, &str)]) -> AstapSolver {
    let program = scratch.join("stub");
    std::fs::write(&program, STUB).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(scratch.join("code"), code).unwrap();
    for (name, text) in files {
        std::fs::write(scratch.join(name), text).unwrap();
    }
    AstapSolver::new(AstapConfig {
        program,
        database_name: "d50".to_string(),
        database_dir: Some(PathBuf::from("/opt/astap")),
        work_dir: scratch.join("work"),
    })
}

/// One 80-byte value card: the keyword padded to 8, `= `, the value.
fn card(keyword: &str, value: &str) -> Vec<u8> {
    let mut card = format!("{keyword:<8}= {value:>20}").into_bytes();
    card.resize(80, b' ');
    card
}

/// A 16-bit FITS file as this test writes it (the shared setup): the structural
/// cards, `END`, the header padded with spaces and the data with zeros to
/// multiples of 2880, each big-endian sample stored as the sample less 32768.
fn fits(width: u32, height: u32, samples: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(card("SIMPLE", "T"));
    bytes.extend(card("BITPIX", "16"));
    bytes.extend(card("NAXIS", "2"));
    bytes.extend(card("NAXIS1", &width.to_string()));
    bytes.extend(card("NAXIS2", &height.to_string()));
    bytes.extend(card("BZERO", "32768"));
    bytes.extend(card("BSCALE", "1"));
    let mut end = vec![b' '; 80];
    end[..3].copy_from_slice(b"END");
    bytes.extend(end);
    while bytes.len() % 2880 != 0 {
        bytes.push(b' ');
    }
    for sample in samples {
        bytes.extend_from_slice(&((i32::from(*sample) - 32768) as i16).to_be_bytes());
    }
    while bytes.len() % 2880 != 0 {
        bytes.push(0);
    }
    bytes
}

/// A FITS file read with this test's own code, never the console's (the shared
/// setup): the cards up to `END`, the header padded to 2880 bytes, then the
/// big-endian samples, each 16-bit sample the stored signed value plus 32768.
fn read_fits(bytes: &[u8]) -> (u32, u32, Vec<u16>) {
    let (mut width, mut height) = (0, 0);
    let mut end = None;
    for offset in (0..bytes.len()).step_by(80) {
        let card = &bytes[offset..offset + 80];
        let keyword = String::from_utf8_lossy(&card[..8]).trim_end().to_string();
        if keyword == "END" {
            end = Some(offset + 80);
            break;
        }
        let value = String::from_utf8_lossy(&card[10..]).trim().to_string();
        match keyword.as_str() {
            "NAXIS1" => width = value.parse().unwrap(),
            "NAXIS2" => height = value.parse().unwrap(),
            _ => {}
        }
    }
    let mut start = end.expect("an END card");
    while start % 2880 != 0 {
        start += 1;
    }
    let samples = bytes[start..]
        .chunks(2)
        .take((width * height) as usize)
        .map(|chunk| (i16::from_be_bytes([chunk[0], chunk[1]]) as i32 + 32768) as u16)
        .collect();
    (width, height, samples)
}

/// The sample at column x, row y.
fn sample(samples: &[u16], width: u32, x: u32, y: u32) -> u16 {
    samples[(y * width + x) as usize]
}

/// The test frame (the shared setup): a 16-bit FITS file whose sample at column
/// c, row r is `1000 + 4·c + 8·r`, written with this test's own writer. Answers
/// the `SolveFrame` and the bytes written.
fn test_frame(
    scratch: &Path,
    name: &str,
    width: u32,
    height: u32,
    bayer: Option<&str>,
) -> (SolveFrame, Vec<u8>) {
    let samples: Vec<u16> = (0..width * height)
        .map(|i| (1000 + 4 * (i % width) + 8 * (i / width)) as u16)
        .collect();
    write_frame(scratch, name, width, height, bayer, &samples)
}

/// A 16-bit FITS file of these samples, and the `SolveFrame` that names it.
fn write_frame(
    scratch: &Path,
    name: &str,
    width: u32,
    height: u32,
    bayer: Option<&str>,
    samples: &[u16],
) -> (SolveFrame, Vec<u8>) {
    let bytes = fits(width, height, samples);
    std::fs::write(scratch.join(name), &bytes).unwrap();
    (
        SolveFrame {
            path: scratch.join(name),
            width,
            height,
            bits: 16,
            bayer: bayer.map(str::to_string),
            binning: 1,
            roi: Region {
                x: 0,
                y: 0,
                width,
                height,
            },
            start: 0.0,
            exposure_s: 1.0,
        },
        bytes,
    )
}

/// The separation of two J2000 places in arcseconds, computed the shared setup's
/// way: `math_core::angular_distance` over `UnitVector::from_radec`.
fn separation_arcsec(from: (f64, f64), to: (f64, f64)) -> f64 {
    angular_distance(
        UnitVector::from_radec(from.0.to_radians(), from.1.to_radians()),
        UnitVector::from_radec(to.0.to_radians(), to.1.to_radians()),
    )
    .to_degrees()
        * 3600.0
}

/// The position angle of one place seen from another, east of north in 0 to 360,
/// from the shared setup's spherical formula.
fn position_angle_deg(from: (f64, f64), to: (f64, f64)) -> f64 {
    let (ra1, dec1, ra2, dec2) = (
        from.0.to_radians(),
        from.1.to_radians(),
        to.0.to_radians(),
        to.1.to_radians(),
    );
    ((ra2 - ra1).sin() * dec2.cos())
        .atan2(dec1.cos() * dec2.sin() - dec1.sin() * dec2.cos() * (ra2 - ra1).cos())
        .to_degrees()
        .rem_euclid(360.0)
}

/// A hint at a place, with a radius and an optional field height.
fn hint(ra_deg: f64, dec_deg: f64, radius_deg: f64, fov_deg: Option<f64>) -> SolveHint {
    SolveHint {
        source: HintSource::Pointing,
        position: Some((ra_deg, dec_deg)),
        radius_deg,
        fov_deg,
    }
}

/// The argument that follows a key.
fn value_after<'a>(args: &'a [String], key: &str) -> &'a str {
    args.windows(2)
        .find(|pair| pair[0] == key)
        .map(|pair| pair[1].as_str())
        .unwrap()
}

/// The work folder is left empty whatever the call answered (US1-S2, US1-S4).
fn assert_work_empty(work: &Path) {
    let entries: Vec<_> = std::fs::read_dir(work).unwrap().collect();
    assert!(entries.is_empty(), "the work folder holds no entry");
}

/// US1-S1: worked answer A reads as a solve of the frame itself, with its centre,
/// its up angle, its scale, its mirror sense and its linear part.
#[tokio::test]
async fn worked_answer_a_reads_as_an_imaging_solve() {
    let answer = astap::read_answer(WORKED_A).unwrap();
    let solve = ImagingSolve::from_answer(
        &answer,
        Prepared {
            x0: 0,
            y0: 0,
            k: 1,
            width: 2328,
            height: 1760,
        },
        2328,
        1760,
    );
    assert!((solve.ra_deg - 154.6303399231494).abs() < 1e-9);
    assert!((solve.dec_deg - 22.039358425145043).abs() < 1e-9);
    assert!((solve.up_east_of_north_deg - 181.16684).abs() < 1e-4);
    assert!((solve.arcsec_per_px - 2.69358).abs() < 1e-4);
    assert!(solve.mirrored);
    let expected = [
        [-7.478186871188252e-4, 1.5241315209850368e-5],
        [1.553441204206e-5, 7.482973284225123e-4],
    ];
    for (row, expected) in solve.cd_deg.iter().zip(expected) {
        for (part, expected) in row.iter().zip(expected) {
            assert!((part - expected).abs() < 1e-18);
        }
    }
    assert_eq!(solve.warning, None);
}

/// US1-S2: the stub adapter solves a Bayer frame through a 618 × 411 prepared
/// copy, with exactly eighteen arguments, and leaves the frame and the work
/// folder as they were.
#[tokio::test]
async fn solve_prepares_a_copy_and_reads_worked_answer_b() {
    let scratch = Scratch::new("imaging-astap-solve");
    let solver = stub(&scratch.0, "0", &[("reply.ini", WORKED_B)]);
    let (frame, bytes) = test_frame(&scratch.0, "frame.fits", 1236, 822, Some("RGGB"));
    let solve = solver
        .solve(frame, hint(83.8221, -5.3911, 3.0, Some(0.4997)))
        .await
        .unwrap();
    assert!((solve.ra_deg - 83.8221).abs() < 1e-9);
    assert!((solve.dec_deg - (-5.3911)).abs() < 1e-9);
    assert!((solve.up_east_of_north_deg - 30.0).abs() < 1e-9);
    assert!((solve.arcsec_per_px - 2.16).abs() < 1e-9);
    assert!(!solve.mirrored);
    assert_eq!(solve.stars, None);
    assert_eq!(solve.width, 1236);
    assert_eq!(solve.height, 822);
    assert_eq!(
        solve.warning.as_deref(),
        Some("Warning scale was inaccurate! Set FOV=0.49d")
    );
    let place = solve.sky_at((618.0, 311.0));
    let separation = separation_arcsec((solve.ra_deg, solve.dec_deg), place);
    assert!(
        (separation - 216.0).abs() < 0.01,
        "216 arcseconds from the centre, found {separation}"
    );
    let angle = position_angle_deg((solve.ra_deg, solve.dec_deg), place);
    assert!(
        (angle - 30.0).abs() < 0.001,
        "at position angle 30 degrees, found {angle}"
    );
    let pixel = solve.pixel_of(place.0, place.1).unwrap();
    assert!(
        (pixel.0 - 618.0).abs() < 1e-6 && (pixel.1 - 311.0).abs() < 1e-6,
        "back at its own pixel, found {pixel:?}"
    );
    let written = std::fs::read_to_string(scratch.0.join("args")).unwrap();
    let args: Vec<&str> = written.lines().collect();
    assert_eq!(args.len(), 18, "exactly eighteen arguments");
    let work = scratch.0.join("work").display().to_string();
    assert_eq!(args[0], "-f");
    assert!(
        args[1].starts_with(&work) && args[1].ends_with("/frame.fits"),
        "the input is the copy in the work folder"
    );
    assert_eq!(args[2], "-o");
    assert_eq!(args[3], args[1].strip_suffix(".fits").unwrap());
    assert_eq!(args[4], "-ra");
    assert_eq!(args[5], "5.588140");
    assert_eq!(args[6], "-spd");
    assert_eq!(args[7], "84.60890");
    assert_eq!(args[8], "-r");
    assert_eq!(args[9], "3.00");
    assert_eq!(args[10], "-fov");
    assert_eq!(args[11], "0.4997");
    assert_eq!(args[12], "-z");
    assert_eq!(args[13], "1");
    assert_eq!(args[14], "-D");
    assert_eq!(args[15], "d50");
    assert_eq!(args[16], "-d");
    assert_eq!(args[17], "/opt/astap");
    let (copy_width, copy_height, samples) =
        read_fits(&std::fs::read(scratch.0.join("input.fits")).unwrap());
    assert_eq!((copy_width, copy_height), (618, 411));
    assert_eq!(sample(&samples, copy_width, 0, 0), 1006);
    assert_eq!(sample(&samples, copy_width, 5, 3), 1094);
    assert_work_empty(&scratch.0.join("work"));
    assert_eq!(
        std::fs::read(scratch.0.join("frame.fits")).unwrap(),
        bytes,
        "the saved frame's bytes are what the test wrote"
    );
}

/// US1-S3: the arguments are ASTAP's own forms, in hours, in declination plus
/// 90°, with the radius held to 180 and `0` without a field height.
#[tokio::test]
async fn solve_args_and_analyse_args_match_astaps_documents() {
    let input = Path::new("/w/frame.fits");
    let base = Path::new("/w/frame");
    let none = SolveHint {
        source: HintSource::None,
        position: None,
        radius_deg: 180.0,
        fov_deg: None,
    };
    assert_eq!(
        astap::solve_args(input, base, &none, "d50", None),
        [
            "-f",
            "/w/frame.fits",
            "-o",
            "/w/frame",
            "-ra",
            "0.000000",
            "-spd",
            "90.00000",
            "-r",
            "180.00",
            "-fov",
            "0",
            "-z",
            "1",
            "-D",
            "d50",
        ]
    );
    let pole = SolveHint {
        source: HintSource::Pointing,
        position: Some((10.0, -90.0)),
        radius_deg: 500.0,
        fov_deg: None,
    };
    let args = astap::solve_args(input, base, &pole, "d50", None);
    assert_eq!(value_after(&args, "-ra"), "0.666667");
    assert_eq!(value_after(&args, "-spd"), "0.00000");
    assert_eq!(value_after(&args, "-r"), "180.00");
    assert_eq!(
        astap::analyse_args(input),
        ["-f", "/w/frame.fits", "-analyse", "30"]
    );
}

/// One stub-adapter solve of the 64 × 48 mono test frame with this `code` and
/// `reply.ini`: the error it answers, its reason and message, and the work
/// folder left empty.
async fn refused(code: &str, ini: Option<&str>, expected: SolveError, reason: &str, message: &str) {
    let scratch = Scratch::new("imaging-astap-error");
    let files: Vec<(&str, &str)> = ini
        .map(|text| vec![("reply.ini", text)])
        .unwrap_or_default();
    let solver = stub(&scratch.0, code, &files);
    let (frame, _) = test_frame(&scratch.0, "frame.fits", 64, 48, None);
    let error = solver
        .solve(frame, SolveHint::whole_sky())
        .await
        .expect_err("an error");
    assert_eq!(error, expected);
    assert_eq!(error.reason(), reason);
    assert_eq!(error.message(), message);
    assert_work_empty(&scratch.0.join("work"));
}

/// US1-S4: every exit code and every unreadable answer becomes its own reason,
/// detail and message, and the work folder is left empty after each.
#[tokio::test]
async fn solve_turns_every_failure_into_a_reason() {
    let failed = "\nPLTSOLVD=F\nCMDLINE=astap_cli -f frame.fits\n";
    refused(
        "1",
        Some(failed),
        SolveError::NotSolved {
            detail: Some("No solution found!".to_string()),
        },
        "not_solved",
        "the imaging solver found no solution",
    )
    .await;
    refused(
        "1",
        Some(&format!("{failed}ERROR=Not enough stars.\n")),
        SolveError::TooFewStars {
            detail: Some("Not enough stars.".to_string()),
        },
        "too_few_stars",
        "the imaging solver found too few stars in the imaging frame",
    )
    .await;
    refused(
        "1",
        Some(&format!("{failed}ERROR=No star database found.\n")),
        SolveError::NoDatabase {
            detail: Some("No star database found.".to_string()),
        },
        "no_database",
        "the imaging solver has no star database it can read",
    )
    .await;
    refused(
        "1",
        Some(&format!("{failed}ERROR=Something else.\n")),
        SolveError::NotSolved {
            detail: Some("Something else.".to_string()),
        },
        "not_solved",
        "the imaging solver found no solution",
    )
    .await;
    refused(
        "1",
        None,
        SolveError::NotSolved {
            detail: Some("No solution found!".to_string()),
        },
        "not_solved",
        "the imaging solver found no solution",
    )
    .await;
    refused(
        "2",
        Some(failed),
        SolveError::TooFewStars { detail: None },
        "too_few_stars",
        "the imaging solver found too few stars in the imaging frame",
    )
    .await;
    refused(
        "32",
        None,
        SolveError::NoDatabase { detail: None },
        "no_database",
        "the imaging solver has no star database it can read",
    )
    .await;
    refused(
        "33",
        None,
        SolveError::NoDatabase { detail: None },
        "no_database",
        "the imaging solver has no star database it can read",
    )
    .await;
    refused(
        "16",
        None,
        SolveError::Failed("the imaging solver could not read the imaging frame".to_string()),
        "failed",
        "the imaging solver could not read the imaging frame",
    )
    .await;
    refused(
        "34",
        None,
        SolveError::Failed("the imaging solver ended with exit code 34".to_string()),
        "failed",
        "the imaging solver ended with exit code 34",
    )
    .await;
    refused(
        "0",
        None,
        SolveError::Failed(
            "the imaging solver's answer could not be read: no .ini file was written".to_string(),
        ),
        "failed",
        "the imaging solver's answer could not be read: no .ini file was written",
    )
    .await;
    refused(
        "0",
        Some("PLTSOLVD=F\n"),
        SolveError::Failed(
            "the imaging solver's answer could not be read: PLTSOLVD is not T".to_string(),
        ),
        "failed",
        "the imaging solver's answer could not be read: PLTSOLVD is not T",
    )
    .await;
    let less_cd2_2 = WORKED_A
        .lines()
        .filter(|line| !line.starts_with("CD2_2"))
        .collect::<Vec<_>>()
        .join("\n");
    refused(
        "0",
        Some(&less_cd2_2),
        SolveError::Failed(
            "the imaging solver's answer could not be read: CD2_2 is missing".to_string(),
        ),
        "failed",
        "the imaging solver's answer could not be read: CD2_2 is missing",
    )
    .await;
    let bad_crval1 = WORKED_A
        .lines()
        .map(|line| {
            if line.starts_with("CRVAL1") {
                "CRVAL1= abc"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    refused(
        "0",
        Some(&bad_crval1),
        SolveError::Failed(
            "the imaging solver's answer could not be read: CRVAL1 is not a number".to_string(),
        ),
        "failed",
        "the imaging solver's answer could not be read: CRVAL1 is not a number",
    )
    .await;
    // a program that is not there
    let scratch = Scratch::new("imaging-astap-error");
    let solver = AstapSolver::new(AstapConfig {
        program: scratch.0.join("nowhere"),
        database_name: "d50".to_string(),
        database_dir: Some(PathBuf::from("/opt/astap")),
        work_dir: scratch.0.join("work"),
    });
    let (frame, _) = test_frame(&scratch.0, "frame.fits", 64, 48, None);
    let error = solver
        .solve(frame, SolveHint::whole_sky())
        .await
        .expect_err("an error");
    assert!(matches!(error, SolveError::Failed(_)));
    assert_eq!(error.reason(), "failed");
    assert!(
        error
            .message()
            .starts_with("the imaging solver could not be started: "),
        "the message names the failure: {}",
        error.message()
    );
    assert_work_empty(&scratch.0.join("work"));
}

/// US1-S5: a program that outlives its limit is killed and reaped, the call
/// answers `TimedOut`, the process is gone and the work folder empty; a search
/// of the whole sky has its own, longer limit.
#[tokio::test]
async fn solve_kills_a_program_that_outruns_its_limit() {
    assert!(
        Path::new("/proc/self").exists(),
        "/proc is mounted, so the pid check below can fail"
    );
    let scratch = Scratch::new("imaging-astap-limits");
    let program = scratch.0.join("sleeper");
    std::fs::write(&program, SLEEPER).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let solver = AstapSolver::new(AstapConfig {
        program,
        database_name: "d50".to_string(),
        database_dir: Some(PathBuf::from("/opt/astap")),
        work_dir: scratch.0.join("work"),
    })
    .with_limits(
        Duration::from_millis(500),
        Duration::from_millis(1500),
        Duration::from_millis(500),
    );
    let (frame, _) = test_frame(&scratch.0, "frame.fits", 64, 48, None);
    let started = Instant::now();
    let error = solver
        .solve(frame, hint(10.0, 20.0, 3.0, None))
        .await
        .expect_err("a timeout");
    assert!(
        started.elapsed().as_secs_f64() < 10.0,
        "the call returns within 10 s"
    );
    assert_eq!(error, SolveError::TimedOut { seconds: 1 });
    assert_eq!(error.reason(), "timed_out");
    assert_eq!(
        error.message(),
        "the imaging solver gave no answer within 1 s"
    );
    let pid = std::fs::read_to_string(scratch.0.join("pid")).unwrap();
    assert!(
        !Path::new(&format!("/proc/{}", pid.trim())).exists(),
        "the program is gone"
    );
    assert_work_empty(&scratch.0.join("work"));
    // a search of the whole sky has its own limit
    let (frame, _) = test_frame(&scratch.0, "frame.fits", 64, 48, None);
    let started = Instant::now();
    let error = solver
        .solve(frame, SolveHint::whole_sky())
        .await
        .expect_err("a timeout");
    assert_eq!(error, SolveError::TimedOut { seconds: 2 });
    assert_eq!(
        error.message(),
        "the imaging solver gave no answer within 2 s"
    );
    assert!(
        started.elapsed().as_secs_f64() >= 1.4,
        "the call took at least 1.4 s of real time"
    );
    assert_work_empty(&scratch.0.join("work"));
}

/// One stub-adapter measure of the 64 × 48 test frame that answers an error, and
/// the work folder left empty.
async fn measure_refused(
    code: &str,
    stdout: &str,
    bayer: bool,
    region: Option<Region>,
    expected: SolveError,
    reason: &str,
    message: &str,
) {
    let scratch = Scratch::new("imaging-astap-measure");
    let solver = stub(&scratch.0, code, &[("reply.stdout", stdout)]);
    let (frame, _) = test_frame(&scratch.0, "frame.fits", 64, 48, bayer.then_some("RGGB"));
    let error = solver.measure(frame, region).await.expect_err("an error");
    assert_eq!(error, expected);
    assert_eq!(error.reason(), reason);
    assert_eq!(error.message(), message);
    assert_work_empty(&scratch.0.join("work"));
}

/// US1-S6: `measure` runs `-analyse 30` on the prepared copy of a frame or of a
/// region moved inward to whole cells, reads the two printed lines whatever the
/// exit code, scales the diameter by the copy's factor, and refuses no stars,
/// unreadable output and an impossible region.
#[tokio::test]
async fn measure_prepares_regions_and_reads_the_two_lines() {
    let lines = "HFD_MEDIAN=3.3\nSTARS=666\n";
    // the whole mono frame
    {
        let scratch = Scratch::new("imaging-astap-measure");
        let solver = stub(&scratch.0, "0", &[("reply.stdout", lines)]);
        let (frame, _) = test_frame(&scratch.0, "frame.fits", 64, 48, None);
        let measure = solver.measure(frame, None).await.unwrap();
        assert_eq!(
            measure,
            StarMeasure {
                half_flux_px: 3.3,
                stars: 666
            }
        );
        let args: Vec<String> = std::fs::read_to_string(scratch.0.join("args"))
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        let work = scratch.0.join("work").display().to_string();
        assert_eq!(args.len(), 4, "exactly four arguments");
        assert_eq!(args[0], "-f");
        assert!(
            args[1].starts_with(&work) && args[1].ends_with("/frame.fits"),
            "the input is the copy in the work folder"
        );
        assert_eq!(args[2], "-analyse");
        assert_eq!(args[3], "30");
    }
    // a region of an RGGB frame, moved inward to whole cells
    {
        let scratch = Scratch::new("imaging-astap-measure");
        let solver = stub(&scratch.0, "0", &[("reply.stdout", lines)]);
        let (frame, _) = test_frame(&scratch.0, "frame.fits", 64, 48, Some("RGGB"));
        let measure = solver
            .measure(
                frame,
                Some(Region {
                    x: 10,
                    y: 10,
                    width: 21,
                    height: 21,
                }),
            )
            .await
            .unwrap();
        assert!((measure.half_flux_px - 6.6).abs() < 1e-12);
        assert_eq!(measure.stars, 666);
        let (width, height, samples) =
            read_fits(&std::fs::read(scratch.0.join("input.fits")).unwrap());
        assert_eq!((width, height), (10, 10));
        assert_eq!(sample(&samples, width, 0, 0), 1126);
        assert_eq!(sample(&samples, width, 9, 9), 1342);
    }
    // the same with a region that is moved inward on every edge
    {
        let scratch = Scratch::new("imaging-astap-measure");
        let solver = stub(&scratch.0, "0", &[("reply.stdout", lines)]);
        let (frame, _) = test_frame(&scratch.0, "frame.fits", 64, 48, Some("RGGB"));
        let measure = solver
            .measure(
                frame,
                Some(Region {
                    x: 11,
                    y: 11,
                    width: 20,
                    height: 20,
                }),
            )
            .await
            .unwrap();
        assert!((measure.half_flux_px - 6.6).abs() < 1e-12);
        let (width, height, samples) =
            read_fits(&std::fs::read(scratch.0.join("input.fits")).unwrap());
        assert_eq!((width, height), (9, 9));
        assert_eq!(sample(&samples, width, 0, 0), 1150);
    }
    // an RGGB frame of the test's own whose cell mean is rounded half up
    {
        let scratch = Scratch::new("imaging-astap-measure");
        let solver = stub(&scratch.0, "0", &[("reply.stdout", lines)]);
        let (frame, _) = write_frame(
            &scratch.0,
            "frame.fits",
            2,
            2,
            Some("RGGB"),
            &[1000, 1001, 1001, 1000],
        );
        solver.measure(frame, None).await.unwrap();
        let (width, height, samples) =
            read_fits(&std::fs::read(scratch.0.join("input.fits")).unwrap());
        assert_eq!((width, height), (1, 1));
        assert_eq!(samples, [1001]);
    }
    // a mono frame more than 2500 rows high, halved once
    {
        let scratch = Scratch::new("imaging-astap-measure");
        let solver = stub(&scratch.0, "0", &[("reply.stdout", lines)]);
        let (frame, _) = test_frame(&scratch.0, "frame.fits", 4, 2502, None);
        let measure = solver.measure(frame, None).await.unwrap();
        assert!((measure.half_flux_px - 6.6).abs() < 1e-12);
        let (width, height, samples) =
            read_fits(&std::fs::read(scratch.0.join("input.fits")).unwrap());
        assert_eq!((width, height), (2, 1251));
        assert_eq!(sample(&samples, width, 0, 0), 1006);
    }
    // the two lines are read whatever the exit code
    {
        let scratch = Scratch::new("imaging-astap-measure");
        let solver = stub(&scratch.0, "1", &[("reply.stdout", lines)]);
        let (frame, _) = test_frame(&scratch.0, "frame.fits", 64, 48, None);
        let measure = solver.measure(frame, None).await.unwrap();
        assert_eq!(
            measure,
            StarMeasure {
                half_flux_px: 3.3,
                stars: 666
            }
        );
    }
    measure_refused(
        "0",
        "HFD_MEDIAN=0.0\nSTARS=0\n",
        false,
        None,
        SolveError::TooFewStars { detail: None },
        "too_few_stars",
        "the imaging solver found too few stars in the imaging frame",
    )
    .await;
    measure_refused(
        "0",
        "",
        false,
        None,
        SolveError::Failed("the imaging solver's measure could not be read".to_string()),
        "failed",
        "the imaging solver's measure could not be read",
    )
    .await;
    measure_refused(
        "0",
        lines,
        false,
        Some(Region {
            x: 60,
            y: 40,
            width: 10,
            height: 10,
        }),
        SolveError::Failed("the region is outside the imaging frame".to_string()),
        "failed",
        "the region is outside the imaging frame",
    )
    .await;
    measure_refused(
        "0",
        lines,
        true,
        Some(Region {
            x: 11,
            y: 11,
            width: 2,
            height: 2,
        }),
        SolveError::Failed("the region is too small".to_string()),
        "failed",
        "the region is too small",
    )
    .await;
}

/// US1-S7: the saved captures of ASTAP's own output replay through the stub
/// program: the solved case puts three catalogue stars within 1.5 pixels of
/// where they were picked out, the three failures answer their reasons with the
/// saved exit codes and `.ini` files, and the star measure reads the listed
/// numbers.
#[tokio::test]
async fn saved_captures_replay_as_astap_answered() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/finder/astap");
    let captures: Value =
        serde_json::from_slice(&std::fs::read(root.join("captures.json")).unwrap()).unwrap();
    let width = captures["frame"]["width"].as_u64().unwrap() as u32;
    let height = captures["frame"]["height"].as_u64().unwrap() as u32;
    // solved: the three catalogue stars, which do not lie in one line
    {
        let case = &captures["cases"]["solved"];
        let scratch = Scratch::new("imaging-astap-captures");
        let ini = std::fs::read_to_string(root.join(case["ini"].as_str().unwrap())).unwrap();
        let wcs = std::fs::read_to_string(root.join(case["wcs"].as_str().unwrap())).unwrap();
        let solver = stub(&scratch.0, "0", &[("reply.ini", &ini), ("reply.wcs", &wcs)]);
        let (frame, _) = test_frame(&scratch.0, "frame.fits", width, height, None);
        let solve = solver.solve(frame, SolveHint::whole_sky()).await.unwrap();
        for star in case["stars"].as_array().unwrap() {
            let pixel = solve
                .pixel_of(
                    star["ra_deg"].as_f64().unwrap(),
                    star["dec_deg"].as_f64().unwrap(),
                )
                .expect("a place the solve reaches");
            let (x, y) = (star["x"].as_f64().unwrap(), star["y"].as_f64().unwrap());
            assert!(
                (pixel.0 - x).abs() <= 1.5 && (pixel.1 - y).abs() <= 1.5,
                "the star at ({x}, {y}) is at {pixel:?}"
            );
        }
    }
    // the three failures, from the saved exit code and `.ini`
    let failures = [
        (
            "not_solved",
            SolveError::NotSolved {
                detail: Some("No solution found!".to_string()),
            },
            "not_solved",
            "the imaging solver found no solution",
        ),
        (
            "too_few_stars",
            SolveError::TooFewStars {
                detail: Some("Not enough stars.".to_string()),
            },
            "too_few_stars",
            "the imaging solver found too few stars in the imaging frame",
        ),
        (
            "no_database",
            SolveError::NoDatabase {
                detail: Some("No star database found.".to_string()),
            },
            "no_database",
            "the imaging solver has no star database it can read",
        ),
    ];
    for (name, expected, reason, message) in failures {
        let case = &captures["cases"][name];
        let scratch = Scratch::new("imaging-astap-captures");
        let ini = std::fs::read_to_string(root.join(case["ini"].as_str().unwrap())).unwrap();
        let code = case["exit"].to_string();
        let solver = stub(&scratch.0, &code, &[("reply.ini", &ini)]);
        let (frame, _) = test_frame(&scratch.0, "frame.fits", width, height, None);
        let error = solver
            .solve(frame, SolveHint::whole_sky())
            .await
            .expect_err("an error");
        assert_eq!(error, expected);
        assert_eq!(error.reason(), reason);
        assert_eq!(error.message(), message);
        assert_work_empty(&scratch.0.join("work"));
    }
    // the star measure
    {
        let case = &captures["cases"]["analyse"];
        let scratch = Scratch::new("imaging-astap-captures");
        let stdout = std::fs::read_to_string(root.join(case["stdout"].as_str().unwrap())).unwrap();
        let code = case["exit"].to_string();
        let solver = stub(&scratch.0, &code, &[("reply.stdout", &stdout)]);
        let (frame, _) = test_frame(&scratch.0, "frame.fits", width, height, None);
        let measure = solver.measure(frame, None).await.unwrap();
        assert!(
            (measure.half_flux_px - case["half_flux_px"].as_f64().unwrap()).abs() < 1e-9,
            "the listed half-flux diameter"
        );
        assert_eq!(measure.stars, case["stars"].as_u64().unwrap() as u32);
    }
}
