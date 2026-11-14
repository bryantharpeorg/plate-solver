---
state: draft
depends_on_landed: ["013-live-finder", "014-goto", "016-mcp", "017-console-stop-confirm", "019-night-legibility", "020-live-chart-calm"]
# Drafted 2026-11-14 by the plate-solver operator against `main` at c1ed295, from gap 3 of
# the field findings of the night of 13–14 November 2026: no live frames from that night
# survive. It stays draft until the owner has reviewed it, settled the owner decisions
# and given the approvals listed under "Owner approvals". Flipping it to `ready` is the
# act that dispatches it.
# Refined 2026-11-14 by probe builds of all three stories in throwaway worktrees and two
# reviews (plan.md § Refinement Record), and rebased onto `main` at 2508423; still draft.
---

# Feature Specification: Saving Live Frames at the Telescope

**Feature Branch**: `025-frame-capture`

**Created**: 2026-11-14

**Status**: Draft

**Input**: On the first night at a dark site nothing the console saw was kept: a capture
of `GET /api/live` was lost with a scratch folder. Three pieces of work need saved
frames: replaying the live chart's blinking (spec 020, live chart calm), checking the
sign of the planned up direction (spec 022; that night sky-up sat about 78° clockwise of
frame-up), and a dark-site fixture for detection work (spec 023, drafted elsewhere).
Let the observer start and stop a recording at the telescope. The console writes each
star frame's centroids, settings and axis counts, every live solve and every preview to
a folder, and on request saves the scope node's raw frames there too, so later tests can
replay them.

## Clarifications

### Session 2026-11-14 (review of `main` at c1ed295)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` holds every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it.
- Q: What was found? → A: Seven things, each checked against the code at c1ed295:
  1. A star frame carries centroids, not pixels: up to 200 `Centroid`s, brightest first,
     plus an optional preview JPEG (`crates/scope-link/proto/scope_node/v1/scope_node.proto:53`),
     and both axes' counts at exposure start and end
     (`crates/scope-link/proto/scope_node/v1/scope_node.proto:68`).
  2. Frames reach the console in one place, the frame arm of `active`
     (`crates/finder/src/link.rs:482`), which keeps only the newest on a watch channel
     (`crates/finder/src/link.rs:218`). Live solving reads that channel and broadcasts
     each `LiveReport` (`crates/finder/src/live.rs:53`), 16 deep
     (`crates/finder/src/live.rs:160`), to `GET /api/live`
     (`crates/finder/src/api.rs:457`). Besides the newest frame, the console keeps only
     the last eight previews (`crates/finder/src/live.rs:110`).
  3. The preview is a 640-pixel, quality-80 JPEG (`crates/scope-node/src/preview.rs:22`),
     sent every 5 s or on request (`crates/scope-node/src/preview.rs:11`). It is lossy
     and small, so it can't serve as a detection fixture.
  4. Only the scope node has the pixels: the 8-bit gray frame
     (`crates/scope-node/src/camera.rs:17`), rpicam's luminance plane with its padding
     and colour dropped (`crates/scope-node/src/rpicam/yuv.rs:1`). The detector reads it
     (`crates/scope-node/src/capture.rs:81`) and the service drops it once the centroids
     are sent (`crates/scope-node/src/service.rs:279`). No call on the scope link
     returns pixels.
  5. Two existing tests implement the `ScopeNode` service trait by hand
     (`crates/finder/tests/aim_cli.rs:22`, `crates/scope-link/tests/link.rs:55`), and
     tonic generates no default for a new call, so a new call on `ScopeNode` would stop
     both compiling. A new service would not: the mount and Stop services were added
     that way (`crates/scope-link/proto/scope_node/v1/scope_node.proto:92`,
     `crates/scope-link/proto/scope_node/v1/scope_node.proto:100`).
  6. The console's tonic clients accept replies up to tonic's default of 4 MiB. A full
     2592 × 1944 frame is about 5 MB. The workspace already raises the limit to 16 MiB
     in one test (`crates/grpc-service/tests/e2e_service.rs:102`).
  7. The repository's one saved night frame is an 8-bit gray PNG described in JSON
     (`fixtures/scope-node/fixtures.json:59`), and a test reads it with `image::open`
     (`crates/scope-node/tests/detect.rs:31`).
- Q: What does each later use need? → A: The live chart's replay needs the frames'
  centroids, ids and arrival times, and the live reports in order. The up-direction check
  needs each solve's `north_deg` (`crates/finder/src/solve.rs:247`), its time, the site
  and the aim point. A detection fixture needs raw frames with their exposure, gain and
  detection settings and the centroids the node found in them. US1 records the first
  two; US2 adds the third.
- Q: How is a recording replayed? → A: `Live::start` takes any frame channel
  (`crates/finder/src/live.rs:151`), and an existing test feeds it hand-made frames that
  way (`crates/finder/tests/live.rs:181`). US1-S3 replays a recorded frame through it.
  A replay tool is out of scope.
