//! Catalog-rendered finder camera with known optics and an injected virtual clock.
#![forbid(unsafe_code)]

pub mod camera;
pub mod clock;
pub mod controls;
pub mod mount;
pub mod node;
mod noise;
pub mod optics;
pub mod render;
pub mod sky;

pub use camera::SimCamera;
pub use clock::VirtualClock;
pub use controls::Simulator;
pub use node::SimNode;
pub use optics::{Optics, TrueDirection, TrueDirections};
pub use sky::Site;
