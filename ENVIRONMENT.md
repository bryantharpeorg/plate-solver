# Environment & Verification — plate-solver

How code reaches `main`, what must be green before it does, and what "correct" means
for this product.

## How changes land

Every change reaches `main` through a pull request opened under the maintainer's own
GitHub account, by the maintainer or by an AI coding agent working as the maintainer.
GitHub enforces the checks below; nothing else gates a merge.

## Enforcement status — as of 2026-10-18

Every claim below is true at time of writing; update this section when enforcement
changes, and never let it claim more than what is enforced.

| Layer | Status |
|-------|--------|
| Branch protection, `main` | **ENFORCED** — **12 required checks**: every `ci.yml` job except `quality-report` and `test-coverage`, each pinned to the GitHub Actions app so no other app can satisfy it. Enforced for admins, so no account can merge a PR with a red or missing check, including the maintainer's account, which agents also push with. Force-push and deletion are disabled. No review is required: a single account authors and merges, so a review requirement could not tell a human from an agent. `main` is the one working branch; the original v1 implementation is preserved at tag **`v1-original`**. |
| Fast CI (`ci.yml`) | **ENFORCED** — 12 jobs run on every PR and are required: `detect`, `rustfmt`, `clippy`, `docs`, `msrv`, `cargo-machete`, `cargo-deny`, `secret-scan`, `hold`, `e2e-eval`, `e2e-canary` and `finder-page` (the finder console's page: typecheck, format, unit tests, the committed build matching its source, and end-to-end tests in Chromium against the release binary). `hold` is a manual brake: it is red while a PR carries the `hold:human` label, and removing the label re-runs it. `test-coverage` (80% workspace line-coverage floor) is **NOT ENFORCED** since 2026-11-14: it runs only when started by hand (`workflow_dispatch`), because it took 32 to 55 minutes a run. The required `test` gate runs a superset of its tests. |
| End-to-end eval (`e2e-eval`, `e2e-canary`) | **ENFORCED** — both are required checks on `main`. The oracle-integrity step, the owned-test-target and tree-untouched checks, the scorecard judgement and the canary all block merges, alongside the E1–E3 assertions that also run inside the `test` gate. Armed unless the Actions variable `E2E_ARMED` is `false-until-YYYY-MM-DD` and that date has not passed (the score is still published and the attestation says DISARMED; the `test` gate is not disarmed by it). Added 2026-10-17 after the end-to-end audit (`docs/audit-e2e-parity.md`): `main` had passed every other gate and solved 0 of the 21 reference images. |
| Oracle and gate changes | The paths in `.github/CODEOWNERS` decide what "correct" means. With no second account to require a review from, changes to them rely on the maintainer reading the diff. |
| Nightly gates | **NOT INSTALLED** — no mutation testing, fuzzing, Miri or SBOM runs. |

## The oracle — what "correct" means

The vendored `reference-solutions/` (tetra3 / cedar-solve / cedar-detect). The parity
contract: **RA/Dec within a few arcseconds, centroids within ±0.1 px, identical matched
catalog IDs.** These references were not produced by the code under test, so agreement
is external validation. Parity is the product's correctness definition, so parity
scenarios block every PR, not just a nightly run.

Enforced by the `e2e-eval` job, in three tiers, on the 21 sample images in
`reference-solutions/cedar-solve/examples/data/medium_fov/`, against results captured
from cedar-solve (`crates/plate-solver/tests/data/cedar_parity.json`):

- **E1 centroid parity** — the contract as written: same status, identical matched
  catalog IDs, RA/Dec, roll and FOV within 5″ — once without an FOV estimate and once
  with a user-style one (nearest degree, ±1°), each against its own cedar answers.
- **E2 image entry point** — JPEG → `solve_from_image`: identical to the composed
  detect → solve path, and within 10″ of cedar (roll 60″, FOV 15″, ≥90% of cedar's
  stars matched). Looser than E1 because this path solves with every detected star,
  while the goldens used the brightest 30; identical IDs are E1's job.
- **E3 gRPC service** — the same through `SolveFromImage`, `ExtractCentroids` and
  `SolveFromCentroids` with the real database: wire = library (status, attitude, FOV,
  distortion, stars), image = composed path, within E2's tolerances of cedar, and
  `hale_bopp` at sigma 8 is `MATCH_FOUND`.

`scripts/ci/cedar-goldens.py --check` re-derives every golden from the vendored cedar
and pins the inputs they were derived from; its header documents where those inputs
come from.

`e2e-canary` proves the gate can fail and that each check bites: ten probes, each
nudging one golden just past one tolerance or dropping one star ID, and five bugs
re-injected from `main` must each turn the tier that owns them red.

## Critical paths — read the diff before merging

- `crates/math-core/**` — the numerical-parity contract (angle convention, Wahba/SVD
  attitude). A silent change here can pass tests yet break parity.
- any `unsafe` block, anywhere — the audited-site census must stay explicit.
- `proto/**` and the gRPC surface (`crates/grpc-service`) — public API and the
  `(x,y)↔(y,x)` boundary; breaking consumers or the coordinate convention is
  high-blast-radius.
- the database format and serialization (`crates/pattern-database`,
  `crates/database-generation`) — the on-disk format is a compatibility contract.
- any change to the dependency graph (`Cargo.toml` / `Cargo.lock`).
- the oracle and its gate — `reference-solutions/**`, the goldens under
  `crates/*/tests/data/`, the e2e tests (`reference_parity.rs`, `image_parity.rs`,
  `e2e_service.rs`), `scripts/ci/**`, `.github/**`, and whatever decides what runs
  under a test's name: every `Cargo.toml`, `build.rs`, `.cargo/**` and the toolchain
  pin. A change here can make a broken product look green without touching product
  code (`.github/CODEOWNERS`).

## Performance targets (not yet asserted by CI)

- Detection: **< 10 ms per 1 M pixels** on RPi-4B-class hardware.
- Solve: ~**10 ms** desktop-class excluding extraction; per-platform mobile budgets
  fixed in `mobile-runtime`.
- Database **memory-mappable** with bounded peak RAM on device.
