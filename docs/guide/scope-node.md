# Scope node manual: a Raspberry Pi at the telescope

The scope node is the `scope-node` program on a small Linux computer that rides on the
telescope. It does three jobs:

- **Finder camera:** captures frames from the finder camera and finds the stars in them.
- **Mount:** drives the mount's two axes over a USB serial cable, and stops them if the
  console goes quiet.
- **Imaging camera:** runs the imaging camera through a local INDI server and relays its frames
  to the console.

This manual takes a Raspberry Pi from a blank card to a running, paired node. The full
reference for every setting and log line is
[`crates/scope-node/README.md`](../../crates/scope-node/README.md).

> **Before you start:** read [safety and security](../safety-and-security.md). In particular,
> stopping the `scope-node` service does not yet stop a moving mount.

## What you need

- **A Raspberry Pi with a 64-bit OS.** The node is proven on a Pi 3 Model B+ running 64-bit
  Raspberry Pi OS Lite (Debian 13, trixie). See [known-good configurations](../hardware.md).
- **A good 5 V supply and cable.** A weak micro-USB cable undervolts a Pi 3B+. Check with
  `vcgencmd get_throttled`, where `throttled=0x0` is good.
- **A finder camera:** a CSI camera module that `rpicam-hello --list-cameras` lists, with a lens
  whose field is inside your pattern database's range (10–30° for the default database).
- **For the mount:** a USB serial adapter for your mount's hand-control jack. On a Sky-Watcher
  GoTo Dobsonian this is the 6-pin RJ-12 jack, not the RJ-45 "Az Motor" jack.
- **For the imaging camera:** INDI and the camera's driver, and the camera's own 12 V supply if
  it is cooled.
- **On a separate build computer:** Rust 1.96 or later and `protoc`.

You can start with none of the hardware: the node has a simulated finder camera and a
simulated imaging camera.

## 1. Prepare the Pi

1. Flash 64-bit Raspberry Pi OS Lite. Turn on SSH, and set a user and an SSH key in the imager.
2. Boot the Pi, log in over SSH and update it:

   ```sh
   sudo apt update && sudo apt full-upgrade
   ```

3. Install the camera tools for a CSI finder camera:

   ```sh
   sudo apt install rpicam-apps   # or rpicam-apps-lite
   rpicam-hello --list-cameras -v
   ```

   The second command must list your camera and its modes.

4. **For an imaging camera**, install INDI and the driver:
   - Enable the `contrib` and `non-free` components in the apt sources: `indi-asi` is in
     contrib and `libasi` in non-free.
   - Install the packages **before you plug the camera in**:

     ```sh
     sudo apt install indi-bin indi-asi libasi
     ```

   - `libasi` installs a udev rule that raises the USB buffer limit when a ZWO camera is
     plugged in. Linux's 16 MB default is smaller than one full 16-bit frame. Check it with
     `cat /sys/module/usbcore/parameters/usbfs_memory_mb`. It should read 64 or more once the
     camera is in.
   - Tested: INDI 1.9.9 and indi-asi 2.2 on the Pi, and libasi 1.27 on a bench.

## 2. Build the binary

Build on another computer and copy the result over. The binary is fully static, so it runs on
any 64-bit Pi OS with no Rust installed. CI does not build this target yet, so build it
yourself.

```sh
sudo apt install protobuf-compiler        # protoc, for the scope-link protocol
rustup target add aarch64-unknown-linux-musl
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld \
  cargo build --release --locked -p scope-node --target aarch64-unknown-linux-musl
file target/aarch64-unknown-linux-musl/release/scope-node   # "statically linked"
```

