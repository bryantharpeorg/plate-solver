# plate-solver

[![ci](https://github.com/bryantharpeorg/plate-solver/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/bryantharpeorg/plate-solver/actions/workflows/ci.yml)

An electronic finder, GoTo and imaging controller for an alt-az GoTo telescope, built around a
fast star-field plate solver written in Rust.

A small camera on the finder scope takes a frame every second or so. Each frame is
plate-solved to the telescope's actual pointing, with no star alignment and no prior hint, in
a few milliseconds. From that the system:

- tells you which way to push the telescope;
- drives the mount to a target and corrects until it is there;
- tracks the target;
- runs an imaging camera at the focuser: focus, darks and flats, a live stack that follows the
  field's rotation, and one-button imaging runs.

You use it from a web page on a phone or laptop, or an AI assistant uses it over MCP.

> **Status: working at one telescope, and looking for feedback.**
>
> - **Field-proven:** the live finder, push-to and the mount link on a Sky-Watcher 400P (16-inch)
>   GoTo Dobsonian, driven from a Raspberry Pi 3B+ with an Arducam OV5647 finder camera. Track
>   has been used in the field; GoTo runs on the simulator, with no field GoTo on record yet.
> - **Bench-proven:** a ZWO ASI071MC Pro imaging camera through INDI, including full frames
>   received on that Pi.
> - **Not yet field-proven:** full imaging runs.
> - **Planned:** a Raspberry Pi 5 rig, which moves everything onto the telescope.
>
> [Known-good configurations](docs/hardware.md) has the details and the limits. If you use
> Cedar, tetra3 or a SynScan mount, I'd especially like to hear how this works on your
> hardware ([feedback](#feedback-wanted)).

## What it does

- **Plate solving.** It gives cedar's answers on cedar's reference frames, and solves about 4×
  faster as a frame stream ([the solver](docs/solver.md)).
- **Live finder.** The chart updates with every frame. It shows where the telescope points,
  the matched stars as proof, what's in the field and what to look at next.
- **Push-to.** "Turn right 2.7°, tilt down 1.4°" to any of 13,000 objects from OpenNGC.
- **GoTo with no alignment.** Every solve taken while the mount is still is a sync, and a
  pointing model is fitted from them.
  GoTo corrects from fresh solves until it is within 60″.
- **Tracking.** Rates come from the pointing model, and live solves correct them. A push by
  hand is followed, not fought.
- **Imaging.** Any INDI camera. Each frame is saved as FITS, and ASTAP solves the subs and any
  frame you ask about. The page
  offers a focus aid, a calibration library and a live stack that is derotated sub by sub.
  Tracking can be steered from the subs. One button, or one MCP tool, runs the whole sequence.
- **Safety built in.** One Stop for everything. A watchdog stops the mount when the console goes
  quiet. Guards cover the Sun, altitude, cable wind and axis range.
  [Safety and security](docs/safety-and-security.md) also lists what isn't covered yet.
- **Night mode.** Red on black and dim, with no white flashes.
- **MCP.** 32 tools let an AI assistant observe and steer through the same guards as the page.

## How it fits together

Two programs run at night:

- **`scope-node`** runs on a Raspberry Pi riding on the telescope. It captures finder frames and
  finds their stars, drives the mount's two axes over USB serial, and relays the imaging
  camera's frames through INDI.
- **`finder`** (the console) runs on a laptop today. It solves, keeps the pointing model, plans
  GoTo, runs Track, and saves, solves and stacks imaging frames. It serves the page and MCP.

The planned Pi 5 rig runs both programs on the telescope and reaches them over an SSH tunnel.
[Architecture](docs/architecture.md) has the diagrams, the data flows and the plan.

## Get started

You need Rust 1.96 or later and `protoc` (Debian/Ubuntu: `sudo apt install protobuf-compiler`).
Run everything from the repository root.

**Solve sample frames** in the console, then open <http://127.0.0.1:8080>:

```sh
cargo run --release -p finder
```

**Run a whole simulated telescope**, with a finder camera, mount and imaging camera:

```sh
cargo run --release -p finder -- --simulate --sim-start 2026-01-16T02:00:00Z
```

**Solve one image from the command line:**

```sh
cargo run --release -p plate-solver --example solve_image -- \
    --db reference-solutions/cedar-solve/tetra3/data/default_database.npz \
    reference-solutions/cedar-solve/examples/data/medium_fov/orion_belt.jpg
```

The [quick start](docs/guide/quick-start.md) walks through all three, including a GoTo and an
imaging run on the simulator.

## Documentation

| | |
|---|---|
| [Quick start](docs/guide/quick-start.md) | Everything on one computer, no hardware. |
| [Console manual](docs/guide/console.md) | Running the console for a real night: options, pairing, phones, ASTAP, MCP. |
| [Scope node manual](docs/guide/scope-node.md) | Putting the scope node on a Raspberry Pi. |
| [First night at the telescope](docs/guide/first-night.md) | The order of checks, and troubleshooting. |
| [A tour of the console page](docs/console-tour.md) | Every view and control. |
| [Architecture](docs/architecture.md) | The pieces, the links and the Pi 5 plan. |
| [Known-good configurations](docs/hardware.md) | Hardware that has run, its limits, and what else should work. |
| [Safety and security](docs/safety-and-security.md) | What stops the mount, and what is exposed on the network. |
| [The plate solver](docs/solver.md) | The command-line solver, pattern databases, and the comparison with cedar. |

[`docs/README.md`](docs/README.md) indexes everything else: the crate references, decisions,
research and specifications.

## Can I trust it?

Here is what backs it up, and where the gaps are.

**The solver is checked against cedar on every pull request.**

- CI solves cedar's 21 reference images three ways: from star positions, from pixels, and over
  gRPC. It compares every answer with cedar's own: identical star IDs within 5″ from star
  positions, and within 10″ with at least 90% of cedar's stars from pixels and over gRPC.
- A canary job then re-injects past bugs and nudges the goldens, to prove that gate still fails
  when it should. See [how correctness is checked](docs/solver.md#how-correctness-is-checked).

**Every change passes required checks before it merges.** There are 14:

- a change-detection job, and a manual brake (the `hold:human` label);
- formatting, Clippy with warnings as errors, and rustdoc;
- the minimum Rust version;
- unused dependencies;
- `cargo-deny` for advisories, licenses, banned crates and sources;
- a secret scan of every committed file;
- the parity gate and its canary;
- the page's checks: types, unit tests, the committed build matching its source, and browser
  tests against the release binary, including night-mode pixel checks;
- the full Rust test suite, on every pull request and again in the merge queue.

[`ENVIRONMENT.md`](ENVIRONMENT.md) says exactly what is enforced.

**The tests:**

- about 1,400 Rust tests, about 590 page unit and component tests, and about 30 browser
  end-to-end tests;
- end-to-end tests that drive the console's API against the simulator, through GoTo, tracking,
  Stop, imaging and MCP;
- an emulator for the mount's motor-controller protocol;
- recorded captures: the real finder camera's output, ASTAP's output, and an INDI exchange with
  INDI's camera simulator.

**What is not tested automatically:**

- No real mount or camera runs in CI. Hardware results come from bench and field nights, logged
  in [known-good configurations](docs/hardware.md).
- The node does not yet stop the mount when it is shut down
  ([safety](docs/safety-and-security.md#what-does-not-stop-the-mount-yet)).
- The console has no login. It listens only on localhost by default.

## How this repository is built

Since late October 2026, most of the code has been written by AI coding agents, driven by an
automated, spec-driven build system (configured by `ergane.yaml`).

1. **Spec.** Each feature starts as a specification in `specs/NNN-name/`: a spec, a plan and a
   task list.
2. **Review.** The maintainer reviews it and approves it in a pull request titled
   `specs NNN: flip … to ready`.
3. **Build.** The system builds it one user story at a time. Each story lands as its own
   squash-merged pull request titled `NNN-name/usN`, through the same required checks and merge
   queue as everything else.
4. **Ownership.** The maintainer owns the specs, the oracle files listed in
   [`.github/CODEOWNERS`](.github/CODEOWNERS), and the gates.

Earlier work, from June to October 2026, used the specifications under `openspec/`, and agents
directed by hand. The original v1 is kept at tag `v1-original`.

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

The imaging side relies on two programs it calls rather than links:

- **[INDI](https://indilib.org)**, which drives the imaging camera (LGPL-2.1-or-later).
- **[ASTAP](https://www.hnsky.org/astap.htm)**, which solves imaging frames.

Copies of tetra3, cedar-solve and cedar-detect are vendored in
[`reference-solutions/`](reference-solutions/) under their own licenses, and CI checks this
project's answers against them on every pull request.

This project is not affiliated with or endorsed by ESA or Steven Rosenthal. Cedar and Cedar Solve
are trademarks of Steven Rosenthal, named here only to describe where this work comes from.

## Repository layout

| Path | What it is |
|------|------------|
| `crates/star-detection` | Star detection: noise estimation, binning, sub-pixel centroiding and brightness. |
| `crates/math-core` | Shared numerics: unit vectors, the pinhole camera model, attitude (Wahba/SVD), FOV refinement. |
| `crates/pattern-database` | The pattern database: `.npz` loader, KD-tree and pattern lookup. |
| `crates/database-generation` | Builds pattern databases from star catalogs (Hipparcos, Tycho, BSC5). |
| `crates/plate-solver` | The solver (`solve_from_centroids`, `solve_from_image`) and the `solve_image` example. |
| `crates/grpc-service` | The `PlateSolver` gRPC service, as a library. |
| `crates/scope-link` | The protocol between the console and the scope node, and its token check. |
| `crates/scope-node` | The program on the telescope's Pi: finder camera, mount link and watchdog, imaging relay through INDI. |
| `crates/simulator` | The simulated finder camera, mount and imaging camera behind `finder --simulate`. |
| `crates/finder` | The console: web server, embedded React page (`web/`), API, MCP, pointing model, GoTo, Track and imaging. |
| `specs/` | One specification per feature from 012 on, which the build system builds from. |
| `openspec/` | The first-generation specifications, PRD and project conventions. |
| `docs/` | Guides, architecture, decisions (`adr/`), research and CI reports. Start at [`docs/README.md`](docs/README.md). |
| `fixtures/` | Recorded captures from real hardware and programs, used as test oracles. |
| `reference-solutions/` | **The oracle**: vendored tetra3, cedar-solve and cedar-detect, with their sample images and databases. |
| `bench/cedar-compare/` | The benchmark against cedar-detect + cedar-solve. |
| `crates/scope-link/proto/`, `crates/grpc-service/proto/` | The gRPC contracts the build compiles. The root `proto/` holds older copies. |
| `scripts/`, `.github/workflows/` | CI, coverage and quality scripts. |

## Development

You need:

- **Rust 1.96 or later** and **`protoc`**. The console, the scope node and `grpc-service` all
  compile protocol definitions. If `protoc` can't find `google/protobuf/duration.proto`, also
  install `libprotobuf-dev`.
- **Node, only to change the page.** The page is built with Node in `crates/finder/web`, and its
  build is committed, so cargo never needs Node.

CI runs, among other checks:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo nextest run --workspace --all-targets --all-features --locked
cargo test --workspace --locked --doc
```

The tests run under [cargo-nextest](https://nexte.st), which runs them in parallel across
test binaries (`.config/nextest.toml`). `cargo test --workspace --locked` runs the same tests
one binary at a time and needs nothing installed. Test builds are optimized
(`[profile.test]` in the root `Cargo.toml`) because the simulator's frames are slow without it.

There are two reports:

- **Coverage.** `scripts/coverage.sh` writes browsable coverage reports for the workspace and
  the page, with per-crate tables, to `target/coverage/`. [`docs/coverage.md`](docs/coverage.md)
  explains how to run and read them.
- **Code quality.** `scripts/quality.sh` writes a SonarQube-style report to `target/quality/`:
  complexity, duplication, lint findings by rule, panic paths, `unsafe` and advisories, with an
  A–E grade per crate and per file. [`docs/quality.md`](docs/quality.md) explains every figure.

## Feedback wanted

Especially:

- **Finder-scope frames:** raw frames or JPEGs from your finder camera, with the camera, the lens
  or finder, and its field of view, whether they solve or not.
- **Comparisons with Cedar or tetra3** on your own hardware, particularly a Raspberry Pi.
- **Other mounts:** whether the scope node's `--probe-mount` reads your Sky-Watcher motor
  board.
- **Failed or wrong solves:** the image and the command you ran.
- **What you would need** to use this with your own telescope.

Please open an issue.

## License

The workspace manifest declares `MIT OR Apache-2.0` for every crate in this repository. The
license texts are not committed yet, and will be before the first release.

Third-party material keeps its own license:

- the vendored reference implementations in [`reference-solutions/`](reference-solutions/):
  tetra3 and cedar-solve are Apache-2.0, and cedar-detect is FSL-1.1-MIT;
- the catalog data and web components embedded in the console, listed in
  [`crates/finder/THIRD_PARTY_NOTICES.md`](crates/finder/THIRD_PARTY_NOTICES.md) and printed by
  `finder --licenses`.
