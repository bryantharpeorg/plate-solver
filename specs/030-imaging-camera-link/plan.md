# Implementation Plan: The Imaging Camera on the Scope Link

**Branch**: `030-imaging-camera-link` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

This plan records every design decision for the feature. A story that would change one
escalates rather than re-deciding it.

## Summary

The scope node gains the imaging side of the camera port: a trait for one exposure on
request, a stand-in camera, and an adapter that is a client of an INDI server on the same
machine and sets the controls the config names neutral on each connection. A new
scope-link service, `ScopeNodeImaging`, carries the camera's description, status and
settings, and answers an exposure with a header, which carries the frame's region of
interest, and chunks of at most 256 KiB. The service is written to be served on a
connection of its own, so that frame bytes never share the heartbeat's. The simulator
gains an imaging camera that looks along the main telescope's axis, and `SimNode`
attaches it. The scope node can also describe and time the camera by itself
(`--probe-camera`) and keep its own copy of each frame. The console's side of all this
is `031-imaging-frames-on-console`, which builds on this trio and which this trio
never reads.

## Technical Context

**Language/Version**: Rust (MSRV 1.96).

**Primary Dependencies**: two new ones, for `scope-node` only, both pure Rust and both
named for the owner (spec owner approval 3): `quick-xml` `=0.42.0` (MIT; its MSRV is
1.86; default features are empty; the feature `async-tokio` is taken) and `base64`
`=0.23.1` (MIT OR Apache-2.0; already in `Cargo.lock` for the console,
`Cargo.lock:241`). Each is added by the story whose code first uses it, `quick-xml` by
US4 and `base64` by US5, because the merge queue's `cargo-machete` refuses a dependency
nothing uses (`.github/workflows/ci.yml:410`). No FITS crate.

**Storage**: on the scope node, `frame-<id>.fits` under `save_dir`, when asked. Nothing
else is kept: the camera keeps its own settings.

**Testing**:
- US1: `crates/scope-node/tests/imaging_port.rs` (new).
- US2: `crates/simulator/tests/imaging.rs` (new).
- US3: `crates/scope-node/tests/imaging_service.rs` and
  `crates/simulator/tests/imaging_link.rs` (new).
- US4: `crates/scope-node/tests/indi.rs`, `crates/scope-node/tests/imaging_config.rs`
  and the stand-in INDI server, `crates/scope-node/tests/support/mod.rs` (new; a folder
  under `tests/` is not a test binary, as `crates/simulator/tests/support/mod.rs`
  already shows, and like that file it starts `#![allow(dead_code)]`).
- US5: `crates/scope-node/tests/indi_expose.rs` (new), reusing that stand-in unchanged,
  and the two operator-staged fixture files.
- US6: `crates/scope-node/tests/imaging_save.rs` (new).
- US7: `crates/scope-node/tests/imaging_probe.rs` (new).
- Every test here is a new integration-test file of `scope-node` or `simulator`, each
  its own test binary; none is a finder test, so none needs a `mod` line in an existing
  file.
- A test's files go where spec.md's shared setup says (**Scratch folders**): under
  `CARGO_TARGET_TMPDIR`, removed on drop, never in a path built from
  `CARGO_MANIFEST_DIR` and `../../target`.
- Every Rust test but the tests of the built `scope-node` (US4-S7, US6-S4 and US7-S3)
  runs in paused tokio time over in-memory streams, so every timeout in this plan is
  asserted exactly and none is waited for in real time.

**Target Platform**: the scope node (a small Linux computer on the telescope, built as a
static binary). The simulator's part runs wherever the tests and `finder --simulate` do.

**Constraints**:
- No `unsafe` (`crates/scope-node/src/lib.rs:2`, `crates/simulator/src/lib.rs:2`).
- The scope node does no pixel work: a FITS container's big-endian samples become the
  link's little-endian samples, and nothing else touches them.
- Nothing is keyed to one camera model.
- Rendering stays cheap: the simulated imaging camera is 1236 × 822 by default and no
  test renders the 4944 × 3284 size (US3-S8 carries that size as bytes of a pattern).
- Budgets are things a test or the probe measures; this plan states none for the real
  camera.

**Scale/Scope**: two new module trees, one additive scope-link service, two node flags,
one config table with a sub-table.

## Constitution Check

*GATE: checked at the start of every story.*

| Principle | How this plan honours it |
|---|---|
| I. Agreeing with cedar is correctness | Nothing touches the solver, star detection or `crates/math-core`; the finder camera's path is unchanged. Behaviour here is grounded in the cited INDI documents and FITS rules (`docs/research/2026-11-15-imaging-camera.md` § 2. INDI) or in the simulator's truth; the simulator's own constants are declared as such. |
| II. Never weaken a gate | No existing test, golden, fixture, `scripts/ci/`, `ci.yml`, `deny.toml`, `CODEOWNERS` or `ergane.yaml` is edited. Every test of this trio is a new file, and no existing test or gate file gains a line. |
| III. Tests first, independent expectations | Oracles: the stand-in camera's sample rule, worked examples (FITS bytes, chunk counts), the simulator's truth (itself checked against the existing sky functions), INDI's documents through hand-written scripts, and one saved capture with values read from it by a separate program. Tests read FITS files with code of their own. Clocks are injected; virtual time only in tests. |
| IV. Critical paths | No `unsafe`. Two dependency changes, `quick-xml` in US4 and `base64` in US5, each named for the owner and made by the story that uses the crate. One scope-link change, additive, in `crates/scope-link/proto`, named for the owner. Nothing under `proto/`, no database-format change, no `build.rs` change. |
| V. Proven open source first, honest licensing | The finder camera's solver is untouched. INDI is reached over a socket and nothing of it is linked or copied; both new crates are permissively licensed. |

## Project Structure

```text
crates/scope-node/
├── Cargo.toml                          # US4: quick-xml; US5: base64
├── README.md                           # US4: ## The imaging camera; US5, US6, US7: its paragraphs
├── src/lib.rs                          # US1: pub mod imaging
├── src/imaging/mod.rs                  # US1: new, the port; US3, US4, US5, US6, US7: one pub mod line each
├── src/imaging/synthetic.rs            # US1: new, the stand-in camera
├── src/imaging/service.rs              # US3: new, ScopeNodeImaging; US6: saving
├── src/imaging/indi/mod.rs             # US4: new, the adapter, its neutral controls; US5: exposures, the frame's roi
├── src/imaging/indi/protocol.rs        # US4: new, INDI elements in and out
├── src/imaging/fits.rs                 # US5: new, reading a FITS BLOB
├── src/imaging/save.rs                 # US6: new, the node's own FITS file
├── src/imaging/probe.rs                # US7: new
├── src/service.rs                      # US3: with_imaging, the route, ready() visibility; US6: with_imaging_save_dir
├── src/config.rs                       # US4: [imaging] with neutral; US6: save_dir
├── src/main.rs                         # US4: build and attach, --check-config; US6: save_dir; US7: --probe-camera
├── tests/imaging_port.rs               # US1: new
├── tests/imaging_service.rs            # US3: new
├── tests/support/mod.rs                # US4: new, the stand-in INDI server
├── tests/indi.rs                       # US4: new
├── tests/imaging_config.rs             # US4: new
├── tests/indi_expose.rs                # US5: new
├── tests/imaging_save.rs               # US6: new
└── tests/imaging_probe.rs              # US7: new
crates/scope-link/proto/scope_node/v1/scope_node.proto   # US3: ScopeNodeImaging, the header's roi among its fields (adds only)
crates/simulator/
├── src/lib.rs                          # US2: pub mod imaging, exports
├── src/imaging.rs                      # US2: new
├── src/optics.rs                       # US2: imaging optics in the state, truth
├── src/noise.rs                        # US2: uniform() becomes pub
├── src/node.rs                         # US3: SimNode attaches the imaging camera
├── tests/imaging.rs                    # US2: new
└── tests/imaging_link.rs               # US3: new
Cargo.lock                              # US4, US5
fixtures/scope-node/indi/               # operator-staged before the flip; no story writes it
```

## Interfaces

### The port (`crates/scope-node/src/imaging/mod.rs`, new, US1)

```rust
//! The imaging side of the camera port (030): one exposure on request.
use std::{fmt, future::Future, pin::Pin};

pub mod synthetic;

/// A colour sensor's mosaic, named by its top-left 2 × 2 cell row by row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BayerPattern { Rggb, Bggr, Grbg, Gbrg }
impl BayerPattern {
    /// `"RGGB"`, `"BGGR"`, `"GRBG"` or `"GBRG"`.
    pub fn as_str(self) -> &'static str;
    /// The inverse, ignoring case and surrounding spaces; anything else is `None`.
    pub fn parse(text: &str) -> Option<Self>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range { pub min: f64, pub max: f64 }

/// A region of the sensor in unbinned pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Roi { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }

/// What a camera says about itself. Nothing here comes from a model's name.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagingInfo {
    pub model: String,
    pub serial: Option<String>,
    pub width: u32,
    pub height: u32,
    pub pixel_um: f64,
    /// Bits a sample as delivered: 8 or 16.
    pub bits: u32,
    /// `None` for a mono sensor.
    pub bayer: Option<BayerPattern>,
    pub exposure_ms: Range,
    /// `None` when the camera has no such control.
    pub gain: Option<Range>,
    pub offset: Option<Range>,
    pub binnings: Vec<u32>,
    pub cooler: bool,
}

/// In a request, an absent field leaves that setting alone. In an answer every field
/// the camera has is present; `gain` and `offset` are `None` only without the control,
/// `set_temp_c` only before one was set.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImagingSettings {
    pub gain: Option<f64>,
    pub offset: Option<f64>,
    pub binning: Option<u32>,
    pub roi: Option<Roi>,
    pub cooler_on: Option<bool>,
    pub set_temp_c: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImagingStatus {
    pub temp_c: Option<f64>,
    /// Percent.
    pub cooler_power: Option<f64>,
    pub set_temp_c: Option<f64>,
    pub exposing: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Exposure { pub exposure_ms: f64, pub purpose: String }

/// One exposure, its samples exactly as the camera delivered them. `Default` exists so
/// that a test's literal ends with `..Default::default()` and a later field breaks none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImagingFrame {
    /// Little-endian, `width × height × bits / 8` bytes, row by row from the top-left.
    pub samples: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub bits: u32,
    pub bayer: Option<BayerPattern>,
    /// Seconds on the node's clock.
    pub start_s: f64,
    pub end_s: f64,
    pub exposure_ms: f64,
    pub gain: Option<f64>,
    pub offset: Option<f64>,
    pub binning: u32,
    pub temp_c: Option<f64>,
    /// The region of interest in force for the exposure, in unbinned sensor pixels (the
    /// plan document's Amendment 6). The frame's own `width` and `height` are what it
    /// delivered; this says where on the sensor it sat.
    pub roi: Roi,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImagingError {
    /// The request names a value the camera does not accept; nothing changed.
    Invalid(String),
    /// An exposure is in progress. Displays as `an exposure is already in progress`.
    Busy,
    /// The exposure was aborted on request. Displays as `the exposure was aborted`.
    Aborted,
    /// The camera or its server failed; the text says how.
    Failed(String),
}
// impl fmt::Display (Invalid and Failed display their text), impl std::error::Error

pub type ImagingFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ImagingError>> + Send + 'a>>;

/// Every method takes `&self`, so `status` and `abort` work while `expose` waits.
pub trait ImagingCamera: Send + Sync {
    fn describe(&self) -> ImagingFuture<'_, ImagingInfo>;
    fn status(&self) -> ImagingFuture<'_, ImagingStatus>;
    /// Change the settings present; answer all the settings in force.
    fn apply<'a>(&'a self, settings: &'a ImagingSettings) -> ImagingFuture<'a, ImagingSettings>;
    fn expose<'a>(&'a self, exposure: &'a Exposure) -> ImagingFuture<'a, ImagingFrame>;
    fn abort(&self) -> ImagingFuture<'_, ()>;
}

/// `edit` laid over `in_force` and checked against `info`. The first failing rule,
/// in the order below, answers `Invalid` with its text.
pub fn check_settings(
    info: &ImagingInfo,
    in_force: &ImagingSettings,
    edit: &ImagingSettings,
) -> Result<ImagingSettings, ImagingError>;

pub fn check_exposure(info: &ImagingInfo, exposure: &Exposure) -> Result<(), ImagingError>;
```

`check_settings`, in this order, with `{}` meaning Rust's `Display` of the number (so
`301`, `-1`, `0.5`) and `–` an en dash:

1. `gain` present: without the control, `this camera has no gain control`; not finite or
   outside the range, `gain {value} is outside the camera's range {min}–{max}`.
2. `offset`: the same two texts with `offset`.
3. `binning` present and not in `info.binnings`:
   `binning {value} is not one of the camera's binnings {the list joined by ", "}`.
4. The region, the edit's or the one in force, against the binning b that will be in
   force: `x` and `y` must be multiples of 2·b, `width` and `height` at least b,
   `x + width ≤ info.width` and `y + height ≤ info.height`; otherwise
   `roi must lie inside the {W} × {H} sensor with x and y multiples of {2·b}`.
5. `cooler_on` `Some(true)` or `set_temp_c` present on a camera without a cooler:
   `this camera has no cooler`.
6. `set_temp_c` not finite or outside −50 to 50: `set_temp_c must be between -50 and 50`.

`check_exposure`: `exposure_ms` not finite or outside the range,
`exposure_ms {value} is outside the camera's range {min}–{max} ms`; a purpose that is
not 1 to 16 bytes of `a` to `z`, `purpose must be 1 to 16 lower-case letters`.

On the two stand-in cameras a delivered frame is `roi.width / b` by `roi.height / b`,
each rounded down, and its `roi` is the region in force. That is the stand-ins' rule,
not the port's: a real camera may deliver less (spec FR-003), so nothing downstream
works a frame's size out from the settings; it reads the frame's own `width` and
`height`, and where it sat from the frame's own `roi`.

**During an exposure**, on every adapter: `describe` and `status` answer; `apply` with
an empty edit (`*edit == ImagingSettings::default()`) answers the settings in force;
`apply` with any field present answers `Busy`, before any check, and changes nothing; a
second `expose` answers `Busy`; `abort` ends the exposure with `Aborted`.

`crates/scope-node/src/lib.rs` gains `pub mod imaging;` after `pub mod exposure;`.

### The stand-in camera (`crates/scope-node/src/imaging/synthetic.rs`, new, US1)

```rust
/// A seeded stand-in for `[imaging] adapter = "simulated"` and for tests: no sky.
pub struct SyntheticImagingCamera { /* clock, Mutex<state>, a Notify for abort */ }
impl SyntheticImagingCamera {
    pub fn new(clock: std::sync::Arc<dyn crate::clock::NodeClock>) -> Self;
}
impl ImagingCamera for SyntheticImagingCamera { /* … */ }
```

- **Description**: model `synthetic imaging camera`, no serial, 64 × 48, pixel 4.78,
  16 bits, RGGB, exposure 1 to 60000 ms, gain 0 to 300, offset 0 to 100, binnings
  `[1, 2]`, a cooler.
- **In force at the start**: gain 100, offset 10, binning 1, the region (0, 0, 64, 48),
  cooler off, no set temperature.
