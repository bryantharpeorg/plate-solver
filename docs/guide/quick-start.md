# Quick start: no hardware needed

You can see the whole system working on one computer, with no telescope, camera or Pi.

- **Step 1** solves still frames.
- **Step 2** runs a simulated telescope: a finder camera, a mount and an imaging camera in the
  same process.

## Before you start

You need:

- **Rust 1.96 or later**, from <https://rustup.rs>.
- **`protoc`**, the Protocol Buffers compiler. The console links the scope-link protocol,
  which is generated at build time.
  - Debian or Ubuntu: `sudo apt install protobuf-compiler`. If the build can't find
    `google/protobuf/duration.proto`, also install `libprotobuf-dev`.
  - macOS: `brew install protobuf`.
- **Git**, to clone the repository.

You do **not** need Node: the web page is built and committed, and it is embedded in the
binary.

```sh
git clone https://github.com/bryantharpeorg/plate-solver.git
cd plate-solver
```

Run every command below from the repository root. The console's default pattern database and
sample frames are found by relative paths, so run it from the root or pass `--db` and
`--samples`.

## 1. Solve still frames

```sh
cargo run --release -p finder
```

The first build takes a while. Then the console prints the database it loaded, the
number of sample frames (21) and its address. Open <http://127.0.0.1:8080>.

1. Press **Chart the Orion sample**, or pick another frame from the grid.
2. The page shows where the frame points:
   - the constellation;
   - large RA and Dec;
   - the field of view, roll and scale;
   - how many stars matched.
3. The chart draws the frame with an RA/Dec grid, the matched catalog stars and the deep-sky
   objects in the field. The layer chips turn each one on or off.
4. Below the chart are **In the field**, **What to look at next** and **The same field in a sky
   survey**. The survey view needs the internet; everything else works offline.

**Open your own frame** solves a JPEG, PNG, BMP or PNM of up to 32 MB. The default database
covers fields of 10–30°. For a narrower camera, build a database first (see
[the solver](../solver.md#build-a-pattern-database)).

**Try push-to:**

1. Open **Setup** and type a site (latitude and longitude, east positive).
2. Press **Target**, search for an object such as `M42`, and pick it.
3. The guidance block says which way to turn and tilt the telescope from where the last frame
   points, for example "right 2.7° · down 1.4°".

## 2. The whole system, simulated

```sh
cargo run --release -p finder -- --simulate --sim-start 2026-01-16T02:00:00Z
```

`--simulate` starts a simulated scope node inside the console:

- a finder camera rendered from the star catalog;
- a two-axis mount;
- a colour imaging camera;
- an imaging solver that answers from the simulator's truth.

The simulated telescope uses its own state folder, so it never touches a real night's
session.

**Always give a night-time `--sim-start`.** The default is the current time. If it is daytime at
the simulated site (40° N, 75° W, unless you saved another site), the Sun guard refuses GoTo and
Track.

Then:

1. **Set the site.** In **Setup**, type latitude 40 and longitude −75, the simulator's sky.
   Alternatively, set your own site and restart the console; the simulator reads the site when
   it starts.
2. **Watch it go live.** The chart now updates on its own ("Live frame N"), with a photo from
   the finder camera below it.
3. **Check the model.** In Setup, **Mount and pointing model** shows the mount connected and
   one sync. Live solves become syncs automatically while the mount is still.
4. **Take a second sync.** Press **Second sync**: the telescope moves at least 90° in azimuth
   and solves again. The model now shows the base tilt.
5. **GoTo.** Pick a target such as M42 and press **GoTo**. Watch "Slewing…", "Solving…",
   "Correction n" and then "On target", with the distance in arcseconds.
6. **Track.** Turn on **Track**. The readout shows the tracking error and the field rotation.
7. **Imaging.** Open **Imaging** in the top bar. Take an imaging frame, solve it, try the focus
   aid, and start an imaging run. [The console tour](../console-tour.md#imaging-page) describes
   each block.

**Desk-only controls.** A few API routes exist only under `--simulate`:

- `POST /api/sim/telescope` pushes the telescope by hand or puts the eyepiece on a star.
- `/api/sim/frames` pauses or resumes the finder camera.
- `/api/sim/focus` defocuses the imaging camera, to exercise the focus aid.

[`crates/finder/README.md`](../../crates/finder/README.md) documents them.

## 3. Connect an AI assistant (optional)

The console serves MCP at `/mcp` on the same port, with 32 tools for pointing, the mount and
imaging. With the console running, add it to Claude Code:

```sh
claude mcp add --transport http finder http://localhost:8080/mcp
```

Then ask, for example, "what is the telescope pointing at?" or "go to M13 and start tracking".
Every move goes through the same guards as the page. The **MCP** section in Setup shows the
address to use from another device.

## Next steps

- [Console manual](console.md): running the console against a real scope node, imaging
  prerequisites, phones and WSL2.
- [Scope node manual](scope-node.md): putting the scope node on a Raspberry Pi.
- [First night at the telescope](first-night.md): the order of checks for a real night.