- Q: Why not save `GET /api/live` with curl, as on that night? → A: Its events lack the
  centroids' brightness, the camera settings, the axis counts, the clock offset and the
  previews, and the capture lives wherever the shell happened to be.

### Session 2026-11-14 (probe builds at c1ed295, review, and `main` at 2508423)

- Q: Which scenarios could not pass as drafted? → A: Two, now reworded. The workspace's
  `serde_json` (`crates/finder/Cargo.toml:32`) parses without `float_roundtrip`, so a
  number a test reads back from a file is often one ULP from the `f64` that was written:
  the US1 probe saw `1175.6363636363635` come back as `1175.6363636363637`, and the US2
  probe saw 6 to 10 of 36 centroids differ. The file holds the exact shortest form; only
  the test's parse is lossy, and no `Cargo.toml` may change. US1-S2 and US2-S3 now
  compare as the shared setup's **Comparing numbers** says.
- Q: What did the page probe find missing? → A: `useSession` builds the session one known
  field at a time (`crates/finder/web/src/useSession.ts:29`) and would drop `recording`,
  so the real page would never show a running recording while every component test
  passed. US3 writes that file, and US3-S5 drives the page through it.
- Q: Does the shared setup point where it says? → A: Only in the right order. Connecting a
  mount re-points the sky along the mount (`crates/simulator/src/controls.rs:34`), whose
  default zero points are near altitude −2° (`crates/simulator/src/mount.rs:34`), while
  `push_to` after the mount moves the tube and keeps the counts
  (`crates/simulator/src/controls.rs:80`). The setup now connects first and pushes after.
- Q: Does any gate see the Recording section at night? → A: No. The one night check of
  Setup (`crates/finder/web/e2e/session.spec.ts:191`) runs on a console with no node, and
  the session carries `node` only once one is paired (`crates/finder/src/session.rs:629`).
  A new check would have to join the simulate project, which means editing
  `playwright.config.ts`. See owner decision 11.
- Q: Which other specs write the same files? → A: `020-live-chart-calm` is `ready`, and
  its US1 landed at 2508423. Its US2 writes `DESIGN.md`, `crates/finder/web/dist` and
  `crates/finder/web/src/api.ts`, as US3 does, so this spec waits for it to land
  (plan.md § Prerequisites). The draft "mount-sky" (pull request 207) also writes files
  US1, US2 and US3 write (plan.md § Risks).

## Owner decisions

Each is a design or taste call. The trio is complete with the recommended answer; the
owner may pick another before flipping, and the trio is then refined again.

1. **Where recordings go.** Recommended: `recordings/<UTC start>/` inside the console's
   state folder, so `--state DIR` moves it with the session and `--simulate` keeps
   simulated nights apart, as it already does for the session
   (`crates/finder/src/main.rs:95`). No new flag. Instead: a `--recordings DIR` flag.
2. **The file format.** Recommended: JSON Lines. `frames.jsonl` has one star frame a
   line, `solves.jsonl` one live report a line, each preview is its own
   `preview-<id>.jpg`, and `recording.json` describes the whole. A line is complete or
   absent, so a console that dies mid-night still leaves a readable recording.
   Instead: one JSON document, or protobuf.
3. **A size cap.** Recommended: a recording ends by itself after 3,600 star frames: an
   hour at 1 s exposures, three hours at 3 s, but about 12 minutes at the node's default
   of 200 ms (`crates/scope-node/src/service.rs:55`); an estimated 0.2 GB an hour at one
   frame a second. Instead: no cap, a cap in bytes, or a cap in time.
4. **Raw pixels need an additive scope-link change.** Recommended: a new service,
   `ScopeNodeRaw`, with one call, `RawFrame`, whose answer the console saves into the
   recording (Clarification 5 rules out a call on `ScopeNode`). Instead: the scope node
   saves raw frames to its own card with no protocol change; the files are then on the
   Pi, need SSH at the telescope, and must be matched to the console's by id.
5. **What "raw" means, and how it is saved.** Recommended: the 8-bit gray frame the
   node's detector reads, saved as an 8-bit gray PNG `raw-<id>.png`, like the existing
   fixture. Not the sensor's 10-bit Bayer data, which the node never asks `rpicam-vid`
   for (`crates/scope-node/src/rpicam/mod.rs:142`). Instead: PGM or FITS.
6. **Raw frames only on request, 1 to 10 at a time.** Recommended. Instead: a switch
   that saves every frame raw, about 1.5 MB a frame.
7. **A page control.** Recommended: a Recording section on the Setup page (US3), with
   `Start recording`, `Stop recording` and `Save 3 raw frames`, worded as plan.md § The
   page gives. Its counts have no singular, so a new recording reads `1 frames · 1
   solves`. Instead: drop US3 and use curl, or singular words for a count of 1.
8. **An MCP tool.** Recommended: none in this spec. Instead: `start_recording` and
   `stop_recording` tools.