- **The sample rule**: with binning b, region origin (x0, y0) and offset o in force,
  sample (c, r) is `256 + 16·round(o) + ((x0/b + c + 3·(y0/b + r)) mod 251)`, as a
  little-endian `u16`.
- **The cooler's rule**, shared with the simulated camera: with the cooler off, or on
  with no set temperature, the sensor reads 20.0 °C and power 0.0; on with a set
  temperature T, it reads T at once and power `clamp(2.5·(20 − T), 0, 100)`.
- **An exposure**: check it, refuse with `Busy` if one runs, mark the camera exposing,
  read the clock for `start_s`, then wait the exposure on tokio time or for an abort,
  whichever comes first. A guard clears the mark on every way out, a dropped future
  included. `end_s` is `start_s + exposure_ms / 1000`, and `roi` the region in force
  when the exposure began. While the mark is set, `apply`
  follows § The port's **During an exposure**: an empty edit answers the settings in
  force, any other answers `Busy`.

### The simulated imaging camera (`crates/simulator/src/imaging.rs`, new, US2)

```rust
//! A 16-bit Bayer camera along the main telescope's axis, with its own seeded sky.
pub const FIELD_SEED: u64 = 20_261_115;
pub const STARS_PER_SQ_DEG: f64 = 320.0;     // brighter than MAG_FAINT
pub const MAG_BRIGHT: f64 = 8.0;
pub const MAG_FAINT: f64 = 16.0;
pub const MAG_SLOPE: f64 = 0.33;             // log10 of the count, per magnitude
pub const ELECTRONS_MAG10_PER_S: f64 = 400_000.0;
pub const SKY_E_PER_S: f64 = 20.0;           // per sample
pub const READ_NOISE_E: f64 = 3.0;
pub const PSF_SIGMA_PX: f64 = 1.2;
pub const FOCUS_SIGMA_PX: f64 = 1.0;         // added in quadrature per unit of focus error
pub const ADU_PER_E_AT_GAIN_0: f64 = 0.25;   // times 10 for every 200 of gain
pub const ADU_PER_OFFSET: f64 = 16.0;

/// The imaging optics. Pixels are (x, y); the field is horizontal, in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImagingOptics {
    pub width: u32,
    pub height: u32,
    pub fov_deg: f64,
    /// The imaging angle: positive turns frame-up toward the finder frame's left.
    pub angle_deg: f64,
    pub focus_error: f64,
    pub pixel_um: f64,
}
// Default: 1236, 822, 0.75, 17.0, 0.0, 19.12

/// Where an imaging frame points, independent of any solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImagingTruth {
    /// J2000 degrees of the frame's centre.
    pub ra_deg: f64,
    pub dec_deg: f64,
    /// The angle of frame-up east of celestial north, 0 to 360.
    pub up_east_of_north_deg: f64,
    pub arcsec_per_px: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimStar {
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub magnitude: f64,
    /// Relative response in red, green and blue.
    pub colour: [f64; 3],
}

/// The seeded field's stars within `radius_deg` of a place, brightest first.
pub fn deep_field(seed: u64, ra_deg: f64, dec_deg: f64, radius_deg: f64) -> Vec<SimStar>;

pub struct SimImagingCamera { /* simulator, seed, stars, noise, state */ }
impl SimImagingCamera {
    pub fn new(simulator: crate::Simulator) -> Self;      // seed 13, the deep field, noise on
    pub fn with_seed(self, seed: u64) -> Self;
    /// Draw these stars and no others.
    pub fn with_stars(self, stars: Vec<SimStar>) -> Self;
    pub fn with_noise(self, on: bool) -> Self;
    pub fn frames_taken(&self) -> u64;
}
impl scope_node::imaging::ImagingCamera for SimImagingCamera { /* … */ }
```

In `crates/simulator/src/optics.rs`, `Pointing` (`crates/simulator/src/optics.rs:65`)
gains `pub imaging: ImagingOptics`, set to the default where the state is built
(`crates/simulator/src/optics.rs:85`), and `optics::Simulator` gains:

```rust
pub fn imaging_optics(&self) -> ImagingOptics;
pub fn set_imaging_optics(&self, optics: ImagingOptics);   // asserts width, height > 0, 0 < fov < 180
pub fn imaging_truth(&self) -> ImagingTruth;               // at the clock's now
pub fn imaging_truth_at(&self, unix_s: f64) -> ImagingTruth;
pub(crate) fn imaging_rotation_at(&self, unix_s: f64) -> (RotationMatrix, ImagingOptics);
```

They are reached through `controls::Simulator`'s `Deref`, which follows a connected
mount first, as the existing truth is. `Optics` gains no field: tests build it by
literal. `crates/simulator/src/lib.rs` gains `pub mod imaging;` and
`pub use imaging::{ImagingOptics, ImagingTruth, SimImagingCamera, SimStar};`.
`SplitMix64::uniform` (`crates/simulator/src/noise.rs:26`) becomes `pub`.

**The attitude.** With R the finder's sky-to-camera rotation at the instant
(`sky::camera_rotation`, `crates/simulator/src/sky.rs:109`), whose rows are the finder's
boresight, left and up in J2000, and e the eyepiece pixel's ray in the finder's frame
(`crates/simulator/src/optics.rs:42`): the imaging boresight is b = `sky::unrotate(R, e)`;
u0 is the finder's up less its part along b, normalised; l0 = u0 × b; and with α the
imaging angle, up = cos α·u0 + sin α·l0 and left = cos α·l0 − sin α·u0. The imaging
rotation is the `RotationMatrix` with the rows b, left, up, and a star's place is
`PinholeCamera::new(width, height, fov).project` of it rotated, as (x, y). The truth is
b's RA and Dec; the angle `atan2(up·east, up·north)` with north = ẑ less its part along
b, normalised, and east = ẑ × b, normalised; and the scale `2·tan(fov/2)/width` radians.

