# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

React + TypeScript + Vite + Tailwind CSS for the page, served by the Rust `finder` binary
(axum), with the production bundle committed and embedded in the binary so `cargo build` and
CI never need Node. The user ruled out plain HTML/CSS/JS ("bland, and scalability suffers")
and delegated the framework: React was chosen for the ecosystem the later pieces need (live
camera view, mount controls, an AI chat panel, offline use) and because v1's `ps-web` already
proved this shape in this repo.

## Users

One amateur astronomer, the project's maintainer, with a Sky-Watcher 400P (16-inch) GoTo
Dobsonian and a finder camera. They use the console about equally at the telescope at night
and indoors testing frames, on an Android phone or a Windows laptop.

## Product Purpose

The finder console turns one camera frame into where the telescope is pointing, what is in
and near the field, and what to look at next. It solves still frames (the bundled samples
or an upload) and, paired with the scope node, the finder camera's live stream. From those it
gives push-to directions, drives GoTo and Track, runs an imaging camera from focus to a live
stack, and lets an AI assistant steer over MCP. A SkySafari link and voice mode are still to
come. Success: at a glance, and with proof, the user can see where the telescope points and
what it is doing.

## Positioning

The solver behind it gives cedar's answers on cedar's reference frames and runs fast enough
for a live camera stream (`docs/solver.md`), and the console goes past push-to to
closed-loop GoTo, tracking, imaging and AI-steered observing.

## Operating Context

- At the telescope: red light only, phone in one hand, glancing between eyepiece and screen.
  The internet is often there but never needed.
- Indoors: desk in normal light, a Windows laptop (WSL2) or phone browser.
- Three kinds of device on one Wi-Fi network (ADR-0001):
  - the console runs in WSL2 on the laptop: it solves, keeps the pointing model, plans moves,
    runs imaging and serves the page and MCP;
  - a scope node (a Raspberry Pi 3B+ riding on the telescope) runs the finder camera (an
    Arducam OV5647 with a 16 mm lens, on its CSI ribbon), a USB cable straight to the mount's
    motor controller and, through INDI, the imaging camera; it finds the stars in each frame
    and carries out axis commands;
  - the phone and the laptop open the page, and share one observing session.
- Planned: a Raspberry Pi 5 on the telescope runs both the console and the scope node, and
  the laptop reaches the page over an SSH tunnel (`docs/architecture.md`).
- The site is typed once (the laptop has no GPS); time comes from the console's clock.
- Out of scope for now: SkySafari's link, the voice assistant and the mobile app. They will
  use the same pointing state when they come. AI clients already reach it over MCP.

## Capabilities and Constraints

- Solve a frame: star detection and a lost-in-space solve with no pointing prior; RA/Dec,
  roll, field of view, matched catalog stars, residuals, timing.
- After a solve, "where it points" comes first: large RA/Dec, the constellation, and what is
  in the field. The annotated frame comes second (user's answer).
- The annotated frame shows detected stars, matched catalog stars with names, and Messier
  objects in the field: the proof.
- Next targets: objects near the field, and objects similar to one in it that are up now.
  Rules today; AI later.
- Works offline; online extras (Aladin survey imagery) are optional.
- The default database covers 10–30° fields; frames from other cameras need a matching one.
  The finder camera's 16 mm lens gives 13.0° × 9.8°, inside it.
- Built in four steps (specs 012–015, landed 2026-10-30 to 11-03; `openspec/project.md` §3):
  1. Targets and push-to, with no hardware: search OpenNGC (Messier, NGC, IC, common names)
     for a target, learn whether it is pointable (above the minimum altitude, default 15°,
     and outside the Sun guard, which is always on), and get "turn clockwise 34° · tilt up
     12°" from where the last frame points. A Setup page holds the site and minimum altitude.
  2. Live finder: the scope node streams star lists from the finder camera, the console
     solves them live, and the aim point (the eyepiece's spot in the finder camera's frame)
     is set by tapping a centred star.
  3. GoTo: no star alignment; every solve is a sync, a pointing model is fitted from them,
     and the console drives the 16-inch Dob's two axes in a closed loop to within 1′.
  4. Tracking: pointing-model rates corrected by solves every 1–5 s, with the field rotation
     and longest exposure reported for an imaging camera.
- MCP (016): an AI assistant gets the console's actions as tools, through the same guards.
- Imaging (030–043): an imaging camera through INDI; every frame saved as FITS, with ASTAP
  solving the subs and any frame asked about; a focus aid, darks and flats, a live stack placed
  sub by sub from each sub's own solve, tracking steered from the subs, and one-button imaging
  runs from GoTo to a saved stack. In night mode the imaging pictures (the frame preview and
  the stack) are red-only with a hold-to-see-colour control, the one exception to the Red Torch
  Rule.
- A simulated scope node (`finder --simulate`) stands in for the telescope at the desk and in
  CI.
- Still out: typed RA/Dec, named stars, the Moon and planets as targets; a horizon that varies
  by direction; a field derotator (Track reports the longest exposure instead); SkySafari,
  voice mode, the mobile app.

## Brand Commitments

None yet. The repository is `plate-solver`; the app is the finder console.

## Evidence on Hand

- 21 cedar sample frames in `reference-solutions/cedar-solve/examples/data/medium_fov/`;
  20 solve at the default threshold, as they do in cedar; `roof.jpg` solves only at 5σ.
- The benchmark against cedar in `docs/solver.md`, given as ratios from one machine.
- Real-sky results from one rig, 12 to 14 Nov 2026, in `docs/field/2026-11-13-first-light.md`:
  live solving, push-to by hand and eight small powered moves. Track was used on a later night,
  20 Nov, for about 13 minutes at 14–24″ RMS (`docs/field/2026-11-20-driveway-track.md`). The
  imaging camera has run on a bench (`docs/hardware.md`). A real night frame from the finder
  camera is in `fixtures/scope-node/`. Never imply hardware results beyond the field reports
  and the hardware page: no field GoTo to a target is recorded yet, and imaging runs have run
  only in the simulator.

## Product Principles

1. Prove, don't claim: every answer shows its evidence, the matched stars on the frame.
2. Night is a first-class mode: nothing on screen may break dark adaptation.
3. Glanceable first, detail on demand.
4. Every view is a client of the same pointing state that SkySafari, the mount and the AI
   will use.

## Accessibility & Inclusion

- Night mode is red-only and dim, with no white flashes, images and embedded views included.
- Large touch targets for one-handed use in the cold (inferred).
- Day mode meets WCAG AA contrast; night mode trades contrast for dark adaptation (inferred).
