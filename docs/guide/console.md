# Console manual

The console is the `finder` program. It does the following:

- Serves the web page.
- Solves finder frames and keeps the pointing model.
- Plans GoTo and runs Track.
- Saves, solves, calibrates and stacks imaging frames.
- Serves MCP for an AI assistant.

Today it runs on a laptop, and the scope node runs on the telescope. The planned Pi 5 rig runs
both on the telescope (see [architecture](../architecture.md#the-raspberry-pi-5-plan-tentative)).

Try it with no hardware first: [quick start](quick-start.md). This manual covers a real night.
[The console tour](../console-tour.md) describes every part of the page, and
[`crates/finder/README.md`](../../crates/finder/README.md) is the full reference: every API
route, setting, file format and MCP tool.

## Build it

You need Rust 1.96 or later and `protoc`, as in the [quick start](quick-start.md#before-you-start).

```sh
cargo build --release -p finder
./target/release/finder --help
```

**The page is embedded.** The binary is self-contained, apart from the pattern database, the
sample frames and, for imaging, ASTAP.

**Default paths are relative.** `--db` and `--samples` point into `reference-solutions/`, so
either run from the repository root or pass both paths.

**Platforms.** The console is tested on x86-64 Linux in CI and used on 64-bit ARM Linux under
WSL2. It needs nothing Windows-specific.

## Options

| Flag | Default | What it does |
|---|---|---|
| `--listen ADDR` | `127.0.0.1:8080` | Where the page, API and `/mcp` listen. `0.0.0.0:8080` opens them to your network; read [network exposure](../safety-and-security.md#network-exposure) first. |
| `--db PATH` | cedar's 10–30° database in `reference-solutions/` | The pattern database for finder frames. It must cover your finder camera's field. |
| `--samples DIR` | cedar's sample frames | Frames offered on the start screen. |
| `--state DIR` | `$XDG_STATE_HOME/finder` or `~/.local/state/finder` | The session (site, pairing, pointing model, settings) and recordings. |
| `--imaging-dir DIR` | `imaging/` inside the state folder | Imaging frames, runs, stacks and calibration masters. |
| `--astap PROGRAM` | none | ASTAP's command-line program, the imaging solver. Needs `--astap-db-name`. Not with `--simulate`, which brings its own solver. |
| `--astap-db-name NAME` | none | ASTAP's star database, such as `d50`. |
| `--astap-db DIR` | where ASTAP looks | The folder holding that database. |
| `--focal-length-mm MM` | none | Sizes the first imaging solve's field until a scale is measured. Give it when the imaging field is under about 0.3° tall. |
| `--simulate` | off | Runs a simulated scope node, mount, cameras and imaging solver, with its own state. See the [quick start](quick-start.md#2-the-whole-system-simulated). |
| `--sim-start UTC` | now | The simulated clock's start, for example `2026-01-16T02:00:00Z`. |
| `--mount-sky`, `--tube-alt DEG` | off | Daylight testing on the real mount: stars are drawn from the mount's encoders instead of the camera. |
| `--mount-sky-night UTC`, `--covers-on` | off | With `--mount-sky`, a night clock for daylight GoTo tests. `--covers-on` confirms the mirror cover and finder cap are on. |
| `--no-mcp` | MCP on | Turns the `/mcp` endpoint off. |
| `--allow-host NAME` | none | A DNS name allowed in the `Host` header for writes and `/mcp`, besides IP addresses and `localhost`. Repeatable. |
| `--licenses` | | Prints the third-party notices for the embedded page and data. |

## A pattern database for your finder

The default database covers horizontal fields of 10–30°. The budget rig's 13° finder and the
planned 17° finder both fit. For a narrower lens, build one that brackets your field, and
pass it with `--db`:

```sh
cargo run --release -p database-generation -- tyc_main.dat tyc_5_15.npz --min-fov 5 --max-fov 15
```

Fields under about 10° need the Tycho catalog, as here. [The solver](../solver.md#build-a-pattern-database)
explains catalogs and options. After pairing, Setup's **Camera field** and **Database range**
lines warn you if the live field is near or outside the database's range.

## Pair a scope node

Install the node first: [scope node manual](scope-node.md). Then, in the console:

1. Open **Setup**.
2. Under **Site**, type the latitude and longitude (east positive). A browser shares its location
   only over HTTPS or localhost, so on a phone over plain HTTP, type the site.
3. Under **Minimum altitude**, keep 15° or set your own horizon.
4. Under **Scope node and camera**, enter the node's address (`IP:50051`), its token, and a
   fixed exposure and gain. Press **Save scope node and camera**.
5. The section turns **Connected** and shows the star count of each frame. The chart on the main
   page goes live.

The link sends a heartbeat every 250 ms. If the console goes quiet for a second, the node stops
the mount. The node's imaging camera uses a second connection to the same node.

## Open it from a phone

```sh
./target/release/finder --listen 0.0.0.0:8080
```

Then open `http://<console-LAN-IP>:8080` on the phone. Every open page shares one session.

**Security.** Anyone on that network can then use the page, the API and `/mcp`. See
[network exposure](../safety-and-security.md#network-exposure).

**Under WSL2's default NAT networking,** the console's address is not the laptop's network
address. Either forward the port from Windows with `netsh interface portproxy` and a firewall
rule, or switch WSL to mirrored networking. The crate reference's section
[MCP under WSL2: portproxy or mirrored mode](../../crates/finder/README.md#mcp-under-wsl2-portproxy-or-mirrored-mode)
gives both recipes.

**If you reach the console by name** (for example a `.local` name), add that name with
`--allow-host`. Otherwise writes are refused.

## Imaging prerequisites

**On the scope node:** an INDI server and the camera's driver, and an `[imaging]` table in the
node's config ([scope node manual](scope-node.md#1-prepare-the-pi)).

**On the console machine: ASTAP.** The console calls ASTAP's command-line solver once for each
imaging solve.

1. Install `astap_cli` and the D50 star database: Ubuntu's `astap-cli` and `d50` packages, or
   ASTAP's own downloads.
2. The `d50` package installs to `/opt/astap`, while `astap_cli` looks in
   `/usr/share/astap/data/`. Pass the folder explicitly:

   ```sh
   ./target/release/finder --listen 0.0.0.0:8080 \
     --astap astap_cli --astap-db-name d50 --astap-db /opt/astap \
     --focal-length-mm 1800
   ```

3. Use your telescope's focal length for `--focal-length-mm`. 1800 mm is the 16-inch Dob's.

Without `--astap`:

- frames can still be taken;
- solving an imaging frame answers "this console has no imaging solver";
- the focus aid, the live stack and imaging runs are unavailable.

**Disk.** A sub is 2 bytes per pixel. A full ASI071MC Pro frame is about 32 MB, and 2 × 2
binning cuts that to a quarter. The Imaging page shows an estimate before a run starts.

- **During a run:** a reserve file of about 260 MB is held, so the stack can still be saved if
  the disk fills.
- **Calibration:** 20 full-frame darks take about 650 MB, plus about 65 MB for their master.
- **Nothing is deleted automatically**, except that a run with **Keep every sub** off removes
  its stacked subs once the stack is saved.

**Memory.** A full-frame ASI071 stack needs roughly 0.7 GB while stacking: about 455 MB for the
stack (with outlier rejection, the default) and about 260 MB for the sub being added. Saving
the stack needs more.

**Where the files go**, under the imaging folder:

| Path | What |
|---|---|
| `frames/<UTC date>/<id>.fits` | Single imaging frames, each with a `.solve.json` once solved |
| `runs/<stack id>/` | One imaging run: `stack.fits`, `stack.png`, `stack.json`, `run.json`, and `subs/` with every kept sub |
| `calibration/` | `index.json`, `darks/` (with bad-pixel maps) and `flats/` |
| `focus.json`, `relation.json` | The focus record, and the measured relation between the finder and imaging cameras |

Finder recordings (Setup › Recording) go to `recordings/<UTC>/` in the state folder.

## Connect an AI assistant

The console serves MCP (Streamable HTTP) at `/mcp` on the same port as the page. It is
stateless and takes no token. Setup's **MCP** section shows the address to copy.

```sh
claude mcp add --transport http finder http://localhost:8080/mcp
```

- **From another device,** use the console's network address. `/mcp` answers only callers on
  the same machine or a private network address, and needs `--listen 0.0.0.0:8080`.
- **Every moving tool passes the page's guards.** Moving tools are limited to two calls a
  second, and each call is logged to standard error as `mcp <tool> <args> -> <result>`.
- **`stop` and `stop_all` are always accepted.**

The 32 tools fall into three groups:

| Group | Tools |
|---|---|
| Mount (8) | `describe_axes`, `get_axis_state`, `goto_target`, `goto_axis_position`, `move_axis`, `set_tracking`, `stop`, `stop_all` |
| Session and pointing (8) | `get_session`, `set_session`, `set_target`, `search_targets`, `get_guidance`, `solve_frame`, `get_pointing`, `get_link` |
| Imaging (16) | `get_imaging_camera`, `set_imaging_camera`, `take_imaging_frame`, `solve_imaging_frame`, `get_focus_reading`, `set_focus_aid`, `list_calibration`, `take_darks`, `take_flats`, `set_stacking`, `get_stack`, `set_steering`, `start_imaging_run`, `stop_imaging_run`, `continue_imaging_run`, `get_imaging_run` |

The crate reference's [MCP clients](../../crates/finder/README.md#mcp-clients) section describes
each tool's arguments and answers.

## Running the console on the Pi (planned)

The Pi 5 plan runs `finder` on the telescope's Pi beside the node.

- **Nothing in the console changes.** `--db` and `--samples` point into a checkout by default,
  so give them explicitly. Set `--state` and `--imaging-dir` only to put the data on the NVMe
  drive.
- **Listen on `127.0.0.1:8080` and reach it over SSH:**

  ```sh
  ssh -N -L 8080:127.0.0.1:8080 <pi>
  ```

  Then open <http://localhost:8080> on the laptop, and point the MCP client at
  `http://localhost:8080/mcp`.
- **Cross-building the console for 64-bit ARM needs more than the node does:** an aarch64 C
  cross-compiler and musl headers, because one of its dependencies compiles C. The packaged
  build and deploy tooling is not in this repository yet.
