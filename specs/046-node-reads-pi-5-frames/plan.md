# Implementation Plan: The Node Reads the Pi 5's Camera Frames

**Branch**: `046-node-reads-pi-5-frames` | **Date**: 2026-11-21 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

The rpicam camera adapter stops assuming the Pi 3B+'s 64-byte row stride. `yuv::Isp`
names the two Raspberry Pi ISPs and their strides, 64 for the VC4 and 128 for the PiSP,
beside `yuv::stride`, which keeps its 64. `listing::isp` reads the ISP from the verbose
camera listing the node already parses at start: its `PipelineHandler` line, or, from an
rpicam-apps that does not print one, an `/rp1/` device path. `discover` frames
rpicam-vid's stream at that ISP's stride. `YuvReader` refuses a frame whose row padding
is not 0, naming the frame, the stride and the width, so a stream at any other stride
fails on its first frame where today it stalls with nothing logged. An unknown pipeline
handler stops the node at start, naming it.

One story, with tests on the saved Pi 5 capture the operator stages (spec owner decision
2), on the Pi 3B+ capture already in `fixtures/scope-node/`, and on synthetic streams at
every listed mode.

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI.

**Primary Dependencies**: none new. `serde_json`, `scope-link` and `tonic`, which the
tests use, are dependencies of the node already. `Cargo.toml` and `Cargo.lock` do not
change.

**Storage**: none.

**Testing**:
- `crates/scope-node/tests/rpicam_node.rs` (new): the built binary over a fake
  `rpicam-hello` and `rpicam-vid`, under `#[tokio::test]` on the wall clock, its frames
  read with the raw-frame call (US1-S1, US1-S2, US1-S5, US1-S9).
- `crates/scope-node/tests/rpicam_stride.rs` (new): the rules on saved listings and
  measured strides, and the adapter over a fake `rpicam-vid`, under `#[tokio::test]`
  (US1-S3, US1-S4, US1-S6 to US1-S8).
- No Vitest or Playwright test: the page is unchanged.

**Target Platform**: `scope-node` on the Pi 5 and the Pi 3B+, under systemd.

**Constraints**:
- `yuv::stride`, `YuvReader::new`, `RpicamCamera::new`, `with_path`, `listing::parse`
  and `CameraInfo` keep their shapes; existing tests call them (§ Shapes that existing
  tests pin).
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, and rustdoc with warnings denied.
- No test uses a camera or a Pi. No test sends a signal to a process it did not start.

**Scale/Scope**: one story; three changed source files, two new test files, no doc.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, star detection or `crates/math-core`. The behaviour is grounded in measurement: the Pi 5's strides as the seat measured them on rpicam-vid's own output, the Pi 3B+'s as `fixtures/scope-node/README.md` records them, both captures' padding read byte by byte, and the listings as the two Pis printed them. What rests on reading, such as libcamera's two Raspberry Pi pipeline handlers and rpicam-vid's write order, is listed in § What was run and what was read and is not invented beyond it. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. The existing test that pins `yuv::stride` to the Pi 3B+'s 1344 (`crates/scope-node/tests/rpicam.rs:96`) passes unchanged because `yuv::stride` keeps its 64. The new tests go in new files. The fixture is the operator's, staged before the flip (spec owner decision 2). |
| III. Tests first, independent expectations | The oracles are the saved captures' bytes, the strides their `fixtures.json` and README record, synthetic streams whose every byte the test sets, the process's exit status and standard error, and the scenarios' own words. No test computes an expected stride with `Isp` or `yuv::stride`, or formats an expected message from the code. The binary's tests over the Pi 5 capture fail on `main` (§ What was run and what was read). |
| IV. Critical paths | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe`, nothing under `proto/` or `crates/scope-link`, no `build.rs`. |
| V. Proven open source first, honest licensing | Nothing is designed that existing software supplies. The stride is libcamera's, as rpicam-vid writes it; the node reads what rpicam-apps already prints. Looked for: the stride in rpicam-vid's `-v 2` log (the seat's measurement: not printed). By reading, libcamera's stream configuration carries the stride, but the node runs rpicam-apps as a separate program (013) and links no libcamera. Not looked for: an rpicam-vid option that writes unpadded rows; the probe ran no rpicam-vid. No reused code, no new license. |

## Project Structure

```text
crates/scope-node/
├── src/rpicam/
│   ├── yuv.rs                                # US1: Isp and its stride; yuv::stride through it; the padding check
│   ├── listing.rs                            # US1: listing::isp
│   └── mod.rs                                # US1: RpicamCamera's isp, with_isp, discover, start
└── tests/
    ├── rpicam_node.rs                        # US1, new
    └── rpicam_stride.rs                      # US1, new
