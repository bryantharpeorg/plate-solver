//! Tests of the console's routes, session, live loop and scope link, in process.
//! No mount moves here. Two tests re-run this binary as a child process.

#[allow(dead_code, unused_imports)]
#[path = "../support/mod.rs"]
mod support;

mod aim;
mod api;
mod freshness;
mod imaging;
mod imaging_checks;
mod imaging_fits;
mod imaging_frames;
mod link;
mod live;
mod notices;
mod pointing;
mod pointing_up;
mod pushto;
mod pushto_separation;
mod rates;
mod recording;
mod recording_raw;
mod session;
mod star_detected;
mod targets;
