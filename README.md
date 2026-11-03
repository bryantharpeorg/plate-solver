# plate-solver

[![ci](https://github.com/bryantharpeorg/plate-solver/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/bryantharpeorg/plate-solver/actions/workflows/ci.yml)

A fast star-field plate solver written in Rust. Give it a frame from a camera on your
telescope's finder scope and it works out where the telescope is pointing (right ascension,
declination and roll) in a few milliseconds, with no prior pointing information.

It re-implements [cedar-detect](https://github.com/smroid/cedar-detect) and
[cedar-solve](https://github.com/smroid/cedar-solve), the star detector and solver behind the
[Cedar](https://github.com/smroid/cedar-server) electronic finder, which build in turn on ESA's
[tetra3](https://github.com/esa/tetra3). See [Prior work](#prior-work).

> **Status: early, and looking for feedback.** On cedar's reference images it gives the same
> answers as cedar and runs faster ([comparison](#how-it-compares-with-cedar)). It has not yet
> been proven on frames from a real finder-scope setup, and it does not talk to a mount yet. If
> you use Cedar or tetra3, I'd especially like to hear how this compares on your hardware
> ([feedback](#feedback-wanted)).

## What it's for

I'm building a budget electronic finder: a small camera on the finder scope of a Sky-Watcher
400P (16-inch) GoTo Dobsonian. Each finder frame is plate-solved to the telescope's actual
pointing, which the mount's GoTo can use to align and correct its aim. That is the job Cedar
does. This project explores doing it with one Rust binary, fast enough to keep up with a live
camera stream.

This repository is the solving half. Camera capture and mount control are not part of it yet.

## Prior work

This project re-implements published work. Its behaviour was defined by the projects below, and
their authors deserve the credit for the algorithms.

- **[TETRA](https://github.com/brownj4/Tetra)**: J. Brown, K. Stubis and K. Cahoy,
  ["TETRA: Star Identification with Hash Tables"](https://digitalcommons.usu.edu/smallsat/2017/all2017/124/),
  Proceedings of the AIAA/USU Conference on Small Satellites, 2017. The original hash-table
  approach to lost-in-space star identification.
- **[tetra3](https://github.com/esa/tetra3)** (European Space Agency; Gustav Pettersson): the
  lost-in-space solver this lineage is built on. Four-star patterns, edge-ratio hashing, the
  pattern database and its `.npz` format, and match verification. Apache-2.0.
- **[cedar-solve](https://github.com/smroid/cedar-solve)** (Steven Rosenthal): a fork of tetra3
  with lattice-field database generation, breadth-first pattern search, solve timeouts and
  status codes. This project's solver and database generator follow it. Apache-2.0.
- **[cedar-detect](https://github.com/smroid/cedar-detect)** (Steven Rosenthal): fast, robust
  star detection with noise estimation, binning, hot-pixel rejection and sub-pixel centroids.
  This project's `star-detection` crate reproduces its output exactly. FSL-1.1-MIT.
- **[Cedar](https://github.com/smroid/cedar-server)** (Steven Rosenthal): the plate-solving
  electronic finder that brings cedar-detect and cedar-solve together, and the closest existing
  system to what this project is for.

Copies of tetra3, cedar-solve and cedar-detect are vendored in
[`reference-solutions/`](reference-solutions/) under their own licenses, and CI checks this
project's answers against them on every pull request.

This project is not affiliated with or endorsed by ESA or Steven Rosenthal. Cedar and Cedar Solve
are trademarks of Steven Rosenthal, named here only to describe where this work comes from.

## How it compares with Cedar

The comparison is against cedar's open-source pipeline, run the way cedar-solve's example script
runs it: Python decodes each frame, calls `cedar-detect-server` over gRPC with shared memory, then
solves with cedar-solve. Both sides used the same 21 sample images, the same settings (sigma 8,
2×2 binning, the brightest 30 stars, no field-of-view hint) and the same 10–30° pattern database.

**Answers.** Both solve 20 of the 21 images. Given the same decoded pixels, they find the same
stars and return the same solution on all 20: position within 0.0005″, with identical matched
catalog stars.

**Speed, as a stream of raw camera frames** (one frame at a time, one core):

| Per frame | plate-solver | cedar-detect + cedar-solve |
|---|---|---|
| Sustained rate | ≈500 fps | ≈120 fps |
| Median | 2.4 ms | 5.1 ms |
| 90th percentile | 3.1 ms | 13.9 ms |
| Slowest frame | 4.3 ms | 53.8 ms |

Where the difference comes from:

- **Finding stars:** the same algorithm at the same speed (2.4 ms vs 2.5 ms in-process). cedar's
  pipeline adds about 1.3 ms a frame for the call from Python to the detector over gRPC.
- **Solving:** about 57× faster (median 0.03 ms vs 1.3 ms). cedar's slowest frames come from here.
- **Start-up:** 0.4 s vs 2.3 s, mostly Python and NumPy imports. It is paid once per process.
- **JPEG frames:** about 54 vs 37 fps. Decoding takes around 13 ms a frame on both sides, so the
  decoder sets the rate; raw frames avoid it.

Limits of these numbers:

- **One machine:** a Snapdragon laptop under WSL2 (arm64). Treat the ratios as more portable than
  the milliseconds.
- **Not finder frames:** these are cedar's 10–30° sample images, 0.3–2.1 megapixels.
- **No hints:** every frame is solved from scratch. A real stream would pass the previous frame's
  field of view to the next solve, which speeds up both solvers and would likely narrow the gap.
- **Cedar's components, not the Cedar product:** this measures cedar-detect and cedar-solve as
  open-source components, not the Cedar product running on its own hardware.

To reproduce, run [`bench/cedar-compare/run.sh`](bench/cedar-compare/); its README covers the
prerequisites.

## Feedback wanted

Especially:

- **Finder-scope frames:** raw frames or JPEGs from your finder camera, with the camera, the lens
  or finder, and its field of view, whether they solve or not.
- **Comparisons with Cedar or tetra3** on your own hardware, particularly a Raspberry Pi.
- **Failed or wrong solves:** the image and the command you ran.
- **What you would need** to use this with your own mount.

Please open an issue.

## Usage

Everything runs through `cargo` from the repository root. Nothing is installed, and there is no
gRPC server binary yet: `grpc-service` is a library.

### See it work: the finder console

```sh
cargo run --release -p finder
```

Open <http://127.0.0.1:8080>, pick one of cedar's sample frames or open your own, and the
console charts it: where the frame points, with the RA/Dec grid, the matched catalog stars and
the Messier objects drawn over the frame, then what is near the field and what to look at next.
It has a red-and-black night mode for the telescope, and a phone on the same network can use it
(`--listen 0.0.0.0:8080`). See [`crates/finder`](crates/finder/README.md).

### Solve an image

```sh
cargo run --release -p plate-solver --example solve_image -- \
    --db reference-solutions/cedar-solve/tetra3/data/default_database.npz \
    reference-solutions/cedar-solve/examples/data/medium_fov/orion_belt.jpg
```

```text
reference-solutions/cedar-solve/examples/data/medium_fov/orion_belt.jpg: MATCH_FOUND  RA 84.0081  Dec -0.7690  Roll 2.54  FOV 10.15 deg  k -0.003  matches 24  RMSE 34"  (26 stars, detect 1 ms, solve 0 ms)
  sky map: https://aladin.cds.unistra.fr/AladinLite/?target=84.0081%20-0.7690&fov=12
```

RA, Dec, Roll and FOV are in degrees. `k` is the fitted radial distortion (negative is barrel).
RMSE is the residual over the matched stars, in arcseconds. Pass several images to solve them in
one run; the database loads once. PNG, JPEG, BMP and PNM files are read.

Frames are read the way cedar reads them, as in the finder console: color turns gray with PIL's
weights (`star_detection::gray`), and stars are detected with cedar's settings
(`DetectParams::cedar()`: 2×2 binning, rows levelled, hot pixels rejected). On cedar's 21 sample
frames that gives cedar's answer to within 4.2″ on all 20 that cedar solves.

The database must cover your camera's field of view. The cedar database used above covers
10–30°. A finder camera usually sees a narrower field, so build a database for it (below).

| Flag | Default | Meaning |
|------|---------|---------|
| `--db PATH` | required | Pattern database (`.npz`): one built below, or a cedar/tetra3 database. |
| `--fov DEG` | none | Horizontal FOV estimate. Leave it off for the first solve: a wrong hint with a tight `--fov-max-error` prevents solving. |
| `--fov-max-error DEG` | 10 | How far the true FOV may be from `--fov`. |
| `--distortion K` | 0 | Radial distortion `k` from an earlier solve with the same lens. Speeds up and steadies wide-field solves. |
| `--bin N` | 2 | Bin the image by 1, 2, 4 or 8 before detection. |
| `--sigma S` | 8 | Detection significance threshold. |
| `--stars N` | 30 | Number of brightest detected stars passed to the solver. |
| `--dark PATH` | none | Dark frame (lens capped, same exposure and gain) subtracted before detection. Removes hot pixels. |
| `--timeout MS` | 5000 | Solve timeout in milliseconds. |
| `--json` | off | Print one JSON object per image instead of text. |

### Build a pattern database

```sh
cargo run --release -p database-generation -- hip_main.dat hip_30_60.npz --min-fov 30 --max-fov 60
```

Set `--min-fov` and `--max-fov` (degrees) to bracket your camera's horizontal FOV, and pick a
catalog deep enough for it. Following tetra3's guidance, the Hipparcos main catalog,
`hip_main.dat` (53 MB), suits fields down to about 10°, and the Tycho catalog, `tyc_main.dat`,
down to about 3°. Both are in the CDS catalog
[I/239](https://cdsarc.cds.unistra.fr/ftp/I/239/). A BSC5 file also works for wide fields. The
catalog type is detected from the file name (`hip`, `tyc` or `bsc5`), and `--help` lists the
other options: magnitude limit, proper-motion epoch and pattern density.

To try it without downloading anything, build from the BSC5 catalog vendored for cedar's tests
(about a minute, 32 MB) and solve with the result:

```sh
cargo run --release -p database-generation -- reference-solutions/cedar-solve/tests/data/bsc5 bsc5_10_30.npz --min-fov 10 --max-fov 30
```

## Repository layout

| Path | What it is |
|------|------------|
| `crates/star-detection` | Star detection: noise estimation, binning, sub-pixel centroiding and brightness. |
| `crates/math-core` | Shared numerics: unit vectors, the pinhole camera model, attitude (Wahba/SVD), FOV refinement. |
| `crates/pattern-database` | The pattern database: `.npz` loader, KD-tree and pattern lookup. |
| `crates/database-generation` | Builds pattern databases from star catalogs (Hipparcos, Tycho, BSC5). |
| `crates/plate-solver` | The solver (`solve_from_centroids`, `solve_from_image`) and the `solve_image` example. |
| `crates/grpc-service` | The `PlateSolver` gRPC service, as a library. |
| `crates/finder` | The finder console: a local web page that plate-solves a frame and charts it. The page (React, in `web/`) is built into the binary. |
| `bench/cedar-compare/` | The benchmark against cedar-detect + cedar-solve. |
| `openspec/` | The specifications: capability specs, PRD and project conventions. Start at `openspec/project.md` and `openspec/PRD.md`. |
| `proto/` | The gRPC interface contracts (`plate_solver.proto`, `cedar_detect.proto`). |
| `reference-solutions/` | **The oracle**: vendored tetra3, cedar-solve and cedar-detect, with their sample images and databases. |
| `scripts/ci/`, `.github/workflows/ci.yml` | CI, including the end-to-end parity gate. |
| `docs/` | Audits, e.g. `docs/audit-e2e-parity.md`. |

## Correctness

"Correct" means agreeing with cedar. CI's `e2e-eval` job solves the 21 sample images in
`reference-solutions/cedar-solve/examples/data/medium_fov/` from star positions (E1), from pixels
(E2) and over gRPC (E3), and compares every answer with cedar's. `e2e-canary` proves that gate can
still fail. [`ENVIRONMENT.md`](ENVIRONMENT.md) lists what CI enforces on `main` and which files
define "correct".

## Development

Building `grpc-service` needs `protoc` (Debian/Ubuntu: `protobuf-compiler`; if it can't find
`google/protobuf/duration.proto`, also install `libprotobuf-dev`). The finder console's page is
built with Node in `crates/finder/web`, and its build is committed, so cargo never needs Node.
CI runs, among other checks:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```

`scripts/coverage.sh` writes browsable coverage reports for the workspace and the page, with
per-crate tables, to `target/coverage/`. [`docs/coverage.md`](docs/coverage.md) explains
how to run and read them.

`scripts/quality.sh` writes a SonarQube-style code-quality report to `target/quality/`:
complexity, duplication, lint findings by rule, panic paths, `unsafe` and advisories, with an
A–E grade per crate and per file. [`docs/quality.md`](docs/quality.md) explains every figure.

## License

The reference implementations vendored in [`reference-solutions/`](reference-solutions/) keep
their own licenses: tetra3 and cedar-solve are Apache-2.0, and cedar-detect is FSL-1.1-MIT. This
project's own license will be stated here before its first release.