fixtures/scope-node/pi5/                      # the operator's, staged before the flip (§ Fixtures)
```

## Interfaces

### `crates/scope-node/src/rpicam/yuv.rs` (US1)

The module comment becomes
`//! YUV420 framing at the row stride the ISP writes; discard Y padding and UV.` Before
`stride`:

```rust
/// The Raspberry Pi ISP behind the camera, which sets the row stride of
/// rpicam-vid's YUV420 frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Isp {
    /// VC4, as on the Pi 3B+: rows padded to a multiple of 64 bytes (measured).
    Vc4,
    /// PiSP, as on the Pi 5: rows padded to a multiple of 128 bytes (measured).
    Pisp,
}
impl Isp {
    /// The row stride of a frame `width` pixels wide.
    pub fn stride(self, width: u32) -> usize {
        let alignment = match self {
            Self::Vc4 => 64,
            Self::Pisp => 128,
        };
        (width as usize).div_ceil(alignment) * alignment
    }
}
```

`stride` keeps its signature, gains the doc comment `/// The VC4's row stride, measured
on the Pi 3B+.`, and its body becomes `Isp::Vc4.stride(width)`.

`YuvReader` gains `frames: u64`, set to 0 by `new`. In `next`, after the `read_exact`
(`crates/scope-node/src/rpicam/yuv.rs:50`) and before the frame is cut:

```rust
        self.frames += 1;
        // rpicam-vid writes 0 in every row's padding, luma and chroma (measured on
        // both ISPs). Anything else there means the frame is not at this stride.
        let (width, stride, luma) = (self.width, self.stride, self.stride * self.height);
        let mut padding = self.bytes[..luma]
            .chunks_exact(stride)
            .map(|row| &row[width..])
            .chain(
                self.bytes[luma..]
                    .chunks_exact(stride / 2)
                    .map(|row| &row[width / 2..]),
            );
        if padding.any(|bytes| bytes.iter().any(|&b| b != 0)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "frame {} does not fit a {stride}-byte row stride at width {width}: \
                     its row padding is not 0",
                    self.frames
                ),
            ));
        }
```

`new`'s checks, its error and the unit test in the file are unchanged.

### `crates/scope-node/src/rpicam/listing.rs` (US1)

After the imports, which gain `use super::yuv::Isp;`:

```rust
fn is_camera(line: &str) -> bool {
    line.split_once(" : ")
        .is_some_and(|(index, _)| index.parse::<usize>().is_ok())
}

/// The ISP behind the listing's first camera. The listing's `PipelineHandler`
/// names it; a listing without one, from an rpicam-apps that did not print it,
/// is placed by the sensor's device path, which on a Pi 5 runs through its RP1.
pub fn isp(listing: &str) -> Result<Isp, CameraError> {
    let mut lines = listing.lines().map(str::trim).skip_while(|l| !is_camera(l));
    let camera = lines
        .next()
        .ok_or_else(|| CameraError("camera listing contains no cameras or modes".into()))?;
    for line in lines.take_while(|l| !is_camera(l)) {
        if let Some(("PipelineHandler", handler)) = line.split_once(" : ") {
            return match handler.trim() {
                "rpi/vc4" => Ok(Isp::Vc4),
                "rpi/pisp" => Ok(Isp::Pisp),
                other => Err(CameraError(format!(
                    "camera listing names the pipeline handler {other}, whose row stride the node does not know"
                ))),
            };
        }
    }
    Ok(if camera.contains("/rp1/") {
        Isp::Pisp
    } else {
        Isp::Vc4
    })
}
```

`parse` and its unit test are unchanged.

### `crates/scope-node/src/rpicam/mod.rs` (US1)

- The import `use yuv::YuvReader;` becomes `use yuv::{Isp, YuvReader};`.
- `RpicamCamera` gains a field `isp: Isp`, which `new` sets to `Isp::Vc4`.
- After `new`:

  ```rust
      /// Frame at the row stride `isp` writes; `new` takes the VC4's.
      pub fn with_isp(mut self, isp: Isp) -> Self {
          self.isp = isp;
          self
      }
  ```

