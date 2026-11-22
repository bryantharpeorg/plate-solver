# Implementation Plan: The Console Uses the Database That Fits

**Branch**: `047-console-uses-the-database-that-fits` | **Date**: 2026-11-21 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

The finder console learns to hold several pattern databases. `--db` repeats; every
database is loaded at start and named by its file name. A new module,
`finder::databases`, holds them in the order given and the one in use. Uploads try the
one in use and then the others until one solves (US1). The live loop does the same on a
frame with no field estimate, and after each solve moves to the database whose range
holds the solved field; with an estimate it solves with the one in use alone, as today
(US2). The session's `field`, a solve report, `GET /api/health`, Setup, the home page's
last line and MCP name the database in use and why, and the finder README says how to
give several (US3). With one `--db` nothing a user sees changes. The tests build their
small databases with `database-generation`'s library from stars already in the
repository.

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI; TypeScript and React for the page.

**Primary Dependencies**: `database-generation` (a workspace crate) as a dev-dependency
of the finder (US1; spec owner approval 1). No new crate, and no npm package.

**Storage**: none. The test databases are written under `CARGO_TARGET_TMPDIR`.

**Testing**:
- US1: `crates/finder/tests/cli/databases.rs` (new), the built `finder`;
  `crates/finder/tests/console/databases/mod.rs` (new, the test databases and no test)
  and `crates/finder/tests/console/databases/several.rs` (new), in process.
- US2: `crates/finder/tests/console/databases/field.rs` (new), the live loop on the
  simulator under `#[tokio::test(start_paused = true)]`.
- US3: `crates/finder/web/src/components/FieldDatabases.test.tsx` and
  `crates/finder/web/src/healthLine.test.ts` (new, Vitest), and
  `crates/finder/tests/mcp/databases.rs` (new).

**Target Platform**: the finder console, on the observer's machine or the Pi.

**Constraints**:
- `AppState::new`, `AppState::shared`, `AppState::with_live_solver`, `Live::start`,
  `DbSolver`, `Session`'s public methods and `FieldView`'s existing fields keep their
  shapes; existing tests call them (§ Shapes that existing tests pin).
- No file under `crates/finder/tests/support/` is added: the layout test lists them
  (`crates/finder/tests/layout/main.rs:126`).
- A test module's name holds only lowercase letters and underscores
  (`crates/finder/tests/layout/main.rs:155`).
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, rustdoc with warnings denied.

**Scale/Scope**: three stories; one new library module, six changed source files, three
changed page files and one new, the README, six new test files.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | The solver, star detection, `crates/math-core`, `crates/pattern-database` and `crates/database-generation` are not changed; each try is today's `solve_centroids` or `solve_frame_with_frame` against one database. Holding several databases and choosing by field is console work, grounded in cedar's own practice of a database per field range (`reference-solutions/docs/05-database-generation.md`, § 1, the catalogs and the fields each is good for) and in what the probe of 2026-11-21 ran; what rests on reading is listed in § What was run and what was read. |
| II. Never weaken a gate | No existing test, golden, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. Every new test is in a new file; the edits to existing test files are one `mod` line each in `crates/finder/tests/cli/main.rs`, `crates/finder/tests/console/main.rs` and `crates/finder/tests/mcp/main.rs`, as the constitution's test-layout rule asks. § Shapes that existing tests pin names every existing test file beside the change. |
| III. Tests first, independent expectations | The oracles are cedar's database's own properties (42,212 stars, 1,010,981 patterns, 10–30°), the BSC5's 9,096 stars, the ranges the test builds its databases with, the names it gives them, the simulator's field (0.1°), the report at which the test changes it, and the scenarios' own words. No test reads a reason, a range or a word from the code under test. Time is paused Tokio time on the simulator's clock. |
| IV. Critical paths | One dependency change: `database-generation` as a dev-dependency of the finder, by path, and one line of `Cargo.lock` (spec owner approval 1). `crates/pattern-database` and `crates/database-generation`, the format's compatibility contract, are read and called, never edited, so the database stays memory-mappable as the
constitution's product limits ask. No `unsafe`, nothing under `proto/` or `crates/scope-link`, no `build.rs`. |
| V. Proven open source first, honest licensing | Nothing here could be adopted: choosing among the console's own databases is a few lines over the solver the repository already has. cedar-solve and tetra3 each load one database a solver (`reference-solutions/cedar-solve/tetra3/tetra3.py`, `load_database`) and leave the choice to the caller; ASTAP, the imaging solver, picks its own database by name (`--astap-db-name`). The test databases are built with the repository's own generator from the vendored BSC5 (Yale, `reference-solutions/cedar-solve/tests/data/CREDITS.md`) and cedar's database's stars, both already in the tree; no new data, no new license. |

**Mount and optics neutrality.** Nothing here names a lens, a camera or a mount. The
choice reads only each database's own `min_fov` and `max_fov` and the solved field, so
any optics, today's lens or a future refractor's finder, choose the same way.

## Project Structure

```text
crates/finder/
├── Cargo.toml                                 # US1: database-generation in [dev-dependencies]
├── README.md                                  # US3: the --db and health rows, § Several databases
├── src/
│   ├── databases.rs                           # US1 new: Database, Databases, uploads; US2: the choice, DatabasesSolver
│   ├── lib.rs                                 # US1: pub mod databases; the health line of the docs
│   ├── main.rs                                # US1: --db repeated, the start lines, Databases::new
│   ├── api.rs                                 # US1: databases, with_databases, health, uploads; US2: with_live
│   ├── solve.rs                               # US1: SolveReport::database
│   ├── live.rs                                # US2: Range, start_with_databases
│   └── session.rs                             # US2: FieldView's database and databases
├── tests/
│   ├── cli/main.rs, cli/databases.rs          # US1: one mod line; new
│   ├── console/main.rs                        # US1: one mod line
│   ├── console/databases/mod.rs               # US1 new (the test databases); US2: one mod line
│   ├── console/databases/several.rs           # US1 new
│   ├── console/databases/field.rs             # US2 new
│   └── mcp/main.rs, mcp/databases.rs          # US3: one mod line; new
└── web/
    ├── src/api.ts                             # US3: DatabaseView, field and Health keys
    ├── src/App.tsx                            # US3: the last line through healthLine
    ├── src/healthLine.ts, healthLine.test.ts  # US3 new
    ├── src/components/FieldWarning.tsx        # US3: the lines and alerts
    ├── src/components/FieldDatabases.test.tsx # US3 new
    └── dist/                                  # US3: rebuilt
Cargo.lock                                     # US1: one line
```

## Interfaces

### `crates/finder/src/databases.rs` (US1, new; US2 adds the choice)

US1 writes the module as below, with `Reason` holding `Only` and `First` only, and
without `order`'s estimate, `solved`, `solve_centroids` and `DatabasesSolver`. US2 adds
`Solved` and `Field`, the estimate in `order`, `solved`, `solve_centroids` and
`DatabasesSolver`. The probe's trial built it whole, as given here.

