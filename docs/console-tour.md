# A tour of the console page

The console is one web page with three views:

- **the console** (`/`): where the telescope points, and what to do next;
- **Setup** (`/setup`): the site, the scope node, the mount and the pointing model;
- **Imaging** (`/imaging`): the imaging camera, focus, calibration, the live stack and imaging
  runs.

Every open copy of the page, on a phone or a laptop, shows the same observing session, as does
an AI assistant connected over MCP. A move made from one shows up on the others.

It is built for one hand in the cold, glancing between the eyepiece and the screen:

- Where the telescope points comes first, and the proof (the matched stars drawn on the frame)
  second.
- Night mode keeps everything red and dim.

To try every part without hardware, run `finder --simulate` ([quick start](guide/quick-start.md#2-the-whole-system-simulated)).

## The top bar

Always visible, left to right:

| Control | What it does | When it shows |
|---|---|---|
| **Finder** | Home: the console view. | Always |
| Frame chip | Opens **Choose a frame**: open a frame from this device (JPEG, PNG, BMP or PNM, up to 32 MB), pick a sample, or set the field of view and detection threshold. | Always |
| **Target** | Opens **Choose a target**: search Messier, NGC, IC and common names from OpenNGC. Each result says whether it is pointable now, below the horizon, too low, inside the Sun guard, or needs a site. | Always |
| Site chip, **Setup** | Go to Setup. | Always |
| **Imaging** | Go to the Imaging view. | Once a scope node is paired |
| "Moved by an MCP client" | Says that an AI assistant is moving the telescope, and with which tool. | While an MCP client moves the mount |
| **Stop** | Stops both axes, Track, GoTo and any MCP motion. If the node doesn't confirm, it tells you to press again, then to switch off the mount's power. | Whenever the mount may move |
| **Night** | Red-on-black night mode. It follows the device's dark setting until you choose, and is remembered per device, not per session. | Always |

A **Drawn sky** banner appears on every view when the console runs with `--mount-sky`. It says
that the stars come from the mount's encoders, not the camera.

## The console view

### Start screen

Before any frame, **Chart a frame** offers **Chart the Orion sample**, **Open your own frame** and
a grid of sample frames.

### Where it points

After a solve, the header gives:

- the constellation;
- large RA and Dec (J2000);
- the field of view, roll and scale in arcseconds per pixel;
- the solve time, matched stars and residual.

When a frame doesn't solve, **Try again at 5σ** (or 3σ) retries with a lower threshold.

### The chart

The frame, drawn with its proof. Layer chips turn each layer on or off:

- **Grid**: the RA/Dec grid.
- **Matched**: catalog stars matched to the frame, with names.
- **Unseen**: catalog stars that should be visible but weren't detected.
- **Deep sky**: Messier, NGC and IC objects in the field.
- **Centroids**: the detected stars.

The chart also carries:

- a compass, a scale bar and the centre reticle;
- the target's reticle, or an arrow at the edge when the target is off the frame;
- the **aim mark**: where the eyepiece or imaging camera points in the finder's frame;
- an up and right mark, once a site is set;
- the imaging camera's field outline, once the aim point is measured.

While the scope node is live, the chart updates with every frame ("Live frame N"), and a photo
from the finder camera sits below it.

### Target guidance

Appears when a target is chosen. It shows:

- what the target is, and whether it is pointable;
- **push-to directions** from where the telescope points now: how far away the target is, and
  which way to turn and tilt (for example "right 2.7° · down 1.4°"), with the pointing's age;
- or the reason it can't be pointed at: the Sun guard, no site, below the horizon or the
  minimum altitude (with rise, transit and set times), or "Pointing stale" when no live solve
  has arrived for five seconds.

With a mount connected, it adds **GoTo** and **Track**:

- **GoTo** slews to the target and corrects from fresh solves until it is within 60″. Its status
  reads Slewing, Solving, Correction n, then On target with the distance, or the reason it
  failed. When a guard refuses, the button is disabled and shows the reason.
- **Track** keeps the target where it is in the field. It shows:
  - its state: armed and waiting for GoTo, waiting at the keyhole near the zenith, or tracking;
  - the tracking error (RMS over the last minute) and the field rotation;
  - the longest exposure before a pixel of smear, once the imaging camera's sensor is entered in
    Setup;
  - after a push by hand, **Recenter** to go back to the target;
  - when Track ends, the reason.

### Below the chart

- **In the field:** the named objects inside the frame. On a phone, a short roster also sits
  near the top.
- **What to look at next:** objects just off the frame, and objects like one in the field ("Like
  M42…") that are up now.
- **The same field in a sky survey:** Aladin Lite survey imagery of the solved field. This is
  the page's only online extra, and the rest works offline.

## Setup

In order down the page:

| Section | What you do there |
|---|---|
| **Site** | Type latitude and longitude (east positive), or use this device's location (HTTPS or localhost only). Save or forget it. |
| **Minimum altitude** | The lowest altitude GoTo and Track will point at, 0–80°, default 15°. |
| **Chart rotation** | Turn the live chart and photos to match how you see the finder. |
| **Scope node and camera** | Pair with a scope node (address, token) and set the finder camera's exposure, gain and detection threshold. Shows Connected or Disconnected, the camera model and stars per frame, and flags out-of-range values. |
| **Mount and pointing model** | The mount's state, board and firmware. The model's syncs, base tilt and RMS residual, and how many more syncs the fixed terms need. **Second sync** moves the telescope 90° or more in azimuth and solves again, to fit the base tilt. **Reset night's model** and **Reset fixed terms** start over. |
| **Imaging camera** | The imaging sensor's size and pixel pitch, used only by Track's longest-exposure readout. |
| **Camera field** and **Database range** | After the first star frame: the live field and the pattern database's range, with a warning when the field is near or outside it, or nothing has solved for 30 s. |
| **Aim point** | Tap a star centred in the eyepiece on the live photo. Shows whether the aim point was tapped or measured from the imaging camera. **Reset aim point** clears it. |
| **Recording** | Record the night's star frames for later replay and, while recording, save three raw frames. Shown once a node is paired. |
| **MCP** | The address to give an MCP client. Says when it is reachable only from this machine, when MCP is off, and which host names are allowed. |

## Imaging page

Below the Imaging run block, a line says when imaging isn't ready, for example "The scope node
is not connected" or "has no imaging camera". The blocks, top to bottom:

1. **Imaging run.** One button runs the whole sequence: slew, track, check focus, solve, centre
   the target, then stack.
   - Before a run: choose **Subs to stack** or **Minutes to stack** (default 120) and whether to
     **Keep every sub**. The page estimates the disk the subs will take.
   - During a run it shows the stage, subs stacked and dropped, and the time.
   - **Stop imaging run** ends it, after a confirming second tap. Track stays on; the top bar's
     **Stop** stops the mount.
   - At a focus pause: **Check focus again** or **Go anyway**.
   - If the last run didn't finish, the block says so.
2. **Camera.** The camera's model, sensor size, pixel size, bit depth and colour pattern, its
   sensor temperature and the cooler's state.
3. **Settings.** Exposure, gain, offset, binning, cooler and set temperature, showing only the
   controls the camera has. Two more settings:
   - a **forwarding limit** in KiB/s, which paces frames over the link;
   - **Also keep each imaging frame at the scope node**.
4. **Imaging frame.** **Take imaging frame** and **Abort exposure**, the frame's progress
   (exposing, receiving, saved), and a preview. In night mode, **Hold to see colour** shows the
   true colours for up to 10 seconds. This, and the same control on the stack, are the only
   exceptions to the red-only rule.
5. **Imaging solve.** **Solve imaging frame** gives the centre, orientation, scale and star
   count. The first solve keeps the scale and measures the aim point: where the imaging
   camera points in the finder's frame. A new aim point waits until Track is off.
6. **Focus.** Needs an imaging solver.
   - **Start focus aid** repeats short frames and reads focus from a star region, the sharpest
     tile or one you keep.
   - **Check focus** compares one frame with the focus record.
   - It shows the reading, the best so far, the trend and a verdict.
7. **Calibration.** Take and manage master frames.
   - **Take darks** with the telescope covered.
   - **Take flats** with a label for the optical setup, then **Take the flats' darks**.
   - The block says which masters the latest frame would get. **Remove** a master you don't want.
   - **Take them anyway** overrides a dark test that thinks light is getting in.
8. **Stack.** **Start stacking** and **Stop stacking** run a live stack outside a run.
   - Colour gains for red, green and blue, and a stretch, then **Apply colour**.
   - It counts dropped subs by reason (for example not solved, too few stars, soft, or taken
     while the mount was not steady).
   - The stacked picture, with Hold to see colour at night.
9. **Tracking from the subs.** Each sub's solve can steer Track.
   - It shows what is steering and why steering is paused, the tracking error and drift, and the
     longest exposure the tracking allows.
   - Settings: corner smear in pixels, **Steer the tracker from the subs**, and tuning.

The Imaging view refreshes once a second.

## Night mode

Night mode turns the whole page red on black and dims it, photos and the survey view included,
with no white flashes. It protects dark adaptation at the eyepiece. Hold to see colour on the
imaging frame and the stack is the one deliberate exception.

## Notices

On every view:

- a session error (for example, a refused setting);
- in the top bar, "The mount did not confirm Stop" and **Moved by an MCP client**;
- the **Drawn sky** banner.

On their own view:

- "That frame did not go through", with **Try again**, and **Pointing stale**, when live solves
  stop: the console view;
- the field warning: Setup;
- "The last imaging run did not finish": Imaging.

## For the details

[`crates/finder/README.md`](../crates/finder/README.md) has the exact behaviour behind every
control: API routes, refusal messages, file formats, imaging-run states and endings.
[`GLOSSARY.md`](../GLOSSARY.md) defines the terms used on the page.
