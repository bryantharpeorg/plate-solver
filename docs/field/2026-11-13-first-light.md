# First light: live solving, push-to and the first powered moves

12 to 14 November 2026, the reference rig's first nights under the sky. This is the record the
README's "Proven on the telescope" list rests on. It leaves out the observing site, network
addresses and the pairing token.

## The rig

| Part | What was used |
|---|---|
| Telescope and mount | Sky-Watcher 400P (16-inch) SynScan GoTo Dobsonian. Motor controller firmware 2.09, board 0x99, 9600 baud, 1,980,000 counts per revolution on both axes (5,500 counts per degree). |
| Scope node | Raspberry Pi 3B, Raspberry Pi OS Lite 64-bit, the static aarch64 `scope-node` built from `main` at `ce80f67`. A Prolific USB-serial adapter to the mount's motor controller. |
| Finder camera | Arducam OV5647 with a 16 mm lens at about f/1.5. Mode 1296 × 972, field 13.0° × 9.8°, 36″ a pixel. |
| Console | `finder` on a Surface laptop under WSL2, with the page open on a phone and the laptop. |
| Pattern database | cedar's bundled 10–30° database, which covers this 13° field. |

## What worked

- **12 November, in town:** the console paired with the scope node. After refocusing the lens,
  the first real-sky solves came in at a 13.0° field and 17 to 20″ RMS.
- **13 November, at a dark site:** live solving with a frame every 3 s. Push-to by hand with
  the page's live guidance found the Double Cluster, the Pleiades, M42 and NGC 253 (low, never
  above about 32° from that site). The aim point, the eyepiece's spot in the finder camera's
  frame, was set on Diphda three times, on Alnilam and on M42; the star settings landed within
  about 7 pixels of one another. The pointing model had 70 syncs by the end of the night.
- **14 November, 00:06 to 00:16, the first powered moves:** eight single-axis position moves,
  each sent as one `goto_axis_position` call to the console's MCP endpoint, each on the owner's
  go, with the owner at the mount's power switch.

| Move | Commanded (counts) | Moved (counts) | On the sky, by plate solve |
|---|---|---|---|
| Altitude up, slow | 440 | 422 | up 4.2′ |
| Altitude down, slow | −422 | −435 | down 4.5′ |
| Altitude up, fast | 5,500 | 5,498 | up 59′ |
| Altitude down, fast | −5,498 | −5,503 | down 58.6′ |
| Azimuth clockwise, slow | 440 | 456 | 4.4 to 5.0′ of azimuth |
| Azimuth anticlockwise, slow | −456 | −441 | 3.9′ of azimuth |
| Azimuth clockwise, fast | 5,500 | 5,517 | 58.6′ of azimuth |
| Azimuth anticlockwise, fast | −5,500 | −5,474 | 57.9′ of azimuth |

Every move ended by itself within 26 counts (about 17″) of its target, and the other axis never
moved. Positive altitude is up and positive azimuth is clockwise seen from above, as intended.
On the 1° moves the sky moved 1 to 2′ less than commanded: backlash or a small scale error, not
separated that night.

## Not tested

Stop on a moving axis, timed rate moves, Track, GoTo to a target, long slews, a garbled mount
reply in the middle of a move, and anything with an imaging camera. Powered GoTo and Track were
held back on purpose until Stop could be confirmed by the mount (spec 017).

## Settings this sky needed

Exposure 3000 ms, gain 16, detection at 8σ. A 1 s exposure failed: the background sat near 0
of 255, the noise estimate collapsed, about 200 false stars were detected and nothing solved.

## Problems found, and what has happened since

| Found | Since |
|---|---|
| Stop was not confirmed by the mount, so powered GoTo and Track stayed off. | Spec 017 (a Stop the console can confirm) landed on 14 Nov. Not yet re-tested on the mount. |
| The camera sits about 79° rotated in its holder, so the chart was sideways to the sky. | Spec 021 (chart orientation) and spec 022 (alt-az guidance in the observer's own directions) landed. |
| Matched-star rings blinked between solid and dashed from frame to frame. | Spec 020 (a calm live chart) landed. |
| After the mount was switched off and on, the scope node did not set up the motor board again. | Spec 024 (mount link recovery) landed. Not yet re-tested on the mount. |
| The Pi logged an invalid motor-controller reply every few minutes. | Specs 024 and 039 changed how replies are retried and read. Not yet re-tested on the mount. |
| Under a dark sky, short exposures detect noise as stars; saturated bright stars fail the detector's peak test, so the aim-point tap cannot snap to them. | Open. The workaround is the longer exposure above, and setting the aim point from the solved position. |
| The first SD card died after a hard power cut. | A practice, not a fix: shut the Pi down before cutting its power. |
| The mount's encoders looked to be on, which the pointing model does not expect. | Unconfirmed. |
