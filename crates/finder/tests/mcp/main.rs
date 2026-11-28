//! Tests of the MCP door and its tools, in process.

#[allow(dead_code, unused_imports)]
#[path = "../support/mod.rs"]
mod support;

mod axis_moves;
mod calibration;
mod databases;
mod endpoint;
mod finder_focus;
mod frames;
mod imaging;
mod imaging_align;
mod imaging_focus;
mod imaging_preview_frames;
mod imaging_run;
mod imaging_solve;
mod motion;
mod nudge;
mod pairing;
mod push_tool;
mod session_rotation;
mod session_tools;
mod stack;
mod steering;
mod target_words;
mod tracking;