`rust-lld` comes with Rust, so no C cross-compiler is needed for the node.
[`crates/scope-node/README.md`](../../crates/scope-node/README.md#static-build-from-wsl) shows
the same build linked with `aarch64-linux-musl-gcc`, which also works.

Copy the binary to the Pi and install it:

```sh
scp target/aarch64-unknown-linux-musl/release/scope-node <pi>:/tmp/
ssh <pi> sudo install -m 0755 /tmp/scope-node /usr/local/bin/scope-node
ssh <pi> scope-node --help
```

## 3. Create the user and the config

```sh
sudo useradd --system --no-create-home --groups video,dialout scope-node
sudo install -d -o scope-node -g scope-node -m 0700 /etc/scope-node
sudo install -d -o scope-node -g scope-node -m 0750 /var/lib/scope-node/frames
```

The groups grant access as follows:

- `video`: the CSI camera.
- `dialout`: the mount's serial adapter.

The node reaches the imaging camera only through `indiserver`. The INDI unit below adds
`plugdev`, and `libasi`'s udev rule already opens ZWO cameras to all users.

**Write `/etc/scope-node/config.toml`.** Start with the simulated camera, so you can test the
link before any hardware:

```toml
listen = "0.0.0.0:50051"     # "127.0.0.1:50051" if the console runs on this Pi
token = "<a long random token, e.g. from: openssl rand -hex 32>"
adapter = "simulated"        # then "rpicam" for a CSI finder camera
mode = "auto"                # for an OV5647, use "1296x972"
exposure_ms = "auto"
gain = "auto"

# Add when the mount is cabled:
# [mount]
# device = "/dev/serial/by-id/usb-<maker>_<serial>-if00-port0"

# Add when the imaging camera and indiserver are running:
# [imaging]
# adapter = "indi"           # or "simulated" (then leave device unset)
# device = "<INDI device name, e.g. ZWO CCD ASI071MC Pro>"  # optional: first camera found
# save_dir = "/var/lib/scope-node/frames"                    # optional: keep a copy here
```

Protect it, because it holds the token:

```sh
sudo chown scope-node:scope-node /etc/scope-node/config.toml
sudo chmod 0600 /etc/scope-node/config.toml
sudo -u scope-node scope-node --check-config
```

`--check-config` prints every setting with the token shown as `****`, then exits. The config is
strict: an unknown key or a bad value is an error that names the key.

Some things to know about the settings:

- **Exposure, gain and detection are start values only.** When the console pairs, it sends
  its own camera settings and they replace these. Set the night's exposure and gain in the
  console's **Setup › Scope node and camera**.
- **`mode = "auto"` takes the camera's first listed mode.** On an OV5647 that is 640 × 480:
  nearly the full field, but at a quarter of the resolution. Name `1296x972` instead, the mode
  every field night has used.
- **Auto exposure stops at 200 ms**, which is too short at night. Give a fixed exposure.
- **Use the `/dev/serial/by-id/…` path for the mount**, not `/dev/ttyUSB0`, which can change
  between boots. Find it with `ls -l /dev/serial/by-id/`.

## 4. Run it as a service

**Write `/etc/systemd/system/scope-node.service`:**

```ini
[Unit]
Description=Scope node
After=network.target indiserver.service

[Service]
Type=simple
User=scope-node
Group=scope-node
SupplementaryGroups=video dialout
ExecStart=/usr/local/bin/scope-node --config /etc/scope-node/config.toml
Restart=always
RestartSec=1
UMask=0077

[Install]
WantedBy=multi-user.target
```

**For an imaging camera, also write `/etc/systemd/system/indiserver.service`:**

```ini
[Unit]
Description=INDI server for the imaging camera
After=network.target

[Service]
User=scope-node
Group=scope-node
SupplementaryGroups=video plugdev
ExecStart=/usr/bin/indiserver -p 7624 indi_asi_ccd
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
```

**Optionally, restart INDI when a ZWO camera is plugged in.** The ZWO driver finds cameras only
when it starts, so a camera plugged in later is invisible. This rule restarts INDI on plug-in:

```sh
echo 'ACTION=="add", SUBSYSTEM=="usb", ENV{DEVTYPE}=="usb_device", ATTR{idVendor}=="03c3", RUN+="/usr/bin/systemctl --no-block restart indiserver.service"' \
  | sudo tee /etc/udev/rules.d/99-asi-indiserver-restart.rules
sudo udevadm control --reload
```

With two ZWO cameras on one Pi, plugging one in also drops the other. The node treats that as a
lost connection, reconnects and retries the exposure once.

**Start the services:**

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now indiserver scope-node   # leave out indiserver if unused
journalctl -u scope-node -f
```

A healthy node is silent: it logs only problems and mount events. Errors appear as
`scope-node: <error>`. If you change the config, restart the node with
`sudo systemctl restart scope-node`, after the mount is still.

INDI has no authentication, so keep port 7624 off untrusted networks. See
[network exposure](../safety-and-security.md#network-exposure).

## 5. Check each part, in this order

1. **The link.** With `adapter = "simulated"`, pair the console (step 6). The console's
   **Setup › Scope node and camera** should show Connected. The chart goes live with the
   detected stars and "No match": the node's simulated stars are random and never solve.
2. **The finder camera.**
   - Set `adapter = "rpicam"` and `mode = "1296x972"` (or your camera's mode), restart, and
     watch the log.
   - `camera capture failed:` lines mean the camera isn't working.
   - If the camera is missing altogether, the node exits at start and systemd restarts it in a
     loop. Until it starts, the mount and imaging camera are down too.
3. **The mount.** With the mount at rest, stop the service first, so that only one program
   opens the serial device. Then run a read-only probe:

   ```sh
   sudo systemctl stop scope-node
   sudo -u scope-node scope-node --probe-mount /dev/serial/by-id/usb-<maker>_<serial>-if00-port0
   ```

   - The probe sends read-only commands and prints the board version and baud. For one axis it
     also prints the counts per revolution, timer frequency, high-speed ratio,
     auxiliary-encoder support and position.
   - Add the `[mount]` table, start the service again, and look for one `mount connected: …`
     line.
   - The node starts even with the mount off, and connects when it is switched on.
4. **The imaging camera.** Connect the camera's 12 V first, then its USB. Check
   `systemctl is-active indiserver`, then run:

   ```sh
   sudo -u scope-node scope-node --probe-camera --probe-exposure-ms 1000
   ```

   - It prints the camera's description and takes three exposures, each with the time from
     the end of the exposure to the frame in hand.
   - To test with no camera, stop `indiserver.service`, leave `device` unset, and run INDI's
     simulator instead: `indiserver -p 7624 indi_simulator_ccd`.
   - Restart the node after adding or changing `[imaging]`.
5. **Stop.** On the first night, after the first sync, start a slow move and press **Stop**. The
   mount must halt. The page has no jog control, so use MCP's `move_axis` or a short GoTo. See
   [first night](first-night.md#the-mount), step 7.

## 6. Pair the console

In the console, open **Setup › Scope node and camera**:

1. Enter the node's address as `IP:50051`.
2. Enter the token.
3. Set the exposure and gain.
4. Press **Save scope node and camera**.

The console stores the token write-only, with mode `0600`, and never shows it again.

On the [Pi 5 plan](../architecture.md#the-raspberry-pi-5-plan-tentative), the console runs on the
same Pi: give the node `listen = "127.0.0.1:50051"` and pair it as `127.0.0.1:50051`.

## Known quirks

- **Fresh exposure range.** The first `--probe-camera` after INDI starts can print an exposure
  range of `0 to 3600000 ms` rather than the camera's real range. The node's INDI client misses
  ranges sent in property updates.
- **Cooler power with no 12 V.** A ZWO camera's reported cooler power still climbs when the
  camera has no 12 V. Only the sensor temperature shows whether the cooler works.
- **Cooler after shutdown.** The node never turns the cooler off by itself, so the camera
  keeps its last setting when the console or the node goes away.
- **Saved frames.** Copies in `save_dir` are never deleted. On a microSD card, leave
  `save_dir` off or watch the free space.
- **No version flag.** The binary has no `--version`. Keep track of which commit you built.

## Not in this repository yet

- An install script or package. The steps above are the manual equivalent.
- An aarch64 build in CI.
- Stopping the mount when the node is stopped or shut down (spec 044, not built yet).
- USB or INDI finder cameras (planned as spec 045). Today the finder is a CSI module through
  `rpicam`, or the simulator.
