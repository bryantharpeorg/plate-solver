# Tracking session captures (015 US8)

`session-track.json` stores API responses as a shared base and top-level deltas,
formatted with Prettier. `session-track.ts` restores every captured field with a
shallow merge. The recorder asserts this reconstruction equals each original
response exactly; no values are dropped or altered.
`record-tracking.rs` gives the reproduction command and uses the existing
simulated mount, catalog rendering, solver, router and paused-clock harness.
No page code produces these values. The clock starts at 2026-01-16 02:00 UTC.

- `armed`: Track accepted before M42's GoTo finishes.
- `no_camera`: M42 tracking after 75 seconds of live solves, without an imaging camera.
- `tracking`: the same instant after saving the 23.5 × 15.6 mm, 3.76 µm camera.
  The readout is RMS 5.076255008384328″, rotation 0.24959114802233465°/min,
  exposure 3.6720824859386423 s. Independent decimal rounding gives **5″**,
  **0.25°/min**, **3.7 s**; tests compare these literal strings exactly.
- `no_limit`: a pole site makes rotation effectively zero. The API returns
  `"no limit"`; the page does not calculate an exposure.
- `keyhole_wait`: restart Track with M42 one degree from the site's zenith.
- `too_low`: restart with M42 descending just above a minimum altitude set
  0.0001° below its current altitude. The tracker ends automatically as it sinks.

Stop tests reuse these recorded states with `mount.moving` held false and the
preceding GoTo removed, isolating the interval before autonomous mount motion.
The camera integration test serves the two captures on either side of its PUT.