9. **What a recording may hold.** Recommended: the site, times and solves as the console
   saw them, but never the node's address or token. A solve line's guidance (a target's
   rise, transit and set, and the turn and tilt to it) would give the site away anyway.
   The README says to review a recording before committing it to this public
   repository. Instead: round the site, which would also mean dropping each solve line's
   guidance.
10. **Raw frames share the scope link's one connection.** Recommended: yes, as drafted.
    A raw reply (about 1.5 MB at 1456 × 1088, 5 MB at 2592 × 1944) travels from the node
    on the same HTTP/2 connection as the heartbeat's answer, Stop's acknowledgement and
    the star frames. The console sends the next heartbeat only after an answer, and only
    an answer renews its deadline (`crates/finder/src/link.rs:473`); both it
    (`crates/finder/src/link.rs:30`) and the node's mount watchdog
    (`crates/scope-node/src/mount/watchdog.rs:42`) act after 1 s without one. On slow
    Wi-Fi a raw transfer can therefore drop the link, and the node then stops the mount,
    ending a GoTo, Track or axis move: the safe direction, but an interruption. The
    README says raw frames are best asked for while the mount is still (FR-015).
    Instead: `POST /api/record/raw` answers 409 while a GoTo, Track or axis move runs;
    or a second connection only for raw frames.
11. **No night check of the Recording section.** Recommended: none. The section uses only
    the classes other Setup sections use (`section-title`, `button`, `button-primary`,
    `quiet`), and the US3 probe's build left the CSS unchanged. No Playwright project
    renders Setup with a node at night, and adding one means editing
    `crates/finder/web/playwright.config.ts`, which no story may. Instead: the owner adds
    such a check outside the factory.

## Owner approvals

Flipping this spec to `ready` approves the recommended answer to every owner decision
above, and the following. Each is named in the FR that makes it. No existing test, golden
or CI file changes.

1. **`POST /api/record`** and the run-only `recording` field of `GET /api/session`
   (FR-001 to FR-009).
2. **An additive scope-link service, `ScopeNodeRaw`**, with one call, `RawFrame`
   (FR-011, FR-012). The scope node on the Pi must be rebuilt before raw frames work.
3. **`POST /api/record/raw`**, its 16 MiB reply limit, and raw replies on the scope
   link's one connection (FR-013 to FR-015, owner decision 10).
4. **The Setup page's Recording section** and its words (FR-016, FR-017), with no night
   check of its own (owner decision 11).
5. **Two new `GLOSSARY.md` terms**, Recording and Raw frame (FR-010, FR-015).

## User Scenarios & Testing *(mandatory)*

All Rust scenarios of US1 and US2 share one setup unless a scenario says otherwise:
- **The console** is built in-process. A `Simulator` over the test database at the site
  40° N 75° W, on a `VirtualClock` from `2026-01-16T02:00:00Z` (Unix 1768528800), as
  `crates/finder/tests/live/support.rs:41` builds it but in this order: first
  `connect_mount` with `SimMount::new` (`crates/simulator/src/mount.rs:232`) on the
  simulator's clock, so frames carry axis counts, then `push_to(50.0, 120.0)`, which
  points the camera at altitude 50° and azimuth 120° and keeps the counts. A `SimNode`
  over it with a 500 ms frame interval and the token `test-token`, served over a fresh
  duplex for each connection with `memory::serve_routes_on` and the node service's
  `routes()`, as `crates/finder/tests/mcp/support.rs:225` serves its node. A session
  paired to it, with the host name `scope.local`, the token `test-token`, that site and
  camera settings of 200 ms and gain 1. Then `AppState::shared` with `with_session`,
  `with_clock` on the simulator's clock, `with_link`, `with_live()` (the real solver) and
  `with_recordings(R)`.
- Each such test runs under `#[tokio::test(start_paused = true)]`, as
  `crates/finder/tests/live.rs:20` does with the real solver, so every wait below is in
  virtual time. US1-S7 is a plain `#[test]`. A test file may share the test database,
  the start and the console clock by including `crates/finder/tests/live/support.rs`
  with `#[path]`, as `crates/finder/tests/live.rs:3` does.
- Before its steps, a scenario on a console whose link can connect waits until the
  session's `node.connected` is true and `node.offset_s` is not null, as
  `crates/finder/tests/mcp/support.rs:263` waits for its link.
- **R** is a path under the workspace's `target/` that does not exist yet, named as
  `crates/finder/tests/simulate_cli.rs:13` names its folders, and removed afterwards.
- Requests go through the router in-process, as `crates/finder/tests/live/support.rs:82`
  sends them, with `Content-Type: application/json`, a `Host` of localhost on port 8080
  and no `Origin` unless a scenario says otherwise. A foreign `Origin` is
  `http://elsewhere.example`.
- **Waiting until** a condition means reading `GET /api/session` every 100 ms until it
  holds, for at most 30 s, and failing otherwise.
- **The folder** is R joined with the session's `recording.folder`. **A frame line** and
  **a solve line** are lines of its `frames.jsonl` and `solves.jsonl`, each parsed by the
  test itself with `serde_json`.
