//! Geometric horizon/equatorial transforms (Meeus ch. 12, 13 and 21).
//! No refraction or nutation: these invert the console's mean-place sky maths.
use math_core::{attitude::RotationMatrix, UnitVector};

/// Latitude north positive, longitude east positive, in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Site {
    pub lat_deg: f64,
    pub lon_deg: f64,
}

impl Default for Site {
    fn default() -> Self {
        Self {
            lat_deg: 40.0,
            lon_deg: -75.0,
        }
    }
}

pub fn julian_date(unix_s: f64) -> f64 {
    unix_s / 86_400.0 + 2_440_587.5
}

/// Meeus eq. 12.4, in radians; mean sidereal time at Greenwich.
fn gmst(jd: f64) -> f64 {
    let d = jd - 2_451_545.0;
    let t = d / 36_525.0;
    (280.460_618_37 + 360.985_647_366_29 * d + 0.000_387_933 * t * t - t * t * t / 38_710_000.0)
        .rem_euclid(360.0)
        .to_radians()
}

/// Horizon components (north, east, up) to the mean equator of date.
fn horizon_to_date(n: f64, e: f64, u: f64, site: Site, unix_s: f64) -> UnitVector {
    let (sl, cl) = site.lat_deg.to_radians().sin_cos();
    let (st, ct) = (gmst(julian_date(unix_s)) + site.lon_deg.to_radians()).sin_cos();
    UnitVector {
        x: (u * cl - n * sl) * ct - e * st,
        y: (u * cl - n * sl) * st + e * ct,
        z: n * cl + u * sl,
    }
}

/// Altitude and north-through-east azimuth in degrees to RA/Dec of date.
pub fn horizontal_to_date(alt: f64, az: f64, site: Site, unix_s: f64) -> UnitVector {
    let (sh, ch) = alt.to_radians().sin_cos();
    let (sa, ca) = az.to_radians().sin_cos();
    horizon_to_date(ch * ca, ch * sa, sh, site, unix_s)
}

fn precession_angles(jd: f64) -> (f64, f64, f64) {
    let t = (jd - 2_451_545.0) / 36_525.0;
    let arcsec = |v: f64| (v / 3600.0).to_radians();
    (
        arcsec((2306.2181 + (0.30188 + 0.017998 * t) * t) * t),
        arcsec((2306.2181 + (1.09468 + 0.018203 * t) * t) * t),
        arcsec((2004.3109 - (0.42665 + 0.041833 * t) * t) * t),
    )
}

fn around_z(v: UnitVector, angle: f64) -> UnitVector {
    let (s, c) = angle.sin_cos();
    UnitVector {
        x: c * v.x - s * v.y,
        y: s * v.x + c * v.y,
        z: v.z,
    }
}

fn around_y(v: UnitVector, angle: f64) -> UnitVector {
    let (s, c) = angle.sin_cos();
    UnitVector {
        x: c * v.x + s * v.z,
        y: v.y,
        z: -s * v.x + c * v.z,
    }
}

/// Inverse IAU 1976 precession, applying the inverse rotations in reverse order.
pub fn to_j2000(date: UnitVector, jd: f64) -> UnitVector {
    let (zeta, z, theta) = precession_angles(jd);
    around_z(around_y(around_z(date, -z), theta), -zeta)
}

pub fn from_j2000(v: UnitVector, jd: f64) -> UnitVector {
    let (zeta, z, theta) = precession_angles(jd);
    around_z(around_y(around_z(v, zeta), -theta), z)
}

/// J2000 direction to geometric altitude and azimuth, in degrees.
pub fn horizontal(v: UnitVector, site: Site, unix_s: f64) -> (f64, f64) {
    let v = from_j2000(v, julian_date(unix_s));
    let (sl, cl) = site.lat_deg.to_radians().sin_cos();
    let (st, ct) = (gmst(julian_date(unix_s)) + site.lon_deg.to_radians()).sin_cos();
    let meridian = v.x * ct + v.y * st;
    let north = -sl * meridian + cl * v.z;
    let east = -st * v.x + ct * v.y;
    let up = cl * meridian + sl * v.z;
    (
        up.atan2(north.hypot(east)).to_degrees(),
        east.atan2(north).to_degrees().rem_euclid(360.0),
    )
}

