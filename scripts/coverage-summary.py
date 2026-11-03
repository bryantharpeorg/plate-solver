#!/usr/bin/env python3
"""coverage-summary.py OUT ROOT [--rust] [--web] — per-crate coverage tables.

Reads what scripts/coverage.sh wrote under OUT (rust/summary.json and rust/lcov.info from
cargo-llvm-cov, web/coverage-summary.json from Vitest) and writes OUT/summary.md and
OUT/summary.json: the Rust numbers per crate, the page's numbers, and the weakest files.
Prints summary.md as well.

Two Rust line figures, because llvm-cov's own one is not a count of source lines:

- "llvm-cov" is the figure the 80% gate and the HTML report use. llvm-cov adds up each
  function's lines, so a line inside a closure or an async block counts once for it and
  again for every function around it. It also counts unit tests as source: they live
  beside the code, in `#[cfg(test)] mod tests { ... }`, and nearly always run.
- "src" counts each line once (lcov's per-line records) and leaves the `#[cfg(test)]`
  blocks out of both sides of the ratio, as Istanbul leaves out *.test.ts. Src functions
  likewise leave out the test functions, with one entry per start line, so the copies of a
  function compiled into the test build and into the library count once.

Blocks are found by layout: the repository is rustfmt-clean (CI checks), so a block runs
from the attribute to the first line that is `}` at the attribute's own indent.
"""

import datetime
import json
import os
import re
import subprocess
import sys

WEAKEST = 12  # files listed in the weakest-files tables
MIN_LINES = 20  # smaller files are left out of those tables: one missed line is not news


def ratio(covered, count):
    return {"covered": covered, "count": count, "percent": round(100.0 * covered / count, 2) if count else None}


def pct(m):
    return "—" if m is None or m.get("percent") is None else f"{m['percent']:.1f}%"


def test_blocks(path):
    """1-based inclusive line ranges of `#[cfg(test)]` items in a rustfmt-formatted file."""
    try:
        with open(path, encoding="utf-8") as f:
            lines = f.read().splitlines()
    except OSError:
        return []
    ranges, i, n = [], 0, len(lines)
    while i < n:
        if lines[i].strip() != "#[cfg(test)]":
            i += 1
            continue
        indent = lines[i][: len(lines[i]) - len(lines[i].lstrip())]
        k = i + 1
        while k < n and (lines[k].strip().startswith(("#[", "//")) or not lines[k].strip()):
            k += 1
        while k < n:
            text = lines[k].rstrip()
            if text.endswith("{"):
                k += 1
                while k < n and lines[k].rstrip() != indent + "}":
                    k += 1
                break
            if text.endswith((";", ",", "}")):
                break
            k += 1
        ranges.append((i + 1, min(k, n - 1) + 1))
        i = k + 1
    return ranges