```rust
//! The pattern databases the console holds, and the one it solves with (047).
use crate::live::{SolveFuture, Solver};
use crate::solve::{self, SolveOptions, Status};
use pattern_database::PatternDatabase;
use scope_link::proto::StarFrame;
use serde::Serialize;
use std::sync::{Arc, Mutex};

/// One loaded pattern database and the name the console gives it.
#[derive(Clone)]
pub struct Database {
    /// Its file name without the extension, such as `default_database`.
    pub name: String,
    /// The loaded database.
    pub db: Arc<PatternDatabase>,
}

impl Database {
    /// The database's field range, in degrees.
    pub fn range(&self) -> (f64, f64) {
        (f64::from(self.db.properties.min_fov), f64::from(self.db.properties.max_fov))
    }
    fn view(&self) -> DatabaseView {
        let (min, max) = self.range();
        DatabaseView { name: self.name.clone(), min_fov_deg: min, max_fov_deg: max }
    }
}

/// Why the console solves with the database it uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// It is the only database.
    Only,
    /// Nothing has solved yet: it is the first given.
    First,
    /// It solved the last frame that solved, whose field lies in no database's range (US2).
    Solved,
    /// The last solved field lies in its range (US2).
    Field,
}

/// A database's name and field range.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DatabaseView {
    /// Its name.
    pub name: String,
    /// Its lower field limit, in degrees.
    pub min_fov_deg: f64,
    /// Its upper field limit, in degrees.
    pub max_fov_deg: f64,
}

/// The database in use and why.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InUseView {
    /// Its name and range.
    #[serde(flatten)]
    pub database: DatabaseView,
    /// Why it is in use.
    pub reason: Reason,
}

/// The databases the console holds, in the order they were given, and the one in use.
pub struct Databases {
    list: Vec<Database>,
    in_use: Mutex<(usize, Reason)>,
}

impl Databases {
    /// The console's databases, in the order given. Names must differ.
    pub fn new(list: Vec<Database>) -> Result<Self, String> {
        if list.is_empty() {
            return Err("no database".into());
        }
        for (i, a) in list.iter().enumerate() {
            if list[..i].iter().any(|b| b.name == a.name) {
                return Err(format!("two databases are named {}", a.name));
            }
        }
        let reason = if list.len() == 1 { Reason::Only } else { Reason::First };
        Ok(Self { list, in_use: Mutex::new((0, reason)) })
    }
    /// One database, named after its star catalog.
    pub fn one(db: Arc<PatternDatabase>) -> Self {
        let name = db.properties.star_catalog.clone();
        Self::new(vec![Database { name, db }]).expect("one database")
    }
    /// The first database given: the simulator's and the drawn sky's stars.
    pub fn first(&self) -> &Database { &self.list[0] }
    /// Every database, in the order given.
    pub fn list(&self) -> &[Database] { &self.list }
    /// Each database's name and range.
    pub fn views(&self) -> Vec<DatabaseView> { self.list.iter().map(Database::view).collect() }
    /// The database in use, and why.
    pub fn in_use(&self) -> InUseView {
        let (index, reason) = *self.in_use.lock().expect("databases");
        InUseView { database: self.list[index].view(), reason }
    }
    /// The order of the tries: with an estimate the one in use alone (US2); without,
    /// the one in use and then the others in the order given.
    fn order(&self, estimate: Option<f64>) -> Vec<usize> {
        let (index, _) = *self.in_use.lock().expect("databases");
        if estimate.is_some() {
            return vec![index];
        }
        std::iter::once(index).chain((0..self.list.len()).filter(|&i| i != index)).collect()
    }
    /// After a solve by `index` with the field `fov_deg` (US2): `index` if it holds
    /// the field, else the first given that does, else `index`.
    fn solved(&self, index: usize, fov_deg: f64) {
        if self.list.len() == 1 {
            return;
        }
        let holds = |i: &usize| {
            let (min, max) = self.list[*i].range();
            (min..=max).contains(&fov_deg)
        };
        let mut in_use = self.in_use.lock().expect("databases");
        *in_use = std::iter::once(index)
            .chain(0..self.list.len())
            .find(holds)
            .map_or((index, Reason::Solved), |i| (i, Reason::Field));
    }
    /// The live loop's tries over centroids (US2). Returns the report, its projection
    /// and the name of the database that solved it, or of the last one tried.
    pub fn solve_centroids(
        &self,
        centroids: &[scope_link::proto::Centroid],
        size: (usize, usize),
        options: &SolveOptions,
    ) -> (solve::SolveReport, Option<crate::sky::Frame>, String) { /* as solve_frame, below,
        with solve::solve_centroids, self.order(options.fov), and self.solved(index,
        frame.fov_deg()) before it returns a solve */ }
    /// An upload's tries (US1): the one in use, then the others, until one solves; a
    /// `TooFew` or `Cancelled` ends them. The database in use does not change.
    pub fn solve_frame(
        &self,
        image: &solve::GrayImage,
        options: &SolveOptions,
    ) -> (solve::SolveReport, Option<crate::sky::Frame>, String) {
        let mut last = None;
        for index in self.order(None) {
            let database = &self.list[index];
            let (report, frame) = solve::solve_frame_with_frame(&database.db, image, options);
            if frame.is_some() {
                return (report, frame, database.name.clone());
            }
            let stop = matches!(report.status, Status::TooFew | Status::Cancelled);
            last = Some((report, database.name.clone()));
            if stop {
                break;
            }
        }
        let (report, name) = last.expect("one database");
        (report, None, name)
    }
}

/// The live loop's solver over every database (US2), on the blocking pool as
/// `DbSolver` solves. A task that panics answers `Cancelled`, as `DbSolver` does.
pub struct DatabasesSolver(pub Arc<Databases>);
impl Solver for DatabasesSolver {
    fn solve(&self, frame: StarFrame, options: SolveOptions) -> SolveFuture<'_> { /* as
        DbSolver (`crates/finder/src/live.rs:31`), calling Databases::solve_centroids and
        setting report.database to the name it returns */ }
}
```

`solved` searches from the database that solved, and the tries begin with the one in
use, so two databases with overlapping ranges do not trade places while the one in use
solves (spec Edge Cases).

### `crates/finder/src/solve.rs` (US1)

`SolveReport` gains, after `grid`:

```rust
    /// The name of the database that solved the frame, or that was tried last (047).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
```

and `solve_detected`'s report sets it `None`. Nothing else in the file changes.

### `crates/finder/src/main.rs` (US1)

`db` becomes `Vec<PathBuf>` with the same `default_value`, its doc line
`/// Pattern database (.npz) covering your camera's field of view. Repeat it to hold`
`/// several; the console tries them in the order given until one solves.` The load
(`crates/finder/src/main.rs:137`) becomes a loop over `args.db`: each is loaded as
today, with today's error, named by `file_stem()` (lossy, the path's display if it has
none), and printed with today's line when `args.db.len()` is 1 and with
`database {name}:` in place of `database:` otherwise. `Databases::new(list)?` follows,
and `AppState::new(db, samples)` (`crates/finder/src/main.rs:176`) becomes
`AppState::shared(databases.first().db.clone(), samples).with_databases(databases)`. A
duplicate name returns through `main`'s error, which prints it and exits 1.

### `crates/finder/src/api.rs` (US1; US2 one call)

