//! Camera port, adapters and cedar star detection for the scope node.
#![forbid(unsafe_code)]

pub mod camera;
pub mod capture;
pub mod clock;
pub mod config;
pub mod detect;
pub mod exposure;
pub mod mount;
pub mod preview;
pub mod rpicam;
pub mod service;
pub mod synthetic;

pub use camera::{Camera, CameraError, CameraInfo, GrayFrame, RangeError, Settings};
