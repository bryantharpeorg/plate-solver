# PRD — Plate Solver (Rust, mobile, gRPC)

> **Historical.** This first PRD predates the finder console, the scope node and the imaging
> chain. Its mobile target was not pursued, and later features relaxed some of its limits (the
> imaging chain solves with a position hint and handles 16-bit FITS). For the system as it is
> now, start at [`docs/README.md`](../docs/README.md).

_Product Requirements Document. Companion to [`project.md`](./project.md) (shared context,
conventions, glossary) and the per-feature changes under [`changes/`](./changes/). Status:
draft for review._

## Problem

Identifying where a camera points on the celestial sphere from a single star image, with **no
prior attitude** (the *lost-in-space* problem), is solved well by the tetra3/cedar family — but
those solvers are **Python**, and the production-grade star detector (cedar-detect) is a
standalone Rust service. A mobile product (a phone in the field, or an embedded star tracker)
needs the **entire** pipeline — detection, database lookup, identification, attitude recovery —
running natively in **Rust** at low, predictable latency and memory, exposed through a clean
interface a consumer app can call.

## Background

The system uses **geometric hashing** of 4-star patterns: 4 stars give 6 pairwise angular
edges; normalizing the 5 smaller edges by the largest yields a rotation- and scale-invariant
**pattern key** used to index a precomputed sky database. A two-stage match — cheap geometric
candidate generation, then authoritative attitude-based verification with a **binomial
false-alarm test** — makes it robust. The reference implementations and their rebuild-level
documentation live in `reference-solutions/` (`docs/01..08`). This product re-implements the
**cedar** variant (which strictly supersedes the original tetra3) end-to-end in Rust.

## Users & use cases

- **Mobile astronomy / EAA app developers** — point a phone at the sky, get RA/Dec/Roll/FOV to
  annotate or align a view; identify what is in frame.
- **Embedded star-tracker / telescope-mount builders** — on-device attitude for go-to/alignment
  without a network round-trip; cedar's lineage (Cedar runs on telescope mounts).
- **AR / sky-overlay tools** — continuous attitude from the camera for sky labeling.
- **Integrators in other languages** — call the solver as a local **gRPC** service.

Primary use case: *consumer supplies a camera frame (8-bit grayscale) + an approximate FOV; the
service returns the attitude (RA, Dec, Roll), refined FOV and distortion, and optionally the
matched stars.*

## Goals

- Re-implement the cedar lost-in-space pipeline **natively in Rust**: detection → DB lookup →
  identification → attitude/FOV/distortion recovery.
- Expose a **gRPC `PlateSolver` service** (`ExtractCentroids`, `SolveFromCentroids`,
  `SolveFromImage`, `GetInfo`) as the consumer contract.
- Run **on a phone**: bounded latency, bounded memory, memory-mappable database, and **UniFFI**
  bindings for direct in-process embedding (iOS/Android) as an alternative to a network hop.
- Preserve **numerical parity** with the Python reference within stated tolerances, as a
  testable correctness contract.
- Ship an **offline database-generation** tool so a phone-sized, FOV-matched DB can be built and
  bundled.

## Non-Goals

- No Python runtime or dependency on the reference solvers at runtime.
- **Partial-sky databases** (`range_ra`/`range_dec`) — cedar covers the whole sky; out of scope.
- The original **tetra3 simpler detector** (threshold + connected-components + moments, doc 03)
  and **per-anchor-star DB enumeration** (doc 05 §5.1) — kept as reference-only; v1 implements
  the cedar lattice-field + cedar-detect path only.
- Hinted solving (solving *with* a prior attitude); only lost-in-space is in scope for v1.
- Non-rectilinear (fisheye) lens models beyond the single-parameter radial distortion `k`.
- Higher-than-8-bit detection pipelines (input is converted to 8-bit grayscale).

## Functional Scope (the 7 feature capabilities)

In dependency order. Capabilities 1–6 are specified in `openspec/specs/`; `mobile-runtime` is
deferred: its change was archived unapplied on 2026-10-25, under `openspec/changes/archive/`.

1. **`math-core`** — coordinate/vector math, `2·arcsin(d/2)` angles, pinhole projection,
   distortion, Wahba/SVD attitude + RA/Dec/Roll, edge-ratio pattern key + 64-bit hash + table
   index + open-addressing, centroid-distance ordering, binomial false-alarm test, residuals.
2. **`star-detection`** — cedar-detect Rust pipeline: noise estimation, binning cascade, 1-D
   7-pixel row gate, hot-pixel rejection, blob formation, 2-D gate, sub-pixel centroid,
   brightness ordering.
3. **`pattern-database`** — on-disk DB format, loader, star KD-tree, key→index hash lookup with
   16-bit and largest-edge/FOV pre-filters.
4. **`database-generation`** — offline: catalog parsing (BSC5/HIP/TYC), proper motion, density
   thinning, Fibonacci lattice-field pattern enumeration, hashing, serialization.
5. **`plate-solver`** — identification + attitude recovery engine: prep, candidate generation,
   verification, refinement, outputs, status codes.
