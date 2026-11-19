//! Tests of the console's routes, session, live loop and scope link, in process.
//! No mount moves here. Two tests re-run this binary as a child process.

#[allow(dead_code, unused_imports)]
#[path = "../support/mod.rs"]
mod support;

mod aim;
mod api;
mod calibration;
mod calibration_bad_pixels;
mod calibration_console;
mod calibration_darks;
mod calibration_flats;
mod calibration_flats_sim;
mod calibration_library;
mod freshness;
mod imaging;
mod imaging_astap;
mod imaging_checks;
mod imaging_control;
mod imaging_fits;
mod imaging_focus_aid;
mod imaging_focus_arithmetic;
mod imaging_focus_check;
mod imaging_focus_far_out;
mod imaging_focus_reading;
mod imaging_focus_record;
mod imaging_focus_region;
mod imaging_frames;
mod imaging_hints;
mod imaging_lease;
mod imaging_preview;
mod imaging_relation;
mod imaging_smoothing;
mod imaging_solve;
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
mod stack_accumulate;
mod stack_camera;
mod stack_colour;
mod stack_display;
mod stack_gate;
mod stack_loop;
mod stack_placement;
mod stack_reject;
mod star_detected;
mod targets;
