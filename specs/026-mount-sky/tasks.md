---
description: "Task list for 026-mount-sky: A Drawn Sky on the Real Mount"
---

# Tasks: A Drawn Sky on the Real Mount

**Prerequisites** (plan.md § Prerequisites):
- `016-mcp`, `017-console-stop-confirm`, `018-live-chart-steady`, `020-live-chart-calm`
  and `024-mount-link-recovery` have landed on `main`;
- the owner approvals in spec.md are given;
- Constitution III is amended (1.4.0) before dispatch;
- no operator-staged file is needed;
- the refinement probe has run (met on 2026-11-17, one round).

**Rules for every story:**
- Never edit an existing test, golden, fixture, `scripts/ci/`, `.github/`, `deny.toml`,
  `CODEOWNERS`, `ergane.yaml`, `.specify/` or `crates/finder/web/playwright.config.ts`.
  If an existing test fails, stop and escalate with its exact output. Do not edit it.
  The one exception is the `mod` line of each new finder test module, added in its
  alphabetical place to the `main.rs` your story's tasks name
  (`crates/finder/tests/motion/main.rs` or `crates/finder/tests/cli/main.rs`), neither of
  which holds a test (constitution 1.2.0). Change nothing else there.
- A new finder test is a module file in its binary's folder, never a file at the top
  level of `crates/finder/tests` or in `crates/finder/tests/support`, either of which
  fails the `layout` test. It reaches shared helpers through `crate::support`, never
  with `#[path]`, takes every temporary folder from `crate::support::Scratch`, and holds
  only the setup its own tests use, since an unused helper fails `clippy -D warnings`.
- Write only the files your story's tasks name; they are its `writes` in plan.md
  § Refinement Record. A story that needs another file escalates.
- Change nothing under `crates/scope-node`, `crates/scope-link` or `crates/simulator`,
  and add no dependency.
- Tests compute their expected values themselves (plan.md § Plan traps 6).
- Run no tool the gates don't run: no `cargo deny`, `cargo machete`, Rust 1.96 or
  `ergane`.

## Phase 1: User Story 1 — GoTo against a drawn sky on the real mount (Priority: P1)

### Tests first

- [ ] T001 [US1] Create `crates/finder/tests/motion/mount_sky.rs`, declared by
  `mod mount_sky;` after `mod mount_identity;` in `crates/finder/tests/motion/main.rs`,
  with the shared setup of spec.md § User Scenarios, copied from `Rig::new` in
  `crates/finder/tests/support/mcp.rs` by way of its copy in
  `crates/finder/tests/motion/mount_identity.rs`, which has a settable reset count
  (keeping its `DB`, `NodeTime`, `Stars` and `Recorded`, with
  `use crate::support::mcp as support;`, and its solver's `timeout_ms: u64::MAX`), but
  without its `set_aim` (`crates/finder/tests/support/mcp.rs:248`) and calling
  `with_mount_sky(transport, 30.0)`, with the `SimMount`'s zero points at `[300, 70]`,
  driven to counts `[20000, 10000]` before the console is built, behind a test `Mount`
  whose reset count the test sets, and helpers for the truth of counts and for
  horizontal from a pointing. Write failing tests for US1-S1 and US1-S2.
- [ ] T002 [US1] Add failing tests to `crates/finder/tests/motion/mount_sky.rs` for the
  GoTo at night and its refusal in the day, each once the session's model holds a sync
  (US1-S3, US1-S4), for a node without a mount (US1-S5), and for the drawn sky stopping
  for good after a reset (US1-S8).
- [ ] T003 [US1] Create `crates/finder/tests/cli/mount_sky.rs`, declared by
  `mod mount_sky;` after `mod flags;` in `crates/finder/tests/cli/main.rs`, with failing
  tests that run the built binary, started as `crates/finder/tests/cli/simulate.rs`
  starts it but without its `--simulate` argument, for the refused flag combinations
  and the `mount-sky/` state folder (US1-S6, US1-S7). A refusal test fails, killing the
  binary, as soon as standard error shows a `finder console:` line, instead of waiting
  on `output()`.

### Implementation

- [ ] T004 [US1] In `crates/finder/src/link.rs`, add `FrameFilter` and
  `LinkManager::start_with_filter` exactly as plan.md § Interfaces gives them, with
  `start` calling it with `None`.
- [ ] T005 [US1] Create `crates/finder/src/mount_sky.rs` with `MountSky`,
  `MountSkyView` (with `power_cycled`) and the filter of plan.md § Interfaces, and
  export it from `crates/finder/src/lib.rs`. Add `Session::mount_sky_terms`, with the
  last-seen reset count, in `crates/finder/src/session.rs`.
- [ ] T006 [US1] In `crates/finder/src/api.rs`, add `AppState::with_mount_sky` and the
  `mount_sky` field of `SessionResponse`. In `crates/finder/src/main.rs`, add
  `--mount-sky` and `--tube-alt`, the `mount-sky` state folder and the start-up line.
- [ ] T007 [US1] Add US1's two rows and the `## Drawn sky on the real mount` section,
  with its ban on hand pushes and the restart after a power cycle, to
  `crates/finder/README.md` (plan.md § Interfaces).
- [ ] T008 [US1] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US1-S1 to US1-S8).

