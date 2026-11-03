#!/usr/bin/env python3
"""quality-report.py prepare|report OUT ROOT, or diff OLD.json NEW.json — scripts/quality.sh's report.

prepare      copies the code the report measures to OUT/raw/src, which rust-code-analysis and
             jscpd read: crates/*/src and crates/*/build.rs with every `#[cfg(test)]` item
             blanked (line numbers stay put), and the page's src/ without its tests.
report       reads what each tool wrote under OUT/raw and writes OUT/summary.json,
             OUT/summary.md and OUT/html/. Prints summary.md.
diff         compares two runs' summary.json, per crate and per function.
clippy-args  prints the lint flags quality.sh gives clippy, one per line.

The grade is SonarQube's maintainability rating (docs/quality.md has the reasoning):

    debt ratio = remediation minutes / (30 minutes x code lines)
    A: up to 5%   B: up to 10%   C: up to 20%   D: up to 50%   E: above 50%

Remediation is 5 minutes per lint finding, none for Info, plus 5 minutes and 1 more per
point for each function whose cognitive complexity is over 15 (SonarQube's rule S3776).
Duplication, panic paths, `unsafe` and advisories are reported beside the grade, not in it.
Test code is out of every figure: the `#[cfg(test)]` items, tests/, and the page's tests.
"""

import collections
import datetime
import glob
import html
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

DEV_MINUTES_PER_LINE = 30  # SonarQube's default development cost
SMELL_MINUTES = 5  # per lint finding, Info excepted
COGNITIVE_THRESHOLD = 15  # S3776's default; above it a function costs 5 min + 1 per point
GRADES = (("A", 0.05), ("B", 0.10), ("C", 0.20), ("D", 0.50))  # else E
DUP_TOKENS, DUP_LINES = 100, 10  # SonarQube's smallest duplicated block
JSCPD_TOKENS, JSCPD_LINES = 50, 5  # jscpd's defaults: smaller clones, listed but not in the %
SEVERITIES = ("Blocker", "Critical", "Major", "Minor", "Info")
WORST_MD, WORST_HTML, RULES_MD = 10, 25, 10

# clippy's groups, as SonarQube severities. Every finding in them is a code smell.
CLIPPY_SEVERITY = {
    "correctness": "Blocker",
    "suspicious": "Critical",
    "perf": "Major",
    "complexity": "Major",
    "style": "Minor",
    "pedantic": "Minor",
    "nursery": "Info",  # lints still being developed: listed, not charged
    "cargo": "Info",
}
# Restriction lints turned on for the security-ish columns. They are not code smells.
PANIC_LINTS = (
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::panic",
    "clippy::todo",
    "clippy::unimplemented",
    "clippy::unreachable",
)
UNSAFE_LINTS = ("unsafe_code", "clippy::undocumented_unsafe_blocks")
# ESLint rule types (meta.type), as severities.
ESLINT_SEVERITY = {"problem": "Major", "suggestion": "Minor", "layout": "Info"}


def clippy_args():
    """The flags after `cargo clippy --`: warnings only, so one crate's findings cannot stop the next."""
    args = ["--cap-lints", "warn", "-W", "clippy::pedantic", "-W", "clippy::nursery"]
    # Complexity is measured once, by rust-code-analysis; clippy's own count is a different metric.
    args += ["-A", "clippy::cognitive_complexity"]
    for lint in PANIC_LINTS + UNSAFE_LINTS:
        args += ["-W", lint]
    return args