- `AppState` gains `databases: Arc<crate::databases::Databases>`, documented
  `/// Every database, the first being db (047).`, which `shared` sets to
  `Databases::one(db.clone())`. `db` stays: it is the first database, and every caller of
  `self.db` today (the simulator, the drawn sky, the star push) keeps it (FR-007).
- `pub fn with_databases(mut self, databases: Databases) -> Self` sets `db` to
  `databases.first().db.clone()` and `databases`; documented to be called before
  `with_simulation`, `with_mount_sky` and `with_live`.
- `health` adds `databases: Vec<DatabaseEntry>` after `database`, each entry `name`,
  `catalog` (`star_catalog`), `stars`, `patterns`, `min_fov`, `max_fov`.
- `solve_upload` (`crates/finder/src/api.rs:1513`) clones `databases` in place of `db`
  and calls `databases.solve_frame(&image, &options)`, setting `report.database` from
  its name. The rest of the function is unchanged.
- US2: `with_live` (`crates/finder/src/api.rs:293`) builds
  `DatabasesSolver(self.databases.clone())`, and `with_live_solver` calls
  `Live::start_with_databases(…, self.databases.clone())` in place of `Live::start(…,
  (min, max))`. `with_live_solver`'s signature is unchanged, so a test's injected solver
  runs under the same session reporting.

`crates/finder/src/lib.rs` gains `pub mod databases;` after `pub mod constellation;`, and
its route line becomes
``//! * `GET /api/health`: the first database, every database, and how many samples there are.``

### `crates/finder/src/live.rs` (US2)

A `pub(crate) enum Range { Fixed((f64, f64)), Databases(Arc<Databases>) }` replaces
`Incoming`'s `db_range`. `Live::start` keeps its signature
(`crates/finder/src/live.rs:154`) and calls a private `begin` with `Range::Fixed`;
`pub fn start_with_databases(session, clock, frames, status, solver, gate, databases:
Arc<Databases>)` calls it with `Range::Databases`. `begin` is today's body, and after
`report.name = None;` it calls `session.use_database(&range)`, so the report's snapshot
and the next read name the database chosen for this frame. `DbSolver` is unchanged.

### `crates/finder/src/session.rs` (US2)

`FieldView` gains, after `unsolved_s`:

```rust
    /// The database in use, its range and why (047).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<crate::databases::InUseView>,
    /// Every database's name and range, in the order given (047).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub databases: Vec<crate::databases::DatabaseView>,
```

`LiveField` gains the same two fields and `fn update(&mut self, range: &Range)`: a fixed
range sets `range`; databases set `range` to the in-use database's and `database` to its
`InUseView`. `receive_live_frame` takes `&Range` and builds the field on the first frame
with `update` and, for databases, `databases.views()`. `pub(crate) fn use_database(&self,
range: &Range)` calls `update` on the field when there is one. `Values::view` copies both
fields. A fixed range therefore serializes exactly as today.

### The page (`crates/finder/web/src/`, US3)

- `api.ts`: `export interface DatabaseView { name: string; min_fov_deg: number;
  max_fov_deg: number }`; `ObservingSession.field` gains
  `database?: DatabaseView & { reason: "only" | "first" | "solved" | "field" }` and
  `databases?: DatabaseView[]`; `Health` gains `databases?` with `name`, `catalog`,
  `stars`, `patterns`, `min_fov` and `max_fov`. Optional, so today's fixtures still type.
- `healthLine.ts`: `export function healthLine(health: Health): string`, the words of
  FR-014; `App.tsx`'s footer (`crates/finder/web/src/App.tsx:294`) calls it.
- `FieldWarning.tsx`: `several` is `databases` longer than one. After `Database range:`,
  when `several`, two `quiet` paragraphs, `Database in use: NAME, WORDS` and
  `Databases: RANGES`, RANGES being `NAME MIN–MAX°` joined by `, `. The alert is, in
  order: the 30 s alert, with several the words of FR-015 and with one today's; with
  several, when no database's range holds `fov_deg`, the outside alert; otherwise
  today's near-edge alert (`crates/finder/web/src/components/FieldWarning.tsx:7`). The
  words of FR-013 and FR-015 are copied exactly.
- `dist/` is rebuilt with `npm run build`.

### The README (`crates/finder/README.md`, US3)

The `--db` row (`crates/finder/README.md:29`) ends
`Repeat it to hold several (§ Several databases).`; the health row
(`crates/finder/README.md:224`) becomes
``| `GET /api/health` | The first database's properties, every database's name and properties in `databases`, and how many samples there are. |``.
Before `## API`, with a blank line before and after:

```markdown
## Several databases

A pattern database covers one range of fields: cedar's covers 10–30°, and a camera behind
a guide scope sees about 2°, which needs a database built for it from a deeper catalog.
Give `--db` once for each database the console should hold, in the order it should try
them:

    finder --db narrow_2deg.npz --db default_database.npz

Every database is loaded when the console starts, and each is named by its file name
without `.npz`; two with the same name are refused. While a frame has no field estimate,
on the first frame and after three failed solves in a row, the console tries the
database in use and then each of the others in the order given, until one solves. It
then uses the database whose range holds the solved field: the one that solved if it
does, otherwise the first given that does, otherwise the one that solved. With an estimate it
solves only with the database in use. So when the lens changes, the next three frames
fail against the old field, the fourth solves without an estimate, and from the fifth the
console uses the database that fits the new field. An uploaded frame is tried the same
way and does not change the database in use.

`GET /api/session`'s `field` names the database in use, its range and why in `database`
(`only`; `first`, before anything has solved; `field`, the solved field lies in its
range; or `solved`, it solved a field that lies in no database's range), and lists every
database in `databases`; `db_min_fov_deg` and `db_max_fov_deg` are the range of the one
in use. A solve report names in `database` the database that solved the frame, or the
last one tried. Setup and the home page's last line show them, and MCP's `get_session`
and `solve_frame` answer the same. With one `--db` the console works as before.

The simulator of `--simulate` and the drawn sky of `--mount-sky` take their stars from the
first database given. Every database is held in memory: about 115 MB for cedar's, and
about 1.7 GB for a 2° database built from Tycho (an estimate).
```

The command line in the block is indented four spaces, so the section holds no nested
fence.

## The test databases

`crates/finder/tests/console/databases/mod.rs` (US1) holds them and no test. It runs the
generator's pipeline as `database-generation`'s `run` does
(`crates/database-generation/src/main.rs:153`), with every star kept and proper motion
off:

1. `clean_and_limit(&mut entries, None)`;
2. `enumerate_patterns(&entries, min, max, 150.0, oversampling, 50)`;
3. `build_pattern_catalog(&entries, &patterns, 0.001, false)`;
4. `DatabaseProperties` with `pattern_mode` `edge_ratio`, `hash_table_type`
   `quadratic_probe`, `pattern_size` 4, the catalog's `pattern_bins`,
   `pattern_max_error` 0.001, the range, the scenario's `star_catalog`, `epoch_equinox`
   2000, `epoch_proper_motion` 2000, `verification_stars_per_fov` 150,
   `star_max_magnitude` 0 and the catalog's `num_patterns`;
