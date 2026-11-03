# finder: the finder console

A local web page for the plate solver. Give it a frame (one of cedar's sample frames, or
your own) and it shows where the frame points (RA/Dec, the constellation, the field of
view), the catalog stars it matched drawn on the frame as proof, the Messier objects in and
near the field, and objects like them that are up now.

It is the start of the electronic finder: the same program will later take the live camera
stream, talk to SkySafari and the mount, and give an AI assistant the console's actions.

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
| `--db PATH` | cedar's `default_database.npz` | Pattern database covering your camera's field of view. |
| `--samples DIR` | cedar's `medium_fov` samples | Folder of frames to offer on the page. |
| `--listen ADDR` | `127.0.0.1:8080` | Address to serve on. `0.0.0.0:8080` lets other devices connect. |
| `--state DIR` | `$XDG_STATE_HOME/finder`, otherwise `~/.local/state/finder` | Folder holding the observing session in `session.json`. |
| `--simulate` | off | In-process simulator on a virtual clock, with an isolated session and pairing. |
| `--sim-start UTC` | now | Start the simulated night at `YYYY-MM-DDTHH:MM:SSZ`, optionally with fractional seconds. Requires `--simulate`. |
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
eyes dark-adapted. It is a toggle in the top bar and the page remembers it.

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

## API

| Route | Answer |
|-------|--------|
| `GET /api/health` | The database's properties and how many samples there are. |
| `GET /api/samples` | The sample frames: `[{name, label}]`. |
| `GET /api/samples/{name}` | A sample frame's file. `…/thumb` is a 320-pixel JPEG preview. |
| `POST /api/solve` | Multipart: an `image` file or a `sample` name, and optionally `fov`, `fov_max_error`, `distortion`, `bin`, `sigma`, `stars`, `timeout_ms` (the `solve_image` flags). Answers a `SolveReport`. |
| `GET /api/similar?id=M42&lat=&lon=&time=` | Objects like `id`, best first. With `lat`/`lon` (degrees, east positive) and optional Unix `time`, each has its altitude and whether it is up. |
| `GET /api/session` | One shared observing session: `{site, min_alt_deg, target, pointing, time}`. |
| `PUT /api/session` | JSON partial edit: `{site?, min_alt_deg?, target?}`. Answers the updated observing session. |
| `POST /api/track` | JSON `{"on":true}` or `{"on":false}`. Answers 200 with the session; on refusal, 409 with a JSON `error`. Same-origin writes only. |
| `POST /api/goto` | No body. Start a GoTo to the session's target; 200 when started, 409 with a JSON `error` naming a refused move. Same-origin writes only. |
| `POST /api/stop` | No body. Always 200, ending tracking, clearing arming and commanding both axes to stop, from any origin. |
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

After the first accepted Track-on request, session reads include
`track: {wanted, state, target, ended_reason?, readout?}`. `state` is `armed`,
`tracking`, `keyhole_wait` or `off`; `target` is the canonical object being tracked.
`ended_reason` holds the last tracker or GoTo ending reason. While tracking, the
readout reports `rms_arcsec` over the last 60 s of live pointing errors and
`field_rotation_deg_per_min` at the target. Uploaded frames never drive tracking.
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

Turn is clockwise positive; tilt is up positive. `eyepiece_on_star` centres the brightest
database star in the field at the current time. Stars continue to drift with the sky.
An exposure already in progress finishes before a push or stop takes effect. Outside
simulate mode every `/api/sim/*` path answers 404. Unknown request fields are refused.

`GET /api/live` is `text/event-stream`. Each `live` event carries
`{frame_id, time, pointing_seq, preview_seq, stars, report, guidance?}`. Time is the
mid-exposure instant mapped to the console clock; `report` is the same `SolveReport`
as an upload, with `name: null`. Guidance is evaluated at that solve. Keep-alive comments
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
one second reports a lost link; sunrise reports the Sun guard. Every halt stops
both axes and cancels further motion. Stop remains available while planning,
solving or another request is pending.

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