6. **`grpc-service`** — the `PlateSolver` gRPC API, message schemas, the `(x,y)↔(y,x)` boundary,
   shared-memory fast path with inline fallback.
7. **`mobile-runtime`** — on-device embedding, UniFFI bindings, mmap DB, performance/memory
   budgets, threading, packaging.

These name *capabilities*, not modules. How they are decomposed into crates and internal
interfaces is for the implementation to determine — see `project.md` §5, which lists the binding
constraints and nothing beyond them.

## Non-Functional Requirements

| Area | Requirement (target / contract) |
|---|---|
| **Solve latency** | ~10 ms per solve on desktop-class hardware excluding extraction (reference: cedar ~10 ms/solve); bounded by `solve_timeout` (default 5000 ms). Mobile target documented per platform in `mobile-runtime`. |
| **Detection latency** | cedar-detect class: **< 10 ms per 1 M pixels** on Raspberry-Pi-4B-class hardware, even with dozens of stars. |
| **Accuracy** | Attitude accuracy on the order of **arcseconds to tens of arcseconds RMSE** (reference quotes ~10 arcsec / 50 µrad); report RMSE, P90E, MAXE. |
| **Numerical parity** | RA/Dec within a few **arcseconds** of cedar on reference images; centroids within ~**±0.1 px**; **identical** matched catalog IDs. Enforced as spec scenarios. |
| **Input** | 8-bit grayscale; color/high-bit converted at the boundary. Centroid origin `(0.5,0.5)` = top-left pixel center; coordinates `(y,x)`. |
| **Memory** | Database **memory-mappable** (memmap2) with a **linear-probe** table option for narrow-FOV / too-big-for-RAM cases; bounded peak RAM on mobile (budget in `mobile-runtime`). |
| **Robustness** | Hot-pixel rejection, trail rejection, tolerance of bright interlopers (moon/streetlights), adaptive local noise; statistical (not fixed-count) match acceptance. |
| **Determinism** | DB generation and solving are deterministic given the same inputs/seed; offline DB build reproducible. |
| **Portability** | Pure-Rust core; mobile builds avoid heavy/native-incompatible deps; parallelism feature-gated and bounded on device. |

## Success Metrics

- **Correctness:** parity scenarios pass — on the reference test images, the Rust solver returns
  RA/Dec within a few arcsec of cedar with identical matched catalog IDs; detection centroids
  match cedar-detect within ±0.1 px.
- **Performance:** detection meets < 10 ms/Mpx on RPi-4B-class hardware; solve meets its
  documented per-platform latency budget on a target phone.
- **Footprint:** a bundled phone-FOV database loads via mmap within the documented RAM ceiling.
- **Usability:** a consumer can obtain attitude from a single `SolveFromImage` gRPC call (or one
  UniFFI call) given an image + FOV estimate.
- **Documentation done:** all 7 feature changes pass `openspec validate --strict` (this PRD's
  authoring milestone).

## Milestones (mapped to features, dependency order)

| Milestone | Feature(s) | Exit criterion |
|---|---|---|
| M0 — Spec set | all (this documentation effort) | 7 changes validate `--strict`; PRD + project + STATUS exist |
| M1 — Math core | `math-core` | geometry/hashing/attitude primitives match reference formulas within parity tolerance |
| M2 — Detection | `star-detection` | centroids match cedar-detect on test images within ±0.1 px |
| M3 — Database | `pattern-database`, `database-generation` | a generated DB round-trips and loads; lookups return correct candidates |
| M4 — Solver | `plate-solver` | reference images solve to parity (RA/Dec arcsec, identical IDs) |
| M5 — Service | `grpc-service` | consumer solves an image end-to-end over gRPC |
| M6 — Mobile | `mobile-runtime` | on-device solve within latency/memory budget via UniFFI/mmap |

## Risks & Mitigations

- **Numerical divergence from Python/NumPy** (SVD sign, float order) → fix conventions (`2·arcsin`,
  `_MAGIC_RAND`, `det(R)<0` reject), test against captured reference outputs within tolerance.
- **SVD/linalg crate gaps** → choose `nalgebra` (3×3 SVD); validate Wahba results vs reference.
- **DB size vs mobile RAM** → memory-map + linear-probe table for narrow FOV; size the DB to the
  device FOV in `database-generation`.
- **gRPC overhead on device** → shared-memory fast path on-host; UniFFI in-process path for mobile.
- **Detection edge cases** (crowding, single-pixel stars, wide-field aberration) → documented
  limitations; binning + hot-pixel handling per cedar-detect.
- **Catalog licensing/size** (HIP 51 MB, TYC 355 MB) → generate device DBs offline; ship only the
  compiled pattern DB.

## Open Questions

- Native Rust DB format vs mirroring the NumPy `.npz` layout (decided per `pattern-database` design).
- Exact per-platform mobile latency/RAM budgets (to be fixed in `mobile-runtime` against target
  devices).
- Whether to expose a streaming/continuous-solve RPC for video (post-v1; lost-in-space is v1).
