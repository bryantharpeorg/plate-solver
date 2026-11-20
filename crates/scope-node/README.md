# Scope node

The scope node captures 8-bit gray frames through a camera port and finds stars with
`star-detection`, using the finder console's cedar settings. It links `scope-link`
and `star-detection`; the solver and pattern database are test-only dependencies.
The system clock reads `CLOCK_MONOTONIC` through safe `rustix` calls, sharing the
sensor timestamp's epoch. `rustix` also creates the private metadata FIFO; `toml`
parses the strict config, and `serde_json` reads incremental camera metadata.

The operator's [camera bring-up notes and saved captures](../../fixtures/scope-node/README.md)
record modes, row stride, timestamps, detection timing and network throughput. Those
files are independent test oracles and must stay unchanged by this crate.

## Static build from WSL

Install an AArch64 musl cross toolchain providing `aarch64-linux-musl-gcc`, plus
`protoc`, then install the Rust target and build with the target and flags explicitly
on the command line:

```sh
rustup target add aarch64-unknown-linux-musl
CC_aarch64_unknown_linux_musl=aarch64-linux-musl-gcc \
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-musl-gcc \
RUSTFLAGS='-C target-feature=+crt-static' \
cargo build --release --locked -p scope-node --target aarch64-unknown-linux-musl
file target/aarch64-unknown-linux-musl/release/scope-node
```

The binary should be statically linked. Install it at `/usr/local/bin/scope-node`.
On the scope node, install Raspberry Pi OS's `rpicam-apps` and ensure `rpicam-hello`
and `rpicam-vid` are on the service's PATH. No repository `.cargo/config.toml` is
needed: keep cross-compilation settings on the command line.

## Config and systemd

Create a dedicated system user and add it to the camera's `video` group:

```sh
sudo useradd --system --no-create-home --groups video scope-node
sudo install -d -o scope-node -g scope-node -m 0700 /etc/scope-node
sudo install -o scope-node -g scope-node -m 0600 config.toml /etc/scope-node/config.toml
```

The config contains the pairing token and must have mode **0600**. Use a token of your
own for an observing session. This example's low-entropy token is only for testing:

```toml
listen = "0.0.0.0:50051"
token = "test-token"
adapter = "rpicam" # or "simulated": seeded stars, no catalog or camera required
mode = "auto" # or a WIDTHxHEIGHT sensor mode from rpicam-hello --list-cameras
exposure_ms = "auto" # or a positive number of milliseconds
gain = "auto" # or a positive number

[detection]
sigma = 8.0
binning = 2
normalize_rows = true
detect_hot_pixels = true
```

Check before starting:

```sh
sudo -u scope-node /usr/local/bin/scope-node --config /etc/scope-node/config.toml --check-config
```

This prints every setting with the token replaced by `****`, then exits. Unknown
keys and invalid values are errors naming the offending key. Fixed camera controls
outside the camera's reported ranges are refused rather than clamped.

Install `/etc/systemd/system/scope-node.service`:

```ini
[Unit]
Description=Scope node camera capture
After=network.target

[Service]
Type=simple
User=scope-node
Group=scope-node
SupplementaryGroups=video
ExecStart=/usr/local/bin/scope-node --config /etc/scope-node/config.toml
Restart=always
RestartSec=1
UMask=0077

[Install]
WantedBy=multi-user.target
```

Run `sudo systemctl daemon-reload` and `sudo systemctl enable --now scope-node`.
A node with a `[mount]` table also needs the serial device's `dialout` group: add it to
`--groups` and to `SupplementaryGroups`.
Camera subprocess failures are reported with their stderr; capture retries after one
second inside the same service process. Settings changes reopen the rpicam subprocess.
Discovery uses `rpicam-hello --list-cameras -v` for modes and control ranges. Capture
uses the first listed mode unless `mode` names another; a size the camera does not list
is refused. It runs at up to 5 frames/s (slower for a longer shutter).
This story supplies capture and config; scope-link serving is added by US4.

## The mount link

With `[mount]` configured, the scope node starts and serves the camera even when
the mount is off or its serial device is missing. It opens the device and tries the
connect sequence in the background, waiting 2 seconds after each failed attempt.
Switch the mount on when ready; it connects automatically. A failure is logged as
`mount not connected: <error>`, once per changed error until a successful connect.

