//! Tests that drive the simulated mount in paused time: GoTo, the pointing model,
//! tracking, Stop and the watchdog, the simulate-mode routes, and the whole-night scenario.

#[allow(dead_code, unused_imports)]
#[path = "../support/mod.rs"]
mod support;

mod goto;
mod imaging_track;
mod model;
mod mount_calls;
mod mount_identity;
mod mount_sky;
mod mount_sky_night;
mod night;
mod recording_raw;
mod simulate;
mod steering;
mod steering_cap;
mod steering_claim;
mod steering_gap;
mod steering_stack;
mod steering_steady;
mod steering_unsteady;
mod stop;
mod tracking;
