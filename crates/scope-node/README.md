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
