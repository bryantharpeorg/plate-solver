# plate-solver

The vocabulary of plate-solver: the solver that works out where a camera points from the stars in its frame, and the finder console that uses it to find, reach and follow objects with a telescope.

## Language

### Observing

**Target**:
The one object the observer has chosen to go to next. Objects the console proposes are suggestions until one is chosen.
_Avoid_: desired object, goal

**Site**:
Where the telescope stands on Earth, as latitude and longitude (east positive).
_Avoid_: location, observer

**Observing session**:
One night's shared state at the telescope (the site, the target, and later the camera, mount and calibration), the same for every phone and laptop connected to the console.
_Avoid_: settings, per-device state

**Pointable**:
Of a target: at or above the minimum altitude and outside the Sun guard right now, so the console will point the telescope at it.
_Avoid_: reachable, visible

**Minimum altitude**:
The lowest altitude the observer will point at, one number for the whole sky (default 15°). Below it a target is too low; below 0° it is below the horizon.
_Avoid_: altitude limit, horizon limit

**Sun guard**:
The console's standing refusal to point at the Sun or move the telescope near it, and to make any motorised move while the Sun is up. Always on, with no override from the page or an MCP client. Under `finder --mount-sky --mount-sky-night`, started with `--covers-on`, it checks the Sun of the night clock: the observer has confirmed the telescope is covered.
_Avoid_: sun warning

**Push-to**:
Moving the telescope by hand, following the console's directions from where it points to the target.
_Avoid_: manual slew

**Turn**:
A direction of motion in azimuth: clockwise or anticlockwise, as seen from above. The
page prints a clockwise turn as "right" and an anticlockwise one as "left", as the
observer faces where the telescope points.
_Avoid_: pan

**Tilt**:
A direction of motion in altitude: up or down.
_Avoid_: raise, lower

**GoTo**:
The console bringing the target to the aim point with the mount's motors: slew, solve, correct, until the target is within tolerance.
_Avoid_: slew (a GoTo usually takes several)

**Slew**:
One motorised move of the mount toward a position.
_Avoid_: GoTo (the whole closed-loop procedure)

**Sync**:
Tying the mount to the sky with one plate solve: a solved star frame from the scope node together with the mount's axis positions at that moment, taken while both axes are nearly still. Frames uploaded by hand are not syncs.
_Avoid_: align, alignment (alone: the finder alignment turns the finder camera on the tube)

**Pointing model**:
The fitted relation between the mount's axis positions and where the telescope points on the sky. Every sync refines it, and it replaces the hand controller's star alignment.
_Avoid_: star alignment, mount model, alignment (alone: see Finder alignment)

**Base tilt**:
How far, and toward which azimuth, the mount's base is off level (TPoint's AN and AW). Fitted once two syncs lie at least 90° apart in azimuth.
_Avoid_: tilt (alone: that is a direction of motion), level error

**Zero points**:
The azimuth and altitude axis readings that correspond to true north and the horizon (TPoint's IA and IE).
_Avoid_: offsets, home position

**Zero-point epoch**:
A stretch of the night over which one set of zero points holds. A slip opens a new one.
_Avoid_: session, alignment (alone: see Finder alignment)

**Fixed terms**:
The pointing model's terms that belong to the telescope rather than the night (collimation, axis non-perpendicularity, tube flexure, the finder camera's rotation on the tube), kept across nights.
_Avoid_: permanent model

**Slip**:
A sync far enough from the model's prediction that the axes must have moved without their readings following, as after a push by hand or a power cycle.
_Avoid_: lost alignment

**Apparent altitude**:
The altitude with atmospheric refraction added, where the telescope must actually point. Pointability is judged on the geometric altitude.
_Avoid_: refracted altitude

**Latest pointing**:
Where the most recent solved frame points: its J2000 RA and Dec at the aim point, the frame's time on the console's clock, and its altitude and azimuth at that time when a site is set.
_Avoid_: current position, last solve

