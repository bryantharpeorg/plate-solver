# Plate Solver — Reference Documentation

This folder documents the three reference codebases in `reference-solutions/` to a
level of detail sufficient to **re-implement the entire system from these documents
alone**. Every algorithm, formula, magic constant, data layout and design decision
relevant to (a) detecting stars in an image and (b) identifying where in the sky that
image is pointing is captured here.

## What is plate solving / "lost-in-space"?

Given a photograph of the night sky and (roughly) the camera's field of view, determine
the celestial direction the camera was pointing — its **right ascension (RA)**,
**declination (Dec)** and **roll** — purely from the geometric arrangement of the stars,
with **no prior pointing knowledge**. This "from scratch" mode is called the
*lost-in-space* problem (as opposed to *tracking*, where you already have an
approximate attitude). The output is a full 3-axis attitude (a rotation matrix between
the camera frame and the celestial frame), plus the refined field of view and lens
distortion.

The pipeline has two halves:

```
        IMAGE                         CENTROIDS                       ATTITUDE
   ┌───────────┐   star detection   ┌────────────┐   plate solving  ┌──────────────┐
   │  pixels   │ ─────────────────► │ (y,x) list │ ───────────────► │ RA, Dec, Roll│
   │  (8-bit)  │  (centroiding)     │  brightest │  (identification)│ FOV, distortion│
   └───────────┘                    │   first    │                  └──────────────┘
                                    └────────────┘
```

1. **Star detection / centroiding** — find the sub-pixel `(y, x)` positions of star-like
   spots, brightest first. (`docs/03`, `docs/04`)
2. **Plate solving / identification** — match a small geometric "fingerprint" of those
   centroids against a precomputed pattern database, verify, and recover the attitude.
   (`docs/05`, `docs/06`)

## The three codebases

| Folder | Language | Role | Lineage |
|---|---|---|---|
| `tetra3/` | Python | **Original** plate solver (star detection + DB generation + solving). Built by ESA (Gustav Pettersson), itself a rewrite of MIT/Brown's *Tetra*. | The ancestor. |
| `cedar-solve/` | Python | **Evolution of tetra3** by Steven Rosenthal. Same package name (`tetra3`), same core algorithm, but a re-engineered database generator (lattice fields), a smarter solver (breadth-first search, timeouts, status codes, 16-bit hash pre-filter), and an integration hook for Cedar Detect. | Fork/superset of tetra3. |
| `cedar-detect/` | Rust | **High-performance star detector only.** Replaces tetra3's Python `get_centroids_from_image` with a fast, robust Rust implementation exposed over gRPC. Does *not* do plate solving. | Companion to cedar-solve. |

"**tetra3**" in the goal refers to the original `tetra3/` package. "**cedar**" refers to
the `cedar-solve/` (the solver, a partial re-port/evolution) together with `cedar-detect/`
(the Rust star detector). cedar-solve still *contains* a `tetra3` Python package — the name
was kept for drop-in compatibility — so be careful: the directory `cedar-solve/tetra3/` is
the *cedar* code, not the original.

## How they fit together at runtime

```
                         ┌──────────────────────── cedar-detect (Rust) ────────────────────────┐
                         │  estimate_noise → scan rows (1D gate) → hot-pixel reject →           │
   image (8-bit gray) ──►│  blob formation → 2D gate → sub-pixel centroid → brightness          │──┐
                         └─────────────────────────────────────────────────────────────────────┘  │ (y,x) centroids
                                          ▲ gRPC (image→centroids), optional shared memory          │ brightest-first
                                          │                                                         ▼
   ┌─────────────────────────── cedar-solve / tetra3 (Python) ──────────────────────────────────────────┐
   │  solve_from_centroids: cluster-bust → choose 4-star pattern → edge-ratio key → hash lookup →        │
   │  geometric verify (SVD attitude, project catalog, count matches, binomial false-alarm test) →       │
   │  refine FOV+distortion (least squares) → RA/Dec/Roll/FOV/distortion/RMSE                              │
   └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
                                          ▲
                                          │ uses
              pattern database (.npz): star_table + pattern_catalog (hash table) + key hashes + largest edges
                                          ▲
                                          │ built offline by
              generate_database(): load star catalog → propagate proper motion → thin by density →
              enumerate 4-star patterns (lattice fields) → edge-ratio keys → hash-table insert
```

tetra3 (original) does the same end to end, but uses its own Python `get_centroids_from_image`
instead of cedar-detect, and a simpler database generator and solver.

## Document index

| Doc | Contents |
|---|---|
| [`01-overview-and-concepts.md`](01-overview-and-concepts.md) | The problem, key ideas (geometric hashing, edge-ratio fingerprints, verification), glossary, end-to-end walkthrough. |
| [`02-coordinate-systems-and-math.md`](02-coordinate-systems-and-math.md) | Coordinate conventions, pinhole camera projection, lens distortion model, attitude/rotation math, edge-ratio hashing math, the false-match probability test. The shared math toolbox. |
| [`03-star-detection-tetra3.md`](03-star-detection-tetra3.md) | tetra3's Python `get_centroids_from_image` and `crop_and_downsample_image`, step by step, with every parameter. |
| [`04-star-detection-cedar-detect.md`](04-star-detection-cedar-detect.md) | cedar-detect's Rust pipeline: noise estimation, binning, 1D row gate, hot-pixel classification, blob formation, 2D gate, sub-pixel centroiding, brightness. |
| [`05-database-generation.md`](05-database-generation.md) | Building the pattern catalog. Both the original tetra3 approach and cedar-solve's lattice-field approach. Catalog parsing, proper motion, hashing, hash table layout, on-disk format. |
| [`06-plate-solving.md`](06-plate-solving.md) | The solve algorithm in full for both tetra3 and cedar-solve: pattern selection, hash lookup, geometric verification, attitude recovery, refinement, outputs. |
| [`07-cedar-detect-service-api.md`](07-cedar-detect-service-api.md) | The gRPC contract, message formats, shared-memory transport, the Python clients, and the server binary. How to wire cedar-detect into a solver. |
| [`08-tetra3-vs-cedar-comparison.md`](08-tetra3-vs-cedar-comparison.md) | Side-by-side of every meaningful difference, default-value tables, and a rebuild checklist. |

## Reading order

- To **understand the system**: read 01 → 02, then 03/04 (detection), then 05 (database),
  then 06 (solving).
- To **rebuild detection only**: 02 (coordinates) + 04 (cedar-detect, the production
  detector) or 03 (the simpler reference detector).
- To **rebuild the solver**: 02 + 05 + 06, with 08 to pick which variant to copy.

## A note on accuracy

All formulas, constants, and array layouts in these docs were transcribed directly from
the source: ESA's original [`tetra3/tetra3.py`](https://github.com/esa/tetra3/blob/f9fa2eb9a32a5efc529e2d86f0b59f35b1e9028d/tetra3/tetra3.py) (no longer copied here),
`cedar-solve/tetra3/tetra3.py` and `cedar-detect/src/*.rs`. Where the two Python
implementations differ, the difference is called out explicitly. Defaults reflect the code at
the upstream commits listed in [`../README.md`](../README.md).
