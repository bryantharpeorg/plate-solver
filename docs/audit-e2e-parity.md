# End-to-end audit — does the rewrite solve real images?

> **Historical snapshot, 2026-10-17.** The fix landed the same week, and the parity gate it
> called for is now the required `e2e-eval` check, with `e2e-canary` proving it can fail. The
> detection divergence (T1–T3) was fixed in #98 on 2026-10-18; cedar then solves 20 of the 21
> images from this project's own centroids, as it does from its own. Findings still marked
> open below were not re-audited.

**Artifact under audit:** `main` @ `a275521`
**Audit date:** 2026-10-17 · **Reference:** cedar-solve / cedar-detect in `reference-solutions/`
**Fix:** commit *"Make the solver solve: cedar parity end to end"* (this branch)

The short version: **on `main` the solver solves none of cedar's 21 sample images; cedar
solves 19 of them from the exact centroids our detector produces.** All 156 tests pass,
because no test runs a real solve. Each stage is close to right and passes its own tests;
the stages are wrong *together* — degrees passed where radians are expected, camera-frame
vectors compared with sky-frame vectors, `(x, y)` handed to code that wants `(y, x)`, a
tolerance passed into the wrong parameter, a threshold divided twice.

With the fix (+359/−191 lines in the solver crate, +121/−30 elsewhere) the
Rust solver solves **the same 19 images, with identical matched HIP IDs, RA/Dec within
0.003″ and roll within 0.004″ of cedar, in 1–16 ms per solve** (cedar: 2–264 ms). That
check is now a test: `crates/plate-solver/tests/reference_parity.rs`, which fails on `main`.

The spec asked for exactly this test (plate-solver spec, *Reference parity* scenario).
It was never built, and nothing in the gates noticed.

---

## 1. How this was measured

| Input | Source |
|---|---|
| 21 sky images | `reference-solutions/cedar-solve/examples/data/medium_fov/` |
| Pattern database | `reference-solutions/cedar-solve/tetra3/data/default_database.npz` — hip_main, 10–30°, 1,010,981 patterns (the database in use) |
| Centroids | our `star_detection::detect_stars` (σ 8, binning 2, hot pixels, row normalization), brightest 30, `(y, x)` |
| Reference solve | cedar-solve `Tetra3.solve_from_centroids` on **the same centroids**, no FOV estimate, `distortion=0`, 5 s timeout — the settings in cedar's own example script |

Feeding both solvers identical centroids isolates the solver from the detector. Each bug
below was then confirmed by **ablation**: with every fix applied, re-introduce one bug at a
time and re-run all 21 images.

## 2. Results

| | Solved | Matched HIP IDs identical to cedar | Worst RA/Dec vs cedar | Solve time |
|---|---|---|---|---|
| cedar-solve (Python) | 19 / 21 | — | — | 2–264 ms |
| Rust, `main` | **0 / 21** (14 time out at 5 s) | — | — | up to 5 s |
| Rust, fixed | **19 / 21** (same 19) | 19 / 19 | 0.003″ | 1–16 ms |

The two images neither solves (`roof`, 4 stars; `virgo`, 12 stars) are cedar `NO_MATCH`s too.

### What each bug costs on its own (ablation)

All fixes applied, then one bug put back:

| Bug put back | Solved | IDs identical | Worst FOV error | Worst RMSE | Max solve |
|---|---|---|---|---|---|
| none (fixed) | 19 | 19 | 0.0000005° | 34″ | 16 ms |
| S1 FOV pre-filter wiring | **0** | 0 | — | — | 435 ms |
| S2 `(x, y)` in `solve_from_image` (image path) | **0** | 0 | — | — | 5 s |
| S3 small-angle coarse FOV | 8 | 4 | 0.026° | 114″ | 499 ms |
| S4 band from the quantized key | 13 | 12 | 0.003° | 29″ | 2.6 s |
| S5 refinement frames | 19 | 19 | **536°** | **462,000″** | 13 ms |
| S6 verify on cluster-busted subset | 19 | **3** | 0.005° | 35″ | 12 ms |
| S7 double Bonferroni | 18 | 18 | 0.0000005° | 34″ | 300 ms |
| S8 combination order | 19 | 19 | 0.002° | 34″ | 26 ms |

S1 and S2 are each sufficient to block every solve; `main` has both.

## 3. Findings

Line numbers are on `main`. Severity: **B** blocks solving, **W** wrong output, **P** parity
or performance, **H** hygiene.

