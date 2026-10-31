//! Target pointability and push-to guidance on the console's clock.

use math_core::{angular_distance, UnitVector};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use crate::catalog::{self, DeepSkyObject};
use crate::ephemeris::{self, AltitudeState, Position};
use crate::session::{Pointing, PointingSource, Site};
use crate::sky::{self, Frame, Observer};
use crate::targets::Target;

/// Pointability, in Sun guard, horizon and minimum-altitude precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    /// The target centre is within 15° of the Sun, regardless of site or altitude.
    SunGuard,
    /// The target is below the geometric horizon.
    BelowHorizon,
    /// The target is above the horizon but below the session minimum altitude.
    TooLow,
    /// The target is at or above the minimum altitude and outside the Sun guard.
    Pointable,
    /// A site is needed to determine altitude.
    NoSite,
}

/// Class a target at the supplied console time (Unix seconds).
pub fn class(target: &Target, site: Option<Site>, min_alt_deg: f64, time: f64) -> Class {
    let jd = sky::julian_date(time);
    let (ra, dec) = sky::precess_from_j2000(target.ra_deg, target.dec_deg, jd);
    let (sun_ra, sun_dec) = ephemeris::sun(jd);
    if angular_distance(
        UnitVector::from_radec(ra.to_radians(), dec.to_radians()),
        UnitVector::from_radec(sun_ra.to_radians(), sun_dec.to_radians()),
    ) <= 15.0_f64.to_radians()
    {
        return Class::SunGuard;
    }
    let Some(site) = site else {
        return Class::NoSite;
    };
    let altitude = sky::alt_az(target.ra_deg, target.dec_deg, &observer(site, time)).0;
    if altitude < 0.0 {
        Class::BelowHorizon
    } else if altitude < min_alt_deg {
        Class::TooLow
    } else {
        Class::Pointable
    }
}

fn observer(site: Site, time: f64) -> Observer {
    Observer {
        lat: site.lat_deg,
        lon: site.lon_deg,
        time,
    }
}

/// Guidance derived together with a session snapshot. Event times are Unix seconds.
#[derive(Debug, Clone, Serialize)]
pub struct Guidance {
    /// Pointability now.
    pub class: Class,
    /// Next rising to the minimum altitude, for a target that is down.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rise: Option<f64>,
    /// Upper transit following that rising, or the next transit if it never rises.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transit: Option<f64>,
    /// Setting below the minimum altitude following that rising.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set: Option<f64>,
    /// True when the target never reaches the minimum altitude on this date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub never: Option<bool>,
    /// Azimuth motion in (−180°, 180°], clockwise positive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_deg: Option<f64>,
    /// Altitude motion in degrees, up positive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tilt_deg: Option<f64>,
    /// Target geometry on or off the latest pointing's frame, when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placement: Option<Placement>,
    /// Live pointing is more than five console seconds old; no turn or tilt is given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale: Option<bool>,
}

/// Read guidance without advancing the latest pointing's held horizontal coordinates.
/// Uploaded frames never go stale; only the target follows `time`.
pub fn guidance(
    target: &Target,
    site: Option<Site>,
    min_alt_deg: f64,
    time: f64,
    pointing: Option<&Pointing>,
) -> Guidance {
    let class = class(target, site, min_alt_deg, time);
    let mut result = Guidance {
        class,
        rise: None,
        transit: None,
        set: None,
        never: None,
        turn_deg: None,
        tilt_deg: None,
        placement: pointing.map(|p| placement(target, &p.frame)),
        stale: None,
    };
    if let Some(site) = site {
        let observer = observer(site, time);
        if matches!(class, Class::BelowHorizon | Class::TooLow) {
            let times = ephemeris::next_times(
                Position::Fixed {
                    ra_deg: target.ra_deg,
                    dec_deg: target.dec_deg,
                },
                &observer,
                min_alt_deg,
            );
            result.rise = times.rise;
            result.transit = Some(times.transit);
            result.set = times.set;
            result.never = (times.state == AltitudeState::NeverRises).then_some(true);
        }
        if class == Class::Pointable {
            if let Some((alt, az)) = pointing.and_then(|p| p.alt_deg.zip(p.az_deg)) {
                let target_alt_az = sky::alt_az(target.ra_deg, target.dec_deg, &observer);
                let (turn, tilt) = turn_tilt((alt, az), target_alt_az);
                result.turn_deg = Some(turn);
                result.tilt_deg = Some(tilt);
            }
        }
    }
    result
}

