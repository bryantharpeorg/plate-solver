//! Sky geometry: a solved frame's sky-to-pixel projection, precession, sidereal time,
//! altitude and azimuth, and coordinate formatting.

use math_core::attitude::RotationMatrix;
use math_core::{distort_centroids, PinholeCamera, UnitVector};
use plate_solver::Solution;
use serde::{Deserialize, Serialize};

/// Julian date of the J2000.0 epoch.
pub const J2000: f64 = 2_451_545.0;

/// The rotation from the celestial frame to the camera frame for a pointing, in radians.
///
/// The inverse of [`math_core::attitude::extract_radec_roll`]: row 0 is the boresight, and
/// at zero roll rows 1 and 2 point east and north. The camera's `j` axis grows toward the
/// left of the image and `k` toward the top, so at zero roll north is up and east is left.
pub fn rotation_from_radec_roll(ra: f64, dec: f64, roll: f64) -> RotationMatrix {
    let (sin_ra, cos_ra) = ra.sin_cos();
    let (sin_dec, cos_dec) = dec.sin_cos();
    let (sin_roll, cos_roll) = roll.sin_cos();
    let east = [-sin_ra, cos_ra, 0.0];
    let north = [-sin_dec * cos_ra, -sin_dec * sin_ra, cos_dec];
    let mix = |a: f64, b: f64| [0, 1, 2].map(|i| a * east[i] + b * north[i]);
    RotationMatrix {
        rows: [
            [cos_ra * cos_dec, sin_ra * cos_dec, sin_dec],
            mix(cos_roll, sin_roll),
            mix(-sin_roll, cos_roll),
        ],
    }
}

/// A solved frame: where each sky direction lands in the image.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    rotation: RotationMatrix,
    camera: PinholeCamera,
    distortion: f64,
    /// The boresight: the sky direction at the image center.
    pub center: UnitVector,
}

// Keep serialization at the console boundary; math-core stays independent of serde.
impl Serialize for Frame {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Camera {
            width: f64,
            height: f64,
            fov: f64,
        }
        #[derive(Serialize)]
        struct View {
            rotation: [[f64; 3]; 3],
            camera: Camera,
            distortion: f64,
            center: [f64; 3],
        }
        View {
            rotation: self.rotation.rows,
            camera: Camera {
                width: self.camera.width,
                height: self.camera.height,
                fov: self.camera.fov,
            },
            distortion: self.distortion,
            center: [self.center.x, self.center.y, self.center.z],
        }
        .serialize(serializer)
    }
}

impl Frame {
    /// The frame of a solution, or `None` when it did not solve.
    pub fn from_solution(solution: &Solution) -> Option<Self> {
        let (ra, dec, roll) = (solution.ra?, solution.dec?, solution.roll?);
        Some(Self {
            rotation: rotation_from_radec_roll(ra, dec, roll),
            camera: solution.camera?,
            distortion: solution.distortion.unwrap_or(0.0),
            center: UnitVector::from_radec(ra, dec),
        })
    }

    /// Image width and height in pixels.
    pub fn size(&self) -> (f64, f64) {
        (self.camera.width, self.camera.height)
    }

    /// Image position `(x, y)` of a sky direction on the pinhole model, before lens
    /// distortion; `None` behind the camera. It holds outside the image too, so it gives the
    /// direction to an off-frame target.
    pub fn project_pinhole(&self, v: UnitVector) -> Option<(f64, f64)> {
        let (pixels, _) = self.camera.project(&[self.rotation.rotate(v)]);
        let (y, x) = pixels[0];
        (x.is_finite() && y.is_finite()).then_some((x, y))
    }

    /// Image position `(x, y)` of a sky direction, lens distortion included. Meant for
    /// directions in or near the image: far outside it the distortion model does not hold.
    pub fn project(&self, v: UnitVector) -> Option<(f64, f64)> {
        let (x, y) = self.project_pinhole(v)?;
        let (w, h) = self.size();
        let (y, x) = distort_centroids(&[(y, x)], w, h, self.distortion, None, None)[0];
        Some((x, y))
    }

