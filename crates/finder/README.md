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

All phones and laptops connected to the console read and edit the same observing session.
A fresh session has no site, target or latest pointing, and a minimum altitude of 15°.
`time` is the console's clock in Unix seconds. The binary uses the system clock and has
no fixed-clock option.

Writes to `PUT /api/session` and `POST /api/solve` follow the same-origin rule. When
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