While disconnected, the mount describes no axes, axis reads and moves return
`mount not connected`, and star frames carry no axis counts. Before the link's first
connect, Stop answers unconfirmed at once and sends nothing; the watchdog also
sends nothing.

Each successful connect logs one line with the board's firmware, baud and each
axis's parameters, for example:

```text
mount connected: 3.40 (board 0x90) at 9600 baud; azimuth 1620000 counts/rev, timer 16000000 Hz, high-speed ratio 16, auxiliary encoders, :W sent; altitude 1620000 counts/rev, timer 16000000 Hz, high-speed ratio 16, auxiliary encoders, :W sent
```

An axis without auxiliary-encoder support reads `no auxiliary encoders, no :W`.
`:W sent` means the connect sequence disabled that axis's auxiliary encoders.
Any existing firmware warnings follow as `mount warning: <text>`.

The link recognises a power cycle when an axis's initialisation bit clears after
having read set, or when counts jump farther than 1.5 times the axis's top rate
over the elapsed time plus 1,000 counts (with the count difference wrapped at 24
bits). Three failed state reads in a row across the two axes lose the link and
log `mount link lost: 3 failed reads in a row`; any good read clears that streak.
The node then connects again automatically.

A detected reset runs the connect sequence again, including `:F3` and disabling
supported auxiliary encoders with `:W`, before accepting further moves. GoTo and
rate commands send nothing while connecting or initialising again. Stop and the
watchdog still reach both axes over the last successful connect's stream, making
any connect attempt under way give the device up first. While disconnected, Stop
sends `:K` on both axes before `:L` on either unacknowledged axis, once per command;
it confirms only when both axes acknowledge.

Each successful re-initialisation counts one reset, even when the same read found
both a cleared bit and a count jump. Reconnecting after link loss also counts one;
the first connect after node startup does not. Every axis state reports the node
process's total as `mount_resets`. After the connect line, the node logs:

```text
mount reset (<cause>): re-initialised <firmware> at <baud> baud
```

The cause is `not initialised`, `counts jumped on Azimuth` (or `Altitude`), or
`link lost`. A failed re-initialisation logs
`mount reset (<cause>): re-initialisation failed: <error>` and retries in the
background, keeping the cause to count once on success. A console restarted after
a power cycle needs a sync before GoTo.

Status (`:f`) and position (`:j`) reads share a budget of five transmissions for
timeouts, malformed frames and framed replies of the wrong length. Each
wrong-length framed reply is counted in `MountLink::health().invalid_replies`,
kept escaped as `last_invalid`, and logged with its command and bytes:

```text
mount reply invalid for :f1: =10\r
```

GoTo records the accepted path per axis in `health().goto_path`: `:S` for an
absolute position, or `:H/:M` for the relative fallback. A changed path logs once
per axis on each board, including its first GoTo:

```text
mount goto path on axis 1: :S
```

Health counts persist across reconnects and resets; the latest invalid bytes and
each axis's previous path remain until a new board reports replacements. A motion
start (`:J`) is always sent once, even when its acknowledgement is lost.

Before every transmission, including retries and Stop, the node reads out the
reply still owed by an exchange cut short by a timeout, an abandoned read or Stop.
It waits only for what remains of that exchange's own 300 ms, then discards input
that has already arrived without waiting. Stop pays the same bounded wait before
sending its first command. A retry behind an exchange whose read-out found no reply
stays open after reading a line, for one more reply to the retry's own 300 ms
deadline (FR-016). After a reply lost outright, whatever is sent next therefore
waits until 300 ms after the answered retry was sent, even though no second reply
will arrive. This extra wait is paid once; it does not keep the next exchange open.
Transmissions preceded by stale input are counted in
`MountLink::line_health().stale_replies`, summed across reconnects; `last_stale`
keeps the latest bytes in order, escaped, until replaced. Each event logs the
command about to be sent and those bytes:

```text
mount stale input before :j1: =111\r
```

## The imaging camera

An `[imaging]` table adds a deep-sky camera beside the finder camera, for the imaging
console's exposures. The camera is reached through an INDI server, which the owner
installs and starts on the scope node: INDI's own `indiserver` and the camera's device
program (for a ZWO camera, `indi_asi_ccd`; INDI's camera simulator for testing,
`indi_simulator_ccd`). INDI core and the device programs are under LGPL-2.1-or-later;
this program is their client over a TCP socket and links none of them.