/// Apply live freshness regardless of the link's current connection state.
pub fn guidance_with_source(
    target: &Target,
    site: Option<Site>,
    min_alt_deg: f64,
    time: f64,
    pointing: Option<&Pointing>,
    source: PointingSource,
) -> Guidance {
    let mut result = guidance(target, site, min_alt_deg, time, pointing);
    if source == PointingSource::Live && pointing.is_some_and(|p| time - p.time > 5.0) {
        result.stale = Some(true);
        result.turn_deg = None;
        result.tilt_deg = None;
    }
    result
}

/// Turn and tilt between `(altitude, azimuth)` pairs in degrees.
/// Exactly −180° maps to +180°, so a half-turn is always clockwise.
pub fn turn_tilt(pointing: (f64, f64), target: (f64, f64)) -> (f64, f64) {
    let turn = (target.1 - pointing.1 + 180.0).rem_euclid(360.0) - 180.0;
    (
        if turn == -180.0 { 180.0 } else { turn },
        target.0 - pointing.0,
    )
}

/// A target's symbol overlapping the frame, or its direction off the frame.
#[derive(Debug, Clone)]
pub enum Placement {
    /// The same pixel geometry as a solve report's object mark.
    OnFrame {
        /// Horizontal pixel coordinate.
        x: f64,
        /// Vertical pixel coordinate.
        y: f64,
        /// Major and minor half-axes in pixels, as `radius_px` and `minor_px`.
        axes: [f64; 2],
        /// Major-axis angle, clockwise from up in degrees.
        angle: f64,
    },
    /// Direction to a target whose symbol does not overlap the frame.
    OffFrame {
        /// Degrees clockwise from up.
        direction_deg: f64,
        /// Angular distance from the frame centre in degrees.
        distance_deg: f64,
    },
}

impl Serialize for Placement {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::OnFrame { x, y, axes, angle } => {
                let mut value = serializer.serialize_struct("Placement", 5)?;
                value.serialize_field("on_frame", &true)?;
                value.serialize_field("x", x)?;
                value.serialize_field("y", y)?;
                value.serialize_field("axes", axes)?;
                value.serialize_field("angle", angle)?;
                value.end()
            }
            Self::OffFrame {
                direction_deg,
                distance_deg,
            } => {
                let mut value = serializer.serialize_struct("Placement", 3)?;
                value.serialize_field("on_frame", &false)?;
                value.serialize_field("direction_deg", direction_deg)?;
                value.serialize_field("distance_deg", distance_deg)?;
                value.end()
            }
        }
    }
}