### Solver (`plate-solver`, `math-core`)

| # | Sev | Where | What is wrong | Fix |
|---|---|---|---|---|
| S1 | B | `candidates.rs:158` | The largest-edge/FOV pre-filter receives `match_max_error` (0.002, a dimensionless ratio tolerance) as the FOV tolerance **in degrees**, and runs even when no FOV estimate was given (using the DB midpoint). f16 edge storage alone exceeds ±0.002°, so every true candidate is rejected. | Apply only when the caller gave an estimate and a tolerance (cedar). |
| S2 | B | `solve.rs:113` | `solve_from_image` builds centroids as `(s.x, s.y)`; the solver wants `(y, x)`. A transposed image is a mirror image, so every attitude comes out a reflection and is rejected. Breaks gRPC `SolveFromImage`. | `(s.y, s.x)`. |
| S3 | B | `verify.rs:77`, `fov.rs:32` | Coarse FOV from a linear pixel→angle formula that ignores the estimate; it evaluates to `2·tan(fov/2)` instead of `fov` (+1% at 20°, **+8% at 54°**). Pattern vectors are not recomputed at the coarse FOV before the attitude fit. | Cedar's angle-ratio formula; recompute vectors (cedar). |
| S4 | B | `candidates.rs:130` | Tolerance band centred on `floor(ratio·bins)/bins` instead of the measured ratio. With bins = 250 and ±0.002 the band misses the true ratio about half the time per ratio (~3% of patterns pass all five); key enumeration also widens to 3⁵ keys. | Band from the unquantized ratios; cedar's key range. |
| S5 | W | `refine.rs:49–60`, `fov.rs:88`, `residuals.rs` | Refinement passes **sky-frame** catalog vectors where camera-frame vectors are expected: FOV is scaled by (angle from RA 0/Dec 0) / (angle from boresight), and residuals are angles between camera-frame and sky-frame vectors. Re-fit uses vectors at the initial, not the coarse, FOV. | Cedar's refinement: derotate catalog vectors, least-squares f and k, residuals in the sky frame. |
| S6 | P | `preparation.rs:147`, `verify.rs:130` | Verification matches against the cluster-busted subset, so `n` and the match count shrink and busted stars can never match. Cedar forms patterns from the busted subset but verifies against all brightest-N centroids. | Carry both lists. |
| S7 | P | `verify.rs:138` + `binomial.rs:146` | `match_threshold` is divided by `num_patterns` in `build_context` and again in `false_alarm_test`: effective threshold ≈1e-17 instead of cedar's ≈1e-11 (≈4 more matches needed at 30 stars). | Divide once. |
| S8 | P | `solve.rs:155` | "Breadth-first" loop over a growing index radius revisits every combination at each radius and tries `(1,2,3,4)`, `(2,3,4,5)` … down to the faintest stars before `(0,1,2,4)`. | Cedar's order: each combination once, by faintest member. |
| S9 | W | `solve.rs:130` | `solve_from_image` overwrites `match_probability` (the false-alarm probability) with the noise estimate — and `noise_estimated_from_image_never_constant` asserts on it, so the test encodes the bug. | `Solution::noise_estimate`. |
| S10 | W | `binomial.rs:70` | For `k ≥ n/2` the CDF is computed as `1 − upper`, which cannot represent values below ~1e-16. On `jan1_g90_e50ms` Rust reports `Prob` 3.4e-10 where cedar reports 3.5e-18 (the other 18 agree to 15 digits). Acceptance is unaffected at cedar's ≈1e-11 threshold; at S7's ≈1e-17 the decision was rounding noise. Sum the small tail directly. | Not in patch. |
| S11 | P | `verify.rs:93` | Gathers catalog stars within the full diagonal; cedar uses half (4× the area, then discarded). The spec says "the diagonal" — see P3. | Half diagonal. |
| S12 | H | `preparation.rs:127` | Each solve creates its own cancel flag with no handle out, so `CANCELLED` is unreachable from the public API. | Not in patch. |
| S13 | W | `refine.rs:49` | Distortion is estimated only when `distortion != 0`; cedar estimates `k` unless `distortion is None` (its default, 0, estimates). | Estimate always (cedar default). |

### Pattern database and generation