- **Comparing numbers.** The workspace's `serde_json` parses without `float_roundtrip`,
  so a number read back from a file can be one ULP from the `f64` that was written. A
  test compares a number it parsed from a file with one it holds by first passing its
  own through `serde_json::to_string` and `serde_json::from_str`, and then compares
  exactly. "After the round trip" below means this. No comparison uses a tolerance, and
  no feature is added.

### User Story 1 - Record a stretch of the night on the console (Priority: P1)

As the observer at the telescope, I start a recording, and the console writes every
star frame it receives, every live solve and every preview to a new folder until I stop
it. Later, a test can read that folder and replay what the console saw.

**Why this priority**: Spec 020's replay and spec 022's sign check need only what the
console already receives, and the next clear night may be weeks away.

**Independent Test**: `cargo test -p finder --test recording --locked`.

**Acceptance Scenarios**:

1. **Given** the setup, **When** a test sends `POST /api/record` with `{"on":true}`,
   waits until `recording.solves` is at least 2 and sends `{"on":false}`, **Then** a
   committed test asserts that:
   - both answers are 200; the first has `recording.on` true and a `recording.folder`
     of the form `YYYY-MM-DDTHH-MM-SSZ` beginning `2026-01-16T02-`, and the second has
     `recording.on` false and `ended_reason` `stopped`;
   - `recording.json` has `format` `finder-recording/1`, `finder` equal to the
     `version` of `GET /api/health`, `site` `{"lat_deg":40.0,"lon_deg":-75.0}`,
     `started_at` equal to the session's `recording.since`, a `started_utc` that
     `finder::simulate::parse_start` reads as `started_at` rounded down to the whole
     second, and the folder's name equal to `started_utc` with each `:` made `-`;
   - `recording.json` has `aim` null, a `model` key, `camera_settings` equal to the first
     answer's `camera_settings`, and `camera` equal to the first answer's `node.camera`;
   - after the stop, `recording.json` has `ended_reason` `stopped`, a `stopped_at` at
     least its `started_at`, `previews` equal to the number of `preview-*.jpg` files in
     the folder, and `frames` and `solves` equal to the number of frame lines and solve
     lines, which equal the session's `recording.frames` and `recording.solves`;
   - frame-line ids strictly increase, and every solve line's `frame_id` is the id of a
     frame line;
   - 1.5 s after the stop, both files have the lines they had at the stop;
   - no file in the folder contains `scope.local` or `test-token`.
2. **Given** the setup, with the test also keeping every star frame it sees on
   `state.link().unwrap().frames()` and every report on `state.live().unwrap().subscribe()`,
   **When** a recording runs until `recording.previews` is at least 1 and
   `recording.frames` at least 3, and is stopped, **Then** a committed test asserts
   that:
   - at least two frame lines have ids the test also saw, and each such line's `id`,
     `width`, `height`, `axis_start` and `axis_end` equal the frame's, with
     `axis_start` not null, and its `start_s`, `end_s`, `exposure_ms`, `gain` and
     `settings` equal the frame's after the round trip, in the shapes FR-003 gives;
   - each such line's `centroids` are the frame's as `[x, y, brightness]` triples, in
     order, each number equal after the round trip;
   - each such line has `preview` exactly when the frame carried one, naming a file
     whose bytes equal the frame's `preview_jpeg`;
   - each solve line is, character for character, `serde_json::to_string` of the
     report with that `frame_id` the test received;
   - the folder holds as many `preview-*.jpg` files as the session's
     `recording.previews`.
3. **Given** a recording made as in US1-S1, **When** a test builds a `StarFrame` from
   the first frame line whose solve line has a non-null `pointing_seq`, and feeds it to
   a new `Live` over a new `Session` through a watch channel, with a `LinkStatus` that
   is connected and has that frame line's `offset_s`, a `FixedClock` at that line's
   `received_at` (`crates/finder/src/session.rs:41`) and a `DbSolver` on the test
   database, as `crates/finder/tests/live.rs:181` feeds frames, **Then** a committed
   test asserts that the new report's `report.pointing.ra_deg` and `dec_deg` are within
   0.01° of that solve line's, and its `time` within 0.05 s of that solve line's `time`.
4. **Given** the setup, **Then** a committed test asserts that:
   - `POST /api/record` with `{}`, `{"on":"yes"}` and `{"on":true,"raw":true}` each
     answers 400, and `{"on":true}` with a foreign `Origin` answers 403, each with a
     JSON `error`, after which `GET /api/session` has no `recording` key and R does not
     exist;
   - `{"on":false}` before any start answers 200 with no `recording` key;
   - a console built the same way without `with_recordings` answers `{"on":true}` with
     409 and the error `this console has no recordings folder`, and one built with
     `with_recordings` but no link or live loop answers 409 and
     `this console has no live link`;
   - a console built the same way whose R is a regular file answers `{"on":true}` with
     500 and an `error` containing `recording folder`.