def _test_blocks():
    spec = importlib.util.spec_from_file_location("coverage_summary", os.path.join(HERE, "coverage-summary.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.test_blocks


def rust_sources(root):
    files = glob.glob("crates/*/src/**/*.rs", root_dir=root, recursive=True)
    return sorted(files + glob.glob("crates/*/build.rs", root_dir=root))


def web_sources(root):
    files = glob.glob("crates/finder/web/src/**/*.ts", root_dir=root, recursive=True)
    files += glob.glob("crates/finder/web/src/**/*.tsx", root_dir=root, recursive=True)
    return sorted(
        f
        for f in files
        if not re.search(r"\.test\.tsx?$|\.d\.ts$", f) and not f.startswith("crates/finder/web/src/test/")
    )


def unit_of(rel):
    if rel.startswith("crates/finder/web/"):
        return "page"
    return rel.split("/")[1]


def read_lines(path):
    with open(path, encoding="utf-8") as f:
        return f.read().splitlines()


# ---------------------------------------------------------------- prepare


def prepare(out, root):
    src = os.path.join(out, "raw", "src")
    shutil.rmtree(src, ignore_errors=True)
    test_blocks = _test_blocks()
    blocks = {}
    for rel in rust_sources(root):
        lines = read_lines(os.path.join(root, rel))
        ranges = test_blocks(os.path.join(root, rel))
        for a, b in ranges:
            lines[a - 1 : b] = [""] * (b - a + 1)
        blocks[rel] = ranges
        write(os.path.join(src, rel), "\n".join(lines) + "\n")
    for rel in web_sources(root):
        os.makedirs(os.path.dirname(os.path.join(src, rel)), exist_ok=True)
        shutil.copyfile(os.path.join(root, rel), os.path.join(src, rel))
    write(os.path.join(out, "raw", "test-blocks.json"), json.dumps(blocks, indent=1))


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


# ---------------------------------------------------------------- collect


def load_json(path, default=None):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError):
        return default


def json_lines(path):
    try:
        with open(path, encoding="utf-8") as f:
            for line in f:
                try:
                    yield json.loads(line)
                except ValueError:
                    continue
    except OSError:
        return


def rca_functions(rel, unit_json, lines, language):
    """Functions from rust-code-analysis's space tree, with their complexity."""
    found = []

    def walk(space, owners, in_function):
        is_function = space["kind"] == "function"
        if is_function and not (language == "rust" and in_function):
            m = space["metrics"]
            children = sum(c["metrics"]["cyclomatic"]["sum"] for c in space["spaces"] if c["kind"] == "function")
            named = [o for o in owners if o and o != "<anonymous>"]
            name = space["name"] or "<anonymous>"
            if language == "rust":
                name = "::".join(named[-1:] + [name])
            elif name == "<anonymous>":
                name = f"(callback in {named[-1]})" if named else "(callback)"
            start, end = space["start_line"], space["end_line"]
            found.append(
                {
                    "unit": unit_of(rel),
                    "path": rel,
                    "name": name,
                    "line": start,
                    "end_line": end,
                    "language": language,
                    # Rust: closures are part of their function, as the cognitive-complexity spec has it.
                    # TypeScript: each function on its own, as sonarjs counts them.
                    "cyclomatic": int(m["cyclomatic"]["sum"] - (children if language != "rust" else 0)),
                    "cognitive": int(m["cognitive"]["sum"]),
                    "ncloc": int(m["loc"]["ploc"]),
                    "params": int(m["nargs"]["total_functions"]),
                    # rust-code-analysis 0.0.25 does not count `loop` or nest what is in it.
                    "loop": language == "rust" and any(re.search(r"\bloop\s*\{", t) for t in lines[start - 1 : end]),
                }
            )
        owner = space["name"] if space["kind"] in ("impl", "trait", "function") else None
        for child in space["spaces"]:
            walk(child, owners + [owner], in_function or is_function)

    walk(unit_json, [], False)
    return found


def collect_files(out, root, notes):
    raw = os.path.join(out, "raw")
    blocks = load_json(os.path.join(raw, "test-blocks.json"), {})
    parse_errors = set(load_json(os.path.join(raw, "rca-parse-errors.json"), []))
    # sonarjs's figures by file and line, in column order: two functions can start on one line.
    sonar_cc = collections.defaultdict(list)
    for r in sorted(load_json(os.path.join(raw, "eslint-cognitive.json"), []), key=lambda r: r.get("column", 0)):
        sonar_cc[(r["path"], r["line"])].append(r["cognitive"])
    if not sonar_cc:
        notes.append("No sonarjs cognitive complexity (eslint did not run): the page's cognitive figures are 0.")
    files, functions = {}, []
    for rel in rust_sources(root) + web_sources(root):
        language = "rust" if rel.endswith(".rs") else "typescript"
        lines = read_lines(os.path.join(root, rel))
        test_lines = sum(b - a + 1 for a, b in blocks.get(rel, []))
        data = load_json(os.path.join(raw, "rca", rel + ".json"))
        if data is None:
            notes.append(f"rust-code-analysis wrote nothing for {rel}; it is left out.")
            continue
        loc = data["metrics"]["loc"]
        fns = rca_functions(rel, data, lines, language)
        if language == "typescript":
            # rust-code-analysis lists functions outermost first, which is sonarjs's column order.
            left = {k: list(v) for k, v in sonar_cc.items() if k[0] == rel}
            for fn in fns:
                values = left.get((rel, fn["line"]))
                fn["cognitive"] = values.pop(0) if values else 0
            for (_, line), rest in left.items():
                for cc in rest:
                    fns.append(
                        {
                            "unit": "page",
                            "path": rel,
                            "name": f"(function at line {line})",
                            "line": line,
                            "end_line": line,
                            "language": language,
                            "cyclomatic": None,
                            "cognitive": cc,
                            "ncloc": None,
                            "params": None,
                            "loop": False,
                        }
                    )
        functions += fns
        files[rel] = {
            "path": rel,
            "unit": unit_of(rel),
            "language": language,
            "lines": len(lines) - test_lines,
            "test_lines": test_lines,
            "ncloc": int(loc["ploc"]),
            "comment_lines": int(loc["cloc"]),
            "functions": len(fns),
            "cyclomatic": sum(f["cyclomatic"] or 0 for f in fns),
            "cognitive": sum(f["cognitive"] for f in fns),
            "max_cognitive": max((f["cognitive"] for f in fns), default=0),
            "parse_error": rel in parse_errors,
        }
        if rel in parse_errors:
            notes.append(f"rust-code-analysis's grammar could not parse all of {rel}; its figures are approximate.")
    return files, functions


def clippy_groups(path):
    groups = {}
    try:
        with open(path, encoding="utf-8") as f:
            for line in f:
                m = re.match(r"\s*clippy::([a-z]+)\s+(clippy::.*)$", line)
                if m and m.group(1) != "all":
                    for name in m.group(2).split(","):
                        groups[name.strip().replace("-", "_")] = m.group(1)
    except OSError:
        pass
    return groups


def collect_clippy(raw, files, notes):
    groups = clippy_groups(os.path.join(raw, "clippy-lint-groups.txt"))
    issues, seen, generated = [], set(), 0
    for d in json_lines(os.path.join(raw, "clippy.jsonl")):
        if d.get("reason") != "compiler-message":
            continue
        m = d["message"]
        code = (m.get("code") or {}).get("code")
        primary = [s for s in m.get("spans", []) if s["is_primary"]]
        if not code or not primary or m["level"] not in ("warning", "error"):
            continue
        span = primary[0]
        key = (code, span["file_name"], span["line_start"], span["column_start"])
        if key in seen:  # one file compiled into two targets reports twice
            continue
        seen.add(key)
        if span["file_name"] not in files:
            generated += 1  # tonic and prost stubs under target/, or a span outside the measured code
            continue
        name = code.removeprefix("clippy::")
        if code in PANIC_LINTS:
            category, severity = "panic", "Major"
        elif code in UNSAFE_LINTS:
            category, severity = "unsafe", "Critical"
        elif code.startswith("clippy::"):
            category, severity = "smell", CLIPPY_SEVERITY.get(groups.get(code, ""), "Minor")
        else:
            category, severity = "smell", "Major"  # a rustc warning
        url = (
            f"https://rust-lang.github.io/rust-clippy/stable/index.html#{name}"
            if code.startswith("clippy::")
            else "https://doc.rust-lang.org/rustc/lints/listing/index.html"
        )
        issues.append(
            {
                "path": span["file_name"],
                "line": span["line_start"],
                "unit": files[span["file_name"]]["unit"],
                "tool": "clippy",
                "rule": code,
                "group": groups.get(code, "rustc" if not code.startswith("clippy::") else "restriction"),
                "severity": severity,
                "category": category,
                "message": m["message"],
                "url": url,
            }
        )
    if not groups:
        notes.append("clippy's lint groups were not read (clippy-driver -W help): severities default to Minor.")
    if generated:
        notes.append(f"{generated} clippy findings in generated code (the gRPC stubs under target/) are left out.")
    return issues


def collect_eslint(raw, root, files, notes):
    results = load_json(os.path.join(raw, "eslint.json"))
    if results is None:
        notes.append("ESLint did not run: the page has no lint findings in this report.")
        return []
    meta = load_json(os.path.join(raw, "eslint-meta.json"), {})
    issues = []
    for r in results:
        rel = os.path.relpath(r["filePath"], root)
        for m in r["messages"]:
            if m.get("fatal") or not m.get("ruleId"):
                notes.append(f"ESLint could not parse {rel}: {m.get('message')}")
                continue
            rule = m["ruleId"]
            issues.append(
                {
                    "path": rel,
                    "line": m.get("line", 1),
                    "unit": "page",
                    "tool": "eslint",
                    "rule": rule,
                    "group": rule.split("/")[0] if "/" in rule else "eslint",
                    "severity": ESLINT_SEVERITY.get(meta.get(rule, {}).get("type"), "Minor"),
                    "category": "smell",
                    "message": m["message"],
                    "url": meta.get(rule, {}).get("url") or f"https://eslint.org/docs/latest/rules/{rule}",
                }
            )
    return issues


def complexity_issues(functions):
    issues = []
    for fn in functions:
        if fn["cognitive"] > COGNITIVE_THRESHOLD:
            over = fn["cognitive"] - COGNITIVE_THRESHOLD
            issues.append(
                {
                    "path": fn["path"],
                    "line": fn["line"],
                    "unit": fn["unit"],
                    "tool": "rust-code-analysis" if fn["language"] == "rust" else "sonarjs",
                    "rule": "cognitive-complexity",
                    "group": "complexity",
                    "severity": "Critical",
                    "category": "smell",
                    "message": f"`{fn['name']}` has cognitive complexity {fn['cognitive']} "
                    f"(threshold {COGNITIVE_THRESHOLD}){' or more: it has a `loop`' if fn['loop'] else ''}.",
                    "url": "https://sonarsource.github.io/rspec/#/rspec/S3776",
                    "debt_min": 5 + over,
                }
            )
    return issues


def collect_duplication(raw, out, files, notes):
    report = load_json(os.path.join(raw, "jscpd", "jscpd-report.json"))
    if report is None:
        notes.append("jscpd did not run: no duplication figures.")
        return []
    src = os.path.join(out, "raw", "src")
    clones = []
    for c in report.get("duplicates", []):
        a, b = c["firstFile"], c["secondFile"]
        pa, pb = os.path.relpath(a["name"], src), os.path.relpath(b["name"], src)
        if pa not in files or pb not in files:
            continue
        clones.append(
            {
                "tokens": c["tokens"],
                "lines": c["lines"],
                "block": c["tokens"] >= DUP_TOKENS and c["lines"] >= DUP_LINES,
                "a": {"path": pa, "start": a["start"], "end": a["end"]},
                "b": {"path": pb, "start": b["start"], "end": b["end"]},
            }
        )
    clones.sort(key=lambda c: (-c["lines"], c["a"]["path"], c["a"]["start"]))
    return clones


def collect_security(raw, root, files):
    advisories = {"cargo": None, "npm": None}
    deny = list(json_lines(os.path.join(raw, "deny.jsonl")))
    if deny:
        cargo = []
        for d in deny:
            f = d.get("fields", {})
            adv = f.get("advisory")
            if d.get("type") == "diagnostic" and adv:
                cargo.append(
                    {
                        "id": adv.get("id"),
                        "package": adv.get("package"),
                        "title": adv.get("title"),
                        "kind": f.get("code"),
                        "url": adv.get("url") or f"https://rustsec.org/advisories/{adv.get('id')}",
                        # deny.toml's ignore list turns an advisory into a note; an active one is an error.
                        "accepted": f.get("severity") not in ("error", "warning"),
                    }
                )
        advisories["cargo"] = cargo
    audit, prod = load_json(os.path.join(raw, "npm-audit.json")), load_json(os.path.join(raw, "npm-audit-prod.json"))
    if audit is not None and "vulnerabilities" in audit:
        runtime = set((prod or {}).get("vulnerabilities", {}))
        npm = []
        for name, v in sorted(audit["vulnerabilities"].items()):
            titles = [x for x in v.get("via", []) if isinstance(x, dict)]
            npm.append(
                {
                    "package": name,
                    "severity": v.get("severity"),
                    "range": v.get("range"),
                    "dev_only": name not in runtime,
                    "fix_available": bool(v.get("fixAvailable")),
                    "advisories": [{"title": x.get("title"), "url": x.get("url")} for x in titles],
                }
            )
        advisories["npm"] = npm
    unused = None
    try:
        with open(os.path.join(raw, "machete.txt"), encoding="utf-8") as f:
            text = f.read()
        unused, crate = [], None
        for line in text.splitlines():
            if " -- " in line and line.rstrip().endswith(":"):
                crate = line.split(" -- ")[0].strip()
            elif line.startswith("\t") and crate:
                unused.append({"crate": crate, "dependency": line.strip()})
    except OSError:
        pass
    forbid = {}
    for rel in files:
        if re.match(r"crates/[^/]+/src/(lib|main)\.rs$", rel):
            text = "\n".join(read_lines(os.path.join(root, rel)))
            unit = unit_of(rel)
            forbid[unit] = forbid.get(unit, False) or bool(re.search(r"#!\[(forbid|deny)\(unsafe_code\)\]", text))
    return {"advisories": advisories, "unused_dependencies": unused, "unsafe_forbidden": dict(sorted(forbid.items()))}


def collect_suppressions(root, files, blocks):
    found = []
    for rel, row in files.items():
        test = blocks.get(rel, [])
        for n, text in enumerate(read_lines(os.path.join(root, rel)), 1):
            if any(a <= n <= b for a, b in test):
                continue
            if row["language"] == "rust":
                m = re.search(r"#!?\[(?:allow|expect)\(([^)]*)\)\]", text)
            else:
                m = re.search(r"(eslint-disable(?:-next-line|-line)?[^*]*|@ts-(?:ignore|expect-error|nocheck))", text)
            if m:
                found.append({"path": rel, "line": n, "unit": row["unit"], "what": m.group(1).strip()})
    return found


# ---------------------------------------------------------------- measure


def grade(ratio):
    if ratio is None:
        return None
    for letter, limit in GRADES:
        if ratio <= limit:
            return letter
    return "E"


def tally(rows, issues, name_key, name):
    """One row of figures for a set of files: a crate, the page, or everything."""
    paths = {r["path"] for r in rows}
    mine = [i for i in issues if i["path"] in paths]
    smells = [i for i in mine if i["category"] == "smell" and i["rule"] != "cognitive-complexity"]
    ncloc = sum(r["ncloc"] for r in rows)
    lines = sum(r["lines"] for r in rows)
    debt = sum(i["debt_min"] for i in mine)
    dup_lines = sum(r["duplicated_lines"] for r in rows)
    out = {
        name_key: name,
        "files": len(rows),
        "lines": lines,
        "ncloc": ncloc,
        "comment_lines": sum(r["comment_lines"] for r in rows),
        "functions": sum(r["functions"] for r in rows),
        "cyclomatic": sum(r["cyclomatic"] for r in rows),
        "cognitive": sum(r["cognitive"] for r in rows),
        "max_cognitive": max((r["max_cognitive"] for r in rows), default=0),
        "complex_functions": sum(1 for i in mine if i["rule"] == "cognitive-complexity"),
        "duplicated_lines": dup_lines,
        "duplicated_blocks": sum(r["duplicated_blocks"] for r in rows),
        "duplication_percent": round(100.0 * dup_lines / lines, 2) if lines else None,
        "small_clones": sum(r["small_clones"] for r in rows),
        "smells": len(smells),
        "smells_by_severity": {s: sum(1 for i in smells if i["severity"] == s) for s in SEVERITIES},
        "panic_paths": sum(1 for i in mine if i["category"] == "panic"),
        "unsafe": sum(1 for i in mine if i["rule"] == "unsafe_code"),
        "debt_minutes": debt,
        "debt_ratio_percent": round(100.0 * debt / (DEV_MINUTES_PER_LINE * ncloc), 3) if ncloc else None,
    }
    out["grade"] = grade(debt / (DEV_MINUTES_PER_LINE * ncloc) if ncloc else None)
    return out


def rules_of(issues):
    """The rules table: each rule's findings, debt and where it fires, most findings first."""
    rules = {}
    for i in issues:
        r = rules.setdefault(
            i["rule"],
            {
                **{k: i[k] for k in ("rule", "tool", "group", "severity", "category", "url")},
                "count": 0,
                "debt_minutes": 0,
                "units": set(),
            },
        )
        r["count"] += 1
        r["debt_minutes"] += i["debt_min"]
        r["units"].add(i["unit"])
    return sorted(({**r, "units": sorted(r["units"])} for r in rules.values()), key=lambda r: (-r["count"], r["rule"]))


def measure(out, root):
    raw = os.path.join(out, "raw")
    notes = []
    files, functions = collect_files(out, root, notes)
    issues = collect_clippy(raw, files, notes) + collect_eslint(raw, root, files, notes) + complexity_issues(functions)
    for i in issues:
        i.setdefault("debt_min", SMELL_MINUTES if i["category"] == "smell" and i["severity"] != "Info" else 0)
    clones = collect_duplication(raw, out, files, notes)
    blocks = load_json(os.path.join(raw, "test-blocks.json"), {})

    dup_lines = collections.defaultdict(set)
    for c in clones:
        for side in ("a", "b"):
            f = c[side]
            files[f["path"]].setdefault("_frags", []).append(c["block"])
            if c["block"]:
                dup_lines[f["path"]].update(range(f["start"], f["end"] + 1))
    for rel, row in files.items():
        frags = row.pop("_frags", [])
        row["duplicated_lines"] = len(dup_lines[rel])
        row["duplicated_blocks"] = sum(1 for b in frags if b)
        row["small_clones"] = sum(1 for b in frags if not b)
    file_rows = []
    for rel, row in files.items():
        t = tally([row], issues, "path", rel)
        file_rows.append({**row, **{k: v for k, v in t.items() if k not in row}})

    rust_files = [r for r in file_rows if r["language"] == "rust"]
    web_files = [r for r in file_rows if r["language"] == "typescript"]
    crates = [
        tally([r for r in rust_files if r["unit"] == name], issues, "crate", name)
        for name in sorted({r["unit"] for r in rust_files})
    ]
    functions.sort(key=lambda f: (-f["cognitive"], -(f["cyclomatic"] or 0), f["path"], f["line"]))
    issues.sort(key=lambda i: (i["path"], i["line"], SEVERITIES.index(i["severity"])))
    security = collect_security(raw, root, files)
    security["unsafe_sites"] = [i for i in issues if i["category"] == "unsafe"]
    security["suppressions"] = collect_suppressions(root, files, blocks)
    return {
        "rust": {
            "totals": tally(rust_files, issues, "crate", "workspace"),
            "crates": crates,
            "files": sorted(rust_files, key=lambda r: r["path"]),
        },
        "web": {
            "totals": tally(web_files, issues, "unit", "page"),
            "files": sorted(web_files, key=lambda r: r["path"]),
        },
        "totals": tally(file_rows, issues, "unit", "all"),
        "functions": functions,
        "issues": issues,
        "rules": rules_of(issues),
        "duplications": clones,
        "security": security,
        "notes": notes,
    }


def stamp(root, raw):
    def git(*args):
        try:
            return subprocess.run(["git", "-C", root, *args], capture_output=True, text=True, check=True).stdout.strip()
        except (OSError, subprocess.CalledProcessError):
            return None

    tools = {}
    try:
        with open(os.path.join(raw, "tools.txt"), encoding="utf-8") as f:
            for line in f:
                if ":" in line:
                    k, v = line.split(":", 1)
                    tools[k.strip()] = v.strip()
    except OSError:
        pass
    return {
        "commit": git("rev-parse", "--short", "HEAD") or "unknown",
        "commit_sha": git("rev-parse", "HEAD"),
        # Uncommitted changes in the measured code make the commit stamp only approximate.
        "dirty": bool(git("status", "--porcelain", "--", "crates")),
        "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
        "tools": tools,
        "model": {
            "development_minutes_per_line": DEV_MINUTES_PER_LINE,
            "smell_minutes": SMELL_MINUTES,
            "cognitive_threshold": COGNITIVE_THRESHOLD,
            "cognitive_minutes": "5 + 1 per point over the threshold",
            "grade_limits_percent": {k: 100 * v for k, v in GRADES},
            "duplicated_block": {"min_tokens": DUP_TOKENS, "min_lines": DUP_LINES},
            "small_clone": {"min_tokens": JSCPD_TOKENS, "min_lines": JSCPD_LINES},
        },
    }


# ---------------------------------------------------------------- markdown


def fmt_int(n):
    return "—" if n is None else f"{n:,}"


def fmt_debt(minutes):
    return f"{minutes} min" if minutes < 60 else f"{minutes / 60:.1f} h"


def plural(n, word, words=None):
    return f"{n:,} {word if n == 1 else words or word + 's'}"


def fmt_pct(p, digits=1):
    return "—" if p is None else f"{p:.{digits}f}%"


def dup_text(r):
    small = f" · {r['small_clones']} small" if r["small_clones"] else ""
    return fmt_pct(r["duplication_percent"]) + small


def advisory_line(security):
    cargo, npm = security["advisories"]["cargo"], security["advisories"]["npm"]
    parts = []
    if cargo is None:
        parts.append("cargo-deny did not run")
    else:
        active = [a for a in cargo if not a["accepted"]]
        parts.append(f"{len(active)} active Rust advisories ({len(cargo) - len(active)} accepted in deny.toml)")
    if npm is None:
        parts.append("npm audit did not run")
    else:
        own = [v for v in npm if v["advisories"]]  # the rest only depend on one of these
        runtime = [v for v in own if not v["dev_only"]]
        if own:
            parts.append(
                f"npm audit: {plural(len(own), 'vulnerable package')} in the page's dependencies, {len(runtime)} at runtime ("
                + ", ".join(f"{v['package']} {v['severity']}{', dev only' if v['dev_only'] else ''}" for v in own)
                + ")"
            )
        else:
            parts.append("npm audit: no advisories in the page's dependencies")
    unused = security["unused_dependencies"]
    if unused is None:
        parts.append("cargo-machete did not run")
    elif unused:
        parts.append(f"{plural(len(unused), 'unused dependency', 'unused dependencies')} (cargo-machete)")
    return "; ".join(parts)


def markdown(report):
    t = report["totals"]
    md = [f"## Quality at `{report['commit']}`" + (" (uncommitted changes)" if report["dirty"] else ""), ""]
    md.append(
        f"**Maintainability {t['grade']}**: {fmt_pct(t['debt_ratio_percent'], 2)} debt ratio, "
        f"{fmt_debt(t['debt_minutes'])} of remediation over {fmt_int(t['ncloc'])} code lines "
        f"in {fmt_int(t['functions'])} functions. {fmt_int(t['smells'])} lint findings, "
        f"{t['complex_functions']} functions over cognitive complexity {COGNITIVE_THRESHOLD}, "
        f"{fmt_pct(t['duplication_percent'])} duplicated lines, {t['panic_paths']} panic paths, "
        f"{plural(t['unsafe'], '`unsafe` site')}. {advisory_line(report['security'])}. "
        "Test code is left out. docs/quality.md explains the figures."
    )
    md += [
        "",
        "| Crate | Grade | Debt ratio | Debt | Code lines | Functions | Cognitive > 15 | Max cognitive "
        "| Duplication | Lint findings | Panic paths | Unsafe |",
        "|---|:-:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    rows = [(f"`{c['crate']}`", c) for c in report["rust"]["crates"]] + [
        ("page (`crates/finder/web`)", report["web"]["totals"])
    ]
    for label, r in rows + [("**all**", t)]:
        md.append(
            f"| {label} | {r['grade'] or '—'} | {fmt_pct(r['debt_ratio_percent'], 2)} | {fmt_debt(r['debt_minutes'])} "
            f"| {fmt_int(r['ncloc'])} | {fmt_int(r['functions'])} | {r['complex_functions']} | {r['max_cognitive']} "
            f"| {dup_text(r)} | {fmt_int(r['smells'])} | {r['panic_paths']} | {r['unsafe']} |"
        )
    md += [
        "",
        f"Worst {WORST_MD} functions by cognitive complexity:",
        "",
        "| Function | File | Cognitive | Cyclomatic | Lines |",
        "|---|---|---:|---:|---:|",
    ]
    for f in report["functions"][:WORST_MD]:
        cc = f"{f['cognitive']}{'+' if f['loop'] else ''}"
        md.append(
            f"| `{f['name']}` | `{f['path']}:{f['line']}` | {cc} | {fmt_int(f['cyclomatic'])} | {fmt_int(f['ncloc'])} |"
        )
    if any(f["loop"] for f in report["functions"][:WORST_MD]):
        md += ["", "\\+ has a `loop`, which rust-code-analysis does not count: the true figure is higher."]
    md += ["", f"Top {RULES_MD} rules:", "", "| Rule | Kind | Severity | Findings | Debt |", "|---|---|---|---:|---:|"]
    kinds = {"smell": "code smell", "panic": "panic path", "unsafe": "unsafe"}
    for r in report["rules"][:RULES_MD]:
        md.append(
            f"| `{r['rule']}` | {kinds[r['category']]} | {r['severity']} | {r['count']} | {fmt_debt(r['debt_minutes'])} |"
        )
    if report["notes"]:
        md += ["", "Notes:", ""] + [f"- {n}" for n in report["notes"]]
    md.append("")
    return "\n".join(md)


# ---------------------------------------------------------------- diff


def diff(old_path, new_path):
    old, new = load_json(old_path), load_json(new_path)
    if old is None or new is None:
        sys.exit("diff: cannot read both files")

    def units(r):
        rows = {c["crate"]: c for c in r["rust"]["crates"]}
        rows["page"] = r["web"]["totals"]
        rows["all"] = r["totals"]
        return rows

    a, b = units(old), units(new)
    keys = [
        ("grade", "Grade"),
        ("debt_ratio_percent", "Debt ratio %"),
        ("debt_minutes", "Debt min"),
        ("ncloc", "Code lines"),
        ("smells", "Lint findings"),
        ("complex_functions", "Cognitive > 15"),
        ("max_cognitive", "Max cognitive"),
        ("duplication_percent", "Duplication %"),
        ("panic_paths", "Panic paths"),
        ("unsafe", "Unsafe"),
    ]
    md = [
        f"## Quality `{old['commit']}` → `{new['commit']}`",
        "",
        "| Unit | " + " | ".join(h for _, h in keys) + " |",
        "|---|" + "---:|" * len(keys),
    ]
    for name in sorted(set(a) | set(b), key=lambda n: (n == "all", n == "page", n)):
        cells = []
        for k, _ in keys:
            x, y = a.get(name, {}).get(k), b.get(name, {}).get(k)
            if x == y:
                cells.append(str(y))
            elif isinstance(x, (int, float)) and isinstance(y, (int, float)):
                cells.append(f"{y} ({y - x:+.4g})")
            else:
                cells.append(f"{x} → {y}")
        md.append(f"| {name} | " + " | ".join(cells) + " |")
    fa = {(f["path"], f["name"]): f["cognitive"] for f in old["functions"]}
    moved = sorted(
        ((f["cognitive"] - fa.get((f["path"], f["name"]), 0), f) for f in new["functions"]),
        key=lambda x: -abs(x[0]),
    )
    moved = [(d, f) for d, f in moved if d][:15]
    if moved:
        md += [
            "",
            "Functions whose cognitive complexity moved most:",
            "",
            "| Function | File | Now | Change |",
            "|---|---|---:|---:|",
        ]
        md += [f"| `{f['name']}` | `{f['path']}` | {f['cognitive']} | {d:+} |" for d, f in moved]
    print("\n".join(md))


# ---------------------------------------------------------------- html

CSS = """
:root { color-scheme: light; --bg: #f7f7f5; --panel: #ffffff; --ink: #1c2024; --muted: #5d6670; --line: #e2e4e7;
  --soft: #f0f1f3; --accent: #2b59c3; --a: #1f8a3b; --b: #8fbf2a; --c: #e2b007; --d: #e2711d; --e: #cf3341;
  --blocker: #b3261e; --critical: #cf3341; --major: #d9731a; --minor: #3d6fb6; --info: #7a838c;
  --dup: #c58af9; --test: #eceef1; --hit: #fff4d6; }
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { color-scheme: dark; --bg: #121417; --panel: #1a1d21;
  --ink: #e4e6e9; --muted: #9aa3ad; --line: #2c3137; --soft: #22262b; --accent: #7aa2ff; --test: #1f2226; --hit: #3a3220;
  --minor: #7aa2ff; --info: #8d96a0; } }
:root[data-theme="dark"] { color-scheme: dark; --bg: #121417; --panel: #1a1d21; --ink: #e4e6e9; --muted: #9aa3ad;
  --line: #2c3137; --soft: #22262b; --accent: #7aa2ff; --test: #1f2226; --hit: #3a3220; --minor: #7aa2ff; --info: #8d96a0; }
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--ink); font: 14px/1.45 system-ui, -apple-system, "Segoe UI", sans-serif; }
a { color: var(--accent); text-decoration: none; } a:hover { text-decoration: underline; }
code, pre, .mono { font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace; font-size: 12.5px; }
header.top { display: flex; flex-wrap: wrap; gap: 4px 16px; align-items: baseline; padding: 12px 16px;
  border-bottom: 1px solid var(--line); background: var(--panel); }
header.top .brand { font-weight: 650; color: var(--ink); } header.top .stamp { color: var(--muted); }
main { max-width: 1240px; margin: 0 auto; padding: 16px; }
h1 { font-size: 22px; margin: 8px 0 4px; word-break: break-all; } h2 { font-size: 16px; margin: 28px 0 8px; }
p.lede { color: var(--muted); margin: 0 0 16px; max-width: 80ch; }
.crumbs { color: var(--muted); font-size: 13px; }
.tiles { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 8px; margin: 12px 0; }
.tile { background: var(--panel); border: 1px solid var(--line); border-radius: 8px; padding: 10px 12px; }
.tile .k { color: var(--muted); font-size: 12px; } .tile .v { font-size: 20px; font-weight: 650; font-variant-numeric: tabular-nums; }
.tile .s { color: var(--muted); font-size: 12px; }
.grade { display: inline-grid; place-items: center; width: 22px; height: 22px; border-radius: 5px; font-weight: 700;
  font-size: 13px; color: #fff; } .grade.big { width: 34px; height: 34px; font-size: 20px; border-radius: 7px; }
.g-A { background: var(--a); } .g-B { background: var(--b); color: #1c2024; } .g-C { background: var(--c); color: #1c2024; }
.g-D { background: var(--d); } .g-E { background: var(--e); } .g-none { background: var(--soft); color: var(--muted); }
.wrap { overflow-x: auto; background: var(--panel); border: 1px solid var(--line); border-radius: 8px; }
table { border-collapse: collapse; width: 100%; }
th, td { padding: 6px 10px; border-bottom: 1px solid var(--line); text-align: left; vertical-align: top; }
th { font-size: 12px; color: var(--muted); font-weight: 600; white-space: nowrap; background: var(--soft); }
table.sortable th { cursor: pointer; user-select: none; } table.sortable th[data-dir="asc"]::after { content: " ▲"; }
table.sortable th[data-dir="desc"]::after { content: " ▼"; }
tbody tr:last-child td { border-bottom: 0; } td.num { text-align: right; font-variant-numeric: tabular-nums; white-space: nowrap; }
td.path { word-break: break-all; } tr.total td { font-weight: 650; }
.sev { display: inline-block; padding: 0 6px; border-radius: 4px; font-size: 11.5px; font-weight: 600; color: #fff; }
.sev-Blocker { background: var(--blocker); } .sev-Critical { background: var(--critical); } .sev-Major { background: var(--major); }
.sev-Minor { background: var(--minor); } .sev-Info { background: var(--info); }
.muted { color: var(--muted); } .note { background: var(--panel); border: 1px solid var(--line); border-left: 3px solid var(--c);
  border-radius: 6px; padding: 8px 12px; margin: 8px 0; }
ul.plain { margin: 6px 0; padding-left: 20px; }
.src { background: var(--panel); border: 1px solid var(--line); border-radius: 8px; overflow-x: auto; }
.src .ln { display: grid; grid-template-columns: 56px 4px 1fr; min-width: max-content; }
.src .ln > a { color: var(--muted); text-align: right; padding-right: 8px; user-select: none; }
.src .ln pre { margin: 0; padding: 0 12px; white-space: pre; } .src .ln .bar { background: transparent; }
.src .ln.dup .bar { background: var(--dup); } .src .ln.test { background: var(--test); color: var(--muted); }
.src .ln.hit { background: var(--hit); } .src .ln:target { outline: 2px solid var(--accent); outline-offset: -2px; }
.src .msg { margin: 2px 0 4px 60px; padding: 3px 10px; border-left: 3px solid var(--major); background: var(--soft);
  font-size: 12.5px; white-space: normal; max-width: 110ch; }
.src .msg.sev-b-Info { border-color: var(--info); } .src .msg.sev-b-Minor { border-color: var(--minor); }
.src .msg.sev-b-Critical, .src .msg.sev-b-Blocker { border-color: var(--critical); }
.legend { display: flex; flex-wrap: wrap; gap: 6px 16px; color: var(--muted); font-size: 12px; margin: 6px 0; }
.legend i { display: inline-block; width: 12px; height: 12px; vertical-align: -2px; margin-right: 4px; border-radius: 2px; }
footer { color: var(--muted); font-size: 12px; padding: 24px 16px; text-align: center; }
@media (max-width: 640px) { th, td { padding: 5px 7px; } .tile .v { font-size: 17px; } }
"""

JS = """
for (const table of document.querySelectorAll("table.sortable")) {
  const heads = [...table.tHead.rows[0].cells];
  heads.forEach((th, i) => {
    th.tabIndex = 0;
    const sort = () => {
      const asc = th.dataset.dir !== "asc";
      heads.forEach((h) => delete h.dataset.dir);
      th.dataset.dir = asc ? "asc" : "desc";
      const key = (tr) => {
        const c = tr.cells[i];
        const v = c.dataset.v ?? c.textContent.trim().toLowerCase();
        return c.dataset.v !== undefined ? Number(v) : v;
      };
      const body = table.tBodies[0];
      const rows = [...body.rows].filter((r) => !r.classList.contains("total"));
      rows.sort((a, b) => (key(a) < key(b) ? -1 : key(a) > key(b) ? 1 : 0) * (asc ? 1 : -1));
      body.prepend(...rows);
    };
    th.addEventListener("click", sort);
    th.addEventListener("keydown", (e) => e.key === "Enter" && sort());
  });
}
"""

KINDS = {"smell": "code smell", "panic": "panic path", "unsafe": "unsafe"}


def esc(text):
    return html.escape(str(text), quote=True)


def slug(path):
    return path.replace("/", "~") + ".html"


def badge(letter, big=False):
    cls = f"g-{letter}" if letter else "g-none"
    return f'<span class="grade {cls}{" big" if big else ""}" title="Maintainability {letter or "—"}">{letter or "—"}</span>'


def num(value, shown=None, sort=None):
    v = sort if sort is not None else (value if isinstance(value, (int, float)) else -1)
    return f'<td class="num" data-v="{v}">{esc(shown if shown is not None else fmt_int(value))}</td>'


def sev(severity):
    return f'<span class="sev sev-{severity}">{severity}</span>'


def tile(key, value, sub=""):
    return f'<div class="tile"><div class="k">{key}</div><div class="v">{value}</div><div class="s">{sub}</div></div>'


def table(heads, rows, sortable=True):
    th = "".join(f"<th>{h}</th>" for h in heads)
    cls = ' class="sortable"' if sortable else ""
    body = "".join(rows) if rows else f'<tr><td colspan="{len(heads)}" class="muted">None.</td></tr>'
    return f'<div class="wrap"><table{cls}><thead><tr>{th}</tr></thead><tbody>{body}</tbody></table></div>'


def dup_cell(r):
    return num(
        r["duplication_percent"], dup_text(r), r["duplication_percent"] if r["duplication_percent"] is not None else -1
    )


def smell_cell(r):
    by = r["smells_by_severity"]
    title = ", ".join(f"{by[s]} {s}" for s in SEVERITIES if by[s])
    return f'<td class="num" data-v="{r["smells"]}" title="{esc(title)}">{fmt_int(r["smells"])}</td>'


UNIT_HEADS = [
    "Grade",
    "Debt ratio",
    "Debt",
    "Code lines",
    "Files",
    "Functions",
    "Cognitive &gt; 15",
    "Max cognitive",
    "Duplication",
    "Lint findings",
    "Panic paths",
    "Unsafe",
]


def unit_cells(r):
    return "".join(
        [
            f'<td data-v="{"ABCDE".index(r["grade"]) if r["grade"] else 9}">{badge(r["grade"])}</td>',
            num(r["debt_ratio_percent"], fmt_pct(r["debt_ratio_percent"], 2)),
            num(r["debt_minutes"], fmt_debt(r["debt_minutes"])),
            num(r["ncloc"]),
            num(r["files"]),
            num(r["functions"]),
            num(r["complex_functions"]),
            num(r["max_cognitive"]),
            dup_cell(r),
            smell_cell(r),
            num(r["panic_paths"]),
            num(r["unsafe"]),
        ]
    )


def function_rows(functions, prefix, in_file=False):
    rows = []
    for f in functions:
        if in_file:
            where = f'<a href="#L{f["line"]}">{f["line"]}</a>'
        else:
            where = f'<a href="{prefix}files/{slug(f["path"])}#L{f["line"]}">{esc(f["path"])}:{f["line"]}</a>'
        cc = f"{f['cognitive']}+" if f["loop"] else str(f["cognitive"])
        title = (
            ' title="Has a loop, which rust-code-analysis does not count: the true figure is higher."'
            if f["loop"]
            else ""
        )
        rows.append(
            f"<tr><td><code>{esc(f['name'])}</code></td>"
            + ("" if in_file else f"<td>{esc(f['unit'])}</td>")
            + f'<td class="{"num" if in_file else "path"}">{where}</td><td class="num" data-v="{f["cognitive"]}"{title}>{cc}</td>'
            + num(f["cyclomatic"])
            + num(f["ncloc"])
            + num(f["params"])
            + "</tr>"
        )
    return rows


FUNCTION_HEADS = ["Function", "Unit", "Where", "Cognitive", "Cyclomatic", "Lines", "Params"]


def rule_rows(rules):
    return [
        f'<tr><td><a href="{esc(r["url"])}"><code>{esc(r["rule"])}</code></a></td><td>{KINDS[r["category"]]}</td>'
        f'<td data-v="{SEVERITIES.index(r["severity"])}">{sev(r["severity"])}</td><td>{esc(r["tool"])}</td>'
        + num(r["count"])
        + num(r["debt_minutes"], fmt_debt(r["debt_minutes"]))
        + f"<td>{esc(', '.join(r['units']))}</td></tr>"
        for r in rules
    ]


RULE_HEADS = ["Rule", "Kind", "Severity", "Tool", "Findings", "Debt", "Where"]


def clone_rows(clones, prefix):
    rows = []
    for c in clones:
        sides = [
            f'<a href="{prefix}files/{slug(s["path"])}#L{s["start"]}">{esc(s["path"])}:{s["start"]}–{s["end"]}</a>'
            for s in (c["a"], c["b"])
        ]
        kind = "block" if c["block"] else "small"
        rows.append(
            f'<tr><td class="path">{sides[0]}</td><td class="path">{sides[1]}</td>{num(c["lines"])}{num(c["tokens"])}<td>{kind}</td></tr>'
        )
    return rows


CLONE_HEADS = ["First", "Second", "Lines", "Tokens", "Kind"]


def page(title, prefix, report, body):
    stamp_text = (
        f"commit <code>{esc(report['commit'])}</code> · {esc(report['generated_at'].replace('T', ' ')[:16])} UTC"
    )
    if report["dirty"]:
        stamp_text += " · uncommitted changes"
    return (
        '<!doctype html>\n<html lang="en"><head><meta charset="utf-8">'
        '<meta name="viewport" content="width=device-width, initial-scale=1">'
        f"<title>{esc(title)}</title><style>{CSS}</style></head><body>"
        f'<header class="top"><a class="brand" href="{prefix}index.html">plate-solver code quality</a>'
        f'<span class="stamp">{stamp_text}</span></header><main>{body}</main>'
        "<footer>scripts/quality.sh wrote this. docs/quality.md explains every figure.</footer>"
        f"<script>{JS}</script></body></html>\n"
    )


def unit_tiles(r):
    by = r["smells_by_severity"]
    return (
        '<div class="tiles">'
        + "".join(
            [
                tile(
                    "Maintainability", badge(r["grade"], big=True), f"{fmt_pct(r['debt_ratio_percent'], 2)} debt ratio"
                ),
                tile("Technical debt", fmt_debt(r["debt_minutes"]), "to clear every finding"),
                tile("Code lines", fmt_int(r["ncloc"]), f"{fmt_int(r['comment_lines'])} comment lines"),
                tile(
                    "Functions",
                    fmt_int(r["functions"]),
                    f"{r['complex_functions']} over cognitive {COGNITIVE_THRESHOLD}",
                ),
                tile("Max cognitive", fmt_int(r["max_cognitive"]), f"{fmt_int(r['cyclomatic'])} cyclomatic in all"),
                tile(
                    "Duplication",
                    fmt_pct(r["duplication_percent"]),
                    f"{r['duplicated_blocks']} blocks, {r['small_clones']} small clones",
                ),
                tile(
                    "Lint findings",
                    fmt_int(r["smells"]),
                    ", ".join(f"{by[s]} {s}" for s in SEVERITIES if by[s]) or "none",
                ),
                tile("Panic paths", fmt_int(r["panic_paths"]), "unwrap, expect, panic! and kin"),
                tile("Unsafe", fmt_int(r["unsafe"]), "unsafe blocks, fns and impls"),
            ]
        )
        + "</div>"
    )


def render_index(report):
    t, sec = report["totals"], report["security"]
    units = [(c["crate"], c) for c in report["rust"]["crates"]] + [("page", report["web"]["totals"])]
    rows = [
        f'<tr><td><a href="units/{u}.html">{"page (crates/finder/web)" if u == "page" else esc(u)}</a></td>{unit_cells(r)}</tr>'
        for u, r in units
    ] + [f'<tr class="total"><td>all</td>{unit_cells(t)}</tr>']
    body = [
        f"<h1>Code quality at {esc(report['commit'])}</h1>",
        '<p class="lede">The Rust workspace (each crate\'s <code>src/</code> and build script) and the finder page '
        "(<code>crates/finder/web/src</code>), test code left out. The grade is SonarQube's maintainability rating; "
        'the other figures sit beside it. <a href="#model">How it is measured</a>.</p>',
        unit_tiles(t),
    ]
    body += [
        f'<p class="lede">{esc(advisory_line(sec))}. {len(sec["suppressions"])} lint suppressions '
        "(<code>#[allow]</code>, <code>eslint-disable</code>) in measured code.</p>"
    ]
    body += ["<h2>Crates and the page</h2>", table(["Unit"] + UNIT_HEADS, rows)]
    body += [
        f"<h2>Worst {WORST_HTML} functions by cognitive complexity</h2>",
        table(FUNCTION_HEADS, function_rows(report["functions"][:WORST_HTML], "")),
        '<p class="muted">A + marks a Rust function with a <code>loop</code>: rust-code-analysis counts neither the '
        "loop nor the nesting inside it, so the true figure is higher.</p>",
    ]
    body += ["<h2>Rules</h2>", table(RULE_HEADS, rule_rows(report["rules"]))]
    blocks = [c for c in report["duplications"] if c["block"]]
    body += [
        "<h2>Duplication</h2>",
        f'<p class="lede">{len(blocks)} duplicated blocks (at least {DUP_TOKENS} tokens over {DUP_LINES} lines, '
        f"SonarQube's size) make the percentage. {len(report['duplications']) - len(blocks)} smaller clones "
        f"(from {JSCPD_TOKENS} tokens over {JSCPD_LINES} lines, jscpd's default) are listed but not counted.</p>",
        table(CLONE_HEADS, clone_rows(report["duplications"], "")),
    ]
    body.append(render_security(sec, ""))
    body.append(render_model(report))
    return page("Code quality", "", report, "".join(body))


def render_security(sec, prefix):
    out = ["<h2>Security and reliability</h2>"]
    sites = [
        f'<tr><td class="path"><a href="{prefix}files/{slug(i["path"])}#L{i["line"]}">{esc(i["path"])}:{i["line"]}</a></td>'
        f"<td><code>{esc(i['rule'])}</code></td><td>{esc(i['message'])}</td></tr>"
        for i in sec["unsafe_sites"]
    ]
    out += ["<h3>unsafe</h3>", table(["Where", "Lint", "Message"], sites, sortable=False)]
    forbid = ", ".join(f"{esc(k)} {'yes' if v else 'no'}" for k, v in sec["unsafe_forbidden"].items())
    out.append(f'<p class="muted"><code>#![forbid(unsafe_code)]</code> at the crate root: {forbid}.</p>')
    cargo, npm = sec["advisories"]["cargo"], sec["advisories"]["npm"]
    out.append("<h3>Dependency advisories</h3>")
    if cargo is None:
        out.append('<p class="note">cargo-deny did not run.</p>')
    else:
        out.append(
            table(
                ["Advisory", "Crate", "Title", "Status"],
                [
                    f'<tr><td><a href="{esc(a["url"])}">{esc(a["id"])}</a></td><td>{esc(a["package"])}</td><td>{esc(a["title"])}</td>'
                    f"<td>{'accepted in deny.toml' if a['accepted'] else '<strong>active</strong>'}</td></tr>"
                    for a in cargo
                ],
                sortable=False,
            )
        )
    if npm is None:
        out.append('<p class="note">npm audit did not run (it needs the network).</p>')
    else:
        rows = []
        for v in npm:
            titles = "".join(f'<li><a href="{esc(x["url"])}">{esc(x["title"])}</a></li>' for x in v["advisories"])
            what = (
                f'<ul class="plain">{titles}</ul>'
                if titles
                else '<span class="muted">through a dependency above</span>'
            )
            rows.append(
                f"<tr><td>{esc(v['package'])}</td><td>{esc(v['severity'])}</td><td>{esc(v['range'] or '')}</td>"
                f"<td>{'dev only' if v['dev_only'] else '<strong>runtime</strong>'}</td><td>{what}</td></tr>"
            )
        out.append('<p class="muted">The page\'s npm dependencies (<code>npm audit</code> on its lockfile):</p>')
        out.append(table(["Package", "Severity", "Range", "Used by", "Advisories"], rows, sortable=False))
    unused = sec["unused_dependencies"]
    out.append("<h3>Unused dependencies (cargo-machete)</h3>")
    if unused is None:
        out.append('<p class="note">cargo-machete did not run.</p>')
    else:
        out.append(
            table(
                ["Crate", "Dependency"],
                [f"<tr><td>{esc(u['crate'])}</td><td>{esc(u['dependency'])}</td></tr>" for u in unused],
                sortable=False,
            )
        )
    out.append("<h3>Lint suppressions</h3>")
    out.append(
        table(
            ["Where", "Suppression"],
            [
                f'<tr><td class="path"><a href="{prefix}files/{slug(s["path"])}#L{s["line"]}">{esc(s["path"])}:{s["line"]}</a></td>'
                f"<td><code>{esc(s['what'])}</code></td></tr>"
                for s in sec["suppressions"]
            ],
            sortable=False,
        )
    )
    return "".join(out)


def render_model(report):
    m = report["model"]
    tools = "".join(f"<li>{esc(k)}: <code>{esc(v)}</code></li>" for k, v in report["tools"].items())
    notes = "".join(f"<li>{esc(n)}</li>" for n in report["notes"])
    limits = ", ".join(f"{k} up to {v:g}%" for k, v in m["grade_limits_percent"].items())
    return (
        '<h2 id="model">How it is measured</h2><div class="wrap" style="padding:4px 16px">'
        f"<p><strong>Grade.</strong> Debt ratio = remediation minutes / ({m['development_minutes_per_line']} minutes × code lines). "
        f"{limits}, E above. Remediation: {m['smell_minutes']} minutes per lint finding (none for Info), and for each function "
        f"over cognitive complexity {m['cognitive_threshold']}, {m['cognitive_minutes']}. Panic paths, unsafe, duplication "
        "and advisories are not in the grade.</p>"
        "<p><strong>Complexity.</strong> Rust: rust-code-analysis, a function's closures included. The page: cyclomatic from "
        "rust-code-analysis, cognitive from sonarjs (SonarSource's implementation), each function on its own.</p>"
        "<p><strong>Code lines</strong> are lines holding code, not comments or blanks. Test code is out: "
        "<code>#[cfg(test)]</code> items, <code>tests/</code>, and the page's <code>*.test.ts(x)</code> and <code>src/test/</code>.</p>"
        f'<p><strong>Tools.</strong></p><ul class="plain">{tools}</ul>'
        + (f'<p><strong>Notes.</strong></p><ul class="plain">{notes}</ul>' if notes else "")
        + "</div>"
    )


def render_unit(report, name, totals, files, issues):
    label = "the page (crates/finder/web)" if name == "page" else name
    fns = [f for f in report["functions"] if f["unit"] == name]
    rows = [
        f'<tr><td class="path"><a href="../files/{slug(r["path"])}">{esc(r["path"])}</a></td>{unit_cells(r)}</tr>'
        for r in files
    ]
    body = [
        f'<div class="crumbs"><a href="../index.html">All</a> › {esc(label)}</div><h1>{esc(label)}</h1>',
        unit_tiles(totals),
        "<h2>Files</h2>",
        table(["File"] + UNIT_HEADS, rows),
        f"<h2>Worst {WORST_HTML} functions</h2>",
        table(FUNCTION_HEADS, function_rows(fns[:WORST_HTML], "../")),
        "<h2>Rules</h2>",
        table(RULE_HEADS, rule_rows(rules_of(issues))),
    ]
    return page(f"{label} · code quality", "../", report, "".join(body))


def render_file(report, root, row, issues, functions, clones, test_ranges):
    rel = row["path"]
    lines = read_lines(os.path.join(root, rel))
    by_line = collections.defaultdict(list)
    for i in issues:
        by_line[i["line"]].append(i)
    dup = set()
    for c in clones:
        for s in (c["a"], c["b"]):
            if s["path"] == rel and c["block"]:
                dup.update(range(s["start"], s["end"] + 1))
    test = set()
    for a, b in test_ranges:
        test.update(range(a, b + 1))
    src = []
    for n, text in enumerate(lines, 1):
        cls = " ".join(c for c, on in (("dup", n in dup), ("test", n in test), ("hit", n in by_line)) if on)
        src.append(
            f'<div class="ln {cls}" id="L{n}"><a href="#L{n}">{n}</a><span class="bar"></span><pre>{esc(text)}</pre></div>'
        )
        for i in sorted(by_line.get(n, []), key=lambda i: SEVERITIES.index(i["severity"])):
            src.append(
                f'<div class="msg sev-b-{i["severity"]}">{sev(i["severity"])} <a href="{esc(i["url"])}"><code>{esc(i["rule"])}</code></a> '
                f"{esc(i['message'])}</div>"
            )
    unit = row["unit"]
    finding_rows = [
        f'<tr><td class="num" data-v="{i["line"]}"><a href="#L{i["line"]}">{i["line"]}</a></td>'
        f'<td data-v="{SEVERITIES.index(i["severity"])}">{sev(i["severity"])}</td><td>{KINDS[i["category"]]}</td>'
        f'<td><a href="{esc(i["url"])}"><code>{esc(i["rule"])}</code></a></td><td>{esc(i["message"])}</td></tr>'
        for i in issues
    ]
    fn_rows = function_rows(sorted(functions, key=lambda f: f["line"]), "../", in_file=True)
    body = [
        f'<div class="crumbs"><a href="../index.html">All</a> › <a href="../units/{unit}.html">{esc(unit)}</a></div>',
        f"<h1>{esc(rel)}</h1>",
        unit_tiles(row),
    ]
    if row.get("parse_error"):
        body.append(
            '<p class="note">rust-code-analysis could not parse all of this file; its figures are approximate.</p>'
        )
    body += [
        "<h2>Functions</h2>",
        table(["Function", "Line", "Cognitive", "Cyclomatic", "Lines", "Params"], fn_rows),
        "<h2>Findings</h2>",
        table(["Line", "Severity", "Kind", "Rule", "Message"], finding_rows),
        "<h2>Duplication</h2>",
        table(CLONE_HEADS, clone_rows(clones, "../")),
        "<h2>Source</h2>",
        '<div class="legend"><span><i style="background:var(--hit)"></i>has findings</span>'
        '<span><i style="background:var(--dup)"></i>duplicated block</span>'
        '<span><i style="background:var(--test)"></i>test code, not measured</span></div>',
        f'<div class="src">{"".join(src)}</div>',
    ]
    return page(f"{os.path.basename(rel)} · code quality", "../", report, "".join(body))


def render_html(report, root, dest, blocks):
    shutil.rmtree(dest, ignore_errors=True)
    write(os.path.join(dest, "index.html"), render_index(report))
    files = report["rust"]["files"] + report["web"]["files"]
    issues_by = collections.defaultdict(list)
    for i in report["issues"]:
        issues_by[i["path"]].append(i)
    fns_by = collections.defaultdict(list)
    for f in report["functions"]:
        fns_by[f["path"]].append(f)
    clones_by = collections.defaultdict(list)
    for c in report["duplications"]:
        clones_by[c["a"]["path"]].append(c)
        if c["b"]["path"] != c["a"]["path"]:
            clones_by[c["b"]["path"]].append(c)
    units = [(c["crate"], c) for c in report["rust"]["crates"]] + [("page", report["web"]["totals"])]
    for name, totals in units:
        mine = [r for r in files if r["unit"] == name]
        write(
            os.path.join(dest, "units", f"{name}.html"),
            render_unit(report, name, totals, mine, [i for i in report["issues"] if i["unit"] == name]),
        )
    for row in files:
        rel = row["path"]
        write(
            os.path.join(dest, "files", slug(rel)),
            render_file(report, root, row, issues_by[rel], fns_by[rel], clones_by[rel], blocks.get(rel, [])),
        )


# ---------------------------------------------------------------- main


def main(argv):
    if not argv or argv[0] not in ("prepare", "report", "diff", "clippy-args"):
        sys.exit(__doc__.splitlines()[0])
    if argv[0] == "clippy-args":
        print("\n".join(clippy_args()))
        return
    if len(argv) != 3:
        sys.exit(__doc__.splitlines()[0])
    if argv[0] == "diff":
        diff(argv[1], argv[2])
        return
    out, root = os.path.abspath(argv[1]), os.path.abspath(argv[2])
    if argv[0] == "prepare":
        prepare(out, root)
        return
    report = {**stamp(root, os.path.join(out, "raw")), **measure(out, root)}
    write(os.path.join(out, "summary.json"), json.dumps(report, indent=1))
    md = markdown(report)
    write(os.path.join(out, "summary.md"), md + "\n")
    render_html(report, root, os.path.join(out, "html"), load_json(os.path.join(out, "raw", "test-blocks.json"), {}))
    print(md)


if __name__ == "__main__":
    main(sys.argv[1:])