| # | Sev | Where | What is wrong | Fix |
|---|---|---|---|---|
| D1 | B | `load.rs:353` | Properties decoder accepts only `S` strings: cedar/tetra3 databases (`<U64`, plus bool and `(2,)` fields) fail with `unsupported properties field type '<U64'`. The database in use cannot be loaded. | Decode `U`; skip fields not needed. |
| D2 | P | `load.rs:388`, `load.rs:72` | Every array element is read into its own `Vec<u8>`: the 14.6 MB cedar database peaks at **1.2 GB** RSS (12 M heap allocations), so it cannot load on a 1 GB Pi. `load_mmap` maps the file and then copies everything anyway. | Not in patch: decode arrays in bulk; keep the catalog as `u16`/`u32`. Bulk decode since **fixed**: peak RSS 159 MB, load ≈0.16 s from a local disk (was ≈1.3 s). The catalog is still widened to `usize`. |
| D3 | B | `catalog.rs:115` | The HIP/TYC parsers expect an invented 6/8-column layout; the real CDS `hip_main.dat` (field 0 = `H`) yields *"star catalog contains no usable entries"*. The fixtures meant to test this, `hip_sample.txt` and `tyc_sample.txt`, are empty files. | Read the CDS layout (fields 1, 5, 8, 9, 12, 13), as cedar does. |
| D4 | B | `patterns.rs:133` | `breadth_first_combinations` is lexicographic (depth-first): with a budget of 40–50 per lattice field, every pattern pairs the 2–3 brightest stars with ever-fainter ones. The fixed solver solves 16 of 19 with these databases. The unit test uses 5 stars, where both orders coincide. | Cedar's order; databases then solve 19 / 19 with identical IDs. |
| D5 | P | `cleanup.rs:33` | Magnitude limit by exact count; cedar takes the left edge of the first 100-bin histogram bucket past the count. 10–30°: 69,049 stars to mag 8.67 vs cedar's 42,212 to 8.03. | Not in patch. |
| D6 | H | `serialize.rs:320` | `hash_table_type` written as `\|S14`, truncating `quadratic_probe` to `quadratic_prob`; cedar's fields (`presort_patterns`, lattice settings) are not written. | Not in patch. |
| D7 | P | — | With D3–D4 fixed and cedar's parameters, the generator still produces 1,119,746 patterns vs cedar's 1,010,981. Solves at parity; cause not yet traced. | Open. |

### Star detection

Exact parity with cedar-detect where it matters most, divergence on three options
(sample images, σ 8):

| Mode | cedar-detect | Rust | Matched ≤1 px | Worst offset | Top-30 same rank |
|---|---|---|---|---|---|
| binning 1 | 2,676 | 2,676 | 2,675 | 0.000 px | 575 / 582 |
| binning 1, hot pixels | 1,760 | 1,760 | 1,760 | 0.000 px | 566 / 573 |
| binning 2 | 1,030 | 1,030 | 1,030 | 0.000 px | 523 / 530 |
| binning 2, hot pixels | 915 | **1,030** | 906 | 0.087 px | 279 / 491 |
| binning 2, `normalize_rows` | 1,009 | **961** | 927 | **0.857 px** | 222 / 527 |
| binning 4 | 269 | 269 | 269 | **0.707 px** | **0 / 268** |

- **T1** `detect.rs:54` — hot-pixel rejection runs only at binning 1; cedar-detect checks the full-resolution pixels under binned candidates. 115 hot pixels survive across the samples.
- **T2** `normalize_rows` diverges (fewer stars; `virgo` solves without it, not with it).
- **T3** binning 4/8: centroids off by up to 0.7 px and brightness order unrelated to cedar's.
- The existing parity test covers binning 1 only (`cedar_goldens_bin1_sigma8.json`).

### gRPC service

| # | Sev | Where | What is wrong |
|---|---|---|---|
| G1 | B | `server.rs:79` | `ExtractCentroids` swaps `x`/`y`: `Star` already has named fields. A star at column 9, row 5 comes back as `x=5.5, y=9.5`. The existing tests use `(4,4)` and `(10,10)`, which can't tell. **Fixed**, with an asymmetric test. |
| G2 | H | — | No binary starts the server; the crate is a library only. |
| G3 | P | `server.rs:140,188` | The whole database is cloned per request (≈100 MB+ for 10–30°). The solver should take `&PatternDatabase`. **Fixed**: the solve entry points take `impl Borrow<PatternDatabase>` and the server passes `&*self.db`. |
| G4 | H | `server.rs:285` | Default timeout 10 s (spec: 5 s); `t_extract_ms` always 0; `hot_pixel_count` always 0; shared memory stubbed. |

