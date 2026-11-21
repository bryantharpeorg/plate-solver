# Project Context — Plate Solver (Rust)

> **Historical.** This is the first-generation project context, kept for its numeric
> conventions and as history. The mobile and UniFFI plans here were not pursued. Features from
> 012 on are specified in [`specs/`](../specs/); `changes/feat-12` to `feat-16` were converted
> to `specs/012`–`016` on 2026-10-28. For the system as it is now, start at
> [`docs/README.md`](../docs/README.md).

> This file is OpenSpec project context for the **plate-solver** repository: a from-scratch
> **Rust** reimplementation of the tetra3/cedar "lost-in-space" star-field plate-solving
> system, delivered to consumers over **gRPC** and embeddable on **mobile** (iOS/Android).
> It is the shared vocabulary and convention reference for every change under
> `openspec/changes/`. The product-level "why/what" lives in [`PRD.md`](./PRD.md).

## 1. Product summary

Given a grayscale photograph of the night sky and a rough horizontal field of view (FOV),
determine where the camera was pointing — its **right ascension (RA)**, **declination
(Dec)**, and **roll** — purely from the geometric arrangement of the stars, with **no prior
pointing knowledge** (the *lost-in-space* problem). The output is a full 3-axis attitude plus
refined FOV and lens distortion, optionally with the identity and pixel position of each
matched catalog star.

The pipeline has two halves:

```
   IMAGE  ──star detection──►  CENTROIDS (y,x, brightest first)  ──plate solving──►  ATTITUDE
  (8-bit gray)  (centroiding)      brightest-first list           (identification)   RA,Dec,Roll,FOV,distortion
```

1. **Star detection / centroiding** — find sub-pixel `(y,x)` positions of star-like spots,
   brightest first.
2. **Plate solving / identification** — match a 4-star geometric "fingerprint" of those
   centroids against a precomputed pattern database, verify statistically, and recover the
   attitude.

The build target is a phone, so **Rust-level performance** is a primary constraint, not an
afterthought.

## 2. The three reference codebases (read-only source of truth)

All specs derive from `reference-solutions/` and its distilled docs in
`reference-solutions/docs/01..08`. **Never modify `reference-solutions/`** — it is the
authoritative source we are re-implementing.

| Folder | Language | Role | Lineage |
|---|---|---|---|
| `tetra3/` | Python | **Original** plate solver (detection + DB gen + solving) by ESA (Gustav Pettersson), a rewrite of MIT/Brown's *Tetra*. | The ancestor. |
| `cedar-solve/` | Python | **Evolution of tetra3** by Steven Rosenthal: re-engineered DB generator (lattice fields), smarter solver (breadth-first, timeouts, status codes, 16-bit hash pre-filter), Cedar Detect hook. | Fork/superset of tetra3. |
| `cedar-detect/` | Rust | **High-performance star detector only**, exposed over gRPC. Does *not* plate-solve. | Companion to cedar-solve. |

> **Naming trap:** `cedar-solve`'s Python package is *also* named `tetra3` (kept for drop-in
> compatibility). So `cedar-solve/tetra3/tetra3.py` is the **cedar** code, **not** the
> original. In these specs, "**tetra3**" = the original `tetra3/` package; "**cedar**" =
> `cedar-solve` (solver) + `cedar-detect` (detector).

**This project rebuilds the cedar path throughout** (it strictly supersedes tetra3).
tetra3's simpler detector (doc 03) and per-anchor DB enumeration (doc 05 §5.1) are
**reference-only / non-goals** for v1.

## 3. Reference Documentation Map

Each feature (OpenSpec change) is derived from and cites specific reference docs. Spec authors
must ground requirements in these and cite the doc number.