5. **Given** the setup, **When** a test sends `{"on":true}` twice, then `{"on":false}`,
   then `{"on":true}`, **Then** a committed test asserts that the first two answers name
   the same folder, the third start names a different one, and both folders exist
   under R.
6. **Given** the setup built with `with_recording_limit(3)` after `with_recordings`,
   **When** a test starts a recording and waits until `recording.on` is false, **Then**
   a committed test asserts `ended_reason` `limit`, `recording.frames` 3 and exactly
   three frame lines. **Given** the setup, **When** a test starts a recording, waits
   until `recording.frames` is at least 1 and deletes the folder, **Then** a committed
   test asserts that `recording.on` becomes false within 5 s with `ended_reason`
   `write_failed`, and that the folder has not been made again.
7. **Given** the built `finder` binary run with `--simulate`, `--state S` and
   `--listen` on port 0 of `127.0.0.1`, S a fresh folder under `target/`, **When** a
   test sends `POST /api/record` with `{"on":true}` and reads `GET /api/session`,
   **Then** a committed test asserts that `recordings/` under S holds exactly one
   folder, named as the session's `recording.folder`, and that it holds
   `recording.json`.

---

### User Story 2 - Save the scope node's raw frames into the recording (Priority: P2)

As the observer at a dark site, while recording, I ask for a few raw frames, and the
console saves the pixels the scope node's detector saw, next to those frames' centroids
and settings, as a fixture for detection work.

**Why this priority**: Only a detection fixture needs pixels, and it needs a rebuild of
the scope node on the Pi.

**Independent Test**: `cargo test -p scope-node --test raw_frame --locked` and
`cargo test -p finder --test recording_raw --locked`.

**Acceptance Scenarios**:

1. **Given** a `NodeService::new` over a `SyntheticCamera` on
   `scope_node::clock::SystemClock` (`crates/scope-node/src/clock.rs:8`) with default
   settings, served with `routes()` over `scope_link::memory::channel_pair`, as
   `crates/simulator/tests/stop_finishes.rs:101` serves one, under
   `#[tokio::test(start_paused = true)]`, and a test reading its `Frames` stream,
   **When** the test calls `RawFrame` with the token, **Then** a committed test asserts
   that the reply's `pixels` hold `width × height` bytes, that a star frame on the stream
   has the reply's `id`, `width` and `height`, and that `star_detection::detect_stars`
   over those pixels, with that frame's `settings.detection` sigma, binning,
   `normalize_rows` and `detect_hot_pixels` and cut to the first 200, gives exactly, bit
   for bit, that frame's centroids, which are not empty. No JSON is involved here.
2. **Given** the same node, **Then** a committed test asserts that `RawFrame` with the
   token `wrong-token` answers `Unauthenticated`; and that with the node's frames stopped
   (`NodeControls::stop_frames`, `crates/scope-node/src/service.rs:62`) and 2 s allowed
   for a frame already being taken, two `RawFrame` calls started together, with frames
   started again 300 ms later (`NodeControls::start_frames`), are both answered, with the
   same `id`.
3. **Given** the setup with a recording on, **When** a test sends `POST /api/record/raw`
   with `{"frames":2}` and waits until `recording.raw.saved` is 2, **Then** a committed
   test asserts that the answer was 200, that `recording.raw` is
   `{"wanted":2,"saved":2}`, and that the folder holds exactly two `raw-<id>.png` files
   and no `.part` file. For each, `image` decodes an 8-bit gray image of the frame
   line's `width` and `height` for that id, and `star_detection::detect_stars` over it,
   with that line's detection settings and cut to 200, gives the line's centroids, each
   detected `[x, y, brightness]` compared after the round trip.
4. **Given** the setup, **Then** a committed test asserts that:
   - `POST /api/record/raw` with `{"frames":1}` and no recording on answers 409 and
     `start a recording first`;
   - a console built the same way without `with_recordings` answers `{"frames":1}` with
     409 and `this console has no recordings folder`;
   - with a recording on, it answers 400 for `{"frames":0}`, `{"frames":11}`,
     `{"frames":2.5}` and `{}`, and 403 for `{"frames":1}` with a foreign `Origin`;
   - with a request of `{"frames":10}` running, it answers a second request with 409
     and `raw frames are already being saved`;
   - on a console whose transport's connector always answers an I/O error
     (`Transport::connector`), with a recording on, it answers 409 and
     `the scope node is not connected`;
   - with a recording on, the simulator's frames stopped (`Simulator::stop_frames`,
     `crates/simulator/src/controls.rs:105`) and 2 s waited, `{"frames":1}` answers 200,
     and within 20 s `recording.raw` is
     `{"wanted":1,"saved":0,"error":"no raw frame arrived within 15 s"}` and the folder
     holds no `raw-` file;
   - a request of `{"frames":10}` followed at once by `{"on":false}` leaves fewer than
     three `raw-` files in the folder 8 s later.
