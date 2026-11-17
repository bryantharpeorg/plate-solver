# An imaging camera on an alt-az Dobsonian: camera, INDI, ASTAP, focus, rotation, stacking

Research notes gathered on 2026-11-15 for specs 030 to 037 (one ask for a target, answered with a live-stacked colour picture). Each fact cites a source from the list at the end. The decisions taken on the strength of these notes are in [ADR-0002](../adr/0002-imaging-cameras-through-indi-frames-relayed-console-stacks.md) and `docs/plans/2026-11-15-specs-030-037-imaging.md`; this note holds facts, not decisions.

**Worked example.** A 4944 × 3284 colour sensor with 4.78 µm pixels (a ZWO ASI071MC Pro) at the prime focus of a 406 mm, 1800 mm, f/4.43 Newtonian on an alt-az GoTo mount. Nothing in the specs is keyed to it.

**Markers**
- **(est.)**: an estimate from stated assumptions. **(inferred)**: a deduction or arithmetic from cited facts.
- **[search]**: seen only as a search engine's extract, because the page refused a direct read. A lead, not a fact. **not found**: looked for and not established; collected in § 13.
- All arithmetic was recomputed for this note. Timings taken on machines used during the research appear as shares and ratios only; budgets are for the tests and bring-up probes to measure.

**Derived numbers for the worked example (inferred)**
- Scale 0.548″/px. Field 45.1′ × 30.0′ (0.752° × 0.4997°), 0.376 square degrees. Half-diagonal 2,968 px = 14.2 mm = 27.1′.
- One full-resolution 16-bit imaging frame is 32,472,192 bytes (32.5 MB, 31.0 MiB). Binned 2 × 2 it is 2472 × 1642 at 1.10″/px, 8.1 MB.
- Seeing of 2″ / 2.5″ / 3″ / 4″ is 3.7 / 4.6 / 5.5 / 7.3 px. Untracked, the sky crosses the sensor at 27 px/s.

## 1. The camera

