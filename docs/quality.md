# Code-quality report

A SonarQube-style dashboard for the Rust workspace and the finder page, built from local
tools: complexity, duplication, size, lint findings ("code smells") by rule and severity,
panic paths, `unsafe`, dependency advisories, and an A–E maintainability grade per crate and
per file. It is a browsable HTML report plus JSON stamped with the commit, so runs can be
compared.

It is evidence, not a gate. Nothing here sets a threshold or fails a check, and it changes
no lint setting: `clippy.toml`, the crates' lint attributes and CI's `clippy -D warnings`
gate are untouched. The extra lints are passed on clippy's command line for this report
only.

## Run it

```sh
scripts/quality.sh
```

A warm run takes about ten seconds; clippy's first build takes as long as a `cargo check`.
Everything lands in `target/quality/` (or `$QUALITY_DIR`):

| Path | What it is |
|------|------------|
| `html/index.html` | The summary: grade, tiles, a row per crate and the page, the worst functions, every rule, duplication, security. Each crate links to its files, each file to its functions, findings, duplicated blocks and source. |
| `summary.md` | The summary in Markdown (the script prints it too). CI appends it to the job summary. |
| `summary.json` | Everything: per crate, file, function and finding, the clones, the advisories, with the commit, the tool versions and the grading constants. |
| `raw/` | Each tool's own output, and `raw/src/`, the copy of the code the tools measured. |

`--report-only` renders the report again from `raw/` without running the tools.

It needs, besides the Rust toolchain with clippy and Node with npm:

```sh
cargo install rust-code-analysis-cli --version 0.0.25 --locked
cargo install cargo-deny --version 0.20.2 --locked     # advisories (optional)
cargo install cargo-machete --version 0.9.2 --locked   # unused dependencies (optional)
```

jscpd and ESLint are pinned in `scripts/quality/package.json` with their own lockfile, so
the page's `package-lock.json` is untouched. The script runs `npm ci` there on first use.
Without cargo-deny or cargo-machete, the report says so and leaves that section out.
`npm audit` reads only the page's lockfile, but it needs the network.

In CI, the `quality-report` job runs the script on pull requests and on pushes to main. It
uploads the `quality-report` artifact (open `html/index.html`) and appends `summary.md` to
the job summary. It is not a required check, every step is `continue-on-error`, and it is
skipped in the merge queue, where nothing reads it.

To compare two runs:

```sh
python3 scripts/quality-report.py diff old/summary.json target/quality/summary.json
```

That prints each crate's figures with their change, and the functions whose cognitive
complexity moved most.

## What is measured

The Rust workspace is each crate's `src/` and `build.rs`. The page is
`crates/finder/web/src`. Test code is left out everywhere: `#[cfg(test)]` items, `tests/`,
benches and examples, the page's `*.test.ts(x)` and `src/test/`, the same split the
coverage report makes (docs/coverage.md). To keep it out, `prepare` copies the code to
`raw/src/` with the `#[cfg(test)]` items blanked, keeping line numbers, and the tools read
that copy. Code generated into `target/` (the tonic and prost gRPC stubs) is left out too;
the report's notes count the clippy findings it drops there. File pages show the whole
file, with test code greyed.

