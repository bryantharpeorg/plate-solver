# 08 — tetra3 vs. cedar: Differences, Defaults, Rebuild Checklist

Both implement the same algorithm family (geometric-hash plate solving, doc 01). This
document is the side-by-side reference: what changed, why, and which variant to copy when
rebuilding. "tetra3" = ESA's original [`tetra3/tetra3.py`](https://github.com/esa/tetra3/blob/f9fa2eb9a32a5efc529e2d86f0b59f35b1e9028d/tetra3/tetra3.py) (no longer copied here). "cedar" = `cedar-solve` (the
Python solver, by Steven Rosenthal) + `cedar-detect` (the Rust detector).

> **Naming trap:** cedar-solve's package is *also* named `tetra3` (drop-in compatible). The
> directory `cedar-solve/tetra3/tetra3.py` is the **cedar** code.

---

## 1. At a glance

| Area | tetra3 (original) | cedar (cedar-solve + cedar-detect) |
|---|---|---|
| Star detection | Python `get_centroids_from_image` (threshold + connected components + moments) | Same Python function **available**, but the production path is **cedar-detect** (Rust, gRPC) |
| Hot-pixel rejection | none | yes (cedar-detect `classify_pixel`) |
| Adaptive/local noise | via `sigma_mode` choice | yes, multi-region + per-blob perimeter inflation |
| DB pattern enumeration | per-anchor-star neighbor combinations | **Fibonacci lattice fields** × breadth-first combinations |
| DB hash table | `2·N`, quadratic probe | `next_prime(2·N)` quadratic **or** `next_prime(3·N)` linear |
| 16-bit key pre-filter | no | yes (`pattern_key_hashes`) |
| `pattern_largest_edge` | optional (`save_largest_edge`) | always stored |
| Limiting magnitude | fixed (`star_max_magnitude=7`) | **auto** from density (default), overridable |
| Partial-sky DB (`range_ra/dec`) | yes | removed (whole sky always) |
| Solver image-pattern search | C(8,4) of 8 brightest (`pattern_checking_stars`) | breadth-first over **all** cluster-busted centroids |
| Centroid cluster-busting (solve) | no | yes |
| Candidate-key ordering | unordered | nearest-first (closest to measured key) |
| Reflection guard | no | reject `det(R) < 0` |
| Nearby-star fetch | cube prefilter + dot test | KD-tree `query_ball_point` |
| Nearby stars kept | `len(centroids)` | `2·num_centroids` (fudge factor) |
| Distortion input | scalar, **range**, or None | scalar or None (range removed) |
| Timeout / cancel | `solve_timeout` (default None) | default 5000 ms + `cancel_solve()` |
| Status codes | none | `MATCH_FOUND/NO_MATCH/TIMEOUT/CANCELLED/TOO_FEW` |
| Residual reporting | `RMSE` | `RMSE`, `P90E`, `MAXE` |
| Extra outputs | matches, visual | + `return_catalog`, `return_rotation_matrix`, `target_sky_coord`, `pattern_centroids` |
| Standalone transforms | — | `transform_to_image_coords`, `transform_to_celestial_coords` |

---

## 2. Default-value tables

### Database generation

| Parameter | tetra3 | cedar |
|---|---|---|
| `pattern_max_error` | 0.005 → **50 bins** | 0.001 → **250 bins** |
| pattern-star density | `pattern_stars_per_fov=10` | (derived from `verification_stars_per_fov`) |
| `verification_stars_per_fov` | 30 | 150 |
| `star_max_magnitude` | 7 | auto (≈ density-derived) |
| `multiscale_step` | 1.5 | 1.5 |
| pattern enumeration | neighbor combinations | `lattice_field_oversampling=100`, `patterns_per_lattice_field=50` |
| hash table | `2·N`, quadratic | `next_prime(2·N)` quad / `next_prime(3·N)` linear |
| `epoch_proper_motion` | `'now'` | `'now'` |
| bundled `default_database` | 10–30°, mag 7 | 10–30°, mag 8 |

### Solving

| Parameter | tetra3 | cedar |
|---|---|---|
| `match_radius` | 0.01 | 0.01 |
| `match_threshold` | 1e-3 | 1e-5 |
| `match_max_error` | (uses DB `pattern_max_error`) | 0.002 (clamped ≥ DB error) |
| `solve_timeout` | None | 5000 ms |
| image patterns tried | C(8,4)=70 (8 brightest) | breadth-first over all (timeout-bounded) |
| `distortion` | 0 (scalar/range/None) | 0 (scalar/None) |
| pole proper-motion cutoff | `cosδ > 0.1` (|Dec|≲84°) | `cosδ > 0.05` (|Dec|≲87°) |

### cedar-detect (Rust)

| Parameter | default |
|---|---|
| `sigma` | 8.0 (Python clients / test binary) |
| `binning` | 2 (when binning requested) |
| `noise_floor` | 0.2 |
| `max_size` (blob) | `width/100` (`/binning + 1` when binned) |
| port | 50051 |
| `NUM_PEAKS` (peak averaging) | 10 |
| de-star sigma (ROI stats) | 8.0 |
| row-normalization bias | 2.0 |

---

## 3. Why each change (design rationale)

- **Lattice-field DB enumeration** — guarantees *uniform pattern density over the whole
  sky* regardless of local star density, and bounds patterns/field. The original
  per-anchor scheme over- or under-populates depending on local crowding.
