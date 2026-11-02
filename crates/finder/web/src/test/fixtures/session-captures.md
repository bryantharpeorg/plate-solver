# Simulator session captures for US8

`session-*.json` are unedited `GET /api/session` responses, formatted with
Prettier. `record-sessions.rs` reproduces them through the existing US6 harness:
real catalog rendering and plate solving, simulated mount, in-process scope link,
and externally supplied paused virtual time starting 2026-01-16 02:00:00 UTC.
The script's header gives the exact capture command. It adds a temporary test;
no existing test is edited.
Refused idle sessions include `goto_refusal`, captured from the shared planner;
accepted and running sessions omit it. No reason is supplied by the page.

- `session-ready`: connected, still mount with one solved sync, target M42.
- `session-goto-m42`: every distinct state/correction pair of a completed M42
  GoTo, including correction 1 and its final solved error in arcseconds.
- `session-below-horizon`: M8 below the horizon at 40° N, 75° W.
- `session-sun-up`: a site with the Sun at upper transit, altitude 5°. M31 is
  pointable according to target guidance, while motor motion is refused by the
  Sun-up guard. This exercises a different condition from target Sun separation.
- `session-no-sync`: the nightly model has just been reset.
- `session-low-approach`: target geometric altitude 15.2°, below the safe upward
  approach's required clearance above the 15° minimum altitude.
- `session-second-sync-slewing`: the one-tap second sync just accepted, before
  telemetry has reported moving axes; Stop must already be available.
- `session-two-syncs`: two syncs at least 90° apart on the simulator's base,
  exactly 1° off level toward 130°. Display tolerances are 0.1° in magnitude and
  2° in direction. The capture fits 0.9996° toward 129.9969°; RMS display rounds
  the recorded residual to 0.1 arcsecond. No expected value is derived from the
  page's formatting code.
