# Scope-node bring-up fixtures and notes

The operator captured these on 2026-10-31 for spec 013's US2, on the bring-up camera, and
replaced `finder-frame.png` with a night frame on 2026-11-09. The
finder's own camera, a mono IMX296, replaces it later. When it does, the operator swaps
every file here, and no test changes: US2's tests take every camera value from
`fixtures.json`.

They live here, outside `crates/`, because the workspace loads every folder under
`crates/` as a crate (013 plan, trap 11).

## The files

- **`list-cameras.txt`**: `rpicam-hello --list-cameras -v`, as captured. The verbose
  listing repeats the plain one's camera and modes, then lists the camera's controls,
  where `ExposureTime` (µs) and `AnalogueGain` give the ranges the camera port reports:
  130 µs to 3.07 s, and gain 1 to 63.94. rpicam lists the controls for the largest mode.
  It was taken later on 2026-10-31 than the stream, on the same camera.
- **`stream.yuv`**: `rpicam-vid --codec yuv420`, 3 frames of 1296 × 972, as captured.
  1296 × 972 is the smallest mode whose rows are padded. Each row is 1344 bytes, 48 past
  the width, so a frame is 1344 × 972 × 3/2 = 1,959,552 bytes: the Y plane's 972 rows of
  1344 bytes, then U and V, each 486 rows of 672. The padding bytes are all 0.
- **`stream.json`**: that stream's `--metadata-format json` output, as captured. It is
  one JSON array, with one object per frame, in frame order. `rpicam-vid` writes `[`
  when it starts and `]` when it exits, so a reader of a live stream parses it object by
  object. Each object also carries `FrameWallClock`, the frame's wall-clock time in ns.
- **`finder-frame.png`**: a night frame of real stars, 1296 × 972, 8-bit gray, unmodified:
  the Y plane from `rpicam-vid`, padding dropped. It was captured on 2026-11-09 with the
  16 mm lens at about f/1.5, 3 s at gain 8, under a light-polluted sky. `detect_stars`
  with `DetectParams::cedar()`'s settings finds 37 stars, so a test comparing two
  detection paths on it compares two real lists. One saturated object sits near the left
  edge. The frame solves with 30 matched stars, and `fixtures.json` records the solution.
  An earlier version of this file was a daytime frame with 40 synthetic stars added.
- **`fixtures.json`**: what the files hold. It gives the camera's model, size, modes, and
  exposure and gain ranges, the stride alignment, the stream's size, stride, frame count, frame bytes and padding
  values, each frame's `SensorTimestamp` (ns) and `ExposureTime` (µs), and the finder
  frame's size, source and synthetic stars.

The capture arguments were:

```text
rpicam-vid --nopreview --codec yuv420 --mode 1296:972 --width 1296 --height 972
  --frames 3 --shutter 20000 --gain 1 --framerate 5 --denoise off --sharpness 0
  --contrast 1 --saturation 1 --metadata stream.json --metadata-format json -o stream.yuv
```

## Bring-up notes

**The node.**
- Raspberry Pi 3 Model B Plus Rev 1.3, with 905 MB of memory.
- Debian 13 (trixie), 64-bit, with kernel 6.18.50+rpt-rpi-v8.
- `rpicam-apps` v1.13.0, and libcamera v0.7.2+rpt20260817.
- It ran on the camera's default `camera_auto_detect=1`, with no overlay line.

**The camera.**
- The bring-up camera is an Arducam B0032: an OmniVision OV5647, 2592 × 1944, 10-bit
  GBRG, on its stock CS-mount lens.
- Its modes:

  | Mode | Frame rate | Note |
  |---|---|---|
  | 640 × 480 | 62.50 fps | cropped |
  | 1296 × 972 | 46.34 fps | 2 × 2 binned, the full field |
  | 1920 × 1080 | 32.81 fps | cropped |
  | 2592 × 1944 | 15.63 fps | the full field |

- 2592 × 1944 failed to start in 7 of the day's 15 tries. The camera configured, and
  then no frame came within 15–30 s. The kernel logged nothing, and power stayed clean.
  1296 × 972 started 15 times in 15 tries.

**The row stride.**
- It is the width rounded up to a multiple of 64 bytes.
- Measured: 640 → 640, 1296 → 1344, 1920 → 1920, 1000 → 1024 and 1040 → 1088. The last
  two are scaled sizes, not sensor modes.

**The clock.**
- `SensorTimestamp` could not be told apart from either monotonic clock.
- `CLOCK_BOOTTIME` minus `CLOCK_MONOTONIC` was 4.4 µs, because the Pi never suspends. The
  last frame's timestamp was 366.1 ms before either clock's "now".
- The node uses `CLOCK_MONOTONIC`, through std's `Instant`.

**The field of view.**
- About 50° × 38°, from a focal length of about 3.8–3.9 mm.
- It was measured from a framing square 61 in away and 17 in below the lens. Its inch
  marks spanned 44.3 px per inch.
- The square sat at the frame's edge, where the lens's barrel distortion shrinks it. So
  the figure is a few degrees wide either way.
- That is outside the bundled 10–30° database. The finder's own camera, with a 16 mm
  lens, is expected at about 18° × 13°.

**Detection time per frame on the Pi 3B+.** This is `detect-timing`, `star-detection` with
`DetectParams::cedar()`'s settings, as the median of 10 runs:
- the daytime 1296 × 972 frame, with 0 stars: 32.1 ms;
- the same frame with 40 synthetic stars added, 27 found: 32.8 ms;
- a synthetic 1456 × 1088 frame with 60 stars: 41.0 ms, with 59 found.
- `finder-frame.png`, the night frame, with 37 real stars found: 33.0 ms;
- two other night frames at f/1.4, 1 s with 21 stars and 0.5 s with 9 stars: 31.7 ms and
  31.5 ms.

Night frames have only been timed in the 1296 × 972 mode.

**Wi-Fi.**
- The link was TharpeHouse-2.4 at 5200 MHz, 80 MHz wide. The signal was −46 to −52 dBm,
  at 433 Mbit/s each way.
- TCP throughput, 40 MiB each way to a host on the wired LAN:
  - Pi to the host, the way frames travel: 121 Mbit/s;
  - the host to the Pi: 130 Mbit/s.

**Power.** The first micro-USB cable undervolted at boot, with `get_throttled` at
0x50005. A better cable reads 0x0 under capture load.
