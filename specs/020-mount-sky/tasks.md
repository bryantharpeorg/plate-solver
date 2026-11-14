---
description: "Task list for 020-mount-sky: A Drawn Sky on the Real Mount"
---

# Tasks: A Drawn Sky on the Real Mount

**Prerequisites** (plan.md § Prerequisites):
- `016-mcp`, `017-console-stop-confirm` and `018-live-chart-steady` have landed on `main`;
- the owner approvals in spec.md are given;
- Constitution III is amended (1.2.0) before dispatch;
- no operator-staged file is needed.

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. Do not edit it.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Change nothing under `crates/scope-node`, `crates/scope-link` or `crates/simulator`,
  and add no dependency.
- Tests compute their expected values themselves (plan.md § Plan traps 6).
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — GoTo against a drawn sky on the real mount (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/mount_sky.rs` with the shared setup of
  spec.md § User Scenarios, modelled on `Rig::new` in
  `crates/finder/tests/mcp/support.rs` but calling `with_mount_sky(transport, 30.0)`,
  with the `SimMount`'s zero points at `[300, 70]`, and helpers for the truth of counts
  and for horizontal from a pointing. Write failing tests for US1-S1 and US1-S2.
- [ ] T002 [US1] Add failing tests to `crates/finder/tests/mount_sky.rs` for the GoTo at
  night and its refusal in the day (US1-S3, US1-S4), and for a node without a mount
  (US1-S5).
- [ ] T003 [US1] Add failing tests to `crates/finder/tests/mount_sky.rs` that run the
  built binary for the refused flag combinations and the `mount-sky/` state folder
  (US1-S6, US1-S7).

### Implementation

- [ ] T004 [US1] In `crates/finder/src/link.rs`, add `FrameFilter` and
  `LinkManager::start_with_filter` exactly as plan.md § Interfaces gives them, with
  `start` calling it with `None`.
- [ ] T005 [US1] Create `crates/finder/src/mount_sky.rs` with `MountSky`,
  `MountSkyView` and the filter of plan.md § Interfaces, and export it from
  `crates/finder/src/lib.rs`. Add `Session::mount_sky_terms` in
  `crates/finder/src/session.rs`.
- [ ] T006 [US1] In `crates/finder/src/api.rs`, add `AppState::with_mount_sky` and the
  `mount_sky` field of `SessionResponse`. In `crates/finder/src/main.rs`, add
  `--mount-sky` and `--tube-alt`, the `mount-sky` state folder and the start-up line.
- [ ] T007 [US1] Add US1's two rows and the `## Drawn sky on the real mount` section to
  `crates/finder/README.md` (plan.md § Interfaces).
- [ ] T008 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US1-S1 to US1-S7).

## Phase 2: User Story 2 — The same test in daylight, on a night clock (Priority: P2)

### Tests first

- [ ] T009 [US2] Create `crates/finder/tests/mount_sky_night.rs` with failing tests for
  the refused flag combinations (US2-S1, US2-S2), the running binary on a night clock
  (US2-S3) and `night_utc` present only with a night clock (US2-S4).

### Implementation

- [ ] T010 [US2] In `crates/finder/src/mount_sky.rs`, add `with_night_utc` and
  `MountSkyView::night_utc`. In `crates/finder/src/api.rs`, add `with_night_clock` and
  pass the night's UTC to the `MountSky`. In `crates/finder/src/main.rs`, add
  `--mount-sky-night` and `--covers-on` with `parse_night` and the warning line.
- [ ] T011 [US2] Add US2's two rows and paragraph to `crates/finder/README.md`, and the
  new Sun guard entry to `GLOSSARY.md` (plan.md § Interfaces).
- [ ] T012 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1 to US2-S4).

## Phase 3: User Story 3 — The page says the sky is drawn (Priority: P3)

### Tests first

- [ ] T013 [US3] Create `crates/finder/web/src/components/MountSkyBanner.test.tsx` with
  failing tests for US3-S1, US3-S2 and US3-S3.

### Implementation

- [ ] T014 [US3] Add the `MountSky` type to `crates/finder/web/src/api.ts`, create
  `crates/finder/web/src/components/MountSkyBanner.tsx`, and place it in
  `crates/finder/web/src/App.tsx` (plan.md § Interfaces).
- [ ] T015 [US3] Add the banner paragraph to `DESIGN.md`. Run the `lint` and `test`
  gates with every existing Vitest and Playwright test unchanged and green, commit the
  rebuilt `crates/finder/web/dist`, and commit the green story (US3-S1, US3-S2 and US3-S3).

## Execution Order

```text
US1 ──► US2
   └──► US3
```

- **US1** comes first: US2 and US3 build on its flags, `MountSky` and `mount_sky` field.
- **US2** and **US3** share no file, so they may run beside each other once US1 has
  merged.
- **`[P]`** marks nothing here: every task shares a file with another in its story.