5. **Given** the setup with the node served by `into_server()` alone, as
   `crates/finder/tests/live/support.rs:52` serves it, and a recording on, **When** a test
   sends `{"frames":1}` and waits until `recording.raw.error` is present, **Then** a
   committed test asserts that `recording.raw` is
   `{"wanted":1,"saved":0,"error":"this scope node cannot send raw frames"}` and that
   the folder holds no `raw-` file. **Given** instead the node's `into_server()` and a
   `ScopeNodeRaw` of the test's own that answers a 4 × 4 frame with 15 pixel bytes, both
   served with `memory::serve_routes_on` (`crates/scope-link/src/memory.rs:70`), **When**
   the same is done, **Then** the committed test asserts that `recording.raw` is
   `{"wanted":1,"saved":0,"error":"the raw frame request failed"}` and that the folder
   holds no `raw-` file.
6. **Given** the setup with the console paired instead to a `NodeService` over a camera
   of the test's own that answers 2592 × 1944 frames of uniform value 10, every 2 s,
   with ranges that allow the session's 200 ms and gain 1, on a `NodeClock` of the
   test's own that reads the simulator's `VirtualClock::node_now`
   (`crates/simulator/src/clock.rs:28`), **When** a test starts a recording, sends
   `{"frames":1}` and waits until `recording.raw.saved` is 1, **Then** a committed test
   asserts that the one `raw-<id>.png` decodes to a 2592 × 1944 8-bit gray image whose
   every pixel is 10.

---

### User Story 3 - Start and stop a recording from the Setup page (Priority: P3)

As the observer with only the page to hand, I open Setup, start a recording, see that it
is running and how much it holds, ask for three raw frames, and stop it.

**Why this priority**: At the telescope the page is often the only thing to hand; curl
needs a terminal.

**Independent Test**: `npm test` in `crates/finder/web` runs
`src/components/RecordingSection.test.tsx`.

**Acceptance Scenarios**:

1. **Given** `SetupPage` rendered with a session holding a connected `node` and no
   `recording`, with `EventSource` stubbed as
   `crates/finder/web/src/components/MountSection.test.tsx:13` stubs it, **Then** a
   committed test asserts one region named `Recording`, after the region named
   `Aim point` in document order, holding a button named `Start recording`, and that
   the region holds no `status` or `alert`. **When** the test clicks it, with `fetch`
   answering 200 and a session, **Then** the committed test asserts that `fetch` was
   called with `/api/record` and
   `{"method":"POST","headers":{"Content-Type":"application/json"},"body":"{\"on\":true}"}`,
   then `onChange`; and that a 409 answer with
   `{"error":"this console has no recordings folder"}` shows an alert with exactly that
   text and calls no `onChange`. **Given** `SetupPage` with no `node`, **Then** a
   committed test asserts no region named `Recording`.
2. **Given** `RecordingSection` rendered with a session holding a connected `node` and
   `recording` `{"on":true,"folder":"2026-01-16T02-00-00Z","since":1768528800,"frames":12,"solves":9,"previews":2}`,
   **Then** a committed test asserts that its status reads exactly
   `Recording to 2026-01-16T02-00-00Z · 12 frames · 9 solves`, that `Stop recording`
   calls `fetch` with `/api/record` and
   `{"method":"POST","headers":{"Content-Type":"application/json"},"body":"{\"on\":false}"}`,
   and that `Save 3 raw frames` calls it with `/api/record/raw` and
   `{"method":"POST","headers":{"Content-Type":"application/json"},"body":"{\"frames\":3}"}`,
   each answered 200 with a session and followed by `onChange`. With the node's
   `connected` false and the same recording, it still shows `Stop recording`.
3. **Given** the same with `raw` `{"wanted":3,"saved":1}`, **Then** a committed test
   asserts that the status's text holds both its US3-S2 line and
   `Raw frames: 1 of 3 saved`, and that `Save 3 raw frames` is disabled. **Given** `raw`
   `{"wanted":3,"saved":0,"error":"this scope node cannot send raw frames"}`, **Then** a
   committed test asserts an alert reading exactly
   `Raw frames stopped: this scope node cannot send raw frames.` and that the button is
   enabled.
4. **Given** `RecordingSection` with the recording of US3-S2 but `on` false and
   `ended_reason` `stopped`, **Then** a committed test asserts the status reads exactly
   `Saved 2026-01-16T02-00-00Z: 12 frames, 9 solves.`, a `Start recording` button and no
   alert. With `ended_reason` `limit`, the committed test asserts the status also holds
   `It stopped at its frame limit.`; with `write_failed`, an alert reading exactly
   `Recording stopped: the console could not write to its folder.`
5. **Given** `SetupPage` fed by `useSession`, as
   `crates/finder/web/src/components/ScopeNodeSection.test.tsx:70` feeds it, with
   `fetch` answering every request with a session holding a connected `node` and the
   recording of US3-S2, **Then** a committed test asserts that the region named
   `Recording` shows a status reading exactly
   `Recording to 2026-01-16T02-00-00Z · 12 frames · 9 solves`.