```toml
[imaging]
adapter = "indi"            # or "simulated": a small stand-in camera, no INDI needed
address = "127.0.0.1:7624"  # the INDI server; the default
device = "ZWO CCD ASI071MC Pro"  # optional
neutral = { "CCD_CONTROLS.WB_R" = 50, "CCD_CONTROLS.WB_B" = 50 }
save_dir = "/var/lib/scope-node/frames"  # keep a copy of every frame here
```

The adapter connects when the node is first asked about the camera, so the node starts
whether or not the INDI server is up. It asks the server for its properties and chooses
the device the table names or, with none named, the first whose `DRIVER_INTERFACE` has
the camera bit; connects it; and switches compression off, the transfer format to FITS
and upload to the client, for each the device defines. The camera's own ranges — gain,
offset, binning, exposure, temperature — are read from the camera at run time; nothing
is keyed to a camera model. The adapter sets the camera's settings on each connection,
so a power-cycled camera is set again when it is next reached. After the table changes,
the node must be rebuilt and restarted, because its binary builds the adapter the table
names.

`neutral` is how a camera's white balance is made neutral. On a camera whose
white-balance controls scale the raw samples, the table names them with their neutral
values — this example's camera's `CCD_CONTROLS.WB_R` and `WB_B` at 50 and 50 — and the
adapter sets them on every connection, one vector a property, every element of the
property at the value the device last reported. Every control neither the table nor a
settings change names stays as the device reports it, and no control is recorded with a
frame. A name the camera does not define fails the connection with
`the camera has no control <property>.<element>`, and a camera that refuses a vector
fails it with its own message; the console shows the camera's error either way.

An exposure that fails — the camera reports `Alert`, no frame arrives within the
exposure plus 60 s of the request, or the server closes the connection — is aborted
with `CCD_ABORT_EXPOSURE` where the connection still stands, waited out 1 s, and
tried once more, reconnecting first when the connection was lost, the frame type set
again before the second request. A second failure answers with the reason in
`the exposure failed twice: <reason>`. A BLOB the adapter cannot read — a format other
than `.fits`, or a FITS header or data it cannot parse — fails at once, with no abort
and no second try. The adapter adds only this to the device program's own handling: it
relies on that program starting an exposure up to three times and restarting a failed
exposure once, by itself, and it does not rely on that program's USB reset, which stays
off unless it is switched on.

`save_dir` in the table names the folder of the node's own copy of every frame taken
with `save`: `save_dir = "/var/lib/scope-node/frames"`. The folder is made when it is
missing. Each copy is one FITS file, `frame-<id>.fits` with the node's own count of
frames — `frame-000001.fits`, then `frame-000002.fits` — and a name already on the
disk gets `-2`, `-3` and so on, so a restart never replaces an earlier file. The copy
is written before the frame's header leaves the node, and the header names the file
and any failure to write it. The node never deletes these files.

Before the console is involved, one command on the scope node describes the camera
and times its downloads:

```sh
scope-node --probe-camera --probe-exposure-ms 1000
```

The probe builds the camera the `[imaging]` table names, prints its description and
takes three exposures of `--probe-exposure-ms` (1000 ms by default), each with the
purpose `probe`. For each it prints the frame's size and how long after the end of the
exposure the frame was in hand on the node — the download time the later budgets
assume. It times the camera and the scope node, and nothing past them: the hop from
the node to the console is read at the console, as the time from the end of an exposure
to the last byte in hand there, which less the probe's figure is the time across the
link. A camera error ends the probe with that error. With no `[imaging]` table the
probe exits with `no [imaging] table in the config`, and it cannot be combined with
`--check-config` or `--probe-mount`.

On the stand-in camera the output is:

```text
imaging camera: synthetic imaging camera
serial: none
sensor: 64 x 48 px, 4.78 um pixels, 16 bits, RGGB
exposure: 1 to 60000 ms
gain: 0 to 300
offset: 0 to 100
binnings: 1, 2
cooler: yes
exposure 1: 64 x 48, 6144 bytes, in hand 1 ms after the end of the exposure
exposure 2: 64 x 48, 6144 bytes, in hand 0 ms after the end of the exposure
exposure 3: 64 x 48, 6144 bytes, in hand 0 ms after the end of the exposure
```

The stand-in's 0 or 1 ms is the timer's step: in real time tokio's timer rounds the
stand-in's wait up to the millisecond, so the sample does not copy the 0 ms a paused
clock measures.
