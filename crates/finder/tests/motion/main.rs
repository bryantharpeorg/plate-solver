//! Tests that drive the simulated mount in paused time: GoTo, the pointing model,
//! tracking, Stop and the watchdog, the simulate-mode routes, and the whole-night scenario.

#[allow(dead_code, unused_imports)]
#[path = "../support/mod.rs"]
mod support;

mod goto;
mod model;
mod mount_calls;
mod mount_identity;
mod night;
mod recording_raw;
mod simulate;
mod stop;
mod tracking;