**Sensor** [1][2]
- Sony IMX071 colour CMOS, 23.6 × 15.6 mm, rolling shutter, 14-bit converter, exposures from 64 µs to 2000 s.
- A 256 MB buffer in the camera holds about 7.9 full 16-bit imaging frames (inferred). The maker says it removes the amp glow that slow USB 2.0 read-out caused on the older model, and offers a 300 s dark as evidence.
- Full well 46 ke⁻ (manual); 48.6 ke⁻ at gain 0 on the maker's chart. Read noise from the chart: 3.27 e⁻ at gain 0, 2.75 at 50, 2.57 at 100, 2.26 at 200 and above. Peak quantum efficiency: the maker "supposes" over 50%; no measured curve.
- Dark current 0.00064 e⁻/s/px at −15 °C (maker's claim). A 10 s sub collects 0.006 e⁻ per pixel, so at these lengths a dark removes fixed pattern, not thermal signal (inferred).
- Bayer pattern RGGB, from a secondary page only [3]; read it from the camera. The window is AR-coated BK7 with no infrared cut (§ 10). Working range −5 to 45 °C and 20 to 80% relative humidity.

**Gain, offset, white balance**
- Ranges are not printed; the camera reports minimum, maximum and default for each control at run time [4].
- Maker's chart: unity gain at setting 94, 2.97 e⁻/ADU at gain 0, full well about 15 ke⁻ at gain 100 [2]. Forum figures say 90 [3][search]. Manual: lower gain for dynamic range, higher gain for lower read noise on short exposures [1].
- Offset is reported as fixed in firmware on this model, at 50 or 65 (owners disagree) [3][search]. Read it.
- The red and blue white-balance controls scale the raw samples. INDIGO's notes say the camera's own "neutral" values give long exposures a coloured background, and advise 50 and 50 with balance done in processing [5]. Set them explicitly and record them (inferred).
- Whether 16-bit output carries the 14-bit value multiplied by 4: not found. It decides every saturation threshold.

**Binning** [1][4]
- In the sensor: bin 3 only, digital, 10-bit, whose "only advantage" is frame rate. In the maker's library: software bin 2, 3 and 4, which the maker recommends. It averages in 16-bit mode (brightness unchanged) and sums in 8-bit mode.
- Binned raw output from a colour sensor stays a mosaic (same-colour pixels combined) unless the library's "mono bin" control is on. The maker does not say so in text; INDI's and INDIGO's camera programs both keep the Bayer tag on binned raw frames (inferred; test on the camera) [5][9].
- Region rule: width a multiple of 8, height a multiple of 2, both in binned pixels. The full sensor fits at bin 1, 2 (2472 × 1642) and 3 (1648 × 1094); at bin 4 it becomes 1232 × 820 (inferred).
- Software binning runs on the host after the USB download, so it shortens nothing on the cable; only a smaller region does (inferred).

**Formats and read-out** [1][4]
- Four formats: raw 8-bit, raw 16-bit, RGB 24-bit and 8-bit luminance. Which ones a camera offers is read at run time.
- Frames a second on USB 3.0 and on USB 2.0: 10 and 2.7 for the full sensor; 28 and 18.8 for 1920 × 1200; 33 and 31.3 for 1280 × 1080; 70 and 69.5 for 640 × 480. The USB 2.0 figures can only be 8-bit output (§ 11).
- The library has a single-exposure mode (start, poll, fetch) and a video mode. The maker's deep-sky app, INDI and INDIGO use the first. Each single exposure has a setup time, with no figure published. Stopping an exposure can block for 0.5 to 4 s while the transfer finishes.
- One independent study, of a small uncooled camera on one host, saw the library reset the USB port once per single exposure, and video stalls on USB 3.0 [6]. Not tested on this class of camera.

**Cooler and power** [1][2]
- Two-stage cooler, 35 to 40 °C below ambient. The set temperature is in whole degrees; the reading is in tenths [4].
- The cooler runs from 12 V: "12V at 2A Max", a 3 to 5 A supply recommended, an 11 to 15 V battery accepted; 0.5 A holds 30 °C below ambient. The fan runs only with the cooler.
- The electronics draw about 500 mA at 5 V from USB, "nearly the maximum" of a USB 2.0 port. With 12 V connected the camera and its hub are powered from it. Some cooled cameras of this make need cooler power connected for reliable exposures even with cooling off [5].
- Window heater: about 6 W by the maker's page, 3.6 W by a dealer [3][search]; switched in software.
- Cooling "won't help with very short exposure such as less than 100ms". At exposures of seconds its use is a repeatable set temperature for darks (inferred).
- Linux caps user-space USB buffers at 16 MB in total by default, less than one full 16-bit frame. The maker's udev rule raises it to 200 MB and INDI's to 1024 MB [7].

## 2. INDI

**Server and protocol** [8]
- `indiserver` is one program that starts a device program for each device (`indi_asi_ccd` for this make of camera, `indi_simulator_ccd` for the simulator) and relays XML between them and its clients, on TCP port 7624 (`-p`) and a local socket (`-u`, default `/tmp/indiserver`).
- It restarts a device program that crashes or exits with an error, up to 10 times by default (`-r`). It disconnects a client that falls more than 128 MB behind (`-m`): about three full-resolution imaging frames as base64 (inferred).
- Protocol version 1.7. A client sends `getProperties`, `enableBLOB` and `newNumberVector`, `newSwitchVector`, `newTextVector`; a device answers with `def…Vector`, `set…Vector`, `message` and `delProperty`. Each vector carries its device, name, a state (Idle, Ok, Busy, Alert) and a UTC timestamp. Each number carries its minimum, maximum and step, which is how ranges are read from the camera.
- INDI core and the ZWO device program are LGPL-2.1-or-later; the maker's library inside is a closed binary under an MIT-text licence [4][11]. A client speaks XML over a socket and links none of it (inferred).

**Standard camera properties** [8], with the ZWO program's own from [9]

| Property | Elements | Notes |
|---|---|---|
| `CONNECTION`, `DRIVER_INFO` | `CONNECT`, `DISCONNECT`; `DRIVER_INTERFACE` and three more | The second holds a bitmask of what the device is; the camera bit is 2 [10] |
| `CCD_INFO` | `CCD_MAX_X`, `CCD_MAX_Y`, `CCD_PIXEL_SIZE`, `CCD_PIXEL_SIZE_X`, `CCD_PIXEL_SIZE_Y`, `CCD_BITSPERPIXEL` | Pixels, microns, bits |
| `CCD_CFA` | `CFA_OFFSET_X`, `CFA_OFFSET_Y`, `CFA_TYPE` | Text, such as RGGB. "Debayering performed at client side" |
| `CCD_EXPOSURE`, `CCD_ABORT_EXPOSURE` | `CCD_EXPOSURE_VALUE`; `ABORT` | Seconds; setting it starts one exposure |
| `CCD_FRAME`, `CCD_BINNING` | `X`, `Y`, `WIDTH`, `HEIGHT`; `HOR_BIN`, `VER_BIN` | Region of the sensor; binning |
| `CCD_FRAME_TYPE` | `FRAME_LIGHT`, `FRAME_BIAS`, `FRAME_DARK`, `FRAME_FLAT` | Sets the header's frame type |
| `CCD_TEMPERATURE` | `CCD_TEMPERATURE_VALUE` | °C. Read: the sensor. Write: the set temperature |
| `CCD_COOLER`, `CCD_COOLER_POWER` | `COOLER_ON`, `COOLER_OFF`; `CCD_COOLER_VALUE` | Switch; percent, read-only |
| `CCD_CONTROLS` | `Gain`, `Offset`, `WB_R`, `WB_B`, others | ZWO program; names come from the maker's library |
| `CCD_VIDEO_FORMAT` | One per format | ZWO program; raw 16-bit unless another was saved. Core's documents name `CCD_CAPTURE_FORMAT` |
| `CCD_TRANSFER_FORMAT`, `CCD_COMPRESSION` | `FORMAT_FITS`, `FORMAT_NATIVE`; `INDI_ENABLED`, `INDI_DISABLED` | FITS by default; fpack compression |
| `UPLOAD_MODE`, `UPLOAD_SETTINGS` | `UPLOAD_CLIENT`, `UPLOAD_LOCAL`, `UPLOAD_BOTH`; `UPLOAD_DIR`, `UPLOAD_PREFIX` | Send to the client, save beside the server, or both |
| `CCD1` | `CCD1` (a BLOB) | The exposure |

**Behind them, in the ZWO program** [9]
- Setting `CCD_TEMPERATURE` switches the cooler on and writes the target. Temperature and cooler power are polled every second. Disconnecting does not switch the cooler off.
- `CCD_BINNING` is a range from 1 to the largest supported value, not a list, and one value serves both axes. `CCD_FRAME` is rounded down to a width of a multiple of 8 and a height of a multiple of 2; read it back (inferred).
- The exposure range comes from the maker's library. A camera with a serial number gets a `Serial Number` text (`SN`). 16-bit samples pass through from the library unshifted.
- It writes `GAIN`, `OFFSET`, `WB_R` and `WB_B` into each FITS header. It leaves white balance as the camera reports it and does not reload gain and offset on connect; a client should set every control (inferred).

**How an exposure arrives** [8][10]
- BLOBs are off until the client sends `<enableBLOB>Also</enableBLOB>` (or `Only`).
- `CCD_EXPOSURE` goes Busy; on completion the device sends `setBLOBVector` `CCD1` holding a FITS file. Over TCP it is base64 text: 43.3 MB for a 32.5 MB frame (inferred).
- From INDI 1.9.7 a client on the server's local socket can take "fast BLOBs": the buffer is passed by file descriptor, marked `attached='true'`, with no base64. Linux only, same machine only, and the client must answer each `pingRequest` with a `pingReply`. Receiving a descriptor in Rust without `unsafe` means a crate that wraps it (inferred): a dependency to name.
- The header INDI writes: `EXPTIME`, `DATE-OBS` (UTC start of the exposure, by the device program's clock), `INSTRUME`, `XBINNING`, `YBINNING`, `PIXSIZE1`, `PIXSIZE2`, `CCD-TEMP`, `FRAME`, `IMAGETYP`, `BAYERPAT`, `XBAYROFF`, `YBAYROFF` and `ROWORDER` = `TOP-DOWN`. The end of the exposure is not stamped.
- Two header cards other programs read: `XORGSUBF` and `YORGSUBF`, a sub-frame's origin "relative to whole frame in binned pixel units", and `SET-TEMP`, the "CCD temperature setpoint in degrees C. Absent if setpoint was not entered." [47]
- FITS stores 2880-byte blocks of big-endian samples, and unsigned 16-bit data as signed with `BZERO` = 32768 [12]. Turning that into little-endian samples loses nothing, and a single-picture 16-bit file needs no library to write (inferred).

**The camera simulator** [10]
- `indi_simulator_ccd`, device name "CCD Simulator", is part of INDI core. Frames are 16-bit; `SIMULATOR_SETTINGS` holds `SIM_XRES` and `SIM_YRES` from 512 to 8192 (default 1280 × 1024), pixel size, `SIM_MAXVAL`, noise, sky glow, limiting magnitude, and `SIM_TIME_FACTOR` (0.01 to 100), which scales how long an exposure takes.
- It has `CCD_GAIN`, `CCD_OFFSET`, a cooler whose temperature ramps between −15 and 25 °C, and a focus simulation (`SIM_FOCUSING`, seeing 3.5″ by default). `CCD_DIRECTORY_LOCATION` and `CCD_DIRECTORY_TOGGLE` make it serve the FITS files of a folder in place of rendered ones.
- `SIM_TESTS` has `BAYER`, `CRASH` (the program aborts) and `TIMEOUT` (an exposure never completes).
- It cannot simulate colour (`BAYER` only tags grey data as RGGB (inferred from the source)); nebulosity; any star without the external GSC catalogue program, whose absence it reports as an error; or a ZWO camera's property names (gain and offset are separate properties here, elements of `CCD_CONTROLS` there).
- Alt-az field rotation is in no release. It arrived on the development branch on 2026-11-02, is computed only when a snooped mount reports alt-az, and is not a setting.
- So it can prove a client's conversation and its handling of a crash and a timeout; rotation, colour and solvable fields need this project's own simulator (inferred).

**Failure handling and versions** [9][10][11]
- The ZWO program tries up to 3 times, 100 ms apart, to start an exposure; polls every millisecond in the last 0.2 s; and restarts a failed exposure once, two attempts in all, before reporting failure. A failed start sets `CCD_EXPOSURE` to Alert. It has no overall deadline for an exposure, so a client needs its own (inferred).
- One further step, off by default (`USB_RESET`), closes the camera, power-cycles its USB port, reopens it, restores the settings and tries once more. It needs extra permissions.
- It sets the library's USB bandwidth control to 40 on connect. INDIGO lists the causes of failed exposures it has seen: bandwidth too high, cooler power not connected, poor cables, hubs, and a weak supply to a small board [5]. A release of the maker's library once broke short exposures on USB 3.0 until reverted [6]: record the library's version.
- Debian stable carries INDI core 1.9.9, the ZWO program of December 2022 (contrib) and the maker's library 1.27 (non-free), for amd64 and arm64. Upstream is core 2.2.5 (2026-10-01), ZWO program 2.8, library 1.41; its 2025 and 2026 work on failed exposures and the USB reset is not in Debian's build.
- A Rust client exists: `indi` 5.1.1, MIT OR Apache-2.0, async, with a web-server stack among its dependencies. Whether it takes fast BLOBs: not found.

## 3. ASTAP

**Program and platforms** [13][15]
- A solver and stacker in Free Pascal, MPL-2.0. `astap_cli` is the solver alone, with no window: upstream builds v2026.11.01 for x86-64, aarch64 and armhf.
- Debian stable has `astap-cli` 2024.05.01 for amd64 and arm64 (testing: 2025.10.08). It predates "solver improved for star poor images" (2026.06.29) and "multi-threading for blind solving" (2026.10.30). Debian packages no star database.

**Command line** [13]
- Where to look: `-f file` (FITS, TIFF, PNG, JPG); `-ra` in **hours**; `-spd`, declination + 90°; `-fov`, the picture's **height** in degrees (0 = automatic); with `-fov 0`, "ASTAP will try all FOV between 10 degrees and 0.3 degrees."; `-r`, "radius of the square search pattern around the start position", in degrees.
- Tuning: `-z` downsample (0 = automatic); `-s` most stars to use (typically 500); `-t` quad tolerance (typically 0.007); `-m` smallest star size in arcseconds, to drop hot pixels; `-check y/n` filters the Bayer pattern of a raw colour picture, at 1 × 1 only; `-speed slow` overlaps the search fields more. The same page, among the preconditions of its sky-brightness (SQM) report rather than in the option list: "DSLR/OSC raw images require 2x2 binning."
- Files: `-d` database folder; `-D` database name, such as `d50`; `-o` output base name; `-wcs` also writes a FITS-conforming `.wcs`; `-log`; `-update` writes the solution into the input file.
- Measuring: `-analyse snr` prints `HFD_MEDIAN=` and `STARS=` on standard output (Linux) without solving; `-extract snr` writes every detected star to a CSV. The page's example prints `HFD_MEDIAN=3.3` and `STARS=666`.
- It reads a start position and scale from the FITS header when present (`RA`, `DEC`, `OBJCTRA`, `FOCALLEN`, `XPIXSZ` and others); "Command-line parameters have priority above fits header values."
- With `-z 0`, "any image with a height dimension above 2500 pixels will be binned 2x2", which also merges each Bayer cell of a full-resolution imaging frame (inferred).
- It takes a picture, not a star list. `-focus1…` (a focus curve from four or more pictures) is not in `astap_cli`.

**Output and exit codes** [13]
- An `.ini` beside the input, always: `PLTSOLVD=T` with `CRPIX1/2`, `CRVAL1/2`, `CDELT1/2`, `CROTA1/2`, `CD1_1` to `CD2_2`, `CMDLINE` and any `WARNING`; or `PLTSOLVD=F` with `ERROR`. A `.wcs` only on success.
- Exit codes: 0 solved, 1 no solution, 2 not enough stars detected, 16 error reading the picture, 32 no star database found, 33 error reading the database, 34 error updating the input file.
- The up direction and whether the picture is mirrored follow from the `CD` matrix. FITS puts the bottom row first by convention, while INDI tags its files `ROWORDER` = `TOP-DOWN` [10]: a spec must pin the row order it writes and prove the angle against a saved capture or the simulator's truth (inferred).

**Star databases** [14]
- Extracts of Gaia DR3 at a fixed density, named by hundreds of stars per square degree; epoch 2025, no proper motions, magnitudes Gaia BP.
- D05 serves fields of 0.6° to 6° (101 MB); D20 0.3° to 6° (393 MB); D50 0.2° to 6° (867 MB); D80 0.15° to 6° (1.2 GB). "Field" is the picture's height.
- The worked example's height is 0.4997°, on ASTAP's line: its automatic choice takes D80 below 0.5° when D80 is installed, and its checklist says "for small field-of-view (<0.5°) expose longer up to 60 seconds and use the D80 star database" [13]. That advice is written for small apertures (inferred). Name the database with `-D`.
- The databases carry no named licence; their readme asks for the credit "ESA/Gaia/DPAC". ESA's own page today puts Gaia data under CC BY-NC 3.0 IGO.

**Requirements and limits** [13]
- "It requires a minimum about 30 stars in the image to solve." "Correct image height in degrees specified within preferably 5% accuracy."
- "Saturated stars will be ignored." "Heavily stretched, saturated, out-of-focus or photo shopped images are problematic." Streaked stars are ignored and the solve can fail.
- "If a global cluster fills the whole field, ASTAP could struggle to solve. Forcing option 'slow' could help." Nothing is said about bright nebulae.
- It matches quads of nearby stars: "Rotation, scaling, and flipping of the image have no influence", so it needs no starting angle. On failure it moves the search field outward in a square spiral.

**Time** [13][16]
- The author's only published timings are for a start 90° away: 4.8 s using 100 stars to 23.8 s using 500, on unstated hardware. The page says a 30° radius is searched "in a minute" in one place and "a few minutes" in another.
- One user on a small ARM board reported about 15 s on average at default settings for a 40 MB file of unstated field; a poster who appears to be the author answered "within 5 or 10 seconds".
- A time for a solve started close to the answer on a field under one degree: not found.

**As a star measure** [13]
- `-analyse` reports "the image median HFD" in pixels, and a star count. Stars with a half-flux diameter above 20 "will not be analysed correctly"; test pictures "should not contain a large bright deep-sky object"; streaks read low.
- Whether its automatic 2 × 2 binning applies to the measure, and how it treats a raw colour mosaic: not found.

## 4. Star counts and limiting magnitude at this field

**Mean stars in one imaging field** (0.376 square degrees), Gaia DR3, brighter than G = [18]

| Sky | 12 | 13 | 14 | 15 |
|---|---|---|---|---|
| Galactic poles | 8 | 16 to 18 | 30 to 36 | 56 to 65 |
| Galactic latitude 30° | 15 to 23 | 36 to 48 | 78 to 97 | 152 to 194 |
| Latitude 15° | 32 to 46 | 74 to 106 | 160 to 226 | 332 to 463 |
| The plane, seven longitudes | 31 to 82 | 75 to 219 | 145 to 555 | 265 to 1,246 |
| The actual field on M42, two orientations | 73 to 80 | 125 to 140 | 205 to 233 | 368 to 414 |
| The same less a 6′ circle on the Trapezium | 50 | 82 | 125 | 226 |
| Orion, off the nebula | 26 | 47 | 87 | 168 |

- Cross-check: a 1981 NASA report gives 71.5 stars per square degree of V 9 to 14 near the north galactic pole, 26.8 per field, against 29.8 from Gaia to G 14.
- Counts in the plane are lower bounds: Gaia is incomplete there from about G 11 [search].

**Worst fields** [18]
- North galactic pole, 176 real fields, as mean / 5th percentile / fewest: 31.4 / 21 / 19 stars to G 14 and 56.5 / 44 / 36 to G 15. South pole: 35.2 / 25 / 18 and 63.1 / 50 / 47. To G 12 the fewest is 2.
- At the north pole 38% of fields hold fewer than 30 stars to G 14.
- Dark clouds, one field each (ρ Ophiuchi, Taurus, Perseus): 18 to 33 stars to G 14 and 30 to 68 to G 15.
- So ASTAP's 30 stars need a reach of about G 15 in the sparsest sky, and a database at least that deep (inferred).

**Limiting magnitude and saturation (est.)**
- No published figure exists for this aperture and sensor at these exposures.
- Photon budget: 1,214 cm² of clear aperture (25% obstruction by diameter), two reflections at 0.9, mean detection efficiency 0.16 to 0.20 over 400 to 700 nm, 3.3 e⁻ read noise, 4″ stars, a 19.5 mag/arcsec² sky, signal-to-noise 10.
- Result: G 15.6 to 15.8 in 2 s, 16.4 to 16.6 in 5 s, 17.0 to 17.1 in 10 s. Uncertain by about a magnitude, and true only near the centre of the field (§ 6, coma).
- With the same budget the peak pixel of a 4″ star saturates near magnitude 6.6 in 2 s and 8.3 in 10 s at gain 0, and near 7.7 and 9.5 at unity gain. In 2″ seeing at unity gain: 9.2 and 11.0.
- The M42 field holds 25 stars brighter than G 10 and 43 brighter than G 11 [18]: the stars a brightness-ordered matcher takes first are the ones that saturate.

## 5. Relating two cameras

**What existing systems store** is a two-number offset, set once by a person.
- Celestron StarSense: centre a star in the eyepiece and press a key; it stores "the offset of the star in the camera". Redone when the camera is "bumped, jarred, or rotated" [19].
- Nexus eFinder: centre a bright star and press OK; the offset is saved in degrees, and redone if the telescope is re-collimated or the finder remounted [19].
- PiFinder and Cedar: pick or tap the star centred in the eyepiece; the pixel is kept [19].
- SharpCap: click, in the second camera's picture, the object centred in the first. CCDciel: enter the pixel by hand or solve one finder picture [20].
- KStars and N.I.N.A. keep no offset between cameras; they solve with the imaging camera itself [20].
- None documents a stored angle between two cameras, and none re-measures by itself during the night. An INDI forum answer of 2018: "because of the rotation caused by that offset, that would perhaps be tricky to implement" [20].
- Large telescopes keep a pointing model for every instrument. One 2.5 m telescope measures a finder-to-telescope zero point at the start of each night and models how the finder sags with zenith distance: 110″ in one axis, 75″ in the other [21].

**Why three numbers here (inferred)**
- Two solves of one moment give each camera's attitude. The fixed relation between them is a rotation with three degrees of freedom: where the imaging camera's centre falls in the finder camera's frame (two numbers) and the angle between the frames (the third).
- Two numbers steer a target to the centre. The third predicts which way the imaging frame is turned on the sky, which stacking on an alt-az mount and any check of an imaging solve's angle need.
- Before the first measurement the angle is unknown: the camera sits at any angle in the focuser. So the first solve must not need an angle (§ 3).
- "Same moment" matters. While tracking the field turns and drifts, so the two solves are reduced to one instant or taken with overlapping exposures.
- The aim point inherits the finder camera's fitted field of view and distortion at that pixel.

**How steady the relation is**
- A sideways shift of the focuser end against the mirror's axis moves the imaging field by 1.9′ per millimetre at 1800 mm (inferred). A finder riding on the same end does not see it.
- A measured figure for a collapsible or truss Dobsonian: not found. "One to several arcminutes from horizon to zenith" is a guess with nothing behind it; size any search window from a measurement.
- Every imaging solve that shares a moment with a live pointing is a fresh measurement of the relation, at no cost (inferred).
- From the owner's field note of 2026-11-14, outside this repository: a finder solve scatters by about 0.3′ to 0.5′ and a finder pixel is about 36″. Those are 33 to 55 and 66 imaging pixels (inferred): enough to land a target in the imaging field, not to centre it or to see a sub trail.

## 6. Focus

**Metrics** [22]
- Half-flux diameter (HFD): "the diameter in pixels that contains half the energy in a star image". "Much more robust in the presence of seeing and noise" than a fitted width; it handles stars so far out that they are rings; and it "varies linearly with focus position, except very close to best focus".
- For a Gaussian it equals the full width at half maximum, 2.3548 σ. The common shortcut (flux-weighted mean distance) reads 6.4% high. A blank or noise-only patch returns about 1.4 times the aperture radius, so a failed detection reads large, not zero.
- Fitted widths assume a peak, which an obstructed Newtonian out of focus does not have (inferred). SharpCap recommends its contrast and Fourier measures for the Moon and planets, not star fields [23]. Star count peaks at focus and needs no size measure [24].

**Computing it** (PHD2's source [22], and what changes here)
- Background: mean and sigma in an annulus around the star, clipped at ±2σ; pixels under mean + 3σ are left out. In nebulosity the background must be local (inferred).
- Sort pixels by distance from the centroid; the radius where the running flux crosses half the total, interpolated, is the half-flux radius.
- A star is flagged saturated at the camera's known maximum, or by a flat top. SharpCap leaves saturated stars out, "as it is impossible to tell how over-saturated they are", wants about 20 stars and reports a placeholder under five [23]. Ekos has a ring mask that keeps centre or edge stars only [24].
- A fixed 7 px aperture cannot read above 9.9 px; in-focus stars here are already 4.6 to 5.5 px, so the aperture must grow with the star (inferred).
- Colour sensor: N.I.N.A. says raw Bayer data "can lead to small focus error" and recommends measuring after demosaicing [25]. ASTAP's author, on another measurement of raw colour pictures, trusts 2 × 2 binning over extracting green [17]. No source settles it for star sizes.

**Sizes at this telescope (inferred)**
- A focus error of 1 mm makes a disc 47 px across. The half-flux diameter of that ring (25% obstruction) grows by 0.034 px per µm, 1.9″ per 100 µm.
- Critical focus zone, 4.88 λ N² at 0.55 µm: 53 µm in total. The "new" zone for 3″ seeing and 15% growth: 209 µm in total [26].
- A 5 px star with focus off by 50 / 100 / 200 µm / 1 mm reads +6% / +21% / +70% / 7 times larger. On a 2.5″ star, growth of 10% is 61 µm of focus error and 30% is 110 µm. These add ring and seeing in quadrature: a model no source measured.
- Coma without a corrector, 645 h / (D N³) arcminutes [26]: 3″ at 574 px from the optical axis, 8.6″ at the middle of the long edge, 12.9″ at the middle of the short edge, 15.5″ (28 px) in the corner.
- Field curvature (radius equal to the focal length [26]): the corner focuses 56 µm from the centre, as much as the whole 53 µm zone.
- A full millimetre out, stars are 34 px across; ASTAP's measure is unreliable above 20 px and its solver calls out-of-focus pictures a problem (§ 3).

**What a reading can and cannot say**
- Seeing sets a floor of 2″ to 3″ on an average night and 5″ or more on a poor one; a scatter of 0.4 to 0.5 px between frames is reported [46][search]. One frame cannot resolve the 53 µm zone (inferred).
- A reading cannot tell focus error from seeing, tube currents, a mirror still cooling or miscollimation. "Best seen so far, with a rise on both sides" is the only verdict the data supports; an absolute "in focus" is not (inferred).
- The sharpest part of the field is where the optical axis lands, not the sensor's centre: 1 to 3 mm of miscollimation moves it 210 to 630 px (inferred). One reviewer of this telescope says collimation holds only with the struts seated to "the last millimetre" [29].
- The telescope's manual expects refocusing as a short-focal-ratio mirror cools [27]. A measured in-focus size for this telescope and camera, and the focuser's travel per turn of its knob: not found.

**How tools guide a hand on the focuser**
- SharpCap: a history of bars, green for better and red for worse than recent readings, the best since reset marked, a line through the last 10; for deep sky "setting the Average Over option to 1 frame is usually sufficient"; reset when conditions change [23].
- N.I.N.A.: a looped exposure with star size and count plotted over the last 100 [25]. Ekos: a time series of sizes with current, previous and first profiles overlaid, and 1 s exposures by default [24].
- "Wait a second or two after each focuser change." "Usually the focusing exposures of 1s or even less give better results" [24].
- A 1920 × 1200 region reads out faster than once a second even on USB 2.0 (§ 11), so a loop on a region is limited by the exposure (inferred). Whether such a region holds 20 unsaturated stars in 1 s: not found.

## 7. Field rotation, sub length and the drive

- The owner's recollection (2026-11-15, from memory, not a measurement): with the hand controller's own star alignment done carefully, this telescope held stars well enough for exposures of about 20 to 30 s with this camera. So the drive itself can do better than the 4 to 15 s most reports below give. Whether the console's own tracker matches it is untested.

**Rate.** Field rotation = 15.041°/h × cos(latitude) × cos(azimuth) / cos(altitude), azimuth from north [30]. It agrees with the numerical derivative of the parallactic angle and with `docs/research/2026-10-21-pointing-model-and-tracking.md` § 3. Zero due east and west, greatest on the meridian, unbounded at the zenith. A point r pixels from the rotation centre moves at r × the rate in radians.

**M42 at transit (inferred)**; the corner is 2,968 px (1,625″) from the centre.

| Latitude | Altitude | Rate | Corner speed | Longest sub for 1″ / 2″ of corner smear | Rotation in 30 / 60 min centred on transit |
|---|---|---|---|---|---|
| 30° N | 54.6° | 22.5°/h | 0.32 px/s | 5.6 / 11.3 s | 11° / 22° |
| 35° N | 49.6° | 19.0°/h | 0.27 px/s | 6.7 / 13.3 s | 9.5° / 19° |
| 40° N | 44.6° | 16.2°/h | 0.23 px/s | 7.8 / 15.7 s | 8° / 16° |

- At 35° N the rate two hours from transit is about two thirds of the transit figure, and three hours from it 43%.
- Any target on the meridian at 35° N: 2″ of corner smear in 17.8 s at altitude 30°, 10.3 s at 60°, 7.0 s at 70°, 3.6 s at 80°, 1.8 s at 85°.
- The limit in arcseconds does not change with binning; in binned pixels it doubles. Smear falls linearly toward the centre. Without a corrector the same corner carries 28 px of coma (§ 6).
- A published worked example allows 7 px in the corners of an APS-C sensor and gets 21 s [30].

**What rotation does to a stack kept on the first sub's grid (inferred)**
- Share of the frame covered by every sub, and the largest centred crop of the same shape with full coverage: 95.6% and 79% of the area after 5° of rotation; 91.9% and 64% after 10°; 85.9% and 47% after 20°; 81.2% and 38% after 30°.
- A canvas padded to the frame's diagonal (5,936 px square, 2.2 times the memory) loses nothing.
- Between subs the corner turns by 14 to 19 px a minute (M42 at transit, 40° to 30° N).

**Reported sub lengths on this class of mount**
- The only directly read report: a 16-inch, 1800 mm Dobsonian of the same optical figures, with a coma corrector and 8.4 µm pixels (0.96″/px), took 18,800 subs of 1 s and kept 18,000, with "no guiding" [31].
- Owners of 12- to 16-inch GoTo Dobsonians report 4 to 15 s as usual, "around 5 second subs" at 0.43″/px, and 20 s as the most before stars go oblong; they blame tracking before rotation [31][search]. Which thread said which is unsafe.
- Smart telescopes on alt-az mounts: one caps subs at 10 s and removes rotation in stacking; another uses 10 s; a third recommends its equatorial mode from 15 s and requires it from 30 s [32][33].
- A measured tracking error for this family of mount: not found. The maker states GoTo accuracy "up to 5 arc min" [27].

**The drive** [27][28]
- Servo motors with an encoder on each motor, and a coarse one on each axis (11,748 counts per revolution, 110″ a count). The maker publishes 1,620,000 motor counts per revolution; the protocol says the figure "might be different for the two axes" and is to be read from the controller.
- In slow motion the controller advances its reference position one count per timer interrupt. Tracking commands are therefore a staircase of single counts; what the tube does in answer is unmeasured (inferred).
- Altitude reverses at transit, and the protocol requires a full stop to change direction. Near transit the altitude rate grows by only 0.0014″/s² (35° N), so 1′ of backlash would take about 5 minutes to cross (inferred).
- From the owner's field note of 2026-11-14, outside this repository:
  - The controller reports 1,980,000 counts per revolution on both axes: 0.6545″ a count, 1.19 imaging pixels (inferred).
  - Moves of 1° or less have run on the real mount. Tracking, timed rate moves, a GoTo to a target and any move over 1° have not.
  - On 1° moves the sky moved 1′ to 2′ less than commanded: backlash or a scale error of 2 to 3%, not separated.
  - The controller's firmware is older than the version a third-party author names for smooth variable-rate tracking (`docs/research/2026-10-21-synscan-mount-control.md` § 3).

**Tracking error against the frame (inferred)**
- 2 px of trail in a 5 s sub is 0.22″/s, 1.5% of the sidereal rate.
- `specs/015-tracking/spec.md` designs for a 1% rate error, ignores errors under 10″ and accepts 20″ RMS. At this scale those are 0.27 px/s across the whole frame (the same speed as rotation at the corner), an 18 px dead band and 36 px RMS.
- So the smear within a sub is set by the tracking loop at least as much as by rotation, and subs of one target can sit tens of pixels apart.

## 8. Live stacking

**Order of steps** in the open stackers: dark (or bias) subtract, flat divide, bad-pixel repair on the mosaic, demosaic, register, resample, accumulate; then background, colour balance and stretch for the display only [34][37][35]. The commercial smart telescopes publish neither their order nor their registration model.

**How they register**

| Stacker | Registration | A sub is accepted when | Source |
|---|---|---|---|
| SharpCap | Star matching; shift and rotation | 3 stars, 10 to 15 preferred | [23] |
| Siril, live | Star matching; shift only unless `-rotate`, then similarity | 10 star pairs | [34] |
| ASTAP, live (window only) | Quads; a six-coefficient linear fit | 3 quad matches | [13] |
| ALS | Triangles on the green channel; similarity | 25 matches by default | [39] |
| AstraLite | Triangles from the 50, 70 or 100 brightest stars; shift and angle | 10 similar triangles | [37] |
| N.I.N.A. Live Stack add-on | Quads and triangles, RANSAC, similarity or affine | 8 inliers and 20% of stars; RMS 1.0 px; stars spread in two dimensions; the two fits agree within 1 px at a corner | [40] |
| OpenLiveStacker | No stars: rotation computed from site, time and target; shift by FFT phase correlation | The shift stays within a step limit | [35] |
| KStars Live Stacker | Each sub's own astrometric solution; a grid mapped sub to sky to reference; a rigid fit | The fit's determinant is within 0.0025 of 1; 80% of grid points agree; the centre moved under half the short side | [36] |

- A sub that fails is dropped and the stack kept, in every stacker read; none rescues one. OpenLiveStacker and one smart telescope count subs taken and subs stacked apart [35][45].
- ASTAP starts a new stack when the pointing moves 0.2° or the exposure changes; KStars when the target or filter changes [13][36].

**Registering from each sub's solve, as KStars does (inferred)**
- A 0.2 px error at the corner allows 14″ (0.004°) of error in a sub's angle and 0.11″ in its centre.
- With N matched stars of centroid scatter s over the field (RMS radius 1,713 px), a fitted centre is good to s/√N and an angle to s/(1,713 √N) radians.
- At the corner that is 0.06 px for s = 0.2 px and N = 50, and 0.13 px for N = 10. For s = 0.55 px (0.3″): 0.15 px and 0.35 px. Differencing two independent solves multiplies these by 1.4. Catalogue and distortion errors are common to every sub of one field and cancel.
- ASTAP's own example of residuals shows 0.48″ to 0.53″ per star, 0.2 px in that picture; "the error is default not reported" [17]. Its stacker prefers star matching and keeps the astrometric route for mosaics [13].
- How many stars ASTAP's final fit uses, and how well two of its solves of one field agree: not found.

**Which transform, and resampling**
- Refraction squeezes the field along altitude, which is fixed on the sensor while the sky turns. Simulated for M42 at 30° to 40° N (est.), a similarity (shift, angle, scale) leaves about 0.1 px at the corner over 30 minutes centred on transit, 0.2 px over 60 and 0.3 to 0.5 px over two hours. An affine fit leaves 0.01 px or less. Shift and angle without scale leave 0.5 to 0.7 px over an hour that starts 90 minutes past transit. Two independent simulations agree within about 20%.
- KStars chose its rigid fit as "much more stable for Alt-Az stacking than Homography" [36].
- Siril's live stack is bilinear in effect and AstraLite samples bilinearly; Siril offline defaults to Lanczos-4 with clamping [34][37]. Taking each 2 × 2 Bayer cell as one colour pixel gives 2472 × 1642 at 1.10″/px with no interpolation (inferred).

**Rejection**
- None: ASTAP's live stack is a running mean ("there is no rejection of poor images"), and Siril's a count-weighted mean of the stack and the new sub [13][34].
- Against a running statistic: SharpCap's "Sigma Clipped" compares each pixel with the running mean after 5 to 10 learning subs; KStars clips a first batch held in memory, then tests later subs against saved per-pixel statistics [23][36].
- In a small window: AstraLite holds a ring of 5 registered subs and accumulates their median; OpenLiveStacker drops the brightest value at each pixel [37][35].
- `seiza-stacking` describes "online residual (delta-sigma) rejection with coverage and rejection maps" (its README; not run) [38].
- Whole-sub gates: SharpCap discards subs with large stars and has a brightness filter for cloud [23]; AstraLite filters on star size and ovality [37]; one smart telescope reports "star trails detected … image discarded" without publishing its test [46][search].

**The satellite belt (inferred)**
- Geostationary satellites appear at declination −atan(sin φ / (6.611 − cos φ)): −5.0° from 30° N, −5.7° from 35° N, −6.3° from 40° N. M42 is at −5.4°, so from roughly 31° to 35° N the belt crosses a field centred on it, and from 30° to 40° N it passes within a degree.
- They are fixed over the ground, so they cross a tracked field at the sidereal rate: 273 px in a 10 s sub.
- A satellite of magnitude 11 to 13 leaves roughly 20 to 120 e⁻ per pixel along its streak, against a few e⁻ of noise (est.). An imagers' forum thread says the tracks "overlap heavily around Orion" [46][search].

**Coverage, display and what is saved**
- Dividing a sum by one count darkens the edges that later subs do not cover: the "darkening along edges" N.I.N.A. had to fix, and the reason a tutorial for one smart telescope's subs advises cropping [40][34]. ASTAP divides by the global count [13]; AstraLite keeps a count for each pixel [37]. Noise at a partly covered pixel is higher by 1/√(share covered) (inferred); shares are in § 7.
- SharpCap sets black, mid and white points automatically and balances colour from histogram peaks or star colours; its default stretch gives "over-saturated bright cores", and "bright targets such as M31 or M42 may be difficult to adjust correctly" [23].
- OpenLiveStacker stretches with asinh(a·x)/asinh(a) and removes a fitted linear gradient [35]. ASTAP fixes colour factors from the first sub [13].
- What is saved: a linear 16- or 32-bit FITS or TIFF, and a stretched 8-bit picture as a view of it; one smart telescope adds a JSON of subs taken against subs stacked [23][35][33].

**Compute cost.** A benchmark of one full-resolution sub (demosaic, star detection on a half-size luminance, triangle matching, bilinear resample, accumulate), as re-run by a second researcher:
- Star detection with matching is about a third of the time, the resample about a third, demosaicing and conversion to floating point about 7% each. Bicubic resampling costs 1.7 times bilinear, Lanczos-4 about 8 times.
- Four threads take the total to about 0.7 of one thread's. Binned 2 × 2, the total is about 0.4 of full resolution: detection and matching barely shrink.
- Not in it: rejection, flat division, bad-pixel repair, files and the network. A naive rejection pass written as separate array operations cost about 9 times everything else together; it has to be one fused pass (inferred).
- The first researcher's single-thread figures were three times larger and did not reproduce; they are dropped.
- Published timings from other people's laptops: 0.6 s a sub for a 26-megapixel camera binned 4 × 4 in the camera maker's program; 15 s unbinned and 1.5 s binned 4 × 4 in Siril on a 2011 machine [16].

**Memory at 4944 × 3284 (inferred)**
- Raw 16-bit 32.5 MB; 16-bit colour 97 MB; 32-bit float colour 195 MB; one float plane 65 MB.
- Sum and weight: 260 MB. With a variance for rejection: 455 MB. With the current sub in float and its resampled copy: 844 MB. Binned 2 × 2, a quarter of each.
- AstraLite's integer sum and count come to 227 MB, and its five-slot median to about 490 MB more. Its README asks for 1 GB at 12 megapixels, 2 GB at 24, and 4 GB with track removal [37]. KStars advises staying under half of the memory available [36].

## 9. Calibration frames

**What each removes, and in what order**
- A dark carries the offset level, hot and warm pixels and any fixed pattern, plus thermal signal, which is negligible here (§ 1). A dark of the sub's own exposure already contains the bias.
- A flat carries vignetting and dust shadows. It needs its own dark or a bias taken off first [35].
- A bad-pixel map comes from a master dark (Siril takes pixels more than 3.5 σ hot [34]) or from the sub itself (AstraLite has both [37]; one smart telescope repairs hot pixels with no calibration frame [46][search]).
- Order: dark, then flat, then bad pixels, then demosaic (AstraLite's call order [37]). OpenLiveStacker's arithmetic: `corrected_flats = flats − dark_flats`; `factor = max(corrected_flats) / corrected_flats`; `corrected = max(image − darks, 0) × factor` [35].
- On an alt-az mount a hot pixel lands on a different part of the sky in every sub, so in a stack it draws a coloured arc about the rotation centre; demosaicing first would spread it into its neighbours. It is repaired on the mosaic from same-colour neighbours (inferred).

**Matching and reuse**
- One smart telescope names its masters by exposure, gain, binning, temperature and the number averaged, adds new darks into the matching master, and suggests retaking beyond ±8 °C [33]. ASTAP picks a master dark by exposure, gain and temperature [13]. Owners of this make of camera say offset must match too [46][search].
- With a set temperature a library made once serves many nights: one maker has the user build a single "Universal Master Dark" and renew it about yearly [33].
- Another takes darks itself at the start of each night behind an internal dark filter [46][search]; its routine checks the mean pixel value against a low threshold before accepting them [45].

**Flats**
- They belong to the whole optical train and to the camera's orientation: turn the camera and they are stale [35].
- Level: "Avoid histogram reaching 100% or below 10%", and keep little of it above 80% or below 30% [23]. N.I.N.A.'s flat wizard takes three test exposures, extrapolates to a target mean and accepts within a tolerance ("20-30% should be typical"); the darks for the flats follow at the same exposure [25].
- On an alt-az mount vignetting stays fixed on the sensor while the sky turns, so an uncorrected stack shows a radial fall-off and smeared dust shadows (inferred).
- How evenly this telescope lights a sensor of this size: not found. A 2-inch drawtube 120 mm ahead of focus is narrower than the 56 mm an f/4.43 cone to the corner needs (est.).

## 10. Exposing the Orion Nebula, and colour

**Saturation**
- The Trapezium (est., good to a factor of two; assuming magnitudes 5.1, 6.7, 6.7 and 8.0, 31% throughput to a green pixel, Gaussian stars): the brightest star's peak pixel fills in 0.2 s in 2″ seeing to 0.8 s in 4″ at gain 0, and in 0.07 to 0.3 s at unity gain. The faintest of the four lasts 6.6 s at gain 0 in 3″ seeing.
- So at any sub length used for stacking the four stars have clipped cores. They stand 16 px or more apart (est.), so they stay separate in the data; it is the display stretch that merges them.
- A surface brightness for the nebula's core in magnitudes per square arcsecond: not found, so when the nebula itself clips here cannot be computed. Measure the share of saturated pixels in each sub (inferred).
- Per pixel and second this telescope and camera collect about 3.5 times the light of an f/5 smart telescope with 2.9 µm pixels (inferred), whose maker's tutorial uses 2 s on this target because "longer exposure times may clip the Core" [32].

**What people do**
- Short subs at low gain for the whole stack [32]. One report: 10 s at high gain for 11 minutes at f/6.3, where "the trapezium is resolved" with a newer stretch mode [41].
- Two stacks, a short one for the core and a long one for the wings, joined with a blurred mask [41].
- A stack of unclipped subs held in floating point never clips; only its display does (inferred).
- The core shows in one short sub; minutes bring out the surroundings; the faint outer dust takes "at least 30 minutes" on a small smart telescope [41], about 9 minutes here by the ratio above (est.).

**Colour**
- The window passes near-infrared, so every Bayer channel records some. A colour-true stack needs a UV/IR-cut filter ahead of the camera or a balance made for the unfiltered response (inferred). A maker's statement for this model: not found. With mirrors only, all colours focus together; a refracting coma corrector adds a small colour shift (inferred).
- Atmospheric dispersion separates blue from red by about 0.69″ × tan(zenith distance) (est., from the standard-air formula): 0.5″ to 0.7″ (0.9 to 1.3 px) at M42's transit altitude from 30° to 40° N, and 1.2″ (2.2 px) at altitude 30°. It lies along altitude, so it turns within the stack.
- Balance is done in processing, with the camera's white-balance controls left neutral (§ 1).

## 11. Frame transport

**Sizes (inferred)**
- Full resolution, 16-bit: 32.5 MB (31.0 MiB); as base64 inside INDI's XML, 43.3 MB; in chunks of 256 KiB, 124 chunks. Binned 2 × 2: 8.1 MB. A 1920 × 1200 region: 4.6 MB.

**USB** [1][42]
- The camera manual's USB 2.0 rates all come to 43 to 44 MB/s if the samples are 8-bit (4944 × 3284 × 2.7; 1920 × 1200 × 18.8; 1280 × 1080 × 31.3). As 16-bit they would be 87 MB/s, above the 53 MB/s ceiling of USB 2.0 bulk transfer [search]. So those rates are 8-bit and the cable carries about 44 MB/s (inferred).
- A full 16-bit frame therefore takes at least 0.74 s over USB 2.0: 1.35 a second at best. A 1920 × 1200 region manages about 9 a second (inferred).
- On USB 3.0 the 10 frames a second are the sensor's limit: about 0.1 s of read-out, then a transfer of 0.1 to 0.2 s (inferred).
- Each exposure also pays the library's unpublished setup time (§ 1).

**Onward from the scope node (inferred)**
- On a small board whose wired network port sits behind its one USB 2.0 bus, with a published ceiling of 300 Mbit/s [42], a full 16-bit frame needs at least 0.87 s on the wire, on the bus the camera is also using.
- At 20 to 50 Mbit/s a full frame takes 5 to 13 s and a 2 × 2 frame 1.3 to 3.3 s.
- INDI can compress before sending (`CCD_COMPRESSION`, fpack) [8]. What that costs on a small board: not found.

**Measured, and not**
- Nothing above is measured on the real camera or on any scope node. A download time for this camera on a small ARM board: not found in any source.
- INDI's ZWO package ships a benchmark of exposure latency by binning (`asi_camera_bench`) [9].

## 12. Prior art and what was not adopted

**Reaching the camera**
- The maker's library called directly. One maker; a closed binary with C++ inside that needs libusb and a C++ runtime; calls into it are foreign calls, which Rust marks `unsafe` (inferred). The Rust wrappers found are each one author's work with little use [44]. Its download address carries no version and has changed contents under one name [4].
- INDIGO. A BSD-style licence, JSON over WebSocket with frames fetched by URL, this camera on its tested list, and an imaging helper with star-size statistics but no stacking [5]. Not in Debian (inferred). ADR-0002 chose INDI; INDIGO would be another camera adapter.
- ASCOM Alpaca. An HTTP service in front of the same maker's library, so a second hop for one client; the Linux host that supports this make is AGPL [44].
- A reimplementation of the maker's USB protocol exists for six other models, not this one [44].

**Solving the imaging frame** [43]
- astrometry.net. GPL-2 code; 2.1 GB of index files for this field; takes a star list; its paper reports 99.7% of 9′ × 13′ fields solved within a second of processor time with the scale known to 1%. Not adopted: the owner chose ASTAP (ADR-0002).
- StellarSolver. GPL-3, a Qt library around the same engine and the same index files.
- This project's pattern database stretched to 0.75°: about 7.7 million stars and 4 to 6 GB held in memory (est.), hours to build, and no claim by its references below "a few degrees". The catalogues they read are complete to magnitude 10 or 11: two to four stars in this field.
- A solver of our own started from the finder's pointing. The open Rust example needs a full starting attitude, angle included, and a position good to a fraction of the star spacing, 2.6′ to 5′ here (inferred). Rejected in ADR-0002; the solver port leaves it open.

**Stacking**
- Siril driven as a program: GPL-3, each sub a round trip through files, no rejection across the stack, shift-only unless asked, and "experimental for the moment" in its own manual [34].
- OpenLiveStacker: GPL-3, a whole appliance that wants to own the camera; binaries for x86-64 only; its HTTP interface is documented by a page that disagrees with the code [35].
- ASTAP's live stack (window only, no rejection), ALS (a window program watching a folder), KStars Live Stacker (inside its viewer) and the N.I.N.A. add-on (Windows): methods and failure lists only [13][39][36][40].
- AstraLite: MIT Rust. Its stacker modules depend only on `rayon` and `itertools`, though the crate as a whole brings a windowing toolkit, so reuse would mean vendoring. Young, one maintainer, version 0.0.571 [37].
- `seiza-stacking` 0.61.0: Apache-2.0, star registration with rotation and scale, rejection with coverage maps, a `LiveStacker` type. First published 2026-07-22 with 72 releases since; it depends on a crate whose licence expression includes CC-BY-4.0, which `deny.toml` does not allow; its default feature pulls in a GPU stack. Not built or run [38].
- DeepSkyStacker: BSD-3-Clause, needs Qt; reference code for star registration only [38].
- The plan writes the stacker as this project's own code and leaves a trial of the two Rust stackers to the owner.

**From a chosen target to a stack**
- A teardown of one smart telescope's firmware, single-sourced and partly machine-written: a GoTo counts as done only when a solved exposure lands within 0.1° of the target; motorised autofocus runs inside the GoTo; darks are checked before stacking; each sub is reported as stacked or dropped; a client silent for 16 s loses its GoTo [45].
- Tolerances for "centred" elsewhere: 30″ in Ekos, 2′ in one MCP server, 0.1° above [20][45]. A solve loop that never converges was common enough that one controller added a forced stop [45].
- With a hand focuser the order changes: the focus check comes before the first imaging solve, because an out-of-focus picture may not solve (§ 3) (inferred).
- MCP servers for telescopes are young and thin; none offers one operation from target to stack. The most developed permission model is KStars's: loopback only, bearer tokens, a global read-only switch [36].

## 13. Not established

- Whether the camera's 16-bit output is the 14-bit value times 4; its offset value and whether it can be set; which formats and binnings it reports; what binned raw output looks like on its mosaic.
- The single-exposure setup time, and any measured download time for this camera over USB 2.0 or 3.0 on a small ARM board.
- What `FORMAT_NATIVE` sends for this make; whether the Rust `indi` crate takes fast BLOBs; how long compression takes on a small board; whether the maker's library is safe to call from several threads (no statement exists).
- ASTAP: its time for a solve started near the answer on a field under one degree; its reliability inside bright nebulosity; D50 against D80 at a 0.4997° height; how it measures stars on a raw colour mosaic and in which pixels it reports them for a tall picture; how many stars its final fit uses and how well two solves of one field agree; whether `astap_cli` writes the `// …` note its page shows after each value of the `.ini`; which row order its `CD` matrix assumes for a file tagged `ROWORDER` = `TOP-DOWN` (both to be read from saved captures).
- Whether 2 to 10 s subs reach G 15 across the field; the real limiting magnitude and star sizes of this telescope.
- Any measured drift between a finder and the main optical axis on a collapsible or truss Dobsonian; how evenly the telescope lights the sensor.
- The in-focus star size of this telescope and camera; the focuser's travel per turn; whether the camera reaches focus in the stock adapter; whether a coma corrector is in the train, which would change the focal length and scale.
- Any tracking measurement on this mount: rate error, backlash, behaviour at very slow rates, and whether its firmware tracks smoothly at changing rates.
- Why this mount's controller reports 1,980,000 counts where its maker publishes 1,620,000.
- The surface brightness of the Orion Nebula's core, hence the exposure at which the nebula, not its stars, clips.
- A maker's statement on infrared and colour for this window; the window heater's real power.
- How the commercial smart telescopes register, what their sub-rejection tests are, and how they treat rotated edges.
- Stacking time for a 16-megapixel sub on the console machine, with rejection: only shares from a benchmark without rejection exist.
- The link rate between scope node and console under real conditions. That, the download time and the stacking time are what the bring-up probes measure.

## Sources

[1] ZWO, ASI071 manual, revision 1.4 (August 2020): https://astronomy-imaging-camera.com/manuals/ASI071MC_Pro_Manual_EN.pdf
[2] ZWO's product page, its gain and efficiency charts, and a 2019 article: https://www.zwoastro.com/product/asi071mc-pro/ , https://astronomy-imaging-camera.com/wp-content/uploads/071Pro-Gain-RN-DR-FW-vs-gain.png , https://astronomy-imaging-camera.com/wp-content/uploads/ASI071-QE-e1509346837511.jpg , https://www.zwoastro.com/2019/08/28/asi071mc-pro-what-improvements-have-been-achieved/
[3] A secondary copy of the specification, https://astropixels.com/bifrost/ZWO_ASI071MC.html ; and, as search extracts only, https://www.cloudynights.com/forums/topic/626710-question-about-asi071mc-pro-cmos-offset/ , https://www.astrobin.com/forum/post/42662/ , https://www.telescopescanada.ca/products/zwo-asi071mc-pro-usb3-0-cooled-color-astronomy-camera
[4] The maker's camera library (header, licence and manual inside the download), its developer FAQ and its binning article: https://www.zwoastro.com/software/product-sdk/ , https://zwoastro.yuque.com/olyczd/brku76/htpsgng0xfegl2ym , https://us.zwoastro.com/blogs/product-information/everything-you-need-to-know-about-astrophotography-pixel-binning-the-fundamentals
[5] https://github.com/indigo-astronomy/indigo/blob/master/indigo_drivers/ccd_asi/README.md and https://github.com/indigo-astronomy/indigo
[6] https://github.com/UMa-Technology/asi-camera-linux-investigation
[7] https://github.com/torvalds/linux/blob/master/Documentation/admin-guide/kernel-parameters.txt (`usbcore.usbfs_memory_mb`) and https://raw.githubusercontent.com/indilib/indi-3rdparty/master/libasi/99-asi.rules
[8] INDI's documents: https://docs.indilib.org/indiserver/ , https://docs.indilib.org/protocol/ , https://docs.indilib.org/drivers/standard-properties/ , https://docs.indilib.org/drivers/basics/binary-transfers.html
[9] INDI's ZWO program, read 2026-11-15: https://github.com/indilib/indi-3rdparty/blob/master/indi-asi/asi_base.cpp , https://github.com/indilib/indi-3rdparty/blob/master/indi-asi/asi_camera_bench.cpp , https://raw.githubusercontent.com/indilib/indi-3rdparty/master/debian/indi-asi/changelog
[10] INDI core, read 2026-11-15: https://github.com/indilib/indi/blob/master/libs/indibase/indiccd.cpp , https://github.com/indilib/indi/blob/master/libs/indidevice/basedevice.h , https://github.com/indilib/indi/blob/master/drivers/ccd/ccd_simulator.cpp
[11] https://sources.debian.org/api/src/indi/ , https://sources.debian.org/api/src/indi-asi/ , https://sources.debian.org/api/src/libasi/ , https://raw.githubusercontent.com/indilib/indi-3rdparty/master/debian/indi-asi/copyright , https://github.com/indilib/indi/releases and https://crates.io/crates/indi
[12] https://fits.gsfc.nasa.gov/fits_standard.html
[13] ASTAP's pages and its live-stacking source: https://www.hnsky.org/astap.htm , https://www.hnsky.org/astap_astrometric_solving.htm , https://sources.debian.org/data/main/a/astap/2024.11.13-1/unit_live_stacking.pas
[14] https://sourceforge.net/projects/astap-program/files/star_databases/ , https://www.cosmos.esa.int/web/gaia-users/license and https://www.cosmos.esa.int/web/esdc/terms-and-conditions
[15] https://packages.debian.org/stable/astap-cli , https://tracker.debian.org/pkg/astap-cli and https://sources.debian.org/data/main/a/astap-cli/2025.10.08-1/debian/copyright
[16] INDI forum threads with timings: https://forum.indilib.org/forum/astroberry/6259-using-astap-solver/47860.html and https://forum.indilib.org/t/ekos-live-stacking/11833
[17] ASTAP forum threads: https://sourceforge.net/p/astap-program/discussion/suggestions/thread/97697564db/ and https://sourceforge.net/p/astap-program/discussion/general/thread/5619af600d
[18] Gaia DR3 counts queried on 2026-11-15 through two public mirrors, https://gaia.ari.uni-heidelberg.de/tap/sync and https://dc.g-vo.org/tap/sync ; NASA CR-3374, pp. 6–7, https://ntrs.nasa.gov/api/citations/19810007398/downloads/19810007398.pdf ; Gaia's validation notes (search extract only), https://gea.esac.esa.int/archive/documentation/GDR3/Catalogue_consolidation/chap_cu9val/sec_cu9val_943/ssec_cu9val_943_grvs.html
[19] https://www.kikkertland.dk/upload_dir/shop/StarSense_AutoAlign_Manual.pdf (p. 12), https://astrodevices.com/wp-content/uploads/2026/02/Nexus_eFinder_User_manual_1.0.pdf , https://pifinder.readthedocs.io/en/release/quick_start.html and https://raw.githubusercontent.com/smroid/cedar-server/main/about.md
[20] https://forums.sharpcap.co.uk/viewtopic.php?p=45629 , https://www.ap-i.net/ccdciel/en/documentation/finder_camera , https://kstars-docs.kde.org/en/user_manual/ekos-align.html , https://nighttime-imaging.eu/docs/master/site/advanced/platesolving/ and https://forum.indilib.org/forum/ccds-dslrs/3271-plate-solving-offset-for-secondary-camera.html
[21] https://www.ing.iac.es/Astronomy/observing/manuals/latex/general/int_user/userman/int_manual_v01/node149.html and https://gmt-docs.readthedocs.io/latest/SWC_architecture/tcs/pointing_kernel/virtual_telescope.html
[22] https://cdn.diffractionlimited.com/help/maximdl/Half-Flux.htm , https://en.wikipedia.org/wiki/Half_flux_diameter , https://www.lost-infinity.com/night-sky-image-processing-part-6-measuring-the-half-flux-diameter-hfd-of-a-star-a-simple-c-implementation/ and PHD2's source https://raw.githubusercontent.com/OpenPHDGuiding/phd2/master/src/star.cpp
[23] SharpCap's manuals: https://docs.sharpcap.co.uk/4.1/6_GettingGoodImages.htm and https://docs.sharpcap.co.uk/4.0/16_LiveStacking.htm
[24] https://kstars-docs.kde.org/en/user_manual/ekos-focus.html , https://www.astrophotography.app/usersguide/bahtinov_aid.htm and https://www.astrophotography.app/usersguide/auto_focusing_aid.htm
[25] N.I.N.A.'s documents: https://nighttime-imaging.eu/docs/master/site/advanced/autofocus/ , https://nighttime-imaging.eu/docs/master/site/tabs/imaging/ and https://nighttime-imaging.eu/docs/master/site/tabs/flatwizard/
[26] https://www.goldastro.com/goldfocus/cfz.php , https://www.goldastro.com/goldfocus/ncfz.php and https://www.telescope-optics.net/newtonian_off_axis_aberrations.htm
[27] https://cdn.shopify.com/s/files/1/0080/7095/5123/files/CollapsibleGoToDobsonian_manual.pdf and http://skywatcher.com/product/bk-dob-16-synscan/
[28] https://inter-static.skywatcher.com/downloads/skywatcher_motor_controller_command_set.pdf
[29] https://www.scopeviews.co.uk/SW400P.htm
[30] https://calgary.rasc.ca/field_rotation.htm and https://www.ianmorison.com/astrophotography-using-an-alt-az-mount/
[31] Read: https://discuss.pixls.us/t/m63-with-16-dobson-a7s-and-19000-images/19070 . Search extracts only: https://www.cloudynights.com/forums/topic/999967-skywatcher-400p-do-short-exposure-imaging/ , https://www.cloudynights.com/forums/topic/902752-astrophotography-with-a-sky-watcher-flextube-400p/ , https://www.cloudynights.com/forums/topic/617691-using-a-skywatcher-12-goto-dob-for-eaa/
[32] https://www.seestar.com/blogs/faq/s50 , https://www.seestar.com/blogs/tutorial/deep-sky-stack , https://www.seestar.com/blogs/tutorial/editing-the-orion-core-nebula-the-trapezium and https://i.zwoastro.com/wp-content/uploads/2025/06/4a9ec16c2b004b518b414facc5eab5e5.pdf
[33] https://support.vaonis.com/portal/en/kb/articles/how-vespera , https://support.vaonis.com/portal/en/kb/articles/vespera-pro-automatic-image-calibration , https://help.dwarflab.com/en/docs/DWARF-3-Smart-Telescope-User-Manual-Part1-App-Interface-Introduction and https://help.dwarflab.com/en/docs/How-to-View-and-Obtain-the-Files-on-DWARF-3
[34] Siril: https://siril.readthedocs.io/en/latest/Livestack.html , https://siril.readthedocs.io/en/latest/preprocessing/registration.html , https://gitlab.com/free-astro/siril/-/raw/master/src/livestacking/livestacking.c and https://siril.org/tutorials/seestar/
[35] https://github.com/artyom-beilis/OpenLiveStacker (commit 43748662) and https://github.com/artyom-beilis/OpenLiveStacker/wiki/Open-Live-Stacker-Manual
[36] KStars: https://kstars-docs.kde.org/en/user_manual/fits-viewer-livestacker.html , https://raw.githubusercontent.com/KDE/kstars/master/kstars/fitsviewer/fitsstack.cpp and https://kstars-docs.kde.org/en/user_manual/ekos-mcp.html
[37] https://github.com/art-den/astra_lite (commit 388caece, 2026-11-14)
[38] https://crates.io/crates/seiza-stacking , https://github.com/theatrus/seiza and https://github.com/deepskystacker/DSS
[39] https://github.com/deufrai/als (commit 5bad4c2f), https://github.com/quatrope/astroalign and https://arxiv.org/abs/1909.02946
[40] https://github.com/isbeorn/nina.plugin.livestack (commit cb178bc5)
[41] https://www.skyatnightmagazine.com/astrophotography/how-stop-overexposing-orion-nebula-core and https://forums.sharpcap.co.uk/viewtopic.php?p=34841
[42] One USB 2.0-only board's published limits, https://www.raspberrypi.com/products/raspberry-pi-3-model-b-plus/ ; USB 2.0's bulk ceiling (search extract only), https://developerhelp.microchip.com/xwiki/bin/view/applications/usb/speeds-specs/high-speed/
[43] https://astrometrynet.readthedocs.io/en/latest/readme.html , http://data.astrometry.net/ , https://ar5iv.labs.arxiv.org/html/0910.2233 , https://github.com/rlancaste/stellarsolver , https://tetra3rs.dev/concepts/tracking/ , https://raw.githubusercontent.com/smroid/cedar-solve/master/tetra3/tetra3.py and this repository's `README.md`
[44] https://ascom-standards.org/AlpacaDeveloper/Index.htm , https://github.com/open-astro/AlpacaBridge , https://crates.io/crates/generic-camera-asi , https://github.com/smroid/asi_camera2 and https://crates.io/crates/regain-zwo
[45] https://github.com/smart-underworld/seestar_alp (commit e8935cb8), https://github.com/zmerlynn/seestar-spec (commit 14ac005e) and https://github.com/veksa/seestar-mcp
[46] Forum pages seen as search extracts only: https://astropix.com/books/GDPI/CHAPTER1/105.HTM , https://forums.sharpcap.co.uk/viewtopic.php?p=17439 , https://www.cloudynights.com/forums/topic/1001639-seestar-s30-pro-star-trails-detectedcannot-stackimage-discarded-message/ , https://www.cloudynights.com/forums/topic/957659-photometry-with-the-seestar-s50/ , https://bbs.zwoastro.com/d/19027/4 , https://www.cloudynights.com/forums/topic/702967-retaking-dark-frames-different-offsets/ , https://app.astrobin.com/forum/topic/250276/acquisition/satellite-trails-ruining-subs-worth-a-planning-tool
[47] The SBIG FITS extension, https://diffractionlimited.com/wp-content/uploads/2016/11/sbfitsext_1r0.pdf ; MaxIm DL's FITS header definitions, https://cdn.diffractionlimited.com/help/maximdl/FITS_File_Header_Definitions.htm