- **Cluster-busting (build & solve)** — stops dense clusters (Pleiades) from producing only
  tiny patterns that waste the pattern budget / fail to match a wide FOV.
- **More bins (250 vs 50) + 16-bit key pre-filter** — finer pattern keys reduce key
  collisions for richer (fainter-limit) databases; the 16-bit hash discards hash-table
  collisions cheaply before vector math.
- **Prime table size + linear-probe option** — better key dispersion; linear probing keeps
  probe chains contiguous for memory-mapped (too-big-for-RAM) databases.
- **Lower `match_threshold` (1e-5)** — fewer false positives, affordable because the richer
  database + pre-filters keep candidate counts manageable.
- **Breadth-first over all centroids + timeout** — more thorough than "8 brightest only",
  bounded by a wall-clock budget; nearest-first key ordering finds the answer sooner.
- **`det(R) < 0` reject** — a cheap, exact false-positive filter (a real match never yields a
  reflection).
- **2× nearby-star fudge factor** — image brightness ranking ≠ catalog ranking, so keeping
  extra catalog stars raises the match count and lowers the false-alarm probability.
- **cedar-detect (Rust)** — speed (RPi-class hardware, <10 ms/Mpx) and robustness
  (hot pixels, trails, interlopers, adaptive local noise) the Python detector lacks.
- **Status codes / cancel / timeout** — operational needs for an embedded star tracker
  (Cedar runs on telescope mounts).

---

## 4. What is identical between them

- The **edge-ratio pattern key** definition (6 edges → sort → normalize by largest → 5
  ratios → quantize). Same `pattern_size=4`. Same `pattern_bins = round(1/(4·err))` formula.
- The **64-bit key packing** and the `_MAGIC_RAND = 2654435761` multiplicative hash (cedar's
  quadratic-probe index function is identical; only the linear-probe path differs).
- The **pinhole projection** and **single-parameter distortion model** (modulo the `k` vs
  `k'` algebra noted in doc 02 §4).
- **Wahba/SVD attitude** and the **RA/Dec/Roll extraction** formulas.
- The **binomial false-alarm test** (with the `−2` DoF and the `/num_patterns` correction).
- The centroid-distance **pattern ordering** for correspondence.
- The Python **`get_centroids_from_image`** and **`crop_and_downsample_image`** code
  (byte-identical apart from comment whitespace).
- Catalog parsing for BSC5/HIP/TYC and proper-motion propagation (modulo the pole cutoff).

---

## 5. Which variant to rebuild

| Goal | Detection | Database | Solver |
|---|---|---|---|
| Max performance / robustness (embedded) | cedar-detect (doc 04) | cedar lattice (doc 05 §5.2) | cedar (doc 06) |
| Pure-Python, few deps, simplest | tetra3 `get_centroids` (doc 03) | tetra3 (doc 05 §5.1) | tetra3 (doc 06) |
| Best accuracy, OK with Python | cedar-detect or tetra3 local-median | cedar | cedar |
| Memory-mapped huge DB (narrow FOV) | either | cedar **linear-probe** | cedar |

The cleanest full rebuild is **cedar throughout** (it strictly supersedes tetra3); reach for
tetra3's simpler pieces only when minimizing dependencies or when you don't need the extra
robustness.

---

## 6. End-to-end rebuild checklist (whole system)

**Shared math (doc 02):** unit-vector ↔ RA/Dec; `2·arcsin(d/2)` angle; pinhole
project/unproject; distortion (un)distort; Wahba SVD attitude + RA/Dec/Roll; edge-ratio key
+ 64-bit hash + table index + open-addressing; centroid-distance ordering; binomial
false-alarm test; residuals.

**Detection (doc 03 or 04):** image → brightest-first `(y,x)` centroids. cedar-detect:
noise estimate (darkest cut) → optional binning → per-row 1D 7-pixel gate → hot-pixel
classify → blob merge → 2D core/neighbors/margin/perimeter gate → quadratic-interp centroid
+ perimeter-subtracted brightness → sort by brightness.

**Database (doc 05):** parse catalog + proper motion + magnitude trim + sort by brightness;
unit vectors + KD-tree; multiscale FOV ladder; density-thin pattern stars; enumerate
4-star patterns (cedar lattice fields or tetra3 neighbor combos); edge-ratio key → hash →
open-address insert (presorted); store `largest_edge` (mrad f16) and (cedar) 16-bit
`key_hashes`; serialize arrays + props to `.npz`.

**Solver (doc 06):** vectorize centroids (+known distortion, +cluster-bust); loop image
patterns (brightest/breadth-first, timeout); pattern key + tolerance band → candidate keys
(nearest-first) → hash probe + pre-filters + edge-ratio band test → per candidate: coarse
FOV, pair stars, SVD attitude (reject `det<0`), gather diagonal-FOV catalog stars, project,
unique-match, binomial accept; on accept re-fit attitude over all matches, extract
RA/Dec/Roll, refine FOV (+`k` least squares), residuals, outputs; status codes on failure.

**Service (doc 07, optional):** gRPC `ExtractCentroids`; image inline-or-shared-memory;
`(x,y)`→`(y,x)` at the boundary; client owns the server subprocess with shared-memory fast
path and inline fallback.

If every box above is implemented per the cited document, you have a complete,
from-scratch reconstruction of the plate solver — detection through attitude.