**Size.** *Code lines* are lines holding code, not comments or blanks (rust-code-analysis's
`ploc`, SonarQube's `ncloc`). *Lines* are all lines of measured code, comments and blanks
included; duplication is a share of them. *Functions* count each `fn` and method. A Rust
closure belongs to its function, while each function on the page counts on its own.

**Cyclomatic complexity** is 1 plus the decision points in a function (rust-code-analysis).

**Cognitive complexity** is SonarSource's measure of how hard a function is to follow: +1
for each `if`, `else`, `match`, loop, `catch`, jump to a label and sequence of `&&`/`||`,
with an extra +1 per level of nesting
([the specification](https://www.sonarsource.com/docs/CognitiveComplexity.pdf)).

- Rust: rust-code-analysis, with a function's closures in its total, as the specification
  has it. Scored by hand against the specification, version 0.0.25 matched on 21 of 27
  cases. It never counts `loop`, neither its +1 nor the nesting of what is inside it. It
  scores `a && b && c || d` as 3 where the specification says 2, and it skips recursion. A
  function with a `loop` is marked **+** in the report: its true figure is higher. There
  are about thirty `loop`s in the workspace's source.
- The page: sonarjs, SonarSource's own implementation in eslint-plugin-sonarjs.
  rust-code-analysis's TypeScript count agreed with it on only 27 of 147 functions, so it
  is not used for the page.

**Duplication** comes from jscpd, comments ignored (`--mode weak`), as in SonarQube's
copy-paste detection. A *duplicated block* is SonarQube's minimum: at least 100 tokens over
at least 10 lines. *Duplication* is the share of lines inside such blocks. jscpd also
finds *small clones*, from 50 tokens over 5 lines (its own default). The report lists them
as refactoring candidates, but they do not count in the percentage.

**Lint findings (code smells).** clippy runs on the library and binary targets (no
`--all-targets`, so no test code), with the `pedantic` and `nursery` groups on top of the
defaults and every lint capped at warn. Each finding's severity comes from its clippy group:

| clippy group | Severity |
|---|---|
| correctness | Blocker |
| suspicious | Critical |
| perf, complexity | Major |
| style, pedantic | Minor |
| nursery, cargo | Info |

The default groups are already denied in CI, so on main nearly every finding is
`pedantic` (Minor) or `nursery` (Info). `nursery` is clippy's group of lints still in
development, which are known to misfire, so they are listed but cost nothing in the grade.
clippy's own `cognitive_complexity` lint is turned off, because the report measures
complexity once, the way described above.

On the page, ESLint runs the recommended rules of ESLint, typescript-eslint and React's
hooks plugin, and sonarjs's recommended set (`scripts/quality/eslint-rules.js`). The
severity comes from the rule's type: `problem` is Major, `suggestion` Minor, `layout` Info.
`scripts/quality/eslint.js` runs ESLint from the repository root, because ESLint 10 lints
only files under its config's directory. The page itself has no ESLint and gains none: only
this report runs it.

**Panic paths** are the non-test places that can panic on purpose: clippy's `unwrap_used`,
`expect_used`, `panic`, `todo`, `unimplemented` and `unreachable`. They are counted, not
judged. An `expect` that states an invariant is often the right call, so they are not in
the grade.

**Unsafe** lists every `unsafe` block, function and impl (rustc's `unsafe_code` lint), plus
clippy's `undocumented_unsafe_blocks` for any block without a `// SAFETY:` comment. The
security section also shows which crates declare `#![forbid(unsafe_code)]`.

**Advisories.** cargo-deny checks the Rust graph against the RustSec database with
`deny.toml`'s policy, as the `cargo-deny` job does. Advisories that `deny.toml` ignores are
listed as *accepted*, and any other is *active*. `npm audit` checks the page's lockfile and
marks each vulnerable package as runtime or dev only. **Unused dependencies** are
cargo-machete's. **Lint suppressions** lists every `#[allow]`/`#[expect]` and
`eslint-disable` in measured code: findings hidden that way are not in the report.

## The grade

The grade is SonarQube's maintainability rating, with its default constants:

```
debt ratio = remediation minutes / (30 minutes × code lines)

A: up to 5%   B: up to 10%   C: up to 20%   D: up to 50%   E: above 50%
```

30 minutes per code line is SonarQube's default estimate of what writing the code cost.
Remediation is the estimated work to clear every finding:

- 5 minutes per lint finding, and none for Info;
- for each function over cognitive complexity 15, 5 minutes plus 1 per point over 15. That
  is SonarQube's rule S3776, which uses the same threshold.

A crate's or the page's grade comes from its total ratio, not an average of its files'
grades, as in SonarQube. Duplication, panic paths, `unsafe` and advisories are reported
beside the grade, not in it. SonarQube keeps them out of the maintainability rating too.

**What the grade can and cannot tell you.** The constants make the grid lenient: below A
takes more than 1.5 minutes of debt per code line, so more than one 5-minute finding for
every 3.3 code lines. On SonarQube's terms, then,
the code is clean, and the grade will rarely move. The useful signal is in the debt ratio
(its ranking of crates and files), the worst-functions list, and the rules table, which
shows what the debt is made of. Most of it is mechanical pedantic lints such as
`must_use_candidate` and the numeric-cast lints, at 5 minutes each.

The 5 minutes per finding is an assumption, not a measurement. SonarQube sets an effort
for each rule, and the clippy rules it imports have none of their own. Treat the debt as a
model for comparing crates and runs, not as a schedule. If a stricter grid is wanted, it
lives in `GRADES` in `scripts/quality-report.py`. It would then no longer be SonarQube's.

## Why not SonarQube itself

SonarQube has had a Rust analyzer since 2025. It runs clippy (85 of its lints at launch), computes
cyclomatic and cognitive complexity, and imports coverage. A self-hosted server still costs
more than it gives here:

- It is a server with Elasticsearch inside, which needs a host kernel setting
  (`vm.max_map_count`) and a couple of GB of memory. It needs an admin account, a token and
  a scanner, and its history lives in the server's database, not in the repository.
- CI could only use it through a server reachable from GitHub's runners, or through
  SonarQube Cloud: an outside service that holds a token and a copy of the analysis.
- Its Rust rules are a subset of what clippy reports directly. For the page, sonarjs is
  SonarSource's own analyzer for JavaScript and TypeScript, run here as an ESLint plugin.

The local tools give the same measures with SonarQube's formulas, in one command, at
versions pinned in the repository, and as an artifact on every pull request. A hosted
dashboard with history would mean SonarQube Cloud, which is free for public repositories.
That is a separate decision.

## Known limits

- rust-code-analysis 0.0.25 is from 2023. Its grammars parse all of the workspace's Rust,
  but not TypeScript's `export type { … } from` (one line in `useSession.ts`). The report
  marks files it could not fully parse.
- jscpd's tokens are not exactly SonarQube's, so a block at the edge of the 100-token
  minimum can fall on either side.
- The tool package's own dependencies carry one moderate advisory: minimatch below 11.0.1,
  a regular-expression denial of service on crafted glob patterns. The only globs it sees
  are the ones in `scripts/quality/`. The fixed minimatch 11 was a week old when this was
  written, so it is not forced in with an override yet.