/// Sky-to-camera attitude with the camera fixed to the tube: top follows increasing
/// altitude, left follows decreasing azimuth. Transforming all three axes back to
/// J2000 includes both parallactic roll and the precession of celestial north.
pub fn camera_rotation(alt: f64, az: f64, site: Site, unix_s: f64) -> RotationMatrix {
    let (sh, ch) = alt.to_radians().sin_cos();
    let (sa, ca) = az.to_radians().sin_cos();
    let axes = [
        (ch * ca, ch * sa, sh),
        (sa, -ca, 0.0),
        (-sh * ca, -sh * sa, ch),
    ];
    RotationMatrix {
        rows: axes.map(|(n, e, u)| {
            let v = to_j2000(horizon_to_date(n, e, u, site, unix_s), julian_date(unix_s));
            [v.x, v.y, v.z]
        }),
    }
}

/// Inverse attitude, used for the eyepiece ray; no alternate projection model.
pub fn unrotate(rotation: RotationMatrix, v: UnitVector) -> UnitVector {
    RotationMatrix {
        rows: [0, 1, 2].map(|i| {
            [
                rotation.rows[0][i],
                rotation.rows[1][i],
                rotation.rows[2][i],
            ]
        }),
    }
    .rotate(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_alt_az_matches_meeus_13b() {
        // Venus, US Naval Observatory, 1987 April 10 19:21 UT. Meeus uses
        // apparent sidereal time; allow 0.01 degree for nutation, as the console does.
        let site = Site {
            lat_deg: 38.921_389,
            lon_deg: -77.065_556,
        };
        let time = (2_446_896.306_25 - 2_440_587.5) * 86_400.0;
        let date = horizontal_to_date(15.1249, 248.0337, site, time);
        let expected =
            UnitVector::from_radec(347.319_337_5_f64.to_radians(), -6.719_892_f64.to_radians());
        assert!(math_core::angular_distance(date, expected).to_degrees() < 0.01);
        // Independent precession example 21.b, inverted (including proper motion).
        let j2000 = to_j2000(
            UnitVector::from_radec(41.547_214_f64.to_radians(), 49.348_483_f64.to_radians()),
            2_462_088.69,
        );
        let expected =
            UnitVector::from_radec(41.054_063_f64.to_radians(), 49.227_750_f64.to_radians());
        assert!(math_core::angular_distance(j2000, expected).to_degrees() < 2e-5);
    }

    #[test]
    fn tube_axes_stay_orthonormal_and_follow_the_horizon_across_sites_and_dates() {
        for site in [
            Site::default(),
            Site {
                lat_deg: -33.0,
                lon_deg: 151.0,
            },
        ] {
            for time in [0.0, 1_768_528_800.0] {
                for (alt, az) in [(0.0, 0.0), (50.0, 120.0), (89.0, 359.9)] {
                    let rotation = camera_rotation(alt, az, site, time);
                    assert!((rotation.det() - 1.0).abs() < 1e-12);
                    let centre = unrotate(
                        rotation,
                        UnitVector {
                            x: 1.0,
                            y: 0.0,
                            z: 0.0,
                        },
                    );
                    let (h, a) = horizontal(centre, site, time);
                    assert!((h - alt).abs() < 1e-10);
                    assert!(((a - az + 180.0).rem_euclid(360.0) - 180.0).abs() < 1e-10);
                    // A 0.1-degree tilt is upward in the camera at every field roll.
                    let above = to_j2000(
                        horizontal_to_date(alt + 0.1, az, site, time),
                        julian_date(time),
                    );
                    let local = rotation.rotate(above);
                    assert!(local.y.abs() < 1e-12);
                    assert!((local.z - 0.1_f64.to_radians().sin()).abs() < 1e-12);
                    assert!((centre.norm() - 1.0).abs() < 1e-12);
                }
            }
        }
    }
}
