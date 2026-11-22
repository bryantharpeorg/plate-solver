---
state: draft
depends_on_landed: ["013-live-finder", "044-node-stops-mount-before-exit"]
# Drafted 2026-11-21 against `main` at 425b9da0 (425b9da079f93237368fe6b39c733a33f8b0dc20),
# at the owner's ask the same day ("Yes, draft it"), from a probe of that day on the
# branch `probe/046` cut from 425b9da0: tests that show the fault, run on `main`'s code,
# and a trial fix with which they pass, with both gates' Rust commands passing. Every
# `path:line` here was read at 425b9da0. The owner took both recommended answers on
# 2026-11-21 ("Yes, both"), and the seat may flip it once probe round 1 is folded. The
# trio has not yet been built from its own text (plan.md § Prerequisites). 044 is named
# because the story's binary tests sit beside 044's and run the node 044 last changed.
# 027's test layout is not named: no story writes a finder test. What rests on a run and
# what on reading is said where it is used (plan.md § What was run and what was read).
---

# Feature Specification: The Node Reads the Pi 5's Camera Frames

**Feature Branch**: `046-node-reads-pi-5-frames`

**Created**: 2026-11-21

**Status**: Draft

**Input**: The finder moved to a Raspberry Pi 5. At the node's configured mode,
1296x972, the console connects but never gets a frame; at 640x480 frames flow. On the Pi
5, `rpicam-vid`'s YUV420 rows are padded to a multiple of 128 bytes, and the node frames
them at 64, the Pi 3B+'s rule. Make the node learn the row stride from what the camera
stack tells it, keep the Pi 3B+ as it is, and refuse loudly, naming the cause, a stream
it cannot frame.

## Clarifications

### Session 2026-11-21 (the probe on `main` at 425b9da0)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it. The probe's branch
  and notes are not in the repository; what they showed is written here.
- Q: What is the row stride today? → A: `yuv::stride`
  (`crates/scope-node/src/rpicam/yuv.rs:5`) rounds the width up to a multiple of 64
  bytes, and `start` frames rpicam-vid's standard output with it
  (`crates/scope-node/src/rpicam/mod.rs:201`). `fixtures/scope-node/README.md:71`
  records 64 as measured on the Pi 3B+, whose ISP is the VC4: 640 → 640, 1296 → 1344,
  1920 → 1920, 1000 → 1024 and 1040 → 1088.
- Q: What does the Pi 5 write? → A: The seat measured the standard output of
  `rpicam-vid --codec yuv420` on the Pi 5 (PiSP ISP, libcamera v0.8.0+rpt20261104,
  rpicam-apps v1.15.0), 3 or 6 frames each: the row stride is the width rounded up to a
  multiple of 128 bytes. 640 → 640, 1296 → 1408, 1920 → 1920, 2592 → 2688, and the
  scaled sizes 1000 → 1024 and 1040 → 1152. The probe read the saved Pi 5 capture's
  three 1296x972 frames: 2,052,864 B each (1408 × 972 × 3/2), with every luma row's
  padding (bytes 1296 to 1407) and every chroma row's (648 to 703) 0. The Pi 3B+ capture
  reads the same way at 1344: all padding 0. rpicam's `-v 2` log does not print the
  stride.
- Q: What does the node do today on the Pi 5? → A: It stalls, silently. The probe ran
  the built node over a fake `rpicam-vid` that writes each frame's pixels and then that
  frame's metadata, the order rpicam-vid writes them by 013's reading
  (`crates/scope-node/src/rpicam/metadata.rs:99`), replaying the Pi 5 capture: no raw
  frame came within 10 s, and the node wrote nothing to standard error (three runs). By
  reading: the node asks for 1,959,552 B a frame where rpicam-vid writes 2,052,864, so
  after the node's first frame rpicam-vid still has 93,312 B of it to write, more than
  a pipe holds by default, and blocks before it writes that frame's metadata; the node
  waits for that metadata for good. No error is raised, so the capture loop's log line
  and restart (`crates/scope-node/src/capture.rs:69`) never come. With a fake that
  writes the metadata beside the pixels instead, as `crates/scope-node/tests/rpicam.rs`
  does, the node gave three misframed frames, the first matching the capture in its row
  0 only, then `rpicam-vid: early eof` (one exploratory run).