| Reference doc | Feeds feature (change) |
|---|---|
| `docs/01-overview-and-concepts.md` | all (concepts, glossary, end-to-end walkthrough) |
| `docs/02-coordinate-systems-and-math.md` | **`math-core`** (`feat-01-foundation-math-core`) |
| `docs/03-star-detection-tetra3.md` | reference-only (tetra3 simple detector; non-goal) |
| `docs/04-star-detection-cedar-detect.md` | **`star-detection`** (`feat-02-star-detection`) |
| `docs/05-database-generation.md` | **`pattern-database`** (`feat-03-`, format/load §6–7) + **`database-generation`** (`feat-04-`, build §1–6) |
| `docs/06-plate-solving.md` | **`plate-solver`** (`feat-05-plate-solver`) |
| `docs/07-cedar-detect-service-api.md` | **`grpc-service`** (`feat-06-grpc-service`) |
| `docs/08-tetra3-vs-cedar-comparison.md` | cross-cutting defaults/decisions (all designs); informs **`mobile-runtime`** |
| perf notes in docs 04/06/08 | **`mobile-runtime`** (`feat-07-mobile-runtime`), deferred: archived unapplied on 2026-10-25, since `PRODUCT.md` puts the mobile app out of scope for now |
| none: a consumer of the solver, grounded in its code, its tests and the sources in `crates/finder/data/README.md` | **`finder-console`** (`feat-11-finder-console`) |
| Meeus, *Astronomical Algorithms* (2nd ed.), ch. 13 (altitude and azimuth), 15 (rising, transit, setting), 21 (precession) and 25 (the Sun); [OpenNGC](https://github.com/mattiaverga/OpenNGC); `GLOSSARY.md` | **`observing-session`**, **`target-catalog`**, **`sky-ephemeris`**, **`push-to`** (`feat-12-targets-and-push-to`) |
| [ADR-0001](../docs/adr/0001-console-owns-pointing-scope-node-drives-axes.md); `docs/research/2026-10-21-scope-node-runtime.md`; Cedar's and PiFinder's aim-point calibration (`docs/research/2026-10-21-pointing-model-and-tracking.md` §2); RFC 5905 (NTP's offset and delay) | **`scope-link`**, **`scope-node`**, **`simulator`**, **`live-finder`** (`feat-13-live-finder`) |
| Sky-Watcher's motor-controller command set and `docs/research/2026-10-21-synscan-mount-control.md`; the TPoint manual v18.20 §7.2 and Taki (1989), via `docs/research/2026-10-21-pointing-model-and-tracking.md` §1–2; Meeus ch. 16 (refraction) | **`pointing-model`**, **`goto`** (`feat-14-goto`) |
| Meeus ch. 13 and 14 (parallactic angle); `docs/research/2026-10-21-pointing-model-and-tracking.md` §3 (rates, zenith keyhole, field rotation, closed loop) | **`tracking`** (`feat-15-tracking`) |
| The MCP specification (Streamable HTTP, security best practices, tools) and the `rmcp` crate, via `docs/research/2026-10-25-mcp-server.md`; ADR-0001 | **`mcp-server`** (`feat-16-mcp`) |

Feature dependency order (implementation order): `math-core` → `star-detection` → `pattern-database`
→ `database-generation` → `plate-solver` → `grpc-service` → `mobile-runtime` (deferred). `finder-console`
sits beside `grpc-service` as a second consumer, over `math-core`, `star-detection`,
`pattern-database` and `plate-solver`; it also adds a requirement each to `star-detection`
(PIL's gray conversion) and `plate-solver` (cedar's detection settings), so the console and
`solve_image` read frames the same way. (`feat-08` to `feat-10` were used by changes that have
since been removed, so the console takes `feat-11`.)

The console then grows in four changes, in this order, each over the capabilities of the ones
before it: `feat-12-targets-and-push-to` (choose a target, learn whether it is pointable, get
push-to directions; no hardware), `feat-13-live-finder` (the scope node streams star frames from
the finder camera and the console solves them live), `feat-14-goto` (the console drives the
mount's two axes to the target through a pointing model fitted from syncs) and
`feat-15-tracking` (the mount follows the target). Per ADR-0001 the console owns pointing and
reaches hardware only through the mount port and camera port. SkySafari, the AI and the mobile
app are out of scope for these four. None of them touches `math-core`, the parity path.
`feat-16-mcp` comes after them: an MCP endpoint in the console whose tools pass its actions and
the mount calls through to MCP clients on the LAN, under the same guards. Its tasks land in
steps after each of the four; the scope node gets no MCP.

## 4. Coordinate & sign conventions (binding across all specs)

- **Pixel coordinates** are `(y, x)` = `(row down, column right)`, origin top-left.
  `(0.5, 0.5)` is the **center of the top-left pixel** (integer floor → pixel index).
  Centroids are always reported `(y, x)`. `size = (height, width)`.
- **Camera-frame unit vectors** `(i, j, k)`: `i` = boresight (optical axis, index 0),
  `j` = image x / horizontal (index 1), `k` = image y / vertical (index 2).
- **Celestial unit vectors**: `x = cos(RA)cos(Dec)`, `y = sin(RA)cos(Dec)`, `z = sin(Dec)`;
  inverse `RA = atan2(y,x) mod 2π`, `Dec = arcsin(z)`.
- **Angular separation** between unit vectors at chord distance `d`: `angle = 2·arcsin(d/2)`
  (used everywhere in preference to `arccos` for small-angle conditioning).
- **Attitude** `R`: 3×3 rotation; row 0 is the boresight in the celestial frame (image-center
  RA/Dec). `RA=atan2(R[0,1],R[0,0])`, `Dec=atan2(R[0,2],‖R[1:3,2]‖)`, `Roll=atan2(R[1,2],R[2,2])`.
- **Diagonal FOV** = `fov · √(w²+h²)/w`.
- gRPC `ImageCoord` is `(x, y)`; the solver wants `(y, x)` — swap at the service boundary.

## 5. Binding constraints

These are **constraints, not design decisions.** They are fixed by the parity contract, the
proto files, or the PRD — violate one and the system is wrong. Everything *not* listed here is
open: the module/crate structure, the dependency choices, and the internal interfaces are for the
implementation to determine from the capability specs. This document does not prescribe them.

**Numerics (fixed by parity):**

- Compute in **f64**; store database vectors as **f32** (the reference dtype). Tolerances will not
  line up otherwise.
- Angles via `2·arcsin(d/2)` everywhere — never `arccos` (small-angle conditioning).
- `pattern_bins = round(1/(4·pattern_max_error))`.
- Hash table index uses the magic constant `_MAGIC_RAND = 2654435761` (⌊2³²/φ⌋, Knuth
  multiplicative hash). This is not a tuning knob — the reference's table layout depends on it.
- The false-alarm statistic must be the same one the reference uses.

**Interfaces (fixed by the contracts in `proto/`):**

- The gRPC surface must interoperate with **cedar-detect's** wire format — it reuses cedar-detect's
  `Image` / `ImageCoord` message shapes. Wire compatibility is the requirement; the server library
  is not specified.
- gRPC `ImageCoord` is `(x, y)`; the solver works in `(y, x)`. Swap at the service boundary.

**Product (fixed by the PRD):**

- Detection input is **8-bit grayscale**. Higher bit depths are out of scope.
- The database must be **memory-mappable** — narrow-FOV databases will not fit in phone RAM.
- **UniFFI** bindings (Rust ↔ Swift/Kotlin) are the in-process mobile embedding path, as an
  alternative to an on-device network hop.
- Parallelism, if used, must be bounded and **off or constrained on mobile**.

**Numerical parity is a correctness contract:** specs assert results match the Python reference
(tetra3/cedar) within stated tolerances (RA/Dec within arcseconds, centroids within ~±0.1 px,
identical matched catalog IDs).

## 6. Glossary

The project's vocabulary, solver and observing terms alike, lives in the root
[`GLOSSARY.md`](../GLOSSARY.md). Specs use its terms.

## 7. Conventions for authoring changes

- One OpenSpec **change** per feature, named `NN-<feature>`. Each carries four artifacts:
  `proposal.md`, `specs/<capability>/spec.md`, `design.md`, `tasks.md`.
- Specs use `### Requirement:` (SHALL/MUST) with `#### Scenario:` blocks (exactly four hashes;
  WHEN/THEN). Every requirement has ≥1 scenario; include parity scenarios where applicable.
- Each change must pass `openspec validate <change> --strict` and reach 4/4 in
  `openspec status --change <change>`.
- Cite the reference doc number behind each requirement; do not invent behavior not grounded in
  `reference-solutions/`. The console's changes (feat-11 on) are consumers of the solver: they
  cite the sources in §3 instead (Meeus, vendor documents, ADRs and the `docs/research/` notes).

### Conventions for the console's changes (feat-12 on)

These were checked against the OpenSpec 1.14.3 validator, which `--strict` enforces.

- **Statement:** a requirement's text is at most 500 characters (aim for about 300), states one
  behaviour with SHALL or MUST in its first sentence, and uses `GLOSSARY.md`'s terms exactly.
- **References:** one physical line right after the statement, such as
  `**Refs**: Meeus ex. 15.a; ADR-0001`. No code paths and no HTML comments in requirements
  (archive drops comments).
- **Scenarios:** WHEN/THEN, each naming its oracle (a worked example, a fixture, the simulator's
  truth, a saved capture) and an explicit tolerance.
- **A new capability's delta** opens with `## Purpose` (at least 50 characters) and only ADDs.
- **Later changes in a set only ADD** to a capability an earlier change in the same set
  introduced: MODIFIED validates, but archive refuses it until the base is archived. So state
  requirements open-endedly ("at least", never "only").
- **MODIFIED** copies the whole existing requirement and keeps every scenario's name.
- **Proposal and tasks:** the Why section is 50–1000 characters; `tasks.md` uses `## N.` groups
  of `- [ ] N.M` items, each saying how it is checked, with each group landing its own tests.
- **The oracle rule:** no spec may require editing an existing test, golden, `scripts/ci/`,
  `.github/workflows/ci.yml` or `CODEOWNERS`. Prefer designs that keep existing tests valid.
  Where a decided change supersedes tested behaviour or changes CI, name it (as a MODIFIED or
  REMOVED delta, in Impact and in tasks) for the owner's explicit approval.
- **Injectable clocks:** every time-dependent behaviour takes its clock from outside. A virtual
  clock is allowed only in tests and `finder --simulate`; anywhere else it would be a way round
  the Sun guard.
- **CI realities:** clippy, coverage and docs build `--all-features` on x86_64, and the coverage
  floor counts Rust only, so hardware shells stay tiny and everything else is tested through
  the ports, protocol emulators, saved fixtures and the simulator. New dependencies build on
  Rust 1.96 and pass cargo-deny; the page adds no npm packages; test tokens have low entropy;
  proto comments hold no `[...]`.
- **UI:** each change with UI follows `DESIGN.md` (field atlas, square ruled boxes, red-only
  night mode, every new view in the night pixel check) and ends with an Impeccable pass and a
  `DESIGN.md` update.