- `discover` (`crates/scope-node/src/rpicam/mod.rs:61`), after `listing::parse`, adds
  `let isp = listing::isp(&listing)?;`, and its last line, `Self::new(info)`, becomes
  `Ok(Self::new(info)?.with_isp(isp))`.
- `start` (`crates/scope-node/src/rpicam/mod.rs:201`) builds the reader with
  `self.isp.stride(mode.width)` in place of `yuv::stride(mode.width)`.

Nothing else in the file changes: the subprocess, its arguments, `failure`, `apply` and
`restart` are as they are.

## Fixtures

Under owner decision 2 (a), the operator stages, before the flip and in a fixtures PR
of its own, as #129 staged 013's, `fixtures/scope-node/pi5/`:

- `list-cameras.txt`: `rpicam-hello --list-cameras -v` on the Pi 5, as captured;
- `stream.yuv`: `rpicam-vid --codec yuv420`, three 1296x972 frames at stride 1408,
  6,158,592 B, as captured with § capture arguments of `fixtures/scope-node/README.md`;
- `stream.json`: that stream's metadata, as captured;
- `fixtures.json`:

  ```json
  {
    "pipeline_handler": "rpi/pisp",
    "stride_alignment": 128,
    "measured_strides": [
      { "width": 640, "stride": 640 },
      { "width": 1296, "stride": 1408 },
      { "width": 1920, "stride": 1920 },
      { "width": 2592, "stride": 2688 },
      { "width": 1000, "stride": 1024 },
      { "width": 1040, "stride": 1152 }
    ],
    "stream": {
      "file": "stream.yuv",
      "metadata": "stream.json",
      "format": "yuv420",
      "width": 1296,
      "height": 972,
      "stride": 1408,
      "frames": 3,
      "frame_bytes": 2052864,
      "padding_byte_values": [0],
      "sensor_timestamps_ns": [437537643000, 437722812000, 437907979000],
      "exposure_times_us": [19956, 19956, 19956]
    },
    "listing": {
      "file": "list-cameras.txt"
    }
  }
  ```

and, in `fixtures/scope-node/README.md` § The row stride (`fixtures/scope-node/README.md:71`),
after its two bullets, a paragraph: on the Pi 5 (PiSP, libcamera v0.8.0+rpt20261104,
rpicam-apps v1.15.0) the stride is the width rounded up to a multiple of 128 bytes, with
the six measured values, the padding 0 in luma and chroma rows, and the folder
`pi5/` holding the capture. `stream.yuv` holds NUL bytes, so git shows it as binary and
it counts in no story's diff.

Under (b), the operator stages only `list-cameras.txt` and a `fixtures.json` without
`stream`, and US1-S1, US1-S6 and US1-S9 run a synthetic 1296x972 stream at 1408 with the
Pi 3B+ capture's `stream.json`; under (c), nothing is staged and the tests write the Pi 5
listing from the Pi 3B+'s (§ Decisions).

## What was run and what was read

**Run on 2026-11-21, before the draft**, on the branch `probe/046` cut from `main` at
425b9da0, with the Pi 5 capture staged in the probe's own `fixtures/scope-node/pi5/`
exactly as § Fixtures gives it, three times each with the same outcome unless said:

- the four binary tests (US1-S1, US1-S2, US1-S5, US1-S9) on `main`'s adapter code:
  US1-S2 passes, and US1-S1, US1-S5 and US1-S9 fail. Over the Pi 5 capture, no raw frame
  came within 10 s and the node wrote nothing to standard error; with an unknown
  handler the node kept running past 5 s; over a capture under the other Pi's listing,
  no error line came in 3 s. `rpicam_stride.rs` does not build on `main`: `Isp`,
  `listing::isp` and `with_isp` do not exist;
- the trial fix as § Interfaces gives it: all nine scenarios' tests pass, and
  `crates/scope-node/tests/rpicam.rs` passes unchanged;
- with the fix, both gates' Rust commands: `lint`'s four and `test`'s two, every
  existing test of the workspace passing unchanged (run twice: at the trial fix, and at
  the probe's last commit, after US1-S8's dark stream was added);
- a mutation, once: without the chroma half of the padding check, US1-S8's dark stream
  is not refused and the frame never comes;