def read_lcov(path):
    """{source file: ({line: hits}, {function start line: hits})} from an lcov file."""
    files, lines, fns, starts = {}, None, None, {}
    with open(path, encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if line.startswith("SF:"):
                lines, fns = files.setdefault(line[3:], ({}, {}))
                starts = {}
            elif lines is None:
                continue
            elif line.startswith("DA:"):
                number, hits = (int(x) for x in line[3:].split(",")[:2])
                lines[number] = max(lines.get(number, 0), hits)
            elif line.startswith("FN:"):
                number, name = line[3:].split(",", 1)
                starts[name] = int(number)
            elif line.startswith("FNDA:"):
                hits, name = line[5:].split(",", 1)
                if name in starts:
                    fns[starts[name]] = max(fns.get(starts[name], 0), int(hits))
            elif line == "end_of_record":
                lines = fns = None
    return files


def crate_of(rel):
    m = re.match(r"crates/([^/]+)/", rel)
    return m.group(1) if m else rel.split("/")[0]


def rust_section(out, root):
    with open(os.path.join(out, "rust", "summary.json"), encoding="utf-8") as f:
        data = json.load(f)["data"][0]
    lcov_path = os.path.join(out, "rust", "lcov.info")
    # coverage.sh --branch goes on without lcov when nightly's export fails: no src columns.
    lcov = read_lcov(lcov_path) if os.path.exists(lcov_path) else None
    llvm = ["lines", "functions", "regions"]
    if data["totals"].get("branches", {}).get("count", 0) > 0:
        llvm.append("branches")
    ours = ["distinct_lines", "src_lines", "src_functions"] if lcov is not None else []

    files, crates = [], {}
    for entry in data["files"]:
        path = entry["filename"]
        rel = os.path.relpath(path, root)
        row = {"path": rel, "crate": crate_of(rel)}
        for kind in llvm:
            row[kind] = ratio(entry["summary"][kind]["covered"], entry["summary"][kind]["count"])
        if lcov is not None:
            lines, fns = lcov.get(path, ({}, {}))
            blocks = test_blocks(path)

            def src(d):
                return {n: h for n, h in d.items() if not any(a <= n <= b for a, b in blocks)}

            for kind, d in (("distinct_lines", lines), ("src_lines", src(lines)), ("src_functions", src(fns))):
                row[kind] = ratio(sum(1 for h in d.values() if h), len(d))
            row["test_blocks"] = blocks
        files.append(row)
        c = crates.setdefault(row["crate"], {"files": 0, **{k: [0, 0] for k in llvm + ours}})
        c["files"] += 1
        for kind in llvm + ours:
            c[kind][0] += row[kind]["covered"]
            c[kind][1] += row[kind]["count"]

    crate_rows = [
        {"crate": name, "files": c["files"], **{k: ratio(*c[k]) for k in llvm + ours}}
        for name, c in sorted(crates.items())
    ]
    totals = {k: ratio(data["totals"][k]["covered"], data["totals"][k]["count"]) for k in llvm}
    for kind in ours:
        totals[kind] = ratio(sum(r[kind]["covered"] for r in files), sum(r[kind]["count"] for r in files))
    return {"llvm_kinds": llvm, "src": lcov is not None, "totals": totals, "crates": crate_rows, "files": files}


def web_section(out, root):
    with open(os.path.join(out, "web", "coverage-summary.json"), encoding="utf-8") as f:
        data = json.load(f)
    kinds = ["statements", "branches", "functions", "lines"]

    def conv(m):
        return {k: ratio(m[k]["covered"], m[k]["total"]) for k in kinds}

    files = [{"path": os.path.relpath(p, root), **conv(m)} for p, m in data.items() if p != "total"]
    return {"kinds": kinds, "totals": conv(data["total"]), "files": sorted(files, key=lambda r: r["path"])}


def weakest(files, key):
    big = [r for r in files if r[key]["count"] >= MIN_LINES and r[key]["percent"] is not None]
    return sorted(big, key=lambda r: (r[key]["percent"], -r[key]["count"]))[:WEAKEST]


def missed(m):
    return m["count"] - m["covered"]


def markdown(rust, web, commit):
    md = [f"## Coverage at `{commit}`", ""]
    if rust:
        t = rust["totals"]
        starred = [k for k in rust["llvm_kinds"] if k != "lines" and (k != "functions" or not rust["src"])]
        if rust["src"]:
            key = "src_lines"
            head = (
                f"Workspace: **{pct(t['lines'])} lines** by llvm-cov's count (the 80% gate's figure). "
                f"Each line counted once: {pct(t['distinct_lines'])}. "
                f"Without the inline `#[cfg(test)]` blocks: **{pct(t['src_lines'])} src lines** "
                f"({missed(t['src_lines'])} of {t['src_lines']['count']} never ran), "
                f"{pct(t['src_functions'])} src functions"
            )
            cols = [
                ("Src lines", lambda r: pct(r["src_lines"])),
                ("Missed", lambda r: str(missed(r["src_lines"]))),
                ("Src functions", lambda r: pct(r["src_functions"])),
            ]
        else:
            key = "lines"
            head = f"Workspace: **{pct(t['lines'])} lines** by llvm-cov's count (no lcov, so no src figures)"
            cols = [("Missed", lambda r: str(missed(r["lines"])))]
        head += "".join(f", {pct(t[k])} {k} (tests included)" for k in starred)
        cols += [(f"{k.capitalize()}*", lambda r, k=k: pct(r[k])) for k in starred]

        def table(first, rows, label):
            out = ["| " + " | ".join(h for h, _ in first + cols) + " |", "|---|" + "---:|" * (len(first) + len(cols) - 1)]
            out += ["| " + " | ".join([label(r)] + [f(r) for _, f in first[1:] + cols]) + " |" for r in rows]
            return out

        llvm_lines = ("Lines (llvm-cov)", lambda r: pct(r["lines"]))
        md += ["### Rust (cargo-llvm-cov, as the `test-coverage` job runs it)", "", head + ". docs/coverage.md explains the difference.", ""]
        md += table([("Crate", None), ("Files", lambda r: str(r["files"])), llvm_lines], rust["crates"], lambda r: f"`{r['crate']}`")
        md += ["", "\\* llvm-cov's own count, tests included.", "", f"Weakest files ({cols[0][0].lower() if rust['src'] else 'lines'}, files with at least {MIN_LINES}):", ""]
        md += table([("File", None)] + ([] if rust["src"] else [llvm_lines]), weakest(rust["files"], key), lambda r: f"`{r['path']}`")
        md.append("")
    if web:
        t = web["totals"]
        md += [
            "### Finder page (Vitest, v8 provider)",
            "",
            "`crates/finder/web/src`, tests and test fixtures left out: "
            + ", ".join(f"**{pct(t[k])} {k}**" if k == "lines" else f"{pct(t[k])} {k}" for k in web["kinds"])
            + ".",
            "",
            f"Weakest files (lines, files with at least {MIN_LINES}):",
            "",
            "| File | Lines | Missed | Branches | Functions |",
            "|---|---:|---:|---:|---:|",
        ]
        for r in weakest(web["files"], "lines"):
            md.append(
                f"| `{r['path']}` | {pct(r['lines'])} | {missed(r['lines'])} "
                f"| {pct(r['branches'])} | {pct(r['functions'])} |"
            )
        md.append("")
    return "\n".join(md)


def main(argv):
    if len(argv) < 2:
        sys.exit(__doc__.splitlines()[0])
    out, root = os.path.abspath(argv[0]), os.path.abspath(argv[1])
    halves = set(argv[2:]) or {"--rust", "--web"}
    try:
        commit = subprocess.run(
            ["git", "-C", root, "rev-parse", "--short", "HEAD"], capture_output=True, text=True, check=True
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        commit = "unknown"
    rust = rust_section(out, root) if "--rust" in halves else None
    web = web_section(out, root) if "--web" in halves else None
    report = {
        "commit": commit,
        "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
        "rust": rust,
        "web": web,
    }
    with open(os.path.join(out, "summary.json"), "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2)
    md = markdown(rust, web, commit)
    with open(os.path.join(out, "summary.md"), "w", encoding="utf-8") as f:
        f.write(md + "\n")
    print(md)


if __name__ == "__main__":
    main(sys.argv[1:])
