//! Known finder optics and shared telescope state. Public angles are degrees.
use crate::{
    clock::VirtualClock,
    imaging::{ImagingOptics, ImagingTruth, LightPath, Scene, SensorPattern},
    render::{self, RenderStar},
    sky::{self, Site},
};
use math_core::{attitude::RotationMatrix, PinholeCamera, UnitVector};
use pattern_database::PatternDatabase;
use std::sync::{Arc, Mutex};

/// Full-frame pixels use (x, y); the horizontal field is in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Optics {
    pub width: u32,
    pub height: u32,
    pub fov_deg: f64,
    pub eyepiece_px: (f64, f64),
}

impl Default for Optics {
    fn default() -> Self {
        Self {
            width: 1456,
            height: 1088,
            fov_deg: 12.0,
            eyepiece_px: (700.0, 420.0),
        }
    }
}

impl Optics {
    pub fn pinhole(self) -> PinholeCamera {
        assert!(self.width > 0 && self.height > 0);
        assert!(self.fov_deg > 0.0 && self.fov_deg < 180.0);
        PinholeCamera::new(
            self.width.into(),
            self.height.into(),
            self.fov_deg.to_radians(),
        )
    }

    fn eyepiece_ray(self) -> UnitVector {
        let (x, y) = self.eyepiece_px;
        assert!(x >= 0.0 && x < f64::from(self.width) && y >= 0.0 && y < f64::from(self.height));
        self.pinhole().unproject(&[(y, x)])[0].expect("finite eyepiece pixel")
    }
}

/// Ground truth, independent of detection or solving. RA/Dec are J2000 degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrueDirection {
    pub alt: f64,
    pub az: f64,
    pub ra: f64,
    pub dec: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrueDirections {
    pub camera: TrueDirection,
    pub eyepiece: TrueDirection,
}

#[derive(Clone, Copy)]
pub(crate) struct Pointing {
    pub optics: Optics,
    pub imaging: ImagingOptics,
    pub site: Site,
    pub alt: f64,
    pub az: f64,
}

/// Cloneable control handle shared with the camera. Alt/az denotes the camera centre.
#[derive(Clone)]
pub struct Simulator {
    pub(crate) db: Arc<PatternDatabase>,
    pub(crate) state: Arc<Mutex<Pointing>>,
    pub(crate) imaging_bench: Arc<Mutex<(SensorPattern, LightPath)>>,
    pub(crate) imaging_scene: Arc<Mutex<Scene>>,
    clock: VirtualClock,
}

impl Simulator {
    pub fn new(db: Arc<PatternDatabase>, site: Site, clock: VirtualClock) -> Self {
        assert!(site.lat_deg.is_finite() && site.lat_deg.abs() <= 90.0 && site.lon_deg.is_finite());
        Self {
            db,
            state: Arc::new(Mutex::new(Pointing {
                optics: Optics::default(),
                imaging: ImagingOptics::default(),
                site,
                alt: 45.0,
                az: 180.0,
            })),
            imaging_bench: Arc::new(Mutex::new((SensorPattern::default(), LightPath::default()))),
            imaging_scene: Arc::new(Mutex::new(Scene::default())),
            clock,
        }
    }

    pub fn clock(&self) -> &VirtualClock {
        &self.clock
    }

    pub fn optics(&self) -> Optics {
        self.state.lock().unwrap().optics
    }

    pub fn set_optics(&self, optics: Optics) {
        optics.eyepiece_ray();
        self.state.lock().unwrap().optics = optics;
    }

    pub fn push_to(&self, alt: f64, az: f64) {
        assert!(alt.is_finite() && alt.abs() <= 90.0 && az.is_finite());
        let mut state = self.state.lock().unwrap();
        state.alt = alt;
        state.az = az.rem_euclid(360.0);
    }

    pub fn telescope(&self) -> TrueDirections {
        self.directions_at(self.clock.now())
    }

    /// Truth at the supplied exposure instant, rather than the later read time.
    pub fn directions_at(&self, unix_s: f64) -> TrueDirections {
        let p = *self.state.lock().unwrap();
        let rotation = sky::camera_rotation(p.alt, p.az, p.site, unix_s);
        let direction = |v: UnitVector| {
            let (ra, dec) = v.to_radec();
            let (alt, az) = sky::horizontal(v, p.site, unix_s);
            TrueDirection {
                alt,
                az,
                ra: ra.to_degrees(),
                dec: dec.to_degrees(),
            }
        };
        let mut camera = direction(sky::unrotate(
            rotation,
            UnitVector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        ));
        camera.alt = p.alt;
        camera.az = p.az;
        TrueDirections {
            camera,
            eyepiece: direction(sky::unrotate(rotation, p.optics.eyepiece_ray())),
        }
    }

    /// The imaging optics in force: the sensor, its field, the imaging angle,
    /// the focus error and the pixel size.
    pub fn imaging_optics(&self) -> ImagingOptics {
        self.state.lock().unwrap().imaging
    }

