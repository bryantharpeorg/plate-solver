# Implementation Plan: Finder's Tests, Organized by System

**Branch**: `027-finder-tests-by-system` | **Date**: 2026-11-14 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

Finder's integration tests change place, not content.

- **US1, one binary a test:** the simulator rig leaves `goto_api.rs` and the tracker rig
  leaves `backlash_sim.rs` for `crates/finder/tests/support/`, so the seven files that
  included those two test files for their rigs stop compiling their tests as well. A new
  `layout` binary asserts that no file with tests is included by another.
- **US2, one binary a system:** the 49 test files become modules of four binaries,
  `motion`, `console`, `mcp` and `cli`, and the three older helper files join `support/`.
  `layout` gains a test that the folder holds nothing else.

No assertion, product source file, manifest or gate changes.

## Technical Context

**Language/Version**: Rust 2021, in `crates/finder/tests` only.

**Primary Dependencies**: nothing new. `Cargo.toml` and `Cargo.lock` are not edited:
cargo finds `tests/name/main.rs` as the test binary `name` by itself.

**Storage**: nothing.

**Testing**:
- US1: a new test binary, `crates/finder/tests/layout/main.rs`, with two tests that read
  the source tree. Both fail at 870b332 and pass after US1.
- US2: a third test in the same file, which fails after US1 and passes after US2; and
  one added assertion in each of the two tests that re-run their own binary.

**Target Platform**: the workspace's test builds.

**Constraints**:
- A moved file must stay a rename in the diff: move it with `git mv` and change only the
  lines this plan names. `cargo fmt` reorders `use` lines; let it, and change nothing
  else by hand.
- The gates build with `-D warnings`. An unused import or a helper item no test uses is
  an error unless this plan's attributes allow for it.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | No solver, detection or database code is touched, and no oracle file: `crates/finder/tests/api.rs` moves with its cedar parity test unchanged. |
| II. Never weaken a gate | No test is deleted, skipped or ignored and no assertion changes. The edits to existing tests are the ones spec.md § Owner approvals names: header lines, two `--exact` arguments, and one added assertion in each of two tests. The 41 runs that stop are second and later runs of tests that still run once. |
| III. Tests first, independent expectations | Each story starts with a `layout` test that fails. Its oracle is the rule as written in spec.md, checked against the files on disk, not against what the mover produced. |
| IV. Critical paths | No `unsafe`, no `Cargo.toml` or `Cargo.lock` change, no `build.rs`, nothing under `proto/` or `.cargo/`. |
| V. Standalone Rust, honest licensing | Unaffected. |

## Project Structure

After US2:

```text
crates/finder/tests/
  support/        helpers only; no file here holds a test
    mod.rs        declares the five modules below
    live.rs       database, clocks, request helper, live-loop node        (was live/support.rs)
    goto.rs       the router-level simulator rig                          (was the top of goto_api.rs)
    tracker.rs    the tracker-level rig and `measured`                    (was the top of backlash_sim.rs)
    stop.rs       the Stop round's rig and gates                          (was stop/support.rs)
    mcp.rs        MCP requests, tools and its simulator rig               (was mcp/support.rs)
  motion/         binary: the simulated mount, in paused time
    main.rs
    goto/         routes, refusals, sim, second_sync
    model/        sim, routes, truth
    tracking/     tracker, pi_loop, night, routes, min_alt, backlash, keyhole
    stop/         retry, motion_ends, watchdog
    simulate.rs   the simulate-mode routes
    mount_calls.rs
    night.rs      the whole-night scenario
  console/        binary: the router in process, no mount motion
    main.rs
    api.rs  star_detected.rs  aim.rs  freshness.rs  pushto.rs  pointing.rs
    targets.rs  rates.rs  notices.rs  link.rs
    live/         loop_, stream, whole_loop
    session/      edits, writes, node
  mcp/            binary: the MCP door and its tools
    main.rs  endpoint.rs  frames.rs  motion.rs  session_tools.rs  tracking.rs  axis_moves.rs
  cli/            binary: tests that start the built `finder`
    main.rs  flags.rs  aim.rs  session.rs  session_node.rs  simulate.rs  track.rs  backlash.rs
  layout/         binary: where tests live, checked
    main.rs
```

## Interfaces

### US1: the two helper files

`crates/finder/tests/support/goto.rs` is lines 1 to 327 of
`crates/finder/tests/goto_api.rs` (everything above the comment
`// Every negative motion assertion first proves the connected mount can move.`), and
`crates/finder/tests/support/tracker.rs` is lines 1 to 334 of
`crates/finder/tests/backlash_sim.rs` (everything above its first
`#[tokio::test(start_paused = true)]`), each with exactly these changes:

1. The helper it includes is now one folder up:

   ```rust
   #[path = "../live/support.rs"]
   pub mod support;
   ```

   in place of `#[path = "live/support.rs"]` and `mod support;`. It is `pub` so a test
   file that writes `support::request(...)` still finds it through the glob import below.
2. `use support::{ConsoleClock, DB, START};` becomes

   ```rust
   #[allow(unused_imports)]
   pub use support::{ConsoleClock, DB, START};
   ```

   The tests use `START` and the helpers do not, and a binary that uses none of the
   three would otherwise fail on an unused import.
3. Every top-level `fn`, `async fn`, `const`, `static` and `struct`, every named struct
   field, and every method in an `impl Type` block gains `pub` if it lacks it. Methods in
   an `impl Trait for Type` block take no `pub`.
4. `support/tracker.rs` starts with `#![allow(dead_code)]` as its first line
   (`goto_api.rs` already has it): `backlash_cli` uses only part of the rig.
5. No blank line is left at the end of either file (`cargo fmt --all --check` fails on
   one).

### US1: `goto_api.rs` and `backlash_sim.rs` keep their tests

`crates/finder/tests/goto_api.rs` becomes this header followed by its lines 328 to 469,
unchanged:

```rust
#![allow(dead_code)]
//! Oracles: rendered catalog stars, simulator mechanics and true eyepiece,
//! sampled every 0.1 s in paused time; angular acceptance is strictly 60 arcseconds.
#[path = "support/goto.rs"]
mod api;
use api::*;
use finder::{api::AppState, ephemeris, samples::Samples, session::Aim, sky};
use scope_node::mount::{Axis, Mount};
use serde_json::json;
use std::{sync::Arc, time::Duration};

```

`crates/finder/tests/backlash_sim.rs` becomes this header followed by its lines 335 to
505, unchanged:

```rust
//! Oracles: configured 0.3 degree mechanical play and the simulator's independent
//! eyepiece ray. Exact command ordering; 60/30 arcsecond sky bounds in paused time.
#[path = "support/tracker.rs"]
mod tracker;
use finder::{session::Session, targets};
use scope_node::mount::{Axis, Mount};
use tracker::*;

```

These import lists are the ones the probe compiled with warnings denied. If the compiler
names another unused or missing import, follow the compiler.

### US1: the seven includers and the two recorders

| File | Line | Becomes |
|---|---|---|
| `goto_refusal_api.rs`, `goto_sim.rs`, `model_api.rs`, `second_sync_sim.rs`, `track_api.rs`, `track_min_alt.rs` | `#[path = "goto_api.rs"]` | `#[path = "support/goto.rs"]` |
| `backlash_cli.rs` | `#[path = "backlash_sim.rs"]` | `#[path = "support/tracker.rs"]` |
| `crates/finder/web/src/test/fixtures/record-sessions.rs`, `crates/finder/web/src/test/fixtures/record-tracking.rs` | `#[path = "goto_api.rs"]` | `#[path = "support/goto.rs"]` |