- as exploratory cases, each once and not committed: `main`'s adapter over the Pi 5
  capture with a fake that writes the metadata beside the pixels gave three frames with
  only row 0 right, then `rpicam-vid: early eof`; and the probe's trial B (§ Decisions),
  under which the two captures and every synthetic mode framed right on either listing,
  a 1472 stream failed with `frame 1 fits none of the row strides [1408, 1344] at width
  1296`, and the cross-listing tests failed by design, the stream overriding the
  listing;
- the Pi 5 capture's bytes: 6,158,592 B, three frames of 2,052,864 B; every luma
  padding byte and every chroma padding byte 0; the Pi 3B+ capture the same at 1344.

**Measured by the seat on the Pi 5, before the probe** (the probe's brief): the strides
of 640, 1296, 1920 and 2592, and of the scaled 1000 and 1040, from the byte counts of
3 or 6 frames each; the padding 0; rpicam's `-v 2` log printing no stride; and the
owner's report that 640x480 gives frames and 1296x972 none.

**Read, not run**:
- rpicam-vid writes a frame's pixels before its metadata
  (`crates/scope-node/src/rpicam/metadata.rs:99`, 013's finding), and the fake does the
  same; and a pipe holds less than the 93,312 B the node leaves unread of the first frame;
- libcamera's two Raspberry Pi pipeline handlers, `rpi/vc4` for the Pi Zero to the Pi 4
  and `rpi/pisp` for the Pi 5 and its chip's boards; not checked against a source here;
- which rpicam-apps release began printing `PipelineHandler`: v1.13.0 (the Pi 3B+
  listing) does not, v1.15.0 (the Pi 5's) does;
- what the console shows (spec Clarifications), the settings-change reopen, a listing of
  several cameras, and a camera that keeps failing past US1-S9's 3 s (spec Edge Cases);
- that nothing else in the workspace assumes 64, by a search for `stride`, `1344` and a
  64 alignment.

**Not run at all**: a Pi, a camera, a real `rpicam-vid` or `rpicam-hello`, the console,
the page, the page half of the `test` gate (`npm ci` installs packages, and the story
writes nothing the page is built from), and the factory's gate boundary itself.

## Read from the diff, not from a test

- **`yuv::stride` goes through `Isp::Vc4`** (FR-001). US1-S3 holds both answers equal for
  the measured widths; the body is the diff's.
- **`listing::isp` reads only the first camera** (FR-002). Neither saved listing has two
  cameras. *Check:* the loop stops at the next camera's line.
- **`new` frames at the VC4's stride** (FR-003). 013's
  `subprocess_exit_reports_stderr_and_capture_restarts_without_restarting_service`
  builds the camera with `new` over the Pi 3B+ capture and passes unchanged, which holds
  it for that capture.

## Shapes that existing tests pin

Every existing test passes unchanged with the story built (run on the probe). The files
beside the change, and what the story does to what they observe:

- **`crates/scope-node/tests/rpicam.rs`**. `padded_stream_and_metadata_match_saved_oracles_byte_for_byte`
  (`crates/scope-node/tests/rpicam.rs:93`) asserts `yuv::stride(s.width) == s.stride`,
  1344, and reads the Pi 3B+ capture through `YuvReader::new` at that stride: the
  stride is unchanged, and the capture's padding is all 0, so the padding check passes.
  `subprocess_exit_reports_stderr_and_capture_restarts_without_restarting_service`
  (`crates/scope-node/tests/rpicam.rs:129`) builds the camera with `RpicamCamera::new`,
  which keeps the VC4's stride, over the same capture: unchanged.
  `listing_reports_every_saved_sensor_value` (`crates/scope-node/tests/rpicam.rs:69`)
  calls `listing::parse`, which is unchanged.
- **The unit tests in the three files.** `invalid_geometry_and_partial_frames_are_errors`
  (`crates/scope-node/src/rpicam/yuv.rs:64`) fails a reader on 5 bytes before any
  padding is read; `absent_camera_and_malformed_sensor_are_errors` and `mod.rs`'s three
  `choose_mode` tests touch nothing the story changes. Their text is unchanged.
- **The node's other binary tests**, `crates/scope-node/tests/cli.rs`,
  `crates/scope-node/tests/exit_on_signal.rs`, `crates/scope-node/tests/exit_stops_the_mount.rs`
  and `crates/scope-node/tests/mount_link.rs`, run the `simulated` adapter or exit before
  discovery (`--check-config` and config refusals): none reaches `discover`.
- **`crates/scope-node/tests/detect.rs`** reads `fixtures/scope-node/finder-frame.png`,
  which is not touched.
- **The simulator and the finder** never build an `RpicamCamera`.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| The listing names the ISP | (b) the first frame's padding picks the stride; (c) a config key | Owner decision 1. (b), built as the probe's trial B (about 5,000 B of diff over (a)), frames by the data and carries bytes across frames; (c) is a key the operator must set per Pi. |
| `PipelineHandler` first, the `/rp1/` path without it | The path alone; the handler alone | The handler is the ISP's own name; the path keeps a Pi 5 on an older rpicam-apps right, and the Pi 3B+ listing has no handler line. |
| An unknown handler is an error | Fall back to 64 | A guess is today's silent stall on a platform the node does not know. |
| Check every frame's padding, luma and chroma | No check; the first frame only; luma only | Without it a stream at another stride stalls silently (spec owner decision 1 (d)). Every frame costs one pass over the padding and keeps the reader stateless. A dark frame shows a misfit only in its chroma rows (US1-S8, run as a mutation). |
| `Isp` beside `yuv::stride`, which keeps its 64 | Change `stride`'s signature | `crates/scope-node/tests/rpicam.rs:96` calls it, and must pass unchanged. |
| `with_isp` beside `new` | A new constructor taking the listing | `new` keeps its signature for the existing tests; `discover` is the only caller that reads a listing. |
| The binary's tests over fake camera programs | In-process tests only | Only the binary runs `discover`, which takes the ISP from the listing; the raw-frame call reads the node's frames byte for byte. |
| A fake that writes pixels, then metadata | The existing test's fake, which writes them side by side | Only the real order shows the stall the owner saw; the side-by-side fake gives misframed frames instead. |
| One story | Two | About 41.5 % of 65,536 B as probed (§ Plan traps 2). |

Option (c) of owner decision 2, the tests writing the Pi 5 listing from the Pi 3B+'s,
would test a listing no camera printed; (b) keeps the printed listing and loses the real
frames.

## User Story 1 - The node frames the Pi 5's camera at its row stride

Tests first: `rpicam_node.rs` and `rpicam_stride.rs`. Then `Isp` and the padding check
in `yuv.rs`; `listing::isp`; `with_isp`, `discover` and `start` in `mod.rs`.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 425b9da0):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  The story runs both, and the judge sees their results.
- **CI in the merge queue** runs every required `ci.yml` job.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** every spec in `depends_on_landed`. Met at 425b9da0.
2. **`main` is still as read.** Every citation here was read at 425b9da0 on 2026-11-21.
   If `main` moves in a file § Project Structure names, or in a test § Shapes names,
   re-read them and re-run the validator.
3. **The Pi 5 capture is on `main`** under owner decision 2 (a): `fixtures/scope-node/pi5/`
   and the README paragraph of § Fixtures, staged by the operator in a PR of its own.
   Not met. Under (b), the listing and its `fixtures.json` only; under (c), nothing.
4. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Decisions met 2026-11-21: the owner took both
   recommended answers ("Yes, both"); the approvals come with the flip.
5. **Probe builds.** Not met: the trio has not been built from its own text.

## Outside the factory

Written here and not run.

**The field check**, by the owner, on the Pi 5 finder, once 046 has landed:

1. Build a new Pi kit from the landing commit, `pi5-rig/build-pi.sh <landing sha>`, and
   install its node binary on the Pi: the node's binary changes, and the Pi's current
   one stalls at 1296x972.
2. Set the node's config back from `mode = "640x480"` to `mode = "1296x972"`, and restart
   the node.
3. Open the console's Setup: it should show a live preview within a few seconds.
4. Ask the console's `get_session`: its frame age should read under 2 s.
5. Read the node's journal: no `camera capture failed:` line should follow the start.

A `does not fit a ...-byte row stride` line in the journal means the Pi writes a stride
neither rule gives: note the width and the line, set the mode back to 640x480, and
report them. A `pipeline handler` line at start means the listing names a handler the
node does not know: report the listing.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* the story's diff stays under 64 KiB, measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. Measured on the
   drafting probe's commits, with the fixture staged apart: 27,204 B, 41.5 % of 65,536 B
   (the code 6,905 B, `rpicam_node.rs` 9,359 B, `rpicam_stride.rs` 10,940 B). At the
   1.35 times a factory build has run over its probe on earlier specs, about 36,700 B,
   56.0 %. Were the story to carry the fixture's text files too, 32,677 B, 49.9 %.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/046-node-reads-pi-5-frames/` or `.specify/`.
4. **The fixture is the operator's.** *Check:* nothing under `fixtures/` is in the diff.
5. **The fake must write pixels before metadata.** *Check:* the fake's script writes each
   frame's bytes, then its metadata object, in one sequence; a fake that writes them side
   by side cannot show the stall US1-S1 fails with on `main`.
6. **An oracle copied from the code proves nothing.** *Check:* the tests write every
   expected stride and message as a literal or read it from a fixture; none calls `Isp`
   or `yuv::stride` for an expected value, or formats an error to get one.
7. **A negative assertion passes when nothing happens.** *Check:* US1-S9 asserts a line
   that is there, not the absence of frames; US1-S5 asserts the status and the line.
8. **The scratch directories must not collide.** *Check:* each fake camera's directory
   under `CARGO_TARGET_TMPDIR` names its test case and the process id.
9. **An unused helper fails clippy.** *Check:* each test file holds only the helpers its
   own tests use.
10. **rustdoc denies warnings.** *Check:* no doc comment holds an angle bracket outside
    backticks.
11. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
12. **The lockfile does not move.** *Check:* every cargo command runs with `--locked`.
13. **Signals only to the test's own child.** *Check:* the binary's tests end the node
    with its own `Child`'s kill.

## Risks

| Risk | Mitigation |
|---|---|
| A real Pi 5 writes a stride the probe did not see at some mode. | The padding check refuses it with the stride named (US1-S8, US1-S9); the field check reads the journal. |
| A future camera stack writes non-zero padding, and every frame is refused. | Owner approval 2 says so; the error names the stride and the width. |
| A loaded host stretches the binary's 10 s, 5 s and 3 s waits. | Each is a bound on a wait that ends as soon as the node answers; none is a duration the test asserts. |
| Owner decision 2 lands as (b) or (c) after the draft. | Only US1-S1, US1-S6 and US1-S9's rigs change (§ Fixtures). |

## Refinement Record

Drafted on 2026-11-21 by a plate-solver spec session against `main` at 425b9da0, from a
probe of the fault and a trial fix that day, a reading of the node's rpicam adapter, its
tests and fixtures, 013's trio and the constitution. Writes come from the task text.
Shared writes name the merge edge that orders them, and the story that last wrote the
same file on `main`. `crates/scope-node/src/rpicam/mod.rs` was last changed by #194, not
a spec's story, and has no entry.

The owner took both of spec.md's recommended answers on 2026-11-21. The record's
`open_questions` are empty. This record approves nothing: the
owner approvals of spec.md are given by the flip.

```yaml
version: 1
reviewed_revision: 425b9da079f93237368fe6b39c733a33f8b0dc20
stories:
  US1:
    tests:
      - path: crates/scope-node/tests/rpicam_node.rs
        status: new
        disposition: add
        assertion: "the built node over a fake rpicam-hello printing the Pi 5 listing and a fake rpicam-vid writing each Pi 5 capture frame's pixels then its metadata, at 1296x972: its first raw frame within 10 s is 1296 by 972 and equals the rows at stride 1408 of one of the capture's three frames"
        decision: "spec.md:US1-S1"
      - path: crates/scope-node/tests/rpicam_node.rs
        status: new
        disposition: add
        assertion: "the built node over the fake camera replaying the Pi 3B+ capture with its own metadata and printing the Pi 3B+ listing: its first raw frame within 10 s is 1296 by 972 and equals the rows at stride 1344 of one of the capture's frames"
        decision: "spec.md:US1-S2"
      - path: crates/scope-node/tests/rpicam_stride.rs
        status: new
        disposition: add
        assertion: "Isp::Pisp.stride gives each Pi 5 measured stride from the capture's fixtures.json, 640 1408 1920 2688 1024 1152, and Isp::Vc4.stride and yuv::stride each give the Pi 3B+ README's 640 1344 1920 1024 1088 for 640 1296 1920 1000 1040"
        decision: "spec.md:US1-S3"
      - path: crates/scope-node/tests/rpicam_stride.rs
        status: new
        disposition: add
        assertion: "listing::isp answers Pisp for the Pi 5 listing, Vc4 for the Pi 3B+ listing, Pisp for the Pi 5 listing without its PipelineHandler line, Vc4 for it naming rpi/vc4, and for rpi/other fails with camera listing names the pipeline handler rpi/other, whose row stride the node does not know"
        decision: "spec.md:US1-S4"
      - path: crates/scope-node/tests/rpicam_node.rs
        status: new
        disposition: add
        assertion: "the built node whose fake rpicam-hello prints the Pi 5 listing naming rpi/other exits within 5 s with status 1, its last stderr line scope-node: camera listing names the pipeline handler rpi/other, whose row stride the node does not know"
        decision: "spec.md:US1-S5"
      - path: crates/scope-node/tests/rpicam_stride.rs
        status: new
        disposition: add
        assertion: "the adapter on the Pi 5 listing at 1296x972 over the fake camera replaying the Pi 5 capture: three frames equal the capture's rows at stride 1408 in order, start_s and end_s exactly from SensorTimestamp and ExposureTime, and the fourth fails holding fake-camera-exit"
        decision: "spec.md:US1-S6"
      - path: crates/scope-node/tests/rpicam_stride.rs
        status: new
        disposition: add
        assertion: "synthetic streams at 640x480, 1296x972, 1920x1080 and 2592x1944: on the Pi 5 listing at strides 640 1408 1920 2688, on the Pi 3B+ listing at 640 1344 1920 2624, three frames each of the stream's size equal its gray images in order, and the fourth fails holding fake-camera-exit"
        decision: "spec.md:US1-S7"
      - path: crates/scope-node/tests/rpicam_stride.rs
        status: new
        disposition: add
        assertion: "at 1296x972, within 5 s: the Pi 5 listing over a stream at stride 1472 fails with frame 1 does not fit a 1408-byte row stride at width 1296: its row padding is not 0; the Pi 3B+ listing over a stream at 1408, and over a dark one at 1408, each fail with the same words for a 1344-byte row stride"
        decision: "spec.md:US1-S8"
      - path: crates/scope-node/tests/rpicam_node.rs
        status: new
        disposition: add
        assertion: "the built node over the Pi 5 capture with the Pi 3B+ listing, 3 s after it accepts a connection, has a stderr line starting camera capture failed: rpicam-vid: frame 1 does not fit a 1344-byte row stride at width 1296: its row padding is not 0; over the Pi 3B+ capture with the Pi 5 listing, the same for a 1408-byte row stride"
        decision: "spec.md:US1-S9"
      - path: crates/scope-node/tests/rpicam.rs
        status: existing
        disposition: preserve
        assertion: "yuv::stride of the Pi 3B+ capture's width is its 1344; its frames read byte for byte at that stride with their times; a camera built with new over it reports the fake's exit and the capture loop restarts it"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/cli.rs
        status: existing
        disposition: preserve
        assertion: "check-config prints every setting with the token masked, and bad keys are refused naming the key"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/detect.rs
        status: existing
        disposition: preserve
        assertion: "the node's and the console's detection agree on the saved finder frame"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "yuv::stride keeps its signature and its 64-byte rule (crates/scope-node/tests/rpicam.rs:96)"
      - "RpicamCamera::new, with_path, listing::parse, YuvReader::new and CameraInfo keep their shapes"
      - "a stream that ends inside a frame fails with early eof, and the capture loop restarts the camera after 1 s, as 013 FR-005 says"
      - "the scope link's protocol, the config's keys, the console, the mount and imaging are unchanged"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test rpicam_node"
        - "cargo nextest run -p scope-node --locked --test rpicam_stride"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, a new Pi kit and the field check of plan.md § Outside the factory"
    writes:
      - crates/scope-node/src/rpicam/listing.rs
      - crates/scope-node/src/rpicam/mod.rs
      - crates/scope-node/src/rpicam/yuv.rs
      - crates/scope-node/tests/rpicam_node.rs
      - crates/scope-node/tests/rpicam_stride.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/src/rpicam/yuv.rs
        other_spec: 013-live-finder
        other_story: US2
        disposition: merge_order
        reason: "046 depends_on_landed 013, whose US2 last changed yuv.rs (ee280ae7)"
      - path: crates/scope-node/src/rpicam/listing.rs
        other_spec: 013-live-finder
        other_story: US2
        disposition: merge_order
        reason: "046 depends_on_landed 013, whose US2 last changed listing.rs (ee280ae7)"
    open_questions: []
```