    /// Sky direction of a full-frame pixel, undoing lens distortion and attitude.
    pub fn unproject(&self, (x, y): (f64, f64)) -> Option<UnitVector> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let pixels = math_core::undistort_centroids(
            &[(y, x)],
            self.camera.width,
            self.camera.height,
            self.distortion,
        );
        let v = self.camera.unproject(&pixels)[0]?;
        let r = self.rotation.rows;
        UnitVector {
            x: r[0][0] * v.x + r[1][0] * v.y + r[2][0] * v.z,
            y: r[0][1] * v.x + r[1][1] * v.y + r[2][1] * v.z,
            z: r[0][2] * v.x + r[1][2] * v.y + r[2][2] * v.z,
        }
        .normalize()
    }

    /// Whether `(x, y)` lies inside the image grown by `margin` pixels on every side.
    pub fn contains(&self, (x, y): (f64, f64), margin: f64) -> bool {
        let (w, h) = self.size();
        x > -margin && x < w + margin && y > -margin && y < h + margin
    }

    /// Horizontal field of view in degrees.
    pub fn fov_deg(&self) -> f64 {
        self.camera.fov.to_degrees()
    }

    /// Vertical field of view in radians (the horizontal one is the camera's `fov`).
    pub fn fov_vertical(&self) -> f64 {
        let (w, h) = self.size();
        2.0 * ((self.camera.fov / 2.0).tan() * h / w).atan()
    }

    /// Angle from the image center to a corner, in radians.
    pub fn half_diagonal(&self) -> f64 {
        let (w, h) = self.size();
        ((self.camera.fov / 2.0).tan() * w.hypot(h) / w).atan()
    }

    /// Angular size of one pixel at the image center, in arcseconds.
    pub fn arcsec_per_px(&self) -> f64 {
        let rad = (2.0 * (self.camera.fov / 2.0).tan() / self.camera.width).atan();
        rad.to_degrees() * 3600.0
    }

    /// Image direction of `(x, y)` seen from the center, in degrees clockwise from up.
    pub fn direction_deg(&self, (x, y): (f64, f64)) -> f64 {
        let (w, h) = self.size();
        (x - w / 2.0)
            .atan2(h / 2.0 - y)
            .to_degrees()
            .rem_euclid(360.0)
    }

    /// Where celestial north points in the image at the center, degrees clockwise from up.
    pub fn north_deg(&self) -> f64 {
        self.north_at(self.center).unwrap_or(0.0)
    }

    /// Where celestial north points in the image at sky direction `v`, degrees clockwise
    /// from up. North turns across a wide frame, so objects use their own.
    pub fn north_at(&self, v: UnitVector) -> Option<f64> {
        let (ra, dec) = v.to_radec();
        let toward_pole = UnitVector::from_radec(ra, dec + 0.1_f64.to_radians());
        let (x0, y0) = self.project_pinhole(v)?;
        let (x1, y1) = self.project_pinhole(toward_pole)?;
        Some((x1 - x0).atan2(y0 - y1).to_degrees().rem_euclid(360.0))
    }
}

/// Precess a J2000 position, in degrees, to the mean equator and equinox of Julian date
/// `jd` (IAU 1976 angles; Meeus, *Astronomical Algorithms*, ch. 21).
pub fn precess_from_j2000(ra_deg: f64, dec_deg: f64, jd: f64) -> (f64, f64) {
    let t = (jd - J2000) / 36_525.0;
    let arcsec = |a: f64| (a / 3600.0).to_radians();
    let zeta = arcsec((2306.2181 + (0.30188 + 0.017998 * t) * t) * t);
    let z = arcsec((2306.2181 + (1.09468 + 0.018203 * t) * t) * t);
    let theta = arcsec((2004.3109 - (0.42665 + 0.041833 * t) * t) * t);
    let (ra, dec) = (ra_deg.to_radians() + zeta, dec_deg.to_radians());
    let a = dec.cos() * ra.sin();
    let b = theta.cos() * dec.cos() * ra.cos() - theta.sin() * dec.sin();
    let c = theta.sin() * dec.cos() * ra.cos() + theta.cos() * dec.sin();
    let ra_out = (a.atan2(b) + z).to_degrees().rem_euclid(360.0);
    let dec_out = c.atan2(a.hypot(b)).to_degrees();
    (ra_out, dec_out)
}

/// Julian date of a Unix time in seconds.
pub fn julian_date(unix_seconds: f64) -> f64 {
    unix_seconds / 86_400.0 + 2_440_587.5
}

/// Greenwich mean sidereal time in degrees, in `[0, 360)` (Meeus eq. 12.4).
pub fn gmst_deg(jd: f64) -> f64 {
    let d = jd - J2000;
    let t = d / 36_525.0;
    let gmst =
        280.460_618_37 + 360.985_647_366_29 * d + 0.000_387_933 * t * t - t * t * t / 38_710_000.0;
    gmst.rem_euclid(360.0)
}

