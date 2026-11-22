# First night at the telescope

This is the order of checks for a first real night. Each check depends on the ones before it,
so don't skip ahead. It assumes:

- the [scope node](scope-node.md) is installed and its parts each pass on their own;
- the [console](console.md) runs and is paired with the node.

Keep a hand near the mount's power switch for the first hour. Every powered move passes the
console's guards: the Sun, the minimum altitude (15° by default), cable wind, the mount's range,
and the watchdog that stops the mount after a second without the console. Read
[safety and security](../safety-and-security.md) once before you start, especially what still
leaves the mount moving.

## Before you leave home

- [ ] The scope node starts with its `[mount]` and, if used, `[imaging]` tables, and
      `--probe-camera` describes the imaging camera.
- [ ] The console is built from the commit you mean to use. For imaging, it is started with
      `--astap astap_cli --astap-db-name d50`, plus `--astap-db /opt/astap` for the Ubuntu
      packages.
- [ ] The Pi can join a network at the site: a site access point or a phone hotspot. The Pi's
      on-board Wi-Fi has less range than a phone's.
- [ ] There is enough free disk for imaging: about 32 MB per full ASI071 sub, a quarter of that
      binned 2 × 2.
- [ ] A spare, flashed microSD card is in the bag (on a Pi that boots from one).
- [ ] If an AI assistant will drive, its MCP session is started after the console, so it sees
      the current tools.

## Set up

1. **Level the base.** A tilt up to about 1.2° needs nothing. 1.2–1.7° needs a Second sync before
   long slews. More than about 1.7° should be shimmed. Shim before the first sync; if you shim
   later, press **Reset night's model**.
2. **Power the mount from its 12 V supply.** Connect the imaging camera's 12 V before its USB.
3. **Start the node, open the page, and check Setup:**
   - the site;
   - Scope node and camera shows **Connected**;
   - the **Camera field** line falls inside the **Database range**, with no warning.

## The finder and the model

4. **First solve.** The chart goes live. If frames don't solve, raise the exposure. Short
   exposures in town solve better with the detection threshold at 6σ, and dark-site frames need
   about 3 s.
5. **Aim point.** Centre a bright star in the eyepiece, then tap it on the live preview under
   Setup › **Aim point**. Later an imaging solve measures this for you.
6. **First sync.** It is automatic: with the mount still, a live solve becomes a sync. Setup ›
   **Mount and pointing model** shows "1 sync".

## The mount

7. **Stop on a moving axis, first.** Start a slow move on one axis, from MCP's `move_axis` or a
   short GoTo, and press **Stop**. The axis must stop at once.
8. **Second sync.** Press **Second sync**. The telescope turns at least 90° in azimuth and solves
   again, and the model shows the base tilt.
9. **A short GoTo** of a few degrees. Note where it lands: earlier 1° moves landed 1–2′ short,
   from backlash or a scale error.
10. **Two long slews** to opposite sides of the sky. Watch the cables.
11. **Track** a star near the meridian.
    - Push the tube by hand more than 5′: Track follows and holds the new place.
    - A smaller nudge is corrected back.
    - **Recenter** sends it back to the target.
12. **Track alone for 30 minutes.** The star should stay put and the link connected. If Track
    ends, write down why. An imaging run cannot outlast Track.

## The imaging camera

13. **Cooler.** Start it off, or at a modest setpoint such as 10 °C below the air. Watch for dew.
14. **Frames.** Take one frame at 1 s, then one at the night's exposure. Each one saves and shows
    a preview.
15. **Focus.** Run the focus aid on a bright star, and focus until it reads best.
16. **Solve.** Solve an imaging frame. That measures the scale and the aim point. A newly
    measured aim point takes effect once Track is off.
17. **Calibration (optional).** Take darks at the night's exposure and temperature, then flats.

## The first imaging run

18. **Set the exposure.** Make it at least as long as the finder camera's, so a finder solve
    falls inside every sub. Choose a bright target well above the minimum altitude.
19. **Start the run.** Put 20 in **Subs to stack** and press **Start imaging run**. Watch it go
    through slewing, tracking, the focus check, solving, centring and stacking.
20. **Let it end.** At 20 subs, Track should still be on, and the run's folder should hold
    `run.json`, the stack and every sub it lists.
21. **Stop a run.** Start a second run and press the top bar's **Stop** while it slews. The
    mount must come to rest and stay there, and the run ends `stopped`.

The console reference has the full 18-step version of this run, with what to write down at each
step: [Field checklist: the first imaging run](../../crates/finder/README.md#field-checklist-the-first-imaging-run).

## When something goes wrong

| Symptom | Likely cause | What to do |
|---|---|---|
| Setup says the node is Disconnected | The node is down or unreachable, or the token is wrong | Check `journalctl -u scope-node`, the address and the token. |
| Frames come in but never solve | The exposure is too short, or the field is outside the database | Lengthen the exposure, check Setup's Camera field against the Database range, lower σ to 6. |
| The mount stops by itself mid-move | The link dropped for a second, and the watchdog stopped it | Improve the Wi-Fi. The stop is by design. |
| The page says "The mount did not confirm Stop" | The node didn't answer within two seconds | Press Stop again. If the mount keeps moving, switch off its power. |
| GoTo refuses | A guard: Sun, altitude, no sync yet, cable wind | The refusal text names the guard. |
| The Imaging link is missing | No scope node is paired | Pair the node in Setup. |
| The imaging camera isn't found | It was plugged in after the INDI driver started | Restart `indiserver`, or add the plug-in udev rule ([scope node manual](scope-node.md#4-run-it-as-a-service)). |
| The Pi reboots or the card fails | Power | Use a better 5 V supply, and connect the camera's 12 V before its USB. |
