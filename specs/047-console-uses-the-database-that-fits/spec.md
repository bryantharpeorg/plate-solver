---
state: draft
depends_on_landed: ["013-live-finder", "016-mcp", "020-live-chart-calm", "022-altaz-guidance", "026-mount-sky", "027-finder-tests-by-system", "030-imaging-camera-link", "031-imaging-frames-on-console", "037-imaging-run", "040-stack-waits-for-steady", "041-imaging-run-on-the-rig", "042-imaging-run-page-and-mcp"]
# Drafted 2026-11-21 against `main` at 425b9da0 (425b9da0, 044 landed), at the owner's ask
# that day ("mock this up as a spec to get all of this work done"), from a probe of that
# day on the branch `probe/047` cut from 425b9da0: the choice rules tried on simulated
# frames and in the real live loop, and a trial build of all three stories with every
# finder test passing. Every `path:line` here was read at 425b9da0. State: draft. The
# owner took the four recommended answers on 2026-11-21 ("yes"); no probe round has yet
# built the stories from this text alone, and the flip is the owner's to give. What rests
# on a run and what on reading is said where it is used
# (plan.md § What was run and what was read).
---

# Feature Specification: The Console Uses the Database That Fits

**Feature Branch**: `047-console-uses-the-database-that-fits`

**Created**: 2026-11-21

**Status**: Draft

**Input**: The finder console loads exactly one pattern database, given by `--db`. The
owner may move the finder camera behind a narrow guide scope, about 2.3° by 1.7°, which
needs a second database built for that field; today's 16 mm lens sees about 17° by 13°,
which cedar's 10–30° database covers. Make the console hold several databases side by
side, use the one that fits the camera in front of it, and say which one it uses and
why, on the session, the page and MCP, so that a future user who changes lenses changes
nothing. `--db` given once works as today.

## Clarifications

### Session 2026-11-21 (the probe on `main` at 425b9da0)

- Q: What does this trio read besides itself? → A: Nothing about the feature. `spec.md`
  holds every requirement and scenario, and `plan.md` every design decision, the
  interfaces, the shapes that existing tests pin and the alternatives rejected. A story
  that would change a decision escalates rather than re-deciding it. The probe's branch
  is not in the repository; what it showed is written here.
- Q: What does the console do with its database today? → A: `--db` is one path
  (`crates/finder/src/main.rs:25`), loaded once (`crates/finder/src/main.rs:138`) into
  `AppState::new` (`crates/finder/src/main.rs:176`), which holds one
  `Arc<PatternDatabase>` (`crates/finder/src/api.rs:102`). That one database solves the
  live loop's frames through `DbSolver` (`crates/finder/src/api.rs:294`), every upload
  (`crates/finder/src/api.rs:1528`), and is the simulator's and the drawn sky's star
  table (`crates/finder/src/api.rs:210`, `crates/finder/src/api.rs:262`). The live loop
  is given its field range once (`crates/finder/src/api.rs:312`), and the session
  reports it from the first frame on as `field.db_min_fov_deg` and `db_max_fov_deg`
  (`crates/finder/src/session.rs:549`, `crates/finder/src/session.rs:779`).
  `GET /api/health` reports the one database (`crates/finder/src/api.rs:1298`).
- Q: Is a database's field range a hard limit on what it solves? → A: No. On the
  simulator's noise-free centroids (run), cedar's 10–30° database solved frames of 5.5°
  to 45°, and a test database built for 4.5–7° from cedar's own stars solved 5° to 28°.
  What stops a database is too few catalog stars in the frame, or patterns of the wrong
  size once a field estimate narrows the solve. With a 12° estimate, the 4.5–7° database
  failed 12° frames it solved without one (run). So the range is a statement of what a
  database was built for, and the console uses it to choose, not to refuse.
