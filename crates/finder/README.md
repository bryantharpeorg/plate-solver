# finder: the finder console

A local web page for the plate solver. Give it a frame (one of cedar's sample frames, or
your own) and it shows where the frame points (RA/Dec, the constellation, the field of
view), the catalog stars it matched drawn on the frame as proof, the Messier objects in and
near the field, and objects like them that are up now.

It is also the electronic finder's console. Paired with a scope node, it solves the finder
camera's live stream, guides a push to a target, drives the mount for GoTo and tracking, runs
the imaging camera from focus to a stacked image, and offers the same actions to MCP clients.
A SkySafari link is not built yet. The [main README](../../README.md#status) says what has been
proven at the telescope.

## Run it

From the repository root:

```sh
cargo run --release -p finder
```

Then open <http://127.0.0.1:8080>. The defaults are cedar's 10–30° database and its 21
sample frames; for your own camera, pass a database built for its field of view (see the
main README):

```sh
cargo run --release -p finder -- --db my_camera.npz --samples ~/frames
```

| Flag | Default | Meaning |
|------|---------|---------|
| `--db PATH` | cedar's `default_database.npz` | Pattern database covering your camera's field of view. Repeat it to hold several (§ Several databases). |
| `--samples DIR` | cedar's `medium_fov` samples | Folder of frames to offer on the page. |
| `--listen ADDR` | `127.0.0.1:8080` | Address to serve on. `0.0.0.0:8080` lets other devices connect. |
| `--state DIR` | `$XDG_STATE_HOME/finder`, otherwise `~/.local/state/finder` | Folder holding the observing session in `session.json`. |
| `--imaging-dir DIR` | `imaging/` in the state folder | Folder for imaging frames. |
| `--simulate` | off | In-process simulator on a virtual clock, with an isolated session and pairing. |
| `--sim-start UTC` | now | Start the simulated night at `YYYY-MM-DDTHH:MM:SSZ`, optionally with fractional seconds. Requires `--simulate`. |
| `--mount-sky` | off | Draw each frame's stars from the real mount's encoders instead of the camera, for testing GoTo without a sky. Needs `--tube-alt`; not with `--simulate`. Do not push the tube by hand in this mode. |
| `--tube-alt DEG` | | The tube's altitude when the console starts, from a level, 0 to 90. Requires `--mount-sky`. |
| `--mount-sky-night UTC` | | Run the console on a night clock from `YYYY-MM-DDTHH:MM:SSZ`, so a daylight test can move the mount. The Sun guard then checks that clock's Sun. Requires `--mount-sky` and `--covers-on`. |
| `--covers-on` | off | Confirm that the mirror's cover and the finder's cap are on. Requires `--mount-sky-night`. |
| `--allow-host NAME` | none | DNS name permitted for same-origin writes, in addition to IP literals and `localhost`. Repeat for multiple names. |
| `--licenses` | | Print the third-party notices for the embedded page and data, and exit. |

### From a phone

The page is built for a phone at the telescope. Two ways to use one:

- **The Surface serves, the phone browses.** Run with `--listen 0.0.0.0:8080` and open
  `http://<the Surface's address>:8080` on the phone, on the same Wi-Fi. Under WSL2 the
  server is only reachable from Windows itself until WSL uses mirrored networking: add
  `networkingMode=mirrored` under `[wsl2]` in `%UserProfile%\.wslconfig`, then run
  `wsl --shutdown` and start it again. Windows may ask to allow the port through its
  firewall.
- **The phone does everything (Android).** In [Termux](https://termux.dev), install Rust
  and git (`pkg install rust git`), clone the repository, and run the same `cargo run`
  command; the first build takes a while on a phone. Then open
  <http://127.0.0.1:8080> in the phone's browser. The server needs about 160 MB of memory.

Location ("what is up now") is entered by hand on the page. Browsers only share GPS
location with pages served over HTTPS or from the same device, so "use this device's
location" works when the phone runs the server itself.

## MCP clients

The console serves Streamable HTTP MCP at `/mcp` on the same port as the page.
On the Surface, clients in WSL and Windows can use `http://localhost:8080/mcp`.
The default `--listen 127.0.0.1:8080` is reachable only from this machine. MCP
reaches the LAN only with `--listen 0.0.0.0:8080`; a phone then uses
`http://<surface-LAN-IP>:8080/mcp`. Setup's MCP section lists the URLs the console
serves, its allowed host names, and whether MCP is off or local only.

`--no-mcp` turns the endpoint off. `/mcp` accepts IP literals, `localhost`, and
the names given with `--allow-host`, just as the page's writes do. For example,
`--listen 0.0.0.0:8080 --allow-host surface.local` accepts
`http://surface.local:8080/mcp`. Names have no scheme or port, match exactly
ignoring ASCII case, and the flag may be repeated.

There is no token or console confirmation for MCP calls. Every moving tool runs
the page's guards: Sun guard, minimum altitude, cable wind and mount range checks,
plus the scope link's watchdog and tracking's safe endings. Direct axis moves
need a sync so the console can check their path on the sky. While an MCP client
moves the telescope, the guidance block and top bar say
“Moved by an MCP client · <tool>”. The top bar's Stop is the page's only Stop;
it stays enabled throughout motion and ends GoTo, direct axis moves and tracking.

The imaging camera has sixteen tools of its own, which answer what the page's
Imaging page does:

- `get_imaging_camera` reads the imaging view: the state, the camera's
  description, its status, the settings in force and the latest imaging frame;
  with a stack the view carries its `stack` and `steering` parts.
- `set_imaging_camera` takes the same body as `PUT /api/imaging/settings` and
  answers the view, or the route's refusal as a tool error.
- `take_imaging_frame` takes no arguments, does what
  `POST /api/imaging/frame` does, waits until the frame is saved and answers the
  view with the frame's colour JPEG preview as an image block; an aborted or
  failed frame answers a tool error with its words. On a console that takes no
  imaging frames, `set_imaging_camera` and `take_imaging_frame` answer
  `this console takes no imaging frames` at once.
- `solve_imaging_frame` takes no arguments, does what
  `POST /api/imaging/solve` does, waits until the solve ends and answers the
  imaging view with the solve; a refusal or a failed solve answers a tool error
  with the route's or the solve's own words. On a console that takes no imaging
  frames it answers `this console takes no imaging frames` at once.
- `get_focus_reading` takes no arguments and reads the focus reading: the
  focus region, the focus record, and the reading, best, trend and verdict of
  the focus aid or the last focus check. Without imaging, while the camera is
  not ready or without a solver it answers the error a start would be refused
  with.
- `set_focus_aid` takes `on`, and with `on` true the same keys as
  `POST /api/imaging/focus`; it starts the aid as the route does and answers
  the focus view at once, or, with `check` true, waits until the check has
  ended and answers its verdict. With `on` false it stops the aid as
  `DELETE /api/imaging/focus` does. A refusal answers a tool error with the
  route's words.
- `list_calibration` reads the calibration library's master darks and master flats, the set being taken and what matches the latest imaging frame, as `GET /api/imaging/calibration` does.
- `take_darks` takes the body of `POST /api/imaging/calibration/darks`, including `anyway`, and answers once the set has started; read its progress with `list_calibration`.
- `take_flats` takes the body of `POST /api/imaging/calibration/flats`, including `darks` and `anyway`, and answers once the step has started; read its progress with `list_calibration`.
- `set_stacking` takes `on` to start or stop the live stack as the page does, with `reject`, `focus_gate`, `gains` and `stretch` on a start, and `gains` and `stretch` on a running stack; it answers the stack's view or the route's refusal as a tool error.
- `get_stack` reads the stack's state, counts and dropped subs by reason, with its picture as a colour JPEG at most 1024 wide when it has one, or `{"state":"none"}` before any stack.
- `set_steering` takes any of `on`, `smear_px`, `step_share` and `rate_share`, as `PUT /api/imaging/steering` does, and answers the imaging view or the route's refusal as a tool error.
- `start_imaging_run` moves the telescope to the session's target and starts an imaging run as `POST /api/imaging/run` does, with optional `subs`, `duration_s` and `keep_subs`; it is rate-limited and logged, and answers at once in `slewing`.
- `stop_imaging_run` ends the run and saves its stack as `DELETE /api/imaging/run` does; it answers once the run has ended, and stops the mount only while the run still slews. Use `stop` to stop the mount at any time.
- `continue_imaging_run` takes `anyway`: false checks focus again from `focus_pause`, true goes on unchecked, as `POST /api/imaging/run/continue` does. An MCP client with nobody at the focuser should ask the observer or pass true.
- `get_imaging_run` reads the run's view, including its state, message, warnings, focus verdict, centring offset, sub counts and ending; it answers null before the first run. Read it every few seconds until `ended`; a `focus_pause` needs `continue_imaging_run` within 15 minutes.

Both taking tools answer a refusal as a tool error with the route's words. On a
console that takes no imaging frames they answer `this console takes no imaging
frames` at once, before reading their arguments; `list_calibration` answers
`{"masters":[]}` there.

The Tools page's finder alignment has two tools:

- `get_finder_alignment` takes no arguments and reads `{"align": ...}`: the
  finder alignment's state and its last imaging frame, with where the imaging
  camera's centre lies from the finder camera's centre, whether it is done and
  the aim point's word for that frame; `align` is null before the first start.
- `set_finder_alignment` takes `on`, and with `on` true optionally `exposure_ms`;
  it starts or stops the finder alignment as `POST` and `DELETE /api/imaging/align`
  do and answers `{"align": ...}`, or the route's refusal as a tool error. It moves
  nothing: the observer turns the ring screws, with Track off, and an MCP client
  says which way.

On a console that takes no imaging frames both answer `this console takes no
imaging frames` at once, before reading their arguments.

The Tools page's focus aid for the finder camera has two tools:

- `get_finder_focus` takes no arguments and reads `{"focus": ...}`: the finder
  camera's focus aid, its readings in the finder camera's pixels, the best so
  far, the trend and `found`; `focus` is null before the first start.
- `set_finder_focus` takes `on` and nothing else; it starts or stops the aid as
  `POST` and `DELETE /api/finder-camera/focus` do and answers `{"focus": ...}`,
  or the route's refusal as a tool error. It moves nothing and changes no
  setting: the observer turns the focuser.

### MCP under WSL2: portproxy or mirrored mode

Under WSL2's default NAT networking, the WSL address listed in Setup is not the
Surface's LAN address and the phone cannot reach it directly. Run the console
with `--listen 0.0.0.0:8080`, then choose either Windows portproxy or WSL mirrored
networking. Both carry the page and `/mcp` on the same port and preserve `Host`.

For NAT, find WSL's IPv4 address with `hostname -I` in WSL, then run in an
administrator PowerShell, replacing the two example addresses with WSL's current
IPv4 address and the Surface's LAN IPv4 address:

```powershell
netsh interface portproxy add v4tov4 listenaddress=192.168.1.20 listenport=8080 connectaddress=172.24.1.2 connectport=8080
New-NetFirewallRule -DisplayName "Finder console MCP" -Direction Inbound -Action Allow -Protocol TCP -LocalPort 8080 -Profile Private -RemoteAddress LocalSubnet
```

Scope the Windows firewall rule to the local subnet (`LocalSubnet`): portproxy
hides the peer address, so the console sees the proxy rather than the phone and
cannot enforce the original peer's LAN membership. Update portproxy if WSL's
address changes. On the phone, paste `http://192.168.1.20:8080/mcp` (using the
Surface's LAN address), or the allowed `surface.local` URL.

Alternatively, on Windows 11 22H2 or later, put this in Windows'
`%UserProfile%\.wslconfig`, then run `wsl --shutdown` and restart WSL:

```ini
[wsl2]
networkingMode=mirrored
```

Allow TCP 8080 through the Hyper-V firewall in administrator PowerShell, replacing
`192.168.1.0/24` with your LAN subnet:

```powershell
New-NetFirewallHyperVRule -Name "FinderConsoleMcp" -DisplayName "Finder console MCP" -Direction Inbound -VMCreatorId '{40E0AC32-46A5-438A-A0B2-2B479E8F2E90}' -Protocol TCP -LocalPorts 8080 -RemoteAddresses 192.168.1.0/24
```

Use the Surface's LAN address in the phone's `/mcp` URL. Windows localhost still
works for clients on the Surface. Microsoft's [WSL networking guide](https://learn.microsoft.com/en-us/windows/wsl/networking)
describes NAT forwarding and mirrored networking; its [Hyper-V firewall guide](https://learn.microsoft.com/en-us/powershell/module/netsecurity/new-netfirewallhypervrule)
describes the firewall rules.

## Night mode

Night mode turns the whole page, frames and sky survey included, into dim red, to keep your
eyes dark-adapted. It is a toggle in the top bar and the page remembers it. An imaging
frame's preview is the one exception: while you hold the Hold to see colour control under
it, the picture shows its own colours for up to 10 seconds, then goes red again.

## How it works

- `src/solve.rs` runs one frame as cedar does: color turned to gray with PIL's weights, stars
  detected with cedar-detect's settings (rows levelled, hot pixels rejected), a solve from the
  brightest 30. The conversion and the settings come from the library
  (`star_detection::gray`, `plate_solver::DetectParams::cedar()`), which `solve_image` uses
  too, so both give the same answer. A test checks every sample frame against cedar's own answers: 19 of the 20
  that solve land within 0.3″ of cedar, and the last within 4.2″. It then places the matched
  stars, the brightest unmatched catalog stars and the Messier objects on the frame, using the
  solver's own attitude and lens model; another test checks that every matched star's catalog
  position lands back on the star it was matched to.
- `src/constellation.rs` finds the constellation from the IAU boundaries, after precessing
  to their B1875 equinox. `src/catalog.rs` holds the Messier list and ranks "something like
  this" by kind, then by whether it is up for your site, then by how far you would slew.
- `src/api.rs` serves the JSON API and the page. One solve runs at a time, and a request
  waits for its turn before its upload is read, so queued uploads don't pile up in memory.
  Uploads are capped at 32 MB, must arrive within 30 seconds, and are decoded with size
  limits.
- The page is a React + TypeScript + Vite + Tailwind app in `web/`. Its production build,
  `web/dist/`, is committed and compiled into the binary, so `cargo` and CI never need
  Node.

The page and data compiled into the binary include third-party works (React, Tailwind CSS,
the Archivo font, the Messier list from OpenNGC, the constellation boundaries from Astropy),
each under its own license: see [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md), which
`finder --licenses` also prints, and [`data/README.md`](data/README.md) for how the data was
extracted.

## Change the page

```sh
cd crates/finder/web
npm install
npm run dev     # http://localhost:5173, hot reload; run `cargo run -p finder` alongside it
npm run build   # writes web/dist; commit it with your change
```

The page uses the API types in `web/src/api.ts`, which mirror the Rust structs in
`src/solve.rs` and `src/catalog.rs`. Change both together.

## Several databases

A pattern database covers one range of fields: cedar's covers 10–30°, and a camera behind
a guide scope sees about 2°, which needs a database built for it from a deeper catalog.
Give `--db` once for each database the console should hold, in the order it should try
them:

    finder --db narrow_2deg.npz --db default_database.npz

Every database is loaded when the console starts, and each is named by its file name
without `.npz`; two with the same name are refused. While a frame has no field estimate,
on the first frame and after three failed solves in a row, the console tries the
database in use and then each of the others in the order given, until one solves. It
then uses the database whose range holds the solved field: the one that solved if it
does, otherwise the first given that does, otherwise the one that solved. With an estimate it
solves only with the database in use. So when the lens changes, the next three frames
fail against the old field, the fourth solves without an estimate, and from the fifth the
console uses the database that fits the new field. An uploaded frame is tried the same
way and does not change the database in use.

`GET /api/session`'s `field` names the database in use, its range and why in `database`
(`only`; `first`, before anything has solved; `field`, the solved field lies in its
range; or `solved`, it solved a field that lies in no database's range), and lists every
database in `databases`; `db_min_fov_deg` and `db_max_fov_deg` are the range of the one
in use. A solve report names in `database` the database that solved the frame, or the
last one tried. Setup and the home page's last line show them, and MCP's `get_session`
and `solve_frame` answer the same. With one `--db` the console works as before.

The simulator of `--simulate` and the drawn sky of `--mount-sky` take their stars from the
first database given. Every database is held in memory: about 115 MB for cedar's, and
about 1.7 GB for a 2° database built from Tycho (an estimate).

## API

| Route | Answer |
|-------|--------|
| `GET /api/health` | The first database's properties, every database's name and properties in `databases`, and how many samples there are. |
| `GET /api/samples` | The sample frames: `[{name, label}]`. |
| `GET /api/samples/{name}` | A sample frame's file. `…/thumb` is a 320-pixel JPEG preview. |
| `POST /api/solve` | Multipart: an `image` file or a `sample` name, and optionally `fov`, `fov_max_error`, `distortion`, `bin`, `sigma`, `stars`, `timeout_ms` (the `solve_image` flags). Answers a `SolveReport`. |
| `GET /api/similar?id=M42&lat=&lon=&time=` | Objects like `id`, best first. With `lat`/`lon` (degrees, east positive) and optional Unix `time`, each has its altitude and whether it is up. |
| `GET /api/session` | One shared observing session: `{site, min_alt_deg, target, pointing, time}`. |
| `PUT /api/session` | JSON partial edit: `{site?, min_alt_deg?, target?}`. Answers the updated observing session. |
| `POST /api/track` | JSON `{"on":true}` or `{"on":false}`. Answers 200 with the session; on refusal, 409 with a JSON `error`. Track on while it follows a push by hand sends the telescope back to the target. Same-origin writes only. |
| `POST /api/record` | JSON `{"on":true}` or `{"on":false}`. Start or stop a recording; answers 200 with the session, 409 or 500 with a JSON `error`. Same-origin writes only. |
| `POST /api/record/raw` | JSON `{"frames":N}`, N from 1 to 10. While recording, save the next N star frames' 8-bit pixels as `raw-<id>.png`; progress is `recording.raw` in the session. Needs a scope node built with 025. Refused while the mount moves. Same-origin writes only. |
| `GET /api/imaging` | The imaging camera the paired scope node relays: `state`, one of `off` (this console takes no imaging frames), `no_link`, `no_service` (the node was built before 030), `no_camera`, `error` and `ready`; with `error`, the node's words as `error`; with `ready`, the camera's `camera` description, its `status`, and the `settings` in force. |
| `PUT /api/imaging/settings` | JSON partial edit of `exposure_ms`, `gain`, `offset`, `binning`, `roi` (a region or null for the whole sensor), `cooler_on`, `set_temp_c`, `forward_kib_per_s` (a whole number from 64 to 1048576 KiB/s, or null for no limit) and `save_at_node`. Answers 200 with the imaging view; 400 with a JSON `error` for a value the console or the camera refuses, 403 for a foreign origin, 415 for another content type, 409 while the state is not `ready` or an exposure is in progress, and 502 with `the scope node did not apply the settings: <message>` when the node's call fails otherwise. Same-origin writes only. |
| `POST /api/imaging/frame` | No body. Take one imaging frame with the settings' `exposure_ms` and the purpose `frame`, receiving the samples on the imaging connection while the mount keeps tracking, and save them untouched as a FITS file. Answers 200 at once with the imaging view, whose `frame` is the exposure just asked for, `exposing`; the frame moves on through `receiving` to `saved`, `failed` or `aborted`, each state read from `GET /api/imaging`. 409 with the state's words while the state is not `ready`, and 409 `an exposure is already in progress` while a frame is `exposing` or `receiving`. Same-origin writes only. |
| `POST /api/imaging/abort` | No body. Abort the exposure in progress through the node's `AbortExposure`, or stop a frame still being received, ending it `aborted`; answers 200 with the imaging view, and with nothing in progress answers 200 and changes nothing. A node that does not answer the abort within 5 s is given up on: the frame ends `aborted` and the call answers then. Same-origin writes only. |
| `GET /api/imaging/frames/{id}/preview.jpg` | The colour JPEG preview of the saved imaging frame with this id, `image/jpeg`, while it is among the last eight saved; 404 with `the console no longer holds that preview` otherwise, and 400 for an id that is not a number. A saved frame's view carries the route as `preview`; a frame whose preview cannot be made, such as a mosaic with no whole 2 × 2 cell, is saved with no `preview`. |
| `POST /api/imaging/solve` | No body. Solve the latest saved imaging frame with the imaging solver; answers 200 with the imaging view, whose `solve` goes from `solving` to `solved` or `failed`, or 409 with a JSON `error`. Same-origin writes only. |
| `POST /api/imaging/focus` | JSON `{}` or any of `check`, `find`, `tile` (`{col, row}`, each 0 to 2) and `exposure_ms`. Start the focus aid on the focus region, or with `check` a focus check that stops by itself with a verdict; answers 200 with the imaging view, whose `focus.aid` reports it, or 400 or 409 with a JSON `error`. Same-origin writes only. |
| `DELETE /api/imaging/focus` | No body. Stop the focus aid, put the imaging settings back and answer 200 with the imaging view. Same-origin writes only. |
| `POST /api/imaging/align` | JSON `{}` or `{"exposure_ms": N}`. Start the finder alignment; answers 200 with the imaging view, whose `align` reports it, or 400, 409 or 415 with a JSON `error`. Same-origin writes only. |
| `DELETE /api/imaging/align` | No body. Stop the finder alignment, put the imaging settings back and answer 200 with the imaging view. Same-origin writes only. |
| `GET /api/finder-camera/focus` | `{"focus": ...}`: the finder camera's focus aid, null before the first start. |
| `POST /api/finder-camera/focus` | JSON `{}`. Start the finder camera's focus aid; answers 200 with `{"focus": ...}`, or 400, 409 or 415 with a JSON `error`. Same-origin writes only. |
| `DELETE /api/finder-camera/focus` | No body. Stop it and answer 200 with `{"focus": ...}`. Same-origin writes only. |
| `GET /api/imaging/calibration` | The calibration library: `masters`, the `set` being taken, what would `match` the latest imaging frame, and `error` when the library's index is unreadable. |
| `DELETE /api/imaging/calibration/{id}` | Remove a master dark or master flat and its files, or stop or drop the set with that id. Answers 200 with the calibration object, 404 for an unknown id. Same-origin writes only. |
| `POST /api/imaging/calibration/darks` | JSON `{"count":N}`, N from 3 to 64. Cover the telescope first. Takes one offset frame and N darks at the settings in force, checks that they are dark, and adds their master dark and its bad-pixel map; progress is `set`. `"anyway":true` takes them whatever the check says. Same-origin writes only. |
| `POST /api/imaging/calibration/flats` | JSON `{"count":N,"label":"..."}` with a flat panel lit: finds the exposure and takes N flats. Then, with the telescope covered, `{"darks":true}`, with `"anyway":true` to pass the dark test: takes their darks and adds the master flat. Same-origin writes only. |
| `POST /api/imaging/stack` | JSON object, every key optional: `reject`, `focus_gate`, `gains` (three numbers from 0.1 to 10), `stretch` (1 to 1000). Start the live stack; on a running stack, change `gains` and `stretch`. Answers 200 with the imaging view, 400 or 409 with a JSON `error`. Same-origin writes only. |
| `DELETE /api/imaging/stack` | No body. Stop the live stack and save it; answers 200 with the imaging view once its files are written: `stack.json`, and with a sub stacked `stack.fits` and `stack.png`; `stack.save_error` when they could not be. On a node that has stopped answering, the abort of the sub in flight is given up after 5 s and the stop answers then. Same-origin writes only. |
| `GET /api/imaging/stack/preview.jpg` | The stack's latest picture, a colour JPEG at most 1920 wide; 404 before there is one. |
| `PUT /api/imaging/steering` | JSON with any of `on` (true or false), `smear_px` (0.1 to 20), `step_share` and `rate_share` (each 0 to 1). Whether the subs steer the tracker during a stack, how many pixels the field may turn at the corner of a sub in one exposure, and the shares of a tracking error and of a drift that one gap takes out. Answers 200 with the imaging view. Run-only: on, 6 px, 0.5 and 0.3 at every start. Same-origin writes only. |
| `POST /api/imaging/run` | JSON with optional `subs` (1 to 100000), `duration_s` (1 to 43200, default 7200) and `keep_subs` (default true). Start an imaging run on the session's target: GoTo, tracking, a first imaging frame, a focus check, centring on the imaging camera's sensor and stacking. Answers 200 with the imaging view, whose `run` says where it has got to; 400 or 409 with a JSON `error`. Same-origin writes only. |
| `DELETE /api/imaging/run` | No body. End the imaging run as `stopped` and save its stack; answers 200 with the imaging view once it has ended. It stops the mount only while the run still slews: `POST /api/stop` stops it at any time, and ends the run too. Same-origin writes only. |
| `POST /api/imaging/run/continue` | JSON `{"anyway":false}` or `{"anyway":true}`. From a focus pause, check focus again, or go on unchecked; 409 with a JSON `error` when no run is paused. Same-origin writes only. |
| `POST /api/goto` | No body. Start a GoTo to the session's target; 200 when started, 409 with a JSON `error` naming a refused move. Same-origin writes only. |
| `POST /api/stop` | No body. Always 200, ending tracking, clearing arming and commanding both axes to stop, from any origin, without taking the motion lock. Sends `StopAll` immediately, retrying in its own task with 200 ms per attempt and 50 ms between attempts for a bounded 2 s window, even if the request goes away. Waits for the round and answers exactly `{"stopped":true}` when the scope node answers OK or there is no scope link; otherwise answers `{"stopped":true,"acknowledged":false}`. This means the node did not answer and the mount may still be moving; press Stop again. The MCP tools `stop` and `stop_all` return the same reply. |
| `POST /api/model/second-sync` | No body. Start the second sync; 200 with `{"started":true}`, or 409 with a JSON `error` naming the refusal. Same-origin writes only. |
| `POST /api/model/reset` | JSON `{"what":"night"}` or `{"what":"fixed"}`. Reset that part of the pointing model and return the session. Same-origin writes only. |

All phones and laptops connected to the console read and edit the same observing session.
A fresh session has no site, target or latest pointing, and a minimum altitude of 15°.
`time` is the console's clock in Unix seconds. The binary uses the system clock except
under `--simulate`, whose virtual clock runs at the real rate.

Session edits, solves, GoTo, Track and model writes follow the same-origin rule. When
a request carries an `Origin`, its scheme, host and port must match the request's own
origin. Its `Host` must also be an IP literal, `localhost`, or an explicitly allowed DNS
name. A different origin or an unlisted DNS name answers 403 with a JSON `error` and
leaves the observing session unchanged. Requests without `Origin`, such as curl and Rust
clients, are allowed. `PUT /api/session` requires a JSON content type; `text/plain` and
other media types answer 415 with a JSON `error`.

Phones opening the console by LAN IP work without extra flags. To open it by a local DNS
name, run, for example:

```sh
cargo run --release -p finder --locked -- --listen 0.0.0.0:8097 --allow-host surface.local
```

Then open `http://surface.local:8097`. Pass names without a scheme or port; matching is
case insensitive and exact. Repeat `--allow-host` for each name you use.

For example, set the site and choose Orion's nebula:

```sh
curl -X PUT http://127.0.0.1:8080/api/session \
  -H 'Content-Type: application/json' \
  -d '{"site":{"lat_deg":40,"lon_deg":-75},"target":"M42"}'
```

The returned target is the full canonical catalog object (`NGC 1976`, Messier `M42`).
Latitude accepts ±90°, longitude ±180° (east positive), and `min_alt_deg` accepts 0–80°.
Omitted fields remain unchanged; `{"target":null}` clears the target and `{"site":null}`
clears the site. A malformed edit, an unknown field (including `sun_guard`) or an invalid
range answers 400 with `{"error":"..."}`. An unknown target id answers 404 with the same
error shape. Each rejected edit leaves the entire observing session unchanged.

Track on requires a connected mount, a sync tonight, a set site, the Sun down and
a pointable target; a 409 names the missing condition. While a GoTo is running,
Track is armed and starts tracking when it completes. Otherwise, a target more
than 1′ from the aim point starts a GoTo first. A failed or halted GoTo ends the
armed request and retains its reason. Track off stops tracking and both axes,
leaving the telescope where it is. `POST /api/stop` also ends tracking as `stopped`
and clears arming, so the next rate update cannot restart the axes.

While Track is tracking, a live pointing more than 5′ from the place Track holds is
a push by hand: it is not corrected, and the pointing model's next sync is marked
as a slip. Once the pointings have stayed within 1′ of one another for 5 s, Track
follows the place the telescope was left at and holds it in place of the target.
A pointing put back within 5′ ends the moving state and is corrected as before.
The check is armed only by a pointing within 5′ of the target, so a keyhole exit
or a moved aim point is not read as a push, and it does not run while a backlash
is being measured.

Track on while Track follows a push by hand or the telescope is still moving by
hand sends the telescope back: the tracker ends, the push clears, both axes are
waited to rest, and the request starts a GoTo to the target and tracks it. With
no push, Track on for the target already being tracked stays the no-op it always
was. Track on for another target while Track is on goes the same way: the old
tracker ends and the request starts a GoTo to the new target. The page offers a
Recenter button while a push is shown and the session's target is the tracked
one.

After the first accepted Track-on request, session reads include
`track: {wanted, state, target, ended_reason?, readout?, push?}`. `state` is `armed`,
`tracking`, `keyhole_wait` or `off`; `target` is the canonical object being tracked.
`ended_reason` holds the last tracker or GoTo ending reason. While tracking, the
readout reports `rms_arcsec` over the last 60 s of live pointing errors and
`field_rotation_deg_per_min` at the target; while Track follows a push by hand,
the readout's field rotation and longest exposure are computed at the place
followed. While the telescope is pushed by hand, session reads carry `track.push`:
`{"state":"moving"}` while it moves, or
`{"state":"following","ra_deg":…,"dec_deg":…,"at":…}` once Track holds the place
followed; the key is omitted once the push ends with Track. Uploaded frames never
drive tracking.
Track state is never persisted: every binary start has tracking off and omits
`track` until Track is turned on again.

Set the imaging camera through a checked session edit:

```json
{"imaging_camera":{"width_mm":23.5,"height_mm":15.6,"pitch_um":3.76}}
```

Sensor width and height must each be more than 0 and at most 100 mm; pixel pitch
must be more than 0 and at most 50 µm. Out-of-range or malformed edits answer 400
with a JSON `error` and change nothing. The camera is retained exactly across
restarts. `{"imaging_camera":null}` clears it; session bodies omit the field
until set and after clearing. With a camera set, the tracking readout adds
`longest_exposure_s`: seconds for 1 px of field-rotation smear at the sensor's
corner, or `"no limit"` when the rotation is below 1e-9 rad/s in magnitude.

Every frame that solves becomes the latest pointing. `pointing` holds its `seq`, solved
`frame`, J2000 `ra_deg` and `dec_deg`, and `time` in Unix seconds from the console's clock
when the uploaded or sample frame was solved. With a site set, it also holds `alt_deg` and
`az_deg` at that frame time. Without a site, those two fields are absent. Setting or
changing the site recomputes them at the original frame time; clearing the site removes
them and keeps the RA, Dec, time, frame and sequence. A frame that does not solve leaves
the entire latest pointing unchanged. Successful API solve reports carry an optional
`pointing_seq` equal to the session pointing's `seq`.

The binary loads and saves the site, minimum altitude and canonical target id across
restarts. `--state DIR` overrides the default folder. Empty or unset `XDG_STATE_HOME`
falls back to `~/.local/state/finder`. Each write syncs `session.json.tmp` before renaming
it to `session.json`. An unreadable or invalid state file is reported on stderr and kept
as `session.json.bad` beside a fresh file. Time and latest pointing are not persisted.

Library callers get an in-memory session by default, without reading or writing a state
folder. To persist explicitly, pass `Session::with_state_file(path)` to
`AppState::with_session`. Tests and simulations can pass an `Arc<dyn Clock>` to
`AppState::with_clock`; `FixedClock` provides repeatable Unix seconds, and all
time-dependent API defaults use that clock.
Use `AppState::with_allowed_hosts` to configure DNS names for a library server's writes.

The scope link pairs the console with one scope node. Set its host and port and its
write-only bearer token through the same session edit:

```sh
curl -X PUT http://127.0.0.1:8080/api/session \
  -H 'Content-Type: application/json' \
  -d '{"node":{"address":"192.168.1.40:50051","token":"test-token"},"camera_settings":{"exposure_ms":200,"gain":4,"sigma":5}}'
```

`node` accepts partial `address` and `token` edits; `"node":null` clears both. Addresses
are IP literals (IPv6 in brackets) or DNS names with a port from 1 to 65535. Tokens must
be non-empty bearer tokens. Every Describe, Configure, Frames and Heartbeat call uses
`authorization: Bearer`. Reads and edit answers return only `token_set`, never the token.
The state file keeps the address, token and camera settings across restarts; each new
temporary file is created mode 0600 and atomically renamed over the old state file.

`camera_settings` accepts partial `exposure_ms`, `gain` and `sigma` edits. Exposure and
gain are `"auto"` or positive numbers; sigma is 1–50. Before any setting is supplied,
exposure and gain are automatic and detection uses cedar's 8σ threshold. A connected
camera refuses numeric values outside its reported ranges with 400 and a JSON `error`.
Settings are sent on each connection and whenever they change. If a new camera cannot
accept a stored value, that control runs automatically while its stored value is kept.

Once pairing is set, `GET /api/session` includes `node` with `address`, `token_set`,
`connected`, `last_frame_age_s`, `offset_s`, `camera`, `stars` and `out_of_range`.
`camera` reports `model`, `width`, `height`, and inclusive `{min,max}` ranges for
`exposure_ms` and `gain`. Unknown telemetry is null. Each `out_of_range` item names
`setting`, `stored`, `min` and `max`. `camera_settings` is omitted until a control is set,
and `node` is omitted while there are no pairing credentials and no connection.

A heartbeat runs every 250 ms. The link disconnects after one second without an answer,
or when its frame stream ends, and retries after 0.5, 1, 2, 4 and then 5 seconds. The
clock offset is node time less console time, from the least-delay heartbeat exchange in
the last eight seconds. The newest star frame is retained without a queue.

Library servers opt into the link with `AppState::with_link(Transport::Tcp)` after
setting their session and clock. `Transport::connector` injects a reusable stream
connector for in-memory tests through the same client. `AppState::link` exposes health
and newest-frame subscriptions. Without the builder, a configured node reads disconnected.

The binary solves the newest star frame whenever free, dropping intervening frames.
The last solved field estimates the next (5% allowance); three consecutive failures
clear the estimate. `pointing.source` is `"live"` or `"upload"`; live pointings add
`frame_id`. Live times are mid-exposure mapped through the link offset, including for
altitude and azimuth. After five seconds, `guidance.stale: true` replaces turn and tilt,
connected or disconnected. Uploaded pointings never expire.

Calibrate by tapping a star centred in the eyepiece:

```sh
curl -X POST http://127.0.0.1:8080/api/aim \
  -H 'Content-Type: application/json' \
  -d '{"frame_id":42,"x":706,"y":420}'
curl -X DELETE http://127.0.0.1:8080/api/aim
```

Both routes enforce the same-origin check and return the session. A tap snaps to the
nearest centroid within 10 px of its preview frame. A miss answers 422 with
`{"error":"no star is close enough to the tap"}`; an expired frame answers 409 with
an `error`. The last eight preview frames are held. The snapped `aim: {x,y}` survives
restarts until reset. Without an aim, reads omit it and pointings use the frame centre.
Unprojection includes the solved camera, distortion and attitude.

`PUT /api/session` also takes `chart_rotation_deg`, in degrees counter-clockwise,
stored within [0, 360); `null` clears it. Reads omit it until set. The page turns its
live chart and photos by it. The MCP tools do not change it.

After the first star frame, reads include `field`: `fov_deg` (null until solved),
`db_min_fov_deg`, `db_max_fov_deg`, and `unsolved_s`, the console seconds of frames
holding at least ten stars without a solve. Any live solve resets that interval.
Library servers call `with_live()` after `with_link`; `with_live_solver` injects a
`live::Solver`. `AppState::live()` exposes reports and previews. Uploads and live
solves share one gate.

## Simulated night and live stream

Run a desk console without a telescope:

```sh
cargo run --release -p finder --locked -- --simulate --sim-start 2026-01-16T02:00:00Z
```

The simulator draws catalog stars through the real camera, detection, scope link and
console solver. It starts at altitude 45°, azimuth 180°, using the saved session's site
or 40° N, 75° W. Its optics are 1456 × 1088 pixels, a 12° horizontal field, and an
eyepiece centred on pixel (700, 420). The virtual clock advances at the real rate from
`--sim-start` (default now). Invalid dates and `--sim-start` without `--simulate` are
refused before any server binds.

Without `--state`, the simulated session is `simulate/session.json` under the default
state folder. The night's default `session.json` is never read or written. `--state DIR`
still uses exactly `DIR/session.json`. A private in-memory pairing connects only to the
in-process node on `127.0.0.1` at a random port with a random token; saved or edited node
addresses and tokens cannot divert it. Camera settings still reach the simulator.

Only simulate mode registers these controls, all behind the console's same-origin check:

| Route | Request / answer |
|-------|------------------|
| `GET /api/sim/telescope` | `{camera: {alt, az, ra, dec}, eyepiece: {alt, az, ra, dec}}`, true directions in degrees, RA/Dec in J2000. |
| `POST /api/sim/telescope` | Exactly one of `{alt_deg, az_deg}`, `{turn_deg, tilt_deg}`, or `{eyepiece_on_star: true}`; answers the true directions. |
| `POST /api/sim/frames` | `{running: false}` stops exposures; `{running: true}` resumes them. Answers `{running}`. |
| `GET /api/sim/focus`, `POST /api/sim/focus` | `{focus_error}`, from -100 to 100: the simulated focuser. 0 is best focus; the simulated imaging camera's stars grow with its size either way. Answers `{focus_error}`. |

Turn is clockwise positive; tilt is up positive. `eyepiece_on_star` centres the brightest
database star in the field at the current time. Stars continue to drift with the sky.
An exposure already in progress finishes before a push or stop takes effect. Outside
simulate mode every `/api/sim/*` path answers 404. Unknown request fields are refused.

The live solve takes the scope node's centroids as they come, with the saturated stars
cedar-detect sets aside among them ([scope node README](../scope-node/README.md), Bright
stars: saturated stars); a frame uploaded on the Solve page or through MCP is detected the
same way.

`GET /api/live` is `text/event-stream`. Each `live` event carries
`{frame_id, time, pointing_seq, preview_seq, stars, report, guidance?, up_deg?}`. Time is the
mid-exposure instant mapped to the console clock; `report` is the same `SolveReport`
as an upload, with `name: null`. Guidance is evaluated at that solve. On solved events,
`up_deg` is present when the session has a site: the zenith's bearing in the frame at the
pointing, clockwise from the frame's up, in degrees in [0, 360). Keep-alive comments
arrive every 15 seconds; subscribers lagging beyond 16 reports skip missed events.

`GET /api/live/preview.jpg?seq=N` serves the JPEG carried by exposure N, while it remains
in the last eight preview frames; an absent or expired preview answers 404. A console
without a live loop answers 404 for both live routes. Library servers opt into simulation
with `with_simulation(start_unix_s, SimulationTransport::Loopback).await?`, then
`with_live()`. Paused-time tests use `SimulationTransport::Memory` over duplex IO in
one runtime.

A GoTo requires a connected mount, a sync that night, a site and a pointable target.
It refuses a target below the horizon or minimum altitude, an approach that would
go below that minimum, the Sun up (centre at or above −0.83°), and any path that
cannot stay 15° clear of the Sun. The Sun guard is always on.

The planner uses the apparent place at arrival and the pointing model, keeps cable
wind within one revolution across zero-point epochs, and approaches both axes from
below by 0.5°. It tries alternate axis orders and a higher path for Sun clearance.
A telescope below the minimum must first lift without turning in azimuth.

After slewing at the mount's top speed, it waits for both axes to stop plus 0.5 s,
then uses a solved exposure starting after that wait as a sync. It corrects at most
five times to bring the target within 1′ of the calibrated aim point.
`GET /api/session` adds `goto: {state, correction, error_arcsec, reason}` only after
one starts, retaining its result until the next GoTo. States are `slewing`,
`solving` (including the settle wait), `correcting` (numbered 1–5), `done`,
`failed`, and `halted`.
The final error is in arcseconds. Stop reports the observer; heartbeat silence of
one second reports a lost link; sunrise reports the Sun guard. A change in the
scope node's watchdog stop count during a GoTo reports
`halted by the scope node's watchdog`. Every halt stops both axes and cancels
further motion. Stop remains available while planning, solving or another request
is pending.

After the first sync, the second-sync route chooses the nearest pointable spot
in turn and tilt at least 90° in azimuth from every sync of the night, allowing
for the initial model's uncertainty before the base tilt is fitted. It uses the
same Sun guard, cable-wind and approach checks as GoTo, waits for a settled solve
and takes the sync, with no correction slews. It leaves the selected target
unchanged and reports progress through the same `goto` session field. Stop also
halts a second sync. No suitable spot, missing sync, site or mount, a moving mount,
or an operation already running answers 409 with a JSON `error` before movement.

Resetting `night` clears the night's syncs, zero points and base tilt, retaining
the fixed terms. Resetting `fixed` clears those hardware terms, retaining the
night's model. Resets are persisted across restarts and command no motion.
Invalid reset bodies answer 400 with a JSON `error`; a missing JSON content type
answers 415. A foreign Origin on either model route answers 403 with a JSON
`error`, leaving both the model and axes unchanged.


## Drawn sky on the real mount

Run the console against the paired scope node with `--mount-sky --tube-alt 30` and
the tube's altitude from a level. The console then draws each frame's stars from the
mount's encoders instead of the camera: the first frame's mid-exposure counts read as
azimuth 180° and the altitude you passed, and every later frame carries the catalog
stars at the pointing its counts give, at its mid-exposure console time. Syncs, the
model, GoTo, tracking and the MCP tools run unchanged on those frames, and every motor
command goes to the real mount. Each frame's preview is dropped.

Without `--state`, the mode keeps its session in `mount-sky/session.json` under the
default state folder, and the node is paired there once; the night's real `session.json`
is never read or written. Every guard is unchanged: the Sun guard, the minimum
altitude, Stop and the watchdog behave as without the mode, and a GoTo still needs a
sync. `GET /api/session` reports `mount_sky` with `tube_alt_deg` and `zero_counts`.

Do not push the tube by hand in this mode. With the board's encoders off, a push moves
the tube and not the counts, so the drawn sky stops matching the tube and nothing
detects it. After a power cycle (the node's reset count changes) the drawn sky stops
for good, and `mount_sky` carries `power_cycled: true`: restart the console with a
fresh `--tube-alt` and reset the night then. Check the tube is level before long moves.

To run the same test in daylight, with the mirror's cover and the finder's cap on,
start with `--mount-sky-night UTC --covers-on`: the console then runs on a night
clock from that UTC and prints `night clock from <UTC>: the Sun guard checks a
pretend Sun; keep the mirror cover and the finder cap on`. The Sun guard checks
that clock's Sun, so a GoTo works in daylight, and every other guard is unchanged.
`mount_sky` carries `night_utc` then. The covers are the observer's own act:
`--covers-on` only confirms it, and the warning repeats it.

What the drawn sky cannot test: slip, gear backlash after the encoder, the base's tilt,
and the 2 to 3% shortfall seen on the sky in the real board's 1° moves.


## Recording a night

Send `POST /api/record` with `{"on":true}` to save the star frames the console
receives, every live solve of those frames, and their previews. Send `{"on":false}`
to stop. A repeated start keeps the recording already on; the next start after a
stop makes a new folder. The route follows the same-origin rule; invalid JSON
bodies answer 400 and a body without a JSON content type answers 415, each with a
JSON `error` and no change.

Each recording lives in `recordings/<UTC start>/` in the state folder (by default
`simulate/recordings/` under `--simulate`). Names are `YYYY-MM-DDTHH-MM-SSZ`, with
`-2`, `-3` and so on for starts in an already used second. The files are:

- `recording.json`: format `finder-recording/1`, the finder version, console start
  time as Unix seconds and UTC, and the session's site, aim point, camera settings,
  pointing model and camera. At the end it gains the stop time, reason and counts.
- `frames.jsonl`: one JSON line for each star frame read after the start, holding
  `id`, `start_s`, `end_s`, `width`, `height`, `exposure_ms`, `gain`, `settings`,
  `centroids` as ordered `[x, y, brightness]` triples, `axis_start`, `axis_end`,
  console `received_at` and the link's `offset_s` (node time less console time).
  Settings hold nullable `exposure_ms`, `gain` and `detection`; detection holds
  `sigma`, `binning`, `normalize_rows` and `detect_hot_pixels`. Each axis position
  holds `azimuth` and `altitude`. Absent settings, axes or offset are null.
  A preview-bearing line also has `preview`, naming its JPEG file. Frames replaced
  on the newest-frame channel before the recorder reads them appear as gaps in ids.
- `solves.jsonl`: the same JSON as the live events, only for recorded star frames.
- `preview-<id>.jpg`: the scope node's preview bytes unchanged, written before the
  corresponding frame line.

A recording ends at 3,600 star frames. Its `ended_reason` is `stopped`, `limit`,
`write_failed` or `node_restarted`. An id that does not rise, as after a scope node
restart or pairing with another node, ends it with `node_restarted` before writing
that star frame or its preview. Start a new recording afterwards. A failed write
ends it without making a deleted folder again. No lines follow a stop's answer.
The session's `recording` holds the folder, start time, counts and end reason from
the first start until the console exits; it is run-only and never saved in
`session.json`. Every console start begins without a recording.

While recording, `POST /api/record/raw` with `{"frames":N}`, a whole number from
1 to 10, saves the next N star frames' raw frames as lossless 8-bit gray PNGs,
`raw-<id>.png`, beside their frame lines. Each PNG is written under a `.part` name
and renamed when complete. Rebuild the scope node with 025 before using this route.
The session's `recording.raw` holds `wanted`, `saved` and an optional `error`:
`this scope node cannot send raw frames`, `no raw frame arrived within 15 s`,
`the raw frame request failed` or `the mount started moving`. A request ends at its
first failure, or with its recording, retaining its progress without adding an
error when the recording ends. Only one raw-frame request runs at a time.
Raw frames are refused with `wait until the mount is still` while an axis move,
GoTo or Track runs, or the mount reports movement. Once a move starts, a running
request stops before its next raw frame with `the mount started moving`; one
already on its way can finish. A raw reply shares the scope link's connection
with the heartbeat: on slow Wi-Fi a transfer can delay the heartbeat past one
second, and the scope node then stops the mount.

A recording holds the site, times and solves, never the node's address or token.
Review one before committing it to this public repository (owner decision 9).
Tests can replay frame lines through `Live::start` with their saved clock offset
and receipt time, as `crates/finder/tests/console/live/loop_.rs` feeds frames.

## Imaging frames

The console follows the imaging camera the paired scope node relays (030) on a
second connection of its own, for the imaging service alone, so imaging bytes
never share the heartbeat's connection. The connection is opened from the link's
own pairing, which under `--simulate` is private; no heartbeat, star frame, mount
call or raw frame ever crosses it.

`GET /api/imaging` answers the imaging view: `state`, and with `ready` the
camera's description, its status and the settings in force. While the link is
connected the console describes the camera, reads the settings in force and its
status every 2 s; in the states `no_service`, `no_camera` and `error` it asks again
every 5 s, and no sooner because a heartbeat was answered. When the link drops the
state is `no_link` within 3 s and the camera's settings are read again on return:
the camera keeps them, and so do the console's own three.

The console keeps three settings of its own, for one run of the console and never
in `session.json`: `exposure_ms` (1000 ms until set, kept within the camera's
range), `forward_kib_per_s` (a whole number from 64 to 1048576 KiB a second, sent
with each exposure as the forwarding limit, or no limit) and `save_at_node`. Every
other setting is the camera's, changed through `PUT /api/imaging/settings` and
read back from the camera: an edit sends only the fields it names, `roi: null`
asks for the whole sensor, and the node's answer is what the view shows.

Frames are saved under the imaging folder, `imaging/` in the state folder or the
folder `--imaging-dir` names. `POST /api/imaging/frame` takes one: the node
exposes the camera for the settings' `exposure_ms`, the console receives the
samples on the imaging connection while the mount keeps tracking, and the frame
is saved untouched. The view's `frame` follows it: `id`, `state` (one of
`exposing`, `receiving`, `saved`, `failed` and `aborted`), `purpose`,
`exposure_ms`, `requested_at` and `roi`; with the cooler on, `set_temp_c`, the
temperature the file's `SET-TEMP` card is written from; from the frame's header
on, `width`, `height`, `bits`, `bayer`, `gain`, `offset`, `binning`, `temp_c`,
`start`, `date_obs`, `bytes` and `received`, and with `save_at_node` the node's
own copy as `node_file`, or the reason there is none as `node_save_error`; once
the last byte is in hand, `received_at`, and when saved, `saved_at` and `file`.
`roi` starts as the region the console's settings held and is replaced by the
header's when the header carries one, so a region moved by another client is
stamped as it really was.

While a frame is `exposing` or `receiving`, a second `POST /api/imaging/frame`
answers 409 `an exposure is already in progress` and a settings change 409
`an exposure is in progress`, and `POST /api/imaging/abort` ends it: the node's
exposure is stopped through `AbortExposure`, or a frame still being received is
stopped at the console, and the frame ends `aborted`, no file written for it.
A node that does not answer `AbortExposure` within 5 s, as over a link that has gone
quiet, is given up on: the console ends the frame `aborted` itself and drops the
call. The node's exposure may then run on to its end, and a frame asked for before
that fails with `the exposure failed: an exposure is already in progress`.
An abort with nothing in progress changes nothing. The view's `status.exposing`
is the console's own: true while its latest frame is `exposing`, false
otherwise. With `save_at_node` the node also keeps a copy of each frame under
its `[imaging] save_dir` folder, written before the frame's header leaves it,
so a frame lost on the link is not lost there.

Each whole frame is saved as `frames/<UTC date of the exposure's
start>/<id, six digits>.fits` under the imaging folder, written under a
`.part` name and renamed. The id is the console's own count, which starts after
the largest id in the date's folder and rises with every frame asked for, saved
or not. An imaging frame is never rewritten or replaced: when the name is
already taken the frame ends `failed` with
`could not save the imaging frame: <file> already exists` and the file there is
left as it is.

The file is one FITS image with the samples untouched, 16-bit samples stored
with `BZERO` 32768 and 8-bit samples as they are. Its cards, in order:
`DATE-OBS` (the exposure's start as UTC to the millisecond), `EXPTIME`, `GAIN`,
`OFFSET`, `XBINNING` and `YBINNING`, `XORGSUBF` and `YORGSUBF` (the frame's
region's origin in binned pixels), `CCD-TEMP`, `SET-TEMP` (when the cooler was
on with a set temperature), `BAYERPAT`, `INSTRUME`, `SERIALNO` (when the camera
has a serial), `IMAGETYP` (`LIGHT`, `DARK` for the purpose `dark` or `FLAT` for
`flat`), `ROWORDER`, `FRAMEID` and `PURPOSE`. `GAIN`, `OFFSET`, `CCD-TEMP` and
`BAYERPAT` are left out when the frame has none.

The time a frame took to reach the console is read from the view:
`received_at − start − exposure_ms/1000`, the time from the end of the exposure
to the last byte in hand, which is what the forwarding limit paces; that less
the scope node's own probe figure for the link is the link alone.

Each saved frame is also given a small colour preview, in memory only and kept
for the last eight saved frames, its route shown as the frame's `preview`. It
is made from the frame's samples as they were saved: each whole 2 × 2 cell of
the camera's mosaic becomes one pixel, red and blue their samples and green the
mean of its two, a mono frame one pixel a sample in all three channels; the
picture is then box-averaged by a whole factor until its longer side is at most
1280 pixels; and each channel is stretched from its median over the largest of
its 99.9th percentile's distance and thirty times its noise, mapped by
`255·asinh(10·t)/asinh(10)` with `t` the value's place in that span, so faint
stars show. The result is encoded as a JPEG of quality 85 and served by
`GET /api/imaging/frames/<id>/preview.jpg`, `image/jpeg`, while the frame is
among the last eight saved; a preview that cannot be made — a frame with no
pixels, such as a mosaic with no whole 2 × 2 cell — does not fail the frame,
whose view then carries no `preview`.

**One holder.** A part of the console that takes imaging frames one after
another holds the imaging camera while it runs. Meanwhile `PUT
/api/imaging/settings` and `POST /api/imaging/frame` answer 409, and
`set_imaging_camera` and `take_imaging_frame` a tool error, with `the imaging
camera is held by` and the holder's name. `GET /api/imaging`, the previews,
`POST /api/imaging/abort` and `get_imaging_camera` stay open to anyone, so an
exposure can always be stopped. A single imaging frame from the page or an MCP
client holds no lease: while it is in flight a second one is refused with `an
exposure is already in progress`, as before.


## Solving imaging frames

An imaging solve asks an imaging solver where a saved imaging frame points: the
place of its centre, the angle of its up axis east of north, its scale in
arcseconds a pixel, whether it is mirrored, and the number of stars matched. The
imaging solver is ASTAP's command-line program `astap_cli`, a free program
(MPL-2.0) the owner installs, together with one of its star databases; the
databases are built from ESA's Gaia mission, whose data asks for the credit
"ESA/Gaia/DPAC". The three flags name it: `--astap PROGRAM` is its program,
`--astap-db-name NAME` its database (required with `--astap`) and
`--astap-db FOLDER` the database's folder when ASTAP does not find it by itself.
`--astap` cannot be combined with `--simulate`; a desk console solves with the
simulated solver adapter, which answers from the simulator's truth instead.

`POST /api/imaging/solve` solves the latest saved imaging frame. It answers 200
at once with the imaging view, whose `solve` carries `frame_id`, `state`
(`solving`, then `solved` or `failed`), `requested_at`, `solver` and `hint`;
once the solve has ended, `took_ms` and `file`, and, solved, `ra_deg`,
`dec_deg`, `up_east_of_north_deg`, `arcsec_per_px`, `mirrored`, `cd_deg`,
`stars` and `warning`, or, failed, `reason`, `error` and `detail`. It answers
409 while the camera is not `ready`, when the console has no imaging solver,
while a solve is running, before any frame has been taken, while the latest
frame is still arriving, or when the latest frame was not saved. A failed solve
is never tried again by the console.

ASTAP is given a prepared copy of the frame, never the saved file itself: each
call makes a work folder `solver/` under the imaging folder, writes the copy
into it and removes the folder afterwards; leftovers from a console that exited
mid-solve may be deleted by hand. The solve's answer is written beside the
frame as a solve sidecar, `frames/<UTC date>/<id>.solve.json`, written whole
under a `.part` name and renamed over any earlier one, with `format`, `frame`,
`solver`, `requested_at`, `hint`, `solved`, `took_ms` and the solve's numbers or
the failure's `reason`, `error` and `detail`. The imaging frame's file is never
touched.

The five reasons a solve fails, with the message the page shows for each:
`not_solved`, `the imaging solver found no solution`; `too_few_stars`, `the
imaging solver found too few stars in the imaging frame`; `no_database`, `the
imaging solver has no star database it can read`; `timed_out`, `the imaging
solver gave no answer within <N> s`; and `failed`, with the words of the fault
itself. Each solve is limited in time: 120 s for a solve whose hint names a
place, 600 s for a search of the whole sky — a search of the whole sky may run
out its 600 s without an answer — and 30 s for a star measure.

A solve is told where to look by a hint, which names its source. The sources,
in the order they are taken: `pointing`, the session's latest live pointing
when its time is no more than 30 s before the exposure's start or after its
end; `last_solve`, the centre of the last imaging frame solved since the
console started, which stops counting once the session's target changes;
`target`, the session's target; and `none`, a search of the whole sky with no
place. A solve takes the first source that exists, and its radius is 3° for
`pointing`, `last_solve` and `target`, and 180° for `none`. A failed solve is
not asked again: after three misses in a row — a miss is a solve that ended
`not_solved` or `timed_out` — the next solve takes the next source that
exists, ending at the search of the whole sky, and a solved solve starts
again from the first.

The hint's field height is the frame's rows times its binning times a scale,
over 3600. The scale has up to three sources, in this order: the kept scale;
`206264.806 × pixel size in mm / focal length in mm`, when a focal length is
set; and none, which leaves the field height out for the solver to find by
itself. Three misses in a row move from one source to the next that exists,
and from the last back to the first, so a kept scale that has stopped being
true is passed over and tried again later, and nothing has to be deleted by
hand; a solved solve starts again from the first. After a change of optics
nothing needs to be done: the next solves turn through the sources by
themselves, or give `--focal-length-mm` its new figure when the frame is under
0.3° high, the smallest height ASTAP searches for by itself.

Every solved imaging frame stores its scale, per sensor pixel, with its frame
id and time, in `relation.json` in the imaging folder, replacing the one
kept, and the imaging view carries it as `relation.scale`. The file names the
camera's model, serial, size and pixel size; a file made for another camera is
ignored. `--focal-length-mm FOCAL_LENGTH_MM` sets the focal length in
millimetres, shown as `relation.focal_length_mm`, and must be a number above
0; it sizes a solve's field until a scale is measured.

### The measured aim point

The console measures where the imaging camera's centre falls in the finder
camera's frame whenever it solves a whole-sensor imaging frame: it fits a
straight line, by least squares in each axis, through the finder camera's live
solves from 10 s before the exposure to 10 s after it, and reads the line at the
middle of the exposure; the imaging angle is the imaging frame's up east of
north less the finder frame's up seen there. A later spec that solves frames of
its own may offer each solve the same reading through `Imaging::note_solve`.

Three things take no aim reading instead, and the imaging view names which of
the six outcomes happened last, as `relation.last`: `no_live_pointing`, when no
live solve falls inside the exposure or fewer than three fall in its reach;
`moving`, when the telescope travels more than 60″ during the exposure beyond
the line's own error, or when the fixes scatter more than 90″ about the line;
and `off_frame`, when the imaging centre falls outside the finder camera's
frame.

The readings are smoothed (FR-024 to FR-027): the console holds the last five
aim readings and answers with the medians of their x, their y and their imaging
angle taken around the circle. With three or more held, a reading more than
120″ on the sky from the median aim point, more than 1° from the median angle
or of the other mirror sense is set aside and moves nothing, shown as
`relation.set_aside`; a reading that is none of those clears the ones set
aside. The third set aside in a row replaces what is held with the three, which
is how a camera turned in the focuser takes over without a control. The
session's aim point is set from the medians only when none is in force, when
the one in force was tapped, or when the medians have moved more than 10″ from
it: a tap or a reset is obeyed, and replaced at the next aim reading. An aim
reading made in a finder frame of another size drops everything held, since
its pixels mean something else.

While Track is on, a newly measured aim point is held back instead of applied,
shown as `relation.held`, and applied at the first live solve after Track is
off, so the tracker never chases it; a restart forgets a held one.

Once the session's aim point is the measured one and a kept scale sizes the
solve's field, a hint from a live pointing is narrowed to the sensor's diagonal
on the sky — the camera's whole width and height, whatever region the frame
covers, since the hint's place is the sensor's centre and a frame of a region
may lie anywhere on the sensor — never under 0.2° or over 3°. The live pointing
the hint uses is the finder camera's live solve nearest the exposure's own
middle.

`relation.json` in the imaging folder keeps the kept scale with the aim
readings held and those set aside, each with the size of the finder frame it
was made in; they are read at start, and dropped when the camera differs. The
aim point itself is the session's, in `session.json`, marked `aim_measured`
while it was measured; a session whose aim point is not the measured one drops
the readings at its first solve.


## Focus

A focus reading is the size of the stars in an imaging frame, measured by the
same imaging solver that solves the frames: the median half-flux diameter, in
sensor pixels, of the stars in the region the console read. Once a scale is
known — an imaging solve's, or the focal length's — the console shows the
reading in arcseconds; until then it is in sensor pixels, which compare exactly
for one camera on one telescope. A frame with fewer than five stars says
nothing, and one whose stars are larger than 20 frame pixels is far out of
focus: the imaging solver's own limit, beyond which its star sizes stop being
trustworthy.

The sensor's nine tiles — a third each way — are the candidates for the focus
region, the part of the sensor the console reads while focusing: the optical
axis rarely lands in the middle of the sensor, and the stars are smallest
where it lands. `POST /api/imaging/focus` with `find`, or with no focus region
kept, takes one whole-sensor imaging frame, measures the nine tiles and keeps
the sharpest; `tile` (`{col, row}`, each 0 to 2) fixes one by hand, which needs
no whole-sensor frame. The region is kept in `focus.json` and reused by the
next start. Far out of focus, where no tile can be measured, the aid reads the
far-out tile with the smallest size for the time being and shows its rough
size, keeps no region, and looks for the sharpest tile again at its first
settled reading; sizes above the limit are rough, so such a tile is only a
place to read a number to turn the focuser by.

The focus record in `focus.json` is this camera's own history: tonight's
smallest settled reading, and the smallest of the earlier nights'. A night is
12 hours, counted from the night's first settled reading, so it needs no site
and no time zone. Only a settled reading of the open focus aid sets the
record: a check's own reading sets nothing, because the smallest of many
single frames is biased low, and a check that set the record would be judged
against itself the next time. Until the aid has passed through focus tonight,
the smaller of the two parts judges every reading; once it has, tonight's part
judges alone, because a pass through focus is the evidence that tonight's
smallest is what tonight's sky allows. The record is kept with the scale an
imaging solve measured, and a record made at a measured scale more than 5% from
the one in force is another telescope's and judges nothing; a record made
before any imaging solve carries no scale, so if the camera is moved to
another telescope before either was solved on, remove `focus.json` to start
again.

Every settled reading is judged against the record: `good` within 10% of it
(and any reading smaller than it), `slightly_soft` within 30%, `clearly_out`
beyond that, `unmeasurable` on a frame that measures nothing, and `no_record`
while this camera has none. No verdict says focus is good in absolute terms:
seeing, collimation, a mirror still cooling and tube currents enlarge stars
just as a focus error does, which is why the reading is judged against this
camera's own record and never against a fixed number.

The aid runs under the imaging camera's lease, held as `the focus aid` from
its start until the settings are back. It remembers the camera's exposure,
binning, region of interest and save setting, sets its own focus exposure and
the smallest binning and reads the focus region alone, one frame after
another; its imaging frames are saved like any others under the purpose label
`focus` and removed once the next is saved, but for the last one, and but for
one an imaging solve was running on. While it runs, an imaging frame or a
settings change asked for meanwhile is refused with `the focus aid` in the
words, and a start is refused the same way while something else holds the
camera. However it ends — `stopped`, by a stop; `done`, by a check; `failed`,
by a fault; or `no_region`, when no tile had five stars to measure — it puts
the settings back and releases the camera. A camera still stopping an aborted
exposure can refuse the settings for a moment, so a put-back the scope node
refuses while the camera is ready is tried again once a second for up to 5 s,
and a stop answers that much later. One that never takes is the aid's
`restore_error`, and the camera keeps the aid's settings until they are changed
by hand: `Use the whole sensor` on the Focus block, or `PUT /api/imaging/settings`
with `{"roi":null}`. Until then the region is named as the aid's, even after a
later check, and the next aid or check puts the whole sensor back rather than
keep it.

From the readings alone the aid works out what to tell a hand on the
focuser: the median of the last three readings, a reading more than 15% from
it as a turn of the focuser, the two frames set aside after each turn while
the telescope stops shaking, the trend of the last turn, the best of the last
600 settled readings, and `found` once the reading has been far from the
best, near it, far and near again. Turn the focuser one way until the reading
has passed its lowest and risen again, then come back to the lowest. The
focus check, `POST /api/imaging/focus` with `check`, is the aid that stops by
itself: it ends at its first settled reading with that reading's verdict, at
its first unmeasurable frame with that frame's, or after nine imaging frames
with `unmeasurable`, and notes nothing in the record.

All of this shows in the imaging view's `focus` object and on the Imaging
page's Focus block, and is kept in `focus.json` in the imaging folder: the
camera, the focus region and the focus record, removed only by hand. Track a
field of stars first, away from any bright nebula; a sparse field, such as one
far from the Milky Way, may leave a tile with fewer than five stars at the
default focus exposure, and the cure is a longer one (`exposure_ms`, the
page's Focus frame time) or focusing on a richer field nearby first and coming
back. `cadence_ms` is how long one reading takes, and a forwarding limit makes
it much longer, so clear the limit while focusing. Once focus is close, choose
`Find the sharpest tile again` if the region was found while it was still
well out of focus, and after a change of telescope or collimation.

## Finder alignment

The Tools page, opened from Setup's `Tools` link while a scope node is paired,
holds utilities used once a setup. Its first is the finder alignment: while the
observer turns the finder camera's ring screws, it shows where the imaging
camera's centre lies from the finder camera's centre, as the finder camera sees
it, also when that is off the finder camera's frame, and says Done once that
centre has held in the middle of the frame for two frames in a row. It moves
nothing.

`POST /api/imaging/align` starts it, with the body `{}` or `{"exposure_ms": N}`.
It holds the imaging camera's lease as `the finder alignment`, remembers the
exposure, the region of interest and the save setting, and takes one
whole-sensor imaging frame after another at 500 ms or `exposure_ms`, under the
purpose label `align`, without a node copy; the binning stays as it is, and
2 × 2 sends a quarter of the bytes. Each frame's file and solve sidecar go once
the next frame is saved, but for the last, and not while a solve from the
Imaging page reads it. Each frame is paired with the finder camera's live solve
nearest the middle of its exposure, no more than 5 s from it; with none, its
`outcome` is `no_live_solve` and it is not solved. Otherwise it is solved with
a hint that names that live solve's centre, a 10° radius and the field height
an imaging solve would be given; a frame that does not solve says so with the
solver's words, and the next frame looks again. `align` in the imaging view
carries `state` (`running` or `ended`), `since`, `exposure_ms`, `frames`, `last`
and, once ended, `ended` (`stopped` or `failed`) and `error`. A measured frame's
`last.centre` carries `x` and `y` in the finder camera's pixels, outside the
frame when the centre is, `right_deg` and `up_deg` along the frame's own rows
and columns (the chart rotation is not applied), `separation_deg` on the sky,
`on_frame` and `middle` (within a quarter of the frame's width and height of its
centre); a frame that did not solve carries `last.unsolved`, how many frames in
a row have not. `last.done` is true once this frame and the one before it each
put the centre in the middle. Each solve is offered to the measured aim point as
`Imaging::note_solve` offers any, and `last.aim` is what that made of it,
`relation.last.outcome`'s word: `measured` or `replaced` when the aim point took
this frame's reading, and `set_aside`, `moving`, `no_live_pointing` or
`off_frame` when it did not, in which case a later imaging solve measures it.
`DELETE /api/imaging/align` stops it, puts the three settings back and frees
the camera; it waits for a solve in flight, which ASTAP's limit holds to 120 s.
It is refused while the camera is not ready, without an imaging solver, while
it already runs, while Track is on, without a field height, when the finder
camera has had no live solve for 30 s, during an exposure no lease holds, for an
exposure outside the camera's range, and while another procedure holds the
camera, with that procedure's name. Track turned on while it runs ends it,
`failed`.

Turn Track off first, and back on after: a turned screw moves the pointing the
console reads, and Track would follow it or drive the mount back. Point the
telescope at a field of stars both cameras solve. Give the imaging camera's
field height first: `--focal-length-mm`, or a scale kept by an imaging solve on
the Imaging page. After a change of optics, solve imaging frames on the Imaging
page until one solves before aligning: the alignment's frames that do not solve
count no misses, so they never move the field height to its next source, while
a solve on the Imaging page during a run counts as that page's own.

## Focusing the finder camera

The Tools page's Focus block focuses either camera. For the imaging camera it
is the Imaging page's Focus block, the same one, under the imaging camera's
lease (see Focus). For the finder camera it is the finder camera's own focus
aid: `POST /api/finder-camera/focus` starts it, `DELETE` stops it, and `GET`
reads it.

Set a fixed exposure and gain for the finder camera on Setup first. The aid
refuses to start while either is automatic, and a run ends `failed` when the
frames it measures change exposure or gain: the scope node's automatic
exposure steps whenever the star count leaves 25 to 40, which turning the
focuser does, and the star count beside the reading means something only at
one exposure. After changing either, start the aid again.

It asks the scope node for one raw frame after another (the raw frames of
recordings, one star frame's pixels each, `width × height` bytes on the scope
link) and measures each at the stars the node found in that frame: the median
half-flux diameter, in the finder camera's pixels, of at most 15 of the
brightest stars whose box of 33 pixels lies inside the frame and holds no
other star, and whose own pixels are all under 250. Each star's light is
summed in circles round its centre a fifth of a pixel at a time, so the
reading follows a star smaller than two pixels and does not jump as a star
moves across the pixel grid. A frame with fewer than five such stars says
nothing (`too_few_stars`), and one whose median is above 20 pixels is far out
(`far_out`), as for the imaging camera. The readings go through the imaging
camera's own arithmetic: the median of the last three since the last change,
a reading more than 15% from it as a turn of the focuser, the trend, the best
so far and `found` once the reading has been far from the best, near it, far
and near again. Beside the reading the page shows how many stars the node
detected in that frame.

It does not stop the live solve, which goes on with the same frames; Track
may stay on, and keeps the stars still. Each reading costs one raw frame on
the scope link, 1,228,800 bytes for the finder camera's 1280 × 960, so run
the console beside the scope node on the Pi, as the field kit does: over a
slower link, such as a phone's hotspot, one raw frame can hold up the link's
heartbeat until the link drops. It ends `stopped` by a stop, or `failed` with
the words when the scope node is not connected, cannot send raw frames, sends
none for 15 s, or changes exposure or gain. Far out of focus the scope node
finds no stars at all, and the aid says so: turn the focuser slowly through
its range, watching the live preview, until stars show, then use the aid.

At its sharpest a star may be small enough for the scope node's detection to
take it for a hot pixel, as cedar-detect itself warns. If the detected count
falls as the reading reaches its lowest, come back to where the count is
highest within a tenth of the best: the live solve needs the stars more than
the last tenth of sharpness. After a change of optics (a spacer, a reducer)
stop the aid and start it again, since the best so far is the old optic's.
Sizes are in the finder camera's pixels, which compare exactly for one camera
behind one optic.

## Calibration frames

A master dark records the sensor's own signal, including its offset and fixed
pattern. A master flat, with its own dark subtracted and each colour normalised
by its median, removes vignetting and dust shadows. A bad-pixel map names sensor
samples to repair from good neighbours of the same colour. The calibration
function works on the sub's mosaic: subtract the master dark first, divide by
the master flat next, then repair bad pixels, all before any colour processing.
It keeps values below zero. Its report says `applied` or `missing` for `dark`
and `flat`, and `map` or `sub` for `bad_pixels`: without a map, hot pixels are
found in the sub itself. It also counts `repaired`, `not_repaired` and `low_flat`
samples; a flat sample below 0.1 is repaired instead of divided by.

A master dark's bad-pixel map lists, row by row from the top, every sample
that is not finite or reads more than 3.5 spreads above its median. The spread
is the median less the 16th percentile, at least one count; a fixed column
pattern therefore widens the spread instead of being mapped. The map is kept
beside the dark as `darks/<id>.badpixels.json`, format `finder-bad-pixels/1`,
with `width`, `height`, `level`, `spread`, `sigmas` (3.5) and `pixels` (x, y
pairs); its size is the master's `bad_pixels`. More than 1 percent of the
samples is too many: no map is written and the search in the sub stands in
for it. A master only added to the library has no map until one is attached.
An unreadable map, an unknown format or a size that differs from the sub also
falls back to that search, with the reason in `problems`.

Without a map, a hot sample must pass three tests: it stands more than eight
of its colour's noises above its largest same-colour neighbour along the row
or column; its height above its colour's median exceeds five times that
neighbour's height; and at most one touching sample is lit. A touching sample
is lit only above both three of its own colour's noises and a tenth of the
candidate's height above its median. Noise is the colour's median less its
16th percentile, at least one count. Same-colour neighbours are two samples
away on a Bayer mosaic, one on mono. Repairs take the median of the good
neighbours, leaving other bad samples out. This search can take the cores of
stars under 1.4 samples across at half height, or whose peaks stand under
about 30 noises above their median. Three hot pixels that touch can be left
unrepaired; in nebulosity a hot pixel must stand more than ten times the
nebulosity's height above the median to be found.

The library lives in `calibration/` under the imaging folder (`imaging/` in
the state folder, or `--imaging-dir`). Its files are `index.json`, format
`finder-calibration/1`; `darks/<id>.fits`; `darks/<id>.badpixels.json` for a
dark's map; and `flats/<id>.fits`. Masters are 32-bit real FITS images, so a
mean finer than one count is retained. An id is `dark-` or `flat-` followed by
the UTC second as `YYYYMMDDTHHMMSSZ`, with `-2`, `-3` and so on for collisions.
Writes use a `.part` file and rename it, and never replace a master. An
unreadable index yields an empty list and `error`; the next addition moves it
to `index.json.unreadable` before writing a new index. The library is separate
from `session.json`.

Each master's entry lists all these keys, with absent values written as null:

- `id`, `kind`, `made_at` (console Unix seconds), `frames` and `checked`
  (whether the dark test judged the contributing darks).
- `model`, `serial`, `width`, `height`, `origin_x`, `origin_y` (origin in binned
  pixels), `bits`, `bayer`, `gain`, `offset` and `binning`.
- `set_temp_c`, `temp_c` (mean sensor temperature) and `exposure_ms`.
- `level` (a dark's median), `bad_pixels` (its map's size), `label` (a flat's
  label), `imaging_angle_deg`, `level_share` and `dark_frames` (the flat's darks).
- `file`, relative to `calibration/`.

The FITS structural cards are `SIMPLE`, `BITPIX` (-32), `NAXIS`, `NAXIS1` and
`NAXIS2`. After them come `IMAGETYP` (`MASTER DARK` or `MASTER FLAT`), `CALID`,
`NFRAMES`, `NDARKS` (flats only), `DARKTEST`, `EXPTIME` (seconds), `GAIN`,
`OFFSET`, `XBINNING`, `YBINNING`, `XORGSUBF`, `YORGSUBF`, `CCD-TEMP`,
`SET-TEMP`, `BAYERPAT`, `INSTRUME`, `SERIALNO`, `ROWORDER` (`TOP-DOWN`) and
`DATE`; flats also carry `FLATLBL` and, when known, `IMGANGLE`. Optional
camera settings and temperatures are omitted when unknown. `DARKTEST` is
false when the dark test was skipped.

Every matching master must have the same camera model and serial, delivered
width and height, binned origin, bit depth, Bayer pattern and binning as the
imaging frame. A dark also needs gain and offset both absent or equal within
0.001, and exposure within 1 ms. With set temperatures, both must have one
and differ by at most 1.0 °C. With neither set, sensor temperatures must be
within 8.0 °C, or one may be unknown. The nearest temperature wins, then the
newest `made_at`, then the greatest id. A flat needs only the camera, size,
origin, bits, pattern and binning: its gain, offset, set temperature, exposure
and sensor temperature do not restrict its match. A requested flat label
restricts it to that label; otherwise the newest flat wins, then the greatest
id. Masters are listed oldest first, by `made_at` then id.

A camera turn of more than 1.0° since the selected flat produces
`flat_turned_deg`; the flat is still selected. The imaging angle is known only
while the aim point is the measured one. Take new flats after turning the
camera. These five tolerances — 0.001 for settings, 1 ms for exposure, 1.0 °C
for set temperature, 8.0 °C for sensor temperature and 1.0° for the turn
warning — are independent of any camera model.

`GET /api/imaging/calibration` always answers 200 with the library object,
exactly `{"masters":[]}` on a console that takes no imaging frames. While the
camera is ready, that same object is `calibration` in the imaging view and in
the session's `imaging`, including those returned by MCP. `match` describes
the latest imaging frame seen saved whose purpose is neither `dark`, `flat`
nor the focus aid's `focus`, using that imaging frame's own set temperature.
It is absent until such an imaging frame is seen and stays through later
exposures, darks, flats and focus frames. It names the imaging frame's id as
`frame`, the selected `dark` and `flat`, the dark's `bad_pixels`, and lists
what is `missing` in dark, flat, bad-pixels order.

`DELETE /api/imaging/calibration/{id}` removes a master and its files and
answers 200 with the updated calibration object. An unknown or malformed id
answers 404 with `no master or set has the id <id>`; malformed ids touch no
files. A console without imaging answers 409 with `this console takes no
imaging frames`. Same-origin writes apply first, so a foreign Origin answers
403 even without imaging. Masters and their files are kept until this DELETE
route removes them; there is no automatic expiry.


Cover the telescope, then send `POST /api/imaging/calibration/darks` with
`{"count":20}` to take darks at the exposure, gain, offset, binning and set
temperature in force. The answer is immediate: `set` names the latest set's
`id`, `kind`, `state`, `wanted`, `taken` and `exposure_ms`; as measurements
arrive it adds `offset_level`, `level`, and on failure `error`. The set first
reads an **offset frame**, a dark at the camera's shortest exposure. Its
median is the **offset level**; its noise is that median less the 16th
percentile, with a floor of one count. Then it takes the requested number
of darks, averages their samples, saves the master dark and attaches its
bad-pixel map. The offset frame is excluded from that mean and from the
master's mean sensor temperature.

The dark test refuses in this order:

- An offset level above 20 percent of full scale:
  `the camera is not covered: its shortest exposure reads <level> of <full scale>`.
- Any dark's median more than twice the offset frame's noise above the offset
  level: `the darks are not dark: one reads <level> against an offset level of <offset level>; cover the telescope`.
- Any dark with more than eight bright samples and more than one in 10,000
  of its samples bright, excluding hot pixels:
  `the darks are not dark: one holds <n> bright samples that are not hot pixels; cover the telescope`.

These failures add no master and put `not_dark: true` in `set`. A covered
warm sensor, a long exposure or sensor glow can read above the line; a
field with almost no stars can pass despite being uncovered. The test's
constants have not been measured on the real camera. Only when the
telescope is covered, `{"count":20,"anyway":true}` takes and averages the
darks whatever these three tests say. The resulting master's `checked` is
false and its FITS card `DARKTEST` is F; otherwise they are true and T.
A sensor more than 1.0 °C from its set temperature fails even with `anyway`:
`the sensor is at <t> °C, not yet at its set temperature <s> °C`.
Every saved imaging frame's own settings and exposure must agree throughout
the set; a camera changing them fails with
`the imaging settings changed during the set`.

Every dark, including the offset frame, is kept as an ordinary imaging
frame under `frames/`, with purpose `dark` and `IMAGETYP` DARK. At the
worked example's 4944 × 3284 size, 20 darks cost about 650 MB of saved
imaging frames and their master about 65 MB. They are not automatically
deleted. Stop a running set with `DELETE /api/imaging/calibration/<id>` or
by aborting its exposure with `POST /api/imaging/abort`. The final state
is `done`, `failed` or `stopped`; before publishing it, the console puts
the original exposure back and releases the imaging camera. If the
camera is no longer ready, it may refuse that restoration.

A set holds the imaging camera from acceptance until its end: an imaging
frame, a settings change, the focus aid or a stack asked for meanwhile is
refused with `a set of darks or flats` in the words. A set asked for while
another holds the camera is refused with that holder's name. A request
while a single imaging frame is in flight answers `an exposure is in progress`;
a request while another set runs answers
`a set of darks or flats is already being taken`. A single imaging frame
started on another thread just before the set acquires the camera can
make the set's first settings change fail, before any dark is taken.


With a flat panel lit, send `POST /api/imaging/calibration/flats` with
`{"count":20,"label":"desk"}`. The count is from 3 to 64. A **flat label**
is the short name for flats taken through one optical train, kept with their
master flat: 1 to 32 characters of `a` to `z`, `0` to `9` and `-`, starting
with a letter or digit. Use a label to distinguish optical trains; take new
flats when the camera is turned.

The first step starts in `search`: it finds an exposure whose largest colour
median is 40 to 50 percent of full scale (2^bits − 1). It starts at the
exposure in force, within the camera's range and at most 30 seconds. A level
at least 90 percent sends the search to one eighth of that exposure;
otherwise it doubles a dim first exposure or halves a bright one. Once two
unsaturated readings are available, their straight line predicts the exposure
for 45 percent. Each exposure is rounded to 0.001 ms, with at most eight
search frames. The failures describe what to fix:

- `the flat's level does not rise with the exposure: <a> ms and <b> ms read <A> and <B>`.
- `the flat is too dim: <e> ms reads <p>% of full scale; is the panel lit?`.
- `the flat is too bright: <e> ms reads <p>% of full scale; dim the panel`.
- `no exposure put the flat between 40 and 50% of full scale in 8 tries; the last, <e> ms, read <p>%`.
- After the search, a flat outside the window fails with
  `a flat reads <p>% of full scale, outside 40 to 50; is the panel steady?`.

The console then takes the requested flats in `frames`, excluding the search
frames, and holds their sum in `needs_darks`. The view adds `label` and the
flats' mean `level_share` of full scale. Cover the telescope and send
`{"darks":true}` to the same route. This second step is `flat_darks`, with
`darks_taken` counting its darks while `taken` still counts the flats. It
reads one offset frame and then the same number of darks at the flats'
exposure, using the three dark tests above. `{"darks":true,"anyway":true}`
passes those tests and records `checked` false and `DARKTEST` F. Settings
and set-temperature checks still apply. Failure returns to `needs_darks`
with `error` and, when the dark test refused it, `not_dark`, so it can be
retried with the flats still in hand.

The master flat is the mean of the flats less the mean of their darks,
normalised by each colour's median, with samples not above zero set to zero.
Its entry and FITS cards keep the flat label, counts of flats and darks,
exposure and the flats' sensor temperature. Each search exposure, flat and
dark is kept under `frames/` as an ordinary imaging frame, with purpose
`flat` or `dark`. The master is saved under `calibration/flats/`.

Each step holds the imaging camera while it runs, restores the exposure that
was in force before that step, and releases the camera before publishing
`needs_darks`, `done`, `failed` or `stopped`. Waiting flats hold no lease:
the camera is free for imaging frames and settings changes between the two
steps, but the settings must agree again when their darks are taken.
While flats wait, requests for new darks or flats answer
`flats are waiting for their darks: take those or drop them first`.
`DELETE /api/imaging/calibration/<id>` drops waiting flats at once, or stops
a running step. Flats in hand live only in memory and are lost when the
console restarts.

## Live stack

A stack builds one picture from subs taken one after another: each is
calibrated, turned into colour, placed on the reference sub's grid and
averaged, with outliers rejected. It runs on whatever the imaging camera
points at. Do a GoTo and turn Track on first. Stacking does not move the
telescope or start Track for you. A trailed sub that solves is stacked
unless the focus gate finds it soft; there is no separate trailing test.

Start from the Imaging page, `POST /api/imaging/stack` with `{}`, or MCP
`set_stacking` with `{"on":true}`. The stack holds the imaging camera from
its start until its taking task returns as the stack ends, releasing it
before the final files are written. Single imaging frames, the focus aid and a set
of darks or flats are refused while it holds the camera, with
`the imaging camera is held by the stack`. A stack requested while another
holder has the camera is refused with that holder's name. A single imaging
frame already in flight also refuses a start. The finder camera continues
solving while the imaging camera stacks.

Subs are handled in this order: check the saved frame and its settings
against the reference sub, choose a hint, solve, take and judge a focus
reading when the focus gate is on, check its placement, read and calibrate
its mosaic through the calibration library, turn it into colour, match its
background and add it with rejection. The first accepted sub is the
reference sub. Missing masters do not stop stacking; `missing` says which
of `dark`, `flat` and `bad_pixels` were absent for the latest calibrated sub.

A sub's hint comes first from live pointing carried through the aim point,
then from the stack's last solved sub, then from the session's target, or
from no position. The initial hint and the fallback after three successive
solve misses use the imaging solver's usual hint selection, which can also
use a previous imaging solve. A live pointing always takes precedence.
The stack's own hint uses its last solved centre and this sub's diagonal
on the sky as its radius; after three misses it is set aside until a sub
solves. A focus gate refusal does not count as a solve miss. The live
pointing's hint is narrow only once the aim point is measured and an
imaging scale is kept; otherwise its radius is 3°. Each stacked sub's solve
keeps the aim point measured while the finder camera solves too, but keeps
no scale and does not replace the page's imaging solve. Solving a single
imaging frame before stacking can keep that scale.

The reference and sub's imaging solves place the sub: their full matrices
map between the two sky projections, carrying shift, rotation, scale and
shear. The linear map is checked at all four corners of the reference grid
and refused if it differs from those projections by more than 0.5 px.
The imaging solver must therefore give consistent centres and matrices to
a fraction of a pixel; a solved position alone is insufficient. A mirror
or an area scale differing by more than 2% is refused, as is a centre
shift beyond half the reference grid's shorter side. Each pixel's own
weight counts the subs that reached it, so uncovered edges stay honest.

Rejection is on by default to remove satellite streaks and other outliers.
Each pixel learns from its first eight subs. Thereafter a sample more than
three running standard deviations from its mean, with a floor of one
count for the deviation, is left out in all three channels when any one
channel fails. The deviation uses the sample variance, divided by the
weight less one. A streak in the learning subs remains at a fraction of
its strength and fades as more subs arrive. Background matching first
adds a separate offset to each channel to match the reference sub's
median, so changing sky brightness does not itself become an outlier.

The twelve dropped-sub reasons are:

- `not_solved`: the imaging solver found no solution.
- `too_few_stars`: the solve or focus reading found too few stars.
- `solver_failed`: the solver, focus measure or solve-sidecar write failed.
- `soft`: the focus reading was far out, or more than 30% above the median of at least five stacked subs' readings; it is taken in the console's kept focus region when that region lies inside the sub, otherwise over the whole sub.
- `off_field`: the placing map failed its corner check, or the centre shifted too far.
- `solve_differs`: the placing map has a wrong area scale, a mirror or no inverse.
- `settings`: size, depth, Bayer pattern, binning, region, gain, offset or exposure differs from the reference sub's settings.
- `frame_failed`: the imaging frame could not be received or saved.
- `unreadable`: a saved sub lacks needed metadata or its FITS file could not be read.
- `behind`: the waiting queue was full, so the oldest waiting sub was dropped.
- `unprocessed`: the stack ended before the sub could be processed.
- `not_steady`: the tracker was not steady during the sub, which was dropped before its solve (040).

The camera is never kept waiting for processing. At most four saved subs
wait; `behind` means processing did not keep up, and the newest subs are
kept in preference to the oldest. Three failed imaging frames in a row end
the stack with reason `camera` and the frame's error. Stopping with the
page, `DELETE /api/imaging/stack`, or `set_stacking` with `{"on":false}`
aborts an exposure in flight and waits for processing to end and files to
be written. The latest stack and picture remain readable after it ends,
until the next start. Neither stacking tool writes a call-log line or is
a moving tool.

The imaging view, the session's `imaging` and MCP `get_stack` carry these
stack keys (optional keys are absent until known):

- `state` (`stacking` or `ended`), `id`, `started_at` and `options`.
- `taken`, `stacked`, `queued`, `dropped` (every reason above) and `exposure_s`, the sum of the stacked subs' exposures in seconds.
- `reference`: its `frame_id`, `width`, `height`, `ra_deg`, `dec_deg`, `up_east_of_north_deg`, `arcsec_per_px` and `exposure_ms`.
- `last`: the last sub reaching the solver, with `frame_id`, `outcome`, `took_ms`, and when known `turn_deg`, `shift_px` (x, y), `rejected_px` and `focus_arcsec`.
- `missing`, `focus_median_arcsec` and `preview`, the latest picture's URL with a counter that changes whenever a picture is made.
- `ended_at`, `ended_reason` (`stopped` or `camera`), `error`, `files` and `save_error`.

Before a stack exists the imaging and session views omit `stack`;
`get_stack` answers `{"state":"none"}`. Its picture, when kept, is a second
MCP content block containing a colour JPEG at most 1024 wide. The page's
JPEG is at most 1920 wide. At night it is red until Hold to see colour is
held, for at most ten seconds per hold.

For display, whole blocks reduce the sums and weights directly; each
channel has a fitted background plane taken away, then the observer's
gains are applied. One span shared by all three channels and an `asinh`
curve produce the picture. These display steps change nothing saved in
the FITS samples. Gains default to `[1,1,1]` and each may be from 0.1 to
10; stretch defaults to 10 and may be from 1 to 1000. A start can also set
`reject` and `focus_gate`, both true by default. On a running stack only
gains and stretch can change; stop first to change either gate. Changing
gains or stretch remakes the picture immediately when one is kept. No
colour matrix is applied, so gains may need adjustment for the camera.

Everything is under the console's imaging folder, in `runs/<id>/`. The
id is the UTC start with colons made dashes, then `Z` and, when the session
has a target, its label (for example `2026-01-16T02-00-00Z-M42`); a clash
adds `-2`, `-3` and so on. Labels keep ASCII letters and digits and replace
each other character with a dash. Every sub, including a dropped one,
is saved as `subs/<frame id, six digits>.fits`, with its solve sidecar
beside it when a solve was attempted. Every other imaging frame stays
under `frames/`.

The start writes `stack.json`. The end writes `stack.fits` and `stack.png`
when at least one sub was stacked, then finishes `stack.json`. A stack
with no stacked sub writes only JSON. Files are written through `.part`
and renamed whole; `files` names those written after a successful save,
and a failure reports `save_error`. If the console exits mid-stack,
nothing but the start's `stack.json` is saved for the stack itself; the
subs and sidecars already saved remain. Nothing of a stack is kept in
`session.json`.

`stack.fits` holds three full-size planes of 32-bit reals, red, green and
blue, row by row from the top: the calibrated mean before display. Its
structural cards are `SIMPLE`, `BITPIX` (-32), `NAXIS` (3), `NAXIS1`,
`NAXIS2` and `NAXIS3`. Following them are `IMAGETYP` (`STACK`), `STACKID`,
`NSUBS`, `EXPTIME` (total seconds), `SUBEXP` (reference sub seconds),
`DATE-OBS`, `INSTRUME`, `XBINNING`, `YBINNING`, `ROWORDER` (`TOP-DOWN`),
`CTYPE1` (`RA---TAN`), `CTYPE2` (`DEC--TAN`), `CRPIX1`, `CRPIX2`, `CRVAL1`,
`CRVAL2`, `CD1_1`, `CD1_2`, `CD2_1` and `CD2_2`. The celestial cards come
from the reference sub's imaging solve; the FITS reference pixel is
(width/2 + 0.5, height/2 + 0.5). `stack.png` is the full-size 8-bit RGB
picture with the final gains, background removal and stretch.

`stack.json` records `format` (`finder-stack/1`), `finder` (version), `id`,
`started_at`, `started_utc`, `target`, `options` and `camera` (model and
serial). At the end it adds `ended_at`, `ended_reason`, `error` when there
is one, `taken`, `stacked`, `dropped`, `exposure_s`, `reference` and `subs`.
The reference is the view's reference with `cd_deg` and `file`, or null
without a stacked sub. Every taken sub is listed in order with `frame_id`,
`file` and `outcome`; stacked subs also have `turn_deg`, `shift_px` and
`rejected_px`, and a measured sub has `focus_arcsec`. A sub that never
arrived has a null `file`. All file names here are relative to this run
folder, such as `subs/000001.fits`, so the folder opens on its own in
another program.

Memory grows with the delivered camera size: three planes of sums, one
plane of weights and, with rejection, three planes of running squared
deviations, plus the sub's mosaic and three colour planes while processing.
The picture is made from a reduction, with no full-size mean in between;
a full-size mean is made only when saving the FITS at the end. At
4944 × 3284 the stack alone is about 455 MB, the sub in hand adds about
260 MB, and saving needs further working buffers for the mean, FITS bytes
and full-size PNG. Binning 2 × 2 quarters the pixel storage and work.

At bring-up, compare `last.took_ms` with the sub length. It measures real
milliseconds from taking the sub off the queue through its outcome,
including solving and calibration. The picture the worker makes after
that sub is not included. Read `behind` too: it exposes processing that
cannot keep pace with the camera.

## Tracking steered by the subs

On a mount whose field turns as it tracks, a stack with Track on uses each
accepted sub's tracking error at the set point to correct the tracker. The
subs steer instead of the finder camera; the finder camera still feeds the
pointing model. Corrections happen only in the gap between exposures. A gap
steps out half the error when it is two counts or more, and takes 0.3 of the
drift rate out of the rate. The settings `step_share` and `rate_share` tune
those shares in the field. An error arrives a sub or more late, so the
tracker judges it against corrections made since it was measured. One gap
changes a rate by at most 0.1″/s; a pair of subs with a large step between
them changes it by nothing. A gap steps at most 30″ on the sky. A correction
never reverses an axis. A step against its motion is at most its own rate
times half a second per gap, so an error in that direction leaves slowly.

The tracker's own corrections steer again when the stack stops, when the
switch is turned off (at once, during a stack too), after eight gaps in a
row without an accepted sub, while the tracker is not steady, or when the
stack began more than 3′ from the set point. After eight empty gaps its
own corrections keep steering until a sub is accepted, applied only as
steps in the stack's gaps; the rate stays as the subs left it, so no
correction changes the rate during an exposure. Every gap without an
accepted sub counts, whether a sub was dropped as `behind`, `settings`,
`frame_failed`, `unreadable` or `solver_failed`, or is still being solved.
A console needing more than eight sub lengths to accept one sub therefore
takes no drift out of the rate. Turning the switch off lets the finder
camera correct as before, during exposures too.

A stack whose first tracking error exceeds 3′ is not steered at all. The
finder camera's aim point and the sensor's centre disagree by more than
Track's own push threshold: steering the subs could look like a push
nobody made. Centre the target, or turn Track off and on to apply the
measured aim point, then start a new stack.

The cap keeps corner stars within the chosen smear, 6 px by default. Before
the first sub, the shortest limit in the hour ahead decides the exposure:
the observer's setting is halved until it fits, never below 1 s or the
camera's shortest exposure. That exposure is held because all subs of a
stack share one exposure and a master dark matches one. The stack sets the
console's exposure setting through its own camera lease; the setting
cannot be edited while the stack holds the camera. It puts the observer's
setting back before releasing the camera as it ends. If the initial change
is refused, the stack ends with reason `camera` without taking a sub. A
refused restoration leaves the stack's exposure setting in place and
changes nothing else about the ending.

`over_limit` means the current limit has fallen below the exposure held.
The stack keeps running until that limit falls under half the held
exposure; then it waits, letting the tracker steer on its own, until the
limit rises again. `would_take_s` says what a stack started now would
hold. A stack begun while the limit was low may hold much less than that.

Before every exposure the stack also waits for the tracker to be steady.
It is not steady before its first rates, during a take-up, during a first
reversal's backlash measurement and the keyhole pass, and for 2 s of
settling after the first rates, a take-up and the keyhole pass. It is also
not steady while the telescope is moved by hand, until Track follows the
push or it comes back, for the second in which the tracker finds an axis
stopped, and after tracking ends. A sub that crosses such a time is dropped
as `not_steady` before its solve. The scope node stops both axes when a
rate command fails and logs `mount command failed`. The console is not
told of this failure; the tracker can only notice a second later.

The imaging view's `steering` part carries these keys; optional keys are
absent until known:

- `source`: `subs` while tracking errors steer, `tracker` while the tracker's own corrections do (no stack, switch off, a stack begun too far out, or no sub accepted in the last eight gaps), or `none` while nothing tracks.
- `subs`: tracking errors used since the tracker was last let go.
- `last`: the newest tracking error, with `east_arcsec`, `north_arcsec` and `time` in Unix seconds at the sub's mid-exposure.
- `holding`: a stack holds the steering. With `source` `tracker`, `on` true and no `far_arcsec`, no sub has been accepted lately.
- `far_arcsec`: present while a stack is not steered because it began this far from its set point.
- `smear_px`: the corner smear setting, from 0.1 to 20 px, default 6.
- `on`: whether the subs steer, default true. Off still caps, pauses and drops subs that cross a time not steady.
- `step_share`: the share of an error stepped out in a gap, from 0 to 1, default 0.5.
- `rate_share`: the share of drift taken out of the rate, from 0 to 1, default 0.3.
- `exposure_s`: the exposure the stack holds, in seconds.
- `limited`: whether that exposure was shortened when the stack began.
- `longest_exposure_s`: the longest exposure that keeps corner stars within the smear now, or `no limit`; absent while nothing tracks.
- `over_limit`: whether the current limit is under the exposure held.
- `would_take_s`: the exposure a stack started now would hold.
- `steady`: false when the stack waits for the tracker to be steady.
- `waiting`: true while the stack waits before an exposure, for the tracker or its limit.
- `pause`: while waiting, the tracker's reason in one word: `starting`, `reversing`, `measuring`, `overhead`, `pushed`, `stalled`, or `limit` while waiting for the longest exposure to rise. Other trackers may give other words.
- `dropped_not_steady`: this stack's subs dropped because the tracker was not steady during them, equal to `stack.dropped.not_steady`; retained after stop and reset for the next stack.
- `error_rms_arcsec`: root mean square of the last ten tracking errors, in arcseconds; present once two errors have been handed over since the tracker was last let go.
- `error_rms_px`: the same tracking error RMS in delivered pixels, at the latest solve's scale.
- `drift_arcsec`: root mean square of the drifts between those errors, in arcseconds per sub, after taking out the gaps' steps; present with the error RMS.
- `drift_px`: the same drift RMS in delivered pixels, at the latest solve's scale.

`stack.dropped` counts dropped subs by reason: the solving, focus,
placement, settings, frame, queue and processing reasons listed under
Live stack, plus `not_steady` for a sub taken across a pause or another
time the tracker was not steady. Such a sub costs no solve and does not
replace the stack's last solved sub.

Drift per sub includes the scatter between two solves, so it does not read
0 on a real sky. All four steering settings are run-only, set with
`PUT /api/imaging/steering`; restarting the console restores 6 px, on,
0.5 and 0.3. Guiding from the imaging camera, deferred in 015, is now part
of tracking through the accepted subs.

## An imaging run

An imaging run goes to the observing session's target with GoTo, waits for
tracking to settle, takes a first imaging frame, checks focus, centres the
target on the imaging camera's sensor by solving imaging frames and asking
Track again, then stacks subs until stopped or a limit is reached. Choose the
target first; on the Imaging page press `Start imaging run`, or from an MCP
client call `start_imaging_run` after `set_target`. The camera and the imaging
solver must be ready, and Track must accept the move under its usual guards.

The imaging camera is held throughout, by the run, the focus aid or the stack
in turn. A single imaging frame, a focus aid, a set of darks or flats, or a
stack asked for meanwhile is refused with the holder's name. The run moves
the mount only by asking Track. It stops the mount only when the run itself
is stopped while it still slews; after a run, Track is still on unless a
guard or Stop ended the motion. A run cannot outlast Track.

The state words are the same on the page, in the imaging view's `run` and over
MCP. In the messages below, angle brackets stand for the target or a reading.

| State | What is happening and what the page says |
|-------|-----------------------------------------|
| `slewing` | GoTo is bringing the target to the aim point: `Slewing to <target>.` |
| `tracking` | Track is on; the run waits for steady motion: `Tracking <target>. Waiting for the mount to settle; nothing to do.` |
| `first_frame` | A first imaging frame is being taken: `Taking a first imaging frame.` |
| `focus_check` | The focus check is taking its readings: `Checking focus.` |
| `focus_pause` | Focus needs the observer, or the check could not finish. The message gives the verdict and advice; the focus aid opens below it, with `Check focus again` and `Go anyway`. |
| `solving` | Another imaging frame is taken and solved to measure the target's offset and the aim point: `Solving an imaging frame.` |
| `centring` | Track is asked again using the measured aim point: `Centring <target>: <offset> from the sensor's centre, try <n> of 3.` Without a new aim point it says `no new aim point yet.` |
| `stacking` | The stack takes subs: `Stacking <target>: <n> stacked, <n> dropped.` |
| `paused_by_tracker` | The stack waits for steady tracking or a usable exposure limit: `Exposures paused: <n> stacked so far. They resume by themselves.` |
| `ended` | The run is over. Its message says why, followed by whether anything was stacked and saved, and the folder. A new run starts from the beginning. |

At a focus pause, turn the focuser while watching the aid, then press
`Check focus again`, or call `continue_imaging_run` with `anyway` false.
`Go anyway`, or `anyway` true, goes on without another check and keeps a
warning. Only the first answer to a pause counts. The pause can last at most
15 minutes; the page shows its time left. Every answer, and a Stop, closes the
aid, which puts the camera's settings back as `## Focus` says, and the run then
holds the camera again. When the aid could not put them back, the run ends
`camera_failed` with the camera's own words after
`the focus aid could not put the imaging camera's settings back:`. Clear the
region with `Use the whole sensor` and start again.

The ending's `reason` is one of the words below. The mount column describes
the motion at the ending, before a later Stop or guard. Check the page's
Track indication and the mount itself, especially after a lost link.

| Ending | Meaning | Mount afterwards | What to do next |
|--------|---------|------------------|-----------------|
| `stopped` | Four sentences distinguish the source: `Imaging run stopped. Track is still on.` is the run's own stop; `Stopped: the mount was stopped.` is the console's Stop or a stop of the run with Track already off; `Ended: the stack was stopped.` is a stack stopped through its own route or an aborted sub; `Ended: the focus check was stopped.` is a focus check stopped through its own route. | In the same order: Track on; at rest (including a run stopped while slewing, or after a guard or lost link ended Track); Track on; Track on. | Nothing if finished; start a new run to take more. Call `stop` if Track is still on and the mount should rest. |
| `count_limit` | The requested number of subs has been stacked. | Track on. | Nothing, or start another run; call `stop` to rest the mount. |
| `duration_limit` | The stacking time limit has been reached. | Track on. | Nothing, or start another run; call `stop` to rest the mount. |
| `too_low` | The target fell below the minimum altitude. | Track off; mount at rest. | Choose a pointable target and start again. |
| `sun_up` | The Sun guard ended Track. | Track off; mount at rest. | Wait until the Sun is down before starting again. |
| `link_lost` | The mount or scope link stopped answering. A link lost while stacking may read `camera_failed` instead. | Track off; the node's watchdog stops the mount when the link is lost. Check that it is at rest. | Check the scope node and the mount's cable, then start again. |
| `mount_failed` | Track or GoTo ended for a reason of its own, a guard's among them; the message carries its words. | Track ended; after a re-centre that Track refused, Track is off. | Check the mount and the reason in the message, then start again. |
| `target_changed` | The observing session's target changed during the run. | Track may still be on; it is off when the target changed during a re-centre. | Start a new run for the new target; call `stop` first if the mount should rest. |
| `pushed_away` | A push by hand settled away from the target or lasted too long. | Track follows where the telescope is left. | Start a new run to return to the target, or call `stop`. |
| `camera_failed` | The imaging camera failed; the message gives its words. A link lost while stacking may read this or `link_lost`. | Track on unless the lost link or another guard ended it. | Check the camera and scope link, then start again; call `stop` if necessary. |
| `solver_unavailable` | The imaging solver cannot run. | Track on. | Check ASTAP and its database, then start again; call `stop` to rest the mount. |
| `solver_failing` | Three imaging frames in a row could not be solved. | Track on. | Check for cloud, dew and focus, then start again; call `stop` if finished. |
| `save_failed` | The console could not save to its imaging folder or reserve room for the stack's save. | Track unchanged. | Check the disk and saved subs before starting again; call `stop` to rest the mount. |
| `focus_timeout` | The focus pause was left unanswered for 15 minutes. | Track on. | Check focus and start again when ready, or call `stop`. |
| `not_centred` | The target is still too far from the sensor's centre after three tries, or four solves measured no new aim point. | Track on. | Check the aim point and exposure settings, then start again; call `stop` if finished. |
| `subs_failing` | No sub was stacked for 10 minutes of stacking with failed subs. The message lists the reasons. | Track on. | Check the sky for cloud, dew or a tree, then start again; call `stop` if finished. |
| `stack_failed` | The stack stopped with a failure whose words the message carries. | Track on. | Check the message and saved subs, then start again; call `stop` if finished. |

The page reports the saved stack only when the save succeeded and at least one
sub was stacked; otherwise it says no sub was stacked, or says the stack was
not saved and gives the subs' folder. An ending can carry warnings about a
check skipped, a target stacked off centre, a save that failed or subs kept.
The line saying Track is still on disappears once Track ends; the `track_on`
value in the saved record is the value at the ending.

`subs` is an optional whole number from 1 to 100000, counting subs stacked,
not exposures taken. `duration_s` is from 1 to 43200 seconds and defaults to
7200 s (two hours). The first limit reached ends the run. Stacking time counts
on the clock from the start of stacking, including tracker pauses; elapsed
time on the page counts from the press. `keep_subs` defaults to true.

The run allows three failed takes in a row of its own imaging frames, with
5 s between retries, and three failed solves in a row. It asks at most three
centring tries, or gives up after four solves in a row that measure no new
aim point. Ten minutes of stacking with failed subs and none stacked ends
the run; a pause by the tracker and subs the console was too far behind to
process (`behind` and `unprocessed`) do not count towards that failure rule.
A re-centre waits up to 5 s after its Track off for a fresh finder camera
pointing. Before every imaging frame of its own, the run waits for the tracker
to have been steady for 10 s or the imaging exposure, whichever is longer.
A push by hand has 10 s to spring back; if it settles away, Track follows the
place left instead and the run ends.

Set the imaging exposure at least as long as the finder camera's exposure,
so that a finder solve falls inside each imaging frame; without that overlap
an imaging solve can measure no new aim point. Centring's tolerance is 5% of
the imaging frame's shorter side at the solved scale, with a floor of 90″.
If the frame is no longer the current one, the tolerance and the allowed
near distance both fall back to 90″. After three tries, or four solves in a
row with no new aim point, a target within a quarter of the frame's shorter
side is stacked with a warning: within 3′ of the centre the stack's steering
will bring it in; otherwise it is stacked where it is. A target farther out
ends the run as `not_centred`.

The run's own imaging frames are taken at the camera's exposure setting.
Only the stack's subs are capped by the tracker's exposure limit. On an
alt-az mount, a long setting can therefore trail the corner stars of the
imaging frames the run solves, even when the stack's subs are short enough.
Inspect those frames as well as the stack during the first night's checks.

The three routes are `POST /api/imaging/run`, `DELETE /api/imaging/run` and
`POST /api/imaging/run/continue`. Their MCP tools are `start_imaging_run`,
`stop_imaging_run` and `continue_imaging_run`; `get_imaging_run` reads the
latest run or null before the first one. The start takes optional `subs`,
`duration_s` and `keep_subs`; the continue takes `anyway`, false to check again
or true to go on unchecked. Read the imaging view or the MCP read tool to
follow progress; starts answer at once, stops answer after the save.

The run's own `Stop imaging run` button asks for a second tap within 5 s;
it is disabled while the stack saves. It cannot resume a stopped run.
`DELETE /api/imaging/run` and `stop_imaging_run` save the stack and leave
Track alone except while the run still slews. The top bar's Stop,
`POST /api/stop` and the MCP `stop` stop the mount at any time and end the
run too; the run then saves its stack. A stop during a re-centre also waits
for the Track off/on hand-over, up to about 7 s in addition to saving.
An exposure somebody aborts is a failed take when it is the run's own
imaging frame; when it is a sub it ends the stack and the run as `stopped`.

`run.json` is in the stack's folder, `runs/<stack id>/` under the console's
imaging folder, or in `runs/<run id>/` when the run ends before stacking.
It records `format` (`imaging-run/1`), `finder` (version), `id`, `by` (`page`
or `mcp`), `target`, `started_at`, `started_utc`, `settings`, `state`,
`events` (each state's `at` and `state`), `subs` (stacked and dropped by
reason), and, when present, `focus`, `centring`, `warnings` and `ended`.
The ending holds `reason`, `at`, `message`, `saved` and `track_on`. The
record omits the live view's message and elapsed time and contains no site,
node address or token. It is refreshed at state changes, at most once a
minute as the stacked count rises, and at the ending.

The stack's folder holds a `.reserve` file while the run stacks, as large
as the stacked picture's files can be: 15 bytes per binned pixel plus
16 MiB, about 260 MB for the worked camera's 4944 × 3284 full frame.
The run removes it before saving the stack. With `keep_subs` false, a
successful save removes the subs listed in `stack.json` and their solve
sidecars; it removes nothing when no sub was stacked or the stack ended by
itself. Copy the stack before leaving the console.

The stacked picture is written only when the stack ends. A console that
exits mid-run leaves the saved subs in `subs/`, a `run.json` with no `ended`
that is at most a minute old while stacking, and no stacked picture. The
page then says the last imaging run did not finish and names the subs'
folder. The old run cannot be resumed; keep its files and start a new one.

### Field checklist: the first imaging run

1. Before dark, put the imaging camera on the focuser, start the scope node and
   the console, and check that the Imaging page shows the camera ready; write
   down the camera and telescope used.
2. After dark, pair, set the site and wait for the first sync; write down the
   pointing model's RMS.
3. Choose a bright target well above the minimum altitude, turn Track on for
   it and leave it for 30 minutes. Write down whether Track ended and with what
   reason, since an imaging run cannot outlast Track.
4. Set the imaging exposure at least as long as the finder camera's exposure,
   so that a finder solve falls inside each imaging frame, then take one
   imaging frame by hand and solve it by hand. Write down how long the imaging
   frame took to arrive, the scale and the time the solve took.
5. Press `Start imaging run` with 20 subs asked for; write down the start time.
6. Write down the time from the press to `tracking`, and GoTo's last error.
7. At the focus check, write down the verdict, the focus reading and the focus
   record; if the run pauses, turn the focuser and press `Check focus again`.
8. At centring, write down each try's offset and tolerance, how many tries it
   took, and whether the telescope stayed still between the first imaging
   solve and the first re-centre's Track off: a newly measured aim point waits
   until Track is off. Open the last imaging frame the run solved and write
   down whether its corner stars are trailed, since the run's own imaging
   frames are not capped as subs are.
9. While stacking, write down the subs stacked and dropped by reason after
   5 minutes, and whether the run was ever paused by the tracker.
10. Let the run end at its 20 subs; check that Track is still on and the folder
    holds `run.json` and the stack, and that the subs it lists are on disk.
    Write down the ending, saved file names and Track state.
11. Start a second run and press the top bar's Stop while it still slews;
    check that the mount is at rest and stays there, and that the run ended
    `stopped`, and write down the message and mount state.
12. Start a third run and, while it stacks, tap the tube lightly; write down
    whether the run went on and what the page said.
13. Still in the third run, hold a hand over the front of the tube for a
    minute; write down the subs dropped by reason and whether the run went on.
14. Press the top bar's Stop while the third run stacks; check that the mount
    is at rest and stays there, and that the run ended `stopped`, and write
    down the message and saved files.
15. From an MCP client, call `set_target`, then `start_imaging_run` with
    `duration_s` set to 300; read `get_imaging_run` until it ends. Write down
    the ending and saved files, and check that the page said who started it.
16. Defocus the telescope on purpose and start a fifth run from the MCP
    client; at the focus pause, refocus and go on with `continue_imaging_run`
    and `anyway` false. Write down the verdict before and after.
17. End the fifth run with `stop_imaging_run` while it stacks; check that
    Track is still on, and write down the ending, Track state and saved files.
18. Copy the five `run.json` files and this filled-in list into the night's
    field note, with the paths of the saved stacks.