- Q: What does the console show? → A: By reading, not run: no preview. Its live preview
  draws nothing until a frame's preview arrives
  (`crates/finder/web/src/components/LivePreview.tsx:32`). Nothing in the console
  changes; with the fix the node sends frames on the Pi 5 as it does on the Pi 3B+.
- Q: Where can the node learn the stride? → A: Three places were weighed (owner decision
  1). The verbose listing the node already reads at start
  (`crates/scope-node/src/rpicam/mod.rs:61`) names the camera's pipeline handler on the
  Pi 5, `PipelineHandler : rpi/pisp`, and its sensor's device path runs through the Pi
  5's RP1 chip, `/base/axi/pcie@1000120000/rp1/...`; the Pi 3B+ listing, from
  rpicam-apps v1.13.0, has no `PipelineHandler` line and a path under `/base/soc/`. The
  stream itself: every padding byte measured is 0, so a frame read at the wrong stride
  shows pixels where its padding should be. A config key: not tried. The probe built
  the first two (plan.md § Decisions).
- Q: What is the fix? → A: Owner decision 1's recommended answer, as the probe built it.
  `yuv::Isp` names the two ISPs and their strides, beside `yuv::stride`, which keeps its
  signature and its 64. `listing::isp` reads the first camera's `PipelineHandler`
  (`rpi/vc4`, `rpi/pisp`, any other an error that names it) or, with none, places it by
  an `/rp1/` device path. `discover` frames at that ISP's stride. `YuvReader` refuses a
  frame whose row padding is not 0, with an error naming the frame, the stride and the
  width, which the adapter and the capture loop already report and recover from.
- Q: Which of 013's requirements does this change? → A: No requirement's text. 013's
  FR-005 reads, in part: "take each frame's Y plane at the row stride the camera
  writes" (`specs/013-live-finder/spec.md:726`). The node does that on the Pi 3B+ and,
  with this spec, on the Pi 5. Its other clauses, the subprocess, the listing and the
  restart after an exit, are as they are.
- Q: What about the 640x480 and 1920x1080 modes? → A: Both widths are multiples of 128,
  so both rules give the width and there is no padding: they work on both Pis today,
  which matches the owner's 640x480 report. 2592x1944 is padded under both rules: 2688
  on the Pi 5 (measured), 2624 on the Pi 3B+ (the 64 rule; not measured there, where the
  mode failed to start in 7 of 15 tries, `fixtures/scope-node/README.md`). US1-S7 runs all
  four on both rules with synthetic streams.
- Q: Does anything else assume 64? → A: No, by a search of the workspace at 425b9da0
  for `stride`, `1344` and a 64 alignment: only `yuv.rs`, its caller
  (`crates/scope-node/src/rpicam/mod.rs:201`), the existing test that pins
  `yuv::stride` (`crates/scope-node/tests/rpicam.rs:96`) and the fixture README. The console, the simulator, the preview and the raw-frame service take frames
  already unpadded.
- Q: Is the imaging camera touched? → A: No. INDI's frames are FITS from the driver and
  never pass through `crates/scope-node/src/rpicam/`.
- Q: What rests on a run, and what on reading? → A: Every scenario was run on the probe,
  with the trial fix of plan.md § Interfaces, three times each with the same outcome;
  on `main`'s code US1-S1, US1-S5 and US1-S9 fail and US1-S2 passes, three times each. plan.md § What was run
  and what was read lists each claim. Nothing was run on a Pi, a camera or a real
  `rpicam-vid`.
- Q: What is left for later? → A: The items under "Out of scope" in § Assumptions.

## Owner decisions

The owner took each recommended answer on 2026-11-21 ("Yes, both"), and the scenarios
name them. The alternatives are kept below for the record.