- Q: What does the live loop already know? → A: The field of the last solve. It solves
  the next frame with that field as its estimate and 5% allowed error, and drops the
  estimate after three failed solves in a row (`crates/finder/src/live.rs:177`,
  `crates/finder/src/live.rs:204`, `crates/finder/src/live.rs:236`; 013's FR-023). That
  is the lens change's signal: three frames fail against the old field, and the fourth
  is solved without an estimate.
- Q: Which rule chooses the database? → A: Owner decision 1. The probe tried four rules
  on the same frames (plan.md § What was run and what was read). Recommended: the
  solved field decides. With no field estimate the console tries the database in use,
  then each of the others in the order given, until one solves; it then uses the
  database whose range holds the solved field. With an estimate it solves only with the
  database in use.
- Q: How does a lens change look under the recommended rule? → A: Run in the real live
  loop, with cedar's database and the 4.5–7° test database, the simulator's field changed
  after the sixth report: the three reports after the change do not solve, the fourth
  solves without an estimate, and from the fifth on the console uses the database whose
  range holds the new field, in both directions (US2-S5, US2-S6).
- Q: What does a field that no database covers look like? → A: Two ways, both run. A
  frame that some database solves although its field lies in no range: the console keeps
  the database that solved it with the reason `solved`, and Setup says the solved field
  is outside every range (US2-S8, US3-S4). Frames with at least 10 stars that no database
  solves: after 30 s Setup names every range (US2-S9, US3-S3), where today it names the
  one.
- Q: What does "`--db` given once works as today" mean here? → A: The same database
  solves every frame and upload, every key the console answers today keeps its value,
  the start line and the page's words are unchanged, and every existing test passes
  unchanged. New keys appear beside the old ones, naming that one database (owner
  approval 3).
- Q: What does a solve cost with several databases? → A: A frame with an estimate costs
  one solve, as today. A frame without one costs at most one solve per database, each
  bounded by the solve's own 5 s timeout (`crates/finder/src/solve.rs:69`), and the live
  loop holds the solve gate across all of them, so an upload waits behind them. A frame
  with too few stars stops after the first try: no database can solve it.
- Q: What does memory cost? → A: Owner decision 3. The loader widens every pattern
  slot to 38 bytes in memory (`crates/pattern-database/src/format.rs:39`,
  `crates/pattern-database/src/load.rs:119`): cedar's 3,032,963 slots take about
  115 MB, and the probe measured 116 MiB of resident memory for its load (run, x86_64).
  A narrow database for the guide scope is estimated at about 22 million patterns and
  1.7 GB in memory, a few hundred MB on disk (plan.md § Memory, an estimate, not run).
  Both fit an 8 GB Pi with about 6 GB to spare.
- Q: Where do the tests' databases come from? → A: Owner decision 4. Recommended: built
  in the test from stars already in the repository with `database-generation`'s own
  library, as a dev-dependency of the finder (owner approval 1): a 4.5–7° database from
  cedar's stars around the simulator's pointing, and 20–30° and 35–50° databases from the
  repository's BSC5 (`reference-solutions/cedar-solve/tests/data/bsc5`). Each is
  written once per target directory and reused. No binary fixture is added, and no
  catalog is downloaded.
- Q: Does anything here change the node, the proto, the mount, imaging or the solver?
  → A: No. The solver is called as today, once per database tried; `pattern-database`
  and `database-generation` are not edited. The imaging solver (ASTAP, `--astap-db`) is
  a different database and is not touched.
- Q: What does this change in earlier specs? → A: 013's FR-028 reads, in part:
  "`GET /api/session` SHALL report the latest solved field of view, the loaded
  database's field range, and how long frames holding at least 10 stars have gone
  unsolved" (`specs/013-live-finder/spec.md:848`); with several databases, the range is
  the database in use's (FR-011). 013's FR-037 reads, in part: "The Setup page SHALL show
  the finder camera's solved field of view beside the loaded pattern database's range"
  (`specs/013-live-finder/spec.md:891`); with several, beside the range of the database
  in use, and its warnings name every range (FR-015). 013's FR-023
  (`specs/013-live-finder/spec.md:824`), the estimate and its three failures, is
  unchanged: the tries happen inside one solve of the live loop. The OpenSpec
  requirement "Serve the console from one binary" says the binary "loads one pattern
  database" (`openspec/specs/finder-console/spec.md:12`); that file is not edited.
- Q: What is outside the factory? → A: Building the narrow database (a catalog download
  that needs the owner's OK), the Pi's deploy installing every database, and the field
  check when the camera and the guide scope arrive (plan.md § Outside the factory). None
  is run here.

## Owner decisions

The owner took each recommended answer on 2026-11-21 ("yes"), and the scenarios that
depend on one name it. The alternatives are kept below for the record.

1. **The choice rule** (FR-009, FR-010). *Settled 2026-11-21: the recommended answer.*
   Recommended: (a) **the solved field decides.**
   With no field estimate, the database in use, then each of the others in the order
   given, until one solves; then the database whose range holds the solved field: the
   one that solved if it does, otherwise the first given that does, otherwise the one
   that solved. With an estimate, only the database in use. Weighed against:
   - *a cold start's time:* (a) costs up to one solve per database on a frame with no
     estimate, each bounded by the 5 s solve timeout, and then one solve a frame; listing
     the database for the fitted optics first makes the first try the right one;
   - *a future user who knows nothing about databases:* (a) needs no setting; the user
     installs the databases once and changes lenses freely;
   - *testability on the factory's x86 gates:* (a) runs in the real live loop on the
     simulator with the test databases of owner decision 4 (US2, run).

   Instead:
   - (b) **try in turn and keep the one that solved**, with no ranges. Run: listed
     narrow first, the 4.5–7° database solved a 12° frame without an estimate, then
     failed three of every four frames against the 12° estimate, and never moved to
     cedar's; (a) moved to cedar's after that first frame (US2-S7).
   - (c) **a stated field**: a new setting, such as the finder's focal length or its
     field in degrees, picks the database whose range holds it. One solve a frame and no
     search, but the user must know the field, and a lens changed without changing the
     setting is never noticed. Run: stated 12° and the lens changed to 5°, every frame
     after the change failed.
   - (d) **one setting naming the database**, the plain switch: the same as giving one
     `--db`, which already works; nothing to build.
2. **How databases are named on the command line** (FR-001). *Settled 2026-11-21: the recommended answer.*
   Recommended: (a) **`--db`
   repeated**, in the order to try them. `--db` given once is today's command, unchanged;
   with none, cedar's database as today; the order is written where the unit is. Instead:
   (b) **`--db-dir FOLDER`**, every `.npz` in a folder in file-name order: the deploy
   drops files in and the order is implicit; or (c) **a list file**, one path a line:
   another file to keep in step with the folder.
3. **Memory** (FR-001). *Settled 2026-11-21: the recommended answer.*
   Recommended: (a) **every database loaded at start.** Estimated
   for the Pi: cedar's about 115 MB and a 2.0–2.6° Tycho database about 1.7 GB, about
   1.8 GB of 8 GB (plan.md § Memory). A switch costs nothing at night. Instead: (b)
   **one at a time**, the others loaded on a switch: the peak is the largest database's
   size plus a load's transient, and every switch stalls the live loop for a load.
4. **The tests' databases** (FR-019). *Settled 2026-11-21: the recommended answer.*
   Recommended: (a) **built in the test** with
   `database-generation`'s library, a dev-dependency of the finder (owner approval 1),
   from cedar's stars and the repository's BSC5, once per target directory. Instead: (b)
   **committed fixtures**: the probe's two smaller ones are 1.75 MB and 2.54 MB of
   binary, which every clone carries and no diff shows.

## Owner approvals

Flipping this spec to `ready` approves the answer to every owner decision above, and the
following. Each is named in the FR that makes it. No existing test, golden or CI file
changes.

1. **`database-generation` as a dev-dependency of `crates/finder`** (FR-019; constitution
   Principle IV). A path dependency inside the workspace; `Cargo.lock` gains one line in
   the finder's dependency list and no new crate.
2. **New public API in the finder's library**: the module `finder::databases`,
   `AppState::with_databases`, `Live::start_with_databases`, `SolveReport::database` and
   `FieldView`'s `database` and `databases` (FR-006, FR-011, FR-012). No existing
   signature changes: `AppState::new`, `AppState::shared` and `Live::start` stay as they
   are.
3. **New keys beside the old ones with one `--db`** (FR-008): `databases` in
   `GET /api/health`, `database` in a solve report, and `database` and `databases` in the
   session's `field` from the live loop.
4. **A start line per database with several** (FR-003): `database NAME: …` in place of
   `database: …`, which stays the line for one.
5. **The finder README's section and rows** (FR-018).

## User Scenarios & Testing *(mandatory)*

Every scenario shares these definitions.

- **The gate command.** Every Rust scenario runs in
  `cargo nextest run --workspace --all-targets --all-features --locked`, the `test`
  gate's first command, each test in its own process. The page's scenarios run in
  `npm test`, a later command of the same gate, in `crates/finder/web`.
- **Cedar's database** is `crate::support::live::DB`, cedar's `default_database.npz`
  (hip_main, 10–30°, 42,212 stars, 1,010,981 patterns).
- **The test databases** are built by `crates/finder/tests/console/databases/mod.rs`
  (US1) with `database-generation`'s library, as plan.md § The test databases gives the
  calls, and written under `CARGO_TARGET_TMPDIR`:
  - **narrow**, 4.5–7°, `star_catalog` `hip_main`: the stars of cedar's database that
    `crate::support::live::sim()`'s simulator (`crates/finder/tests/support/live.rs:41`),
    its optics set to a 30° field, draws with `field_stars(1e6, 100.0)`; lattice
    oversampling 20;
  - **mid**, 20–30°, `star_catalog` `bsc5`: the repository's BSC5; oversampling 20;
  - **wide**, 35–50°, `star_catalog` `bsc5`: the repository's BSC5; oversampling 100.
- **Databases named as given** are a `Databases` built from those, each with the name the
  scenario gives it, in the scenario's order, and handed to
  `AppState::with_databases`.
- **The live rig** (US2, US3-S8) is `crate::support::live::sim()`, its optics set with
  `set_optics` to the scenario's horizontal field (the rest `Optics::default()`); a
  `Session` given the node `scope.local` on port 50051, the token `test-token` and the camera
  settings exposure 200 ms and gain 1, as `crate::support::live::state_with_interval`
  gives its own (`crates/finder/tests/support/live.rs:70`), with no site, target or aim;
  a `SimNode` over the simulator with the token `test-token` and a 500 ms frame interval
  behind `crate::support::live::transport` (`crates/finder/tests/support/live.rs:46`);
  and `AppState::shared` over cedar's database with `Samples::default()`, then
  `with_databases` with the scenario's databases, `with_session`, `with_clock` with the
  simulator's clock as `ConsoleClock`, `with_link` and `with_live()`, in that order.
  **A report** is the next `LiveReport` from `live().subscribe()`, taken
  with `crate::support::live::event`; **the session** is `GET /api/session` read right
  after it. **The field after the sixth** means `set_optics` is called again after the
  sixth report and before the seventh is awaited. The test runs under
  `#[tokio::test(start_paused = true)]`.
- **An upload** is `POST /api/solve` through `finder::router` in process, with a
  multipart form holding either `sample` naming `orion_belt.jpg` from cedar's
  `medium_fov` samples, or an `image` file. **A black frame** is a 64 by 64 black PNG
  made with the `image` crate.
- **The binary** (US1-S1 to US1-S3) is the built `finder`, `env!("CARGO_BIN_EXE_finder")`,
  run with each `--db` the scenario gives, `--listen` on the loopback address and port 0, `--samples` naming a
  missing folder and `--state` naming a `crate::support::Scratch` folder, standard error
  piped; **its start lines** are the lines of standard error before
  `finder console: http://`.
- **Oracles.** Cedar's database's own properties, the BSC5 star count, the ranges the
  test databases are built with, the names the test gives, the simulator's field (within
  0.1°), the frame at which the test changes it, and the words of these scenarios. No
  expected value comes from the code under test.

### User Story 1 - The console holds several databases and solves uploads with each (Priority: P1)

As the owner, I give `finder` both the finder lens's database and the guide scope's, and
the console loads both, lists them, and solves a frame I upload with whichever fits.

**Why this priority**: Everything else rests on the console holding more than one
database. Uploads and the health report are where the list first shows.

**Independent Test**: `crates/finder/tests/cli/databases.rs` (US1-S1 to US1-S3) and
`crates/finder/tests/console/databases/several.rs` (US1-S4 to US1-S7), both new, the
second beside the test databases' module `crates/finder/tests/console/databases/mod.rs`,
new. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs them; during
development, `cargo nextest run -p finder --locked --test cli databases::` and
`cargo nextest run -p finder --locked --test console databases::`.

**Acceptance Scenarios**:

1. **Given** a copy of cedar's database at `copy.npz` in the scratch folder, **When** a
   committed test runs the binary with `--db` that copy and then `--db` cedar's file, and
   reads `GET /api/health` from it, **Then** it asserts that its start lines hold one
   starting `database copy: 42212 stars, 1010981 patterns, FOV 10-30 deg, loaded in ` and
   one starting `database default_database: 42212 stars`; that the answer's `databases`
   names `copy` and then `default_database`; that the first's `stars` is 42212; and that
   the second's `min_fov` is 10.
2. **Given** cedar's file, **When** a committed test runs the binary with `--db` that
   file twice, **Then** it asserts that the binary exits with a status other than 0 and
   that its standard error holds `two databases are named default_database`.
3. **Given** cedar's file, **When** a committed test runs the binary with `--db` that file
   once, **Then** it asserts that exactly one start line starts with `database`, and that
   it starts `database: 42212 stars, 1010981 patterns, FOV 10-30 deg, loaded in `.
4. **Given** an `AppState` over cedar's database with narrow, cedar's, mid and wide named
   `narrow`, `cedar`, `mid` and `wide` as given, **When** a committed test reads
   `GET /api/health`, **Then** it asserts that `databases` is those four in that order,
   each with that `name`, the `catalog` `hip_main`, `hip_main`, `bsc5` and `bsc5`, and
   the `min_fov` and `max_fov` 4.5 and 7, 10 and 30, 20 and 30, 35 and 50; that the
   second's `stars` is 42212 and `patterns` 1010981; that the third's `stars` is 9096;
   and that `database` has the `min_fov` 4.5 and the `max_fov` 7, the first database's.
5. **Given** an `AppState` over cedar's database and its samples with narrow and cedar's
   named `narrow` and `cedar` as given, **When** a committed test uploads
   `orion_belt.jpg`, **Then** it asserts that the answer's `status` is `match_found` and
   its `database` is `cedar`.
6. **Given** an `AppState` built with `AppState::shared` over cedar's database and its
   samples, and no `with_databases`, **When** a committed test uploads `orion_belt.jpg`,
   **Then** it asserts that the answer's `status` is `match_found` and its `database` is
   `hip_main`.
7. **Given** an `AppState` over cedar's database with narrow and cedar's named `narrow`
   and `cedar` as given, **When** a committed test uploads a black frame, **Then** it
   asserts that the answer's `status` is `too_few` and its `database` is `narrow`: the
   tries stopped at the first.

---

### User Story 2 - The live loop uses the database whose range holds the field (Priority: P1)

As the observer, I change the finder's lens mid-night, and within a few frames the
console solves with the database built for the new field, and says so.

**Why this priority**: This is the owner's aim: the console uses the database that fits
the camera in front of it, with nothing to set.

**Independent Test**: `crates/finder/tests/console/databases/field.rs`, new, beside
US1's module. The `test` gate's
`cargo nextest run --workspace --all-targets --all-features --locked` runs it; during
development, `cargo nextest run -p finder --locked --test console databases::field::`.

**Acceptance Scenarios**:

1. **Given** the live rig at 12° built with `crate::support::live::state` and no
   `with_databases`, **When** a committed test takes a report and the session, **Then**
   it asserts that the report solved with `database` `hip_main`; that the session's
   `field.database` is
   `{"name":"hip_main","min_fov_deg":10.0,"max_fov_deg":30.0,"reason":"only"}`; that
   `field.databases` is that one entry without `reason`; and that `db_min_fov_deg` and
   `db_max_fov_deg` are 10 and 30.
2. **Given** the live rig at 12° with cedar's and narrow named `cedar` and `narrow` as
   given, **When** a committed test takes a report and the session, **Then** it asserts
   that the report solved with `database` `cedar`; that `field.fov_deg` is 12 within
   0.1; that `field.database` names `cedar` with the reason `field`; and that
   `field.databases` is `cedar` 10–30 and then `narrow` 4.5–7.
3. **Given** the live rig at 5° with cedar's and narrow named `cedar` and `narrow` as
   given, **When** a committed test takes a report and the session, **Then** it asserts
   that the report solved with `database` `narrow`; that `field.fov_deg` is 5 within
   0.1; that `field.database` is
   `{"name":"narrow","min_fov_deg":4.5,"max_fov_deg":7.0,"reason":"field"}`; and that
   `db_min_fov_deg` and `db_max_fov_deg` are 4.5 and 7.
4. **Given** the live rig at 12° with mid and wide named `mid` and `wide` as given,
   **When** a committed test takes a report and the session, **Then** it asserts that
   the report's `status` is `NoMatch` and its `database` is `wide`, the last tried; that
   `field.database` names `mid` with the reason `first`; and that `field.fov_deg` is
   null.
5. **Given** the live rig at 12° with cedar's and narrow named `cedar` and `narrow` as
   given, the field after the sixth 5°, **When** a committed test takes sixteen reports,
   each with its session, **Then** it asserts that reports 1 to 6 solved with `cedar`
   at 12 within 0.1, the session naming `cedar` with the reason `field`; that reports 7
   to 9 did not solve, the session still naming `cedar`; and that reports 10 to 16
   solved with `narrow` at 5 within 0.1, the session naming `narrow` with the reason
   `field`.
6. **Given** the live rig at 5° with cedar's and narrow named `cedar` and `narrow` as
   given, the field after the sixth 12°, **When** a committed test takes sixteen reports,
   each with its session, **Then** it asserts that reports 1 to 6 solved with `narrow` at
   5 within 0.1, the session naming `narrow` with the reason `field`; that reports 7 to 9
   did not solve, the session still naming `narrow`; that report 10 solved with `narrow`
   at 12 within 0.1, the session then naming `cedar` with the reason `field`; and that
   reports 11 to 16 solved with `cedar` at 12 within 0.1.
7. **Given** the live rig at 12° with narrow and cedar's named `narrow` and `cedar` as
   given, narrow first, **When** a committed test takes sixteen reports, each with its
   session, **Then** it asserts that report 1 solved with `narrow` at 12 within 0.1, the
   session then naming `cedar` with the reason `field`, and that reports 2 to 16 solved
   with `cedar` at 12 within 0.1, the session naming `cedar` with the reason `field`.
8. **Given** the live rig at 12° with narrow and wide named `narrow` and `wide` as given,
   **When** a committed test takes a report and the session, **Then** it asserts that
   the report solved with `database` `narrow`; that `field.fov_deg` is 12 within 0.1; and
   that `field.database` is
   `{"name":"narrow","min_fov_deg":4.5,"max_fov_deg":7.0,"reason":"solved"}`: the field
   lies in no range, so the database that solved it stays.
9. **Given** the live rig at 12° with mid and wide named `mid` and `wide` as given,
   **When** a committed test takes reports, each with its session, until
   `field.unsolved_s` is at least 30, for at most 62 reports, **Then** it asserts that
   `unsolved_s` reached 30; that no report solved; that `field.fov_deg` is null; and that
   `field.database` names `mid` with the reason `first`.

---

### User Story 3 - The page, MCP and the README say which database is in use (Priority: P2)

As the observer with a phone, I open Setup and read which database the console uses and
why; as an MCP client, I read the same from `get_session`; as a future user, I read in
the README how to give the console several databases.

**Why this priority**: US1 and US2 put the facts in the console's answers; this story
puts them where the observer and an agent read them.

**Independent Test**: `npm test` in `crates/finder/web`, a command of the `test` gate,
runs `src/components/FieldDatabases.test.tsx` (US3-S1 to US3-S5) and
`src/healthLine.test.ts` (US3-S6, US3-S7), both new; and
`cargo nextest run --workspace --all-targets --all-features --locked` runs
`crates/finder/tests/mcp/databases.rs` (US3-S8, US3-S9), new. During development,
`npm test -- FieldDatabases healthLine` and
`cargo nextest run -p finder --locked --test mcp databases::`.

**Acceptance Scenarios**:

1. **Given** `SetupPage` rendered with a session whose `field` has `fov_deg` 5, the
   range 4.5–7, `unsolved_s` 0, `database` `narrow` 4.5–7 with the reason `field`, and
   `databases` `cedar` 10–30 and `narrow` 4.5–7, **When** a committed test reads the
   page, **Then** it asserts the texts `Database in use: narrow, the solved field lies in
   its range`, `Databases: cedar 10–30°, narrow 4.5–7°` and `Database range: 4.5–7°`, and
   no alert.
2. **Given** that page with `fov_deg` null and `database` `cedar` 10–30 with the reason
   `first`, **When** a committed test reads it, **Then** it asserts the text
   `Database in use: cedar, the first given; nothing has solved yet`.
3. **Given** the page of US3-S2 with `unsolved_s` 30, **When** a committed test reads it,
   **Then** it asserts an alert reading exactly
   `Frames with at least 10 stars have not solved for 30 s with any database (cedar 10–30°, narrow 4.5–7°). The camera's field may be outside every range; a database built for the camera's field may be needed.`
4. **Given** the page of US3-S1 with `fov_deg` 8 and the reason `solved`, **When** a
   committed test reads it, **Then** it asserts an alert reading exactly
   `The solved field, 8°, is outside every database's range (cedar 10–30°, narrow 4.5–7°). A database built for the camera's field may be needed.`
   and the text `Database in use: narrow, it solved the last frame; its field lies in no range`.
5. **Given** the page with a `field` that has `database` `cedar` 10–30 with the reason
   `only` and `databases` holding only `cedar` 10–30, `fov_deg` 12 and `unsolved_s` 0,
   **When** a committed test reads it, **Then** it asserts the text
   `Database range: 10–30°`, no text starting `Database in use:` or `Databases:`, and no
   alert.
6. **Given** a `Health` with `database` hip_main, 42,212 stars, 10–30°, `version`
   `0.1.0` and no `databases`, **When** a committed test calls `healthLine`, **Then** it
   asserts exactly `Database: hip_main, 42,212 stars, fields 10–30° · finder 0.1.0`.
7. **Given** that `Health` with `databases` `narrow` 4.5–7 and `cedar` 10–30, **When** a
   committed test calls `healthLine`, **Then** it asserts exactly
   `Databases: narrow, fields 4.5–7°; cedar, fields 10–30° · finder 0.1.0`.
8. **Given** the live rig at 12° with cedar's database named `first` and again `second`
   as given, behind the MCP router of `crate::support::mcp::app` with the peer
   `127.0.0.1`, **When** a committed test takes a report, reads `GET /api/session` and
   calls `get_session`, **Then** it asserts that the report solved with `database`
   `first`; that `get_session`'s `structuredContent.field.database` is
   `{"name":"first","min_fov_deg":10.0,"max_fov_deg":30.0,"reason":"field"}`; and that
   its `field.databases` equals the route's.
9. **Given** an `AppState` over cedar's database with cedar's database named `first`
   and again `second` as given, behind the MCP router, **When** a committed test calls
   `solve_frame` with `orion2.jpg` from cedar's `medium_fov` samples in standard
   base64, **Then** it asserts that `structuredContent.status` is `match_found` and
   `structuredContent.database` is `first`.

### Edge Cases

- **An upload during a cold start** waits for the live loop's tries: they hold the solve
  gate together. Read, not run.
- **Two databases with overlapping ranges**: the one that solved stays while its range
  holds the field, so the one in use stays while it solves; otherwise the first given
  whose range holds it. Read from FR-010, not run.
- **A frame with fewer stars than a solve needs** ends the tries at the first database,
  live and uploaded (US1-S7); the three-failure count is 013's, unchanged.
- **A lens change while an estimate is held** costs the three failures of 013's FR-023
  before any other database is tried. A faster switch, trying the others on the first
  failure, would add up to one solve per database to every failed frame, clouds
  included; not taken (plan.md § Decisions).
- **The simulator and the drawn sky** (`--simulate`, `--mount-sky`) take their stars from
  the first database given (FR-007). A narrow database given first makes the desk
  console's sky deeper and its frames denser. Read, not run.
- **The database in use is not kept across restarts.** A console starts with the first
  given and finds the right one on the first frame that solves.
- **`--licenses`** prints and exits before any database is loaded, with any number of
  `--db` (`crates/finder/tests/cli/flags.rs:14`, unchanged).
- **A load that fails** ends the console as today, with
  `cannot load the database PATH: ERROR` for the first file that fails.

## Requirements *(mandatory)*

### Functional Requirements

**Several databases, uploads and health** (US1):

- **FR-001**: `finder` SHALL take `--db PATH` any number of times; with none it SHALL
  load cedar's `default_database.npz` as today. It SHALL load every database at start,
  in the order given (owner decisions 2 and 3).
- **FR-002**: Each database's name SHALL be its file name without the extension. Two
  databases with the same name SHALL be refused at start with an error holding
  `two databases are named NAME`, and a non-zero exit.
- **FR-003**: With one database the start line SHALL be today's
  (`crates/finder/src/main.rs:141`). With several, each SHALL have that line with
  `database NAME:` in place of `database:`, in the order given (owner approval 4).
- **FR-004**: `GET /api/health` SHALL keep `database`, the first database given, and SHALL
  add `databases`: every database in the order given, each with `name`, `catalog`,
  `stars`, `patterns`, `min_fov` and `max_fov`.
- **FR-005**: An upload, `POST /api/solve` or MCP `solve_frame`, SHALL be solved with the
  database in use and then each of the others in the order given until one solves; a
  solve that ends `too_few` or `cancelled` SHALL end the tries. Its report SHALL name in
  `database` the database that solved it, or the last one tried. An upload SHALL NOT
  change the database in use.
- **FR-006**: The module `finder::databases` SHALL hold `Database` (a name and an
  `Arc<PatternDatabase>`) and `Databases` (the list in the order given and the database in
  use), as plan.md § Interfaces gives them. `AppState::with_databases` SHALL set the list;
  `AppState::new` and `AppState::shared` SHALL hold one database named after its
  `star_catalog`, as today's single database.
- **FR-007**: The simulator of `--simulate`, the drawn sky of `--mount-sky` and the
  simulator's star push SHALL use the first database given.
- **FR-008**: With one database, every solve SHALL use it, every key the console answers
  today SHALL keep its value, and the new keys SHALL name that database (owner approval
  3).

**The live loop's choice** (US2):

- **FR-009**: The live loop SHALL solve a frame that has a field estimate with the
  database in use only. A frame without one SHALL be solved with the database in use and
  then each of the others in the order given until one solves; `too_few` or `cancelled`
  SHALL end the tries. 013's FR-023 is unchanged: the estimate, its 5% and its three
  failures are the live loop's, around the whole of one frame's tries (owner decision 1).
- **FR-010**: After a solve, the database in use SHALL be the one that solved if its
  range holds the solved field, otherwise the first given whose range holds it,
  otherwise the one that solved. A range holds a field when `min_fov ≤ field ≤ max_fov` (owner
  decision 1).
- **FR-011**: Once the live loop has received a frame, the session's `field` SHALL keep
  its keys and SHALL add `database`, the database in use with `name`, `min_fov_deg`,
  `max_fov_deg` and `reason`, and `databases`, every database's `name`, `min_fov_deg` and
  `max_fov_deg` in the order given. `db_min_fov_deg` and `db_max_fov_deg` SHALL be the
  range of the database in use. `reason` SHALL be `only` with one database; with several,
  `first` until a frame solves, `field` when the solved field lies in its range, and
  `solved` when it lies in no database's range. A live report's `report.database` SHALL
  name the database that solved the frame, or the last one tried.
- **FR-012**: `Live::start` SHALL keep its signature and behaviour, a fixed range and a
  `field` without `database` or `databases`. `Live::start_with_databases` SHALL start the
  same loop over a `Databases`, and `AppState::with_live` SHALL use it.

**The page, MCP and the README** (US3):

- **FR-013**: With several databases, Setup's field block SHALL show, after
  `Database range:`, `Database in use: NAME, WORDS` and `Databases: NAME MIN–MAX°, …`, the
  words being, for `first`, `the first given; nothing has solved yet`; for `field`,
  `the solved field lies in its range`; and for `solved`,
  `it solved the last frame; its field lies in no range`. With one, the block SHALL be
  today's.
- **FR-014**: The home page's last line SHALL be today's with one database, and with
  several `Databases: NAME, fields MIN–MAX°; … · finder VERSION`.
- **FR-015**: With several databases, Setup's alert SHALL be, after 30 s of unsolved
  frames with at least 10 stars,
  `Frames with at least 10 stars have not solved for 30 s with any database (RANGES). The camera's field may be outside every range; a database built for the camera's field may be needed.`;
  otherwise, when the solved field lies in no database's range,
  `The solved field, FIELD°, is outside every database's range (RANGES). A database built for the camera's field may be needed.`;
  otherwise today's near-edge alert, against the range of the database in use. RANGES is
  the `Databases:` list without its label. With one database the alerts SHALL be
  today's.
- **FR-016**: MCP's `get_session` and `solve_frame` SHALL answer the keys of FR-005 and
  FR-011, as the routes do; no tool's description or schema changes.
- **FR-017**: `crates/finder/web/dist` SHALL be the build of the page's source.
- **FR-018**: `crates/finder/README.md` SHALL say, in the `--db` row of its flag table,
  that `--db` repeats; in the `GET /api/health` row, that `databases` lists every
  database; and in a section `## Several databases` before `## API`, what plan.md
  § Interfaces gives (owner approval 5).

**The tests' databases** (US1):

- **FR-019**: `crates/finder/Cargo.toml`'s `[dev-dependencies]` SHALL gain
  `database-generation` by path, and the test databases SHALL be built with its library
  in `crates/finder/tests/console/databases/mod.rs`, from cedar's database and the
  repository's BSC5, written under `CARGO_TARGET_TMPDIR` under a temporary name and
  renamed when whole, and reused when present (owner decision 4, owner approval 1).
- **FR-020**: Nothing else SHALL change: the scope node, the scope link's protocol,
  `proto/`, the mount, imaging and its ASTAP database, `crates/plate-solver`,
  `crates/pattern-database`, `crates/database-generation`, `crates/simulator`, and every
  existing test.

### Key Entities

- **A database**: one loaded pattern database and its name, the file name without
  `.npz`.
- **The databases**: every database, in the order given; the first is the sky of the
  simulator and the drawn sky.
- **The database in use**: the one the live loop solves with while it holds a field
  estimate, and the first it tries without one; with its reason.
- **A reason**: `only`, `first`, `field` or `solved`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every acceptance scenario has a committed test that asserts it, and the
  `lint` and `test` gates pass.
- **SC-002**: No existing test, golden, `scripts/ci/` file, `.github/` file,
  `CODEOWNERS` entry, `ergane.yaml` or `playwright.config.ts` is edited. Every existing
  test passes unchanged (plan.md § Shapes that existing tests pin).
- **SC-003**: With cedar's and a narrow database, a lens change is solved on the fourth
  live report after it and, from the fifth, with the database whose range holds the new
  field, in both directions (US2-S5, US2-S6), where today the console has one database.
- **SC-004**: With one `--db`, the start line, every solve and every existing key are as
  today (US1-S3, US1-S6, US2-S1).

## Assumptions

- **Earlier features are in place.** Every spec in `depends_on_landed` has landed.
  Every citation here was read at 425b9da0.
- **The simulator stands for the sky.** Its frames' centroids carry the detection of
  rendered frames, but the test databases are small: the narrow one holds only the
  stars around the rig's pointing, so it solves only there. What a real narrow database
  does with real frames is the field check's.
- **A database's range is what it was built for**, not what it can solve (Clarifications).
  The rule uses it to choose.
- **The catalog for the narrow database** is the owner's to download (plan.md § Outside
  the factory). The generator reads CDS catalog I/239's `tyc_main.dat` (Tycho-1) in its
  own format (`crates/database-generation/src/catalog.rs:181`); Tycho-2 would need a
  conversion first. Read, not run.
