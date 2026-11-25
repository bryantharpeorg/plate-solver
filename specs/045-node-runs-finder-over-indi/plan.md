# Implementation Plan: The Node Runs the Finder Camera over INDI

**Branch**: `045-node-runs-finder-over-indi` | **Date**: 2026-11-25 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

The scope node gains a third camera adapter for the finder camera, `indi`, for the ZWO
ASI120MM Mini: the camera port over the imaging side's own INDI client, so the finder
camera and the imaging camera share one `indiserver`, each on its own connection and
device. `IndiFinder` describes the camera once at start, as rpicam's `discover` does;
reports one mode at the sensor's size and 1 frame/s, the device's exposure range and
its gain range from 1; and takes each frame as one exposure, whose 16-bit samples give
their high bytes to detection. It sends the gain when the camera's last answer
differs, and passes on no frame at a gain it did not send: after a server restart the
client's retry runs on a freshly started device program at its own gain, and that frame
fails, so the next exposure sends the gain again. The INDI client takes a range a set vector carries,
which fixes the bench's placeholder exposure range for both cameras. The config takes
`adapter = "indi"` and an `[indi]` table of `address`, `device` and `neutral`, and
refuses two INDI cameras with the same `address` unless each names its own device.

Two stories: the adapter and the client's fix, over the imaging tests' stand-in server;
then the config, the binary, the README and the adapter inside the node's service.

## Technical Context

**Language/Version**: Rust, MSRV 1.96, stable in CI.

