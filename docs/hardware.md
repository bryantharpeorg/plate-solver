# Known-good configurations

This page lists the hardware plate-solver has actually run on, what each part was shown to do,
and where it falls short. Status words are strict:

- **Field-proven**: recorded working at the telescope under the sky.
- **Bench-proven**: recorded working on real hardware indoors.
- **Simulated**: covered by the simulator and the test suite only.
- **Planned**: chosen but not yet run.

To buy parts for a build, start from the [parts lists](parts-list.md).

## At a glance

| Configuration | Status | Use it for |
|---|---|---|
| [Budget rig: Raspberry Pi 3B+ and an Arducam OV5647](#budget-rig-raspberry-pi-3b-and-arducam-ov5647) | Supported. The finder, push-to and mount link are field-proven; the imaging camera is bench-proven, with limits. | Finder, push-to, GoTo and Track. Imaging works, but slowly. |
| [Raspberry Pi 5 rig](#raspberry-pi-5-rig-planned) | Planned. Parts on order, not run yet. | Everything, with the console on the telescope and full frames kept local. |
| [No hardware: `finder --simulate`](guide/quick-start.md#2-the-whole-system-simulated) | Simulated. Runs in CI on every pull request. | Trying the console, development, testing. |

## Budget rig: Raspberry Pi 3B+ and Arducam OV5647

This is the configuration every field night so far has used. It is supported: the finder and
push-to are field-proven on it, the mount link and its moves are field-proven, and Track has
been used in the field. No field GoTo is on record yet. The
[first-light report](field/2026-11-13-first-light.md) records the first three nights, and the
[20 November report](field/2026-11-20-driveway-track.md) the first Track. The imaging camera
works on it too, but USB 2 and the Pi 3B+'s Wi-Fi make full-size imaging frames slow. See
[limits](#limits-of-the-budget-rig).

| Part | What was used | How it connects | Configured by | Status |
|---|---|---|---|---|
| Scope node | Raspberry Pi 3 Model B+, 64-bit Debian 13 (trixie) / Raspberry Pi OS Lite | Rides on the telescope; micro-USB power | the `scope-node` static aarch64 binary | Field-proven |
| Finder camera | Arducam B0032 (OmniVision OV5647), CS mount | CSI ribbon; `rpicam-vid` | `adapter = "rpicam"`, `mode = "1296x972"` | Field-proven |
| Finder lens | Arducam 16 mm f/1.4 C-mount, on a C-to-CS ring | — | — | Field-proven: measured 13.0° × 9.75°, 36″ per pixel |
| Mount | Sky-Watcher 400P (16-inch) collapsible GoTo Dobsonian, motor firmware 2.09 | USB serial adapter (Prolific chip) into the motor box's 6-pin RJ-12 hand-control jack. No hand controller. | `[mount] device = "/dev/serial/by-id/…"` | Connect and 1° moves field-proven; Track used in the field; no GoTo recorded yet |
| Mount power | 12 V 5 A lithium power pack | The mount's 12 V input | — | Used in the field |
| Imaging camera | ZWO ASI071MC Pro (4944 × 3284, colour RGGB, cooled) | USB into the Pi (USB 2 on a 3B+). The cooler needs its own 12 V. | `[imaging] adapter = "indi"`, with `indiserver` + `indi_asi_ccd` | The node's probe passed on the Pi; the camera's other checks passed on a desktop bench; see limits |
| Console | `finder` on a Windows-on-ARM laptop under WSL2 | Wi-Fi to the scope node | `finder` flags | Field-proven |
| Pattern database | cedar's default database (10–30° fields), shipped in `reference-solutions/` | — | `--db` (the default) | Field-proven with the 13° finder field |
| Imaging solver | ASTAP's `astap_cli` with its D50 star database | Run by the console | `--astap`, `--astap-db-name d50`, and `--astap-db /opt/astap` for the Ubuntu packages | Tested on recorded ASTAP output; no field solve on record yet |

The stock lens that comes with the OV5647 board (about 3.9 mm) is **not** supported. Its field,
about 50° × 38°, is far outside the default database's 10–30°.

### What it has been shown to do

- **Finder solving, city sky.** At gain 8 and f/1.4, 1 s exposures solved 6 of 6 frames with
  14–20 matched stars. 0.5 s solved 5 of 6, and 12 of 12 with the detection threshold lowered
  from 8σ to 6σ. 0.25 s solves at most 2 of 6, at any threshold.
- **Finder solving, dark site.** 3 s exposures at gain 16 solved every frame. 1 s frames failed
  there: the sky background was so low that the noise estimate collapsed.
- **Star detection on the Pi.** Detection takes about 3% of each update at a 1 s exposure. The
  exposure is the other 97%.
- **Mount.** The node read the board and drove both axes. Eight test moves of up to 1°, slow
  and fast, on both axes, each stopped by itself within 26 encoder counts of its target, about
  17″ ([the moves](field/2026-11-13-first-light.md#what-worked)).
- **Finder solve rate.** On 20 November, 500 ms exposures solved about two a second, as fast
  as the exposures came.
- **Tracking.** On 20 November, Track ran for about 13 minutes, and the console's tracking error
  read 14–24″ RMS throughout ([the report](field/2026-11-20-driveway-track.md)).
- **Imaging camera.** On the Pi, the node's camera probe described the camera and received full
  frames through INDI, three of three, twice. Exposure, darks, cooling (with 12 V), colour order
  and the probe also passed on a desktop bench, through the same INDI driver.

### Limits of the budget rig

| Limit | Effect | What helps |
|---|---|---|
| **The OV5647's binned mode caps exposure at about 3 s** | Enough for the finder, but there's no headroom at a dark site. | Use 1 s in town and 3 s at gain 16 at a dark site. Lower `sigma` to 6 for short exposures. |
| **The node's default camera mode is 640 × 480** | Nearly the full field, but at a quarter of the resolution. | Set `mode = "1296x972"`, the mode every field night has used. |
| **Auto exposure stops at 200 ms** | Too short at night. | Set exposure and gain in the console's camera settings. The console applies them every time it connects. |
| **USB 2 for the imaging camera** | A full ASI071 frame is about 31 MiB and takes about 3.5 s to reach the node after the exposure ends. | Bin 2 × 2 (a quarter of the data), or use a region of interest. |
| **Every imaging frame is relayed whole over Wi-Fi** | Each full frame also has to cross the Pi 3B+'s Wi-Fi to the console. | Bin, or pace the relay with the forwarding limit. "Also keep each imaging frame at the scope node" saves a copy but still relays. |
| **Saving at the node writes to the microSD card** | Each save adds time and wears the card. | Leave "Also keep each imaging frame at the scope node" off on this rig. |
| **Power** | A weak micro-USB cable undervolts the Pi. Plugging the ASI071 into the Pi caused a short undervoltage. | Use a good 5 V supply and cable. Connect the camera's 12 V before its USB. |
| **microSD cards** | One card's controller failed. Power at the telescope is the suspected cause. | Keep a spare, flashed card. The Pi 5 rig boots from an SSD instead. |
| **On-board Wi-Fi range** | From a driveway the Pi 3B+ could not reach a house network that a phone beside it could ([20 November](field/2026-11-20-driveway-track.md#problems-found-and-what-has-happened-since)). | Use a phone hotspot, or a closer access point. A link drop stops the mount after one second, by design. |
| **No stacking on the node** | By design ([ADR-0002](adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md)): the Pi 3B+ has under 1 GB of memory. | Stacking runs on the console. |

The imaging camera on this rig is best kept to binned frames and modest sub counts. Full-frame
runs work, but each sub waits on USB 2 and then on the Wi-Fi relay.

## Raspberry Pi 5 rig (planned)

The next rig replaces the Pi 3B+ and puts the console on the telescope as well. Nothing here
has run yet. The [architecture page](architecture.md#the-raspberry-pi-5-plan-tentative) shows
how the pieces fit.

| Part | Choice | Why |
|---|---|---|
| Computer | Raspberry Pi 5 (4 or 8 GB, 8 recommended), official 27 W supply and active cooler | USB 3 for the imaging camera, and the memory to run the console and stack |
| Power and storage | A UPS HAT with an M.2 NVMe slot and one 21700 cell (Geekworm X1208), with the Pi booting from an NVMe SSD | Ends microSD failures; rides through bumped power; reports power loss |
| Case and Wi-Fi | A metal case, with a USB Wi-Fi adapter because the case weakens the on-board antenna | Range at the telescope |
| Finder camera | ZWO ASI120MM Mini (mono, 1280 × 960, USB 2) with the same 16 mm C-mount lens | About 17° × 13° at about 48″ per pixel, near the middle of the default 10–30° database. No exposure cap of a few seconds. |
| Imaging camera | The same ASI071MC Pro on USB 3. The cable used so far linked at USB 2, so it needs a USB 3 cable. | Frame download expected to fall from seconds to a fraction of a second |
| Console | `finder` on the Pi, on localhost, reached over an SSH tunnel | Full imaging frames stay on the Pi; only previews cross the Wi-Fi |

**Expected, not measured:**

- Frame saves on the NVMe drive should take a small fraction of the time they take on a
  microSD card.
- Imaging frames should download over USB 3 in a fraction of a second.
- The UPS HAT should end the undervoltage seen when the camera plugs in.
- Finder solve rates should stay camera-limited, at about two a second for 500 ms exposures.
- Tracking error should not change, since it isn't set by the computer.

**Not built yet:**

- The ASI120MM Mini cannot be the finder camera until the scope node has a finder adapter
  for INDI cameras (planned as spec 045). Until then a Pi 5 can run the existing OV5647 on a
  Pi 5 camera cable, through `rpicam`, with one catch:
  - The Pi 5 pads each frame row to a multiple of 128 bytes; the node expects the Pi 3B+'s 64.
    A capture from a Pi 5's camera (`fixtures/scope-node/pi5/`) shows that at 1296 × 972 the
    node would stall with nothing logged.
  - Spec 046 has the node learn the stride from the camera listing. It is specified, not built.
  - Widths that are already a multiple of 128, such as 640, are not affected.
- A power-loss service that presses Stop and then shuts the Pi down.
- Install and deploy tooling for the Pi in this repository.

## What should work beyond these parts

These are reasoned from the code, not tested.

**Mounts**

- **Should work:** other Sky-Watcher alt-az GoTo mounts on the same motor-controller
  protocol, such as other collapsible and SynScan GoTo Dobsonians.
  - The node tries both 9600 and 115200 baud.
  - It reads each axis's counts per revolution, timer and high-speed ratio from the board, so nothing
    is fixed to the 400P.
  - It warns about firmware older than 3.39, whose slow rates may not be smooth.
- **Not supported:**
  - equatorial mounts, because the pointing model is alt-az;
  - the mount's own Wi-Fi, because the node opens serial devices only;
  - any "go to RA/Dec" mount interface, by design.
- **Cable warning:** a Sky-Watcher Dob's motor box takes a 6-pin RJ-12 hand-control plug. An
  RJ-45 "EQDIR" cable made for equatorial mounts does not fit. Never plug it into the RJ-45 jack
  marked "Az Motor".

**Imaging cameras**

- **Should work:**
  - other ZWO cameras on `indi_asi_ccd`;
  - any INDI camera driver that delivers FITS. The node reads gain, offset, binning, exposure
    range and cooler from the camera at run time, and nothing is keyed to a model.
- **Mono cameras** have run only against INDI's CCD simulator.
- **A camera driver that sends anything other than FITS** is refused.

**Finder cameras**

- **Should work:** other libcamera CSI sensors with YUV420 modes, whose field with your lens
  is inside your pattern database's range.
  - For a narrower field (a 35 mm or 50 mm lens), build a matching database with
    `database-generation` (see [the solver](solver.md#build-a-pattern-database)).
- **Not yet:** USB or INDI finder cameras (spec 045).

**Computers**

- **Scope node:** proven on a Pi 3B+ (64-bit OS). It needs `rpicam-apps` for a CSI finder and
  an INDI server for the imaging camera. It has not run on a Pi 5 yet, and needs spec 046 there
  for full-size CSI frames.
- **Console:** proven on Linux under WSL2, and CI runs it on x86-64 Linux. It has been built
  for 64-bit ARM Linux, but has not yet run on a Pi.