## Phase 2: User Story 2 — The same test in daylight, on a night clock (Priority: P2)

### Tests first

- [ ] T009 [US2] Create two failing test modules:
  - `crates/finder/tests/cli/mount_sky_night.rs`, declared by `mod mount_sky_night;`
    after `mod mount_sky;` in `crates/finder/tests/cli/main.rs`, for the refused flag
    combinations (US2-S1, US2-S2), each refusal test killing the binary as US1's
    `crates/finder/tests/cli/mount_sky.rs` does, and the running binary on a night
    clock (US2-S3);
  - `crates/finder/tests/motion/mount_sky_night.rs`, declared by `mod mount_sky_night;`
    after `mod mount_sky;` in `crates/finder/tests/motion/main.rs`, for `night_utc`
    present only with a night clock (US2-S4), with its own copy of the part of that
    setup it needs (the node, the session and `with_mount_sky`, with no solver or
    recording mount) from US1's `crates/finder/tests/motion/mount_sky.rs`, which it
    does not edit.

### Implementation

- [ ] T010 [US2] In `crates/finder/src/mount_sky.rs`, add `with_night_utc` and
  `MountSkyView::night_utc`. In `crates/finder/src/api.rs`, add `with_night_clock` and
  pass the night's UTC to the `MountSky`. In `crates/finder/src/main.rs`, add
  `--mount-sky-night` and `--covers-on` with `parse_night` and the warning line.
- [ ] T011 [US2] Add US2's two rows and paragraph to `crates/finder/README.md`, and the
  night clock sentence to the end of the Sun guard entry's line in `GLOSSARY.md`
  (plan.md § Interfaces).
- [ ] T012 [US2] Run the `lint` and `test` gates with every existing test unchanged and
  green. Commit the green story (US2-S1 to US2-S4).

## Phase 3: User Story 3 — The page says the sky is drawn (Priority: P3)

### Tests first

- [ ] T013 [US3] Create `crates/finder/web/src/components/MountSkyBanner.test.tsx` with
  failing tests for US3-S1, US3-S2, US3-S3 and US3-S4, rendering `App` over a served
  session as plan.md § Technical Context says.

### Implementation

- [ ] T014 [US3] Add the `MountSky` type, with `power_cycled`, to `crates/finder/web/src/api.ts`, keep
  `mount_sky` in `request` in `crates/finder/web/src/useSession.ts`, create
  `crates/finder/web/src/components/MountSkyBanner.tsx`, and place it in
  `crates/finder/web/src/App.tsx` (plan.md § Interfaces).
- [ ] T015 [US3] Add the banner paragraph to `DESIGN.md`. Run the `lint` and `test`
  gates with every existing Vitest and Playwright test unchanged and green, commit the
  rebuilt `crates/finder/web/dist`, and commit the green story (US3-S1, US3-S2, US3-S3 and US3-S4).

## Execution Order

```text
US1 ──► US2
   └──► US3
```

- **US1** comes first: US2 and US3 build on its flags, `MountSky` and `mount_sky` field.
  It needs 024's reset count, so 024 has landed before dispatch.
- **US2** and **US3** share no file, so they may run beside each other once US1 has
  merged.
- **`[P]`** marks nothing here: every task shares a file with another in its story.
