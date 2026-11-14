# plate-solver Constitution

Every agent attempt in this repository reads and obeys this document. It distils the
rules that already govern `main`. Where it points at another file, that file is the
detail and this is the obligation. If the two disagree, stop and escalate rather than
pick one.

## Core Principles

### I. Agreeing with cedar is correctness (NON-NEGOTIABLE)

"Correct" means agreeing with the vendored references in `reference-solutions/`
(tetra3, cedar-solve, cedar-detect). The parity contract: RA/Dec within a few
arcseconds, centroids within ±0.1 px, identical matched catalog IDs. Those references
were not produced by the code under test, which is what makes agreement evidence.
Compute in f64 and store database vectors as f32. Take angles via `2·arcsin(d/2)`,
never `arccos`. Use the reference's false-alarm statistic and `_MAGIC_RAND`. The full
list of fixed numerics is `openspec/project.md` §5. A behaviour not grounded in
`reference-solutions/` (solver work) or in the cited sources (console work, feat-11
onward) is invented, and invented behaviour is a defect.

### II. Never weaken a gate

The checks are how anyone knows the product works, so a change that makes them pass
by making them weaker is worse than a red build. Do not:

- delete or skip a test case;
- broaden a tolerance;
- re-bless a golden to match new output;
- let missing data count as success;
- exclude a failure from a statistic;
- mark a test ignored;
- lower the 80% coverage floor of the manual `test-coverage` run;
- edit `scripts/ci/`, `.github/`, `CODEOWNERS` or `reference-solutions/` to get green.

The oracle rule from `openspec/project.md` §7 binds every story: no story may edit an
existing test, golden, `scripts/ci/`, `.github/workflows/ci.yml` or `CODEOWNERS`,
unless its spec names that edit for the owner's explicit approval. If a story cannot
pass without one, it is blocked. Say so; do not route around it.

### III. Tests first, with independent expectations

Each story lands the tests that prove it, in the same change. An expected value comes
from somewhere other than the code under test: a cedar golden, a worked example (Meeus
and others), a fixture, the simulator's truth, or a saved capture. Every scenario
states its oracle and an explicit tolerance. Time-dependent behaviour takes its clock
from outside. A virtual clock is allowed only in tests and `finder --simulate`.

### IV. Read the critical paths twice

These paths can make a broken product look green, or break consumers, without failing
a test. A change to them needs a stated reason in the story, the smallest diff that
achieves it, and tests that would catch the regression. `ENVIRONMENT.md` § Critical
paths is the authoritative list:

- `crates/math-core/**`: the numerical-parity contract.
- Any `unsafe` block: the audited-site census must stay explicit.
- `proto/**` and `crates/grpc-service`: the public API. gRPC `ImageCoord` is `(x, y)`
  and the solver works in `(y, x)`, swapped only at the service boundary.
- `crates/pattern-database` and `crates/database-generation`: the on-disk format is a
  compatibility contract.
- `Cargo.toml` and `Cargo.lock`: every dependency change.
- `build.rs` and `.cargo/**`: these decide what runs under a test's name.

### V. Standalone Rust with honest licensing

The solver is our own Rust. The references are oracles for development and tests, never
runtime dependencies, and their source is never copied into `crates/`. Keep their
licenses and credits intact: tetra3 and cedar-solve are Apache-2.0, cedar-detect is
FSL-1.1-MIT. Keep detection, numerics, database access, database generation, solving and
transports in their separate crates.

## Additional Constraints

- **Toolchain:** MSRV is Rust 1.96 (`rust-version` in `Cargo.toml`). New dependencies
  must build on it and pass `cargo-deny` and `cargo-machete`. Every cargo invocation
  uses `--locked`.
- **Cargo in the factory sandbox:** the toolchain comes in through the caches
  `ergane.yaml` declares, so `CARGO_HOME` and `RUSTUP_HOME` are set but cargo is not on
  `PATH`. Run `export PATH="$CARGO_HOME/bin:$PATH" CC=gcc CXX=g++` before any cargo
  command, as the gate commands do.
- **Warnings are errors:** CI builds with `RUSTFLAGS="-D warnings"`. clippy, coverage and
  docs build with `--all-features` on x86_64, and rustdoc denies warnings too.
- **Product limits:** detection input is 8-bit grayscale. The database must stay
  memory-mappable. Any parallelism must be bounded and off or constrained on mobile.
- **Finder console:** `crates/finder/web/dist` is committed and embedded in the binary,
  and must be the build of its source, so cargo never needs Node. The page adds no npm
  packages. UI work follows `DESIGN.md`, including the red-only night mode.
- **Secrets:** none, ever. Test tokens use low-entropy values, because `ripsecrets` scans
  the whole tree.
- **Vocabulary:** use the terms in `GLOSSARY.md` exactly, in code, specs and messages.

## Ergane floor (floor version 1.0.0)

These two rules come from Ergane's default floor and are kept on purpose.

- **Acceptance criteria must be provable from the diff.** The judge sees only the story's
  diff and its criteria. A criterion that names a CI observation, a running system or a
  commit message describes evidence the judge cannot see, so correct work fails it.
  Write criteria as tests and code the diff contains.
- **Work arrives as vertical slices with tests first.** The factory dispatches one story
  at a time, and the gates run tests against what comes back. A story that is not a
  testable slice is not a thing this factory can land.

## Development Workflow and Quality Gates

- **One story, one PR.** The PR title is the landing commit's subject: it is squashed onto
  `main` through the merge queue. Keep the diff to what the story needs, with no drive-by
  refactors.
- **Local gates:** `lint` (fmt and clippy) and `test` (all targets and doctests), as
  declared in `ergane.yaml`. Both must pass before a story is proposed.
- **The queue runs everything else.** These `ci.yml` jobs are required checks:
  `e2e-eval` (E1–E3 against cedar), `e2e-canary`, `finder-page`, `docs`, `msrv`,
  `cargo-machete`, `cargo-deny`, `secret-scan` and `hold`. A change that passes the local
  gates but would fail one of these is not done. `test-coverage` (the 80% workspace
  line-coverage floor) runs only when the maintainer starts it by hand and is not a
  required check. New code still needs tests.
- **Enforcement truth:** `ENVIRONMENT.md` states what `main` actually enforces. It must
  never claim more than is enforced.

## Governance

This constitution sits above any story's plan or task list. A conflict between them is
resolved by escalating to the maintainer, never by an agent choosing.

Amending this document is the maintainer's act, in its own PR, never part of a story.
So is amending `ergane.yaml`, `ENVIRONMENT.md` § Enforcement status, or the oracle files
named in `.github/CODEOWNERS`.

**Version**: 1.1.0 | **Ratified**: 2026-10-28 | **Last Amended**: 2026-11-14