- **Out of scope:**
  - a stated field or focal length for the finder (owner decision 1 (c));
  - loading databases on demand (owner decision 3 (b));
  - keeping the database in use across restarts;
  - the imaging solver's database;
  - making `database-generation` faster for narrow fields (plan.md § Risks);
  - editing `openspec/specs/finder-console/spec.md`.
- **Outside the factory:** the narrow database, the Pi's deploy and the field check, in
  plan.md § Outside the factory, by the owner. A new field kit's Pi binary is needed once
  047 lands.
- **The factory's gates and CI:** the gates are `lint` and `test`, copied in plan.md
  § Verification. CI's other jobs run only in the merge queue, so no scenario's Then
  clause rests on them.

## Work Graph

```yaml
US1:
  depends_on: []
  implements: [FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007, FR-008, FR-019, FR-020]
US2:
  depends_on: []
  depends_on_merged: [US1]
  implements: [FR-009, FR-010, FR-011, FR-012]
US3:
  depends_on: []
  depends_on_merged: [US2]
  implements: [FR-013, FR-014, FR-015, FR-016, FR-017, FR-018]
```

US2 merges after US1: it builds on `finder::databases` and adds its test module to US1's
`crates/finder/tests/console/databases/mod.rs`. US3 merges after US2: the page reads the
keys US2 adds, and its MCP test reads them through `get_session`.
