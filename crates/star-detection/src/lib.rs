//! Star detection pipeline.
//!
//! This crate implements the front-end image preprocessing for the plate solver:
//! robust per-image noise estimation, a binning cascade for multi-scale
//! detection, the `Star` output type, sub-pixel centroiding, and
//! brightness/ordering.

pub mod binning;
pub mod centroid;
pub mod detect;
pub mod gray;
pub mod noise;
pub mod saturated;
pub mod star;

pub use detect::{detect_stars, detect_stars_with_noise_floor};