**The field.** The sky is cut into cells 0.25° high in Dec; band j =
⌊(dec + 90)/0.25⌋ has n_j = max(1, round(360·cos(dec at the band's middle)/0.25)) cells
in RA. A cell's stars come from `SplitMix64::new(seed, j·2²⁰ + i)`: a Poisson count of
mean `STARS_PER_SQ_DEG` times the cell's area in square degrees; then for each star, in
this order, four uniforms: RA across the cell, Dec across it, the magnitude
`8 + log10(1 + u·(10^(0.33·8) − 1))/0.33`, and the colour
`[0.6 + 0.8·u, 1.0, 1.4 − 0.8·u]`. `deep_field` gathers the cells that touch the circle,
keeps the stars inside it, and sorts by magnitude, then RA.

**A frame.** `expose` checks the request, refuses with `Busy` if one runs, and then:

1. reads `start_s` from the simulator clock's `node_now()` and takes the imaging rotation
   R0 and the optics at the clock's `now()`; the settings in force are fixed here, and
   their region is the frame's `roi`;
2. waits the exposure on tokio time, or answers `Aborted`;
3. takes the imaging rotation R1 at the clock's `now()`;
4. takes its stars: the list given to `with_stars`, or `deep_field(FIELD_SEED, …)` about
   the truth's centre at the start, out to half the frame's diagonal plus 0.05°;
5. for each star, its places p0 and p1 under R0 and R1; n = clamp(⌈|p1 − p0|⌉, 1, 64)
   points at `p0 + (k + 0.5)/n·(p1 − p0)`; at each, a Gaussian of width
   σ = √(1.2² + (1.0·focus_error)²) carrying E/n electrons, where
   E = 400,000·10^(−0.4·(m − 10))·seconds. A sample whose centre is at distance d gets
   `(E/n)·colour[its filter]·exp(−d²/(2σ²))/(2πσ²)`, for samples within ⌈4σ⌉ + 1. The
   filter of sensor pixel (x, y) is red at even x and even y, blue at odd and odd,
   green otherwise;
6. adds the sky, `SKY_E_PER_S`·seconds, to every sample;
7. with noise on, adds `√(mean + 3²)·z` to each sample, z one `SplitMix64::normal()` a
   sample in row order from `SplitMix64::new(seed, frame number)`, the number counting
   this camera's frames from 0;
8. turns electrons into counts, `16·offset + electrons·0.25·10^(gain/200)`;
9. at binning 2, averages samples of the same colour. Delivered sample (c, r), for c
   below ⌊width/2⌋ and r below ⌊height/2⌋ of the region, is the mean of those of the
   four region samples `(2·(c − c%2) + c%2 + 2i, 2·(r − r%2) + r%2 + 2j)`, i and j each
   0 or 1, counted from the region's origin, that lie inside the region. A sample
   outside the region is left out of the mean; the one with i = j = 0 is always inside.
   At the default size this matters for the last delivered row only: 822 is not a
   multiple of 4, so row 410 is the mean of two samples of region row 820 (row 822 does
   not exist), and region row 821 reaches no delivered sample;
10. rounds with `f64::round` (half away from zero), clamps to 0 to 65535 and writes
    little-endian.

Only the region of interest is rendered. Its description: model
`simulated imaging camera`, serial `SIM-0001`, the optics' size and pixel, 16 bits, RGGB,
exposure 1 to 3600000 ms, gain 0 to 300, offset 0 to 100, binnings `[1, 2]`, a cooler by
the cooler's rule above. In force at the start: gain 120, offset 10, binning 1, the whole
sensor, cooler off. These constants are the simulator's own, chosen so a default frame is
cheap and has a few stars above the noise in one second; the densities they stand for
are in `docs/research/2026-11-15-imaging-camera.md` § 4. Star counts and limiting
magnitude at this field.

### The scope link (`crates/scope-link/proto/scope_node/v1/scope_node.proto`, US3)

Appended at the end of the file, after `RawFrameReply`
(`crates/scope-link/proto/scope_node/v1/scope_node.proto:156`):

```proto
// The imaging camera (030). Every call requires the session's bearer token.
// A node built before 030 lacks this service and answers UNIMPLEMENTED; a node with
// no imaging camera answers FAILED_PRECONDITION to every call.
service ScopeNodeImaging {
  rpc DescribeImaging(DescribeImagingRequest) returns (ImagingInfo);
  rpc ImagingStatus(ImagingStatusRequest) returns (ImagingStatusReply);
  // An absent field leaves that setting alone. Returns the settings in force.
  rpc ConfigureImaging(ImagingSettings) returns (ImagingSettings);
  // One exposure: one header, then its samples in chunks of at most 262144 bytes.
  rpc Expose(ExposeRequest) returns (stream ExposeEvent);
  rpc AbortExposure(AbortExposureRequest) returns (AbortExposureReply);
}
message DescribeImagingRequest {}
message ImagingStatusRequest {}
message AbortExposureRequest {}
message AbortExposureReply {}
message ImagingRange {
  double min = 1;
  double max = 2;
}
message ImagingInfo {
  string model = 1;
  optional string serial = 2;
  // Unbinned sensor pixels.
  uint32 width = 3;
  uint32 height = 4;
  double pixel_um = 5;
  // Bits a sample as delivered: 8 or 16.
  uint32 bits = 6;
  // "RGGB", "BGGR", "GRBG" or "GBRG"; empty for a mono sensor.
  string bayer = 7;
  ImagingRange exposure_ms = 8;
  // Absent when the camera has no such control.
  ImagingRange gain = 9;
  ImagingRange offset = 10;
  repeated uint32 binnings = 11;
  bool cooler = 12;
}
message ImagingRoi {
  uint32 x = 1;
  uint32 y = 2;
  uint32 width = 3;
  uint32 height = 4;
}
message ImagingSettings {
  optional double gain = 1;
  optional double offset = 2;
  optional uint32 binning = 3;
  ImagingRoi roi = 4;
  optional bool cooler_on = 5;
  optional double set_temp_c = 6;
}
message ImagingStatusReply {
  optional double temp_c = 1;
  // Percent.
  optional double cooler_power = 2;
  optional double set_temp_c = 3;
  bool exposing = 4;
}
message ExposeRequest {
  double exposure_ms = 1;
  // 1 to 16 lower-case letters; "dark" and "flat" choose the camera's frame type.
  string purpose = 2;
  // Forward at most this many bytes of samples a second; 0 means no limit.
  uint32 max_bytes_per_s = 3;
  // Also keep the frame on the node's disk.
  bool save = 4;
}
message ExposeEvent {
  oneof event {
    ImagingFrameHeader header = 1;
    bytes chunk = 2;
  }
}
message ImagingFrameHeader {
  // The node's count of imaging frames, from 1 for each run of the node.
  uint64 id = 1;
  uint32 width = 2;
  uint32 height = 3;
  uint32 bits = 4;
  string bayer = 5;
  // Exposure start and end in seconds on the node's monotonic clock.
  double start_s = 6;
  double end_s = 7;
  double exposure_ms = 8;
  optional double gain = 9;
  optional double offset = 10;
  uint32 binning = 11;
  optional double temp_c = 12;
  // Little-endian samples, row by row: width × height × bits / 8 bytes in all.
  uint64 byte_count = 13;
  string purpose = 14;
  // The node's own copy: its file name, or why there is none; both empty when not asked.
  string node_file = 15;
  string node_save_error = 16;
  // The region of interest in force for the exposure, in unbinned sensor pixels: the
  // frame's own `roi` (the plan document's Amendment 6).
  ImagingRoi roi = 17;
}
```

`crates/scope-link/build.rs` already passes `--experimental_allow_proto3_optional`, so
it does not change.

### The node's service (`crates/scope-node/src/imaging/service.rs`, new, and `crates/scope-node/src/service.rs`, US3)

```rust
/// The most bytes of samples in one chunk.
pub const CHUNK_BYTES: usize = 262_144;

/// The node's one imaging camera and its frame count. Its fields are private to this
/// module; `crates/scope-node/src/service.rs` builds it with `new` and keeps it in the
/// runtime, and only this module's five calls reach the camera.
pub(crate) struct ImagingHost { /* camera: Arc<dyn ImagingCamera>, next_id: AtomicU64 (1),
                                  save_dir: OnceLock<PathBuf> (US6) */ }
impl ImagingHost {
    pub(crate) fn new(camera: Arc<dyn ImagingCamera>) -> Self;
    /// US6. The first call sets the folder; a later one changes nothing.
    pub(crate) fn set_save_dir(&self, dir: PathBuf);
}

#[tonic::async_trait]
impl ScopeNodeImaging for NodeService { /* the five calls */ }
```

In `crates/scope-node/src/service.rs`: `Runtime` (`crates/scope-node/src/service.rs:97`)
gains `imaging: std::sync::OnceLock<Arc<ImagingHost>>`; `NodeControls::ready`
(`crates/scope-node/src/service.rs:76`) becomes `pub(crate)`; `routes`
(`crates/scope-node/src/service.rs:196`) adds
`ScopeNodeImagingServer::with_interceptor(self.clone(), self.runtime.token.clone())`;
and `NodeService` gains

```rust
/// Attach the node's imaging camera. Panics with `an imaging camera is already
/// attached` when called twice. No constructor's signature changes.
pub fn with_imaging(self, camera: impl ImagingCamera + 'static) -> Self;
pub(crate) fn imaging_host(&self) -> Option<&Arc<ImagingHost>>;
```

The four unary calls first await `controls.ready(false)`, as `describe` does
(`crates/scope-node/src/service.rs:414`), so a node made unresponsive is silent here
too; then, with no host, they answer `Status::failed_precondition("no imaging camera")`
as the call's status. `expose` is different, so that its caller meets one shape: the
interceptor's token check alone answers as the call's own status (`Unauthenticated`);
after it `expose` always returns `Ok` with a stream, at once, and awaits
`ready(false)` inside its task. Every later failure, a missing host included, is that
stream's first and only item.

- **`describe_imaging`, `imaging_status`, `configure_imaging`** convert to and from the
  port's types and call `describe`, `status` and `apply`. On the wire an empty `bayer`
  is mono, an absent `ImagingRange` is no control, and an absent `roi` is unchanged.
- **Errors**: `Invalid(t)` is `Status::invalid_argument(t)`; `Busy` is
  `Status::resource_exhausted("an exposure is already in progress")`; `Aborted` is
  `Status::aborted("the exposure was aborted")`; `Failed(t)` is `Status::internal(t)`.
- **`expose`** returns at once `Ok` with a `tokio_stream::wrappers::ReceiverStream` over
  a channel of 4 `Result<ExposeEvent, Status>` and spawns a task that owns the sender:
  1. it awaits `controls.ready(false)`; with no host it sends
     `Status::failed_precondition("no imaging camera")` as the one item and ends;
  2. `camera.expose(&Exposure { exposure_ms, purpose })`; an error is sent as the one
     item, mapped as **Errors** says, and the task ends;
  3. the id is taken (`fetch_add`), whether or not anyone still listens;
  4. with `save` true: no save folder gives `node_save_error`
     `this scope node has no save_dir` (US6 writes the file here, before the header);
  5. the header is sent, its `roi` always set from `frame.roi`, then
     `frame.samples.chunks(CHUNK_BYTES)` in order. With
     `max_bytes_per_s` R above 0, before chunk k the task sleeps until the instant the
     first chunk was sent plus (bytes already sent)/R seconds;
  6. a failed send (the caller went away) ends the task; the exposure has already run
     to its end (spec owner decision 11).
- **`abort_exposure`** calls `camera.abort()` and answers `AbortExposureReply {}`.

The task holds one frame's samples once; chunks are copied out one at a time, so a
32 MiB frame costs the node its own 32 MiB plus 256 KiB.

### `SimNode` (`crates/simulator/src/node.rs`, US3)

`SimNode::new` (`crates/simulator/src/node.rs:26`) and `from_camera`
(`crates/simulator/src/node.rs:34`) keep their signatures and attach
`SimImagingCamera::new(simulator.clone())` with `with_imaging`. A third constructor takes
the caller's:

```rust
pub fn from_cameras(
    simulator: Simulator,
    camera: SimCamera,
    imaging: SimImagingCamera,
    token: impl AsRef<[u8]>,
    initial: Settings,
) -> Result<Self, RangeError>;
```

### The config (`crates/scope-node/src/config.rs`, US4; `save_dir`, US6)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImagingAdapter { Indi, Simulated }

/// Optional imaging camera; unknown keys are errors, as everywhere in this file.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImagingConfig {
    pub adapter: ImagingAdapter,
    #[serde(default)]
    pub address: Option<std::net::SocketAddr>,
    #[serde(default)]
    pub device: Option<String>,
    /// `indi` only: controls set to these values on each connection, keyed
    /// `"<property>.<element>"` (spec owner decision 13; the plan document's
    /// Amendment 2). Empty by default.
    #[serde(default)]
    pub neutral: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub save_dir: Option<std::path::PathBuf>,   // US6
}
impl ImagingConfig {
    /// `address`, or 127.0.0.1:7624.
    pub fn address(&self) -> std::net::SocketAddr;
    /// `neutral` as `(property, element, value)`, each key split at its first `.`, in
    /// the table's sorted order. `validate` has refused a key that does not split.
    pub fn neutral_controls(&self) -> Vec<(String, String, f64)>;
    /// What `--check-config` prints after `[imaging]`, one line a key: `adapter`; for
    /// `indi`, `address` (the default included), `device` when set and `neutral` when
    /// not empty; `save_dir` when set (US6). Texts are `key = "value"`; `neutral` is one
    /// inline table, `neutral = { "K1" = v1, "K2" = v2 }`, keys in sorted order, each
    /// quoted with `"` and `\` escaped, values with `Display` (`50`, `52.5`).
    pub fn printed(&self) -> String;
}
// Config gains, after `mount`: #[serde(default)] pub imaging: Option<ImagingConfig>,
```

Worked example (US4-S7): `neutral = { "CCD_CONTROLS.WB_R" = 50, "CCD_CONTROLS.WB_B" = 50 }`
in the file prints `neutral = { "CCD_CONTROLS.WB_B" = 50, "CCD_CONTROLS.WB_R" = 50 }`, and
`neutral_controls()` is `[("CCD_CONTROLS", "WB_B", 50.0), ("CCD_CONTROLS", "WB_R", 50.0)]`.
A TOML integer is read as an `f64` by serde, so `50` and `50.0` are the same.

`Config::parse` (`crates/scope-node/src/config.rs:119`) keeps its error form,
`invalid {key} at line {n}`, and gains one more missing-field note beside the two there
(`crates/scope-node/src/config.rs:137`): `missing field `adapter`` adds
` (missing imaging.adapter)`; the top-level `adapter` has a default, so only the table's
can be missing. `validate` (`crates/scope-node/src/config.rs:154`) adds, in this order:
with `simulated`, an `address` gives `imaging.address applies only to the indi adapter`,
a `device` `imaging.device applies only to the indi adapter` and a non-empty `neutral`
`imaging.neutral applies only to the indi adapter`; an empty `device` gives
`imaging.device must not be empty`; a `neutral` key with no `.`, or with nothing before
or after its first `.`, gives `imaging.neutral key {key} must be PROPERTY.ELEMENT`; a
`neutral` value that is not finite gives `imaging.neutral {key} must be a finite number`;
an empty `save_dir` `imaging.save_dir must not be empty` (US6). The keys are checked in
the table's sorted order and the first failure answers. `masked` does not change.

### The INDI adapter (`crates/scope-node/src/imaging/indi/`, new, US4 and US5)

```rust
// indi/mod.rs
pub trait IndiStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send> IndiStream for T {}
pub type IndiConnect = std::sync::Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = std::io::Result<Box<dyn IndiStream>>> + Send>>
        + Send + Sync,
>;

/// How long each wait is. Tests assert these exactly in paused time.
pub const SETUP_WAIT: Duration = Duration::from_secs(5);     // the device and its CONNECTION
pub const READY_WAIT: Duration = Duration::from_secs(10);    // CCD_EXPOSURE and CCD_INFO
pub const SETTLE: Duration = Duration::from_millis(200);     // quiet before the first sends
pub const ANSWER_WAIT: Duration = Duration::from_secs(5);    // one setting's answer
pub const DOWNLOAD_WAIT: Duration = Duration::from_secs(60); // a frame, after its exposure
pub const RETRY_PAUSE: Duration = Duration::from_secs(1);

/// One control set on each connection: (property, element, value), as
/// `ImagingConfig::neutral_controls` gives them. Empty for no such controls.
pub type NeutralControls = Vec<(String, String, f64)>;

pub struct IndiCamera { /* a sender of commands to the adapter's one task */ }
impl IndiCamera {
    /// `address` is only the text the errors name.
    pub fn new(connect: IndiConnect, address: String, device: Option<String>,
               neutral: NeutralControls, clock: Arc<dyn NodeClock>) -> Self;
    /// Over TCP, with `TcpStream::connect`.
    pub fn tcp(address: SocketAddr, device: Option<String>, neutral: NeutralControls,
               clock: Arc<dyn NodeClock>) -> Self;
}
impl ImagingCamera for IndiCamera { /* each method sends a command and awaits its answer */ }

// indi/protocol.rs
pub struct Element {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub text: String,
    pub children: Vec<Element>,
}
impl Element { pub fn attr(&self, name: &str) -> Option<&str>; }
/// Complete top-level elements of an INDI stream, one at a time; `None` at its end.
pub struct Reader<R> { /* quick_xml::Reader over a tokio BufReader */ }
impl<R: tokio::io::AsyncRead + Unpin> Reader<R> {
    pub fn new(read: R) -> Self;
    pub async fn next(&mut self) -> std::io::Result<Option<Element>>;
}
pub fn get_properties() -> String;
pub fn enable_blob(device: &str) -> String;
pub fn new_switch(device: &str, property: &str, element: &str) -> String;
pub fn new_number(device: &str, property: &str, values: &[(&str, f64)]) -> String;
```

**What the adapter sends**, byte for byte, each ended by a newline, attribute values
escaped (`&amp;`, `&lt;`, `&gt;`, `&quot;`), numbers written with Rust's `Display`
(`0.5`, `2`, `-10`):

```text
<getProperties version="1.7"/>
<enableBLOB device="D">Also</enableBLOB>
<newSwitchVector device="D" name="P"><oneSwitch name="E">On</oneSwitch></newSwitchVector>
<newNumberVector device="D" name="P"><oneNumber name="E1">v1</oneNumber><oneNumber name="E2">v2</oneNumber></newNumberVector>
```

**What it reads.** `def…Vector` and `set…Vector` for numbers, switches, texts and BLOBs,
`delProperty` and `message`; anything else is skipped. It keeps, for its one device, the
latest value, range and state of every property. A number is the element's text parsed
as an `f64` after trimming; a text that does not parse leaves the value as it was. A
switch is `On` or anything else. States are `Idle`, `Ok`, `Busy` and `Alert`.

**One task.** The adapter is one task that owns the connection and the properties, and
a channel of commands; the port's methods send a command and await a oneshot. A second
task reads `Element`s and hands them to the first through a channel, because a
half-read element cannot be dropped and picked up again. The task waits, all at once,
on the next element, the next command and its own deadlines. While an exposure waits,
`describe` and `status` are answered from what it holds, an `apply` with an empty edit
is answered from what it holds, an `apply` with any field present and a second `expose`
answer `Busy` and send nothing, and `abort` is carried out (§ The port, **During an
exposure**).

**Connecting** happens at the first command and again at the first command after the
connection was lost:

1. call the connector; an error is `Failed("cannot reach the INDI server at {address}:
   {error}")`;
2. send `getProperties`;
3. within `SETUP_WAIT`, choose the device: the named one, at its first element; or, with
   none named, the first whose `DRIVER_INFO` text `DRIVER_INTERFACE`, read as a whole
   number n, has the camera bit (`n & 2 != 0`); and see its `CONNECTION` defined. Otherwise
   `Failed("the INDI server at {address} has no device {name}")` or
   `Failed("the INDI server at {address} offers no camera")`;
4. if `CONNECT` is not `On`, send `new_switch(device, "CONNECTION", "CONNECT")`;
5. within `READY_WAIT`, see `CCD_EXPOSURE` and `CCD_INFO` defined, then wait until no
   element has come for `SETTLE`; otherwise
   `Failed("the INDI camera {device} did not come ready within 10 s")`;
6. send `enable_blob(device)`; then, for each of `CCD_COMPRESSION`, `CCD_TRANSFER_FORMAT`
   and `UPLOAD_MODE` that is defined, in that order and whatever its value,
   `new_switch` of `INDI_DISABLED`, `FORMAT_FITS` and `UPLOAD_CLIENT`. No answer is
   awaited;
7. with neutral controls (spec owner decision 13, FR-021): first check every one, in the
   list's order, against what the device has defined; one whose property is not a
   defined number vector, or whose element is not in it, is
   `Failed("the camera has no control {property}.{element}")`, and nothing more is
   sent. Then, for each property in the order it first appears in the list, send one
   `new_number(device, property, values)` whose values are every element of that
   property in the order it was defined, the named ones at their values and every other
   at its latest value; and await the device's next `setNumberVector` of that property
   within `ANSWER_WAIT`, an `Alert` being
   `Failed("the camera refused {property}: {message}")` and no answer
   `Failed("the camera did not answer {property} within 5 s")`, as **`apply`** words
   them. The answer's values become the latest, so a later `apply` sends the neutral
   values back. Whatever is sent in steps 6 and 7 is sent whatever the device's values
   already are.

A failure in any step fails the connection: it is dropped, the command that connected
answers the failure, and the next command connects again from step 1. So a camera whose
`neutral` names a control it lacks answers every call with that text (spec edge
cases), which the console shows as its `error` state (031). Steps 6 and 7 run again on every new connection,
the second try of a failed exposure's included (§ **When it goes wrong**).

**`describe`**:

| Field | From |
|---|---|
| `model` | the device's name |
| `serial` | text `Serial Number`, element `SN`; none when undefined or empty |
| `width`, `height` | `CCD_INFO` `CCD_MAX_X`, `CCD_MAX_Y` |
| `pixel_um` | `CCD_INFO` `CCD_PIXEL_SIZE` |
| `bits` | `CCD_INFO` `CCD_BITSPERPIXEL`; a value other than 8 or 16 fails the `describe` with `Failed("unsupported sample size {n} bits")` |
| `bayer` | `BayerPattern::parse` of text `CCD_CFA` `CFA_TYPE`; none when undefined |
| `exposure_ms` | the minimum and maximum of `CCD_EXPOSURE` `CCD_EXPOSURE_VALUE`, times 1000 |
| `gain` | the range of `CCD_CONTROLS` `Gain`; else of `CCD_GAIN` `GAIN`; else none |
| `offset` | the range of `CCD_CONTROLS` `Offset`; else of `CCD_OFFSET` `OFFSET`; else none |
| `binnings` | every whole number from `CCD_BINNING` `HOR_BIN`'s minimum to its maximum, at most 16 of them; `[1]` when undefined |
| `cooler` | whether `CCD_COOLER` is defined |

**`status`**: `temp_c` is `CCD_TEMPERATURE` `CCD_TEMPERATURE_VALUE`, `cooler_power` is
`CCD_COOLER_POWER` `CCD_COOLER_VALUE`, each none when undefined; `set_temp_c` is the
temperature the adapter last sent; `exposing` is whether an exposure waits. Once the
adapter is connected `status` never waits on the device. The first call of any method
connects first, and fails as **Connecting** says when it cannot. The set temperature is
remembered by this process only: after the scope node restarts it is none, though the
camera's cooler may still be on.

**`apply`** runs `check_settings` against the description and the settings in force,
then sends, for each field present in the edit and in this order, one vector, awaiting
after each the device's next `set…Vector` of that property within `ANSWER_WAIT`:

1. `binning`: `CCD_BINNING` with `HOR_BIN` and `VER_BIN`;
2. `roi`: `CCD_FRAME` with `X`, `Y`, `WIDTH`, `HEIGHT`;
3. `gain` or `offset`: with `CCD_CONTROLS`, one vector holding every element of it in
   the order it was defined, each at its latest value but for the changed ones; without
   it, `CCD_GAIN` `GAIN` and `CCD_OFFSET` `OFFSET`, one vector each. The other elements
   of `CCD_CONTROLS` are the camera maker's own controls, white balance among them: the
   adapter sends them back at their latest values, which for a control the neutral list
   names is the value step 7 of **Connecting** set, and records none of them with a
   frame (spec owner decision 13);
4. `cooler_on`: `CCD_COOLER` with `COOLER_ON` or `COOLER_OFF`;
5. `set_temp_c`, or a remembered one when the cooler was just switched on:
   `CCD_TEMPERATURE` with `CCD_TEMPERATURE_VALUE`, sent only while the cooler is on.

An answer in the state `Alert` is `Failed("the camera refused {property}: {message}")`,
the message being the answer's `message` attribute, or the device's latest `message`
element, or `no reason given`; no answer is
`Failed("the camera did not answer {property} within 5 s")`. Either ends the `apply`.
The answer is the settings in force read back from the properties: `Gain` and `Offset`
(or `GAIN` and `OFFSET`), `HOR_BIN`, `CCD_FRAME`'s four numbers, `COOLER_ON`, and the
remembered set temperature.

In US4 `expose` answers `Failed("exposures arrive with US5")` and `abort` answers `Ok`;
US5 replaces both. US4 decodes no BLOB and does not declare `base64`: US5 adds the
crate with the code that uses it (§ Plan traps 19).

**`expose`** (US5) runs `check_exposure`, answers `Busy` if one waits, and then:

1. when `CCD_FRAME_TYPE` is defined, sends `new_switch` of `FRAME_DARK` for the purpose
   `dark`, `FRAME_FLAT` for `flat`, `FRAME_LIGHT` otherwise, awaiting nothing;
2. sends `new_number(device, "CCD_EXPOSURE", &[("CCD_EXPOSURE_VALUE", seconds)])` and
   notes the clock and the region: `CCD_FRAME`'s latest `X`, `Y`, `WIDTH` and `HEIGHT`,
   each rounded to a whole number, or, when `CCD_FRAME` is undefined, (0, 0,
   `CCD_MAX_X`, `CCD_MAX_Y`);
3. at the first `setNumberVector` of `CCD_EXPOSURE` in the state `Busy`, notes the clock
   as `start_s`; if none comes, `start_s` is the reading of step 2;
4. waits for a `setBLOBVector` of `CCD1`. Its `oneBLOB`'s `format` must be `.fits`, else
   `Failed("the camera sent {format}, not .fits")`; its text, less white space, is
   decoded from standard base64 with the `base64` crate, which this story adds to
   `crates/scope-node/Cargo.toml`, and read as plan.md § Reading a FITS BLOB says, on
   the blocking pool, since a full frame is 43 MB of text;
5. answers the frame: `end_s = start_s + exposure_ms/1000`; `gain` and `offset` from the
   properties; the Bayer pattern from the header's `BAYERPAT`, else `CCD_CFA`; `binning`
   from `XBINNING`, else `HOR_BIN`, else 1; `temp_c` from `CCD-TEMP`, else
   `CCD_TEMPERATURE`; `roi` the region noted in step 2.

**When it goes wrong.** An attempt fails when `CCD_EXPOSURE` comes in the state `Alert`
(`the camera reported a failed exposure`, with `: {message}` when there is one), when no
BLOB has come `exposure + DOWNLOAD_WAIT` after step 2
(`no imaging frame arrived within 60 s of the exposure's end`), or when the stream ends
(`the INDI server closed the connection`). After the first failure the adapter sends
`new_switch(device, "CCD_ABORT_EXPOSURE", "ABORT")` if it is still connected, waits
`RETRY_PAUSE`, connects again if it must, and runs steps 1 to 5 once more, so the frame
type is sent again before the exposure. After the
second it sends the abort again and answers
`Failed("the exposure failed twice: {the second reason}")`. A BLOB that cannot be read
fails at once, with no second try. This is all the adapter adds to INDI's own handling:
it relies on the device program starting an exposure up to three times and restarting a
failed one once, by itself (`docs/research/2026-11-15-imaging-camera.md` § 2. INDI; the
restart count as spec.md's Clarifications give it), and never on that
program's USB reset, which is off unless switched on.

**`abort`**: with an exposure waiting, sends the abort switch and ends the exposure with
`Aborted` at once; a BLOB that comes when none waits is dropped.

### Reading a FITS BLOB (`crates/scope-node/src/imaging/fits.rs`, new, US5)

```rust
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub bits: u32,
    /// Little-endian.
    pub samples: Vec<u8>,
    /// Keyword and value text, quotes and spaces trimmed, in file order.
    pub cards: Vec<(String, String)>,
}
impl Picture { pub fn card(&self, keyword: &str) -> Option<&str>; }
/// One FITS image as a camera sends it.
pub fn read(bytes: &[u8]) -> Result<Picture, String>;
```

The header is 80-byte cards to `END`, padded to a multiple of 2880 bytes; a card's value
starts after `= ` and ends at a ` /` outside quotes. `SIMPLE` must be `T`, else
`the camera's FITS header is not valid` (also when `END` is missing). `NAXIS` must be 2,
else `the camera sent a {n}-axis FITS picture`. `BITPIX` 8 gives the bytes as they are;
`BITPIX` 16 gives, for each big-endian signed sample s, the `u16` `clamp(s + BZERO, 0,
65535)` written little-endian, `BZERO` being 0 when absent; any other is
`unsupported FITS BITPIX {n}`. Fewer data bytes than `NAXIS1 × NAXIS2 × |BITPIX|/8` is
`the camera's FITS data is short`.

**The worked FITS frame** of US5-S1 is 5760 bytes: one header block with the cards
`SIMPLE  =                    T`, `BITPIX  =                   16`,
`NAXIS   =                    2`, `NAXIS1  =                    4`,
`NAXIS2  =                    2`, `BZERO   =                32768`,
`BSCALE  =                    1`, `XBINNING=                    1`,
`YBINNING=                    1`, `CCD-TEMP=                 -9.5`,
`BAYERPAT= 'RGGB    '` and `END`, each padded with spaces to 80 bytes and the block to
2880; then the 16 bytes `80 00 80 01 80 02 80 03 00 00 1C 40 7F FE 7F FF` and zeros to
2880. The test builds it with a helper of its own and sends it as base64.

### The script format (`crates/scope-node/tests/support/mod.rs`, new, US4)

The file's first line is `#![allow(dead_code)]`, as
`crates/simulator/tests/support/mod.rs:1` has it. Each test binary that includes the
module uses only part of it (`fixture` is called by `indi_expose.rs` alone, and US5 may
not edit this file), and an unused item in a test binary's private module is a warning
the gates deny.

A script is UTF-8 text, one directive or one line of the server's own words a line:

```text
// a comment
@ device CCD Simulator
@ expect getProperties
@ send
<defSwitchVector device="CCD Simulator" name="CONNECTION" state="Idle" perm="rw" rule="OneOfMany">
  <defSwitch name="CONNECT">Off</defSwitch><defSwitch name="DISCONNECT">On</defSwitch>
</defSwitchVector>
@ expect newSwitchVector CONNECTION CONNECT=On
@ send after 200
<setSwitchVector device="CCD Simulator" name="CONNECTION" state="Ok"><oneSwitch name="CONNECT">On</oneSwitch></setSwitchVector>
@ expect after 200 enableBLOB text=Also
@ close
```

- A line that starts `//`, and a blank line, are ignored outside a `@ send` block.
- Directives run in order. An `@ expect` *completes* at the stamp of the element it
  took, and a `@ send` when its lines are written. `after MS`, on either, counts from
  the completion of the nearest earlier `@ expect` or `@ send` of the same script, or
  from the moment the connection was made when there is none.
- **Stamps.** A reader task of the stand-in reads the adapter's elements as they
  arrive, whatever directive the script has reached, and stamps each with
  `tokio::time::Instant::now()` as its last byte is read. Under paused time the stamp
  is the virtual instant the adapter sent the element.
- `@ device NAME` (the rest of the line) names the device every later expected element
  that has a `device` attribute must carry.
- `@ expect [after MS] TAG [PROPERTY] [KEY=VALUE …]` takes the adapter's next element,
  the oldest not yet taken, waiting for it if none has come. Its tag
  must be TAG; its `name` attribute PROPERTY, when given; for each pair, a child whose
  `name` is KEY must have that text, compared as numbers when both parse as `f64` and
  as trimmed text otherwise; the pair `text=VALUE` is the element's own text. It must
  hold no child the pairs do not name. Anything else fails the script, naming both.
  With `after MS` the element's stamp must be exactly MS milliseconds after the point
  `after` counts from, else the script fails, naming the two times; without it the
  element may come at any time. This is how a test proves when the adapter sent
  something (US4-S1, US5-S3, US5-S4, US5-S6).
- `@ send [after MS]` sleeps until MS milliseconds of tokio time after the point `after`
  counts from (not at all without `after`, or when that instant has passed), then
  writes the lines up to the next `@ ` line, joined and ended by a newline.
- `@ close` closes the connection. At the end of a script the connection stays open.

```rust
/// The tests' stand-in for an INDI server. It reads the adapter's elements with a
/// scanner of its own: from `<tag` to `/>`, or to `</tag>`.
pub struct Scripts { /* the scripts, one for each connection in turn */ }
impl Scripts {
    pub fn new(scripts: Vec<String>) -> Self;
    /// A connector for `IndiCamera::new`: each call serves the next script over a fresh
    /// `tokio::io::duplex(1 << 20)`; with none left it answers `ConnectionRefused`.
    pub fn connect(&self) -> scope_node::imaging::indi::IndiConnect;
    /// How many connections were made.
    pub fn connections(&self) -> usize;
    /// Panics with the first mismatch, if any script was not played to its end, or if
    /// the adapter sent an element that no `@ expect` took. This last is how a scenario
    /// proves that the adapter "sent nothing" (US4-S4, US4-S8, US5-S6).
    pub async fn finish(self);
}
pub fn fixture(name: &str) -> String;   // reads fixtures/scope-node/indi/<name>
```

### The two worked scripts (US4, US5)

Each `def…Vector` is sent with `state="Idle"` and `perm="rw"`, in the table's order,
numbers as `<defNumber name="E" format="%g" min="…" max="…" step="1">value</defNumber>`.
Values written `v (min–max)`.

**The colour-camera script**, device `ZWO CCD ASI071MC Pro`:

1. `@ expect getProperties`, then send `CONNECTION` (switch: `CONNECT` Off,
   `DISCONNECT` On) and `DRIVER_INFO` (text: `DRIVER_NAME` `ZWO CCD`, `DRIVER_EXEC`
   `indi_asi_ccd`, `DRIVER_VERSION` `2.8`, `DRIVER_INTERFACE` `2`).
2. `@ expect newSwitchVector CONNECTION CONNECT=On`, then send, in one `@ send` block,
   `setSwitchVector` `CONNECTION` `Ok` with `CONNECT` On, and these definitions:

   | Property | Kind | Elements |
   |---|---|---|
   | `CCD_EXPOSURE` | number | `CCD_EXPOSURE_VALUE` 1 (0.000064–2000) |
   | `CCD_ABORT_EXPOSURE` | switch | `ABORT` Off |
   | `CCD_FRAME` | number | `X` 0 (0–4943), `Y` 0 (0–3283), `WIDTH` 4944 (1–4944), `HEIGHT` 3284 (1–3284) |
   | `CCD_BINNING` | number | `HOR_BIN` 1 (1–4), `VER_BIN` 1 (1–4) |
   | `CCD_INFO` | number | `CCD_MAX_X` 4944, `CCD_MAX_Y` 3284, `CCD_PIXEL_SIZE` 4.78, `CCD_PIXEL_SIZE_X` 4.78, `CCD_PIXEL_SIZE_Y` 4.78, `CCD_BITSPERPIXEL` 16 |
   | `CCD_CFA` | text | `CFA_OFFSET_X` `0`, `CFA_OFFSET_Y` `0`, `CFA_TYPE` `RGGB` |
   | `CCD_TEMPERATURE` | number | `CCD_TEMPERATURE_VALUE` 12.5 (−50–50) |
   | `CCD_COOLER` | switch | `COOLER_ON` Off, `COOLER_OFF` On |
   | `CCD_COOLER_POWER` | number | `CCD_COOLER_VALUE` 0 (0–100) |
   | `CCD_CONTROLS` | number | `Gain` 90 (0–240), `Offset` 20 (0–100), `WB_R` 52 (1–99), `WB_B` 95 (1–99) |
   | `CCD_FRAME_TYPE` | switch | `FRAME_LIGHT` On, `FRAME_BIAS` Off, `FRAME_DARK` Off, `FRAME_FLAT` Off |
   | `Serial Number` | text | `SN` `0a1b2c3d4e5f6071` |
   | `CCD1` | BLOB | `CCD1` |

3. `@ expect after 200 enableBLOB text=Also`: the adapter's `SETTLE` runs from the last
   element of step 2's one block, so `enableBLOB` comes exactly 200 ms after that block
   was written.

It defines none of `CCD_COMPRESSION`, `CCD_TRANSFER_FORMAT` and `UPLOAD_MODE`, so a
camera built with neutral controls sends its `newNumberVector`s right after
`enableBLOB` (US4-S8 adds `@ expect newNumberVector CCD_CONTROLS Gain=90 Offset=20
WB_R=50 WB_B=50` and its answer after step 3). A
scenario adds its own expectations and answers after these; an answer to a setting is
the property's `set…Vector` in the state `Ok` with the values asked for, unless the
scenario says otherwise (US4-S3's `CCD_FRAME` answers `WIDTH` 1928, its
`CCD_TEMPERATURE` answers `Busy` with 12.5, and it later sends `CCD_TEMPERATURE` 5.0
and `CCD_COOLER_POWER` 80).

**The mono-camera script**, with no device named by the adapter:

1. `@ expect getProperties`, then send, all in one `@ send` block: for the device
   `Telescope Simulator`,
   `CONNECTION` (`CONNECT` Off) and `DRIVER_INFO` with `DRIVER_INTERFACE` `5`; then, for
   the device `CCD Simulator`, `CONNECTION` (`CONNECT` On), `DRIVER_INFO` with
   `DRIVER_INTERFACE` `22`, and:

   | Property | Kind | Elements |
   |---|---|---|
   | `CCD_EXPOSURE` | number | `CCD_EXPOSURE_VALUE` 1 (0.01–3600) |
   | `CCD_ABORT_EXPOSURE` | switch | `ABORT` Off |
   | `CCD_FRAME` | number | `X` 0, `Y` 0, `WIDTH` 1280 (1–1280), `HEIGHT` 1024 (1–1024) |
   | `CCD_INFO` | number | `CCD_MAX_X` 1280, `CCD_MAX_Y` 1024, `CCD_PIXEL_SIZE` 5.2, `CCD_PIXEL_SIZE_X` 5.2, `CCD_PIXEL_SIZE_Y` 5.2, `CCD_BITSPERPIXEL` 16 |
   | `CCD_GAIN` | number | `GAIN` 90 (0–300) |
   | `CCD_OFFSET` | number | `OFFSET` 0 (0–6000) |
   | `CCD_COMPRESSION` | switch | `INDI_ENABLED` On, `INDI_DISABLED` Off |
   | `UPLOAD_MODE` | switch | `UPLOAD_CLIENT` Off, `UPLOAD_LOCAL` On, `UPLOAD_BOTH` Off |
   | `CCD1` | BLOB | `CCD1` |

2. `@ device CCD Simulator`, `@ expect after 200 enableBLOB text=Also` (step 1's
   definitions are one `@ send` block),
   `@ expect newSwitchVector CCD_COMPRESSION INDI_DISABLED=On`,
   `@ expect newSwitchVector UPLOAD_MODE UPLOAD_CLIENT=On`.

### Recording the conversation (the saved capture; the owner, before the flip)

The saved capture is the INDI project's own camera simulator answering exactly what the
adapter sends for: a named device, `describe`, the region (0, 0, 64, 48), and one
exposure of 1 s with the purpose `frame`. It is recorded once, on any machine with INDI
installed, with this script and nothing of ours: start `indiserver indi_simulator_ccd`
in one terminal, then run the script with the output file's name as its one argument.
It has not been run: the grilling session could not run downloaded programs
(§ Prerequisites 4).

```bash
#!/usr/bin/env bash
set -euo pipefail
OUT=$1; D='CCD Simulator'
exec 3<>/dev/tcp/127.0.0.1/7624
: > "$OUT"; ALL=$(mktemp)
step() {   # step SECONDS 'EXPECT LINE' 'XML TO SEND'
  printf '@ expect %s\n' "$2" >> "$OUT"
  printf '%s\n' "$3" >&3
  printf '@ send\n' >> "$OUT"
  timeout "$1" cat <&3 | tee -a "$ALL" >> "$OUT" || true
  printf '\n' >> "$OUT"
}
defined() { grep -q "name=\"$1\"" "$ALL"; }
sw() { step 2 "newSwitchVector $1 $2=On" "<newSwitchVector device=\"$D\" name=\"$1\"><oneSwitch name=\"$2\">On</oneSwitch></newSwitchVector>"; }
printf '@ device %s\n' "$D" >> "$OUT"
step 3 'getProperties' '<getProperties version="1.7"/>'
sw CONNECTION CONNECT
sleep 2
step 2 'enableBLOB text=Also' "<enableBLOB device=\"$D\">Also</enableBLOB>"
if defined CCD_COMPRESSION;     then sw CCD_COMPRESSION INDI_DISABLED; fi
if defined CCD_TRANSFER_FORMAT; then sw CCD_TRANSFER_FORMAT FORMAT_FITS; fi
if defined UPLOAD_MODE;         then sw UPLOAD_MODE UPLOAD_CLIENT; fi
step 3 'newNumberVector CCD_FRAME X=0 Y=0 WIDTH=64 HEIGHT=48' "<newNumberVector device=\"$D\" name=\"CCD_FRAME\"><oneNumber name=\"X\">0</oneNumber><oneNumber name=\"Y\">0</oneNumber><oneNumber name=\"WIDTH\">64</oneNumber><oneNumber name=\"HEIGHT\">48</oneNumber></newNumberVector>"
if defined CCD_FRAME_TYPE;      then sw CCD_FRAME_TYPE FRAME_LIGHT; fi
step 8 'newNumberVector CCD_EXPOSURE CCD_EXPOSURE_VALUE=1' "<newNumberVector device=\"$D\" name=\"CCD_EXPOSURE\"><oneNumber name=\"CCD_EXPOSURE_VALUE\">1</oneNumber></newNumberVector>"
```

Then the values the test compares are read from the capture by a second program that
shares nothing with the adapter. It takes the script's name as its one argument and
prints the JSON:

```python
#!/usr/bin/env python3
import base64, json, re, struct, sys
import xml.etree.ElementTree as ET
text = "".join(l for l in open(sys.argv[1]) if not l.startswith("@ "))
root = ET.fromstring("<all>" + text + "</all>")
def num(prop, el, attr=None):
    hit = None
    for v in root.iter():
        if v.tag in ("defNumberVector", "setNumberVector") and v.get("name") == prop:
            for n in v:
                if n.get("name") == el and (attr is None or n.get(attr) is not None):
                    hit = float(n.get(attr) if attr else n.text)
    return hit
blob = [b for b in root.iter("oneBLOB")][-1]
fits = base64.b64decode(re.sub(r"\s", "", blob.text))
head = fits[: fits.index(b"END" + b" " * 77) + 80]
card = lambda k: int(re.search(rb"%-8s=\s*(-?\d+)" % k, head).group(1))
w, h, bits, zero = card(b"NAXIS1"), card(b"NAXIS2"), card(b"BITPIX"), card(b"BZERO")
start = (len(head) + 2879) // 2880 * 2880
vals = [v + zero for v in struct.unpack(">%dh" % (w * h), fits[start : start + 2 * w * h])]
json.dump({
    "model": "CCD Simulator",
    "width": num("CCD_INFO", "CCD_MAX_X"), "height": num("CCD_INFO", "CCD_MAX_Y"),
    "pixel_um": num("CCD_INFO", "CCD_PIXEL_SIZE"), "bits": num("CCD_INFO", "CCD_BITSPERPIXEL"),
    "exposure_ms": [1000 * num("CCD_EXPOSURE", "CCD_EXPOSURE_VALUE", "min"),
                    1000 * num("CCD_EXPOSURE", "CCD_EXPOSURE_VALUE", "max")],
    "gain": [num("CCD_GAIN", "GAIN", "min"), num("CCD_GAIN", "GAIN", "max")],
    "offset": [num("CCD_OFFSET", "OFFSET", "min"), num("CCD_OFFSET", "OFFSET", "max")],
    "binnings": list(range(int(num("CCD_BINNING", "HOR_BIN", "min")),
                           int(num("CCD_BINNING", "HOR_BIN", "max")) + 1)),
    "frame": {"width": w, "height": h, "bits": bits,
              "first_samples": vals[:8], "sample_sum": sum(vals)},
}, sys.stdout, indent=1)
```

The two outputs are committed by the owner as
`fixtures/scope-node/indi/ccd-simulator.script` and
`fixtures/scope-node/indi/ccd-simulator.expect.json`, with a line in
`fixtures/scope-node/README.md` giving INDI's version and the date. The capture is
reviewed before it is committed: it holds nothing but the simulator's properties. If
the simulator refuses a 64 × 48 region or needs a star catalogue to finish an exposure,
the region or the simulator's settings are changed in the script and in US5-S8 before
the flip (§ Risks). A capture of the owner's real camera is a field check, made the
same way, and is not needed to flip.

### The binary (`crates/scope-node/src/main.rs`, US4; US6; US7)

`Args` (`crates/scope-node/src/main.rs:18`) is unchanged by US4. After the config is
loaded, `--check-config` prints, after the mount's lines
(`crates/scope-node/src/main.rs:49`), `\n[imaging]\n` and `ImagingConfig::printed()`, or
`\n# no imaging camera configured`. `serve` (`crates/scope-node/src/main.rs:75`) builds
the imaging camera the table names and gives it to the service with `with_imaging`:
`ImagingAdapter::Indi` is `IndiCamera::tcp(address(), device.clone(),
neutral_controls(), clock.clone())` and `Simulated` is
`SyntheticImagingCamera::new(clock.clone())`. With no table nothing
is attached. The adapter connects at its first use, so the node starts whether or not
the INDI server is up. US6 adds `.with_imaging_save_dir(dir)` when `save_dir` is set.

US7 adds two flags after `probe_mount` (`crates/scope-node/src/main.rs:26`):

```rust
/// Describe the imaging camera, take three exposures and print their timing, then exit.
#[arg(long, conflicts_with_all = ["check_config", "probe_mount"])]
probe_camera: bool,
/// The probe's exposure in milliseconds.
#[arg(long, requires = "probe_camera", default_value_t = 1000.0)]
probe_exposure_ms: f64,
```

With `--probe-camera`: load the config; with no `[imaging]` table fail with
`no [imaging] table in the config` (the binary prints `scope-node: ` before it and exits
1, as for every error, `crates/scope-node/src/main.rs:34`); else build the camera as
`serve` does and run `probe::run` to standard output.

### The node's copy (`crates/scope-node/src/imaging/save.rs`, new, US6)

```rust
/// Write one frame as FITS under `dir` and answer the file's name.
pub fn write(dir: &Path, id: u64, frame: &ImagingFrame, model: &str, purpose: &str)
    -> std::io::Result<String>;
```

`NodeService::with_imaging_save_dir(self, dir: PathBuf) -> Self` sets the host's folder
with `ImagingHost::set_save_dir` (doing nothing without an imaging camera). In `expose`,
with `save` true and a folder, step 4 runs `write` on the blocking pool with the model
from `describe()`; `Ok(name)` fills `node_file`, an error fills `node_save_error` with
`could not save the imaging frame: {error}`. `write` makes the folder if it is missing, names the
file `frame-<id as six digits>.fits`, then `frame-<id>-2.fits`, `-3` and so on while the
name exists, writes `<name>.part` with `create_new` and renames it. The file's cards, in
order: `SIMPLE`, `BITPIX`, `NAXIS`, `NAXIS1`, `NAXIS2`, for 16 bits `BZERO` 32768 and
`BSCALE` 1, `EXPTIME`, `GAIN`, `OFFSET`, `XBINNING`, `YBINNING`, `XORGSUBF` and
`YORGSUBF` (`frame.roi.x` and `frame.roi.y` each divided by `frame.binning`, rounded
down), `CCD-TEMP`, `BAYERPAT`, `INSTRUME`, `IMAGETYP`, `ROWORDER`, `FRAMEID` (the node's
id), `PURPOSE`, `NODE-T0` (`start_s`), `END`. `GAIN`, `OFFSET`, `CCD-TEMP` and
`BAYERPAT` are left out when the frame has none.

**The format** is the one the console's own writer follows (031's plan.md § FITS at the
console). It is written out here because the scope node must not depend on the
console's crate. The header is `SIMPLE = T`, `BITPIX`, `NAXIS = 2`, `NAXIS1`, `NAXIS2`,
then for 16 bits `BZERO = 32768` and `BSCALE = 1`, then the cards above, then `END`,
padded with spaces to a multiple of 2880 bytes; the data follows, padded with zeros to
a multiple of 2880. A card is 80 bytes: the keyword padded to 8, `= `, then a logical
(`T` or `F`) or a number right-justified to column 30, or a text in single quotes from
column 11, padded inside the quotes to 8 characters, quotes doubled, cut at 68. A real
is written with Rust's `{:?}` with `e` made `E` (`0.5`, `100.0`, `-9.5`, `1E-7`). A
16-bit sample v is stored as the big-endian `i16` `v − 32768`; an 8-bit sample as it
is. `EXPTIME` (seconds), `GAIN`, `OFFSET`, `CCD-TEMP` and `NODE-T0` are reals;
`XBINNING`, `YBINNING`, `XORGSUBF`, `YORGSUBF` and `FRAMEID` are integers; `BAYERPAT`,
`INSTRUME` (the camera's model), `IMAGETYP` (`DARK` for the purpose `dark`, `FLAT` for
`flat`, `LIGHT` otherwise), `ROWORDER` (`TOP-DOWN`) and `PURPOSE` are texts.

The node has no wall clock, so there is no `DATE-OBS`. It writes no `SET-TEMP` or
`SERIALNO`: those are what the console stamps from its own settings (spec owner
decision 12; 031's owner decision 16 holds the open alternative, which would move them
here). `write`'s signature does not change for the region: the frame carries it. This is
a second small writer on purpose: the scope node must not depend on the console's
crate, and the console's tests read the node's file with code of their own (031's
US2-S12).

### The probe (`crates/scope-node/src/imaging/probe.rs`, new, US7)

```rust
/// Describe the camera, take three exposures, and write one line for each fact.
pub async fn run(
    camera: &dyn ImagingCamera,
    clock: &dyn NodeClock,
    exposure_ms: f64,
    out: &mut (dyn std::io::Write + Send),
) -> Result<(), ImagingError>;
```

Lines, each ended by a newline and flushed as it is written, numbers with `Display`:
`imaging camera: {model}`; `serial: {serial or "none"}`;
`sensor: {width} x {height} px, {pixel_um} um pixels, {bits} bits, {RGGB… or "mono"}`;
`exposure: {min} to {max} ms`; `gain: {min} to {max}` or `gain: none`; `offset: …`
likewise; `binnings: {joined by ", "}`; `cooler: yes` or `cooler: no`; then for n = 1, 2,
3, after `expose` with the purpose `probe`:
`exposure {n}: {width} x {height}, {bytes} bytes, in hand {ms} ms after the end of the exposure`,
with ms = `max(0, round((clock.now() − end_s)·1000))`. A camera error ends the run with
that error; a failed write is `Failed`.

The probe times the camera and the scope node, and nothing past them. The hop from the
node to the console is read at the console: a saved frame's
`received_at − start − exposure_ms/1000` is the time from the end of the exposure to
the last byte in hand there, which the Imaging page shows as
`in hand {t} s after the exposure`; that less the probe's figure is the time across the
link (031's plan.md § The README).

### The README

`crates/scope-node/README.md`, a new `## The imaging camera` after `## The mount link`
(`crates/scope-node/README.md:103`): US4 writes what runs where (an INDI server and the
camera's device program on the scope node, started by the owner; this program is its
client), the `[imaging]` table with an example, how a device is chosen, that the
camera's ranges are read from it, and a paragraph on white balance and the camera's
other controls: `neutral` is how a camera's white balance is made neutral; on a camera
whose white-balance controls scale the raw samples the table names them with their
neutral values, the research note's camera's `neutral = { "CCD_CONTROLS.WB_R" = 50,
"CCD_CONTROLS.WB_B" = 50 }` as the example; the adapter sets them on each connection,
so a power-cycled camera is set again when it is next reached; a name the camera lacks
fails the connection with `the camera has no control {name}`; every control neither the
table nor a settings change names stays as the device reports it; and no control is
recorded with a frame (spec owner decision 13). US5 adds the failed-exposure paragraph:
what the adapter does, that it relies on the device program starting an exposure up to
three times and restarting a failed one once, and that it does not rely on the USB
reset. US6 adds
`save_dir`, the file names and that nothing is ever deleted. US7 adds the probe and a
sample of its output.

## Shapes that existing tests pin

These are not changed by any story:

- **The `ScopeNode` service and every existing message.** Two hand-written
  implementations of the trait (`crates/finder/tests/cli/aim.rs:22`,
  `crates/scope-link/tests/link.rs:55`) compile unchanged because the new calls are in a
  new service. No existing message gains a field.
- **The finder camera's port.** `Camera`, `GrayFrame`, `Settings` and `check_range`
  (`crates/scope-node/src/camera.rs:31`, `crates/scope-node/src/camera.rs:62`) are
  untouched; the imaging side is a second trait.
- **`NodeService`'s four constructors** (`crates/scope-node/src/service.rs:114`,
  `crates/scope-node/src/service.rs:149`) and `routes`' existing services. Every test and
  the simulator call them; `with_imaging` is a new builder, and the new service is one
  more route.
- **`SimNode::new` and `from_camera`** (`crates/simulator/src/node.rs:26`,
  `crates/simulator/src/node.rs:34`), `Simulator::new`, `Optics` and its four fields
  (`crates/simulator/src/optics.rs:13`), `SimCamera` and `render`
  (`crates/simulator/src/render.rs:58`). Twenty-four call sites outside the simulator
  build a `SimNode`; all keep compiling and now also carry an imaging camera they never
  call.
- **`Config::parse` for a config without `[imaging]`**, and `Config::masked`'s text
  (`crates/scope-node/src/config.rs:192`): `crates/scope-node/tests/cli.rs:50` and the
  config's unit tests read them. The imaging table is printed by `main`, after the
  mount's lines.
- **Shapes this spec pins for its own later stories and for 031 to 037.** Tests build
  scope-link messages with `..Default::default()`; the stand-in INDI server of US4 is
  used by US5 unchanged. Three more, which the later specs will meet. 031's console
  tests implement `ScopeNodeImaging` by hand (its US2-S5 and US2-S11), so from then on
  a new call on that service stops an existing test compiling, which is plan trap 7
  again for the new service: a later spec's calls go in a new service of its own. A
  test's own camera builds an `ImagingFrame`, so a field added to it later breaks every
  such literal that does not end with `..Default::default()`, which the shared setup
  requires. Files once written are pinned too: a card added to the node's copy later
  appears only in later files.

## Decisions and the alternatives rejected

| Decision | Rejected | Why |
|---|---|---|
| A second trait, `ImagingCamera`, beside `Camera` | Widening `Camera` and `GrayFrame` | The finder camera streams 8-bit frames into detection; the imaging camera takes one deep exposure on request. Sharing a trait would put optional fields and unused methods on both, and would touch every existing adapter and test. |
| `&self` methods that answer futures | `&mut self`; synchronous `describe` | `abort` and `status` must work during `expose`, and an INDI camera is a network away even to describe. |
| Settings as a partial edit answered by all the settings in force | A whole settings object each time | The camera holds the truth (another client or the device program may round a value); the console need not know values it does not change. |
| A new service, served on a connection of its own | Calls on `ScopeNodeRaw`; one connection | A frame may take many seconds to cross; the heartbeat's 1 s limit stops the mount. A separate connection has its own flow control and cannot delay a heartbeat. The console's link manager opens it (031). |
| An unlimited, unary-free stream of chunks of at most 256 KiB | One message a frame; a larger chunk | tonic's 4 MiB default stays; memory per message is small; the size is the plan document's. |
| The rate limit is asked for by the console with each exposure | A node config key | The observer tunes it at the telescope from the Imaging page's `Forwarding limit` field, or an MCP client does, with no shell on the node. |
| The node finishes an exposure whose caller has gone | Abort on a dropped call | A dropped call is how a brief link loss looks; the camera stays in a known state and only an explicit abort stops it. |
| `quick-xml` and `base64` for the INDI adapter, each declared by the story that first calls it (US4, US5) | A hand-written XML reader and decoder; the `indi` client crate; both crates declared in US4 | A stream reader for untrusted-length text is where hand-written code goes wrong; `quick-xml` is small, MIT and pure Rust. The `indi` crate brings a web-server stack with it. A crate declared before its first use is refused by the merge queue's `cargo-machete`. |
| A timed expectation in the INDI script format | A list of received elements and times that each test reads | The script already says what must come next; saying when in the same place keeps one oracle, and US5 uses the stand-in unchanged. |
| One INDI connection, BLOBs enabled with `Also` | A second connection with `Only`; fast BLOBs | One connection is enough for one exposure at a time; fast BLOBs need file-descriptor passing, which means `unsafe` or another crate. |
| The adapter as one task with a command channel | A mutex round the connection | `status` and `abort` must be answered while an exposure waits. |
| The adapter always sends the three transfer switches and the frame type when they are defined | Sending only when the value differs | The conversation then depends only on what the device defines, so a saved capture can be recorded by a script. |
| One retry, then an error | Unlimited retries; none | INDI's device program already retries inside; one more try covers a lost connection; after that the observer must know. |
| The purpose as a short free label | A fixed list on the wire | Later specs add `sub`, `dark`, `flat`, `focus` and `run` with no change to the node. |
| Two stand-ins: a small one in the scope node, the sky one in the simulator | One | The simulator depends on the scope node, so the node's binary cannot build the simulator's camera; the node still needs something for `adapter = "simulated"` and the probe's test. |
| A star field of the simulator's own | The pattern database's stars; a catalogue file | The database has one star a square degree; a catalogue file is tens of megabytes in the repository. |
| Gaussian noise, one deviate a sample | The finder renderer's exact Poisson | At a sky of 20 electrons the difference is invisible and the cost is a third. |
| A hand-written FITS reader for the camera's BLOB and writer for the node's copy | A FITS crate | One image, a dozen cards and big-endian samples need no library; `fitsio` wraps C and the pure-Rust writers are young. |
| `IMAGETYP` `LIGHT`, `DARK`, `FLAT` | The two-word values some programs write | Both forms are read by the programs that will open these files. `LIGHT` is a named exception to the glossary's "avoid light": it is the FITS convention's value in that card, not this project's word for a sub. |
| The scope node records a frame's region (`ImagingFrame.roi`, the header's `roi`) and writes its origin in its own copy | The console stamping the region from its settings, with nothing on the scope link (the review's draft); leaving the region to 033 | The plan document's Amendment 6 puts the region in the header and the file. The node knows the region the camera really used, so a camera changed by another INDI client is stamped right, and the node's copy can be placed by itself (spec owner decision 12). |
| The INDI adapter sets the controls `[imaging] neutral` names on each connection, sends every other control back at its latest value, and records none with a frame | A bring-up step for the observer with an INDI client (the review's draft); recording every control with each frame; names of one maker's controls in code | The plan document's Amendment 2: white balance neutral once, when it connects, and recorded by no spec. The config, not the code, names the maker's controls; a misspelt name fails the connection rather than being skipped (spec owner decision 13). |
| The node writes its own FITS copy | INDI's `UPLOAD_BOTH`; a shared FITS crate | The stand-ins must save too, the file must carry the node's frame id, and the node must not depend on the console's crate. |

## Verification (every story, before proposing)

- **The factory gates** (`ergane.yaml` at 49bb210, unchanged since dbc2b5b):
  - `lint` = `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
    `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`
    and `RUSTDOCFLAGS="-D warnings" cargo doc -p finder --no-deps`;
  - `test` = `cargo nextest run --workspace --all-targets --all-features --locked`,
    `cargo test --workspace --locked --doc`, and then, in `crates/finder/web`,
    `npm ci --no-audit --no-fund`, `npm run typecheck`, `npm run format:check`,
    `npm test`, `npm run build`, `git status --porcelain -- dist | (! grep .)` and
    `CI=1 npm run e2e`.

  Every story runs both, and the judge sees their results. `--locked` means US4 commits
  `Cargo.lock` with `quick-xml` and what it brings, and US5 commits it again with
  `base64` as a dependency of `scope-node`; every other story leaves it alone.
- **CI in the merge queue** runs every `ci.yml` job, including `cargo-deny`,
  `cargo-machete`, the build on Rust 1.96 and the finder-page checks. No gate here runs
  them, so the operator runs `cargo deny check` and `cargo machete` on the probe builds
  of US4 and of US5 before the flip (§ Prerequisites 5). `cargo-machete` is why each
  crate is declared by the story that first calls it (§ Plan traps 19).

## Prerequisites before this spec is flipped to `ready`

1. **The earlier specs have landed.** `025-frame-capture`: met (US3 at 820c558).
   `028-finder-tests-do-less`: met on `main` at 49bb210, where all six of its stories
   have landed (US2 at 1928771, US1 at 54e69f4, US3 at 64c3862, US4 at 6bd5a28, US6 at
   dbc2b5b, US5 at 49bb210). The worktree is at 49bb210.
2. **The trio is re-anchored to `main`.** Met at 49bb210. Every `path:line` in spec.md
   and plan.md was compared, line for line, at dbc2b5b and at 49bb210. Two had moved
   and are corrected: `run` in `crates/scope-node/tests/cli.rs` from line 8 to 15 and
   `CONFIG` from 46 to 50. The scope node's own scratch guard is now cited at
   `crates/scope-node/tests/cli.rs:8`. Every other citation reads the same at both
   commits. If `main` moves again before the flip, this is done again and the validator
   re-run.
3. **The owner decisions are settled and the approvals given** (spec § Owner decisions,
   § Owner approvals). Not met. Of the thirteen decisions, 3 and 8 are settled by the
   plan document's Amendments 4 and 5, 12 by Amendment 6 and the white balance of 13 by
   Amendment 2; the other nine are open, with the open part of 13 (a name the camera
   lacks). Any answer other than the recommended one is written into the trio and
   re-validated before the flip.
4. **The saved capture is recorded and staged.** Not met. The owner runs § Recording the
   conversation against the INDI project's camera simulator and commits
   `fixtures/scope-node/indi/ccd-simulator.script` and
   `fixtures/scope-node/indi/ccd-simulator.expect.json`. US5 cannot pass without them,
   and no story may write them. If the recording shows the adapter's conversation must
   differ (a property the simulator lacks, a region it refuses), this plan is corrected
   first.
5. **The trio is probe-built.** Not met. No story has been compiled. Before the flip
   each of the seven stories is built from this trio in a throwaway worktree, tests
   first, with both gates, `cargo deny check` and `cargo machete`; each diff is measured
   as plan trap 3 says; and what the probes find is folded back in. US2 and US4 are
   built first: US2 because every later story stands on its camera, US4 because it looks
   the largest and carries the new dependency. The first things to prove are in § Risks.
6. **The documents this trio cites are on `main`.** Not met: the plan document,
   ADR-0002, the changed `GLOSSARY.md` and the research note are in the drafting
   worktree, uncommitted. They land with or before this trio, none of them by a story.
   The three changes this trio asked of them are made in that worktree and land with
   them: `GLOSSARY.md` carries the new **Purpose label** that spec owner approval 6
   words, with `probe` and `run` among its labels as the audit's glossary edit has
   them; the research note's source list holds the SBIG FITS extension document, which
   spec.md's Clarifications cite for `XORGSUBF` and `YORGSUBF`; and the note's § 2
   sentence on the ZWO program reads one restart, two attempts in all.
7. **No constitution change is needed for this spec.** Met. Principle V's "the solver is
   our own Rust" is not touched here; the imaging solver is 032's.
8. **Outside the factory, before the first use at the telescope:** INDI and the camera's
   device program installed on the scope node, the node's `[imaging]` table written, the
   scope node rebuilt from `main` after US7, and the probe run. Not needed to flip.
9. **The plan document agrees with the settled decisions.** Not met on `main`, where the
   plan document is not yet (Prerequisite 6); met in the drafting worktree. The plan
   document's Amendments 4 and 5 settle two of the three departures the review found on
   this side (owner decisions 3 and 8), and Amendments 2 and 6 are carried here (owner
   decisions 13 and 12). Its § Interfaces shared across the specs now carries the third,
   `save` on `ExposeRequest` ("whether the node also keeps the frame") with
   `[imaging] save_dir`, as this trio has it; owner decision 9 is still open. If the
   owner settles it otherwise, the plan document's § Interfaces shared across the specs
   is corrected in the same change, so that 031 to 037 read what 030 builds.

## Plan traps

Lessons from building 012 to 028 on this factory, and the traps this design adds, each
with the check that keeps it from recurring. They bind every story.

1. **Gate evidence or nothing.** The judge credits only gate results and the story's
   diff. *Check:* every Then clause names a committed test that the `test` gate runs.
2. **A named command is read literally.** *Check:* commands are copied from
   § Verification.
3. **One evidence budget.** *Check:* each story keeps its diff under 64 KiB, measured
   with `git diff <base> -- . ':(exclude)crates/finder/web/dist' ':(exclude)Cargo.lock' | wc -c`.
   A story that cannot escalates; it does not drop a scenario.
4. **The sandbox has only the gates' tools.** *Check:* no task runs `cargo deny`,
   `cargo machete`, an INDI program, `socat` or anything else the gates do not.
5. **A spec fixed mid-epic needs a relaunch.** *Check:* no builder edits
   `specs/030-imaging-camera-link/` or `.specify/`.
6. **An oracle copied from the code proves nothing.** *Check:* tests compute the sample
   rule themselves, read saved FITS files with their own code and never through
   `scope_node::imaging::fits` or `scope_node::imaging::save`, play INDI through a
   stand-in with its own scanner, and take the saved capture's expected values from the
   JSON a separate program wrote.
7. **A new call on `ScopeNode`, or a new field on an old message, breaks tests.**
   *Check:* everything new is in `ScopeNodeImaging` and its own messages.
8. **Real sockets and paused time do not mix.** With time paused, tokio jumps to the
   next timer whenever every task is idle, so a timeout fires while a real socket is
   still delivering. *Check:* the INDI adapter takes a connector, and every test of it
   runs over `tokio::io::duplex`; only the tests of the built `scope-node` run in real
   time, with generous limits.
9. **A half-read element.** A `select!` that drops a pending read loses the bytes it had
   taken. *Check:* one task reads elements and sends whole ones through a channel; the
   adapter's task selects on the channel.
10. **Blocking the node's two worker threads.** Decoding 43 MB of base64 or writing a
    32 MB file inline would hold a worker the heartbeat needs. *Check:* base64, FITS
    conversion and the node's file write run on `spawn_blocking`; US3-S8 proves the
    stream itself never holds a heartbeat.
11. **A guard that outlives its exposure.** A camera left `exposing` after an error or a
    dropped future refuses every later exposure. *Check:* both stand-ins clear the mark
    with a drop guard; US1-S7, US3-S5 and US3-S7 take a frame after an abort or a drop.
12. **The mosaic's phase.** A region that starts on an odd pixel, or binning that mixes
    colours, silently turns RGGB into something else. *Check:* `check_settings` requires
    x and y to be multiples of twice the binning; US2-S7 asserts which samples a pure
    red star reaches at both binnings.
13. **Sidereal drift is large at this scale.** At 2.18″ a pixel an untracked star moves
    7 px a second. *Check:* geometry scenarios use 20 ms exposures and place stars at
    the truth for mid-exposure; only US2-S6 and US2-S9 use long ones.
14. **The simulated mount's take-up.** A rate command first takes up backlash, during
    which the tube does not move. *Check:* US2-S6 waits 60 s at the rate before its
    exposure and asserts the star's true places are more than 10 px apart.
15. **Whole-struct literals break at the next story.** *Check:* tests build scope-link
    messages and `ImagingFrame` literals with `..Default::default()`. (The views this
    rule guards at the console are 031's.)
16. **A pacing rule that counts the chunk it is about to send.** *Check:* the wait before
    chunk k is the bytes already sent over the rate; US3-S8 gives the worked time,
    7.69 s.
17. **A FITS card one column out.** Readers are strict about `= ` in columns 9 and 10
    and numbers ending in column 30. *Check:* US6-S1 reads the file with the test's own
    fixed-column reader, and § Reading a FITS BLOB gives the worked bytes.
18. **Each worktree builds on its own, and scratch files go where cargo says.** *Check:*
    no story sets `CARGO_TARGET_DIR`; every file or folder a test makes is under
    `CARGO_TARGET_TMPDIR` behind a drop guard of the test file's own; and no test builds
    a path from `CARGO_MANIFEST_DIR` and `../../target`, which 028 removed.
19. **A dependency declared before anything calls it.** `cargo-machete` is a required
    check of the merge queue and no local gate runs it, so a story that declares a crate
    early passes both gates and is refused at the queue. *Check:* `quick-xml` is
    declared by US4, whose reader calls it, and `base64` by US5, whose `expose` decodes
    with it; no story declares a crate its own diff does not call.
20. **A time that includes connecting.** The INDI adapter connects at its first call and
    then waits 200 ms of quiet. *Check:* every timed INDI scenario uses a camera on
    which `describe()` has returned, or says that it times the first call (spec.md's
    shared setup, **The camera of US4-S1**).
21. **A stand-in that cannot say when.** An expectation that only waits for the next
    element proves order, not time. *Check:* every scenario that states when the adapter
    sent something uses `@ expect after MS`, whose stamp is taken when the element
    arrives, whatever directive the script has reached.
22. **Neutral values undone by the next change.** If the adapter sent its neutral
    vector without awaiting the answer, a quick `apply` would send the device's old
    white balance back as "the latest value". *Check:* step 7 of **Connecting** awaits
    each answer before the call that connected goes on; US4-S8 asserts the `apply`
    after it sends `WB_R` 50 and `WB_B` 50.

## Risks

| Risk | Mitigation |
|---|---|
| Nothing here has been compiled. An interface may not build as written: `quick-xml`'s async reader over a stream with many top-level elements and a 43 MB text; a `dyn` connector's bounds; `OnceLock` in `Runtime`; prost's names for `ImagingStatus` the call and `ImagingStatusReply` the message. | Prerequisite 5: probe builds of all seven stories before the flip, US2 and US4 first, then US5. If `quick-xml` cannot read the stream, owner decision 4's alternative (a hand-written reader) is taken and the trio corrected. |
| A story passes 64 KiB. The likeliest, by what each writes, are US4 (eleven files: the XML reader, the adapter's property model and task with its neutral controls, the script interpreter with its timed expectations, both worked scripts written out, the config with `neutral`, the binary, a README section and eight scenarios) and US2 (a renderer and nine scenarios), then US5. This is an estimate from the interfaces; nothing has been measured. | Measured on the probes, US2 and US4 first. The splits are written in advance. US4: the config table with `neutral`, the binary's attaching and `--check-config`, and the README section (US4-S6, US4-S7; the `[imaging]` table of FR-020, the binary's half of FR-025, FR-026) become a story of their own after the adapter, `depends_on_merged` on it, writing `crates/scope-node/src/config.rs`, `crates/scope-node/src/main.rs`, `crates/scope-node/README.md` and `crates/scope-node/tests/imaging_config.rs`; the adapter story keeps `quick-xml`, the stand-in and US4-S1 to US4-S5 and US4-S8. US2 can shed US2-S8 and US2-S9 into a story of their own (the seeded field). Each split is a spec change made before the flip. |
| The saved capture cannot be recorded as written: the INDI simulator may refuse a 64 × 48 region, need a star catalogue program to finish an exposure, or define properties in an order that breaks the script's quiet period. | Prerequisite 4 is the owner's, before the flip; the recording script and US5-S8 are corrected to what the simulator does. US5's other seven scenarios need no capture. |
| The real camera's device program names its controls otherwise than `Gain` and `Offset`, or reports no offset. | The description then has no gain or offset range, which the console, the files and the page already handle. A capture of the real camera is a field check; a correction is one table row in § The INDI adapter. |
| 60 s is too short, or too long, for a real download on the real node. | The probe measures it at bring-up; the constant is one line and owner decision 6. |
| A frame of 32 MiB on the node costs about 110 MB at its peak: the base64 text, the FITS bytes and the samples. | Stated here so the owner can weigh it against the node's memory; `binning` 2 quarters it. Not measured. |
| The second connection doubles what a poor link must carry at the worst moment. | The rate limit (US3-S8); the heartbeat is on its own connection and proven unaffected on the simulator; the real link is a field check. |
| Specs 031 to 037 need a field this spec's types lack. | `ImagingFrame` gains `roi` here (the plan document's Amendment 6) and derives `Default`; tests end every literal of it, and of a scope-link message, with `..Default::default()`, so a field added later breaks none of this spec's tests, though it still changes the scope link, which a later spec must name for approval. |
| `main` moves again before the flip and shifts a cited line. | Prerequisite 2: the line-for-line comparison is run again against the new `main`, as it was for 49bb210. |
| A camera's white balance is not in `[imaging] neutral`, because its controls have other names or the table was not written, so its files are scaled by values no file records. | The README says the table is how white balance is made neutral and gives the research note's camera as the example; reading the real camera's control names and writing the table is a field check (spec Assumptions, Outside the factory). No spec records controls per frame (the plan document's Amendment 2). |
| The real camera rejects or clamps a neutral value, or answers it in the state `Alert`. | The connection fails with the camera's words (`the camera refused CCD_CONTROLS: …`) and the console shows `error`; the table is corrected. |
| Owner decision 9 is settled otherwise than recommended, after 032 to 037 were drafted from the plan document, which carries `save` on `ExposeRequest` and `save_dir` as this trio has them. | Prerequisite 9; no spec after 031 calls `Expose` or reads `save_dir`, and 031 takes both from this trio's plan.md. |
| The INDI scripts' timed expectations are exact to the millisecond of virtual time; a reader or a channel that needs a timer tick between two sends would fail an `after 0`. | The 1 s pause, the 60.5 s wait and the 200 ms settle are the times that matter and stay timed. If a probe build shows an `after 0` cannot hold, that one expectation is made untimed before the flip. |

## Refinement Record

Drafted on 2026-11-15 by the grilling session against `main` at dbc2b5b, from
`docs/plans/2026-11-15-specs-030-037-imaging.md`, the surveys of the camera path and the
console made that morning, and INDI's documents and two device programs' sources read
that day. **No probe build has been run.** No story of this trio has been compiled,
linted or tested, no interface above has been built, and no diff has been measured: the
session wrote documents only. Every `path:line` was read at dbc2b5b, re-read at
49bb210 and checked by the validator there; every number in a scenario was worked by
hand and is shown as a worked
example where it is not obvious. The two programs of § Recording the conversation have
not been run either.

A second agent reviewed the draft the same day, when this trio and 031 were one. Its
twenty-eight findings were each checked against the files, the code at dbc2b5b and
49bb210, and the research, and applied, still with no probe build. Those on this side
of the link: `base64` moved to the story that uses it; the binning rule made to fit a
sensor whose height is not a multiple of 4; the INDI scenarios' times tied to a
connected camera and proved by timed expectations in the script format; scratch folders
put where 028 put them; behaviour tested in the story that builds it (the service's
"no save folder" answer, `from_cameras`); a frame's region stamped into the node's file;
white balance and the departures from the plan document written as owner decisions.

An auditor then compared the trio with 032 to 037, and its findings were applied the
same day, still with no probe build. Those on this side: the INDI adapter sets the
controls `[imaging] neutral` names on each connection (Amendment 2), which replaces the
bring-up step and the hand-off to 034; the region of interest travels with the frame
from the scope node and is written into the node's file as into the console's
(Amendment 6); and the departures two amendments settle are marked so. The worktree
moved to 49bb210 meanwhile: every citation was compared at dbc2b5b and 49bb210 and the
two that moved are corrected (§ Prerequisites 2), so the record below is reviewed at
49bb210.

The trio was then split in two at the owner's ask, the same day and still with no probe
build. The console's stories, with their scenarios, requirements, decisions, traps and
record entries, moved to 031, and what stayed was renumbered from 1: the story of the
node's copy is US6 and the probe's US7, where they were the ninth and twelfth; the
node's copy's requirements are FR-033 to FR-036 and the probe's FR-037 and FR-038. No
interface, scenario value or decision changed. The one cut inside a story is US6's: the
scenario that read the node's file from the console is 031's US2-S12, and US6 writes
nothing under `crates/finder`.

What a first refinement round must do is § Prerequisites 5, and its findings replace
this paragraph. Until then the record below is the draft's own account: its tests are
the scenarios, its writes are the files § Project Structure names, and its shared writes
name the merge edge that orders two stories of this spec on one file, or the story of
025 that last wrote the file on `main`. A file last written by an older landed
spec (013, 014, 016, 017, 020, 024 or 027, each of which 028 already depends on) has no
entry, because the validator refuses a shared write that names a spec outside this
spec's `depends_on_landed`. The record's `open_questions` are empty for the same kind of
reason: the validator refuses a record that holds any. What is not established is
written where it can stay until a probe settles it: § Risks, and § Prerequisites 4 and
5. Nothing in this record is an owner approval: those are in spec.md, and flipping to
`ready` gives them.

```yaml
version: 1
reviewed_revision: 49bb210fe4860dd98ba24727074e7bd3cc0d7124
stories:
  US1:
    tests:
      - path: crates/scope-node/tests/imaging_port.rs
        status: new
        disposition: add
        assertion: "the stand-in camera describes itself and its status exactly as the plan gives them"
        decision: "spec.md:US1-S1"
      - path: crates/scope-node/tests/imaging_port.rs
        status: new
        disposition: add
        assertion: "an empty apply answers the settings in force, an edit changes only what it names, and the cooler follows its rule"
        decision: "spec.md:US1-S2"
      - path: crates/scope-node/tests/imaging_port.rs
        status: new
        disposition: add
        assertion: "a 500 ms exposure returns after exactly 500 ms with node-clock times, the region in force as its roi, and every sample equal to the sample rule"
        decision: "spec.md:US1-S3"
      - path: crates/scope-node/tests/imaging_port.rs
        status: new
        disposition: add
        assertion: "binning 2 and a region of interest deliver 8 x 4 samples that follow the sample rule from 426, with roi (8, 4, 16, 8)"
        decision: "spec.md:US1-S4"
      - path: crates/scope-node/tests/imaging_port.rs
        status: new
        disposition: add
        assertion: "each refused setting answers Invalid with its exact text and changes nothing"
        decision: "spec.md:US1-S5"
      - path: crates/scope-node/tests/imaging_port.rs
        status: new
        disposition: add
        assertion: "an exposure outside the range or a bad purpose answers Invalid with its exact text at once"
        decision: "spec.md:US1-S6"
      - path: crates/scope-node/tests/imaging_port.rs
        status: new
        disposition: add
        assertion: "a second expose is Busy, an apply that names a setting is Busy while an empty one answers, abort ends the first with Aborted at once, status follows, and a new exposure then works"
        decision: "spec.md:US1-S7"
      - path: crates/scope-node/src/camera.rs
        status: existing
        disposition: preserve
        assertion: "the finder camera's port, its range refusals and its unit tests are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the finder camera's Camera trait, GrayFrame and Settings are unchanged (spec.md FR-001)"
      - "no unsafe in crates/scope-node"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test imaging_port"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/scope-node/src/imaging/mod.rs
      - crates/scope-node/src/imaging/synthetic.rs
      - crates/scope-node/src/lib.rs
      - crates/scope-node/tests/imaging_port.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/src/imaging/mod.rs
        other_spec: 030-imaging-camera-link
        other_story: US3
        disposition: merge_order
        reason: "US3 depends_on_merged US1, and adds pub mod service after US1's module line"
    open_questions: []
  US2:
    tests:
      - path: crates/simulator/tests/imaging.rs
        status: new
        disposition: add
        assertion: "the simulated camera's description, starting settings and default optics are exactly the plan's"
        decision: "spec.md:US2-S1"
      - path: crates/simulator/tests/imaging.rs
        status: new
        disposition: add
        assertion: "the truth's scale, centre and angle agree with a worked scale, the existing eyepiece truth and a position angle the test computes from the sky functions"
        decision: "spec.md:US2-S2"
      - path: crates/simulator/tests/imaging.rs
        status: new
        disposition: add
        assertion: "two stars placed from the truth land within 0.3 px of the frame centre and of 300 arcseconds north of it"
        decision: "spec.md:US2-S3"
      - path: crates/simulator/tests/imaging.rs
        status: new
        disposition: add
        assertion: "frames repeat from the seed and frame number, and an empty frame's mean and deviation match the worked sky and noise"
        decision: "spec.md:US2-S4"
      - path: crates/simulator/tests/imaging.rs
        status: new
        disposition: add
        assertion: "a star's width is 1.20 px in focus and 3.23 px at a focus error of 3"
        decision: "spec.md:US2-S5"
      - path: crates/simulator/tests/imaging.rs
        status: new
        disposition: add
        assertion: "a star smears by the sky's drift in 2 s, and by the mount's motion between its two true places"
        decision: "spec.md:US2-S6"
      - path: crates/simulator/tests/imaging.rs
        status: new
        disposition: add
        assertion: "binning 2 keeps the RGGB mosaic and halves the frame, its last row is the mean of the two samples that exist, and a region of interest shifts a star by its origin and is the frame's roi"
        decision: "spec.md:US2-S7"
      - path: crates/simulator/tests/imaging.rs
        status: new
        disposition: add
        assertion: "the seeded field repeats, has the worked density and magnitude share, is sorted, and agrees where two fields overlap"
        decision: "spec.md:US2-S8"
      - path: crates/simulator/tests/imaging.rs
        status: new
        disposition: add
        assertion: "the default field puts its brightest nearby star in a 1 s frame, and the camera honours Busy for expose and apply, Aborted, refusals and the simulator clock"
        decision: "spec.md:US2-S9"
      - path: crates/simulator/tests/node.rs
        status: existing
        disposition: preserve
        assertion: "the finder simulator's frames, previews and node are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/simulator/tests/sky.rs
        status: existing
        disposition: preserve
        assertion: "the simulator's sky functions and existing truth are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "Optics, SimCamera, render and every existing public name of the simulator are unchanged (spec.md FR-006)"
      - "no unsafe in crates/simulator"
      - "no test renders the 4944 x 3284 size"
    checks:
      development:
        - "cargo nextest run -p simulator --locked --test imaging"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs"
    writes:
      - crates/simulator/src/imaging.rs
      - crates/simulator/src/lib.rs
      - crates/simulator/src/noise.rs
      - crates/simulator/src/optics.rs
      - crates/simulator/tests/imaging.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
  US3:
    tests:
      - path: crates/scope-node/tests/imaging_service.rs
        status: new
        disposition: add
        assertion: "DescribeImaging, ImagingStatus and ConfigureImaging answer the stand-in camera's values, and a refused gain is InvalidArgument with its text"
        decision: "spec.md:US3-S1"
      - path: crates/scope-node/tests/imaging_service.rs
        status: new
        disposition: add
        assertion: "Expose answers a header with id, size, times, byte count and the frame's roi, then chunks of at most 262144 bytes equal to the sample rule; the next id is 2, and a region configured first is the next header's roi"
        decision: "spec.md:US3-S2"
      - path: crates/scope-node/tests/imaging_service.rs
        status: new
        disposition: add
        assertion: "each of the five calls answers Unauthenticated with a wrong token"
        decision: "spec.md:US3-S3"
      - path: crates/scope-node/tests/imaging_service.rs
        status: new
        disposition: add
        assertion: "a node with no imaging camera answers FailedPrecondition and no imaging camera, and a node without the service Unimplemented"
        decision: "spec.md:US3-S4"
      - path: crates/scope-node/tests/imaging_service.rs
        status: new
        disposition: add
        assertion: "a second Expose is ResourceExhausted, AbortExposure ends the first with Aborted, and status follows"
        decision: "spec.md:US3-S5"
      - path: crates/scope-node/tests/imaging_service.rs
        status: new
        disposition: add
        assertion: "a bad exposure or purpose is InvalidArgument, a camera failure Internal with its text, and save without a folder reports this scope node has no save_dir"
        decision: "spec.md:US3-S6"
      - path: crates/scope-node/tests/imaging_service.rs
        status: new
        disposition: add
        assertion: "a dropped Expose leaves the exposure running to its end, and the next frame has id 2"
        decision: "spec.md:US3-S7"
      - path: crates/simulator/tests/imaging_link.rs
        status: new
        disposition: add
        assertion: "a 32472192-byte frame crosses in 124 chunks, paced at 4 MiB/s in 7.5 to 8 s and unpaced at once, while 30 heartbeats on the other connection keep their 250 ms slots and the mount keeps moving"
        decision: "spec.md:US3-S8"
      - path: crates/simulator/tests/imaging_link.rs
        status: new
        disposition: add
        assertion: "SimNode serves the simulated imaging camera, 1236 x 822 in 8 chunks, beside the unchanged finder camera, and from_cameras attaches the caller's camera, whose starless noiseless frame is 162 everywhere"
        decision: "spec.md:US3-S9"
      - path: crates/scope-link/tests/link.rs
        status: existing
        disposition: preserve
        assertion: "its hand-written ScopeNode implementation compiles and behaves unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/finder/tests/cli/aim.rs
        status: existing
        disposition: preserve
        assertion: "its hand-written ScopeNode node compiles unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/raw_frame.rs
        status: existing
        disposition: preserve
        assertion: "RawFrame and the star-frame stream are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/simulator/tests/node.rs
        status: existing
        disposition: preserve
        assertion: "SimNode's finder camera, frames and previews are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "no existing scope-link message, field or call changes (spec.md FR-013)"
      - "NodeService's four constructors and SimNode::new and from_camera keep their signatures"
      - "an older node keeps working with the new console, and an older console with the new node"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test imaging_service"
        - "cargo nextest run -p simulator --locked --test imaging_link"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a rebuild of the scope node"
    writes:
      - crates/scope-link/proto/scope_node/v1/scope_node.proto
      - crates/scope-node/src/imaging/mod.rs
      - crates/scope-node/src/imaging/service.rs
      - crates/scope-node/src/service.rs
      - crates/scope-node/tests/imaging_service.rs
      - crates/simulator/src/node.rs
      - crates/simulator/tests/imaging_link.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-link/proto/scope_node/v1/scope_node.proto
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "030 depends_on_landed 025, whose US2 last changed the scope link proto"
      - path: crates/scope-node/src/service.rs
        other_spec: 025-frame-capture
        other_story: US2
        disposition: merge_order
        reason: "030 depends_on_landed 025, whose US2 last changed service.rs"
      - path: crates/scope-node/src/service.rs
        other_spec: 030-imaging-camera-link
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US3, and adds with_imaging_save_dir"
      - path: crates/scope-node/src/imaging/service.rs
        other_spec: 030-imaging-camera-link
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US3, and adds saving before the header"
      - path: crates/scope-node/src/imaging/mod.rs
        other_spec: 030-imaging-camera-link
        other_story: US4
        disposition: merge_order
        reason: "US4 depends_on_merged US3, and adds pub mod indi after US3's line"
    open_questions: []
  US4:
    tests:
      - path: crates/scope-node/tests/indi.rs
        status: new
        disposition: add
        assertion: "over the colour-camera script the adapter sends getProperties, CONNECT and, exactly 200 ms after the last definition, enableBLOB Also, and describes and reports the script's camera exactly"
        decision: "spec.md:US4-S1"
      - path: crates/scope-node/tests/indi.rs
        status: new
        disposition: add
        assertion: "with no device named it picks the device whose DRIVER_INTERFACE has the camera bit, skips CONNECT when connected, and switches compression off and upload to the client"
        decision: "spec.md:US4-S2"
      - path: crates/scope-node/tests/indi.rs
        status: new
        disposition: add
        assertion: "apply sends CCD_BINNING, CCD_CONTROLS, CCD_FRAME, CCD_COOLER and CCD_TEMPERATURE in order and answers the settings the camera reports, the rounded width included"
        decision: "spec.md:US4-S3"
      - path: crates/scope-node/tests/indi.rs
        status: new
        disposition: add
        assertion: "an out-of-range gain is Invalid and sends nothing, an Alert answer is Failed with the camera's message, and no answer is Failed after exactly 5 s"
        decision: "spec.md:US4-S4"
      - path: crates/scope-node/tests/indi.rs
        status: new
        disposition: add
        assertion: "an unreachable server, a missing device, a server with no camera and a 12-bit sample size each answer Failed with their text, the two waits after exactly 5 s, and a closed connection is made again"
        decision: "spec.md:US4-S5"
      - path: crates/scope-node/tests/imaging_config.rs
        status: new
        disposition: add
        assertion: "the [imaging] table parses with its defaults and its neutral table, and refuses a missing adapter, misplaced keys, an empty device, unknown keys and bad neutral keys or values without printing the token"
        decision: "spec.md:US4-S6"
      - path: crates/scope-node/tests/imaging_config.rs
        status: new
        disposition: add
        assertion: "the built scope-node --check-config prints the [imaging] table with its neutral line, or that no imaging camera is configured"
        decision: "spec.md:US4-S7"
      - path: crates/scope-node/tests/indi.rs
        status: new
        disposition: add
        assertion: "with neutral controls the adapter sends one CCD_CONTROLS vector after enableBLOB on each connection, the named elements at their values and the rest as reported, a later apply keeps them, and a control the camera lacks or refuses fails describe with its text"
        decision: "spec.md:US4-S8"
      - path: crates/scope-node/tests/cli.rs
        status: existing
        disposition: preserve
        assertion: "--check-config's existing lines and the refusals of unknown keys are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
      - path: crates/scope-node/tests/mount_config.rs
        status: existing
        disposition: preserve
        assertion: "the [mount] table parses and prints as before"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "a config without [imaging] parses to the same Config::settings and masked text"
      - "the node starts whether or not an INDI server is running"
      - "no unsafe in crates/scope-node"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test indi"
        - "cargo nextest run -p scope-node --locked --test imaging_config"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs: cargo-deny, cargo-machete and msrv judge quick-xml"
    writes:
      - Cargo.lock
      - crates/scope-node/Cargo.toml
      - crates/scope-node/README.md
      - crates/scope-node/src/config.rs
      - crates/scope-node/src/imaging/indi/mod.rs
      - crates/scope-node/src/imaging/indi/protocol.rs
      - crates/scope-node/src/imaging/mod.rs
      - crates/scope-node/src/main.rs
      - crates/scope-node/tests/imaging_config.rs
      - crates/scope-node/tests/indi.rs
      - crates/scope-node/tests/support/mod.rs
    no_write_reason: ""
    shared_writes:
      - path: Cargo.lock
        other_spec: 030-imaging-camera-link
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds base64 to scope-node's entry after US4 added quick-xml"
      - path: crates/scope-node/Cargo.toml
        other_spec: 030-imaging-camera-link
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds base64 after US4 added quick-xml"
      - path: crates/scope-node/README.md
        other_spec: 030-imaging-camera-link
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds the failed-exposure paragraph"
      - path: crates/scope-node/src/main.rs
        other_spec: 030-imaging-camera-link
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US4, and passes save_dir to the service"
      - path: crates/scope-node/src/config.rs
        other_spec: 030-imaging-camera-link
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US4, and adds save_dir"
      - path: crates/scope-node/src/imaging/mod.rs
        other_spec: 030-imaging-camera-link
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds pub mod fits"
      - path: crates/scope-node/src/imaging/indi/mod.rs
        other_spec: 030-imaging-camera-link
        other_story: US5
        disposition: merge_order
        reason: "US5 depends_on_merged US4, and adds exposures to the adapter"
    open_questions: []
  US5:
    tests:
      - path: crates/scope-node/tests/indi_expose.rs
        status: new
        disposition: add
        assertion: "an exposure sets the frame type and CCD_EXPOSURE, times start at Busy on the node clock, the frame's roi is the device's CCD_FRAME, and the worked FITS BLOB becomes the worked little-endian samples"
        decision: "spec.md:US5-S1"
      - path: crates/scope-node/tests/indi_expose.rs
        status: new
        disposition: add
        assertion: "dark and flat choose their frame types, an 8-bit mono BLOB passes through with gain and offset from CCD_GAIN and CCD_OFFSET, and the roi follows the region the device reports"
        decision: "spec.md:US5-S2"
      - path: crates/scope-node/tests/indi_expose.rs
        status: new
        disposition: add
        assertion: "an Alert is followed at once by an abort, exactly 1 s later by the frame type and one more try that succeeds, each element at the time the script states, and two Alerts answer Failed with the camera's message"
        decision: "spec.md:US5-S3"
      - path: crates/scope-node/tests/indi_expose.rs
        status: new
        disposition: add
        assertion: "no frame within 60 s of the exposure's end, twice, answers Failed after exactly 122 s, the aborts and the second try at the times the script states, with status answered at once meanwhile"
        decision: "spec.md:US5-S4"
      - path: crates/scope-node/tests/indi_expose.rs
        status: new
        disposition: add
        assertion: "a closed connection is made again for the second try, and two closed connections answer Failed"
        decision: "spec.md:US5-S5"
      - path: crates/scope-node/tests/indi_expose.rs
        status: new
        disposition: add
        assertion: "abort sends CCD_ABORT_EXPOSURE at the time the script states and answers Aborted in that instant, a late BLOB is discarded, and a second expose and an apply that names a setting are Busy and send nothing"
        decision: "spec.md:US5-S6"
      - path: crates/scope-node/tests/indi_expose.rs
        status: new
        disposition: add
        assertion: "a compressed BLOB, a 3-axis FITS, a float FITS and short data each answer Failed with their text and no second try"
        decision: "spec.md:US5-S7"
      - path: crates/scope-node/tests/indi_expose.rs
        status: new
        disposition: add
        assertion: "the saved capture of the INDI camera simulator is played to its end and the description and frame equal the values a separate program read from it"
        decision: "spec.md:US5-S8"
      - path: crates/scope-node/tests/cli.rs
        status: existing
        disposition: preserve
        assertion: "the node's command line behaves as before"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "the scope node does no pixel work: samples change byte order and nothing else (spec.md FR-028)"
      - "the stand-in INDI server of US4 is used unchanged"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test indi_expose"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs (cargo-deny and cargo-machete judge base64), and the first exposure of the real camera through the node"
    writes:
      - Cargo.lock
      - crates/scope-node/Cargo.toml
      - crates/scope-node/README.md
      - crates/scope-node/src/imaging/fits.rs
      - crates/scope-node/src/imaging/indi/mod.rs
      - crates/scope-node/src/imaging/mod.rs
      - crates/scope-node/tests/indi_expose.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/README.md
        other_spec: 030-imaging-camera-link
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds save_dir to the section"
      - path: crates/scope-node/src/imaging/mod.rs
        other_spec: 030-imaging-camera-link
        other_story: US6
        disposition: merge_order
        reason: "US6 depends_on_merged US5, and adds pub mod save"
    open_questions: []
  US6:
    tests:
      - path: crates/scope-node/tests/imaging_save.rs
        status: new
        disposition: add
        assertion: "with a save folder a saved frame is frame-000001.fits, written before the header, with the node's cards, the region's origin among them, NODE-T0 and the stream's samples"
        decision: "spec.md:US6-S1"
      - path: crates/scope-node/tests/imaging_save.rs
        status: new
        disposition: add
        assertion: "save false writes nothing, names count up, a taken name gets -2 and is never replaced, dark and flat set IMAGETYP, and a binned region's origin is written in binned pixels"
        decision: "spec.md:US6-S2"
      - path: crates/scope-node/tests/imaging_save.rs
        status: new
        disposition: add
        assertion: "without a save folder, or with one that cannot be written, the header says why and the whole frame still follows"
        decision: "spec.md:US6-S3"
      - path: crates/scope-node/tests/imaging_save.rs
        status: new
        disposition: add
        assertion: "the [imaging] table takes save_dir, refuses an empty one, and --check-config prints it"
        decision: "spec.md:US6-S4"
      - path: crates/scope-node/tests/cli.rs
        status: existing
        disposition: preserve
        assertion: "--check-config's existing lines are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "with save false the node writes nothing to disk, as before (spec.md FR-033)"
      - "a failed save never stops a frame being relayed (spec.md FR-035)"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test imaging_save"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and a rebuild of the scope node"
    writes:
      - crates/scope-node/README.md
      - crates/scope-node/src/config.rs
      - crates/scope-node/src/imaging/mod.rs
      - crates/scope-node/src/imaging/save.rs
      - crates/scope-node/src/imaging/service.rs
      - crates/scope-node/src/main.rs
      - crates/scope-node/src/service.rs
      - crates/scope-node/tests/imaging_save.rs
    no_write_reason: ""
    shared_writes:
      - path: crates/scope-node/README.md
        other_spec: 030-imaging-camera-link
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds the probe to the section"
      - path: crates/scope-node/src/main.rs
        other_spec: 030-imaging-camera-link
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds --probe-camera"
      - path: crates/scope-node/src/imaging/mod.rs
        other_spec: 030-imaging-camera-link
        other_story: US7
        disposition: merge_order
        reason: "US7 depends_on_merged US6, and adds pub mod probe"
    open_questions: []
  US7:
    tests:
      - path: crates/scope-node/tests/imaging_probe.rs
        status: new
        disposition: add
        assertion: "the probe writes the stand-in camera's description and three exposure lines with 0 ms in hand, in exactly 1.5 s"
        decision: "spec.md:US7-S1"
      - path: crates/scope-node/tests/imaging_probe.rs
        status: new
        disposition: add
        assertion: "a mono camera without controls prints none and no, a frame 350 ms after a 100 ms exposure reads 250 ms, and a camera error ends the run"
        decision: "spec.md:US7-S2"
      - path: crates/scope-node/tests/imaging_probe.rs
        status: new
        disposition: add
        assertion: "the built scope-node --probe-camera prints the lines and exits 0, fails without an [imaging] table, and conflicts with --check-config"
        decision: "spec.md:US7-S3"
      - path: crates/scope-node/tests/cli.rs
        status: existing
        disposition: preserve
        assertion: "--check-config and the start-up refusals are unchanged"
        decision: "plan.md:Shapes that existing tests pin"
    preserved_behavior:
      - "--probe-mount and --check-config behave as before"
    checks:
      development:
        - "cargo nextest run -p scope-node --locked --test imaging_probe"
      handoff: declared_gates
      independent: declared_pipeline
      release:
        - "release owner: the repository owner, through the merge queue's ci.yml jobs, and the probe run on the scope node with the real camera"
    writes:
      - crates/scope-node/README.md
      - crates/scope-node/src/imaging/mod.rs
      - crates/scope-node/src/imaging/probe.rs
      - crates/scope-node/src/main.rs
      - crates/scope-node/tests/imaging_probe.rs
    no_write_reason: ""
    shared_writes: []
    open_questions: []
```