### Spec and process

- **P1 — the gate that would have caught all of it was specified and not built.** The
  plate-solver spec's *Reference parity* scenario asks for exactly the test added here.
  Twelve green checks per PR did not include it.
- **P2 — tests that cannot fail.** Empty databases; symmetric coordinates for a
  coordinate-order test; a 5-element case for a combination-order test; empty catalog
  fixtures; and one test (S9) asserting the bug. A useful rule for reviewers: a test for an
  ordering, swap, or unit must use inputs where the wrong answer differs from the right one.
- **P3 — spec defects the implementation followed faithfully:**
  - *Diagonal FOV gathers candidates* says the radius is the diagonal; cedar uses half.
  - `size=(height,width)` in the spec, `(width, height)` in the code (consistent in code,
    a trap for callers).
  - Refinement: "distortion by least squares when distortion was not a fixed scalar" —
    cedar's actual rule is "unless `None`", and the API has no `None`.
  - "Swap at the service boundary" was applied to `Star`, which has named fields.
  - Target pixel ↔ sky conversions are specified but not implemented. They're needed for
    telescope use: the eyepiece is rarely at the image centre.

## 4. Wide-field accuracy (beyond parity)

For a camera on a telescope mount, cedar's algorithm itself is the limit on wide,
distorted lenses: one refinement pass, attitude fitted before the FOV/distortion
refinement. Synthetic 2592×1944 frames (binned 2×, hot pixels, light-pollution
gradient, cedar's radial distortion model), solved against a 30–60° database:

| Frame (true FOV, k) | cedar = fixed Rust (pos / roll error) | + iterative refinement |
|---|---|---|
| Cassiopeia (45°, −0.05) | 2.3′ / 5.3′ | 0.08′ / 0.07′ |
| Cygnus (50°, 0) | 1.0′ / 0.7′ | 0.06′ / 0.03′ |
| Cygnus (50°, −0.05) | **17.4′ / 24.2′** | 0.04′ / 0.05′ |
| Cygnus (50°, −0.10) | 0.4′ / 0.05′ | 0.04′ / 0.03′ |
| Pegasus (54°, −0.08) | 6.0′ / 1.3′ | 0.07′ / 0.01′ |

cedar and the fixed Rust solver return identical answers on these (Rust 0–58 ms, cedar
2–1,522 ms). Three passes of re-match → least-squares f, k → attitude re-fit bring every
frame under 5″. On the 21 real 10–30° images the same loop moves results by a median 3.7″
(max 15.6″) and lowers median RMSE from 10.3″ to 8.2″ — so it must be an **opt-in mode**,
not the default, or it breaks the parity contract. A prototype is in a separate patch
(`PLATE_SOLVER_ITERATIVE_REFINE`); it should become a solve option through a spec change.

Caveat: the synthetic frames use the same single-term distortion model the solver fits;
real lenses will be messier.

## 5. Recommended order of work

1. **Make `reference_parity` a required check.** The test itself takes ~4 s in release and
   ~10 s in a debug build, using files already in the repo.
2. Land the fix commit (S1–S9, S11, S13, D1, D3, D4, G1).
3. Detection parity for the binned path (T1–T3): extend the golden file to binning 2
   with hot pixels and `normalize_rows`, and binning 4.
4. Loader memory (D2) and the per-request clone (G3) before any Pi or phone deployment.
5. Target pixel ↔ sky conversions (P3), then iterative refinement as an option (§4).
6. Spec corrections (P3) so a rebuild from specs wouldn't reproduce these.

## 6. Reproduce

```bash
cargo test --release -p plate-solver --test reference_parity      # fails on main, passes here
cargo run --release -p plate-solver --example solve_image -- \
    --db reference-solutions/cedar-solve/tetra3/data/default_database.npz \
    reference-solutions/cedar-solve/examples/data/medium_fov/*.jpg
cargo run --release -p database-generation -- hip_main.dat wide.npz --min-fov 30 --max-fov 60
```

`hip_main.dat` is CDS catalog I/239 (`https://cdsarc.cds.unistra.fr/ftp/cats/I/239/hip_main.dat`).
The gRPC crate needs `protoc`; `-p plate-solver` and `-p database-generation` don't.

Verification: 160 tests pass (156 existing + 4 new), clippy and rustfmt clean, ten
consecutive full runs. One earlier run reported a single failure that did not reproduce.