1. **How the node learns the stride** (FR-002, FR-003, FR-004). *Settled 2026-11-21: the recommended answer.* Recommended:
   (a) the listing names the ISP, and every frame's padding is checked. The node reads
   `PipelineHandler` from the listing it already parses at start (`rpi/vc4` → 64,
   `rpi/pisp` → 128); a listing without that line, from an rpicam-apps that does not
   print it, is placed by the sensor's device path (`/rp1/` → 128, otherwise 64); any
   other handler stops the node at start with an error naming it. Every frame whose
   padding is not 0 is refused with an error naming the frame, the stride and the width.
   Weighed against:
   - **Platforms the owner may add.** libcamera's Raspberry Pi support has two pipeline
     handlers, `rpi/vc4` (the Pi Zero to the Pi 4) and `rpi/pisp` (the Pi 5 and the
     boards built on its chip), by reading, not checked against a source here; a Pi of
     either kind needs no change, and a platform with another handler fails at start
     with its name until a line is added. A future equatorial rig changes the mount and
     the imaging camera, neither of which is here; its finder would run this same path.
   - **Testability on the factory's x86 gates.** Every rule is a pure function of saved
     text and saved or synthetic bytes, and the built node runs over fake camera
     programs (US1's rigs).
   - **Loud versus silent.** An unknown handler fails at start; a stream at a stride
     other than the rule's fails on its first frame with its stride named (US1-S8,
     US1-S9), where today it stalls with nothing logged. It can still pass silently in
     one case: a width that is a multiple of the true alignment and of the rule's, such
     as 640 or 1920, has no padding to check, and a platform whose alignment exceeded
     that width's would stall as today. Neither Pi has one.
   - Instead: (b) the stream picks the stride: the first frame's padding chooses
     between 128 and 64, the larger first (the probe's trial B). It needs no listing
     and frames even a listing that misnames the ISP, but it can choose only among the
     strides it is given, needs the padding to be 0 to choose at all, and carries bytes
     read ahead into the next frame: the probe's trial B added about 60 lines to (a)'s
     reader. A frame whose pixels are 0 wherever the wrong stride's padding falls would
     be framed wrongly (not seen: both captures' chroma reads 106 to 159). (c) A config key for the
     alignment, as an override beside (a) or alone: a new key, which the operator must
     set per Pi; alone, a missing or wrong value misframes as today. (d) (a) without the
     padding check: a stream at another stride stalls silently, as today.
2. **The Pi 5 capture as a fixture** (US1-S1, US1-S6, US1-S9). *Settled 2026-11-21: the recommended answer.* Recommended: (a)
   the operator stages it before the flip, in `fixtures/scope-node/pi5/`, in a fixtures
   PR of its own, as #129 staged 013's: `list-cameras.txt`, `stream.yuv` (three
   1296x972 frames at stride 1408, 6,158,592 B), `stream.json`, a `fixtures.json` that
   records what they hold (plan.md § Fixtures), and a Pi 5 paragraph in
   `fixtures/scope-node/README.md` § The row stride. The repository is public; the
   frames show a blurry indoor corner. A story cannot carry the stream: no builder has
   the bytes, and every fixture is read-only to a story. The real capture is the one
   oracle no code wrote: real chroma, real padding, real metadata. Instead: (b) no
   capture: the operator stages only the Pi 5 listing, as `fixtures/scope-node/pi5/
   list-cameras.txt` with a `fixtures.json` holding the measured strides, and US1-S1,
   US1-S6 and US1-S9 run synthetic streams at 1296x972 with the Pi 3B+ capture's
   metadata; or (c) nothing staged, the tests writing the Pi 5 listing from the Pi
   3B+'s with its device path and `PipelineHandler` line changed, a listing no camera
   printed.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden, fixture or CI
file changes.

1. **New public API in the node's library**: `scope_node::rpicam::yuv::Isp` and its
   `stride`, `scope_node::rpicam::listing::isp`, and `RpicamCamera::with_isp` (FR-001 to
   FR-003). No existing signature changes.
2. **The node refuses what it framed silently before** (FR-002, FR-004): a camera
   listing naming a pipeline handler other than `rpi/vc4` and `rpi/pisp` stops it at
   start, and a frame whose row padding is not 0 is an error the capture loop logs and
   restarts after. Neither happens with either saved capture.
3. **The operator-staged Pi 5 capture** in `fixtures/scope-node/pi5/` and its README
   paragraph, under owner decision 2 (plan.md § Prerequisites 3).

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate command.** Every scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command.
- **The Pi 5 capture** is `fixtures/scope-node/pi5/` as staged (owner decision 2): its
  `fixtures.json`'s `stream` (1296x972, stride 1408, 3 frames of 2,052,864 B, the
  metadata file, each frame's `SensorTimestamp` and `ExposureTime`), its
  `measured_strides` and its listing. **The Pi 3B+ capture** is `fixtures/scope-node/`
  and its `fixtures.json`, as on `main`. **The Pi 5 listing** and **the Pi 3B+ listing**
  are those captures' `list-cameras.txt`, read whole.
- **A frame's rows** are a saved frame's first height rows of stride bytes, each cut to
  its first width bytes, joined in order.
- **A synthetic stream** at a width w, height h and stride s is three frames; frame k
  (0, 1, 2) is h rows of s bytes, whose byte c of row r is 1 + (31r + 7c + 13k) mod 250
  for c below w and 0 otherwise, then h rows of s/2 bytes, 128 for c below w/2 and 0
  otherwise. **Its gray image** is the first w bytes of each of its first h rows.
  **Dark** means every luma byte below w is 0 instead.
- **The fake camera** is a directory under `CARGO_TARGET_TMPDIR` holding an executable
  `rpicam-vid` written by the test, which takes its `--metadata` path from its
  arguments, opens it, and then, for each frame of a stream in order, writes the frame's
  bytes to standard output and then that frame's metadata object to the path, `[`
  before the first and `,` before each other, then `]`; then it writes
  `fake-camera-exit` to standard error and exits 7. The metadata objects are the Pi 5
  capture's `stream.json`, in order, unless a scenario names the Pi 3B+ capture's.
  For the node rig the directory also holds an executable `rpicam-hello` that prints a
  listing the scenario names and exits 0.
- **The node rig** is the built `scope-node`, `env!("CARGO_BIN_EXE_scope-node")`, started
  with `--config` naming a file under `CARGO_TARGET_TMPDIR` that sets `listen` to a free
  `127.0.0.1` port, `token` to `'test-token'`, `adapter` to `'rpicam'` and `mode` to
  `'1296x972'`; with the fake camera's directory first on the child's `PATH`, standard
  input and output null, standard error piped and `kill_on_drop`. It is waited on until a
  TCP connection to its port succeeds, for at most 10 s. **Its first raw frame** is
  `raw_frame` on a `ScopeNodeRawClient` over a tonic `Channel` to its address with the
  `BearerToken` `test-token`, awaited for at most 10 s
  (`crates/scope-link/proto/scope_node/v1/scope_node.proto:151`). **Its standard error**
  is read after the test kills it, or after it exits.
- **The adapter rig** is `listing::parse` of a listing, its first camera with
  `active_mode` set to the listed mode of the scenario's size, then
  `RpicamCamera::new(info)`, `.with_isp(listing::isp(listing))` and `.with_path` of a
  `PATH` that puts the fake camera's directory first, as
  `crates/scope-node/tests/rpicam.rs:129` builds its camera. **Taking a frame** is
  `next_frame` awaited.
- **Oracles.** The saved captures' own bytes and the strides their `fixtures.json` and
  `fixtures/scope-node/README.md` record, the synthetic streams' bytes the test sets,
  the process's exit status and standard error, and the words in these scenarios. No
  expected value comes from the code under test: no test computes an expected stride
  with `Isp` or `yuv::stride`, or an expected message from the code.

### User Story 1 - The node frames the Pi 5's camera at its row stride (Priority: P1)

As the observer, I start the finder on the Pi 5 at 1296x972, and the console's Setup
shows a live preview, as it did on the Pi 3B+; and if the node ever meets a camera it
cannot frame, its log says so.

**Why this priority**: On the Pi 5 the finder gives no frame at its configured mode, and
nothing says why.

**Independent Test**: `crates/scope-node/tests/rpicam_node.rs` (US1-S1, US1-S2, US1-S5,
US1-S9) and `crates/scope-node/tests/rpicam_stride.rs` (US1-S3, US1-S4, US1-S6 to
US1-S8), both new. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs them; during
development, `cargo nextest run -p scope-node --locked --test rpicam_node` and
`cargo nextest run -p scope-node --locked --test rpicam_stride`.

**Acceptance Scenarios**:

1. **Given** the node rig over the fake camera replaying the Pi 5 capture, its
   `rpicam-hello` printing the Pi 5 listing, **When** a committed test takes its first
   raw frame, **Then** it asserts that the reply is 1296 wide and 972 high and that its
   pixels equal the rows, at stride 1408, of one of the capture's three frames.
2. **Given** the node rig over the fake camera replaying the Pi 3B+ capture with its own
   `stream.json`, its `rpicam-hello` printing the Pi 3B+ listing, **When** a committed
   test takes its first raw frame, **Then** it asserts that the reply is 1296 wide and
   972 high and that its pixels equal the rows, at stride 1344, of one of the capture's
   three frames.
3. **Given** the Pi 5 capture's `measured_strides` and the Pi 3B+ strides of
   `fixtures/scope-node/README.md` § The row stride, written in the test as 640 → 640,
   1296 → 1344, 1920 → 1920, 1000 → 1024 and 1040 → 1088, **When** a committed test
   asks `Isp::Pisp.stride` for each Pi 5 width and `Isp::Vc4.stride` and `yuv::stride`
   for each Pi 3B+ width, **Then** it asserts that every answer equals the measured
   stride: 640, 1408, 1920, 2688, 1024 and 1152 on the Pi 5.
4. **Given** five listings: the Pi 5 listing; the Pi 3B+ listing; the Pi 5 listing with
   every line holding `PipelineHandler` removed; the Pi 5 listing with
   `PipelineHandler : rpi/pisp` replaced by `PipelineHandler : rpi/vc4`; and the Pi 5
   listing with it replaced by `PipelineHandler : rpi/other`, **When** a committed test
   calls `listing::isp` on each, **Then** it asserts that the first four answer
   `Isp::Pisp`, `Isp::Vc4`, `Isp::Pisp` and `Isp::Vc4`, and that the fifth fails with
   the message
   `camera listing names the pipeline handler rpi/other, whose row stride the node does not know`.
5. **Given** the node rig over the fake camera replaying the Pi 5 capture, its
   `rpicam-hello` printing the Pi 5 listing with `PipelineHandler : rpi/pisp` replaced by
   `PipelineHandler : rpi/other`, **When** a committed test awaits the node's exit for at
   most 5 s, **Then** it asserts that the node exits within it with status 1 and that the
   last line of its standard error is
   `scope-node: camera listing names the pipeline handler rpi/other, whose row stride the node does not know`.
6. **Given** the adapter rig on the Pi 5 listing at 1296x972, over the fake camera
   replaying the Pi 5 capture, **When** a committed test takes four frames, **Then** it
   asserts that the first three equal the rows of the capture's frames in order, at
   stride 1408; that each one's `start_s` equals its `SensorTimestamp` / 1e9 and its
   `end_s` that plus its `ExposureTime` / 1e6, exactly; and that the fourth fails with an
   error holding `fake-camera-exit`.
7. **Given** the adapter rig on the Pi 5 listing over synthetic streams at 640x480,
   1296x972, 1920x1080 and 2592x1944 with strides 640, 1408, 1920 and 2688, and the
   adapter rig on the Pi 3B+ listing over synthetic streams at the same sizes with
   strides 640, 1344, 1920 and 2624, each stream in a fake camera of its own, **When** a
   committed test takes four frames from each, **Then** it asserts that each of the
   first three is the stream's size and equals its frames' gray images in order, and
   that the fourth fails with an error holding `fake-camera-exit`.
8. **Given** three adapter rigs at 1296x972, each over a fake camera of its own: on the
   Pi 5 listing over a synthetic stream at stride 1472; on the Pi 3B+ listing over a
   synthetic stream at stride 1408; and on the Pi 3B+ listing over a dark synthetic
   stream at stride 1408, **When** a committed test takes a frame from each, awaited for
   at most 5 s, **Then** it asserts that each fails within it, the first with an error
   holding
   `frame 1 does not fit a 1408-byte row stride at width 1296: its row padding is not 0`
   and the other two with an error holding
   `frame 1 does not fit a 1344-byte row stride at width 1296: its row padding is not 0`.
9. **Given** the node rig over the fake camera replaying the Pi 5 capture with its
   `rpicam-hello` printing the Pi 3B+ listing, and a second over the fake camera
   replaying the Pi 3B+ capture with its own `stream.json` and its `rpicam-hello`
   printing the Pi 5 listing, **When** a committed test waits 3 s after each accepts a
   connection and then kills it, **Then** it asserts that a line of the first's standard
   error starts
   `camera capture failed: rpicam-vid: frame 1 does not fit a 1344-byte row stride at width 1296: its row padding is not 0`,
   and a line of the second's starts
   `camera capture failed: rpicam-vid: frame 1 does not fit a 1408-byte row stride at width 1296: its row padding is not 0`.

### Edge Cases

- **The 640x480 and 1920x1080 modes** have no padding under either rule, so the padding
  check has nothing to read there and the stride is the width either way (US1-S7).
- **A width a multiple of 64 but not of 128**, such as 1344, has no padding at 64 and 64
  bytes of it at 128; the rule takes the listing's word. No listed mode of either camera
  is such a width.
- **A listing with several cameras.** `listing::isp` reads the first camera's lines, up
  to the next camera's, as `discover` takes the first camera
  (`crates/scope-node/src/rpicam/mod.rs:74`). Read, not run: neither saved listing has
  two cameras.
- **A Pi 5 whose rpicam-apps does not print `PipelineHandler`** is placed by its `/rp1/`
  device path (US1-S4's third listing). Which rpicam-apps release began printing the
  line is not known here: v1.13.0 does not, v1.15.0 does.
- **A frame refused for its padding.** The adapter stops rpicam-vid, as it does after any
  failed read: it waits up to 1 s for it to exit and then kills it
  (`crates/scope-node/src/rpicam/mod.rs:290`), and the capture loop logs
  `camera capture failed: ` and the error and retries after 1 s
  (`crates/scope-node/src/capture.rs:69`). On a camera that keeps failing, the log gains
  a line every few seconds and the console still gets no frame (US1-S9). Read, not run,
  past the 3 s of US1-S9.
- **A stream that ends inside a frame** still fails with `early eof`, as today
  (`crates/scope-node/src/rpicam/yuv.rs:46`); 013's
  `subprocess_exit_reports_stderr_and_capture_restarts_without_restarting_service`
  holds it.
- **A camera whose padding is not 0** would have every frame refused (owner approval 2).
  Neither saved capture has one: all their padding bytes are 0.
- **A settings change** reopens rpicam-vid (`crates/scope-node/src/rpicam/mod.rs:215`),
  and the new process is framed at the same ISP's stride; the frame count of the
  padding error starts again at 1 with each process. Read, not run.

## Requirements *(mandatory)*

### Functional Requirements

**The row stride** (US1):

- **FR-001**: `crates/scope-node/src/rpicam/yuv.rs` SHALL gain a public `Isp` enum with
  `Vc4` and `Pisp`, and `Isp::stride(self, width: u32) -> usize`, which rounds the width
  up to a multiple of 64 bytes for `Vc4` and of 128 for `Pisp`. `yuv::stride(width)`
  SHALL keep its signature and answer what `Isp::Vc4.stride(width)` answers.
- **FR-002**: `crates/scope-node/src/rpicam/listing.rs` SHALL gain
  `pub fn isp(listing: &str) -> Result<Isp, CameraError>`. It SHALL read the listing's
  first camera, from its `<index> : <model> [...]` line to the next camera's line or the
  end: a `PipelineHandler : rpi/vc4` line SHALL answer `Isp::Vc4`, a
  `PipelineHandler : rpi/pisp` line `Isp::Pisp`, and any other `PipelineHandler` value
  SHALL fail with `camera listing names the pipeline handler <value>, whose row stride
  the node does not know`. Without a `PipelineHandler` line it SHALL answer `Isp::Pisp`
  when the camera's line holds `/rp1/`, and `Isp::Vc4` otherwise (owner decision 1).
- **FR-003**: `RpicamCamera` SHALL gain `pub fn with_isp(self, isp: Isp) -> Self`;
  `RpicamCamera::new` SHALL keep its signature and frame at `Isp::Vc4`'s stride.
  `RpicamCamera::discover` SHALL take the ISP from `listing::isp` of the listing it
  already reads and frame rpicam-vid's stream at that ISP's stride; an error from
  `listing::isp` SHALL end discovery, so the node exits 1 with
  `scope-node: <message>` (`crates/scope-node/src/main.rs:46`).
- **FR-004**: `YuvReader::next` SHALL refuse a frame in which any byte of a luma row from
  the width to the stride, or of a chroma row from half the width to half the stride,
  is not 0, with an `io::Error` of kind `InvalidData` and the message
  `frame <n> does not fit a <stride>-byte row stride at width <width>: its row padding is not 0`,
  `<n>` counting a reader's frames from 1 (owner decision 1). `YuvReader::new` SHALL
  keep its signature and its refusals.
- **FR-005**: Nothing else SHALL change: the scope link's protocol and `CameraInfo`, the
  config and its keys, `crates/scope-node/src/main.rs`, `capture.rs`, `service.rs`,
  `rpicam/metadata.rs`, everything under `crates/scope-node/src/imaging/` and
  `crates/scope-node/src/mount/`, every file under `crates/finder`, and every existing
  test and fixture.

### Key Entities

- **The ISP**: the Raspberry Pi image signal processor behind the camera, VC4 or PiSP,
  which sets the row stride of rpicam-vid's YUV420 frames.
- **The row stride**: the bytes from the start of one luma row to the next; a chroma
  row's is half of it.
- **Row padding**: a row's bytes past its pixels, 0 on both ISPs as measured.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, fixture, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited. Every existing
  test passes unchanged (plan.md § Shapes that existing tests pin).
- **SC-003**: Given the Pi 5's capture, the built node sends frames whose pixels are the
  capture's rows (US1-S1), where today it sends none and logs nothing; given the Pi
  3B+'s, it sends them as today (US1-S2).
- **SC-004**: A stream the node cannot frame, or a listing it cannot place, gives an
  error that names the stride or the handler (US1-S5, US1-S8, US1-S9), never a silent
  stall.

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed.
  Every citation here was read at 425b9da0.
- **The fake camera stands for rpicam-vid** for the order of its writes, pixels and
  then metadata, which 013's adapter was built on (`crates/scope-node/src/rpicam/metadata.rs:99`).
  Whether the real Pi 5's rpicam-vid frames as the fake does is the field check's.
- **The Pi 5's padding is 0**, as measured on three frames each of three sizes and in
  the saved capture; FR-004 relies on it.
- **Out of scope:**
  - any change to the console, the preview, the scope link, the mount or imaging;
  - a watchdog on frame age, which would catch any stall and not only this one;
  - a config key for the stride (owner decision 1 (c));
  - other camera stacks than rpicam-apps, and the ASI120MM finder camera over INDI, which
    is spec 045's.
- **Outside the factory:** the field check on the Pi 5 finder and the Pi's config going
  back to 1296x972, in plan.md § Outside the factory, by the owner. A new Pi kit is
  needed once 046 lands.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005]
```

One story: the code is three files of the adapter, and the tests are the bulk
(plan.md § Plan traps 2).
