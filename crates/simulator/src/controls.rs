//! Telescope and service controls shared by all clones of a simulator.
use crate::{mount::SimMount, optics, sky, Site, VirtualClock};
use math_core::UnitVector;
use pattern_database::PatternDatabase;
use scope_node::service::NodeControls;
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

/// The catalog sky together with the running node's capture and response controls.
#[derive(Clone)]
pub struct Simulator {
    sky: optics::Simulator,
    pub(crate) controls: NodeControls,
    mount: Arc<Mutex<Option<SimMount>>>,
}
impl Simulator {
    pub fn new(db: Arc<PatternDatabase>, site: Site, clock: VirtualClock) -> Self {
        Self {
            sky: optics::Simulator::new(db, site, clock),
            controls: NodeControls::default(),
            mount: Arc::new(Mutex::new(None)),
        }
    }

    /// Connect a mount using this simulator's virtual clock. Its true direction
    /// is the eyepiece direction; the finder camera retains its known aim point.
    pub fn connect_mount(&self, mount: SimMount) {
        *self.mount.lock().unwrap() = Some(mount);
        self.follow_mount();
    }

    /// Disconnect the mount, leaving the telescope at its last true direction.
    pub fn disconnect_mount(&self) {
        self.follow_mount();
        *self.mount.lock().unwrap() = None;
    }

    /// Connected mount handle, shared with the telescope and its camera clones.
    pub fn mount(&self) -> Option<SimMount> {
        self.mount.lock().unwrap().clone()
    }

    // Refresh before every access to the existing optics API (including the
    // camera's pointing snapshot). No background task or sampling tick is needed.
    fn follow_mount(&self) {
        if let Some(mount) = self.mount() {
            let direction = mount.true_direction();
            let site = self.sky.state.lock().unwrap().site;
            let rotation =
                sky::camera_rotation(direction.alt, direction.az, site, self.sky.clock().now());
            let vector = sky::unrotate(
                rotation,
                UnitVector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            );
            let (ra, dec) = vector.to_radec();
            self.sky
                .point_eyepiece_at(ra.to_degrees(), dec.to_degrees());
        }
    }

    /// Existing camera-centred push control. With a mount connected, the push
    /// changes its true eyepiece direction and leaves both axis readings alone.
    pub fn push_to(&self, alt: f64, az: f64) {
        self.sky.push_to(alt, az);
        self.push_mount_to_eyepiece();
    }

    fn push_mount_to_eyepiece(&self) {
        if let Some(mount) = self.mount() {
            let eyepiece = self.sky.telescope().eyepiece;
            mount.push_to(eyepiece.alt, eyepiece.az);
        }
    }

    /// Centre a J2000 direction at the eyepiece by hand, preserving axis counts.
    pub fn point_eyepiece_at(&self, ra: f64, dec: f64) {
        self.follow_mount();
        self.sky.point_eyepiece_at(ra, dec);
        self.push_mount_to_eyepiece();
    }
    pub fn push_by(&self, turn: f64, tilt: f64) {
        let current = self.telescope().camera;
        self.push_to((current.alt + tilt).clamp(-90.0, 90.0), current.az + turn);
    }
    pub fn stop_frames(&self) {
        self.controls.stop_frames();
    }
    pub fn start_frames(&self) {
        self.controls.start_frames();
    }
    pub fn set_unresponsive(&self, value: bool) {
        self.controls.set_unresponsive(value);
    }
}
impl Deref for Simulator {
    type Target = optics::Simulator;
    fn deref(&self) -> &Self::Target {
        self.follow_mount();
        &self.sky
    }
}
