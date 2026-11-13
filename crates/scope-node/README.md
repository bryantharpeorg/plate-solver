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