/// Where and when the sky is seen from.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Observer {
    /// Latitude in degrees, north positive.
    pub lat: f64,
    /// Longitude in degrees, east positive.
    pub lon: f64,
    /// Unix time in seconds.
    pub time: f64,
}

/// Altitude and azimuth in degrees of a J2000 position, azimuth from north through east.
///
/// Precessed to the date; mean sidereal time, no nutation or refraction. That is within a
/// few hundredths of a degree above the horizon, plenty to tell what is up.
pub fn alt_az(ra_deg: f64, dec_deg: f64, observer: &Observer) -> (f64, f64) {
    let jd = julian_date(observer.time);
    let (ra, dec) = precess_from_j2000(ra_deg, dec_deg, jd);
    let hour_angle = (gmst_deg(jd) + observer.lon - ra).to_radians();
    let (dec, lat) = (dec.to_radians(), observer.lat.to_radians());
    let alt = (lat.sin() * dec.sin() + lat.cos() * dec.cos() * hour_angle.cos()).asin();
    let az = (-hour_angle.sin() * dec.cos())
        .atan2(lat.cos() * dec.sin() - lat.sin() * dec.cos() * hour_angle.cos());
    (alt.to_degrees(), az.to_degrees().rem_euclid(360.0))
}

/// The 16-point compass name of an azimuth in degrees, as `"SSE"`.
pub fn compass_point(az_deg: f64) -> &'static str {
    const POINTS: [&str; 16] = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];
    POINTS[(az_deg.rem_euclid(360.0) / 22.5).round() as usize % 16]
}

