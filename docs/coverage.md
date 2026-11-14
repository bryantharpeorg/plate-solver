# Coverage reports

CI's `test-coverage` job checks one number: workspace line coverage of at least 80%. Since
2026-11-14 it runs only when started by hand (Actions > CI > Run workflow) and is not a
required check. These reports show what is behind it, per crate, per file and per line, for the Rust workspace and
for the finder console's page.

They are evidence, not a gate. Nothing here sets a threshold, and the 80% floor in
`.github/workflows/ci.yml` is unchanged.

## Run it

```sh
scripts/coverage.sh
```

That runs the workspace tests under `cargo llvm-cov` with the CI gate's flags
(`--workspace --locked --all-features`), then the page's Vitest suite with v8 coverage.
The instrumented debug build runs the simulator's long tests slowly, so expect the Rust half
to take as long as a cold `cargo test` or longer; the page takes seconds. Everything lands in
`target/coverage/`:

| Path | What it is |
|------|------------|
| `rust/html/index.html` | The Rust report: per-file table, then each file with its lines and regions highlighted. |
| `rust/lcov.info` | lcov, for editor gutters (e.g. VS Code's Coverage Gutters) and trend tools. |
| `rust/summary.json` | llvm-cov's own summary: per-file and total lines, functions and regions. |
| `web/index.html` | Istanbul's report for `crates/finder/web/src`: per-directory and per-file tables, per-line highlighting. |
| `web/lcov.info`, `web/coverage-summary.json`, `web/coverage-final.json` | The same, machine-readable. |
| `summary.md` | One page: each crate, the page, and the weakest files. The script also prints it. |
| `summary.json` | `summary.md` as data, per file and per crate, with the commit. Diff two to track movement. |

Options: `--rust` or `--web` runs one half. `--report-only` re-renders the Rust reports from
the last `cargo llvm-cov` run without running the tests again. `--branch` adds Rust branch
coverage (below). `COVERAGE_DIR=...` writes somewhere other than `target/coverage`.

It needs the tools CI installs:

```sh
cargo install cargo-llvm-cov --version 0.8.7 --locked
rustup component add llvm-tools-preview
```

The page half runs `npm ci` in `crates/finder/web` if `node_modules` is missing.
`npm run coverage` in that directory runs the page half on its own.

In CI, a manual `test-coverage` run re-renders the gate's own run (`--report-only`, no second test run)
and `finder-page` runs the page half. They upload the `coverage-rust` and `coverage-page`
artifacts and append `summary.md` to the job summary. Both steps are `continue-on-error`, so
neither can fail its job.

## Read it

**The HTML.** In the Rust report, red lines never ran. A red highlight inside a green line is
a *region* that never ran: a span with a single execution count, such as an `if` arm, a
closure, or the right side of an `&&`. A covered line can hold an uncovered region, so the
regions column is the strictest. The margin shows how many times each line ran. The page's
report is Istanbul's: statements, branches (each arm of an `if`, `?:`, `&&`, `??` or default
parameter), functions and lines, with uncovered branches marked inline.

**Two Rust line figures.** `summary.md` shows both, because llvm-cov's own figure is not a
count of source lines:

- **Lines (llvm-cov)** is the figure the 80% gate and the HTML use. llvm-cov adds up each
  function's lines, so a line inside a closure or an async block counts once for it and once
  more for each function around it. It also counts the unit tests: in Rust they live in the
  file they test, in `#[cfg(test)] mod tests { ... }`, and that code nearly always runs.
- **Src lines** counts each line once (lcov's per-line records) and leaves the
  `#[cfg(test)]` blocks out of both sides of the ratio, as Istanbul leaves out `*.test.ts`.
  This is the figure to read for "how much of the product do the tests run". **Src
  functions** leaves out the test functions the same way. The library and its test build
  each carry a copy of every function, so src functions counts one per start line.

`scripts/coverage-summary.py` finds the test blocks by layout, which works because CI
enforces rustfmt. Regions and branches have no src-only form: they are llvm-cov's counts,
tests included.

**What is already left out.** cargo-llvm-cov's defaults drop integration tests (`tests/`),
examples, benches, build scripts, code generated into `target/` (the tonic and prost gRPC
stubs) and every dependency. Every file in the Rust report is under `crates/*/src/`. Stable
Rust does not measure doctests (CI runs them separately). `reference-solutions/` and
`bench/cedar-compare` are outside the workspace. On the page, the tests, `src/test/`
(helpers and fixtures) and `main.tsx` are left out. `main.tsx` only mounts `<App />` in a
real browser, so only Playwright reaches it. Every other file under `src/` counts, whether a
test imports it or not.

**Weakest files** lists the lowest-covered files with at least 20 lines. "Missed" is the
number of lines that never ran, a better guide than the percentage to where a test would pay
off.

## Branch coverage

Rust branch coverage is still unstable in rustc, so it needs nightly:

```sh
rustup toolchain install nightly --profile minimal --component llvm-tools-preview
scripts/coverage.sh --rust --branch
```

That builds and runs the workspace again with the nightly compiler. The tables gain a
`Branches` column, and the HTML marks each branch as taken or not. Nightly's numbers can
differ slightly from the stable gate's. To keep the stable build's artifacts, give it its
own `CARGO_TARGET_DIR`. The page's branch coverage needs nothing extra.

On nightly 2026-11-02 (LLVM 23.1), llvm-cov crashes exporting lcov, with or without
`--branch`. The script then keeps the HTML and `summary.json`, skips `lcov.info`, and
`summary.md` shows llvm-cov's own columns without the src figures. On `3aa8dde` that run
measured 76.8% of 2,564 branches.