5. `serialize_to_path` to `CARGO_TARGET_TMPDIR/047-NAME.npz.PID.part`, renamed to
   `047-NAME.npz`; a file already there is reused, so the tests of one target directory
   build each once, and two processes building at once each rename a whole file.

| Name | Stars | Range | Oversampling | `star_catalog` | Probe: stars, patterns, file |
|---|---|---|---|---|---|
| narrow | cedar's, drawn by `sim()` at a 30° field with `field_stars(1e6, 100.0)`, `CatalogId::Hip(index + 1)` | 4.5–7° | 20 | `hip_main` | 966, 71,828, 1.75 MB |
| mid | the repository's BSC5, `parse_bsc5` | 20–30° | 20 | `bsc5` | 9,096, 179,266, 4.54 MB |
| wide | the repository's BSC5 | 35–50° | 100 | `bsc5` | 9,096, 95,949, 2.54 MB |

The module also holds `pub static NARROW`, `MID` and `WIDE` (`LazyLock`, loaded with
`pattern_database::load_from_path`), and `pub fn databases(&[(&str,
&Arc<PatternDatabase>)]) -> Databases`, the names as given. It declares `mod several;`
(US1), and US2 adds `mod field;`. The narrow database holds only the stars around
`sim()`'s pointing (alt 50°, az 120°, `crates/finder/tests/support/live.rs:43`), so it
solves only there: the rig never moves the simulator. Its oversampling of 20 against
cedar's 100 keeps it small; it solved every 4.5–7° frame the probe gave it.

## Memory

**Measured** (run, x86_64): loading cedar's database raised the test process's resident
memory by 119,256 kB (116 MiB). The loader holds every slot of the pattern table as four
`usize` star indices, an `f32` edge and a `u16` hash: 38 bytes a slot on a 64-bit
machine, the Pi 5 included (`crates/pattern-database/src/format.rs:39`,
`crates/pattern-database/src/load.rs:119`). Cedar's 3,032,963 slots make 115.3 MB.

**Estimated** (not run, no catalog downloaded), for a database built for the SV165 and
the ASI120MM Mini, a field of 2.29° by 1.72°, with `--min-fov 2.0 --max-fov 2.6`:

- the generator builds patterns at the fields of its ladder (`fov_ladder`): 2.0° and
  2.6° here, against 10, 14.4, 20.8 and 30° for cedar's;
- the lattice fields at each scale go as `1 / fov²` (`num_fields_for_sky`,
  `crates/database-generation/src/num_fields.rs:10`), and a 2° field of Tycho (about 25
  stars a square degree) holds about as many stars as a 10° field of cedar's hip_main
  (about 1 a square degree), so the patterns a field yields after deduplication are
  taken as cedar's;
- so the patterns scale as Σ 1/fov²: 0.398 against cedar's 0.0182, 21.8 times its
  1,010,981, about 22 million;
- the quadratic table holds the next prime above twice that, about 44 million slots:
  **about 1.7 GB in memory**, and on disk about 0.9 GB uncompressed (star indices above
  65,535 are stored as `u32`) and a few hundred MB as a compressed `.npz`, at cedar's
  ratio of 39%. A linear-probe table would hold three times the patterns, about 2.5 GB.

On an 8 GB Pi: both loaded, about 1.8 GB, leaving about 6 GB. One at a time: a peak of
about 1.7 GB plus a load's transient (the raw arrays and the widened ones together), and
each switch a load during the night. A 1.8–3.0° range would add a third scale, about 34
million patterns and 2.6 GB.

## What was run and what was read

**Run on 2026-11-21, before the draft**, on the branch `probe/047` cut from `main` at
425b9da0 (commits `2917e069`, `e2507035`, `2d942982`: the probe; `979c4460`, `5bdbb294`,
`a7ea6df2`, `bc66d8a0`: the trial). Logs are in the operator's probe folder, not in the
repository.

- **Which database solves which field** (`probe_several::p2_solve_matrix`), on the
  simulator's centroids at its pointing: cedar's solved 5.5° to 45° and not 5°; narrow
  solved 5° to 28° and not 40° or 45°; mid solved 20° to 45° and not 12°; wide solved 28°
  to 45° and not 12° or 25°; at 2.3° each frame held 2 stars and none solved.
- **The four rules over the same frames** (`probe_several::p3_choice_rules`), cedar's and
  narrow in both orders: (a) moved from narrow to cedar's after one frame at 12°, and
  across a lens change both ways within four frames; (b), listed narrow first, kept the
  narrow database at 12° and solved one frame in four; (c) with a stated 12° failed every
  frame after a change to 5°; (d) as one `--db`.
- **(a) and (b) in the real live loop** (`probe_several::p4_live_loop_lens_change`): the
  frames rendered and detected by the simulator's node; (a) gave US2-S5 and US2-S6's
  sequences exactly; (b) gave the same at 12° to 5°, and from 5° to 12° solved reports 10
  and 14 only.
- **Memory**: cedar's load, `probe_several::p1_memory_of_cedar_database`.
- **The trial of all three stories**, as § Interfaces gives them: every Rust scenario of
  US1, US2 and US3 (US1-S1 to S7, US2-S1 to S9, US3-S8 and S9) passes, three runs each
  for the `databases::` tests; with the trial, `cargo nextest run -p finder` passed
  every finder test, 970 of them, and `cargo clippy -p finder --all-targets
  --all-features -- -D warnings`, `cargo fmt --all --check` and
  `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps` passed.
- **The test databases** built in the test from a cold target directory, three processes
  at once, each reusing or renaming a whole file.

**Read, not run**:
- the page's source and tests (US3-S1 to S7): the probe has no `node_modules`, and
  installing one is not allowed here, so Vitest, the typecheck, Prettier, the build and
  Playwright did not run; the trial's page files are written to these words;
- the workspace-wide `lint` and `test` gates: only the finder's commands above ran;
- a narrow database built from Tycho, its memory (§ Memory) and the generator's work on
  it (§ Risks);
- real frames, the ASI120MM, the guide scope, the Pi and its deploy;
- an upload during a cold start, overlapping ranges, and a restart (spec Edge Cases).

## Read from the diff, not from a test

- **The search starts from the database that solved** (FR-010, `solved`). No scenario
  has overlapping ranges. *Check:* `solved` searches from `index` first.
- **An upload does not change the database in use** (FR-005). *Check:* `solve_frame`
  never calls `solved`.
- **The README section and rows.** No test reads them. Copy them from § Interfaces.
- **`Live::start` is unchanged.** Its callers in the tests hold it (§ Shapes).

## Shapes that existing tests pin

Every existing test passes unchanged with the three stories' Rust built (run on the
trial, 970 finder tests). The files beside the change:

- **`crates/finder/tests/console/live/loop_.rs`** and
  **`crates/finder/tests/console/recording.rs`** call `Live::start` with `(10.0, 30.0)`
  (`crates/finder/tests/console/live/loop_.rs:170`,
  `crates/finder/tests/console/live/loop_.rs:273`,
  `crates/finder/tests/console/live/loop_.rs:338`,
  `crates/finder/tests/console/live/loop_.rs:410` and
  `crates/finder/tests/console/recording.rs:386`): its signature stays, with a fixed
  range. `every_frame_for_ten_seconds_solves_at_the_true_eyepiece_and_reports_field`
  (`crates/finder/tests/console/live/loop_.rs:20`) reads `db_min_fov_deg` 10 and
  `db_max_fov_deg` 30 (`crates/finder/tests/console/live/loop_.rs:51`) from a console
  built with `with_live`: with one database those are its range.
  `field_estimate_carries_with_five_percent_allowance_then_three_failures_clear_it`
  (`crates/finder/tests/console/live/loop_.rs:145`) holds 013's FR-023, unchanged.