/// Place one target using the solve report's object-symbol geometry.
pub fn placement(target: &Target, frame: &Frame) -> Placement {
    // Messier targets use the report's catalog entry, preserving its exact geometry.
    let object = target
        .messier
        .as_deref()
        .and_then(catalog::find)
        .cloned()
        .unwrap_or_else(|| DeepSkyObject {
            id: target.id.clone(),
            designation: Some(target.id.clone()),
            name: target.common_names.first().cloned(),
            code: target.code.clone(),
            kind: target.kind,
            family: target.family,
            ra_deg: target.ra_deg,
            dec_deg: target.dec_deg,
            constellation: crate::constellation::containing(target.ra_deg, target.dec_deg),
            major_arcmin: target.major_arcmin,
            minor_arcmin: target.minor_arcmin,
            pa_deg: target.pa_deg,
            mag: target.mag,
        });
    if let Some(mark) = crate::solve::object_mark(frame, &object) {
        return Placement::OnFrame {
            x: mark.x,
            y: mark.y,
            axes: [mark.radius_px, mark.minor_px],
            angle: mark.angle_deg,
        };
    }
    let vector = object.vector();
    let point = frame.project_pinhole(vector).unwrap_or_else(|| {
        // Behind the camera, move the vector into the front hemisphere while keeping
        // its transverse direction. This extends the nearby-object bearing to all sky.
        let front = UnitVector {
            x: vector.x + 2.0 * frame.center.x,
            y: vector.y + 2.0 * frame.center.y,
            z: vector.z + 2.0 * frame.center.z,
        }
        .normalize()
        .expect("a unit vector plus twice the centre is nonzero");
        frame
            .project_pinhole(front)
            .expect("the shifted vector is in front of the camera")
    });
    Placement::OffFrame {
        direction_deg: frame.direction_deg(point),
        distance_deg: angular_distance(frame.center, vector).to_degrees(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Site;
    use crate::targets;

    const TIME: f64 = 1_768_528_800.0; // 2026-01-16 02:00 UT
    const SITE: Site = Site {
        lat_deg: 40.0,
        lon_deg: -75.0,
    };

    fn read(id: &str, min: f64) -> Guidance {
        guidance(targets::find(id).unwrap(), Some(SITE), min, TIME, None)
    }

    #[test]
    fn january_evening_classes_and_sun_guard_precedence() {
        // Oracle: US5's January evening, rounded altitudes within 0.1°.
        for (id, expected, altitude) in [
            ("M42", Class::Pointable, 42.9),
            ("M8", Class::BelowHorizon, -66.6),
            ("M75", Class::SunGuard, 0.0),
        ] {
            let target = targets::find(id).unwrap();
            assert_eq!(read(id, 15.0).class, expected);
            let alt = sky::alt_az(
                target.ra_deg,
                target.dec_deg,
                &Observer {
                    lat: 40.0,
                    lon: -75.0,
                    time: TIME,
                },
            )
            .0;
            if id == "M75" {
                assert!(alt < 0.0);
            } else {
                assert!((alt - altitude).abs() < 0.1, "{id}: {alt}");
            }
        }
        let m75 = targets::find("M75").unwrap();
        let jd = sky::julian_date(TIME);
        let (ra, dec) = sky::precess_from_j2000(m75.ra_deg, m75.dec_deg, jd);
        let (sun_ra, sun_dec) = ephemeris::sun(jd);
        let separation = math_core::angular_distance(
            math_core::UnitVector::from_radec(ra.to_radians(), dec.to_radians()),
            math_core::UnitVector::from_radec(sun_ra.to_radians(), sun_dec.to_radians()),
        )
        .to_degrees();
        assert!((separation - 3.8).abs() < 0.1);
    }

    #[test]
    fn a_fifty_degree_minimum_makes_orion_too_low() {
        assert_eq!(read("M42", 50.0).class, Class::TooLow);
    }

    #[test]
    fn without_a_site_the_sun_guard_still_comes_first() {
        for (id, expected) in [("M75", Class::SunGuard), ("M42", Class::NoSite)] {
            let result = guidance(targets::find(id).unwrap(), None, 15.0, TIME, None);
            assert_eq!(result.class, expected);
            assert!(result.rise.is_none() && result.transit.is_none() && result.set.is_none());
            assert!(result.turn_deg.is_none() && result.tilt_deg.is_none());
        }
    }

    #[test]
    fn lagoon_next_rises_to_the_minimum_and_then_transits_and_sets() {
        let result = read("M8", 15.0);
        let (rise, transit, set) = (
            result.rise.unwrap(),
            result.transit.unwrap(),
            result.set.unwrap(),
        );
        assert!(TIME < rise && rise < transit && transit < set);
        assert_ne!(result.never, Some(true));
        let target = targets::find("M8").unwrap();
        let alt = |time| {
            sky::alt_az(
                target.ra_deg,
                target.dec_deg,
                &Observer {
                    lat: 40.0,
                    lon: -75.0,
                    time,
                },
            )
            .0
        };
        // Independent oracle: existing geometric sky::alt_az, 0.05° crossing tolerance.
        assert!((alt(rise) - 15.0).abs() < 0.05);
        assert!(alt(rise - 30.0) < 15.0 && alt(rise + 30.0) > 15.0);
        assert!((alt(set) - 15.0).abs() < 0.05);
        assert!(alt(set - 30.0) > 15.0 && alt(set + 30.0) < 15.0);
        assert!(alt(transit) > alt(transit - 60.0) && alt(transit) > alt(transit + 60.0));
    }

    #[test]
    fn lagoon_never_reaches_thirty_degrees_but_has_a_transit() {
        let result = read("M8", 30.0);
        assert_eq!(result.never, Some(true));
        assert!(result.rise.is_none() && result.set.is_none());
        let transit = result.transit.unwrap();
        assert!(transit > TIME);
        let target = targets::find("M8").unwrap();
        let altitude = sky::alt_az(
            target.ra_deg,
            target.dec_deg,
            &Observer {
                lat: 40.0,
                lon: -75.0,
                time: transit,
            },
        )
        .0;
        // Oracle: culmination = 90° - |latitude - declination of date|, about 25.6°.
        assert!((altitude - 25.6).abs() < 0.1);
    }

    #[test]
    fn across_north_is_clockwise_and_up() {
        let (turn, tilt) = turn_tilt((30.0, 350.0), (42.0, 24.0));
        assert!((turn - 34.0).abs() < 1e-9 && (tilt - 12.0).abs() < 1e-9);
    }

    #[test]
    fn the_other_way_is_anticlockwise_and_down() {
        let (turn, tilt) = turn_tilt((50.0, 20.0), (41.0, 350.0));
        assert!((turn + 30.0).abs() < 1e-9 && (tilt + 9.0).abs() < 1e-9);
    }

    #[test]
    fn both_exact_half_turns_are_clockwise() {
        assert_eq!(turn_tilt((0.0, 0.0), (0.0, 180.0)), (180.0, 0.0));
        assert_eq!(turn_tilt((0.0, 180.0), (0.0, 0.0)), (180.0, 0.0));
    }

    #[test]
    fn a_target_that_is_not_pointable_has_no_turn_or_tilt() {
        let frame = crate::sky::Frame::from_solution(&plate_solver::Solution {
            ra: Some(1.0),
            dec: Some(0.0),
            roll: Some(0.0),
            camera: Some(math_core::PinholeCamera::new(800.0, 600.0, 0.2)),
            ..Default::default()
        })
        .unwrap();
        let pointing = crate::session::Pointing {
            seq: 1,
            frame,
            ra_deg: 1.0_f64.to_degrees(),
            dec_deg: 0.0,
            alt_deg: Some(30.0),
            az_deg: Some(350.0),
            time: TIME,
        };
        for (id, min) in [("M8", 15.0), ("M42", 50.0), ("M75", 15.0)] {
            let result = guidance(
                targets::find(id).unwrap(),
                Some(SITE),
                min,
                TIME,
                Some(&pointing),
            );
            assert!(result.turn_deg.is_none() && result.tilt_deg.is_none());
        }
    }

    #[test]
    fn non_messier_symbols_overlap_the_frame_even_with_their_centres_outside() {
        // Oracle: a zero-roll pinhole frame, north up and east left. Place the
        // centre at x=-5 analytically; a 1° major axis still overlaps the frame.
        let frame = Frame::from_solution(&plate_solver::Solution {
            ra: Some(0.0),
            dec: Some(0.0),
            roll: Some(0.0),
            camera: Some(math_core::PinholeCamera::new(
                800.0,
                600.0,
                20_f64.to_radians(),
            )),
            ..Default::default()
        })
        .unwrap();
        let mut target = targets::find("NGC 7000").unwrap().clone();
        target.messier = None;
        target.ra_deg = (405.0 / 400.0 * 10_f64.to_radians().tan())
            .atan()
            .to_degrees();
        target.dec_deg = 0.0;
        target.major_arcmin = Some(60.0);
        target.minor_arcmin = Some(30.0);
        target.pa_deg = Some(25.0);
        let Placement::OnFrame { x, y, axes, angle } = placement(&target, &frame) else {
            panic!("the symbol must overlap the left edge");
        };
        assert!((x + 5.0).abs() < 1e-9);
        assert!((y - 300.0).abs() < 1e-9);
        assert!(x + axes[0] > 0.0);
        assert!((axes[0] / axes[1] - 2.0).abs() < 1e-9);
        assert!((angle - 335.0).abs() < 1e-9);
    }

    #[test]
    fn off_frame_bearings_extend_to_targets_behind_the_camera() {
        let frame = Frame::from_solution(&plate_solver::Solution {
            ra: Some(0.0),
            dec: Some(0.0),
            roll: Some(0.0),
            camera: Some(math_core::PinholeCamera::new(800.0, 600.0, 0.2)),
            ..Default::default()
        })
        .unwrap();
        let mut target = targets::find("NGC 7000").unwrap().clone();
        target.messier = None;
        target.major_arcmin = None;
        // Oracle: on the equator, positive RA is left (270°) at zero roll;
        // angular distance from RA=0 is the absolute longitude, 1e-9° tolerance.
        for (ra, expected) in [(60.0, 270.0), (120.0, 270.0), (240.0, 90.0)] {
            target.ra_deg = ra;
            target.dec_deg = 0.0;
            let Placement::OffFrame {
                direction_deg,
                distance_deg,
            } = placement(&target, &frame)
            else {
                panic!("the target must be off the frame");
            };
            assert!((direction_deg - expected).abs() < 1e-9);
            assert!((distance_deg - ra.min(360.0 - ra)).abs() < 1e-9);
        }
    }
}