---

### Edge Cases

- **A frame replaced before the recorder reads it.** The link keeps only the newest
  frame. At the node's cadence of 200 ms or more the recorder keeps up; a frame it
  misses shows as a gap in the ids, as a lagging link's frames already do.
- **The link drops or is re-paired.** The recording carries on and its lines resume
  with the frames. A new pairing's frames go into the same recording; their `offset_s`
  tells the clocks apart.
- **The console exits mid-recording.** The lines written so far stay readable, and
  `recording.json` has no `stopped_at`. Nothing resumes at the next start.
- **`--simulate`.** Its state folder is the simulated one, so by default its recordings
  go under `simulate/recordings/`.
- **A solve of a frame from before the start or after the stop** is not written.
- **Raw frames from an older scope node.** The node lacks the service and the raw
  request ends with its error (US2-S5); the recording carries on.
- **A recording that ends during a raw request.** The request stops with it, and
  `recording.raw` keeps the counts it had, with no `error`. The page shows an ended
  recording, so it reads no progress from it.
- **Raw frames on slow Wi-Fi.** A raw reply shares the connection with the heartbeat, so
  it can drop the link and make the node stop the mount (owner decision 10).
- **A raw frame's size.** 1.3 MB for 1296 × 972 and about 5 MB for 2592 × 1944 before
  compression; the repository's saved night frame compressed to about 0.6 MB.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `POST /api/record` SHALL accept exactly `{"on":true}` or `{"on":false}`
  under the same-origin rule and answer 200 with the session. Any other body SHALL
  answer 400 and a foreign origin 403, each with a JSON `error` and no change.
- **FR-002**: `{"on":true}` with no recording on SHALL start one in a new folder under
  the recordings folder, named for the console's UTC then as `YYYY-MM-DDTHH-MM-SSZ`,
  with `-2`, `-3` and so on added when that name is taken. With a recording on it SHALL
  keep that recording. `{"on":false}` SHALL end the recording on, and otherwise change
  nothing.
- **FR-003**: A recording SHALL append to `frames.jsonl` one line per star frame it reads
  from the link's newest-frame channel after its start; a frame replaced before it is
  read is missing, and shows as a gap in the ids. A line holds the frame's `id`,
  `start_s`, `end_s`, `width`, `height`, `exposure_ms`, `gain`, `settings`, its
  `centroids` as `[x, y, brightness]` triples in the frame's order, `axis_start` and
  `axis_end`, the console's `received_at` and the link's `offset_s` at arrival (null
  when unknown). `settings` is `{exposure_ms, gain, detection}`, or null when the frame
  has none, with `exposure_ms` and `gain` null when automatic and `detection`
  `{sigma, binning, normalize_rows, detect_hot_pixels}` or null when absent. Each axis
  field is `{azimuth, altitude}`, or null when absent. A frame with a preview SHALL also
  have `preview` naming `preview-<id>.jpg`, which holds the preview's bytes unchanged
  and is written before the line. Lines and `recording.json` are written with
  `serde_json`.
- **FR-004**: A recording SHALL append to `solves.jsonl` each live report it receives of
  a frame it recorded, as the JSON that a `live` event of `GET /api/live` carries.
- **FR-005**: Each line SHALL be written by one write to a file opened for appending each
  time, so a line is whole or absent and a deleted folder fails the next write.
- **FR-006**: `recording.json` SHALL be written at the start with `format`
  `finder-recording/1`, `finder` (the console's version), `started_at` (console Unix
  seconds), `started_utc` (`YYYY-MM-DDTHH:MM:SSZ`), and the session's `site`, `aim`,
  `camera_settings`, `model` and `node.camera` as `camera`, each null when absent. It
  SHALL be rewritten at the end with `stopped_at`, `ended_reason`, `frames`, `solves` and
  `previews` added. No file of a recording SHALL hold the node's address or token.
- **FR-007**: A recording SHALL end with `ended_reason` `stopped` on `{"on":false}`,
  `limit` after its 3,600th star frame, and `write_failed` when a write fails. No line
  SHALL be written after the answer to `{"on":false}`.
  `AppState::with_recording_limit(n)` SHALL set another limit; the scenarios use it
  (US1-S6), and the default of 3,600 is a constant that no scenario runs to.
- **FR-008**: `GET /api/session` SHALL carry
  `recording: {on, folder, since, frames, solves, previews, ended_reason?}` from the
  first accepted start until the console exits, and omit it before. Every start of the
  console begins with no recording.
- **FR-009**: `AppState::with_recordings(DIR)` SHALL enable recording under DIR. Without
  it, `{"on":true}` SHALL answer 409 with `this console has no recordings folder`;
  without a live loop, 409 with `this console has no live link`; and when DIR or the new
  folder can't be made, 500 with an `error` naming the recording folder.
- **FR-010**: The `finder` binary SHALL record under `recordings/` in its state folder.
  `crates/finder/README.md` SHALL document the route, the files and owner decision 9's
  note, and `GLOSSARY.md` SHALL define **Recording**.
