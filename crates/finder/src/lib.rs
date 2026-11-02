//! Finder console: a local web page that plate-solves one frame and shows where the
//! telescope points, what is in the field and what to look at next.
//!
//! The `finder` binary serves the page and a small JSON API over one loaded pattern
//! database. The page is a React app in `web/`; its production build (`web/dist/`) is
//! committed and embedded in the binary, so building the crate never needs Node.
//!
//! * `GET /api/health`: the database and how many samples there are.
//! * `GET /api/samples`, `GET /api/samples/{name}`, `GET /api/samples/{name}/thumb`: the
//!   sample frames, and previews of them.
//! * `POST /api/solve`: a multipart frame (an `image` file or a `sample` name) and
//!   options, answered with a [`solve::SolveReport`].
//! * `GET /api/similar?id=M42&lat=&lon=&time=`: objects like a given one, ranked for an
//!   observer, answered with a [`catalog::SimilarReport`].
//! * `GET /api/session`: the shared observing session's site, minimum altitude,
//!   canonical target, latest pointing (initially null) and console time in Unix seconds.
//! * `PUT /api/session`: a JSON partial edit with `site`, `min_alt_deg` or a target id
//!   such as `"M42"`. Omitted fields remain; `site: null` and `target: null` clear them.
//!   Invalid fields answer 400, unknown targets 404, with a JSON `error` and no changes.
//!
//! A fresh session has no site, target or pointing and a 15° minimum altitude. Site
//! coordinates are `lat_deg` (±90°) and `lon_deg` (±180°, east positive); the minimum
//! altitude accepts 0–80°. Unknown fields are refused, including Sun guard overrides.
//!
//! Writes (`PUT /api/session` and `POST /api/solve`) enforce the same-origin rule:
//! an `Origin` must match the request's scheme, Host and port. With an Origin, Host
//! must also be an IP literal, `localhost`, or a DNS name explicitly allowed with
//! repeatable `--allow-host NAME` flags (for example `--allow-host surface.local`).
//! A rejected origin or host answers 403 with a JSON `error`. Requests without an
//! Origin remain allowed for curl and Rust clients. Session edits require a JSON
//! content type; other media types answer 415 with a JSON `error`.
//! Library servers configure names with [`AppState::with_allowed_hosts`].
//!
//! Every successful solve records the latest pointing: its sequence, solved frame,
//! J2000 RA/Dec, console time, and altitude/azimuth when a site is set. Its report
//! carries the sequence as optional `pointing_seq`; a frame that does not solve
//! leaves the pointing unchanged. Site edits recompute altitude/azimuth at the
//! frame's original time; clearing the site drops only those horizontal coordinates.
//!
//! The binary keeps site, minimum altitude and target in `session.json` under
//! `--state DIR`, defaulting to `$XDG_STATE_HOME/finder` or `~/.local/state/finder`.
//! Writes are atomic; unreadable state is reported on stderr and retained with a `.bad`
//! suffix. The library defaults to memory only; opt into persistence with
//! [`session::Session::with_state_file`] and [`AppState::with_session`].
//! [`AppState::with_clock`] accepts an injectable [`session::Clock`] for tests and
//! simulations. The binary always uses [`session::SystemClock`] and has no fixed-clock
//! option. Time-dependent API defaults use the same clock.

pub mod api;
pub mod catalog;
pub mod constellation;
pub mod ephemeris;
pub mod grid;
pub mod link;
pub mod live;
pub mod pushto;
pub mod samples;
pub mod session;
pub mod simulate;
pub mod sky;
pub mod solve;
pub mod stars;
pub mod targets;

pub use api::{router, AppState};

/// Pointing model fitted from scope-node syncs.
pub mod model;
