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
The console's standing refusal to point at the Sun or move the telescope near it, and to make any motorised move while the Sun is up. Always on, with no override from the page or an MCP client.
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
_Avoid_: align, alignment

**Pointing model**:
The fitted relation between the mount's axis positions and where the telescope points on the sky. Every sync refines it, and it replaces the hand controller's star alignment.
_Avoid_: star alignment, mount model, alignment

**Base tilt**:
How far, and toward which azimuth, the mount's base is off level (TPoint's AN and AW). Fitted once two syncs lie at least 90° apart in azimuth.
_Avoid_: tilt (alone: that is a direction of motion), level error

**Zero points**:
The azimuth and altitude axis readings that correspond to true north and the horizon (TPoint's IA and IE).
_Avoid_: offsets, home position

**Zero-point epoch**:
A stretch of the night over which one set of zero points holds. A slip opens a new one.
_Avoid_: session, alignment

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

**Aim point**:
The spot in the finder camera's frame on which the main telescope's eyepiece is centred. Moves bring the aim point, not the boresight, onto the target.
_Avoid_: telescope boresight, frame centre

**Chart rotation**:
How far the page turns every live view of a frame, in degrees counter-clockwise, set in Setup to match how the finder camera sits in its holder. A display setting: marks, taps and the aim point stay in the frame's own pixels.
_Avoid_: turn (the base's motion in azimuth), roll (the solve's angle to north)

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
A camera at the main telescope's focuser, used for photographs. The console only needs its sensor's size and pixel pitch, to report field rotation and exposure limits.
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
An occasional small JPEG of a star frame's image, for the page to show under the chart.
_Avoid_: thumbnail, live view

**Recording**:
One folder of what the console saw between the observer's start and stop: each star frame's centroids, settings and axis counts, each live solve and each preview, kept for later tests.
_Avoid_: capture, log, session

**Raw frame**:
The 8-bit gray pixels of one star frame, exactly as the scope node's detector read them, saved on request as a PNG next to that frame's line.
_Avoid_: raw (alone), RAW file, full-resolution preview

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
The hardware-neutral way the scope node takes frames from a camera.
_Avoid_: driver

**Camera adapter**:
The part that speaks one camera's own interface behind the camera port, such as the Arducam's or the simulator's.
_Avoid_: driver

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

### Solving

**Centroid**:
Sub-pixel `(y,x)` location of a detected star.

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