The `mod api;`, `mod goto_api;` and `mod backlash_sim;` lines under them stay, so every
use of `api::Rig`, `goto_api::Rig` and `backlash_sim::measured` below is unchanged.

### The guard: `crates/finder/tests/layout/main.rs`

US1 writes the file with everything down to and including
`support_holds_helpers_and_no_tests`. US2 appends `code_lines` and
`tests_are_grouped_into_one_binary_per_system` and edits nothing above them.

```rust
//! Where finder's integration tests live, checked.
//!
//! The rules (specs/027-finder-tests-by-system): a test function is compiled into one
//! test binary, shared helpers live under `tests/support/` and hold no tests, and the
//! tests are grouped into one binary per system. These tests read the source tree; the
//! oracle is the rule, not what the tree happens to hold.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// A line that makes the next function a test.
fn declares_a_test(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("#[test]") || line.starts_with("#[tokio::test")
    })
}

/// The files a source file pulls in with a path attribute at the start of a line.
fn included(file: &Path) -> Vec<PathBuf> {
    let text = fs::read_to_string(file).unwrap();
    text.lines()
        .filter_map(|line| line.trim_start().strip_prefix("#[path = \""))
        .filter_map(|rest| rest.split('"').next())
        .map(|relative| file.parent().unwrap().join(relative))
        .collect()
}

#[test]
fn no_file_with_tests_is_included_by_another() {
    let mut files = Vec::new();
    rust_files(&tests_dir(), &mut files);
    assert!(
        files.len() > 40,
        "the scan found only {} files",
        files.len()
    );
    let mut includes = 0;
    for file in &files {
        for target in included(file) {
            includes += 1;
            let text = fs::read_to_string(&target).unwrap_or_else(|e| {
                panic!("{} includes {}: {e}", file.display(), target.display())
            });
            assert!(
                !declares_a_test(&text),
                "{} includes {}, which holds tests: they would run once in each binary",
                file.display(),
                target.display()
            );
        }
    }
    assert!(includes > 0, "the scan found no path attribute at all");
}

#[test]
fn support_holds_helpers_and_no_tests() {
    let support = tests_dir().join("support");
    for name in ["goto.rs", "tracker.rs"] {
        assert!(
            support.join(name).is_file(),
            "tests/support/{name} is missing"
        );
    }
    let mut files = Vec::new();
    rust_files(&support, &mut files);
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        assert!(!declares_a_test(&text), "{} holds a test", file.display());
    }
}

/// The lines of a source file that are neither blank nor comments.
fn code_lines(file: &Path) -> Vec<String> {
    fs::read_to_string(file)
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn tests_are_grouped_into_one_binary_per_system() {
    let mut entries: Vec<String> = fs::read_dir(tests_dir())
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(
                entry.path().is_dir(),
                "tests/{} is a file: a test belongs in a module of cli, console, mcp or motion",
                entry.file_name().to_string_lossy()
            );
            entry.file_name().to_string_lossy().into_owned()
        })
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        ["cli", "console", "layout", "mcp", "motion", "support"]
    );

    let mut helpers: Vec<String> = fs::read_dir(tests_dir().join("support"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    helpers.sort();
    assert_eq!(
        helpers,
        [
            "goto.rs",
            "live.rs",
            "mcp.rs",
            "mod.rs",
            "stop.rs",
            "tracker.rs"
        ]
    );

    for binary in ["cli", "console", "mcp", "motion"] {
        let main = tests_dir().join(binary).join("main.rs");
        let lines = code_lines(&main);
        assert_eq!(
            lines[..3],
            [
                "#[allow(dead_code, unused_imports)]",
                "#[path = \"../support/mod.rs\"]",
                "mod support;"
            ],
            "{binary}/main.rs starts with the shared support tree"
        );
        for line in &lines[3..] {
            let module = line
                .strip_prefix("mod ")
                .and_then(|rest| rest.strip_suffix(';'));
            assert!(
                module.is_some_and(|name| name.chars().all(|c| c.is_ascii_lowercase() || c == '_')),
                "{binary}/main.rs holds only module declarations, found: {line}"
            );
        }
        assert!(lines.len() > 3, "{binary}/main.rs declares no module");
    }
}
```

### US2: the helper folder

1. `git mv` `live/support.rs` to `support/live.rs`, `mcp/support.rs` to `support/mcp.rs`
   and `stop/support.rs` to `support/stop.rs`. Their contents do not change.
2. In `support/goto.rs` and `support/tracker.rs`, the two lines
   `#[path = "../live/support.rs"]` and `pub mod support;` become one:
   `pub use super::live as support;`.
3. `crates/finder/tests/support/mod.rs` is new:

   ```rust
   //! Helpers shared by finder's integration-test binaries. Nothing under this directory
   //! is a test: a test function is compiled into exactly one binary.
   pub mod goto;
   pub mod live;
   pub mod mcp;
   pub mod stop;
   pub mod tracker;
   ```

### US2: the four binaries

Each `main.rs` is a doc comment, the support include, and one `mod` line for each file
or folder beside it, in alphabetical order. `motion/main.rs` in full:

```rust
//! Tests that drive the simulated mount in paused time: GoTo, the pointing model,
//! tracking, Stop and the watchdog, the simulate-mode routes, and the whole-night scenario.

#[allow(dead_code, unused_imports)]
#[path = "../support/mod.rs"]
mod support;

mod goto;
mod model;
mod mount_calls;
mod night;
mod simulate;
mod stop;
mod tracking;
```

The other three open with these doc comments:

- `console/main.rs`: `//! Tests of the console's routes, session, live loop and scope link, in process.` and
  `//! No mount moves here. Two tests re-run this binary as a child process.`
- `mcp/main.rs`: `//! Tests of the MCP door and its tools, in process.`
- `cli/main.rs`: ``//! Tests that start the built `finder` binary.``

Each folder inside a binary (`motion/goto`, `motion/model`, `motion/stop`,
`motion/tracking`, `console/live`, `console/session`) has a `mod.rs` holding one `mod`
line for each file beside it, alphabetical, and nothing else.

### US2: the edits inside a moved test file

After `git mv` to the path § Move map gives, a test file changes in these ways only:

1. Each helper included by path is bound through the support tree under the same name:

   | The two lines | Become the one line |
   |---|---|
   | `#[path = "live/support.rs"]` and `mod support;` | `use crate::support::live as support;` |
   | `#[path = "mcp/support.rs"]` and `mod support;` | `use crate::support::mcp as support;` |
   | `#[path = "stop/support.rs"]` and `mod support;` | `use crate::support::stop as support;` |
   | `#[path = "support/goto.rs"]` and `mod api;` (or `mod goto_api;`) | `use crate::support::goto as api;` (or `as goto_api;`) |
   | `#[path = "support/tracker.rs"]` and `mod tracker;` (or `mod backlash_sim;`) | `use crate::support::tracker as tracker;` (or `as backlash_sim;`) |

   Every later `use support::*;`, `api::Rig` and `support::request(...)` in the file then
   resolves as before.
