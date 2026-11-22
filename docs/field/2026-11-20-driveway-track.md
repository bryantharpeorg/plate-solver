# The first Track, from a driveway in town

20 November 2026, an evening test of the reference rig on a sloped driveway in town. This is the
record behind the README's line that Track has been used at the telescope. It leaves out the
site, network names and addresses, and the pairing token.

Most of the night's own log was kept on the console's laptop and is not in this repository yet.
This report rests on two sources:

- the figures the owner noted from the console that night;
- the scope node's state, read over SSH before the rig moved outside.

What the laptop's log would add is listed under [not on record](#not-on-record).

## The rig

| Part | What was used |
|---|---|
| Telescope and mount | The same Sky-Watcher 400P and motor controller as at [first light](2026-11-13-first-light.md#the-rig). Powered from a 12 V 5 A lithium power pack, with a Prolific USB-serial adapter in the motor box's hand-control jack. |
| Scope node | Raspberry Pi 3B+, Raspberry Pi OS Lite 64-bit, on a new microSD card flashed that evening after the previous card failed. The static aarch64 `scope-node` built from `main` at `fc161a54`, run by systemd, with `indiserver` beside it. |
| Finder camera | Arducam OV5647 with the 16 mm lens, as at first light, through `rpicam`. |
| Imaging camera | ZWO ASI071MC Pro through INDI (`indi_asi_ccd`), on the Pi's USB 2. Its cooler ran from a separate 12 V 2.1 A supply. |
| Console | `finder` on the Windows-on-ARM laptop under WSL2. The test kit called for `main` at `2ae73c66` or later; the commit actually run wasn't noted. |
| Network | A phone hotspot, joined by both the Pi and the laptop. |

The driveway slopes: a level on the base read about one bubble length off.

## What worked

- **Finder solving.** With 500 ms exposures, live solves came in at about two a second. The
  exposure set that pace, not the computer.
- **Track.** Track ran for about 12.8 minutes. Throughout, the console's tracking error (the RMS
  over the last minute) read between 14″ and 24″.
- **Imaging frames.** The node took frames from the ASI071 and relayed them to the console.
  - A frame took about 1 s to download over USB 2.
  - Saving an imaging frame at the node, to the microSD card, took about 1.5 s.
  - The node's camera probe, run indoors earlier that evening, measured 3.4 to 3.6 s per full
    4944 × 3284 frame. The frame size behind the 1 s figure wasn't noted, so the two may not
    compare.

## Problems found, and what has happened since

| Found | Since |
|---|---|
| On the driveway the Pi 3B+'s on-board Wi-Fi could not join the house network, though a phone and a tablet beside it could. | A phone hotspot was used instead. The Pi was off the network, so the hotspot was added by putting its card in the laptop and leaving a one-time setup step on the boot partition. The Pi 5 rig adds a USB Wi-Fi adapter ([hardware](../hardware.md#raspberry-pi-5-rig-planned)). |
| The hotspot link dropped at times. | Open. A drop of over a second stops the mount, by design. The Pi 5 plan moves the console onto the Pi, so that link no longer crosses Wi-Fi ([architecture](../architecture.md#the-raspberry-pi-5-plan-tentative)). |
| One under-voltage of about 6 s when the ASI071 was plugged in, in the same minute as the camera's first USB reset. `vcgencmd get_throttled` read `0x50000`: it had happened, and was over. | The Pi went onto a new 5.1 V supply later that night; no reading on it is recorded. Connect the camera's 12 V before its USB. The Pi 5 rig's UPS HAT is expected to end it. |
| The ASI071 reset on the USB bus each time the INDI driver started. | Read as the driver opening the camera, not a fault. No failure was traced to them. |
| The previous microSD card failed earlier that day. | Replaced the same evening. Keep a spare, flashed card. The Pi 5 rig boots from an NVMe SSD instead. |
| Imaging frames are slow on this rig: about 1 s to download over USB 2 and 1.5 s to save to the card. | The Pi 5 rig's USB 3 and NVMe drive are expected to cut both. Not yet measured. |

## Not on record

Some of these may have been tried that night, but none is recorded here:

- the target Track held, and where the GoTo that started it landed (Track starts with a GoTo
  when the target is more than 1′ away);
- why Track ended;
- the finder camera's gain and detection threshold;
- how often the hotspot dropped, and whether a drop stopped the mount;
- the base tilt the pointing model fitted;
- following a push by hand, and **Recenter**;
- Track for the 30 minutes the [first-night checklist](../guide/first-night.md#the-mount) asks
  for;
- any imaging solve, focus reading, calibration frame, stack or imaging run.