- **FR-011**: The scope link SHALL gain a service `ScopeNodeRaw`, under the pairing
  token, with one call `RawFrame`, whose reply carries the `id`, `width`, `height` and
  8-bit gray `pixels` (`width × height` bytes, row-major, no padding) of the next star
  frame the node sends. No existing message, field or call SHALL change.
- **FR-012**: The scope node SHALL serve `ScopeNodeRaw` among its routes and answer every
  waiting `RawFrame` call with the pixels its detector read for that star frame.
- **FR-013**: `POST /api/record/raw` SHALL accept exactly `{"frames":N}`, N a whole
  number from 1 to 10, under the same-origin rule. It SHALL answer 409 with
  `this console has no recordings folder` when the console has none,
  `start a recording first` when no recording is on, `the scope node is not connected`
  when the link offers no raw client, and `raw frames are already being saved` while a
  request runs; and 400 for any other body. Otherwise it SHALL answer 200 with the
  session and ask for N raw frames one after another, saving each as an 8-bit gray PNG
  `raw-<id>.png` in the recording's folder, written under a `.part` name and then
  renamed.
- **FR-014**: `recording.raw` SHALL report `{wanted, saved, error?}` for the recording's
  latest raw request. A request SHALL end at its first failure, with `error`
  `this scope node cannot send raw frames` when the node lacks the service,
  `no raw frame arrived within 15 s` after 15 s without an answer, and
  `the raw frame request failed` otherwise, a reply whose pixels are not
  `width × height` bytes included. It SHALL also end when its recording ends, leaving
  `recording.raw` as it was, with no `error`.
- **FR-015**: The console's raw calls SHALL accept replies of up to 16 MiB, on the scope
  link's paired connection (owner decision 10). `crates/finder/README.md` SHALL document
  the route, that the scope node must be rebuilt for it, and that raw frames are best
  asked for while the mount is still, and `GLOSSARY.md` SHALL define **Raw frame**.
- **FR-016**: The Setup page SHALL show a section headed Recording, after the aim-point
  section, whenever `session.node` is present, connected or not, reading `recording`
  from `GET /api/session` as it reads `track`. With no recording on it SHALL offer
  `Start recording`. While one is on, it SHALL show a status naming the folder and
  counts, `Stop recording`, and `Save 3 raw frames`, disabled while a raw request runs,
  with that request's progress or error. After a recording ends it SHALL say what was
  saved and why it ended, unless the observer stopped it. With a node and no
  `recording`, it SHALL show no `status` or `alert`.
- **FR-017**: `DESIGN.md` SHALL describe the section, `.impeccable/design.json` SHALL
  list it as a component, and `crates/finder/web/dist` SHALL be the build of its source.

### Key Entities

- **Recording**: one folder of what the console saw between a start and a stop.
- **Frame line**: one star frame in `frames.jsonl`.
- **Solve line**: one live report in `solves.jsonl`.
- **Raw frame**: the 8-bit gray pixels of one star frame, as the node's detector read
  them.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file, `CODEOWNERS`
  entry, `ergane.yaml` or `playwright.config.ts` is edited.
- **SC-003**: A recorded star frame replays to its live solve within 0.01° (US1-S3), and
  a saved raw frame reproduces its frame's centroids exactly: bit for bit over the scope
  link (US2-S1), and after the shared setup's round trip against its frame line
  (US2-S3).

## Assumptions

- **Earlier features are in place.** 013, 014, 016, 017, 019 and 020-live-chart-calm
  have landed. Every citation here was read at c1ed295 and re-checked at 2508423, where
  only `crates/finder/src/solve.rs` had moved. 020-live-chart-calm's US2 will move the
  cited lines of `DESIGN.md` and `crates/finder/web/src/api.ts` (plan.md
  § Prerequisites).
- **Out of scope:**
  - a replay tool, loader or replaying node;
  - dark-sky detection, the noise floor and auto-exposure (spec 023);
  - the sensor's raw Bayer data;
  - MCP tools for recording (owner decision 8);
  - recording uploads, MCP calls, or mount telemetry beyond each frame's axis counts;
  - listing, fetching or deleting recordings through the API;
  - any change to the preview's size, quality or schedule;
  - rounding or removing the site (owner decision 9);
  - any change to `--simulate`.
- **Outside the factory:** rebuilding the console on the Surface and, for US2, the scope
  node on the Pi; the first recording at the telescope; choosing which recordings become
  fixtures and committing them, by the owner; and any night check of the Recording
  section (owner decision 11).
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007, FR-008, FR-009, FR-010]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-011, FR-012, FR-013, FR-014, FR-015]
US3:
  depends_on: []
  depends_on_merged: [US1, US2]
  implements: [FR-016, FR-017]
```

Both edges are merge edges. US2 writes into US1's recording folder and extends its
`Recorder`, route table, route list, README section and glossary. US3 reads the
`recording` field of both and calls both routes. The stories run one after another.