2. `cli/flags.rs` (was `cli.rs`, line 8): `include_str!("../THIRD_PARTY_NOTICES.md")`
   becomes `include_str!("../../THIRD_PARTY_NOTICES.md")`. No other test file uses a path
   relative to itself; paths built from `CARGO_MANIFEST_DIR` are unaffected.
3. `cargo fmt --all` may move the new `use` line among the file's other `use` lines.

An inner attribute at the top of a file (`#![allow(dead_code)]`) stays where it is; it is
valid at the top of a module.

### US2: the two tests that re-run their own binary

In `console/targets.rs` (`search_works_from_an_empty_working_directory`) and
`console/session/edits.rs` (`library_without_a_state_file_leaves_home_and_xdg_empty`),
the child's arguments name the test by its full path:

```rust
        .args([
            "--exact",
            &format!(
                "{}::search_works_from_an_empty_working_directory",
                module_path!().split_once("::").unwrap().1
            ),
            "--nocapture",
        ])
```

(`module_path!()` is `console::targets` there; the test's name to the harness is
`targets::search_works_from_an_empty_working_directory`.) And before the existing
`assert!(output.status.success(), ...)` (the variable is `out` in `edits.rs`):

```rust
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("test result: ok. 1 passed"),
        "the child ran no test: {}",
        String::from_utf8_lossy(&output.stdout)
    );
```

### US2: the two recorders

In both recorder files the two lines `#[path = "support/goto.rs"]` and `mod api;` become

```rust
#[allow(dead_code, unused_imports)]
#[path = "support/mod.rs"]
mod support;
use support::goto as api;
```

## Move map

Authoritative for FR-005. "Filter" is what follows `cargo test -p finder --locked` (or
`cargo nextest run -p finder --locked`) to run one module, and is what a command in a
landed spec that named the old binary now means.