- **`crates/finder/tests/console/api.rs`**: `health_reports_the_database`
  (`crates/finder/tests/console/api.rs:147`) reads `database` 42,212 stars, 10–30°: it
  stays the first database.
- **`crates/finder/tests/cli/flags.rs`**: `licenses_prints_the_notices_and_exits`
  (`crates/finder/tests/cli/flags.rs:12`) passes `--db no-such-database.npz` with
  `--licenses`: a repeatable flag takes it, and nothing is loaded.
- **The binary's tests that pass `--db` once**: `crates/finder/tests/cli/session.rs:33`,
  `crates/finder/tests/cli/session_rotation.rs:24`, `crates/finder/tests/cli/backlash.rs:25`,
  `crates/finder/tests/cli/aim.rs:102`, `crates/finder/tests/cli/track.rs:30` and
  `crates/finder/tests/console/session/writes.rs:185` wait for `finder console: http://`;
  the start line before it is today's.
- **`crates/finder/tests/mcp/frames.rs`** compares `solve_frame`'s result with the route's
  answer (`crates/finder/tests/mcp/frames.rs:80`): both gain `database` alike.
- **Every `AppState::shared` caller** in the tests gets one database named after its
  `star_catalog`; none reads the new keys.
- **`crates/finder/tests/layout/main.rs`**: the support folder's files are listed
  (`crates/finder/tests/layout/main.rs:126`), so the test databases live in
  `tests/console/databases/`; module names are letters and underscores
  (`crates/finder/tests/layout/main.rs:155`).
- **The page**: `crates/finder/web/src/components/FieldWarning.test.tsx` renders `field`
  without `database` or `databases`, and its words stay; `App.test.tsx`'s health
  (`crates/finder/web/src/App.test.tsx:23`) has no `databases`, so the last line stays
  `Database: hip_main, 42,212 stars, fields 10–30°`. No Playwright test reads either
  (searched). Not run (§ What was run and what was read).
- **US1's test files are existing tests for US2 and US3**, and US2's for US3. US2 adds
  one `mod` line to US1's `crates/finder/tests/console/databases/mod.rs` and no other
  line; US3 writes none of them.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| The solved field decides | Try in turn and keep; a stated field; a named database | Owner decision 1. |
| `--db` repeated | A folder; a list file | Owner decision 2. |
| Every database loaded at start | One at a time | Owner decision 3. |
| Test databases built in the test | Committed fixtures | Owner decision 4. |
| The tries inside one solve of the live loop | Change 013's estimate and failure count | FR-023 and its test stay; the loop sees one solve a frame. |
| Three failures before another database is tried | Try the others on the first failure | Every cloudy frame would cost one solve per database. |
| A `too_few` ends the tries | Try every database anyway | No database solves a frame without enough stars. |
| The range chooses, never refuses | Refuse a database whose range misses the field | Databases solve outside their ranges (spec Clarifications); refusing would lose solves. |
| `Live::start` kept, `start_with_databases` added | Change `Live::start`'s range argument | Five existing tests call it. |
| `AppState::db` kept as the first database | Remove it | The simulator, the drawn sky and the star push read it; FR-007. |
| The name is the file stem, duplicates refused | Number them; use `star_catalog` | Two databases from one catalog are common; a file name is what the user typed. |
| New keys with one database too | Hide them with one | One shape for every client; owner approval 3. |
| The test databases under `tests/console/databases/` | A new support file | The layout test lists the support folder. |
| Three stories | Two | US1 and US2 together measured about 53,000 B, 81 % of 65,536 B, on the trial (§ Plan traps 2). |

## User Story 1 - Several databases, uploads and health

Tests first: `crates/finder/tests/console/databases/mod.rs` and `several.rs`, and
`crates/finder/tests/cli/databases.rs`, with their `mod` lines. Then the dev-dependency;
`databases.rs` without the choice; `SolveReport::database`; `AppState`'s field,
`with_databases`, `health` and `solve_upload`; `main.rs`; `lib.rs`.

## User Story 2 - The live loop's choice

Tests first: `field.rs` and its `mod` line. Then `Reason`'s two values, `order`'s
estimate, `solved`, `solve_centroids` and `DatabasesSolver`; `live.rs`'s `Range` and
`start_with_databases`; `session.rs`'s fields; `with_live`'s two calls.

## User Story 3 - The page, MCP and the README

Tests first: `FieldDatabases.test.tsx`, `healthLine.test.ts` and
`crates/finder/tests/mcp/databases.rs` with its `mod` line. Then `api.ts`, `healthLine.ts`,
`App.tsx` and `FieldWarning.tsx`, `npm run build`, and the README.

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

  Every story runs both, and the judge sees their results.
- **CI in the merge queue** runs every required `ci.yml` job.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** every spec in `depends_on_landed`. Met at 425b9da0.
2. **`main` is still as read.** Every citation here was read at 425b9da0 on 2026-11-21.
   If `main` moves in a file § Project Structure names, or in a test § Shapes names,
   re-read them and re-run the validator.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Not met: all four decisions are open.
4. **Probe builds.** Not met: no probe round has built the stories from this text alone.
   The trial was built alongside the draft.
5. **The page half has run once** on the trial or a probe round: `npm test`, the
   typecheck, Prettier and the build. Not met (§ What was run and what was read).
6. **No operator-staged files are needed.**

## Outside the factory

Written here and not run.

**Building the narrow database**, by the owner:

1. **The catalog** needs the owner's OK to download: CDS catalog I/239, the Hipparcos and
   Tycho Catalogues, `tyc_main.dat` (about 355 MB, 1,058,332 stars), which the
   generator reads in its CDS form (`crates/database-generation/src/catalog.rs:181`) and
   recognises by the `tyc` in its name (`crates/database-generation/src/main.rs:267`).
   Tycho-2 (I/259, about 2.5 million stars) is not in a form the generator reads and
   would need converting to its eight-column form first
   (`crates/database-generation/src/catalog.rs:151`).
2. **The command**, for the SV165 (120 mm) and the ASI120MM Mini (1280 by 960 at
   3.75 µm), a field of 2.29° by 1.72° at 6.4″ a pixel:

   ```bash
   cargo run --release -p database-generation -- tyc_main.dat narrow_sv165.npz \
     --min-fov 2.0 --max-fov 2.6 --epoch-proper-motion 2026.9
   ```

   About 22 million patterns and 1.7 GB in memory once loaded (§ Memory, an estimate).
   The generator's run is long (§ Risks); run it on the desk machine, not the Pi.
3. **The check against simulated frames**, before it goes to the Pi: a local test, not
   committed, as the probe's `p2_solve_matrix` does: a simulator whose sky is the new
   database, its optics set to 2.29°, frames at a few pointings solved with it alone,
   each `match_found` with its field within 0.05° of 2.29°; and the same frames
   against cedar's, which should fail.