    /// What else is in the simulated sky.
    pub fn scene(&self) -> Scene {
        self.imaging_scene.lock().unwrap().clone()
    }

    /// Asserts every number finite, every `electrons`, `electrons_per_s` and colour not
    /// negative, and every `sigma_arcsec` above 0.
    pub fn set_scene(&self, scene: Scene) {
        assert!(scene.satellites.iter().all(|satellite| {
            satellite.from.0.is_finite()
                && satellite.from.1.is_finite()
                && satellite.to.0.is_finite()
                && satellite.to.1.is_finite()
                && satellite.electrons.is_finite()
                && satellite.electrons >= 0.0
        }));
        assert!(scene.sources.iter().all(|source| {
            source.ra_deg.is_finite()
                && source.dec_deg.is_finite()
                && source.sigma_arcsec.is_finite()
                && source.sigma_arcsec > 0.0
                && source.electrons_per_s.is_finite()
                && source.electrons_per_s >= 0.0
                && source.colour.iter().all(|c| c.is_finite() && *c >= 0.0)
        }));
        *self.imaging_scene.lock().unwrap() = scene;
    }

    /// The sensor's own charge and column pattern.
    pub fn sensor_pattern(&self) -> SensorPattern {
        self.imaging_bench.lock().unwrap().0.clone()
    }

    /// Set the sensor pattern. Every rate and the amplitude must be finite and not negative.
    pub fn set_sensor_pattern(&self, pattern: SensorPattern) {
        assert!(pattern.column_amplitude.is_finite() && pattern.column_amplitude >= 0.0);
        assert!(pattern
            .hot_pixels
            .iter()
            .all(|hot| hot.electrons_per_s.is_finite() && hot.electrons_per_s >= 0.0));
        self.imaging_bench.lock().unwrap().0 = pattern;
    }

    /// The cover, panel and light lost on the way to the sensor.
    pub fn light_path(&self) -> LightPath {
        self.imaging_bench.lock().unwrap().1.clone()
    }

    /// Set the light path. Panics unless 0 < corner light ≤ 1, depths are from 0 to 1,
    /// radii are positive, and panel rates and colours are finite and not negative.
    pub fn set_light_path(&self, path: LightPath) {
        assert!(path.corner_light > 0.0 && path.corner_light <= 1.0);
        assert!(path.dust.iter().all(|shadow| {
            shadow.radius_px.is_finite()
                && shadow.radius_px > 0.0
                && (0.0..=1.0).contains(&shadow.depth)
        }));
        if let Some(panel) = path.flat_panel {
            assert!(panel.electrons_per_s.is_finite() && panel.electrons_per_s >= 0.0);
            assert!(panel.colour.iter().all(|c| c.is_finite() && *c >= 0.0));
        }
        self.imaging_bench.lock().unwrap().1 = path;
    }

    /// Set the imaging optics. The sensor must have pixels and a field.
    pub fn set_imaging_optics(&self, optics: ImagingOptics) {
        assert!(optics.width > 0 && optics.height > 0);
        assert!(optics.fov_deg > 0.0 && optics.fov_deg < 180.0);
        self.state.lock().unwrap().imaging = optics;
    }

    /// The focus error of the imaging optics: 0 is best focus, and its sign
    /// does not change a star's size.
    pub fn focus_error(&self) -> f64 {
        self.state.lock().unwrap().imaging.focus_error
    }

    /// Turn the simulated focuser. Panics on a value that is not finite.
    pub fn set_focus_error(&self, error: f64) {
        assert!(error.is_finite());
        self.state.lock().unwrap().imaging.focus_error = error;
    }

    /// Where the imaging frame points now, on the clock's reading.
    pub fn imaging_truth(&self) -> ImagingTruth {
        self.imaging_truth_at(self.clock.now())
    }

    /// Where the imaging frame points at the supplied instant.
    pub fn imaging_truth_at(&self, unix_s: f64) -> ImagingTruth {
        let (rotation, optics) = self.imaging_rotation_at(unix_s);
        imaging_truth_of(&rotation, &optics)
    }