**Primary Dependencies**: none new. The adapter uses the node's INDI client
(`quick-xml` and `base64`, 030's), and the tests use `tonic`, `scope-link` and `tokio`,
dependencies of the node already. `Cargo.toml` and `Cargo.lock` do not change.

**Storage**: none.

**Testing**:
- `crates/scope-node/tests/finder_script/mod.rs` (new, US1): the finder script, the gain
  step, the exposures, the FITS writer, the worked 16-bit frame, the 8-bit ramp and an
  8-bit frame of v, the test clock and the finder rig's camera, which both stories' test
  files use; it starts with
  `#![allow(dead_code)]`, as `crates/scope-node/tests/support/mod.rs:1` does, since each
  file uses a part.
- `crates/scope-node/tests/indi_finder.rs` (new, US1): the adapter and the client over the
  stand-in server of `crates/scope-node/tests/support/mod.rs`, under paused time, and two
  cameras on one stand-in (US1-S1 to US1-S10).
- `crates/scope-node/tests/indi_finder_node.rs` (new, US2): the config, the built binary
  under `--check-config`, with no server and over a TCP bridge to the stand-in, and three
  scenarios of the adapter inside the node's service over an in-memory link, under paused
  time (US2-S1 to US2-S9).
- No Vitest or Playwright test: the page is unchanged.

**Target Platform**: `scope-node` on the Pi 5, under systemd, beside `indiserver` with
`indi_asi_ccd`.

**Constraints**:
- The camera port, the imaging port, `IndiCamera`'s constructors, `CameraInfo` and the
  scope link's protocol keep their shapes (§ Shapes that existing tests pin).
- `crates/scope-node/tests/support/mod.rs` is used and never edited; US1's
  `crates/scope-node/tests/finder_script/mod.rs` is used by US2 and never edited by it.
- `RUSTFLAGS="-D warnings"`, `--all-features` clippy, and rustdoc with warnings denied.
- No test uses a camera, a Pi or a real INDI server, and no test sends a signal to a
  process it did not start.

**Scale/Scope**: two stories; one new source file and four changed, two new test files
and one shared test module, one README section.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Detection is untouched: the adapter gives the existing cedar detector 8-bit frames, as rpicam's adapter does. The behaviour is grounded in INDI's own properties as the bench log and 030's scripts read them, and in the probe's runs, one against a real `indiserver`; what rests on reading (the device's name for the ASI120MM, its gain step, the set vector's range rule) is listed in § What was run and what was read and invented nowhere. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. The new tests go in new files; the stand-in server is used as it is. The client's fix changes no existing test's observation (§ Shapes that existing tests pin; run). |
| III. Tests first, independent expectations | The oracles are the scripts' definitions and answers the tests write, the FITS bytes the tests build with their own writer, the INDI client's documented waits, the process's exit status and standard error, and the scenarios' words. No test computes an expected pixel with `gray`, an expected exposure with `FRAME_MS`, or an expected message from the code. Time comes from a paused tokio clock, or, for the built node, is only waited on. |
| IV. Critical paths | None is touched: no `Cargo.toml` or `Cargo.lock` change, no `unsafe` (the crate forbids it), nothing under `proto/` or `crates/scope-link`, no `build.rs`. |
| V. Proven open source first, honest licensing | The camera is reached through INDI's own server and ZWO's INDI device program, both LGPL-2.1-or-later, over a socket, behind the camera port, and linked by nothing; the gates reach them through the stand-in server and the existing saved capture of INDI's simulator. Looked for: an existing client (the node's own, 030's, is reused whole); ZWO's SDK (turned down by the owner on 2026-11-09: it ends the static musl build); RAW8 in the device program (by reading only; owner decision 1 (b)). The README names the programs and their licence (FR-011). No copied source, no new licence. |

## Project Structure

```text
crates/scope-node/
├── README.md                                 # US2: ## The finder camera over INDI
├── src/
│   ├── imaging/indi/mod.rs                   # US1: a set vector's min and max
│   ├── indi_finder.rs                        # US1, new: IndiFinder, gray, FRAME_MS, PURPOSE
│   ├── lib.rs                                # US1: pub mod indi_finder
│   ├── config.rs                             # US2: Adapter::Indi, FinderIndiConfig, Config::indi, validate_indi, masked
│   └── main.rs                               # US2: --check-config's [indi], the indi adapter served
└── tests/
    ├── finder_script/mod.rs                  # US1, new: shared by both new test files
    ├── indi_finder.rs                        # US1, new
    └── indi_finder_node.rs                   # US2, new
```

## Interfaces

### `crates/scope-node/src/imaging/indi/mod.rs` (US1)

In `Devices::record`'s `setNumberVector` arm (`crates/scope-node/src/imaging/indi/mod.rs:241`),
after the value is taken and inside the loop over `oneNumber`s, as the probe built it:

```rust
                            // 045: a range the answer carries replaces the
                            // defined one; one that does not parse is kept.
                            if let Some(number) = child
                                .attr("name")
                                .and_then(|name| elements.iter_mut().find(|e| e.name == name))
                            {
                                if let Some(min) = child.attr("min").and_then(parse_number) {
                                    number.min = min;
                                }
                                if let Some(max) = child.attr("max").and_then(parse_number) {
                                    number.max = max;
                                }
                            }
```

Nothing else in the file changes: `describe` reads `number_range` as today, which now
holds the latest range.

### `crates/scope-node/src/indi_finder.rs` (US1, new)

```rust
//! The finder camera over INDI (045): the camera port over the imaging
//! side's INDI client, so the finder camera and the imaging camera can share one
//! INDI server, each on its own device and connection.
use crate::{
    camera::{check_range, FrameFuture, Mode},
    config::CameraMode,
    imaging::{Exposure, ImagingCamera, ImagingError, ImagingFrame, ImagingSettings},
    Camera, CameraError, CameraInfo, GrayFrame, RangeError, Settings,
};

/// The finder camera's frame time in ms: the one mode's `fps` is `1000 / FRAME_MS`,
/// which caps automatic exposure, and the first automatic exposure is this
/// long.
pub const FRAME_MS: f64 = 1000.0;

/// The purpose every exposure of the finder camera carries: a light frame.
pub const PURPOSE: &str = "finder";

fn failed(error: ImagingError) -> CameraError {
    CameraError(error.to_string())
}

pub struct IndiFinder<C> {
    camera: C,
    info: CameraInfo,
    settings: Settings,
    /// Whether the camera has a gain control.
    has_gain: bool,
    /// The gain the camera last answered, at discovery or to an apply;
    /// `None` after a restart or a frame at another gain.
    reported_gain: Option<f64>,
}

impl<C: ImagingCamera> IndiFinder<C> {
    /// Describe the camera once, before the node serves (the port's `info`
    /// is synchronous): a failure ends the start, as rpicam's does.
    pub async fn discover(camera: C, mode: CameraMode) -> Result<Self, CameraError> {
        let described = camera.describe().await.map_err(failed)?;
        if let Some(bayer) = described.bayer {
            return Err(CameraError(format!(
                "the INDI camera {} is a colour camera ({}); the finder camera must be mono",
                described.model,
                bayer.as_str()
            )));
        }
        let size = Mode {
            width: described.width,
            height: described.height,
            fps: 1000.0 / FRAME_MS,
        };
        if let CameraMode::Size { width, height } = mode {
            if (width, height) != (size.width, size.height) {
                return Err(CameraError(format!(
                    "camera has no {width}x{height} mode; it lists {}x{}",
                    size.width, size.height
                )));
            }
        }
        // The node's gain is positive: a range from 0 starts at 1.
        let (gain_min, gain_max) = match described.gain {
            Some(range) => (range.min.max(1.0), range.max.max(1.0)),
            None => (1.0, 1.0),
        };
        for (name, min, max) in [
            (
                "exposure_ms",
                described.exposure_ms.min,
                described.exposure_ms.max,
            ),
            ("gain", gain_min, gain_max),
        ] {
            if !min.is_finite() || !max.is_finite() || max < min {
                return Err(CameraError(format!("camera has invalid {name} range")));
            }
        }
        let in_force = camera
            .apply(&ImagingSettings::default())
            .await
            .map_err(failed)?;
        Ok(Self {
            camera,
            info: CameraInfo {
                model: described.model,
                width: size.width,
                height: size.height,
                modes: vec![size],
                active_mode: Some(size),
                exposure_min_ms: described.exposure_ms.min,
                exposure_max_ms: described.exposure_ms.max,
                gain_min,
                gain_max,
            },
            settings: Settings::default(),
            has_gain: described.gain.is_some(),
            reported_gain: in_force.gain,
        })
    }
}

/// The 8-bit gray pixels of a mono frame: a 16-bit sample's high byte, an
/// 8-bit sample as it is.
pub fn gray(frame: &ImagingFrame) -> Result<Vec<u8>, CameraError> {
    let count = frame.width as usize * frame.height as usize;
    let pixels: Vec<u8> = match frame.bits {
        8 => frame.samples.clone(),
        16 => frame.samples.iter().skip(1).step_by(2).copied().collect(),
        bits => return Err(CameraError(format!("unsupported sample size {bits} bits"))),
    };
    if pixels.len() != count {
        return Err(CameraError(format!(
            "the INDI frame holds {} samples, not {} × {}",
            pixels.len(),
            frame.width,
            frame.height
        )));
    }
    Ok(pixels)
}

impl<C: ImagingCamera + 'static> Camera for IndiFinder<C> {
    fn info(&self) -> CameraInfo {
        self.info.clone()
    }
    fn apply(&mut self, settings: &Settings) -> Result<(), RangeError> {
        check_range(&self.info, settings)?;
        self.settings = settings.clone();
        Ok(())
    }
    fn next_frame(&mut self) -> FrameFuture<'_> {
        Box::pin(async move {
            if self.has_gain {
                // Automatic gain starts from the camera's own, inside the range.
                let wanted = self.settings.gain.unwrap_or_else(|| {
                    self.reported_gain
                        .unwrap_or(self.info.gain_min)
                        .clamp(self.info.gain_min, self.info.gain_max)
                });
                if self.reported_gain != Some(wanted) {
                    let edit = ImagingSettings {
                        gain: Some(wanted),
                        ..ImagingSettings::default()
                    };
                    let answer = self.camera.apply(&edit).await.map_err(failed)?;
                    self.reported_gain = answer.gain;
                }
            }
            let exposure = Exposure {
                exposure_ms: self.settings.exposure_ms.unwrap_or_else(|| {
                    FRAME_MS.clamp(self.info.exposure_min_ms, self.info.exposure_max_ms)
                }),
                purpose: PURPOSE.to_string(),
            };
            let frame = self.camera.expose(&exposure).await.map_err(failed)?;
            // No frame is passed on at a gain the adapter did not send: the
            // client's retry after a restart runs on a new connection, whose
            // camera has its own gain. The next exposure sends it again.
            if self.has_gain && frame.gain != self.reported_gain {
                let sent = self.reported_gain.take().unwrap_or_default();
                return Err(CameraError(format!(
                    "the finder camera exposed at gain {}, not at the gain {sent} sent",
                    frame.gain.unwrap_or_default()
                )));
            }
            Ok(GrayFrame {
                pixels: gray(&frame)?,
                width: frame.width,
                height: frame.height,
                start_s: frame.start_s,
                end_s: frame.end_s,
                exposure_ms: frame.exposure_ms,
                gain: frame.gain.unwrap_or(1.0),
            })
        })
    }
    /// After a failure the gain is sent again with the next exposure: a
    /// restarted server's camera may have its own.
    fn restart(&mut self) -> Result<(), CameraError> {
        self.reported_gain = None;
        Ok(())
    }
}
```

The range check before `in_force` keeps `f64::clamp` from panicking on a device whose
range is reversed or not finite; US1-S5's third discovery holds it.

`next_frame`'s last check is the rule of owner decision 2: no frame is passed on at a
gain the adapter did not send. `reported_gain` is the camera's last answer (discovery's
settings in force, or an `apply`'s), so a camera that quantizes the gain it is sent
answers its own value and its frames pass; a frame whose gain differs comes from a
camera that lost the gain, as the client's retry on a freshly started device program
does. Its error takes the capture loop's existing path
(`crates/scope-node/src/capture.rs:69` to `crates/scope-node/src/capture.rs:77`), so the next frame sends the gain
again, 1 s later, with its own axis readings (§ Decisions).

### `crates/scope-node/src/lib.rs` (US1)

`pub mod indi_finder;` after `pub mod imaging;`.

### `crates/scope-node/src/config.rs` (US2)

- `Adapter` gains `Indi`, after `Simulated`, with the doc comment
  `/// 045: a camera of an INDI server, named by the `[indi]` table.`; the derive and the
  lowercase rename give it the text `indi`.
- Before `/// The imaging camera's adapter.`:

```rust
/// 045: the finder camera's INDI server and device, for `adapter = "indi"`;
/// unknown keys are errors.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinderIndiConfig {
    #[serde(default)]
    pub address: Option<SocketAddr>,
    #[serde(default)]
    pub device: Option<String>,
    /// Controls set to these values on each connection, keyed
    /// `"<property>.<element>"`, as `[imaging]`'s. Empty by default.
    #[serde(default)]
    pub neutral: std::collections::BTreeMap<String, f64>,
}

impl FinderIndiConfig {
    /// `address`, or 127.0.0.1:7624, INDI's own default port.
    pub fn address(&self) -> SocketAddr {
        self.address
            .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 7624)))
    }

    /// `neutral` as `(property, element, value)`, each key split at its first
    /// `.`, in sorted order. `validate` has refused a key that does not split.
    pub fn neutral_controls(&self) -> Vec<(String, String, f64)> {
        self.neutral
            .iter()
            .map(|(key, value)| {
                let (property, element) = key.split_once('.').unwrap_or((key.as_str(), ""));
                (property.to_string(), element.to_string(), *value)
            })
            .collect()
    }

    /// What `--check-config` prints after `[indi]`: `address`, the default
    /// included, `device` when set and `neutral` when not empty, as
    /// `[imaging]` prints them.
    pub fn printed(&self) -> String {
        let mut out = format!("address = \"{}\"\n", self.address());
        if let Some(device) = &self.device {
            out.push_str(&format!("device = \"{device}\"\n"));
        }
        if !self.neutral.is_empty() {
            let entries = self
                .neutral
                .iter()
                .map(|(key, value)| {
                    format!(
                        "\"{}\" = {value}",
                        key.replace('\\', "\\\\").replace('"', "\\\"")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("neutral = {{ {entries} }}\n"));
        }
        out
    }
}
```

- `Config` gains, after `imaging`:
  `/// 045: the finder camera's INDI server and device.`, `#[serde(default)]`,
  `pub indi: Option<FinderIndiConfig>,`.
- `validate` calls `self.validate_indi()?;` after the `[imaging]` checks and before the
  token's, and `Config` gains:

```rust
    /// 045: the `[indi]` table only with `adapter = "indi"`, and two INDI
    /// cameras of one server each on a device named and their own.
    fn validate_indi(&self) -> Result<(), ConfigError> {
        if self.adapter != Adapter::Indi {
            if self.indi.is_some() {
                return Err(ConfigError(
                    "the [indi] table applies only to adapter = \"indi\"".into(),
                ));
            }
            return Ok(());
        }
        let finder = self.indi.clone().unwrap_or_default();
        if finder
            .device
            .as_ref()
            .is_some_and(|device| device.is_empty())
        {
            return Err(ConfigError("indi.device must not be empty".into()));
        }
        for (key, value) in &finder.neutral {
            match key.split_once('.') {
                Some((property, element)) if !property.is_empty() && !element.is_empty() => {}
                _ => {
                    return Err(ConfigError(format!(
                        "indi.neutral key {key} must be PROPERTY.ELEMENT"
                    )))
                }
            }
            if !value.is_finite() {
                return Err(ConfigError(format!(
                    "indi.neutral {key} must be a finite number"
                )));
            }
        }
        let Some(imaging) = self
            .imaging
            .as_ref()
            .filter(|imaging| imaging.adapter == ImagingAdapter::Indi)
        else {
            return Ok(());
        };
        if imaging.address() != finder.address() {
            return Ok(());
        }
        match (&finder.device, &imaging.device) {
            (Some(a), Some(b)) if a == b => Err(ConfigError(format!(
                "indi.device and imaging.device both name {a}: the finder camera and the imaging camera need a device each"
            ))),
            (Some(_), Some(_)) => Ok(()),
            _ => Err(ConfigError(format!(
                "the finder camera and the imaging camera share the INDI server at {}: set indi.device and imaging.device",
                finder.address()
            ))),
        }
    }
```

- `masked`'s adapter match gains `Adapter::Indi => "indi"`.
- The file's `#[cfg(test)] mod tests` (`crates/scope-node/src/config.rs:346`) does not
  change.

### `crates/scope-node/src/main.rs` (US2)

- The `use scope_node::{...}` list gains `indi_finder::IndiFinder`.
- Under `--check-config`, right after `print!("{}", config.masked());`
  (`crates/scope-node/src/main.rs:68`):

```rust
        if config.adapter == Adapter::Indi {
            print!(
                "\n[indi]\n{}",
                config.indi.clone().unwrap_or_default().printed()
            );
        }
```

- The adapter match (`crates/scope-node/src/main.rs:80`) gains:

```rust
        Adapter::Indi => {
            let indi = config.indi.clone().unwrap_or_default();
            let camera = IndiCamera::tcp(
                indi.address(),
                indi.device.clone(),
                indi.neutral_controls(),
                Arc::new(SystemClock),
            );
            serve(
                IndiFinder::discover(camera, config.mode).await?,
                settings,
                &config,
            )
            .await
        }
```

### `crates/scope-node/README.md` (US2)

Before `## The imaging camera` (`crates/scope-node/README.md:313`), this section, with
the blank line after it:

````markdown
## The finder camera over INDI

`adapter = "indi"` takes the finder camera from an INDI server, as the imaging camera
does: a mono camera such as ZWO's ASI120MM Mini, through INDI's `indiserver` and the
camera's device program (`indi_asi_ccd`), which the owner installs and starts on the
scope node. The node is their client over a TCP socket and links none of them (INDI
core and the device programs are under LGPL-2.1-or-later). The finder camera and the
imaging camera may share one server and one device program; each has its own
connection and its own device.

```toml
adapter = "indi"
exposure_ms = 1000       # or "auto"
gain = 30                # in the camera's own units, or "auto"

[indi]
address = "127.0.0.1:7624"               # the INDI server; the default
device = "ZWO CCD ASI120MM Mini"         # the device's name as the server gives it
neutral = { "CCD_CONTROLS.Offset" = 10 } # optional, set on each connection
```

The `[indi]` table is optional and applies only to `adapter = "indi"`. With no `device`
the node takes the first device whose `DRIVER_INTERFACE` has the camera bit. When the
`[imaging]` table is also `indi` with the same `address`, both tables must name their
device and the two names must differ; the node refuses to start otherwise. The rule
compares the two addresses, not the server they reach, so give one server the same
`address` in both tables. `ZWO CCD ASI120MM Mini` is the name ZWO's device program is
expected to give the camera, from the `ZWO CCD <model>` it gave the ASI071; run
`indi_getprop -p 7624` on the scope node to read the name the server really gives it.
`neutral` sets each control it names on each connection, as `[imaging]`'s does; the
camera's `Offset` is the one way to lift the star frames' offset level, which the high
byte below can leave at 0.

The node describes the camera once, when it starts, from the device's own properties:
its model is the device's name, its one mode the sensor's size, its exposure range the
device's, and its gain range the device's `Gain` from 1 (the node takes no gain of 0).
`mode` is `auto` or that size. With the server down, the device missing or a colour
camera named, the node exits 1 with the reason, and systemd starts it again a second
later (`Restart=always`, `RestartSec=1`); the mount and the imaging camera are not
served meanwhile. By default systemd gives up on a unit started 5 times within 10 s,
so a server away for about 5 s would leave the node stopped until
`systemctl reset-failed scope-node`. Add this to the `[Unit]` section of the unit
under Config and systemd, so that systemd keeps starting it:

```ini
[Unit]
StartLimitIntervalSec=0
```

Each star frame is one exposure: the node asks for it, the device program takes it and
sends it as FITS, and detection reads its 8-bit gray pixels. A 16-bit sample gives its
high byte (the ASI120MM's 12 bits, shifted up, give their top 8); an 8-bit sample is
taken as it is. Faint stars are the camera's gain's to bring up: raise `gain`, or leave
it `auto`. With `exposure_ms = "auto"` the first exposure is 1000 ms and automatic
exposure goes no longer, the mode reporting 1 frame/s; a fixed exposure may be as long
as the camera allows. Automatic gain multiplies the camera's own gain, which on these
cameras is a logarithmic scale: its steps are fine near 1 and coarse near the top, and
with no star it takes four frames from 1 to 64. A star frame's start is the moment the
device reports the exposure under way, and its end that plus the exposure.

When the INDI server restarts, an exposure in flight is tried once more a second later
on a new connection; when that fails too, the node logs `camera capture failed: ` and
the reason and tries again every second until the server answers. An exposure the
camera took at a gain other than the one the node sent (the first on a freshly
started device program, which has its own gain) is not sent as a star frame: the
node logs it the same way and sends the gain again with the next exposure. The
imaging camera's exposure in flight is tried once more the same way, but it and the
exposures after it run at the device program's own settings (on the ASI071, gain 0 and
the cooler off) until its settings are applied again: on a new connection the node
sends it only its `neutral` controls.
````

## Frame timing

What each of the frame's times costs the console, by the arithmetic below; no figure is
measured on the camera, and the bench check of § Outside the factory measures the two
terms. At the equator the sky turns 15″ a second.

- **`start_s`, and so the mid-exposure time** of a live solve and of a sync
  (`crates/finder/src/live.rs:249`, `crates/finder/src/session.rs:1554`), is the node
  clock's reading when the client reads the device's first `Busy`
  (`crates/scope-node/src/imaging/indi/mod.rs:1468`): late by the hop from the device
  program to the client, or early if the program reports `Busy` before the sensor
  integrates. Estimated at 50 ms, not measured: 0.75″ of sky, 0.016 px at 48″ a pixel and
  0.16 px at 4.8″ a pixel. With no `Busy` at all, `start_s` is the request's reading
  (`crates/scope-node/src/imaging/indi/mod.rs:1725`), earlier than the start.
- **One device program for both cameras.** `indi_asi_ccd` is a single process serving
  the finder camera and the imaging camera, and its output reaches `indiserver` through
  one stream. An ASI071 frame is 32,472,192 B, about 43 MB as base64. By reading, the
  finder camera's `Busy` and its BLOB can queue behind that transfer, which makes
  `start_s` late by up to about 1 s rather than 50 ms whenever an imaging frame is on its
  way: 15″ of sky, 0.31 px at 48″ a pixel and 3.1 px at 4.8″ a pixel. Not measured;
  bench step 4 measures it.
- **`end_s`** is `start_s` plus the exposure asked for
  (`crates/scope-node/src/imaging/indi/mod.rs:1733`), as 030's FR-027 says; the device's
  real exposure is taken to be the one asked for (by reading). The bench saw the ASI071
  gather about 11 % more signal a second above 1 s, which is a sensor mode, not a time.
- **The axes** are read before the request and after the frame is in hand
  (`crates/scope-node/src/capture.rs:80` and `crates/scope-node/src/capture.rs:82`), so their mean lags mid-exposure by half
  of (the download less the request's lag to the start). For each 0.1 s of that, the mean
  is 0.05 s late: on an untracked Dob at rest, nothing; tracking at the sky's rate,
  0.75″, 0.016 px at 48″ and 0.16 px at 4.8″; at a sync's limit of 0.2° a second
  (`crates/finder/src/model.rs:258`), 36″, 0.75 px at 48″ and 7.5 px at 4.8″. The same
  rule measures the axes' rate over the exposure alone, while they moved over the
  exposure, the lag and the download, so it reads the rate high and refuses sooner.
- **The first exposure after a GoTo settles** is the first whose `start_s` is at or after
  the settle (`crates/finder/src/goto.rs:966`), which comes 500 ms after the stop: a
  `start_s` late by 50 ms lets through an exposure that began up to 50 ms before it,
  with the mount already still; late by up to 1 s behind an imaging frame, one that
  began up to 1 s before the settle, which is up to 500 ms before the stop, while the
  mount still moved.
- **Frame period.** Against a real `indiserver` with the simulator's 1280 × 1024 16-bit
  frames (measured, on one machine, not a Pi), a 1 s frame's start followed the last one's
  end by about 2 % of the period: the download, the detection and the request's lag
  together.

## What was run and what was read

**Run on 2026-11-25, before the draft**, on the branch `probe/045` cut from `main` at
be7f3ff6, with the factory's toolchain (rustc 1.100.0) and `--locked`:

- the trial build of § Interfaces, in six commits, one of them probe-only (the leads and
  the Docker run's client): the client's range (1,238 B of diff), the adapter, config and
  binary, the tests, the scenarios' form with the README section and the gates' fixes,
  and last the reversed-range refusal, the 60 s bounds on the service reads and US2-S2
  and US2-S3 as two tests; on that last commit, every scenario's test of US1 (10) and US2
  (7) passing three times;
- both gates' Rust commands, `lint`'s four and `test`'s two, the workspace's 1495 tests
  passing (one probe-only test ignored) with no existing test changed, on the commit
  before the last; the last changed only the node's two new test files and
  `indi_finder.rs`, and on it `cargo fmt --all --check`, clippy and `cargo doc` of
  `scope-node` with warnings denied, and every `scope-node` test (245 passing, the
  probe-only one ignored) passed; before that, clippy 1.100 refused `chunks_exact(2)`
  (§ Plan traps 8), and the fix is in § Interfaces;
- every existing `crates/scope-node` test (226) with the client's range fix alone and with
  the trial adapter;
- the leads as runs, probe-only (`tests/probe_045_leads.rs` on the probe, never in a
  story): the 16-bit options on synthetic frames (spec owner decision 1); the service over
  a camera whose gain range starts at 0 and whose gain is 0 (one frame in 30 s, the
  refusal logged each second) and from 1 (gain 1, 4, 16, 64, 100); automatic exposure
  from 100 ms at 1 frame/s (400, then 1000 ms held) and with no mode (to 1,638,400 ms by
  the eighth frame); and two unnamed clients of one stand-in server both taking its first
  camera, two named each their own;
- mutations, each against both new test files: listed in § Mutations;
- one run against a real server, probe only: the image `asi-bench-sim` (Debian trixie,
  INDI 1.9.9) with `--network none`, `indiserver -p 7624 indi_simulator_ccd
  indi_simulator_guide`, and the probe's x86 node inside it with the finder camera on
  `Guide Simulator` and the imaging camera on `CCD Simulator`, its exposures of 1 s
  fixed at gain 30, and a client of the probe's reading the finder camera's star frames and
  taking 1 s imaging exposures in a loop. `--check-config` printed the `[indi]` table;
  the finder camera was described as 1280 × 1024 at 1 frame/s, exposure 10 to 3,600,000 ms, gain
  1 to 100; both cameras sent frames side by side with no failure; with `indiserver`
  stopped for 4 s and, in a second run, 6 s, the node logged
  `camera capture failed: the exposure failed twice: cannot reach the INDI server at 127.0.0.1:7624: Connection refused (os error 111)`
  and then the same reason without the prefix once a second; the imaging exposures
  failed the same way; and once the server was back, both cameras sent frames again, the
  finder camera's at gain 30, its ids continuing with no gap;
- in the same image, five starts: no server (exit 1 at once,
  `scope-node: cannot reach the INDI server at 127.0.0.1:7624: Connection refused (os error 111)`);
  a device the server lacks (exit 1 after 5 s, `... has no device Missing CCD`); a
  telescope named as the finder (exit 1 after 10 s,
  `... the INDI camera Telescope Simulator did not come ready within 10 s`); both INDI
  cameras on one server unnamed (exit 1 at once with FR-008's text); and no device named
  with a mount and a camera on the server (the camera taken, the node serving).

**Run by cynical review round 1, the same day** (its own scratch copy of the probe's
`f6c3df0a`; read here from its log): the service with the probe's adapter, gain auto
and the node's own synthetic field (36 stars), the server closing under the second
exposure and the second connection's camera at gain 0 — frames at gain 1 and then 0,
and none in 120 s of paused time, `cannot apply camera settings: gain 0 is outside the
camera's range 1–100` 118 times; lead 2 again at an offset level of 3 in 12-bit units
(owner decision 1); and `libindidriver.so.1` of the simulator image (1.9.9) exporting
`IUUpdateMinMax`.

**Run in the fold of review round 1, the same day**, on `probe/045` from `f6c3df0a`,
with the same toolchain and `--locked`, in five commits that only add: 3aa0bb38, the
rule of owner decision 2 (§ Interfaces, `next_frame`), US1-S7's test rewritten to it and
the restart scenario with automatic gain over star frames; bd2e382f, the tests as the
stories now hold them (the shared test module, US1-S8 and US1-S9 of the draft and the
restart scenario moved to US2 as US2-S7 to US2-S9, US2-S7 of the draft moved to US1 as
US1-S9, US1-S3 given its restart); 57844515, `neutral` in `[indi]` (owner decision 4
(e)) and the README section folded; b2b19ba4, "finder camera" in the messages and
comments; df2ea0c9, the README's wording of the shared-server rule. Runs:
- the four candidate rules of § Decisions on the restart scenario, each patched in and
  restored: none (the draft) and the gain sent before every exposure both gave star
  frames at gain 1 and then 0, then 31 refusals in 30 s of paused time and no frame; the
  check taking the exposure again at once gave gain 1 twice, the second frame starting
  at reading(3000), with no log line; the check failing the frame gave gain 1 twice, the
  second at reading(4000), with one `camera capture failed:` line;
- at b2b19ba4: `cargo fmt --all --check`, clippy and `cargo doc` of `scope-node` with
  warnings denied, every `scope-node` test (246 passing, the probe-only one ignored) and
  the node's doc-tests; both new test files (18 scenarios and the stand-in's own 4 tests
  in each) three times, every one passing each time, the restart scenario among them;
- US1 alone: with `config.rs`, `main.rs` and the README as at be7f3ff6 and US2's test
  file absent, clippy of `scope-node` and every `scope-node` test (233 passing), the
  tree restored after;
- thirteen mutations, listed in § Mutations;
- df2ea0c9 changed the README alone, after those runs.

**Run by probe round 1, the same day, after the flip**: a builder building both stories
from this trio alone, on the branch `probe/045-r1` from `79a87345` (`main` at be7f3ff6
and the trio), with the factory's toolchain and `--locked`, in four commits: 4a04655e
and 765b1249, US1's tests first and US1; f73f2456 and 49cfed7b, US2's. Runs:
- before each story its test file did not build, as T002 and T007 say; US2-S7 to US2-S9
  alone passed on US1's code, `start_s` reading(5300) and reading(4000) exactly;
- each story's two focused commands three times, every test passing each time;
  § Interfaces' code compiled and passed as written;
- at 49cfed7b, the gates' Rust commands: `lint`'s four all passing; workspace nextest
  1,489 passed, 0 failed, 0 skipped, the 1,463 existing tests unchanged and 26 new;
  `cargo test --workspace --locked --doc` ok; `cargo-machete` clean, and `cargo-deny`
  clean with the host's cargo cache `--offline`, the factory's cache lacking one crate;
- 45 mutations on 765b1249 and 49cfed7b, 35 failing a scenario's test (§ Mutations);
- the optional run against a real server, in the drafting probe's image with
  `--network none` and the same two simulators, the tip's node set up from the README
  section alone:
  `--check-config` printed `[indi]`; one finder camera exposure a second, a BLOB each and
  one `CCD_GAIN` vector per connection; with the server stopped 4 s, the node logged
  `camera capture failed: the exposure failed twice: ...` and the bare reason twice,
  then one exposure a second again, with no `exposed at gain` line; both tables unnamed,
  exit 1 with FR-008's text.

It found 0 blocking and 9 clarity defects, 045-R1 to 045-R9, every citation checked at
`79a87345` and none stale.

**Run in the fold of probe round 1, the same day**, on `probe/045-r1` from 49cfed7b, in
two commits that change test files only: 31f390ce, US1-S8's unparsed `max` (045-R2) and
US1-S10 (045-R3); 21c62324, US2-S5's `[imaging]` table and US2-S6's named device
(045-R5). Runs: each story's test file three times, every test passing each time; five
mutations, each failing the case written for it (§ Mutations); at 21c62324,
`cargo fmt --all --check`, clippy and `cargo doc` of `scope-node` with warnings denied,
every `scope-node` test (239 passing) and its doc tests.

**Read, not run**:
- the ASI120MM's device name, `ZWO CCD ASI120MM Mini`, from the ASI071's on the bench;
  its gain range of 0–100 and its gain step; RAW8 through `CCD_VIDEO_FORMAT`;
- the rule that a `setNumberVector`'s `oneNumber` may carry `min` and `max`, from the
  bench log of `indi_asi_ccd` 2.2; INDI's white paper and libindi's source were not read;
- what the console does with each time (§ Frame timing), and the live pointing's 5 s
  staleness (`crates/finder/src/pushto.rs:184`);
- the one device program's stream delaying the finder camera's `Busy` and BLOB behind
  an imaging frame (§ Frame timing);
- systemd's start limit: its defaults read with `systemctl show`
  (`DefaultStartLimitIntervalUSec=10s`, `DefaultStartLimitBurst=5`) and its effect from
  `man systemd.unit`, not run with the node;
- what a freshly started device program resets: by the client's code, the finder camera
  sends only its gain, the exposure and `FRAME_LIGHT` with each request, and step 6's
  switches and step 7's neutral controls on each connection before a retried exposure
  is armed (`crates/scope-node/src/imaging/indi/mod.rs:887`, `crates/scope-node/src/imaging/indi/mod.rs:915`, and `crates/scope-node/src/imaging/indi/mod.rs:1066` to `crates/scope-node/src/imaging/indi/mod.rs:1075`);
  binning, region and video format it never sends;
- the imaging camera's settings after a reconnection (the bench's `Gain=0` and
  `COOLER_OFF=On` on a fresh connection, and the client's step 7);
- the imaging console's handling of a failed exposure during a server restart;
- that nothing else in the workspace matches on the config's `Adapter`: a search for
  `Adapter::Rpicam` and `config::Adapter` finds only `crates/scope-node/src/main.rs:81`
  and `crates/scope-node/src/config.rs:341` and `crates/scope-node/src/config.rs:351`.

**Not run at all**: the ASI120MM, the ASI071, `indi_asi_ccd`, a Pi, the console against
the finder camera over INDI, the page half of the `test` gate (`npm ci` installs packages, and no
story writes anything the page is built from), and the factory's gate boundary itself.

## Mutations

Each run once against both new test files on the probe's b2b19ba4, the file restored
after, and the last five against their story's test file on `probe/045-r1` at 21c62324
(the fold of probe round 1); every mutation failed at least one scenario's test:

| Mutation | Scenarios failed |
| --- | --- |
| A set vector's `min` ignored (FR-001) | US1-S1, US1-S4, US1-S5, US1-S8 |
| The gain range not raised to 1 (FR-002) | US1-S1, US1-S2, US1-S4, US2-S6, US2-S8, US2-S9 |
| The low byte of each 16-bit sample (FR-003) | US1-S2 |
| `restart` keeps the last gain (FR-004) | US1-S3, US2-S7 |
| A frame at a gain the adapter did not send passed on (FR-004) | US1-S7, US2-S9 |
| The shared-server rule skipped (FR-008) | US2-S3 |
| `--check-config` without the `[indi]` table (FR-009) | US2-S4 |
| A colour camera taken (FR-002) | US1-S5 |
| The mode at 0.5 fps (FR-002, FR-003) | US1-S1, US2-S6, US2-S8 |
| A reversed range taken (FR-002) | US1-S5 |
| The binary passing no neutral controls (FR-010) | US2-S6 |
| An `[indi] neutral` key not checked (FR-007) | US2-S2 |
| `[indi] neutral` not printed (FR-009) | US2-S4 |
| An unparsed `max` taken as 0, not kept (FR-001) | US1-S8 |
| The gain range's maximum not raised to 1 (FR-002) | US1-S10 |
| A camera with no gain control taken as having one (FR-002, FR-004) | US1-S10 |
| The finder camera built from `[imaging]`'s address (FR-010) | US2-S5 |
| The binary passing no device (FR-010) | US2-S6 |

Probe round 1's pass (45 mutations, on 765b1249 and 49cfed7b) left ten alive: the five
above, each now failing the case added for it, and five that no test can see, in § Read
from the diff (an unparsed `min` taken as 0, `gray`'s sample-count refusal removed,
`PURPOSE` another word that chooses a light frame, the binary ignoring `config.mode`,
the README without `StartLimitIntervalSec=0`).

The draft's pass (ten mutations, on f6c3df0a) is superseded: its "frame's own gain not
taken" no longer exists, since the fold's rule fails such a frame. In the draft, a first
pass before the service reads had their 60 s bounds ran into the run's 5-minute timeout
on the climb from the floor with the floor removed; the bounds (§ Plan traps 6) end such
a stall within the test.

## Read from the diff, not from a test

- **`PURPOSE` is `finder`** (FR-003): the scripts expect `FRAME_LIGHT`, which any purpose
  but `dark` and `flat` chooses; the word itself is the diff's.
- **The failure of FR-004 reaches the journal** through the capture loop's
  `camera capture failed: ` (`crates/scope-node/src/capture.rs:69`): US1-S7 asserts the
  text and US2-S9 the second it costs, and no test reads standard error for it.
- **An unparsed `min` is kept** (FR-001): the finder script defines the minimum as 0, so
  kept and taken as 0 look alike; US1-S8's unparsed `max` holds the rule.
- **`gray`'s refusals** of a sample size other than 8 or 16 bits and of a sample count
  other than width times height (FR-003): no frame through `IndiCamera` reaches them,
  since `fits::read` refuses another `BITPIX` (`crates/scope-node/src/imaging/fits.rs:124`)
  and short data (`crates/scope-node/src/imaging/fits.rs:143`).
- **The binary passes `config.mode`** (FR-010): no built-node test names a mode; the
  adapter's refusal of another size is US1-S5's.
- **The README's `StartLimitIntervalSec=0`** (FR-011): no test reads the README; the
  section is copied from § Interfaces.

## Shapes that existing tests pin

Every existing test passes unchanged with both stories built (run on the probe). The
files beside the change, and what the stories do to what they observe:

- **`crates/scope-node/tests/indi.rs`, `indi_expose.rs` and `indi_settings.rs`.** Their
  scripts' set vectors are written by the stand-in's `set_number`
  (`crates/scope-node/tests/support/mod.rs:782`), which writes no `min` or `max`, so
  FR-001 changes nothing they observe. `the_saved_capture_plays_to_its_end`
  (`crates/scope-node/tests/indi_expose.rs:979`) plays INDI's simulator's saved capture,
  whose set vectors carry ranges only for `CCD_FRAME`, `CCD_STREAM_FRAME` and
  `GUIDER_FRAME`; its expected exposure, gain, offset and binnings come from
  `CCD_EXPOSURE`, `CCD_GAIN`, `CCD_OFFSET` and `CCD_BINNING`, which none re-ranges.
- **`crates/scope-node/tests/support/mod.rs`** is used by both new files and the shared
  test module and not edited; its own four tests run in each file that includes it,
  unchanged.
- **`crates/scope-node/tests/cli.rs`** runs a `simulated` config under `--check-config`
  and with bad keys: the `[indi]` block prints only for `indi`, and the refusals name
  their keys as before.
- **`crates/scope-node/tests/imaging_config.rs`** parses `[imaging]` tables and prints
  them: their code, texts and `printed` are unchanged (`[indi]`'s `neutral` has its own
  copy of their rules and print, with `indi.` texts), and none of its configs has
  `adapter = "indi"`, so `validate_indi` returns at once.
- **`crates/scope-node/tests/rpicam.rs`, `rpicam_node.rs` and `rpicam_stride.rs`**: the
  rpicam adapter is untouched, and the binary's adapter match only gains an arm.
- **`crates/scope-node/tests/raw_frame.rs`, `detect.rs`, `noise_floor.rs`,
  `exit_on_signal.rs`, `exit_stops_the_mount.rs` and `stop_before_exit.rs`** run the
  `simulated` adapter: unchanged.
- **`crates/scope-node/tests/imaging_port.rs`, `imaging_service.rs`, `imaging_save.rs` and
  `imaging_probe.rs`**: the imaging port is unchanged.
- **The unit tests** in `config.rs` (three; `minimal_config_uses_automatic_controls_and_cedar_defaults`
  still reads `Adapter::Rpicam` as the default), `camera.rs`, `exposure.rs` and
  `imaging/indi/mod.rs`: their text is unchanged.
- **The simulator and the finder console** never name the node's config `Adapter`.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| A camera adapter for the finder camera over the existing INDI client | ZWO's SDK linked; a second INDI client | Owner, 2026-11-09: the SDK ends the static musl build. The client is 030's, tested and on the bench. |
| Describe once at start, exit on failure | Wait at start; serve without a description | Owner decision 8: the service reads `info` once; rpicam's start does the same. |
| The high byte | RAW8; a stretch; a smaller shift | Owner decision 1, measured. The camera's gain is the one lever, as on rpicam. |
| Gain in the camera's units, from 1 | From 0; decibels made linear; fixed only | Owner decision 2: from 0 the service stalls (run). |
| 1 frame/s, first automatic exposure 1000 ms | 0.5 frame/s; no mode; a key | Owner decision 3: no mode lets automatic exposure run to the camera's 2000 s (run). |
| Send the gain when the camera's last answer differs | Send it with every exposure; send it on change only | A vector a frame is a round trip each frame; a change-only rule leaves a restarted camera at its own gain. |
| No frame passed on at a gain the adapter did not send: the frame fails, and the next exposure sends the gain | The check taking the exposure again at once; the gain sent before every exposure; the client putting its caller's settings back on a new connection | Owner decision 2, review round 1's blocker. Measured on the restart scenario (§ What was run): with no check, or with the gain sent before every exposure, the retry brought a frame at gain 0 and the service stalled, since the retry runs inside the client after the gain went to the old connection. Both checks hold; failing the frame is the smaller (one check, one error, no second exposure path) and leaves the capture loop's axis readings around the one exposure passed on, where taking it again at once would span the failed try, the reconnection and two exposures; it costs the frame 1 s and one journal line. The client's rule is 030's code for both cameras, the imaging side's follow-up (§ Outside the factory). A fresh device program resets nothing else the finder camera sends (§ What was run and what was read). The check compares the camera's last answer, not the gain wanted, so a camera that quantizes the gain does not fail every frame. |
| `[indi]` with `address`, `device` and `neutral`, the shared-server rule in `validate` | Flat keys; `device` always required; no rule; no `neutral` | Owner decision 4 with (e). The rule needs both tables, so it sits where both are read, and `--check-config` refuses alike. `neutral` is the one lever on the offset level (decision 1); it copies `[imaging]`'s rules and print rather than sharing a function with them, so `[imaging]`'s code is not touched (about 2.9 KB in all). |
| The finder script in a shared test module, US1's | A copy in each test file | US2's three service scenarios need the finder script; a copy would cost US2 about 6 KB of diff. The module follows `support/mod.rs`'s form. |
| `StartLimitIntervalSec=0` on the node's unit | systemd's default limit; waiting at start | Owner decision 8: with the default, a server away for about 5 s at the node's start leaves it failed. The README says so and the Pi's unit carries it. |
| The client takes a set vector's range | Leave it; a spec of its own | Owner decision 5: 13 lines, both cameras' ranges right on a freshly started device program. |
| Refuse a colour camera as the finder camera | Treat its mosaic as gray | Detection on a mosaic is not what any test or source grounds; refusing names the mistake of a finder camera pointed at the ASI071. |
| Two stories, the service scenarios in US2 | One; three; the service scenarios in US1 | One would be about 92.5 % of 65,536 B as probed; two are 44.3 % and 48.3 % on the drafting probe, and 48.5 % and 48.6 % built from this text with the fold's cases (§ Plan traps 2). The service scenarios run US1's adapter inside the node's service, which the node serves from US2 on. |

## User Story 1 - The finder camera is described and framed over INDI

Tests first: `finder_script/mod.rs` and `indi_finder.rs`. Then the client's range in
`imaging/indi/mod.rs`; then `indi_finder.rs` and `lib.rs`.

## User Story 2 - The node runs the finder camera over INDI beside the imaging camera

Tests first: `indi_finder_node.rs`, over US1's `finder_script/mod.rs`. Then `config.rs`,
`main.rs` and the README.

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at be7f3ff6):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  The story runs both, and the judge sees their results.
- **CI in the merge queue** runs every required `ci.yml` job.

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed:** every spec in `depends_on_landed`. Met at be7f3ff6.
2. **`main` is still as read.** Every citation here was read at be7f3ff6 on 2026-11-25.
   If `main` moves in a file § Project Structure names, or in a test § Shapes names,
   re-read them and re-run the validator.
3. **The owner decisions are settled and the owner approvals given** (spec § Owner
   decisions, § Owner approvals). Decisions met: the owner answered all eight on
   2026-11-25 at 19:21 UTC, as recommended and as review round 1 amended them (spec
   § Clarifications). The approvals are given by the flip.
4. **Probe builds.** Met. The owner flipped the spec on 2026-11-25 ("flip to ready") on the
   condition that every prerequisite is met; probe round 1 built both stories from this
   text alone the same day (branch `probe/045-r1`, tip `49cfed7b`, plus the fold's
   31f390ce and 21c62324): 0 blocking and 9 clarity defects, folded here (§ What was run
   and what was read). The flip's merge, on the owner's go, gives the owner approvals;
   `main` still at be7f3ff6.
5. **Under owner decision 7 (b) only:** the camera's saved definitions are staged by the
   operator. Under (a), nothing is staged.

## Outside the factory

Written here and not run.

**The bench check**, by the owner, on the desk machine, in the `asi-bench-asi` image that
took the ASI071 on 2026-11-20 (indi-asi 2.2, libasi 1.27), the camera's USB passed into
the container as then, once 045 has landed or from its probe's branch:

1. Plug the ASI120MM in, start `indiserver -p 7624 indi_asi_ccd`, and connect the device
   first (`indi_setprop -p 7624 '<device>.CONNECTION.CONNECT=On'`): a camera defines
   `CCD_EXPOSURE` and `CCD_CONTROLS` only once connected. Then save the definitions' XML
   from a client of the server's own socket, as the 11-20 kit captured its finding 1
   (`indi_getprop` prints values only, never a `min` or `max`), and
   `scope-node --probe-camera`'s description for the ranges: the device's name (expected
   `ZWO CCD ASI120MM Mini`), `DRIVER_INTERFACE`, `CCD_INFO`, `CCD_EXPOSURE`'s defined
   range and the range its first answer carries, `CCD_CONTROLS`' elements with their
   ranges and starting values, and whether a `CCD_VIDEO_FORMAT` is defined (owner
   decision 1 (b)). These are owner decision 7's data.
2. With the server running, plug the ASI071 in, then unplug and replug the ASI120MM; save
   whether `indi_getprop` lists each without a restart (owner decision 6). Watch
   `udevadm monitor` meanwhile, and while the device program opens each camera: a camera
   that leaves and comes back on the bus when opened would make the rule a restart loop.
3. With the ASI120MM as `[imaging]`, run `scope-node --probe-camera --probe-exposure-ms 1000`:
   its description, now with the camera's own exposure range, and how long after each
   exposure's end the frame is in hand: the download of § Frame timing.
4. With the ASI120MM as the finder camera (`adapter = "indi"`, its device named,
   `exposure_ms = 1000`, `gain = 30`) and the ASI071 as `[imaging]` on the same server,
   run the node and the console on the desk machine: Setup should show a live preview,
   and an imaging exposure should come at the same time. Read the finder camera's star
   frames' `start_s` for a minute with the imaging camera idle and a minute while it
   takes full frames back to back: their spacing, and any `start_s` late by up to about a
   second when an imaging frame lands, is § Frame timing's shared stream. Then restart
   `indiserver`: both should send again within a few seconds, and the journal should hold
   `camera capture failed:` lines only for the outage, among them at most one
   `the finder camera exposed at gain ...` line; read the imaging frames' `GAIN` and
   `CCD-TEMP` cards after the restart (by the client's code, the device program's own
   gain and the cooler off: the follow-up below).
5. Note the frames' brightness at gains 0 (refused: 1), 30, 60 and 100 with the cap on: the
   gain step of owner decision 2.
6. With the cap on, take the camera's shortest exposure at its own `Offset` and at two
   higher ones: the median in 12-bit units and its high byte, the offset level the star
   frames start from (owner decisions 1 and 4 (e)), and the `Offset` that lifts it to a
   few high-byte levels for `[indi] neutral`.

**The Pi**, by the owner, once 045 has landed:

1. Build a new Pi kit from the landing commit and install its node.
2. Set the node's config to `adapter = "indi"`, `[indi] device` the name step 1 read,
   `exposure_ms = 1000` and a gain from step 5, and `[indi] neutral` the `Offset` of step
   6 when it needs one; keep `[imaging]` with the ASI071's device. Run `--check-config`.
   Then set the same exposure and gain in the console's Setup: on pairing the console
   sends its own stored camera settings (`crates/finder/src/link.rs:476` and `crates/finder/src/link.rs:477`),
   which override the node's.
3. Give the node's unit `StartLimitIntervalSec=0` in its `[Unit]` (the Pi's deploy
   script now carries it), and check that `systemctl show -p StartLimitIntervalUSec
   scope-node` reads 0.
4. Replace or drop the udev rule as owner decision 6 is answered at the bench, with both
   cameras plugged in, and restart `indiserver` and the node.
5. Read the node's journal: no `camera capture failed:` line after the start.

**The field check**, by the owner, with either optic:

- **The 16 mm lens** (about 17.2° × 12.9° at about 48″ a pixel, inside the shipped
  10–30° database): Setup should show stars within a few frames and the live pointing
  should solve; try `exposure_ms = "auto"` and `gain = "auto"` once and note where they
  settle.
- **The SV165 guide scope** (about 1.72° × 1.29° at about 4.8″ a pixel): needs a narrow
  Tycho database, a separate measurement (047's § Outside the factory); with it, the same
  checks at a fixed 500 or 1000 ms, the smear of an untracked 1 s exposure being about
  3 px near the equator.
- In both, the push-to guidance should not go stale between frames at the exposure used.

**A follow-up for the owner, not this spec: the imaging camera's settings after a
reconnection.** When the INDI server restarts while the imaging camera exposes, 030's
client tries the exposure once more on a new connection, and on it sends only the
neutral controls (`crates/scope-node/src/imaging/indi/mod.rs:915`); the retry's path
re-applies nothing else (`crates/scope-node/src/imaging/indi/mod.rs:1066` to `crates/scope-node/src/imaging/indi/mod.rs:1075`).
A freshly started device program has its own settings: the ASI071's were `Gain=0` and
`COOLER_OFF=On` on the bench's fresh connection of 2026-11-20. So the retried exposure
and those after it run at gain 0 with the cooler off until the console applies its
settings again, and nothing in the frame says they are not the run's (by reading; bench
step 4 reads it). Review round 1 found it (its finding 11); the owner answered that it is
a spec of its own (decision 6). The finder camera's rule here (owner decision 2) is the
finder camera's alone.

## Plan traps

They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then names a committed test the `test` gate runs.
2. **One evidence budget.** *Check:* the story's diff stays under 64 KiB, measured with
   `git diff <base> -- . ':(exclude)crates/finder/web/dist' | wc -c`; each story is
   sized under 60 % of 65,536 B, 39,321 B. Measured on the drafting probe's df2ea0c9,
   each story's files against be7f3ff6: US1 29,009 B, 44.3 % (the client 1,238 B, the
   adapter 7,445 B, `lib.rs` 334 B, `indi_finder.rs` 13,217 B, `finder_script/mod.rs`
   6,775 B); US2 31,660 B, 48.3 % (`config.rs` 6,860 B, `main.rs` 1,661 B, the README
   4,882 B, `indi_finder_node.rs` 18,257 B). Built from this text alone by probe round
   1, each from its base: US1 30,690 B (46.8 %) and US2 31,019 B (47.3 %), 1.06 and
   0.98 times the drafting probe's; the 1.35 times a factory build has run over its
   probe on earlier specs did not happen. With the cases the fold of probe round 1 added
   (`probe/045-r1` at 21c62324, each story's files): US1 31,804 B (48.5 %), US2
   31,844 B (48.6 %), each about 7,500 B under 39,321 B. Keep to § Interfaces. One
   story would be about 60,650 B, 92.5 %.
3. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/045-node-runs-finder-over-indi/` or `.specify/`.
4. **The stand-in server is an existing test file, and the shared test module is US1's.**
   *Check:* `crates/scope-node/tests/support/mod.rs` is not in the diff; the finder script,
   the FITS writer and the test clock live in `crates/scope-node/tests/finder_script/mod.rs`,
   US1's, built with the stand-in's public builders and opening with
   `#![allow(dead_code)]`; US2's diff does not touch it, and each test file declares
   `mod finder_script;` and `mod support;`.
5. **The service keeps capturing.** `finish` fails on an element no `@ expect` took, and a
   service or a node asks for its next frame at once. *Check:* US2-S6 to US2-S9 end their
   scripts with the next exposure's vectors, and wait (100 ms of paused time, or 1 s of
   real time in US2-S6) before `finish`; the wait is not an asserted duration.