**The Pi's deploy**: every database is copied beside the binaries, and the console's
unit names each with `--db`, the one for the optics fitted first, for example
`--db /opt/finder/narrow_sv165.npz --db /opt/finder/default_database.npz` with the guide
scope fitted. The order matters only for a cold start's first try and for the
simulator's sky. The Pi's memory then holds both (§ Memory).

**The field check**, by the owner, with a console built from a commit with 047, both
databases installed, when the ASI120MM and the guide scope arrive:

1. Start the console with both databases, the guide scope's first, the 16 mm lens on.
   Setup should name `default_database` with `the solved field lies in its range`
   within a few frames, the field about 17°.
2. Swap the lens for the guide scope without stopping the console. Within about five
   frames Setup should name the narrow database, the field about 2.3°.
3. Swap back: the same, back to `default_database`.
4. With the cap on, Setup should say after 30 s that frames have not solved with any
   database only if the frames hold at least 10 stars; with the cap on they will not, so
   the warning should stay away.

**A new field kit's Pi binary is needed** once 047 lands.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* each story's diff stays under 64 KiB, measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`. Measured on the
   trial's commits and split by the files each story writes: US1 about 31,000 B (47 % of
   65,536 B), US2 about 22,000 B (34 %), US3 about 17,100 B (26 %); all three together
   69,928 B. At the 1.35 times a factory build has run over its probe on earlier specs,
   US1 would be about 41,900 B (64 %), US2 29,700 B (45 %) and US3 23,100 B (35 %).
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/047-console-uses-the-database-that-fits/` or `.specify/`.
4. **An earlier story's test file is an existing test.** *Check:* US2 adds one `mod` line
   to US1's `mod.rs` and nothing else; US3 writes no file of US1's or US2's tests.
5. **No new support file.** *Check:* nothing new under `crates/finder/tests/support/`.
6. **An oracle copied from the code proves nothing.** *Check:* the tests write the
   reasons, the words, the ranges and the counts as literals.
7. **A negative assertion passes when nothing happens.** *Check:* "did not solve" in
   US2-S5 and S6 stands beside the solves before and after it; "no alert" in US3-S1 and
   S5 beside the texts asserted.
8. **The test databases are built once.** *Check:* written under a temporary name and
   renamed; reused when present; never under `/tmp` or the source tree.
9. **The rig never moves the simulator.** *Check:* no `push_to` in US2's tests; the
   narrow database holds only the stars around `sim()`'s pointing.
10. **The dist is rebuilt.** *Check:* US3 commits `crates/finder/web/dist` after
    `npm run build`.
11. **An unused helper fails clippy.** *Check:* each test file holds only the helpers its
    own tests use; the databases module's three statics are all used by US1.
12. **rustdoc denies warnings.** *Check:* no doc comment holds an angle bracket outside
    backticks.
13. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
14. **The lockfile moves once.** *Check:* US1's `Cargo.lock` gains `database-generation`
    in the finder's list and nothing else; every cargo command runs with `--locked` after
    it.

## Risks

| Risk | Mitigation |
|---|---|
| A cold start with several databases costs a solve per database on a frame that none solves, up to the 5 s timeout each, holding the solve gate. | List the database for the fitted optics first (§ Outside the factory); a `too_few` frame stops at the first. |
| The generator's work for a 2° Tycho database is large: every lattice field scans every pattern star (`crates/database-generation/src/patterns.rs:76`, `crates/database-generation/src/patterns.rs:81`), about 2.6 million fields by about a million stars, some 10¹² distance checks against about 5 × 10⁹ for cedar's (estimates). | Outside the factory, on the desk machine. A faster enumeration would be a change to `crates/database-generation`, a critical path, and a spec of its own. |
| A 1.7 GB database takes a while to load at start. | Once per start; the Pi loads it before the console serves. |
| A test database built at oversampling 20 has gaps a real one would not. | The rig's frames solved on every run; the narrow one is checked only at its own pointing. |
| US1 at a factory build's usual overrun reaches about 64 % of the budget. | It stays well under the 64 KiB refusal (§ Plan traps 2); the cases are the bulk. |
| The page half has not run. | Prerequisite 5. |

## Refinement Record

Drafted on 2026-11-21 by a plate-solver spec session against `main` at 425b9da0, from a
probe and a trial that day, a reading of the finder's console, its tests, the pattern
database and its generator, 013's and 016's trios and the constitution. Writes come from
the task text. Shared writes name the merge edge that orders them, and the story that
last wrote the same file on `main`.

The four owner decisions of spec.md are open. The record is written for their
recommended answers, and its `open_questions` are empty so that the validator checks the
rest; the decisions stay open in spec.md § Owner decisions until the owner answers them,
and a different answer reopens US2 (decision 1) or US1 (decisions 2 to 4). This record
approves nothing: the owner approvals of spec.md are given by the flip.

