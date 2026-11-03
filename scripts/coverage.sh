#!/usr/bin/env bash
# coverage.sh [--rust | --web] [--branch] [--report-only] — browsable coverage reports.
#
# Runs the workspace tests under cargo-llvm-cov exactly as CI's `test-coverage` job does
# (--workspace --locked --all-features), and the finder page's Vitest suite with v8
# coverage. Writes everything to target/coverage (or $COVERAGE_DIR):
#
#   rust/html/index.html   per-file table, per-line and per-region highlighting
#   rust/lcov.info         lcov, for editors and trend tools
#   rust/summary.json      llvm-cov's per-file and total summary
#   web/index.html         Istanbul's HTML report for crates/finder/web/src
#   web/lcov.info, web/coverage-summary.json
#   summary.md, summary.json   per crate, the page, and the weakest files (both runs)
#
# --rust / --web   run one half only (default: both)
# --branch         add Rust branch coverage; needs a nightly toolchain with
#                  llvm-tools-preview (rustup toolchain install nightly -c llvm-tools-preview)
# --report-only    re-render the Rust reports from the last `cargo llvm-cov` run without
#                  running tests again (CI uses this after the gate has measured)
#
# This is evidence, not a gate: it sets no threshold. The 80% floor lives in ci.yml.
# How to read the output: docs/coverage.md.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="${COVERAGE_DIR:-$root/target/coverage}"
rust=1 web=1 branch=() toolchain=() report_only=0

for arg in "$@"; do
  case "$arg" in
    --rust) web=0 ;;
    --web) rust=0 ;;
    --branch) branch=(--branch) toolchain=(+nightly) ;;
    --report-only) report_only=1 ;;
    -h | --help) sed -n '2,22p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "coverage.sh: unknown argument: $arg (try --help)" >&2; exit 2 ;;
  esac
done

cd "$root"
mkdir -p "$out"

if [ "$rust" = 1 ]; then
  if ! cargo "${toolchain[@]}" llvm-cov --version >/dev/null 2>&1; then
    echo "coverage.sh: cargo-llvm-cov is not installed. CI pins it:" >&2
    echo "  cargo install cargo-llvm-cov --version 0.8.7 --locked" >&2
    echo "  rustup component add llvm-tools-preview" >&2
    exit 1
  fi
  if [ "$report_only" = 0 ]; then
    cargo "${toolchain[@]}" llvm-cov --no-report --workspace --locked --all-features "${branch[@]}"
  fi
  rm -rf "$out/rust"
  mkdir -p "$out/rust"
  cargo "${toolchain[@]}" llvm-cov report "${branch[@]}" --html --output-dir "$out/rust"
  cargo "${toolchain[@]}" llvm-cov report "${branch[@]}" --lcov --output-path "$out/rust/lcov.info"
  cargo "${toolchain[@]}" llvm-cov report "${branch[@]}" --json --summary-only \
    --output-path "$out/rust/summary.json"
fi

if [ "$web" = 1 ]; then
  (
    cd crates/finder/web
    [ -d node_modules ] || npm ci --no-audit --no-fund
    # vite.config.ts writes to target/coverage/web; follow $COVERAGE_DIR if it moved.
    npx vitest run --coverage --coverage.reportsDirectory="$out/web"
  )
fi

halves=()
[ "$rust" = 1 ] && halves+=(--rust)
[ "$web" = 1 ] && halves+=(--web)
python3 "$root/scripts/coverage-summary.py" "$out" "$root" "${halves[@]}"

echo
[ "$rust" = 1 ] && echo "Rust report: $out/rust/html/index.html"
[ "$web" = 1 ] && echo "Page report: $out/web/index.html"
echo "Summary:     $out/summary.md"
