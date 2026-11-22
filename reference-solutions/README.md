# Reference solutions

These are pinned copies of the projects this repository's plate solver re-implements. They are
test oracles, never runtime dependencies (`.specify/memory/constitution.md`, ADR-0003):

- CI runs the cedar-solve copy on every pull request to re-derive the parity goldens
  (`scripts/ci/cedar-goldens.py --check`).
- `bench/cedar-compare` builds the cedar-detect copy, and `star-detection`'s parity test reads
  its test images.
- The finder console's default pattern database and sample frames come from the cedar-solve
  copy.

Each copy keeps its own license, and neither is covered by this project's license. `docs/`
is this project's own notes on the algorithms, not a copy.

| Copy | Upstream | Commit | License | Changed here |
|---|---|---|---|---|
| `cedar-solve/` | [smroid/cedar-solve](https://github.com/smroid/cedar-solve), version 0.5.1 | [`680e41cce4c6`](https://github.com/smroid/cedar-solve/commit/680e41cce4c681398f6d329969a6dcc9079ebe7b), 2026-05-29 | Apache-2.0 | Nothing |
| `cedar-detect/` | [smroid/cedar-detect](https://github.com/smroid/cedar-detect), version 0.8.0 | [`74a1d44ac8da`](https://github.com/smroid/cedar-detect/commit/74a1d44ac8da8560013a9e3f4a365260b98dc94b), 2026-05-31 | FSL-1.1-MIT | `Cargo.toml`: a cargo-machete ignore for `prost` (`c151c3c`) |

Each commit was found on 2026-11-21 by matching every copied file's git blob hash against the
upstream tree. All 72 of cedar-solve's files are identical to its commit. 30 of cedar-detect's 31
are identical, and the 31st is the `Cargo.toml` change above.

ESA's [tetra3](https://github.com/esa/tetra3) (Apache-2.0) is not copied here. cedar-solve
carries its own fork of tetra3 under `cedar-solve/tetra3/`. A full copy of esa/tetra3 at
[`f9fa2eb9a32a`](https://github.com/esa/tetra3/commit/f9fa2eb9a32a5efc529e2d86f0b59f35b1e9028d)
sat here unused from June to November 2026. It was removed then, and git history keeps it.

Updating a copy changes the oracle, so it is the owner's decision (`.github/CODEOWNERS`).
Replace the folder with the upstream tree at the new commit and update this table. Expect the
goldens and parity tests to need re-deriving.