/// Right ascension in degrees as `05h 40m 46s`.
pub fn format_ra(ra_deg: f64) -> String {
    let seconds = (ra_deg.rem_euclid(360.0) / 15.0 * 3600.0).round() as i64 % 86_400;
    format!(
        "{:02}h {:02}m {:02}s",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

/// Declination in degrees as `−04° 14′ 46″`, with a true minus sign.
pub fn format_dec(dec_deg: f64) -> String {
    let seconds = (dec_deg.abs() * 3600.0).round() as i64;
    let sign = if dec_deg < 0.0 && seconds > 0 {
        '\u{2212}'
    } else {
        '+'
    };
    format!(
        "{sign}{:02}° {:02}′ {:02}″",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use math_core::attitude::extract_radec_roll;
    use std::f64::consts::TAU;

    fn wrap(a: f64) -> f64 {
        (a + TAU / 2.0).rem_euclid(TAU) - TAU / 2.0
    }

    #[test]
    fn rotation_inverts_extract_radec_roll() {
        for (ra, dec, roll) in [
            (1.4868, -0.0741, 1.0533),
            (0.1, 1.2, 5.9),
            (4.0, -1.1, 0.0),
            (6.0, 0.4, 3.1),
        ] {
            let r = rotation_from_radec_roll(ra, dec, roll);
            assert!((r.det() - 1.0).abs() < 1e-12, "not a rotation");
            let (ra2, dec2, roll2) = extract_radec_roll(&r);
            assert!(wrap(ra2 - ra).abs() < 1e-12, "ra {ra} -> {ra2}");
            assert!((dec2 - dec).abs() < 1e-12, "dec {dec} -> {dec2}");
            assert!(wrap(roll2 - roll).abs() < 1e-12, "roll {roll} -> {roll2}");
        }
    }

    fn frame(roll_deg: f64) -> Frame {
        Frame {
            rotation: rotation_from_radec_roll(1.0, 0.3, roll_deg.to_radians()),
            camera: PinholeCamera::new(800.0, 600.0, 10_f64.to_radians()),
            distortion: 0.0,
            center: UnitVector::from_radec(1.0, 0.3),
        }
    }

    #[test]
    fn zero_roll_puts_north_up_and_east_left() {
        let f = frame(0.0);
        let (cx, cy) = (400.0, 300.0);
        let north = f.project(UnitVector::from_radec(1.0, 0.31)).unwrap();
        assert!(
            (north.0 - cx).abs() < 1e-6 && north.1 < cy,
            "north at {north:?}"
        );
        let east = f.project(UnitVector::from_radec(1.01, 0.3)).unwrap();
        assert!(east.0 < cx, "east at {east:?}");
        assert!(f.north_deg().abs() < 1e-6 || (f.north_deg() - 360.0).abs() < 1e-6);
    }

    #[test]
    fn roll_turns_north_clockwise_from_up() {
        // Positive roll mixes north into the camera's left axis, turning north counter-
        // clockwise in the image; north then sits at 360° minus the roll.
        let north = frame(30.0).north_deg();
        assert!((north - 330.0).abs() < 0.01, "north at {north}°");
    }

    #[test]
    fn center_projects_to_image_center_and_behind_is_none() {
        let f = frame(10.0);
        let c = f.project(f.center).unwrap();
        assert!((c.0 - 400.0).abs() < 1e-9 && (c.1 - 300.0).abs() < 1e-9);
        let (ra, dec) = f.center.to_radec();
        let opposite = UnitVector::from_radec(ra + TAU / 2.0, -dec);
        assert!(f.project(opposite).is_none());
        assert!(
            f.contains(c, 0.0) && !f.contains((-5.0, 10.0), 0.0) && f.contains((-5.0, 10.0), 6.0)
        );
    }

    #[test]
    fn field_geometry() {
        let f = frame(0.0);
        let horizontal = 10_f64.to_radians();
        assert!(f.fov_vertical() < horizontal && f.half_diagonal() > horizontal / 2.0);
        // 10° over 800 px averages 45″ a pixel; the pinhole's center pixels are a little
        // larger: 2·tan(5°)/800 rad.
        let center = (2.0 * 5_f64.to_radians().tan() / 800.0).to_degrees() * 3600.0;
        assert!((f.arcsec_per_px() - center).abs() < 1e-5 && center > 45.0);
        assert!((f.direction_deg((500.0, 300.0)) - 90.0).abs() < 1e-9);
        assert!((f.direction_deg((400.0, 400.0)) - 180.0).abs() < 1e-9);
    }

    #[test]
    fn precession_matches_meeus_example_21b() {
        // θ Persei to 2028 Nov 13.19 TD (JD 2462088.69), from J2000 after proper motion.
        let (ra, dec) = precess_from_j2000(41.054_063, 49.227_750, 2_462_088.69);
        assert!((ra - 41.547_214).abs() < 2e-5, "ra {ra}");
        assert!((dec - 49.348_483).abs() < 2e-5, "dec {dec}");
    }

    #[test]
    fn sidereal_time_matches_meeus_example_12a() {
        // 1987 April 10, 0h UT: 13h 10m 46.3668s.
        let expected = (13.0 + 10.0 / 60.0 + 46.3668 / 3600.0) * 15.0;
        assert!((gmst_deg(2_446_895.5) - expected).abs() < 1e-5);
        assert!((julian_date(0.0) - 2_440_587.5).abs() < 1e-12);
    }

    #[test]
    fn alt_az_matches_meeus_example_13b() {
        // Venus from the US Naval Observatory, 1987 April 10 19:21 UT: Meeus gives
        // h = 15.1249°, A = 68.0337° from south, i.e. 248.0337° from north. Meeus uses
        // apparent sidereal time and the apparent place of the date, so feed this function
        // the J2000 place that precesses to it and allow for nutation.
        let observer = Observer {
            lat: 38.921_389,
            lon: -77.065_556,
            time: (2_446_896.306_25 - 2_440_587.5) * 86_400.0,
        };
        let (ra_date, dec_date) = (347.319_337_5, -6.719_892);
        let (ra, dec) = undo_precession(ra_date, dec_date, julian_date(observer.time));
        let (alt, az) = alt_az(ra, dec, &observer);
        assert!((alt - 15.1249).abs() < 0.01, "alt {alt}");
        assert!((az - 248.0337).abs() < 0.01, "az {az}");
    }

    /// The J2000 position that precesses to `(ra, dec)` at `jd`, by fixed-point iteration.
    fn undo_precession(ra: f64, dec: f64, jd: f64) -> (f64, f64) {
        let mut guess = (ra, dec);
        for _ in 0..10 {
            let (r, d) = precess_from_j2000(guess.0, guess.1, jd);
            guess = (guess.0 + (ra - r), guess.1 + (dec - d));
        }
        guess
    }

    #[test]
    fn compass_and_formatting() {
        assert_eq!(compass_point(0.0), "N");
        assert_eq!(compass_point(159.5), "SSE");
        assert_eq!(compass_point(359.0), "N");
        assert_eq!(compass_point(-90.0), "W");
        assert_eq!(format_ra(85.190_964), "05h 40m 46s");
        assert_eq!(format_ra(359.999_99), "00h 00m 00s");
        assert_eq!(format_dec(-4.246_105), "\u{2212}04° 14′ 46″");
        assert_eq!(format_dec(89.264), "+89° 15′ 50″");
        assert_eq!(format_dec(-0.000_01), "+00° 00′ 00″");
    }
}