| Old file (`crates/finder/tests/`) | New file (`crates/finder/tests/`) | Tests | Filter for one module | Last written by |
|---|---|---|---|---|
| `goto_api.rs` | `motion/goto/routes.rs` | 6 | `--test motion goto::routes::` | 014-goto US6 |
| `goto_refusal_api.rs` | `motion/goto/refusals.rs` | 4 | `--test motion goto::refusals::` | 014-goto US8 |
| `goto_sim.rs` | `motion/goto/sim.rs` | 9 | `--test motion goto::sim::` | 014-goto US6 |
| `second_sync_sim.rs` | `motion/goto/second_sync.rs` | 5 | `--test motion goto::second_sync::` | 014-goto US7 |
| `model_sim.rs` | `motion/model/sim.rs` | 5 | `--test motion model::sim::` | 014-goto US5 |
| `model_api.rs` | `motion/model/routes.rs` | 3 | `--test motion model::routes::` | 014-goto US7 |
| `sim_truth.rs` | `motion/model/truth.rs` | 1 | `--test motion model::truth::` | 015-tracking US2 |
| `backlash_sim.rs` | `motion/tracking/backlash.rs` | 5 | `--test motion tracking::backlash::` | 015-tracking US5 |
| `keyhole_sim.rs` | `motion/tracking/keyhole.rs` | 5 | `--test motion tracking::keyhole::` | 015-tracking US6 |
| `tracking_sim.rs` | `motion/tracking/tracker.rs` | 10 | `--test motion tracking::tracker::` | 015-tracking US3 |
| `tracking_loop.rs` | `motion/tracking/pi_loop.rs` | 6 | `--test motion tracking::pi_loop::` | 015-tracking US4 |
| `tracking_night.rs` | `motion/tracking/night.rs` | 2 | `--test motion tracking::night::` | 015-tracking US9 |
| `track_api.rs` | `motion/tracking/routes.rs` | 10 | `--test motion tracking::routes::` | 015-tracking US7 |
| `track_min_alt.rs` | `motion/tracking/min_alt.rs` | 1 | `--test motion tracking::min_alt::` | finder: a minimum-altitude edit reaches a running tracker (# |
| `stop_retry.rs` | `motion/stop/retry.rs` | 11 | `--test motion stop::retry::` | 017-console-stop-confirm US1 |
| `stop_motion_ends.rs` | `motion/stop/motion_ends.rs` | 9 | `--test motion stop::motion_ends::` | 017-console-stop-confirm US2 |
| `watchdog_goto.rs` | `motion/stop/watchdog.rs` | 3 | `--test motion stop::watchdog::` | 017-console-stop-confirm US4 |
| `mount_calls.rs` | `motion/mount_calls.rs` | 3 | `--test motion mount_calls::` | 014-goto US2 |
| `sim_api.rs` | `motion/simulate.rs` | 3 | `--test motion simulate::` | 013-live-finder US7 |
| `night_sim.rs` | `motion/night.rs` | 1 | `--test motion night::` | 014-goto US9 |
| `api.rs` | `console/api.rs` | 14 | `--test console api::` | finder: a local web console that plate-solves a frame and ch |
| `star_detected.rs` | `console/star_detected.rs` | 4 | `--test console star_detected::` | 020-live-chart-calm US1 |
| `live.rs` | `console/live/loop_.rs` | 10 | `--test console live::loop_::` | 013-live-finder US6 |
| `live_stream.rs` | `console/live/stream.rs` | 2 | `--test console live::stream::` | 013-live-finder US7 |
| `e2e_live.rs` | `console/live/whole_loop.rs` | 1 | `--test console live::whole_loop::` | 013-live-finder US7 |
| `aim_api.rs` | `console/aim.rs` | 2 | `--test console aim::` | 013-live-finder US6 |
| `freshness_api.rs` | `console/freshness.rs` | 1 | `--test console freshness::` | 013-live-finder US6 |
| `pushto_api.rs` | `console/pushto.rs` | 7 | `--test console pushto::` | 012-targets-and-push-to US5 |
| `pointing_api.rs` | `console/pointing.rs` | 5 | `--test console pointing::` | 012-targets-and-push-to US4 |
| `targets_api.rs` | `console/targets.rs` | 6 | `--test console targets::` | 012-targets-and-push-to US2 |
| `session_api.rs` | `console/session/edits.rs` | 9 | `--test console session::edits::` | 012-targets-and-push-to US5 |
| `session_writes.rs` | `console/session/writes.rs` | 6 | `--test console session::writes::` | 012-targets-and-push-to US4 |
| `session_node.rs` | `console/session/node.rs` | 4 | `--test console session::node::` | 013-live-finder US5 |
| `rates.rs` | `console/rates.rs` | 6 | `--test console rates::` | 015-tracking US1 |
| `notices.rs` | `console/notices.rs` | 1 | `--test console notices::` | 012-targets-and-push-to US2 |
| `link.rs` | `console/link.rs` | 6 | `--test console link::` | 013-live-finder US5 |
| `mcp_endpoint.rs` | `mcp/endpoint.rs` | 12 | `--test mcp endpoint::` | 016-mcp US1 |
| `mcp_frames.rs` | `mcp/frames.rs` | 7 | `--test mcp frames::` | 016-mcp US3 |
| `mcp_motion.rs` | `mcp/motion.rs` | 10 | `--test mcp motion::` | 016-mcp US4 |
| `mcp_session_tools.rs` | `mcp/session_tools.rs` | 7 | `--test mcp session_tools::` | 016-mcp US2 |
| `mcp_tracking.rs` | `mcp/tracking.rs` | 6 | `--test mcp tracking::` | 016-mcp US6 |
| `mcp_axis_moves.rs` | `mcp/axis_moves.rs` | 17 | `--test mcp axis_moves::` | 016-mcp US5 |
| `cli.rs` | `cli/flags.rs` | 2 | `--test cli flags::` | finder: a local web console that plate-solves a frame and ch |
| `aim_cli.rs` | `cli/aim.rs` | 1 | `--test cli aim::` | 013-live-finder US6 |
| `session_cli.rs` | `cli/session.rs` | 6 | `--test cli session::` | 012-targets-and-push-to US3 |
| `session_node_cli.rs` | `cli/session_node.rs` | 1 | `--test cli session_node::` | 013-live-finder US5 |
| `simulate_cli.rs` | `cli/simulate.rs` | 3 | `--test cli simulate::` | tests: simulate_cli calibrates the aim only on a frame after |
| `track_cli.rs` | `cli/track.rs` | 2 | `--test cli track::` | 015-tracking US7 |
| `backlash_cli.rs` | `cli/backlash.rs` | 1 | `--test cli backlash::` | 015-tracking US5 |

Helpers:

| Old file | New file | After which story |
|---|---|---|
| top of `goto_api.rs` | `support/goto.rs` | US1 |
| top of `backlash_sim.rs` | `support/tracker.rs` | US1 |
| `live/support.rs` | `support/live.rs` | US2 |
| `mcp/support.rs` | `support/mcp.rs` | US2 |
| `stop/support.rs` | `support/stop.rs` | US2 |

Totals: 49 files, 261 tests; `motion` 102, `console` 84, `mcp` 59, `cli` 16.

## Shapes that existing tests pin

- **Test names gain a module path.** `cargo test -p finder --test tracking_sim` named a
  binary; the same tests are now `--test motion tracking::tracker::`. Landed specs keep
  their text. Nothing in the repository runs those commands: the gates run the whole
  workspace.
- **Two tests name themselves** to re-run their binary (FR-008). They are the only
  tests that pass a test name to the harness.
- **One test reads a file relative to itself** (`cli.rs`, the notices).
- **The cedar parity test of the console** (`every_sample_gives_cedars_answer` in
  `api.rs`) reads `crates/plate-solver/tests/data/cedar_parity.json` through
  `CARGO_MANIFEST_DIR`, so moving `api.rs` does not move what it reads.
- **Scratch folders** are built from `CARGO_MANIFEST_DIR` and the process id
  (`../../target/...`), not from the test file's place.
- **The oracle files are in other crates** and are not touched: `reference_parity.rs`,
  `image_parity.rs`, `e2e_service.rs`, `parity.rs`.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| Four binaries by what a test needs to run. | One binary; one binary per source module. | One binary puts 261 tests behind one link and one name. A binary per module is a dozen binaries again. Four matches the four ways finder is driven. |
| Move files whole; a system is a folder. | Split `api.rs`, `session_writes.rs` and others by subject. | A split is not a rename: the diff stops showing that nothing changed. It can follow later, one file at a time. |
| Keep the five helper files as they are. | Merge the three simulator rigs and the four database fixtures now. | That changes helper code under 261 tests in the same change that moves them. It is worth doing, separately. |
| Two stories. | One story; one story per binary. | Measured at 56 KB and 40 KB, both under the factory's preferred 64 KiB. One story would be about 96 KB. Four more stories would each cost a gate and a queue run for a mechanical step. |
| A guard test in a `layout` binary of finder. | A script in the `lint` gate; no guard. | A story cannot add a gate command, and without a guard the next story adds a top-level file again. |
| `use crate::support::x as name;` | Rewriting each file's `use support::...` lines. | One changed line a file, and every test body stays byte for byte. |
| `pub use super::live as support;` in the two rigs. | Including `live.rs` a second time by path. | A second include compiles a second database fixture into the binary. |
| A `mod` line for every module, written out. | A macro or build script that declares a module for every file in a folder. | Cargo does not rebuild a test binary because a file appeared beside it, so a new test file could go uncompiled and its tests unrun while the gate passed. A written `mod` line cannot be skipped that way. The price is approval 7 in spec.md: one shared line for each new module. |
| Leave `rates.rs` and `notices.rs` as integration tests. | Move them into `crates/finder/src`. | They are pure tests and would be better inline, but that edits product source files; out of scope. |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` with pull request 214 merged):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.
- **CI in the merge queue** runs every required `ci.yml` job, including the cedar e2e
  eval. The coverage floor runs only by hand.

## Prerequisites before this spec is flipped to `ready`

1. **Pull request 214 has merged and the factory's checkout is at or after it.** The
   `test` gate's first command must be the nextest line in § Verification, which is the
   command spec.md's scenarios name.
2. **The earlier specs have landed:** 012 through 020. Met at 870b332.
3. **`crates/finder/tests` is still as read.** Every line number here is at 870b332. If
   a file there has changed since, re-read it and re-run the probe.
4. **The owner approvals are given.** Flipping gives them.
5. **024, 025 and 026 wait for this one.** Each adds test files to
   `crates/finder/tests`; their drafts are refreshed against this layout after it lands.
6. **The refinement probe has run** (§ Refinement Record).

## Plan traps

1. **Gate evidence or nothing.** The judge sees the diff and each gate's command and
   exit code, not the test output. *Check:* every Then clause names a committed test in
   the diff, or something read from the diff.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **A move that is not a rename looks like a rewrite.** *Check:* every move is a
   `git mv`, and `git diff -M --name-status` shows `R` for all 52 files of US2.
4. **A vacuous pass.** A test that re-runs its binary with a name that matches nothing
   passes. *Check:* FR-008's assertion that the child ran one test is in the diff.
5. **Warnings are errors.** An unused import in the two split test files, or a helper
   item one binary does not use, fails the build. *Check:* the headers and attributes of
   § Interfaces, exactly.
6. **A scenario that is green before the change is a guard, not evidence.** *Check:*
   US1-S1 and US1-S2 fail at 870b332; US2-S1, US2-S2 and US2-S3 fail after US1.
7. **Formatting is the repository's.** *Check:* run `cargo fmt --all`, then
   `cargo fmt --all --check`. Do not reformat anything by hand.
8. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/027-finder-tests-by-system/`.
9. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
10. **The guard must not match itself.** Its own source holds the text it searches for.
    *Check:* it matches only at the start of a line, as written.

## Risks

| Risk | Mitigation |
|---|---|
| A test depends on running alone in its process. | The probe ran all 350 finder tests under `cargo nextest` (a process each) and under `cargo test` (a process a binary) and all passed. |
| Another story adds a test file to `crates/finder/tests` while this epic runs. | 024, 025 and 026 wait for it; nothing else is ready. A file that does arrive fails US2's guard by name. |
| The split helper files drift from this plan's line numbers. | Prerequisite 3; the split points are also given as text, not only as numbers. |
| A builder tidies a helper while moving it. | FR-002, FR-006 and FR-007 list the only edits; US1-S4 and US2-S6 are read from the diff. |
| A later story's `mod` line conflicts with another story's. Each new test module adds one line to `main.rs` or a folder's `mod.rs`; two stories adding neighbouring lines to one file do not merge cleanly. | A spec whose stories add modules to the same file orders them with `depends_on_merged` and lists the file under `shared_writes`, naming this spec's US2 as its last writer. Lines are alphabetical, one module a line, so only neighbours collide. 024, 025 and 026 as drafted already order the stories that add finder tests. Two epics in flight that add neighbours to one file remain possible; the later one would need its story rebuilt on the new `main`. |
| The recorders rot, since nothing compiles them. | They are edited in both stories (FR-010, FR-011); re-recording is by hand and rare. |

## Refinement Record

Drafted on 2026-11-14 by the plate-solver operator against `main` at 870b332. That day
both stories were built, by script and in order, in a throwaway worktree of 4b4cc96
(`main` at f8dab49 with pull request 214's gate change; `crates/finder/tests` is the
same at 870b332), with the factory's own toolchain and the gates' `RUSTFLAGS`:

- **US1** (tag `probe-027-us1`): the two `layout` tests failed before and passed after.
  `cargo fmt --all --check` passed, the build passed with warnings denied, and
  `cargo nextest run -p finder --all-targets --all-features --locked` ran 349 tests in
  52 binaries, all passing (388 before, less the 41 extra runs, plus 2). 14 files
  changed; the factory's output check measured the diff at 56,437 bytes, with no size
  warning. A first attempt that also moved the three older helper files measured
  68,727 bytes, which is why they move in US2.
- **US2** (tag `probe-027-us2`): the third `layout` test failed after US1 and passed
  after. 350 tests in 7 binaries, all passing, under `cargo nextest`; an earlier run of
  the same layout under `cargo test -p finder --all-targets` passed too (cli 16, console
  84, mcp 59, motion 102). The `lint` gate passed as declared. 68 files changed, 52 of
  them renames; the output check measured 39,683 bytes. The two re-run tests passed with
  FR-008's assertion, so the child did run one test.
- **Found by the probe:** the two re-run tests pass vacuously in a merged binary unless
  FR-008 is applied; the split helper files need `pub` items, the re-export and
  `#![allow(dead_code)]`; a trailing blank line in a split file fails `cargo fmt`.
- **Not run:** the page half of the `test` gate, which this spec does not touch, and the
  gates inside the factory's sandbox.

Writes come from the task text. Shared writes name the landed stories that last wrote
each file. Nothing in this record is an owner approval: those are in spec.md, and
flipping to `ready` gives them.

```yaml
version: 1
reviewed_revision: 870b332c5282d1395cc3389b83d38188e5d0b63d
stories:
  US1:
    tests:
    - path: crates/finder/tests/layout/main.rs
      status: new
      disposition: add
      assertion: "no file named by a path attribute holds a test; support/ holds goto.rs and tracker.rs and no test"
      decision: "spec.md:US1"
    - path: crates/finder/tests/goto_api.rs
      status: existing
      disposition: preserve
      assertion: "its six GoTo route tests, bodies unchanged, under a header that includes support/goto.rs"
      decision: "plan.md:Interfaces"
    - path: crates/finder/tests/backlash_sim.rs
      status: existing
      disposition: preserve
      assertion: "its five backlash tests, bodies unchanged, under a header that includes support/tracker.rs"
      decision: "plan.md:Interfaces"
    - path: crates/finder/tests/goto_refusal_api.rs
      status: existing
      disposition: preserve
      assertion: "every test of its own unchanged; only the path attribute changes, so the included rig's tests no longer compile into it"
      decision: "spec.md:FR-003"
    - path: crates/finder/tests/goto_sim.rs
      status: existing
      disposition: preserve
      assertion: "every test of its own unchanged; only the path attribute changes, so the included rig's tests no longer compile into it"
      decision: "spec.md:FR-003"
    - path: crates/finder/tests/model_api.rs
      status: existing
      disposition: preserve
      assertion: "every test of its own unchanged; only the path attribute changes, so the included rig's tests no longer compile into it"
      decision: "spec.md:FR-003"
    - path: crates/finder/tests/second_sync_sim.rs
      status: existing
      disposition: preserve
      assertion: "every test of its own unchanged; only the path attribute changes, so the included rig's tests no longer compile into it"
      decision: "spec.md:FR-003"
    - path: crates/finder/tests/track_api.rs
      status: existing
      disposition: preserve
      assertion: "every test of its own unchanged; only the path attribute changes, so the included rig's tests no longer compile into it"
      decision: "spec.md:FR-003"
    - path: crates/finder/tests/track_min_alt.rs
      status: existing
      disposition: preserve
      assertion: "every test of its own unchanged; only the path attribute changes, so the included rig's tests no longer compile into it"
      decision: "spec.md:FR-003"
    - path: crates/finder/tests/backlash_cli.rs
      status: existing
      disposition: preserve
      assertion: "every test of its own unchanged; only the path attribute changes, so the included rig's tests no longer compile into it"
      decision: "spec.md:FR-003"
    preserved_behavior:
    - "each of the six tests of goto_api.rs and the five of backlash_sim.rs still runs, once (spec.md FR-001)"
    - no line inside a test function changes (spec.md SC-003)
    - "the rigs' behaviour: support/goto.rs and support/tracker.rs differ from the code they came from only in visibility, one include path and one re-export (spec.md FR-002)"
    checks:
      development:
      - cargo nextest run -p finder --locked --test layout
      - cargo nextest run -p finder --all-targets --all-features --locked
      - cargo fmt --all --check
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/layout/main.rs
    - crates/finder/tests/support/goto.rs
    - crates/finder/tests/support/tracker.rs
    - crates/finder/tests/goto_api.rs
    - crates/finder/tests/backlash_sim.rs
    - crates/finder/tests/goto_refusal_api.rs
    - crates/finder/tests/goto_sim.rs
    - crates/finder/tests/model_api.rs
    - crates/finder/tests/second_sync_sim.rs
    - crates/finder/tests/track_api.rs
    - crates/finder/tests/track_min_alt.rs
    - crates/finder/tests/backlash_cli.rs
    - crates/finder/web/src/test/fixtures/record-sessions.rs
    - crates/finder/web/src/test/fixtures/record-tracking.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/goto_api.rs
      other_spec: 014-goto
      other_story: US6
      disposition: merge_order
      reason: "027 depends_on_landed 014-goto, whose US6 last wrote this file"
    - path: crates/finder/tests/backlash_sim.rs
      other_spec: 015-tracking
      other_story: US5
      disposition: merge_order
      reason: "027 depends_on_landed 015-tracking, whose US5 last wrote this file"
    - path: crates/finder/tests/goto_refusal_api.rs
      other_spec: 014-goto
      other_story: US8
      disposition: merge_order
      reason: "027 depends_on_landed 014-goto, whose US8 last wrote this file"
    - path: crates/finder/tests/goto_sim.rs
      other_spec: 014-goto
      other_story: US6
      disposition: merge_order
      reason: "027 depends_on_landed 014-goto, whose US6 last wrote this file"
    - path: crates/finder/tests/model_api.rs
      other_spec: 014-goto
      other_story: US7
      disposition: merge_order
      reason: "027 depends_on_landed 014-goto, whose US7 last wrote this file"
    - path: crates/finder/tests/second_sync_sim.rs
      other_spec: 014-goto
      other_story: US7
      disposition: merge_order
      reason: "027 depends_on_landed 014-goto, whose US7 last wrote this file"
    - path: crates/finder/tests/track_api.rs
      other_spec: 015-tracking
      other_story: US7
      disposition: merge_order
      reason: "027 depends_on_landed 015-tracking, whose US7 last wrote this file"
    - path: crates/finder/tests/backlash_cli.rs
      other_spec: 015-tracking
      other_story: US5
      disposition: merge_order
      reason: "027 depends_on_landed 015-tracking, whose US5 last wrote this file"
    - path: crates/finder/web/src/test/fixtures/record-sessions.rs
      other_spec: 014-goto
      other_story: US8
      disposition: merge_order
      reason: "027 depends_on_landed 014-goto, whose US8 last wrote this file"
    - path: crates/finder/web/src/test/fixtures/record-tracking.rs
      other_spec: 015-tracking
      other_story: US8
      disposition: merge_order
      reason: "027 depends_on_landed 015-tracking, whose US8 last wrote this file"
    - path: crates/finder/tests/layout/main.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and appends its test to this file"
    - path: crates/finder/tests/support/goto.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and changes how this helper reaches the live helpers"
    - path: crates/finder/tests/support/tracker.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and changes how this helper reaches the live helpers"
    - path: crates/finder/tests/goto_api.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and moves this file into a binary's folder"
    - path: crates/finder/tests/backlash_sim.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and moves this file into a binary's folder"
    - path: crates/finder/tests/goto_refusal_api.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and moves this file into a binary's folder"
    - path: crates/finder/tests/goto_sim.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and moves this file into a binary's folder"
    - path: crates/finder/tests/model_api.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and moves this file into a binary's folder"
    - path: crates/finder/tests/second_sync_sim.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and moves this file into a binary's folder"
    - path: crates/finder/tests/track_api.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and moves this file into a binary's folder"
    - path: crates/finder/tests/track_min_alt.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and moves this file into a binary's folder"
    - path: crates/finder/tests/backlash_cli.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and moves this file into a binary's folder"
    - path: crates/finder/web/src/test/fixtures/record-sessions.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and changes the include again"
    - path: crates/finder/web/src/test/fixtures/record-tracking.rs
      other_spec: 027-finder-tests-by-system
      other_story: US2
      disposition: merge_order
      reason: "US2 depends_on_merged US1, and changes the include again"
    open_questions: []
  US2:
    tests:
    - path: crates/finder/tests/layout/main.rs
      status: new
      disposition: add
      assertion: "tests/ holds exactly cli, console, layout, mcp, motion and support; support/ holds exactly its six files; each main.rs is the support include and module declarations"
      decision: "spec.md:US2"
    - path: crates/finder/tests/goto_api.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/goto/routes.rs with its 6 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/goto_refusal_api.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/goto/refusals.rs with its 4 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/goto_sim.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/goto/sim.rs with its 9 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/second_sync_sim.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/goto/second_sync.rs with its 5 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/model_sim.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/model/sim.rs with its 5 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/model_api.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/model/routes.rs with its 3 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/sim_truth.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/model/truth.rs with its 1 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/backlash_sim.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/tracking/backlash.rs with its 5 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/keyhole_sim.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/tracking/keyhole.rs with its 5 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/tracking_sim.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/tracking/tracker.rs with its 10 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/tracking_loop.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/tracking/pi_loop.rs with its 6 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/tracking_night.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/tracking/night.rs with its 2 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/track_api.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/tracking/routes.rs with its 10 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/track_min_alt.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/tracking/min_alt.rs with its 1 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/stop_retry.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/stop/retry.rs with its 11 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/stop_motion_ends.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/stop/motion_ends.rs with its 9 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/watchdog_goto.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/stop/watchdog.rs with its 3 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/mount_calls.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/mount_calls.rs with its 3 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/sim_api.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/simulate.rs with its 3 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/night_sim.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/motion/night.rs with its 1 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/api.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/api.rs with its 14 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/star_detected.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/star_detected.rs with its 4 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/live.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/live/loop_.rs with its 10 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/live_stream.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/live/stream.rs with its 2 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/e2e_live.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/live/whole_loop.rs with its 1 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/aim_api.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/aim.rs with its 2 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/freshness_api.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/freshness.rs with its 1 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/pushto_api.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/pushto.rs with its 7 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/pointing_api.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/pointing.rs with its 5 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/targets_api.rs
      status: existing
      disposition: preserve
      assertion: "moved to crates/finder/tests/console/targets.rs with its 6 test(s) unchanged; its re-run test names itself by full path and asserts the child ran one test (spec.md FR-008)"
      decision: "plan.md:Move map"
    - path: crates/finder/tests/session_api.rs
      status: existing
      disposition: preserve
      assertion: "moved to crates/finder/tests/console/session/edits.rs with its 9 test(s) unchanged; its re-run test names itself by full path and asserts the child ran one test (spec.md FR-008)"
      decision: "plan.md:Move map"
    - path: crates/finder/tests/session_writes.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/session/writes.rs with its 6 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/session_node.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/session/node.rs with its 4 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/rates.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/rates.rs with its 6 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/notices.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/notices.rs with its 1 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/link.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/console/link.rs with its 6 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/mcp_endpoint.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/mcp/endpoint.rs with its 12 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/mcp_frames.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/mcp/frames.rs with its 7 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/mcp_motion.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/mcp/motion.rs with its 10 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/mcp_session_tools.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/mcp/session_tools.rs with its 7 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/mcp_tracking.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/mcp/tracking.rs with its 6 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/mcp_axis_moves.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/mcp/axis_moves.rs with its 17 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/cli.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/cli/flags.rs with its 2 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/aim_cli.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/cli/aim.rs with its 1 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/session_cli.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/cli/session.rs with its 6 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/session_node_cli.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/cli/session_node.rs with its 1 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/simulate_cli.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/cli/simulate.rs with its 3 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/track_cli.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/cli/track.rs with its 2 test(s) unchanged
      decision: "plan.md:Move map"
    - path: crates/finder/tests/backlash_cli.rs
      status: existing
      disposition: preserve
      assertion: moved to crates/finder/tests/cli/backlash.rs with its 1 test(s) unchanged
      decision: "plan.md:Move map"
    preserved_behavior:
    - "all 261 finder integration tests still run, each once, with unchanged bodies except the two FR-008 names (spec.md SC-003)"
    - "US1's two layout tests still pass and are not edited (spec.md US2-S7)"
    - "the console's cedar parity test and the file it reads (crates/plate-solver/tests/data/cedar_parity.json) are unchanged"
    - "the helper files live.rs, mcp.rs and stop.rs are moved byte for byte (spec.md FR-006)"
    checks:
      development:
      - cargo nextest run -p finder --locked --test layout
      - cargo nextest run -p finder --all-targets --all-features --locked
      - cargo test -p finder --all-targets --all-features --locked
      - cargo fmt --all --check
      - git diff -M --name-status
      handoff: declared_gates
      independent: declared_pipeline
      release:
      - "release owner: the repository owner, through the merge queue: test, lint and the ci.yml jobs"
    writes:
    - crates/finder/tests/goto_api.rs
    - crates/finder/tests/goto_refusal_api.rs
    - crates/finder/tests/goto_sim.rs
    - crates/finder/tests/second_sync_sim.rs
    - crates/finder/tests/model_sim.rs
    - crates/finder/tests/model_api.rs
    - crates/finder/tests/sim_truth.rs
    - crates/finder/tests/backlash_sim.rs
    - crates/finder/tests/keyhole_sim.rs
    - crates/finder/tests/tracking_sim.rs
    - crates/finder/tests/tracking_loop.rs
    - crates/finder/tests/tracking_night.rs
    - crates/finder/tests/track_api.rs
    - crates/finder/tests/track_min_alt.rs
    - crates/finder/tests/stop_retry.rs
    - crates/finder/tests/stop_motion_ends.rs
    - crates/finder/tests/watchdog_goto.rs
    - crates/finder/tests/mount_calls.rs
    - crates/finder/tests/sim_api.rs
    - crates/finder/tests/night_sim.rs
    - crates/finder/tests/api.rs
    - crates/finder/tests/star_detected.rs
    - crates/finder/tests/live.rs
    - crates/finder/tests/live_stream.rs
    - crates/finder/tests/e2e_live.rs
    - crates/finder/tests/aim_api.rs
    - crates/finder/tests/freshness_api.rs
    - crates/finder/tests/pushto_api.rs
    - crates/finder/tests/pointing_api.rs
    - crates/finder/tests/targets_api.rs
    - crates/finder/tests/session_api.rs
    - crates/finder/tests/session_writes.rs
    - crates/finder/tests/session_node.rs
    - crates/finder/tests/rates.rs
    - crates/finder/tests/notices.rs
    - crates/finder/tests/link.rs
    - crates/finder/tests/mcp_endpoint.rs
    - crates/finder/tests/mcp_frames.rs
    - crates/finder/tests/mcp_motion.rs
    - crates/finder/tests/mcp_session_tools.rs
    - crates/finder/tests/mcp_tracking.rs
    - crates/finder/tests/mcp_axis_moves.rs
    - crates/finder/tests/cli.rs
    - crates/finder/tests/aim_cli.rs
    - crates/finder/tests/session_cli.rs
    - crates/finder/tests/session_node_cli.rs
    - crates/finder/tests/simulate_cli.rs
    - crates/finder/tests/track_cli.rs
    - crates/finder/tests/backlash_cli.rs
    - crates/finder/tests/live/support.rs
    - crates/finder/tests/mcp/support.rs
    - crates/finder/tests/stop/support.rs
    - crates/finder/tests/cli/aim.rs
    - crates/finder/tests/cli/backlash.rs
    - crates/finder/tests/cli/flags.rs
    - crates/finder/tests/cli/main.rs
    - crates/finder/tests/cli/session.rs
    - crates/finder/tests/cli/session_node.rs
    - crates/finder/tests/cli/simulate.rs
    - crates/finder/tests/cli/track.rs
    - crates/finder/tests/console/aim.rs
    - crates/finder/tests/console/api.rs
    - crates/finder/tests/console/freshness.rs
    - crates/finder/tests/console/link.rs
    - crates/finder/tests/console/live/loop_.rs
    - crates/finder/tests/console/live/mod.rs
    - crates/finder/tests/console/live/stream.rs
    - crates/finder/tests/console/live/whole_loop.rs
    - crates/finder/tests/console/main.rs
    - crates/finder/tests/console/notices.rs
    - crates/finder/tests/console/pointing.rs
    - crates/finder/tests/console/pushto.rs
    - crates/finder/tests/console/rates.rs
    - crates/finder/tests/console/session/edits.rs
    - crates/finder/tests/console/session/mod.rs
    - crates/finder/tests/console/session/node.rs
    - crates/finder/tests/console/session/writes.rs
    - crates/finder/tests/console/star_detected.rs
    - crates/finder/tests/console/targets.rs
    - crates/finder/tests/layout/main.rs
    - crates/finder/tests/mcp/axis_moves.rs
    - crates/finder/tests/mcp/endpoint.rs
    - crates/finder/tests/mcp/frames.rs
    - crates/finder/tests/mcp/main.rs
    - crates/finder/tests/mcp/motion.rs
    - crates/finder/tests/mcp/session_tools.rs
    - crates/finder/tests/mcp/tracking.rs
    - crates/finder/tests/motion/goto/mod.rs
    - crates/finder/tests/motion/goto/refusals.rs
    - crates/finder/tests/motion/goto/routes.rs
    - crates/finder/tests/motion/goto/second_sync.rs
    - crates/finder/tests/motion/goto/sim.rs
    - crates/finder/tests/motion/main.rs
    - crates/finder/tests/motion/model/mod.rs
    - crates/finder/tests/motion/model/routes.rs
    - crates/finder/tests/motion/model/sim.rs
    - crates/finder/tests/motion/model/truth.rs
    - crates/finder/tests/motion/mount_calls.rs
    - crates/finder/tests/motion/night.rs
    - crates/finder/tests/motion/simulate.rs
    - crates/finder/tests/motion/stop/mod.rs
    - crates/finder/tests/motion/stop/motion_ends.rs
    - crates/finder/tests/motion/stop/retry.rs
    - crates/finder/tests/motion/stop/watchdog.rs
    - crates/finder/tests/motion/tracking/backlash.rs
    - crates/finder/tests/motion/tracking/keyhole.rs
    - crates/finder/tests/motion/tracking/min_alt.rs
    - crates/finder/tests/motion/tracking/mod.rs
    - crates/finder/tests/motion/tracking/night.rs
    - crates/finder/tests/motion/tracking/pi_loop.rs
    - crates/finder/tests/motion/tracking/routes.rs
    - crates/finder/tests/motion/tracking/tracker.rs
    - crates/finder/tests/support/goto.rs
    - crates/finder/tests/support/live.rs
    - crates/finder/tests/support/mcp.rs
    - crates/finder/tests/support/mod.rs
    - crates/finder/tests/support/stop.rs
    - crates/finder/tests/support/tracker.rs
    - crates/finder/web/src/test/fixtures/record-sessions.rs
    - crates/finder/web/src/test/fixtures/record-tracking.rs
    no_write_reason: ""
    shared_writes:
    - path: crates/finder/tests/model_sim.rs
      other_spec: 014-goto
      other_story: US5
      disposition: merge_order
      reason: "027 depends_on_landed 014-goto, whose US5 last wrote this file"
    - path: crates/finder/tests/sim_truth.rs
      other_spec: 015-tracking
      other_story: US2
      disposition: merge_order
      reason: "027 depends_on_landed 015-tracking, whose US2 last wrote this file"
    - path: crates/finder/tests/keyhole_sim.rs
      other_spec: 015-tracking
      other_story: US6
      disposition: merge_order
      reason: "027 depends_on_landed 015-tracking, whose US6 last wrote this file"
    - path: crates/finder/tests/tracking_sim.rs
      other_spec: 015-tracking
      other_story: US3
      disposition: merge_order
      reason: "027 depends_on_landed 015-tracking, whose US3 last wrote this file"
    - path: crates/finder/tests/tracking_loop.rs
      other_spec: 015-tracking
      other_story: US4
      disposition: merge_order
      reason: "027 depends_on_landed 015-tracking, whose US4 last wrote this file"
    - path: crates/finder/tests/tracking_night.rs
      other_spec: 015-tracking
      other_story: US9
      disposition: merge_order
      reason: "027 depends_on_landed 015-tracking, whose US9 last wrote this file"
    - path: crates/finder/tests/stop_retry.rs
      other_spec: 017-console-stop-confirm
      other_story: US1
      disposition: merge_order
      reason: "027 depends_on_landed 017-console-stop-confirm, whose US1 last wrote this file"
    - path: crates/finder/tests/stop_motion_ends.rs
      other_spec: 017-console-stop-confirm
      other_story: US2
      disposition: merge_order
      reason: "027 depends_on_landed 017-console-stop-confirm, whose US2 last wrote this file"
    - path: crates/finder/tests/watchdog_goto.rs
      other_spec: 017-console-stop-confirm
      other_story: US4
      disposition: merge_order
      reason: "027 depends_on_landed 017-console-stop-confirm, whose US4 last wrote this file"
    - path: crates/finder/tests/mount_calls.rs
      other_spec: 014-goto
      other_story: US2
      disposition: merge_order
      reason: "027 depends_on_landed 014-goto, whose US2 last wrote this file"
    - path: crates/finder/tests/sim_api.rs
      other_spec: 013-live-finder
      other_story: US7
      disposition: merge_order
      reason: "027 depends_on_landed 013-live-finder, whose US7 last wrote this file"
    - path: crates/finder/tests/night_sim.rs
      other_spec: 014-goto
      other_story: US9
      disposition: merge_order
      reason: "027 depends_on_landed 014-goto, whose US9 last wrote this file"
    - path: crates/finder/tests/star_detected.rs
      other_spec: 020-live-chart-calm
      other_story: US1
      disposition: merge_order
      reason: "027 depends_on_landed 020-live-chart-calm, whose US1 last wrote this file"
    - path: crates/finder/tests/live.rs
      other_spec: 013-live-finder
      other_story: US6
      disposition: merge_order
      reason: "027 depends_on_landed 013-live-finder, whose US6 last wrote this file"
    - path: crates/finder/tests/live_stream.rs
      other_spec: 013-live-finder
      other_story: US7
      disposition: merge_order
      reason: "027 depends_on_landed 013-live-finder, whose US7 last wrote this file"
    - path: crates/finder/tests/e2e_live.rs
      other_spec: 013-live-finder
      other_story: US7
      disposition: merge_order
      reason: "027 depends_on_landed 013-live-finder, whose US7 last wrote this file"
    - path: crates/finder/tests/aim_api.rs
      other_spec: 013-live-finder
      other_story: US6
      disposition: merge_order
      reason: "027 depends_on_landed 013-live-finder, whose US6 last wrote this file"
    - path: crates/finder/tests/freshness_api.rs
      other_spec: 013-live-finder
      other_story: US6
      disposition: merge_order
      reason: "027 depends_on_landed 013-live-finder, whose US6 last wrote this file"
    - path: crates/finder/tests/pushto_api.rs
      other_spec: 012-targets-and-push-to
      other_story: US5
      disposition: merge_order
      reason: "027 depends_on_landed 012-targets-and-push-to, whose US5 last wrote this file"
    - path: crates/finder/tests/pointing_api.rs
      other_spec: 012-targets-and-push-to
      other_story: US4
      disposition: merge_order
      reason: "027 depends_on_landed 012-targets-and-push-to, whose US4 last wrote this file"
    - path: crates/finder/tests/targets_api.rs
      other_spec: 012-targets-and-push-to
      other_story: US2
      disposition: merge_order
      reason: "027 depends_on_landed 012-targets-and-push-to, whose US2 last wrote this file"
    - path: crates/finder/tests/session_api.rs
      other_spec: 012-targets-and-push-to
      other_story: US5
      disposition: merge_order
      reason: "027 depends_on_landed 012-targets-and-push-to, whose US5 last wrote this file"
    - path: crates/finder/tests/session_writes.rs
      other_spec: 012-targets-and-push-to
      other_story: US4
      disposition: merge_order
      reason: "027 depends_on_landed 012-targets-and-push-to, whose US4 last wrote this file"
    - path: crates/finder/tests/session_node.rs
      other_spec: 013-live-finder
      other_story: US5
      disposition: merge_order
      reason: "027 depends_on_landed 013-live-finder, whose US5 last wrote this file"
    - path: crates/finder/tests/rates.rs
      other_spec: 015-tracking
      other_story: US1
      disposition: merge_order
      reason: "027 depends_on_landed 015-tracking, whose US1 last wrote this file"
    - path: crates/finder/tests/notices.rs
      other_spec: 012-targets-and-push-to
      other_story: US2
      disposition: merge_order
      reason: "027 depends_on_landed 012-targets-and-push-to, whose US2 last wrote this file"
    - path: crates/finder/tests/link.rs
      other_spec: 013-live-finder
      other_story: US5
      disposition: merge_order
      reason: "027 depends_on_landed 013-live-finder, whose US5 last wrote this file"
    - path: crates/finder/tests/mcp_endpoint.rs
      other_spec: 016-mcp
      other_story: US1
      disposition: merge_order
      reason: "027 depends_on_landed 016-mcp, whose US1 last wrote this file"
    - path: crates/finder/tests/mcp_frames.rs
      other_spec: 016-mcp
      other_story: US3
      disposition: merge_order
      reason: "027 depends_on_landed 016-mcp, whose US3 last wrote this file"
    - path: crates/finder/tests/mcp_motion.rs
      other_spec: 016-mcp
      other_story: US4
      disposition: merge_order
      reason: "027 depends_on_landed 016-mcp, whose US4 last wrote this file"
    - path: crates/finder/tests/mcp_session_tools.rs
      other_spec: 016-mcp
      other_story: US2
      disposition: merge_order
      reason: "027 depends_on_landed 016-mcp, whose US2 last wrote this file"
    - path: crates/finder/tests/mcp_tracking.rs
      other_spec: 016-mcp
      other_story: US6
      disposition: merge_order
      reason: "027 depends_on_landed 016-mcp, whose US6 last wrote this file"
    - path: crates/finder/tests/mcp_axis_moves.rs
      other_spec: 016-mcp
      other_story: US5
      disposition: merge_order
      reason: "027 depends_on_landed 016-mcp, whose US5 last wrote this file"
    - path: crates/finder/tests/aim_cli.rs
      other_spec: 013-live-finder
      other_story: US6
      disposition: merge_order
      reason: "027 depends_on_landed 013-live-finder, whose US6 last wrote this file"
    - path: crates/finder/tests/session_cli.rs
      other_spec: 012-targets-and-push-to
      other_story: US3
      disposition: merge_order
      reason: "027 depends_on_landed 012-targets-and-push-to, whose US3 last wrote this file"
    - path: crates/finder/tests/session_node_cli.rs
      other_spec: 013-live-finder
      other_story: US5
      disposition: merge_order
      reason: "027 depends_on_landed 013-live-finder, whose US5 last wrote this file"
    - path: crates/finder/tests/track_cli.rs
      other_spec: 015-tracking
      other_story: US7
      disposition: merge_order
      reason: "027 depends_on_landed 015-tracking, whose US7 last wrote this file"
    - path: crates/finder/tests/live/support.rs
      other_spec: 013-live-finder
      other_story: US6
      disposition: merge_order
      reason: "027 depends_on_landed 013-live-finder, whose US6 last wrote this file"
    - path: crates/finder/tests/mcp/support.rs
      other_spec: 016-mcp
      other_story: US4
      disposition: merge_order
      reason: "027 depends_on_landed 016-mcp, whose US4 last wrote this file"
    - path: crates/finder/tests/stop/support.rs
      other_spec: 017-console-stop-confirm
      other_story: US2
      disposition: merge_order
      reason: "027 depends_on_landed 017-console-stop-confirm, whose US2 last wrote this file"
    open_questions: []
```