    /// The imaging frame's axes in J2000, with the optics in force.
    pub(crate) fn imaging_rotation_at(&self, unix_s: f64) -> (RotationMatrix, ImagingOptics) {
        let p = *self.state.lock().unwrap();
        let rotation = sky::camera_rotation(p.alt, p.az, p.site, unix_s);
        let boresight = sky::unrotate(rotation, p.optics.eyepiece_ray());
        let up = unit(rotation.rows[2]);
        let along = up.x * boresight.x + up.y * boresight.y + up.z * boresight.z;
        // Frame-up, less its part along the boresight, and the left it turns
        // toward as the imaging angle grows.
        let flat_up = unit([
            up.x - along * boresight.x,
            up.y - along * boresight.y,
            up.z - along * boresight.z,
        ]);
        let flat_left = cross(flat_up, boresight);
        let angle = p.imaging.angle_deg.to_radians();
        let (sin, cos) = angle.sin_cos();
        let up = [
            cos * flat_up.x + sin * flat_left.x,
            cos * flat_up.y + sin * flat_left.y,
            cos * flat_up.z + sin * flat_left.z,
        ];
        let left = [
            cos * flat_left.x - sin * flat_up.x,
            cos * flat_left.y - sin * flat_up.y,
            cos * flat_left.z - sin * flat_up.z,
        ];
        (
            RotationMatrix {
                rows: [[boresight.x, boresight.y, boresight.z], left, up],
            },
            p.imaging,
        )
    }

    /// Place a J2000 direction at the eyepiece pixel, accounting for the tube's roll.
    /// Like alt-az pointing itself, this is singular at the zenith: a fixed lateral
    /// offset cannot reach the tiny polar cap inside that offset.
    pub fn point_eyepiece_at(&self, ra: f64, dec: f64) {
        assert!(ra.is_finite() && dec.is_finite() && dec.abs() <= 90.0);
        let mut p = self.state.lock().unwrap();
        let (alt, az) = sky::horizontal(
            UnitVector::from_radec(ra.to_radians(), dec.to_radians()),
            p.site,
            self.clock.now(),
        );
        let ray = p.optics.eyepiece_ray();
        let radius = ray.x.hypot(ray.z);
        let up = alt.to_radians().sin();
        assert!(
            up.abs() <= radius,
            "eyepiece direction is inside the zenith singularity"
        );
        let camera_alt = (up / radius).asin() - ray.z.atan2(ray.x);
        let along = ray.x * camera_alt.cos() - ray.z * camera_alt.sin();
        p.alt = camera_alt.to_degrees();
        p.az = (az + ray.y.atan2(along).to_degrees()).rem_euclid(360.0);
    }

    /// Database indices and projected PSFs in brightness order at the current time.
    pub fn field_stars(&self, exposure_ms: f64, gain: f64) -> Vec<(usize, RenderStar)> {
        let p = *self.state.lock().unwrap();
        render::field_stars(
            &self.db,
            p.optics,
            sky::camera_rotation(p.alt, p.az, p.site, self.clock.now()),
            exposure_ms,
            gain,
        )
    }
}

/// A unit vector from its components.
fn unit(components: [f64; 3]) -> UnitVector {
    UnitVector {
        x: components[0],
        y: components[1],
        z: components[2],
    }
    .normalize()
    .expect("a nonzero direction")
}

/// The cross product of two unit vectors.
fn cross(a: UnitVector, b: UnitVector) -> UnitVector {
    UnitVector {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

/// The truth of an imaging frame's pointing, from its axes and its optics: the
/// boresight's J2000 place, the angle of frame-up east of celestial north and
/// the scale in arcseconds a pixel.
fn imaging_truth_of(rotation: &RotationMatrix, optics: &ImagingOptics) -> ImagingTruth {
    let boresight = unit(rotation.rows[0]);
    let up = unit(rotation.rows[2]);
    let (ra, dec) = boresight.to_radec();
    // Celestial north and east as the frame sees them: the parts of the pole
    // perpendicular to the boresight, and the pole's cross with it.
    let north = unit([
        -boresight.z * boresight.x,
        -boresight.z * boresight.y,
        1.0 - boresight.z * boresight.z,
    ]);
    let east = cross(
        UnitVector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        boresight,
    )
    .normalize()
    .expect("a boresight off the pole");
    let angle = (up.x * east.x + up.y * east.y + up.z * east.z)
        .atan2(up.x * north.x + up.y * north.y + up.z * north.z)
        .to_degrees()
        .rem_euclid(360.0);
    let radians = 2.0 * (optics.fov_deg.to_radians() / 2.0).tan() / f64::from(optics.width);
    ImagingTruth {
        ra_deg: ra.to_degrees(),
        dec_deg: dec.to_degrees(),
        up_east_of_north_deg: angle,
        arcsec_per_px: radians * 180.0 / std::f64::consts::PI * 3600.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_optics_defaults_and_independent_controls() {
        let mut optics = Optics::default();
        assert_eq!((optics.width, optics.height), (1456, 1088));
        assert_eq!(optics.fov_deg, 12.0);
        assert_eq!(optics.eyepiece_px, (700.0, 420.0));
        optics.width = 800;
        optics.height = 600;
        optics.fov_deg = 20.0;
        optics.eyepiece_px = (350.0, 240.0);
        assert_eq!(optics.pinhole().width, 800.0);
        assert_eq!(optics.pinhole().height, 600.0);
        assert_eq!(optics.pinhole().fov, 20_f64.to_radians());
        assert_eq!(optics.eyepiece_px, (350.0, 240.0));
    }
}