```yaml
version: 1
reviewed_revision: 425b9da0
stories:
  US1:
    tests:
      - path: crates/finder/tests/cli/databases.rs
        status: new
        disposition: add
        assertion: "finder with --db a copy of cedar's file named copy.npz and then cedar's own: start lines database copy: 42212 stars, 1010981 patterns, FOV 10-30 deg, loaded in and database default_database: 42212 stars; health's databases names copy then default_database; the first has 42212 stars, the second min_fov 10"
        decision: "spec.md:US1-S1"
      - path: crates/finder/tests/cli/databases.rs
        status: new
        disposition: add
        assertion: "finder with --db cedar's file twice exits non-zero, its stderr holding two databases are named default_database"
        decision: "spec.md:US1-S2"
      - path: crates/finder/tests/cli/databases.rs
        status: new
        disposition: add
        assertion: "finder with --db cedar's file once: exactly one start line starts with database, and it starts database: 42212 stars, 1010981 patterns, FOV 10-30 deg, loaded in"
        decision: "spec.md:US1-S3"
      - path: crates/finder/tests/console/databases/several.rs
        status: new
        disposition: add
        assertion: "health over narrow, cedar, mid and wide lists them in that order with catalogs hip_main, hip_main, bsc5, bsc5 and ranges 4.5-7, 10-30, 20-30, 35-50; cedar has 42212 stars and 1010981 patterns, mid 9096 stars; database is the first's, 4.5 to 7"
        decision: "spec.md:US1-S4"
      - path: crates/finder/tests/console/databases/several.rs
        status: new
        disposition: add
        assertion: "an upload of orion_belt.jpg over narrow then cedar answers match_found with database cedar"
        decision: "spec.md:US1-S5"
      - path: crates/finder/tests/console/databases/several.rs
        status: new
        disposition: add
        assertion: "an upload of orion_belt.jpg to AppState::shared over cedar's database alone answers match_found with database hip_main"
        decision: "spec.md:US1-S6"
      - path: crates/finder/tests/console/databases/several.rs
        status: new
        disposition: add
        assertion: "an upload of a 64 by 64 black PNG over narrow then cedar answers too_few with database narrow: the tries stop at the first"
        decision: "spec.md:US1-S7"
      - path: crates/finder/tests/console/api.rs
        status: existing
        disposition: preserve
        assertion: "health reports database with 42212 stars, min_fov 10 and max_fov 30, and 21 samples"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/cli/flags.rs
        status: existing
        disposition: preserve
        assertion: "finder --licenses --db no-such-database.npz prints the notices and exits 0 without loading a database"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/mcp/frames.rs
        status: existing
        disposition: preserve
        assertion: "solve_frame's structured result equals the route's answer for the same frame"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/layout/main.rs
        status: existing
        disposition: preserve
        assertion: "the support folder holds exactly its six files, and every test binary's main.rs only declares modules named in lowercase letters and underscores"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "with one --db every solve uses it, the start line is today's, and every key the console answers keeps its value (FR-008)"
      - "AppState::new, AppState::shared, AppState::with_live_solver, Live::start and DbSolver keep their signatures"
      - "the simulator, the drawn sky and the star push use the first database (FR-007)"
      - "crates/pattern-database and crates/database-generation are not edited"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test cli databases::"
        - "cargo nextest run -p finder --locked --test console databases::"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - Cargo.lock
      - crates/finder/Cargo.toml
      - crates/finder/src/api.rs
      - crates/finder/src/databases.rs
      - crates/finder/src/lib.rs
      - crates/finder/src/main.rs
      - crates/finder/src/solve.rs
      - crates/finder/tests/cli/databases.rs
      - crates/finder/tests/cli/main.rs
      - crates/finder/tests/console/databases/mod.rs
      - crates/finder/tests/console/databases/several.rs
      - crates/finder/tests/console/main.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/api.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and changes with_live and with_live_solver after US1's databases field"
      - path: crates/finder/src/databases.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds the choice to the module US1 creates"
      - path: crates/finder/tests/console/databases/mod.rs
        other_spec: 047-console-uses-the-database-that-fits
        other_story: US2
        disposition: merge_order
        reason: "US2 depends_on_merged US1, and adds one mod line to the module US1 creates"
      - path: crates/finder/src/api.rs
        other_spec: 042-imaging-run-page-and-mcp
        other_story: US10
        disposition: merge_order
        reason: "047 depends_on_landed 042, whose US10 last changed api.rs (c5f30038)"
      - path: crates/finder/src/main.rs
        other_spec: 041-imaging-run-on-the-rig
        other_story: US8
        disposition: merge_order
        reason: "047 depends_on_landed 041, whose US8 last changed main.rs (61b54a47)"
      - path: crates/finder/src/lib.rs
        other_spec: 037-imaging-run
        other_story: US3
        disposition: merge_order
        reason: "047 depends_on_landed 037, whose US3 last changed lib.rs (581be8c3)"
      - path: crates/finder/src/solve.rs
        other_spec: 020-live-chart-calm
        other_story: US1
        disposition: merge_order
        reason: "047 depends_on_landed 020, whose US1 last changed solve.rs (25084233)"
      - path: crates/finder/Cargo.toml
        other_spec: 016-mcp
        other_story: US3
        disposition: merge_order
        reason: "047 depends_on_landed 016, whose US3 last changed the finder's Cargo.toml among spec stories (6a50c046); #196 changed it since, outside any spec"
      - path: Cargo.lock
        other_spec: 030-imaging-camera-link
        other_story: US6
        disposition: merge_order
        reason: "Cargo.lock was last changed by 030's US6 (58aff47c), landed"
      - path: crates/finder/tests/cli/main.rs
        other_spec: 041-imaging-run-on-the-rig
        other_story: US8
        disposition: merge_order
        reason: "047 depends_on_landed 041, whose US8 last changed cli/main.rs (61b54a47)"
      - path: crates/finder/tests/console/main.rs
        other_spec: 042-imaging-run-page-and-mcp
        other_story: US11
        disposition: merge_order
        reason: "047 depends_on_landed 042, whose US11 last changed console/main.rs (7164e87f)"
    open_questions: []
  US2:
    tests:
      - path: crates/finder/tests/console/databases/field.rs
        status: new
        disposition: add
        assertion: "the live rig at 12 deg over cedar's database alone: the report solves with database hip_main; field.database is hip_main 10-30 with reason only; databases is that one entry; db_min_fov_deg 10 and db_max_fov_deg 30"
        decision: "spec.md:US2-S1"
      - path: crates/finder/tests/console/databases/field.rs
        status: new
        disposition: add
        assertion: "12 deg over cedar then narrow: the report solves with cedar; fov_deg 12 within 0.1; database cedar with reason field; databases cedar 10-30 then narrow 4.5-7"
        decision: "spec.md:US2-S2"
      - path: crates/finder/tests/console/databases/field.rs
        status: new
        disposition: add
        assertion: "5 deg over cedar then narrow: the report solves with narrow; fov_deg 5 within 0.1; database narrow 4.5-7 with reason field; db_min_fov_deg 4.5 and db_max_fov_deg 7"
        decision: "spec.md:US2-S3"
      - path: crates/finder/tests/console/databases/field.rs
        status: new
        disposition: add
        assertion: "12 deg over mid then wide: the report is NoMatch with database wide, the last tried; database mid with reason first; fov_deg null"
        decision: "spec.md:US2-S4"
      - path: crates/finder/tests/console/databases/field.rs
        status: new
        disposition: add
        assertion: "cedar then narrow, 12 deg and 5 deg after the sixth report: reports 1 to 6 solve with cedar at 12, in use cedar for field; 7 to 9 do not solve, still cedar; 10 to 16 solve with narrow at 5, in use narrow for field"
        decision: "spec.md:US2-S5"
      - path: crates/finder/tests/console/databases/field.rs
        status: new
        disposition: add
        assertion: "cedar then narrow, 5 deg and 12 deg after the sixth report: 1 to 6 solve with narrow at 5; 7 to 9 do not solve, still narrow; 10 solves with narrow at 12 and the session then names cedar for field; 11 to 16 solve with cedar at 12"
        decision: "spec.md:US2-S6"
      - path: crates/finder/tests/console/databases/field.rs
        status: new
        disposition: add
        assertion: "narrow then cedar at 12 deg: report 1 solves with narrow at 12 and the session names cedar for field; reports 2 to 16 solve with cedar at 12, cedar for field"
        decision: "spec.md:US2-S7"
      - path: crates/finder/tests/console/databases/field.rs
        status: new
        disposition: add
        assertion: "narrow then wide at 12 deg: the report solves with narrow; fov_deg 12 within 0.1; database narrow 4.5-7 with reason solved, the field lying in no range"
        decision: "spec.md:US2-S8"
      - path: crates/finder/tests/console/databases/field.rs
        status: new
        disposition: add
        assertion: "mid then wide at 12 deg, at most 62 reports: unsolved_s reaches 30; no report solves; fov_deg null; database mid with reason first"
        decision: "spec.md:US2-S9"
      - path: crates/finder/tests/console/live/loop_.rs
        status: existing
        disposition: preserve
        assertion: "every frame for ten seconds solves at the true eyepiece and the field reads 12 deg with db_min_fov_deg 10 and db_max_fov_deg 30; the estimate carries with 5 percent and three failures clear it; Live::start takes a fixed range"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/console/recording.rs
        status: existing
        disposition: preserve
        assertion: "a recording's live loop started with Live::start and the range 10 to 30 records as before"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "013's FR-023: the estimate with 5 percent and its three failures are the live loop's, unchanged"
      - "Live::start keeps its signature and a field without database or databases"
      - "with one database the field's keys keep their values, and the new ones name it with reason only"
      - "US1's crates/finder/tests/console/databases/several.rs and crates/finder/tests/cli/databases.rs pass unchanged; mod.rs gains only mod field"
    checks:
      development:
        - "cargo nextest run -p finder --locked --test console databases::field::"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/finder/src/api.rs
      - crates/finder/src/databases.rs
      - crates/finder/src/live.rs
      - crates/finder/src/session.rs
      - crates/finder/tests/console/databases/field.rs
      - crates/finder/tests/console/databases/mod.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/src/live.rs
        other_spec: 022-altaz-guidance
        other_story: US3
        disposition: merge_order
        reason: "047 depends_on_landed 022, whose US3 last changed live.rs (20ce2fe1)"
      - path: crates/finder/src/session.rs
        other_spec: 040-stack-waits-for-steady
        other_story: US6
        disposition: merge_order
        reason: "047 depends_on_landed 040, whose US6 last changed session.rs (f1847647)"
    open_questions: []
  US3:
    tests:
      - path: crates/finder/web/src/components/FieldDatabases.test.tsx
        status: new
        disposition: add
        assertion: "Setup with narrow in use for field at 5 deg among cedar and narrow: texts Database in use: narrow, the solved field lies in its range, Databases: cedar 10-30°, narrow 4.5-7° and Database range: 4.5-7°, and no alert"
        decision: "spec.md:US3-S1"
      - path: crates/finder/web/src/components/FieldDatabases.test.tsx
        status: new
        disposition: add
        assertion: "Setup with cedar in use for first and no solve: the text Database in use: cedar, the first given; nothing has solved yet"
        decision: "spec.md:US3-S2"
      - path: crates/finder/web/src/components/FieldDatabases.test.tsx
        status: new
        disposition: add
        assertion: "Setup after 30 s unsolved with two databases: the alert reads exactly Frames with at least 10 stars have not solved for 30 s with any database (cedar 10-30°, narrow 4.5-7°). The camera's field may be outside every range; a database built for the camera's field may be needed."
        decision: "spec.md:US3-S3"
      - path: crates/finder/web/src/components/FieldDatabases.test.tsx
        status: new
        disposition: add
        assertion: "Setup with a solved field of 8 deg in no range, narrow in use for solved: the alert reads exactly The solved field, 8°, is outside every database's range (cedar 10-30°, narrow 4.5-7°). A database built for the camera's field may be needed. and the text Database in use: narrow, it solved the last frame; its field lies in no range"
        decision: "spec.md:US3-S4"
      - path: crates/finder/web/src/components/FieldDatabases.test.tsx
        status: new
        disposition: add
        assertion: "Setup with one database, cedar for only, at 12 deg: the text Database range: 10-30°, no text starting Database in use: or Databases:, and no alert"
        decision: "spec.md:US3-S5"
      - path: crates/finder/web/src/healthLine.test.ts
        status: new
        disposition: add
        assertion: "healthLine with one database and no databases returns exactly Database: hip_main, 42,212 stars, fields 10-30° · finder 0.1.0"
        decision: "spec.md:US3-S6"
      - path: crates/finder/web/src/healthLine.test.ts
        status: new
        disposition: add
        assertion: "healthLine with databases narrow 4.5-7 and cedar 10-30 returns exactly Databases: narrow, fields 4.5-7°; cedar, fields 10-30° · finder 0.1.0"
        decision: "spec.md:US3-S7"
      - path: crates/finder/tests/mcp/databases.rs
        status: new
        disposition: add
        assertion: "the live rig at 12 deg over cedar's database named first and second behind the MCP router: the report solves with first; get_session's field.database is first 10-30 with reason field; its databases equal the route's"
        decision: "spec.md:US3-S8"
      - path: crates/finder/tests/mcp/databases.rs
        status: new
        disposition: add
        assertion: "solve_frame with orion2.jpg in base64 over cedar's database named first and second answers match_found with database first"
        decision: "spec.md:US3-S9"
      - path: crates/finder/web/src/components/FieldWarning.test.tsx
        status: existing
        disposition: preserve
        assertion: "Setup shows 12° beside 10-30° without a warning, warns near or outside the range and after 30 s unsolved, with today's words"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/web/src/App.test.tsx
        status: existing
        disposition: preserve
        assertion: "the home page's last line from a health without databases reads Database: hip_main, 42,212 stars, fields 10-30°"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "with one database Setup's field block, its alerts and the home page's last line are today's"
      - "no MCP tool's description or schema changes"
      - "crates/finder/web/dist is the build of the source"
      - "US1's and US2's test files pass unchanged; US3 writes none of them"
    checks:
      development:
        - "npm test -- FieldDatabases healthLine"
        - "cargo nextest run -p finder --locked --test mcp databases::"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, a new field kit's Pi binary and the field check of plan.md § Outside the factory"
    writes:
      - crates/finder/README.md
      - crates/finder/tests/mcp/databases.rs
      - crates/finder/tests/mcp/main.rs
      - crates/finder/web/dist
      - crates/finder/web/src/App.tsx
      - crates/finder/web/src/api.ts
      - crates/finder/web/src/components/FieldDatabases.test.tsx
      - crates/finder/web/src/components/FieldWarning.tsx
      - crates/finder/web/src/healthLine.test.ts
      - crates/finder/web/src/healthLine.ts
    no_write_reason: ""
    shared_writes:
      - path: crates/finder/README.md
        other_spec: 042-imaging-run-page-and-mcp
        other_story: US11
        disposition: merge_order
        reason: "047 depends_on_landed 042, whose US11 last changed the finder's README (7164e87f)"
      - path: crates/finder/tests/mcp/main.rs
        other_spec: 042-imaging-run-page-and-mcp
        other_story: US10
        disposition: merge_order
        reason: "047 depends_on_landed 042, whose US10 last changed mcp/main.rs (c5f30038)"
      - path: crates/finder/web/dist
        other_spec: 042-imaging-run-page-and-mcp
        other_story: US9
        disposition: merge_order
        reason: "047 depends_on_landed 042, whose US9 last rebuilt dist (5d8f5353)"
      - path: crates/finder/web/src/api.ts
        other_spec: 042-imaging-run-page-and-mcp
        other_story: US9
        disposition: merge_order
        reason: "047 depends_on_landed 042, whose US9 last changed api.ts (5d8f5353)"
      - path: crates/finder/web/src/App.tsx
        other_spec: 031-imaging-frames-on-console
        other_story: US7
        disposition: merge_order
        reason: "047 depends_on_landed 031, whose US7 last changed App.tsx (11e9f60f)"
      - path: crates/finder/web/src/components/FieldWarning.tsx
        other_spec: 013-live-finder
        other_story: US9
        disposition: merge_order
        reason: "047 depends_on_landed 013, whose US9 wrote FieldWarning.tsx (2a2f9d28)"
    open_questions: []
```
