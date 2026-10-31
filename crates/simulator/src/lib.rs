//! Catalog-rendered finder camera with known optics and an injected virtual clock.
#![forbid(unsafe_code)]

pub mod camera;
pub mod clock;
mod noise;
pub mod optics;
pub mod render;
pub mod sky;

pub use camera::SimCamera;
pub use clock::VirtualClock;
pub use optics::{Optics, Simulator, TrueDirection, TrueDirections};
pub use sky::Site;
