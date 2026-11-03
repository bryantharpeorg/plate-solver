#!/usr/bin/env bash
# quality.sh [--report-only] — a SonarQube-style code-quality report for the workspace and the page.
#
# Complexity and size (rust-code-analysis; sonarjs's cognitive complexity for the page),
# duplication (jscpd), lint findings (clippy with its pedantic and nursery groups, ESLint with
# SonarSource's rules for the page), panic paths and `unsafe` (clippy restriction lints),
# dependency advisories (cargo-deny, npm audit) and unused dependencies (cargo-machete),
# graded A to E per crate and per file. Test code is left out. Writes to target/quality
# (or $QUALITY_DIR):
#
#   html/index.html   the summary, then each crate, the page and every file, its source marked
#   summary.md        the summary in Markdown (the script prints it too)
#   summary.json      everything, per file, function and finding, stamped with the commit
#   raw/              each tool's own output
#
# --report-only   render the report again from raw/ without running the tools
#
# This is evidence, not a gate: it sets no threshold and changes no lint setting. The extra
# clippy lints are given on the command line, capped at warn, and clippy.toml still applies.
# How to read the output: docs/quality.md.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="${QUALITY_DIR:-$root/target/quality}"
raw="$out/raw"
tools="$root/scripts/quality"
report="$root/scripts/quality-report.py"
report_only=0

for arg in "$@"; do
  case "$arg" in
    --report-only) report_only=1 ;;
    -h | --help) sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "quality.sh: unknown argument: $arg (try --help)" >&2; exit 2 ;;
  esac
done

cd "$root"

if [ "$report_only" = 0 ]; then
  missing=0
  if ! command -v rust-code-analysis-cli >/dev/null 2>&1; then
    echo "quality.sh: rust-code-analysis-cli is not installed:" >&2
    echo "  cargo install rust-code-analysis-cli --version 0.0.25 --locked" >&2
    missing=1
  fi
  if ! command -v npm >/dev/null 2>&1; then
    echo "quality.sh: npm is not installed (jscpd and ESLint run on Node)." >&2
    missing=1
  fi
  [ "$missing" = 0 ] || exit 1

  rm -rf "$raw"
  mkdir -p "$raw/rca" "$raw/jscpd"
  [ -d "$tools/node_modules" ] || (cd "$tools" && npm ci --no-audit --no-fund >&2)
  {
    echo "rustc: $(rustc --version)"
    echo "clippy: $(cargo clippy --version)"
    echo "rust-code-analysis: $(rust-code-analysis-cli --version)"
    echo "jscpd: $("$tools/node_modules/.bin/jscpd" --version)"
    echo "eslint: $("$tools/node_modules/.bin/eslint" --version)"
    echo "eslint-plugin-sonarjs: $(node -p "require('$tools/node_modules/eslint-plugin-sonarjs/package.json').version")"
  } >"$raw/tools.txt"

  # The measured code, test items blanked: what rust-code-analysis and jscpd read.
  python3 "$report" prepare "$out" "$root"

  echo "quality.sh: complexity and size (rust-code-analysis)" >&2
  (cd "$raw/src" && rust-code-analysis-cli --metrics -O json -o "$raw/rca" -p crates)
  # Its tree-sitter grammars are older than some syntax; note the files they trip on.
  (cd "$raw/src" && rust-code-analysis-cli -f ERROR -p crates) 2>/dev/null \
    | sed -n 's/^In file //p' \
    | python3 -c 'import json, sys; print(json.dumps([l.strip() for l in sys.stdin]))' >"$raw/rca-parse-errors.json"

  echo "quality.sh: duplication (jscpd)" >&2
  # raw/ is under target/, which .gitignore lists, hence --no-gitignore. Comments are not
  # tokens (--mode weak), as in SonarQube's copy-paste detection.
  "$tools/node_modules/.bin/jscpd" --min-tokens 50 --min-lines 5 --mode weak \
    --format rust,typescript,tsx --no-gitignore --absolute --reporters json \
    --output "$raw/jscpd" --silent --no-tips "$raw/src/crates" >&2

  echo "quality.sh: lints (clippy, ESLint)" >&2
  mapfile -t lints < <(python3 "$report" clippy-args)
  # Without --all-targets: the library and binary targets, so no test code. Warnings do not
  # fail the build, so the exit status is cargo's own (a compile error stops the report).
  cargo clippy --workspace --all-features --locked --message-format=json -- "${lints[@]}" >"$raw/clippy.jsonl"
  # The group of every clippy lint, from the toolchain that just ran, for the severities.
  clippy-driver rustc -W help >"$raw/clippy-lint-groups.txt" 2>/dev/null || true
  node "$tools/eslint.js" "$root" "$raw"

  echo "quality.sh: dependencies (cargo-deny, cargo-machete, npm audit)" >&2
  if command -v cargo-deny >/dev/null 2>&1; then
    echo "cargo-deny: $(cargo deny --version)" >>"$raw/tools.txt"
    # deny.toml's policy, as the cargo-deny job runs it; a non-zero exit is a finding, not a failure.
    cargo deny --format json --log-level info --all-features check advisories 2>"$raw/deny.jsonl" >/dev/null || true
  else
    echo "quality.sh: cargo-deny is not installed; no Rust advisories (cargo install cargo-deny --locked)" >&2
  fi
  if command -v cargo-machete >/dev/null 2>&1; then
    echo "cargo-machete: $(cargo machete --version 2>/dev/null | head -1)" >>"$raw/tools.txt"
    cargo machete >"$raw/machete.txt" 2>&1 || true
  else
    echo "quality.sh: cargo-machete is not installed; no unused-dependency check" >&2
  fi
  # The page's lockfile only: no install. Needs the network, so a failure only leaves it out.
  (
    cd crates/finder/web
    npm audit --json --package-lock-only >"$raw/npm-audit.json" 2>/dev/null || true
    npm audit --json --package-lock-only --omit dev >"$raw/npm-audit-prod.json" 2>/dev/null || true
  )
fi

[ -d "$raw/rca" ] || { echo "quality.sh: no $raw to report from; run without --report-only" >&2; exit 1; }
python3 "$report" report "$out" "$root"

echo
echo "Report:  $out/html/index.html"
echo "Summary: $out/summary.md"