6. **A stall must fail, not hang.** Under paused time a service that refuses its own
   settings loops forever, its clock racing (the draft probe's mutation of the gain floor
   ran the climb from the floor into nextest's 5-minute timeout). *Check:* every read of
   a star frame in US2-S7 to US2-S9 is wrapped in `tokio::time::timeout` of 60 s of
   paused time, unwrapped.
7. **An oracle copied from the code proves nothing.** *Check:* expected pixels, exposures,
   gains, ranges and messages are literals or the scripts' own values; no test calls
   `gray`, `FRAME_MS` or `check_range` for an expected value.
8. **clippy 1.100 refuses `chunks_exact(2)`** (`chunks_exact_to_as_chunks`). *Check:* the
   high bytes are taken as § Interfaces gives them.
9. **Real time in US2-S6.** The built node runs on the wall clock, and the stand-in's
   `@ expect after` asserts paused instants exactly. *Check:* US2-S6's script has no
   `after` on an `@ expect`; its `@ send after` lines only wait.
10. **An unused helper or import fails clippy.** *Check:* each test file holds only what
    its own tests use.
11. **rustdoc denies warnings.** *Check:* no doc comment holds an angle bracket outside
    backticks (`IndiFinder<C>` is written in backticks).
12. **Each worktree builds on its own.** *Check:* no story sets `CARGO_TARGET_DIR`.
13. **The lockfile does not move.** *Check:* every cargo command runs with `--locked`; the
    tests' `base64`, `tonic` and `tokio` are the node's dependencies already.
14. **Signals only to the test's own child.** *Check:* US2's binary tests end the node
    with their own `Child`'s kill, with `kill_on_drop` set in US2-S6.
15. **The scratch files must not collide.** *Check:* each config file under
    `CARGO_TARGET_TMPDIR` names its test case and the process id.
16. **The device's name is the test's.** *Check:* no test or assertion holds
    `ZWO CCD ASI120MM Mini`; the finder camera's device is `Finder CCD`, or the
    stand-in's `CCD Simulator`.
17. **config.rs's own tests stay as they are.** *Check:* the diff adds nothing inside
    `#[cfg(test)] mod tests` (`crates/scope-node/src/config.rs:346`).
18. **The star field is the simulated camera's, taken first.** *Check:* US2-S9 takes
    `SyntheticCamera`'s frame at default settings before it builds the finder rig, so the
    test clock's readings start at the rig, and asserts the 25 centroids as a condition
    of the scenario, never a count it computed.
19. **The terms are GLOSSARY.md's.** *Check:* every message and README sentence says
    "finder camera", never "finder" alone, and "star frame" for what the node sends.

## Risks

| Risk | Mitigation |
|---|---|
| The real device program defines the ASI120MM otherwise than the finder script (other `CCD_CONTROLS` elements, no `Offset`, another range). | The adapter reads whatever the device defines and sends every `CCD_CONTROLS` element back at its value (030's FR-024); the bench check's step 1 reads it; owner decision 7's swap pins it. |
| Automatic gain's steps, fine near 1 and coarse near the top of a logarithmic scale, hunt near the top. | Fixed gain, as the Pi runs today; the field check notes where automatic settles. |
| The download on the Pi is longer than estimated. | § Frame timing gives each error per 0.1 s of it; the bench check measures it. |
| An imaging frame on the one device program's stream delays the finder camera's `start_s` by up to about 1 s. | § Frame timing states the cost; bench step 4 measures it. |
| A plug-in restarts `indiserver` during an imaging run. | Owner decision 6; the finder camera recovers by itself, no frame at a gain it did not send (US1-S7, US2-S7, US2-S9); the imaging camera's exposures resume at the device program's own settings, the owner's follow-up. |
| The node exits in a loop while the finder camera is unplugged, taking the mount with it. | Owner decision 8 says so; the README says it; with `StartLimitIntervalSec=0` systemd keeps starting it, where its default would leave it failed after 5 starts within 10 s; `adapter = "simulated"` serves the mount without a camera. |

## Refinement Record

Drafted on 2026-11-25 by the plate-solver operator seat's spec session against `main` at
be7f3ff6, from a probe of that day (a trial build of both stories, the leads as runs and
one run against a real `indiserver`), a reading of the node's camera port, service,
capture loop, auto-exposure, config and binary, the INDI client and its tests, the
console's uses of a frame's times, 013's, 030's and 048's trios and the constitution.
Folded the same day with cynical review round 1 and the owner's answers, from the fold's
commits on the probe, and then with probe round 1's nine clarity defects, from the
fold's commits on `probe/045-r1` (§ What was run and what was read). Writes come from
the task text.
Shared writes name the merge edge that orders them, and the spec story that last wrote
the same file on `main`. The two stories share no file: US2 uses US1's shared test
module and does not write it.

The eight owner decisions of spec.md are answered, each as recommended and as review
round 1 amended it (spec § Clarifications, the owner's answers); the record is written to
those answers and its `open_questions` are empty. This record approves nothing: the
owner approvals of spec.md are given by the flip.

```yaml
version: 1
reviewed_revision: be7f3ff686adc431878819f7e30ee56a9c5d48ea
stories:
  US1:
    tests:
      - path: crates/scope-node/tests/indi_finder.rs
        status: new
        disposition: add
        assertion: "the finder rig naming Finder CCD over the finder script at gain 0: info is model Finder CCD, 1280 by 960, one mode 1280x960 at 1 fps as modes and active_mode, exposure_min_ms 0.032 and exposure_max_ms 2000000 from the answer's range, gain_min 1 and gain_max 100; the script plays to its end"
        decision: "spec.md:US1-S1"
      - path: crates/scope-node/tests/indi_finder.rs
        status: new
        disposition: add
        assertion: "over the finder script at gain 0, the gain step 1 and an exposure of 1 s with the worked 16-bit frame, a frame with default settings: pixels 0x00 0x00 0x00 0x01 0x12 0x80 0xFF 0xFF, 4 by 2, exposure_ms 1000, gain 1, start_s reading(400) and end_s reading(400) plus 1.0 exactly; the script, with the gain vector before the exposure, plays to its end"
        decision: "spec.md:US1-S2"
      - path: crates/scope-node/tests/indi_finder.rs
        status: new
        disposition: add
        assertion: "over the finder script at gain 50, the gain step 30, two exposures of 0.5 s with the 8-bit ramp, the gain step 30 and a third such exposure: two frames at exposure_ms 500 and gain 30, then restart and a third; each frame's pixels 0 1 2 253 254 255, exposure_ms 500, gain 30; the script, holding a gain step before the first and the third exposures only, plays to its end"
        decision: "spec.md:US1-S3"
      - path: crates/scope-node/tests/indi_finder.rs
        status: new
        disposition: add
        assertion: "over the finder script at gain 0, apply refuses gain 101 with gain 101 is outside the camera's range 1–100, gain 0.5 with gain 0.5 is outside the camera's range 1–100, and exposure_ms 0.01 with exposure_ms 0.01 ms is outside the camera's range 0.032–2000000 ms; nothing is sent after enableBLOB and the script plays to its end"
        decision: "spec.md:US1-S4"
      - path: crates/scope-node/tests/indi_finder.rs
        status: new
        disposition: add
        assertion: "discovery fails with the INDI camera ZWO CCD ASI071MC Pro is a colour camera (RGGB); the finder camera must be mono over the colour script; with camera has no 640x480 mode; it lists 1280x960 over the finder script at mode 640x480; with camera has invalid exposure_ms range over the finder script whose answer reads min 3000 max 2000; and with cannot reach the INDI server at 127.0.0.1:7624: no script left to play over no script; the scripts play to their end"
        decision: "spec.md:US1-S5"
      - path: crates/scope-node/tests/indi_finder.rs
        status: new
        disposition: add
        assertion: "discovery naming Other CCD over the finder script made @ device Other CCD without its CONNECT and enableBLOB expectations fails exactly 5 s after it began with the INDI server at 127.0.0.1:7624 has no device Other CCD; the script plays to its end"
        decision: "spec.md:US1-S6"
      - path: crates/scope-node/tests/indi_finder.rs
        status: new
        disposition: add
        assertion: "two scripts, the first closing after the next exposure's two vectors, the second at gain 0 with an exposure, the gain step 30 and an exposure: three frames at exposure_ms 500 and gain 30; the first at gain 30, the second fails with the finder camera exposed at gain 0, not at the gain 30 sent, the third at gain 30; 2 connections; both scripts play to their end"
        decision: "spec.md:US1-S7"
      - path: crates/scope-node/tests/indi_finder.rs
        status: new
        disposition: add
        assertion: "IndiCamera describe naming Finder CCD over the finder script gives exposure_ms 0.032 to 2000000; over the finder script whose answer's max 2000 reads max x it gives 0.032 to 3600000, the defined max kept; both scripts play to their end"
        decision: "spec.md:US1-S8"
      - path: crates/scope-node/tests/indi_finder.rs
        status: new
        disposition: add
        assertion: "the finder rig naming CCD Simulator over the mono script, its @ device CCD Simulator put on a line of its own, whose first send block starts with the colour script's first block, and the imaging IndiCamera naming ZWO CCD ASI071MC Pro over the colour script whose first send block starts with the mono script's, each on its own stand-in: a frame and a 1000 ms light exposure made at once both answer 1100 ms after they began, the finder camera's pixels six 7s and the imaging samples six 9s, each description names its own device, and both scripts, whose @ device lines name their own device, play to their end"
        decision: "spec.md:US1-S9"
      - path: crates/scope-node/tests/indi_finder.rs
        status: new
        disposition: add
        assertion: "two finder rigs naming Finder CCD, one over the finder script at gain 0 with no Gain element in CCD_CONTROLS then an exposure of 1 s with the worked 16-bit frame, one over the finder script whose Gain is 0 of 0-0.5: a frame with default settings from the first; each info gives gain_min 1 and gain_max 1, the frame's gain is 1, and both scripts, the first with no gain step, play to their end"
        decision: "spec.md:US1-S10"
      - path: crates/scope-node/tests/indi.rs
        status: existing
        disposition: preserve
        assertion: "the colour and mono scripts are connected, chosen by name or camera bit, described and reported exactly as their tables say; their set vectors carry no range"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/indi_expose.rs
        status: existing
        disposition: preserve
        assertion: "exposures answer the worked frames with their times and region, are retried once and fail twice as 030 words it, and the saved simulator capture plays to its end with its expected exposure, gain, offset and binnings"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/indi_settings.rs
        status: existing
        disposition: preserve
        assertion: "applies send their vectors in order and answer the settings in force, the neutral controls set on each connection"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/raw_frame.rs
        status: existing
        disposition: preserve
        assertion: "the simulated finder camera's raw pixels are what detected its streamed stars"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the camera port, the imaging port, IndiCamera's constructors, CameraInfo and the scope link's protocol keep their shapes"
      - "a set vector's value is taken as before, and a defNumber still defines the range (030 FR-022)"
      - "the rpicam and simulated adapters, the service, the capture loop and auto-exposure are unchanged (013 FR-004, FR-015)"
      - "crates/scope-node/tests/support/mod.rs is used, not edited"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test indi_finder"
        - "cargo nextest run -p scope-node --locked --test indi --test indi_expose --test indi_settings"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, a new Pi kit and the bench and field checks of plan.md § Outside the factory"
    writes:
      - crates/scope-node/src/imaging/indi/mod.rs
      - crates/scope-node/src/indi_finder.rs
      - crates/scope-node/src/lib.rs
      - crates/scope-node/tests/finder_script/mod.rs
      - crates/scope-node/tests/indi_finder.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/src/imaging/indi/mod.rs
        other_spec: 030-imaging-camera-link
        other_story: US6
        disposition: merge_order
        reason: "045 depends_on_landed 030, whose US6 last changed the INDI client (58aff47c)"
      - path: crates/scope-node/src/lib.rs
        other_spec: 044-node-stops-mount-before-exit
        other_story: US1
        disposition: merge_order
        reason: "045 depends_on_landed 044, whose US1 last changed lib.rs (6fb836c9)"
    open_questions: []
  US2:
    tests:
      - path: crates/scope-node/tests/indi_finder_node.rs
        status: new
        disposition: add
        assertion: "Config::parse of adapter = 'indi' alone gives Adapter::Indi and indi None; with [indi] address 127.0.0.1:7700, device Finder CCD and neutral CCD_CONTROLS.Offset 20, indi holds that address and device and the one neutral control CCD_CONTROLS Offset 20"
        decision: "spec.md:US2-S1"
      - path: crates/scope-node/tests/indi_finder_node.rs
        status: new
        disposition: add
        assertion: "parse refuses [indi] with adapter rpicam with the [indi] table applies only to adapter = \"indi\", an empty indi.device with indi.device must not be empty, neutral key Offset with indi.neutral key Offset must be PROPERTY.ELEMENT, and neutral CCD_CONTROLS.Offset nan with indi.neutral CCD_CONTROLS.Offset must be a finite number"
        decision: "spec.md:US2-S2"
      - path: crates/scope-node/tests/indi_finder_node.rs
        status: new
        disposition: add
        assertion: "adapter indi with [imaging] indi on the same server: finder camera named and imaging not, and imaging named and finder camera not, each refused with the finder camera and the imaging camera share the INDI server at 127.0.0.1:7624: set indi.device and imaging.device; both Same CCD refused with indi.device and imaging.device both name Same CCD: the finder camera and the imaging camera need a device each; imaging at 127.0.0.1:7625, devices A and B, and imaging simulated each parse"
        decision: "spec.md:US2-S3"
      - path: crates/scope-node/tests/indi_finder_node.rs
        status: new
        disposition: add
        assertion: "the built node with --check-config on adapter indi, exposure_ms 1000, gain 30, [indi] device Finder CCD and neutral CCD_CONTROLS.Offset 20 exits 0 printing adapter = \"indi\" and \\n[indi]\\naddress = \"127.0.0.1:7624\"\\ndevice = \"Finder CCD\"\\nneutral = { \"CCD_CONTROLS.Offset\" = 20 }\\n and not test-token; a simulated config prints no [indi]"
        decision: "spec.md:US2-S4"
      - path: crates/scope-node/tests/indi_finder_node.rs
        status: new
        disposition: add
        assertion: "the built node on adapter indi with [indi] address a 127.0.0.1 port where nothing listens and [imaging] adapter indi at another such port exits within 5 s with status 1, its last stderr line scope-node: cannot reach the INDI server at 127.0.0.1:<port>: Connection refused (os error 111), port being the [indi] port"
        decision: "spec.md:US2-S5"
      - path: crates/scope-node/tests/indi_finder_node.rs
        status: new
        disposition: add
        assertion: "the built node at exposure_ms 100 and gain 90 with [indi] device CCD Simulator and neutral CCD_OFFSET.OFFSET 20 over a TCP bridge to the mono script whose first send block starts with the colour script's first block, with no timed expect, the neutral vector CCD_OFFSET OFFSET=20 answered, an exposure of 0.1 s answered with the 8-bit ramp and the next exposure expected: Describe gives CCD Simulator, 1280 by 1024, one active mode 1280x1024 at 1 fps, exposure 10 to 3600000, gain 1 to 300; the first star frame within 10 s is 3 by 2, exposure_ms 100, gain 90, end_s 0.1 after start_s within 1e-9; after 1 s and the kill the script, with the neutral vector and no gain vector, plays to its end"
        decision: "spec.md:US2-S6"
      - path: crates/scope-node/tests/indi_finder_node.rs
        status: new
        disposition: add
        assertion: "the service rig at exposure_ms 500 and gain 30 over a connector refusing dials 2, 3 and 4 with ConnectionRefused: star frame 1 is id 1, gain 30, start_s reading(400); star frame 2 is id 2, gain 30, start_s reading(5300); 5 dials; each read within 60 s of paused time; both scripts play to their end"
        decision: "spec.md:US2-S7"
      - path: crates/scope-node/tests/indi_finder_node.rs
        status: new
        disposition: add
        assertion: "the service rig with default settings over the finder script at gain 0 with gain steps 1 4 16 64 100 each before an exposure of 1 s with the flat frame, then the next exposure's two vectors: five star frames, each read within 60 s of paused time, have exposure_ms and gain 1000 and 1, 1000 and 4, 1000 and 16, 1000 and 64, 1000 and 100; after 100 ms the script, whose sixth exposure has no gain step, plays to its end"
        decision: "spec.md:US2-S8"
      - path: crates/scope-node/tests/indi_finder_node.rs
        status: new
        disposition: add
        assertion: "the service rig at exposure_ms 500 with automatic gain over two scripts of the star field, the first the gain step 1, an exposure and the next exposure's two vectors then close, the second an exposure, the gain step 1, an exposure and the next two vectors: two star frames, each read within 60 s of paused time, each with at least 25 centroids, id 1 at gain 1 and start_s reading(400), id 2 at gain 1 and start_s reading(4000); after 100 ms both scripts play to their end"
        decision: "spec.md:US2-S9"
      - path: crates/scope-node/tests/cli.rs
        status: existing
        disposition: preserve
        assertion: "check-config of a simulated config prints every setting with the token masked, and bad keys are refused naming the key"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/imaging_config.rs
        status: existing
        disposition: preserve
        assertion: "the [imaging] table parses with its defaults and neutral controls, is refused with its exact texts, and check-config prints it or says none is configured"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/rpicam_node.rs
        status: existing
        disposition: preserve
        assertion: "the built node on adapter rpicam frames the Pi 5 and Pi 3B+ captures at their strides and refuses an unknown pipeline handler"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the rpicam and simulated adapters start, serve and print under --check-config as before (013 FR-007)"
      - "the [imaging] table, its code, its texts and the imaging adapter's construction are unchanged (030 FR-020, FR-025)"
      - "config.rs's own unit tests are unchanged"
      - "US1's indi_finder.rs and finder_script/mod.rs are used, not edited"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test indi_finder_node"
        - "cargo nextest run -p scope-node --locked --test cli --test imaging_config --test indi_finder"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, a new Pi kit and the bench and field checks of plan.md § Outside the factory"
    writes:
      - crates/scope-node/README.md
      - crates/scope-node/src/config.rs
      - crates/scope-node/src/main.rs
      - crates/scope-node/tests/indi_finder_node.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/src/config.rs
        other_spec: 048-node-detects-stars-on-a-dark-sky
        other_story: US1
        disposition: merge_order
        reason: "045 depends_on_landed 048, whose US1 last changed config.rs (ce7f675a)"
      - path: crates/scope-node/src/main.rs
        other_spec: 044-node-stops-mount-before-exit
        other_story: US1
        disposition: merge_order
        reason: "045 depends_on_landed 044, whose US1 last changed main.rs (6fb836c9)"
      - path: crates/scope-node/README.md
        other_spec: 048-node-detects-stars-on-a-dark-sky
        other_story: US2
        disposition: merge_order
        reason: "045 depends_on_landed 048, whose US2 last changed the node's README (75cb2c8c)"
    open_questions: []
```