**Tracking**:
The mount following the target across the sky so that it stays at the aim point.
_Avoid_: tracking mode (that is the solver's hinted solving)

**Tracking error**:
How far the sky at the set point is from the target, as east and north arcseconds at a stated time. While a stack runs it comes from each sub's imaging solve and steers the tracker; otherwise the tracker steers on the finder camera's live pointing.
_Avoid_: guide error, guiding, drift (that is how far the target moves during one sub)

**Followed push**:
A push by hand of more than the push threshold while Track is on, after which Track holds the place the telescope was left at until the observer turns Track on for the target again.
_Avoid_: drift, pushed target

**Aim point**:
The spot in the finder camera's frame where the main telescope is looking: the centre of the imaging camera's sensor, or of the eyepiece's view when no imaging camera is attached. The console measures it from an imaging solve and a live pointing of the same moment; without an imaging camera the observer sets it by tapping a centred star. Moves bring the aim point, not the boresight, onto the target.
_Avoid_: telescope boresight, frame centre

**Chart rotation**:
How far the page turns every live view of a frame, in degrees counter-clockwise, set in Setup to match how the finder camera sits in its holder. A display setting: marks, taps and the aim point stay in the frame's own pixels.
_Avoid_: turn (the base's motion in azimuth), roll (the solve's angle to north)

**Imaging angle**:
How far the imaging camera's frame is turned from the finder camera's frame, about the line of sight. Measured together with the aim point.
_Avoid_: camera rotation (the finder camera's rotation on the tube is a fixed term), roll (that is a solve's angle from celestial north)

**Set point**:
Where on the imaging camera's sensor the imaging run wants the target, and where a sub's tracking error is measured from. The sensor's centre unless the run names another place.
_Avoid_: lock position, guide position, aim point (that is a spot in the finder camera's frame)

**Imaging run**:
The console's whole procedure from a chosen target to a saved stack: GoTo, tracking, the focus check, centring the target on the imaging camera's sensor, and stacking subs until it is stopped.
_Avoid_: session (that is the observing session), sequence, capture, job

### Equipment

**Finder console**:
This project's app: it shows where the telescope points and what is there, and moves it to targets. "The console" for short.
_Avoid_: Finder (alone), the app, the server

**Finder camera**:
The camera that gives the console its view of the sky, attached to the scope node, whether it looks through the finder scope or through its own lens.
_Avoid_: guide camera, finder (alone)

**Finder scope**:
The small optical scope on the telescope tube. Named only when the optics matter.
_Avoid_: finder (alone)

**Imaging camera**:
A camera at the main telescope's focuser, used for photographs. The scope node takes its exposures and relays them; the console saves, solves, calibrates and stacks them.
_Avoid_: main camera

**Scope node**:
The small computer riding on the telescope that runs the finder camera and the mount link: it finds the stars in each frame and carries out the console's axis commands.
_Avoid_: the Pi (that is just today's hardware), agent, rig

**Simulator**:
A stand-in scope node, for a desk or CI: a simulated mount, deliberately off level and with an aim-point offset, and star fields drawn from the catalog for wherever it points.
_Avoid_: mock, emulator

**Scope link**:
The token-paired gRPC link (`scope_node.v1`) between the finder console and the scope node.
_Avoid_: agent link, RPC

**Star frame**:
One exposure as the scope node sends it: its id, exposure start and end on the node's clock, the settings it was made with and its centroids.
_Avoid_: frame (alone), image

**Preview**:
An occasional small JPEG of a star frame's image, for the page to show under the chart, or a colour JPEG of an imaging frame, for the Imaging page and MCP clients.
_Avoid_: thumbnail, live view

**Recording**:
One folder of what the console saw between the observer's start and stop: each star frame's centroids, settings and axis counts, each live solve and each preview, kept for later tests.
_Avoid_: capture, log, session

**Raw frame**:
The 8-bit gray pixels of one star frame, exactly as the scope node's detector read them, saved on request as a PNG next to that frame's line, or measured by the finder camera's focus aid.
_Avoid_: raw (alone), RAW file, full-resolution preview

**Imaging frame**:
One exposure from the imaging camera as the scope node relays it: its samples exactly as the camera delivered them, before any colour processing, with its exposure start and end on the node's clock and the settings it was made with.
_Avoid_: frame (alone), image, star frame (the finder camera's), raw frame (the finder camera's 8-bit pixels)

**Sub**:
An imaging frame taken for a stack.
_Avoid_: light, light frame, subframe

**Dark**:
An imaging frame taken with the telescope covered, to record the sensor's own signal. A **master dark** is the average of a set taken with the same settings.
_Avoid_: dark frame (in code and specs; fine in prose to the observer)

**Flat**:
An imaging frame of an evenly lit field, to record vignetting and dust shadows. A **master flat** is the average of a set, less its own dark.
_Avoid_: flat field

**Offset level**:
What a sample reads with no light and no time to gather any: the median of the **offset frame**, a dark of the camera's shortest exposure that the console takes before the darks it means to average. Those darks are checked against it.
_Avoid_: bias (alone), bias frame, pedestal, black level

**Flat label**:
The short name the observer gives the flats taken through one optical train, kept with their master flat.
_Avoid_: flat name, profile, train (alone)

**Calibration library**:
The console's master darks and master flats, each kept with a record of how it was made: the camera, gain, offset, binning and set temperature, and for a master dark its exposure.
_Avoid_: calibration set, dark library

**Bad-pixel map**:
The samples of an imaging camera's sensor that read high whatever the light, found in a master dark and kept with it. A sub's bad pixels are replaced from neighbours of their own colour before the sub is turned into colour.
_Avoid_: defect map, hot-pixel list, cosmetic correction

**Stack**:
The picture built from a run's subs: each calibrated, turned into colour, rotated and shifted into place by its imaging solve, and averaged with outliers rejected.
_Avoid_: live view, integration, master light

**Reference sub**:
The first sub a stack accepts. Every later sub is rotated and shifted onto its grid by the two subs' imaging solves.
_Avoid_: reference frame, key frame, master

**Imaging solver**:
The part that works out where an imaging frame points. Today it is the ASTAP program.
_Avoid_: plate solver (alone: the finder camera's solver is this project's own)

**Solver port**:
The hardware-neutral way the console asks an imaging solver for an imaging solve or a star measure. A **solver adapter** speaks one solver's own interface behind it, such as ASTAP's or the simulator's.
_Avoid_: driver, backend

**Imaging solve**:
Where an imaging frame points: the J2000 RA and Dec of its centre, the angle of its up direction east of celestial north, and its scale in arcseconds per pixel.
_Avoid_: plate solve (alone), WCS

**Focus reading**:
The median size of the stars in an imaging frame's focus region, as a half-flux diameter: in arcseconds, or in sensor pixels until the console knows the scale; for the finder camera, of the stars the scope node found in one of its raw frames, in the finder camera's pixels. Smaller is sharper.
_Avoid_: focus score, HFD (alone), FWHM

**Focus region**:
The part of the imaging camera's frame where focus readings are taken: the tile of a full frame with the smallest reading, or a tile fixed by hand.
_Avoid_: focus box, ROI (alone)

**Focus record**:
The smallest settled reading this imaging camera has given on this telescope: tonight's, and the earlier nights'. A reading is judged against the smaller of the two, or against tonight's alone once the focus aid has passed through focus tonight; never against an absolute number.
_Avoid_: best focus, focus target

**Focus aid**:
A loop of short exposures of the focus region, or of the finder camera's raw frames beside the live solve, each with its focus reading, the best so far and the trend, shown while the observer turns the focuser.
_Avoid_: autofocus (there is no motor), focus mode

**Clock offset**:
Node time less console time, taken from the least-delay heartbeat exchange of the last 8 s.
_Avoid_: clock skew, drift

**Live pointing**:
A latest pointing made from a star frame, timed at its mid-exposure mapped to the console's clock.
_Avoid_: live position

**Mount port**:
The hardware-neutral way the console moves a mount: two axes, each able to report its position and capabilities, go to a position, move at a rate and stop.
_Avoid_: driver, mount API

**Mount adapter**:
The part that speaks one mount's own protocol behind the mount port, such as Sky-Watcher's motor-controller protocol or the simulator's.
_Avoid_: driver

**Camera port**:
The hardware-neutral way the scope node takes frames from a camera: a stream of star frames from the finder camera, and exposures on request from the imaging camera.
_Avoid_: driver

**Camera adapter**:
The part that speaks one camera's own interface behind the camera port, such as the Arducam's, the simulator's, or INDI's for an imaging camera.
_Avoid_: driver, plugin, provider

**Axis position**:
An axis's own reading of where it is, in the mount's frame rather than the sky's. The pointing model turns axis positions into a direction on the sky.
_Avoid_: mount alt/az, encoder alt/az

**MCP client**:
A program that calls the console's MCP tools over the house LAN, such as Claude on the Surface or an app on the phone. It needs no token, and passes the same guards as the page, so it can move the telescope nowhere the page could not.
_Avoid_: agent, the AI (alone), bot

**MCP tool**:
One action the console offers MCP clients, passing through to one of the console's own actions or one of the scope link's mount calls.
_Avoid_: command, endpoint

**Direct axis move**:
Moving an axis to an axis position, or at a rate for a set time, asked for by axis rather than by target. Only MCP clients ask for one; it needs a sync and passes the same guards as a GoTo.
_Avoid_: raw move, manual slew

**Purpose label**:
The short word an exposure is asked for with, saying what it is for: `frame` and `probe`, and later `focus`, `dark`, `flat`, `sub`, `run` and `align`.
_Avoid_: frame type (that is INDI's), kind

**Lease** (on the imaging camera):
The console's record of which one of its procedures is taking exposures with the imaging camera: the focus aid, a set of darks or flats, the stack, the imaging run or the finder alignment. A second taker is refused with the holder's name. A single imaging frame holds none.
_Avoid_: lock, reservation, mutex

**Finder fix**:
One live solve of the finder camera as the console's history keeps it: when its star frame was taken, and how it maps the sky onto the finder camera's frame.
_Avoid_: sample, sync (that ties the mount to the sky)

**Star measure**:
The median half-flux diameter, in pixels, and the star count that a solver adapter reports for an imaging frame or a region of one.
_Avoid_: focus reading (that is in arcseconds, for the focus region), HFD (alone)

**Aim reading**:
One reading of the aim point and the imaging angle, taken from an imaging solve and the finder camera's live solves of the same moment. The console smooths the last few into the aim point it uses.
_Avoid_: measure (that is the star measure), sample, calibration

**Relation** (between the cameras):
What the console keeps between the finder camera and the imaging camera: the scale an imaging solve measured, the aim readings held, and from them the aim point and the imaging angle.
_Avoid_: alignment (the finder alignment is the procedure, not what is kept), calibration, offset

**Solve hint**:
What the console tells the imaging solver before an imaging solve: where to look, how far around it, and how tall the field is, each when known.
_Avoid_: seed, guess, prior

**Solve sidecar**:
The file beside a saved imaging frame that records its imaging solve, or why it failed. The imaging frame's own file is never rewritten.
_Avoid_: header update, WCS file

**Finder alignment**:
Imaging frames taken while the observer turns the finder camera's ring screws, with Track off, each showing where the imaging camera's centre lies from the finder camera's centre, also off the finder camera's frame, until it holds in the middle of the frame.
_Avoid_: relation (that is what the console keeps between the cameras), collimation, calibration

**Focus check**:
Up to nine imaging frames of the focus region taken without the observer, ended by the first settled reading and a verdict against the focus record.
_Avoid_: autofocus (there is no motor), focus test

**Settled reading**:
The median of three focus readings taken one after another at one focuser position. The focus aid shows it, and only it can set the focus record.
_Avoid_: average, smoothed HFD

### Solving

**Centroid**:
Sub-pixel `(y,x)` location of a detected star.

**Saturated star**:
A star whose brightest pixels read 250 or more, the finder camera's one saturation level, so its top is flat. 051's focus aid measures none; the scope node's detection keeps one that cedar-detect's gates set aside, by its brightness, when it is round or lies along the frame's trail, dark around and whole in the frame.
_Avoid_: blown star, overexposed star

**Pattern**:
A group of 4 stars used as a matching unit.

**Pattern key / geometric hash**:
The 5 sorted, quantized, normalized edge ratios of a pattern; the content-addressable, rotation- and scale-invariant fingerprint.

**Edge**:
Angular distance between a pair of pattern stars (6 per pattern).

**Largest edge**:
Max of the 6 edges; the normalizer and an angular-size (→FOV) proxy.

**Hash table / pattern catalog**:
Open-addressed array mapping pattern-key hash → pattern.

**Probing**:
Open-addressing collision resolution: quadratic (`hash+c²`) or linear (`hash+c`).

**Verification star**:
A catalog star kept densely for Stage-B match counting.

**Attitude / rotation matrix**:
3-axis orientation relating camera and sky frames.

**Boresight**:
The camera optical axis direction (image center). The eyepiece's centre is the aim point, not the boresight.

**Roll**:
Rotation about the boresight relative to celestial north.

**FOV**:
Field of view (horizontal unless stated); **diagonal FOV** = `fov·√(w²+h²)/w`.

**Proper motion**:
Slow apparent stellar motion; catalogs give epoch position + annual rate.

**Lost-in-space**:
Solving with no prior attitude (the mode implemented here).

**Hinted solving**:
Solving a frame with a prior estimate of where it points, such as the previous frame's attitude; the opposite of lost-in-space.
_Avoid_: tracking mode

**Hot pixel**:
Defective sensor pixel bright in isolation; must not be mistaken for a star.

**Cluster-buster**:
Density thinning so dense clusters (e.g. Pleiades) don't monopolize pattern generation/selection.
