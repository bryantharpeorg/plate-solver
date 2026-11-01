//! Solar coordinates, altitude events and atmospheric refraction (Meeus,
//! *Astronomical Algorithms*, second edition, chapters 25, 15 and 16).
//!
//! All times are supplied by the caller. Event times are Unix seconds in UT;
//! [`sun`] takes a Julian date in dynamical time (TD/TT). Event altitudes are
//! geometric: any desired refraction or limb allowance belongs in the requested
//! altitude. Refraction is applied only by the explicit conversions below;
//! [`sky::alt_az`] remains geometric.

use crate::sky::{self, Observer};

const SECONDS_PER_DAY: f64 = 86_400.0;
const SIDEREAL_DAY: f64 = SECONDS_PER_DAY * 360.0 / 360.985_647_366_29;

/// Apparent altitude in degrees from a true (geometric) altitude in degrees.
///
/// Adds Saemundsson's refraction (Meeus eq. 16.4):
/// `R = 1.02 / tan(h + 10.3 / (h + 5.11))` arcminutes, with the tangent's
/// argument in degrees. Assumes the standard atmosphere, 1010 hPa and 10 °C,
/// so the pressure and temperature scale factor is one. Input must be finite
/// and in `[0, 90]` degrees.
pub fn apparent_from_true(altitude_deg: f64) -> f64 {
    let argument = (altitude_deg + 10.3 / (altitude_deg + 5.11)).to_radians();
    altitude_deg + 1.02 / argument.tan() / 60.0
}

/// True (geometric) altitude in degrees from an apparent altitude in degrees.
///
/// Subtracts Bennett's refraction (Meeus eq. 16.3):
/// `R = 1 / tan(h0 + 7.31 / (h0 + 4.4))` arcminutes, with the tangent's
/// argument in degrees. Assumes 1010 hPa and 10 °C. Input must be finite and
/// in `[0, 90]` degrees; an apparent altitude at the horizon gives a negative
/// true altitude. This is an approximate inverse of [`apparent_from_true`].
pub fn true_from_apparent(altitude_deg: f64) -> f64 {
    let argument = (altitude_deg + 7.31 / (altitude_deg + 4.4)).to_radians();
    altitude_deg - 1.0 / argument.tan() / 60.0
}

/// The Sun's apparent right ascension and declination, in degrees, of date.
///
/// `jd_tt` is the Julian date in TD/TT, as in Meeus example 25.a. Chapter 25's
/// low-accuracy elliptical series includes aberration and the principal nutation
/// correction, with a target accuracy of 0.01° during 1950–2100. This is a
/// geocentric position, with RA in `[0, 360)`.
pub fn sun(jd_tt: f64) -> (f64, f64) {
    let t = (jd_tt - sky::J2000) / 36_525.0;
    let mean_longitude = (280.46646 + (36_000.769_83 + 0.000_303_2 * t) * t).rem_euclid(360.0);
    let anomaly = (357.52911 + (35_999.050_29 - 0.000_153_7 * t) * t).to_radians();
    let center = (1.914602 - (0.004817 + 0.000014 * t) * t) * anomaly.sin()
        + (0.019993 - 0.000101 * t) * (2.0 * anomaly).sin()
        + 0.000289 * (3.0 * anomaly).sin();
    let omega = (125.04 - 1934.136 * t).to_radians();
    let longitude = (mean_longitude + center - 0.00569 - 0.00478 * omega.sin()).to_radians();
    // Mean obliquity (22.2), followed by ch. 25's apparent-place correction.
    let obliquity = (23.0
        + 26.0 / 60.0
        + (21.448 - (46.815 + (0.00059 - 0.001813 * t) * t) * t) / 3600.0
        + 0.00256 * omega.cos())
    .to_radians();
    let ra = (obliquity.cos() * longitude.sin())
        .atan2(longitude.cos())
        .to_degrees()
        .rem_euclid(360.0);
    let dec = (obliquity.sin() * longitude.sin()).asin().to_degrees();
    (ra, dec)
}

/// The equatorial position used for altitude events.
#[derive(Debug, Clone, Copy)]
pub enum Position {
    /// A fixed catalog position in J2000, precessed just as [`sky::alt_az`] does.
    Fixed {
        /// J2000 right ascension in degrees.
        ra_deg: f64,
        /// J2000 declination in degrees.
        dec_deg: f64,
    },
    /// Three daily positions of a moving body, interpolated by Meeus ch. 3/15.
    ///
    /// Coordinates are referred to the equator and equinox of date. Sidereal
    /// time is mean, as in [`sky::alt_az`]; apparent input coordinates introduce
    /// only the small nutation difference (as in example 15.a). RA is unwrapped
    /// across 0° before interpolation; successive RAs must differ by <180°.
    /// For next-day events the same quadratic extends into the following day;
    /// callers must supply samples accurate over the whole requested interval.
    Daily {
        /// Julian date at 0h TT of the middle sample.
        epoch_jd: f64,
        /// `(RA, Dec)` in degrees at `epoch_jd - 1`, `epoch_jd`, `epoch_jd + 1`.
        coordinates: [(f64, f64); 3],
        /// TT minus UT, in seconds, for the interpolation argument.
        delta_t_seconds: f64,
    },
}

impl Position {
    fn at(self, jd_ut: f64) -> (f64, f64) {
        match self {
            Self::Fixed { ra_deg, dec_deg } => sky::precess_from_j2000(ra_deg, dec_deg, jd_ut),
            Self::Daily {
                epoch_jd,
                coordinates: [previous, current, next],
                delta_t_seconds,
            } => {
                let n = jd_ut - epoch_jd + delta_t_seconds / SECONDS_PER_DAY;
                // Meeus eq. 3.3: y = y2 + n/2 * (a + b + n * (b - a)).
                let interpolate = |a: f64, b: f64| n * (a + b + n * (b - a)) / 2.0;
                let ra = current.0
                    + interpolate(
                        signed_degrees(current.0 - previous.0),
                        signed_degrees(next.0 - current.0),
                    );
                let dec = current.1 + interpolate(current.1 - previous.1, next.1 - current.1);
                (ra.rem_euclid(360.0), dec)
            }
        }
    }
}

/// Whether the position crosses the requested altitude on the UT date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AltitudeState {
    /// There is a rising and a setting.
    RisesAndSets,
    /// The position stays at or above the altitude, including lower culmination.
    AlwaysAbove,
    /// The position never rises above the altitude, even at upper culmination.
    NeverRises,
}

/// Altitude events in Unix seconds (UT), together with their altitude state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AltitudeTimes {
    /// Whether the position crosses the altitude or stays on one side of it.
    pub state: AltitudeState,
    /// Rising above the altitude, absent when there is no crossing in the interval.
    pub rise: Option<f64>,
    /// Upper meridian transit (the time the position is highest).
    pub transit: f64,
    /// Setting below the altitude, absent when there is no crossing in the interval.
    pub set: Option<f64>,
}

fn signed_degrees(angle: f64) -> f64 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}

fn midnight(time: f64) -> f64 {
    time.div_euclid(SECONDS_PER_DAY) * SECONDS_PER_DAY
}

#[derive(Clone, Copy)]
enum Event {
    Transit,
    Crossing,
}

fn corrected_fraction(
    position: Position,
    site: &Observer,
    jd0: f64,
    altitude_deg: f64,
    mut m: f64,
    event: Event,
) -> f64 {
    for _ in 0..3 {
        let jd = jd0 + m;
        let (ra, dec) = position.at(jd);
        let hour_angle = signed_degrees(sky::gmst_deg(jd) + site.lon - ra);
        m += match event {
            Event::Transit => -hour_angle / 360.0,
            Event::Crossing => {
                let (lat, dec, h) = (
                    site.lat.to_radians(),
                    dec.to_radians(),
                    hour_angle.to_radians(),
                );
                let altitude = (lat.sin() * dec.sin() + lat.cos() * dec.cos() * h.cos())
                    .clamp(-1.0, 1.0)
                    .asin()
                    .to_degrees();
                (altitude - altitude_deg) / (360.0 * dec.cos() * lat.cos() * h.sin())
            }
        };
    }
    m
}

struct DateEvents {
    state: AltitudeState,
    transits: [f64; 3],
    rises: Option<[f64; 3]>,
    sets: Option<[f64; 3]>,
}

fn date_events(position: Position, site: &Observer, altitude_deg: f64) -> DateEvents {
    let start = midnight(site.time);
    let jd0 = sky::julian_date(start);
    let (ra, dec) = position.at(jd0);
    let m0 = ((ra - site.lon - sky::gmst_deg(jd0)) / 360.0).rem_euclid(1.0);
    let correct = |m, event| {
        // Correct adjacent rotations too: a UT day can contain two crossings.
        // Never wrap a corrected fraction: changing the time by one solar day
        // would move it away from the crossing by nearly four minutes.
        [-1.0, 0.0, 1.0].map(|offset| {
            let m = corrected_fraction(position, site, jd0, altitude_deg, m + offset, event);
            start + m * SECONDS_PER_DAY
        })
    };
    let transits = correct(m0, Event::Transit);
    let (lat, dec) = (site.lat.to_radians(), dec.to_radians());
    let cos_h = (altitude_deg.to_radians().sin() - lat.sin() * dec.sin()) / (lat.cos() * dec.cos());
    let state = if cos_h <= -1.0 {
        AltitudeState::AlwaysAbove
    } else if cos_h >= 1.0 {
        AltitudeState::NeverRises
    } else {
        AltitudeState::RisesAndSets
    };
    let (rises, sets) = if state == AltitudeState::RisesAndSets {
        // The half-angle form avoids arccos, as required by the numerical contract.
        let h0 = 2.0 * ((1.0 - cos_h) / 2.0).sqrt().asin().to_degrees();
        (
            Some(correct((m0 - h0 / 360.0).rem_euclid(1.0), Event::Crossing)),
            Some(correct((m0 + h0 / 360.0).rem_euclid(1.0), Event::Crossing)),
        )
    } else {
        (None, None)
    };
    DateEvents {
        state,
        rises,
        transits,
        sets,
    }
}

/// Rising, upper transit and setting on the UT calendar date of `site.time`.
///
/// Implements Meeus ch. 15's hour-angle estimate followed by three corrections
/// for each event, using [`sky::gmst_deg`]. The events need not be in
/// rising/transit/setting order; if two events of a kind occur on the date, the
/// first is returned. The transit is given even when the position stays above
/// or below the altitude. A moving body's correction may carry a nominal event
/// outside the date if there is no such event on that UT date. Inputs must be
/// finite, with physical latitudes, declinations and altitudes in `[-90, 90]`.
pub fn on_date(position: Position, site: &Observer, altitude_deg: f64) -> AltitudeTimes {
    let events = date_events(position, site, altitude_deg);
    let start = midnight(site.time);
    let on_day = |times: [f64; 3]| {
        times
            .into_iter()
            .filter(|time| (start..start + SECONDS_PER_DAY).contains(time))
            .min_by(f64::total_cmp)
            .unwrap_or(times[1])
    };
    AltitudeTimes {
        state: events.state,
        rise: events.rises.map(on_day),
        transit: on_day(events.transits),
        set: events.sets.map(on_day),
    }
}

fn first_at_or_after(times: impl Iterator<Item = f64>, after: f64) -> Option<f64> {
    times.filter(|&time| time >= after).min_by(f64::total_cmp)
}

/// The next rising at or after `site.time` within 24 sidereal hours, followed by
/// the first upper transit and setting after that rising.
///
/// Searches today's and tomorrow's UT events. Transit and setting may belong to
/// the following UT date. For a position always above or never rising, returns
/// that state with the next transit and no crossings. If a moving position has
/// no rising in the search interval, crossings are absent and transit is the
/// next transit after `site.time`.
pub fn next_times(position: Position, site: &Observer, altitude_deg: f64) -> AltitudeTimes {
    let today = date_events(position, site, altitude_deg);
    let tomorrow_site = Observer {
        time: midnight(site.time) + SECONDS_PER_DAY,
        ..*site
    };
    let tomorrow = date_events(position, &tomorrow_site, altitude_deg);
    let transit = first_at_or_after(
        today.transits.into_iter().chain(tomorrow.transits),
        site.time,
    )
    .expect("an upper transit occurs in the adjacent rotations");
    let rise = today
        .rises
        .into_iter()
        .chain(tomorrow.rises)
        .flatten()
        .filter(|&time| time >= site.time && time <= site.time + SIDEREAL_DAY)
        .min_by(f64::total_cmp);
    let Some(rise) = rise else {
        return AltitudeTimes {
            state: today.state,
            rise: None,
            transit,
            set: None,
        };
    };
    let rising_site = Observer {
        time: rise,
        ..*site
    };
    let rising_date = date_events(position, &rising_site, altitude_deg);
    let following_date = date_events(
        position,
        &Observer {
            time: midnight(rise) + SECONDS_PER_DAY,
            ..*site
        },
        altitude_deg,
    );
    AltitudeTimes {
        state: AltitudeState::RisesAndSets,
        rise: Some(rise),
        transit: first_at_or_after(
            rising_date
                .transits
                .into_iter()
                .chain(following_date.transits),
            rise,
        )
        .expect("upper transit follows rising"),
        set: first_at_or_after(
            rising_date
                .sets
                .into_iter()
                .chain(following_date.sets)
                .flatten(),
            rise,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sky::{self, Observer};

    const DAY: f64 = 86_400.0;

    fn unix(jd: f64) -> f64 {
        (jd - 2_440_587.5) * DAY
    }

    fn site(time: f64) -> Observer {
        Observer {
            lat: 40.0,
            lon: -75.0,
            time,
        }
    }

    fn assert_crossing(ra: f64, dec: f64, time: f64, altitude: f64, rising: bool) {
        let at = sky::alt_az(ra, dec, &site(time)).0;
        let before = sky::alt_az(ra, dec, &site(time - 30.0)).0;
        let after = sky::alt_az(ra, dec, &site(time + 30.0)).0;
        // Independent oracle: sky's existing geometric altitude, tolerance 0.05°.
        assert!((at - altitude).abs() < 0.05, "altitude {at} at {time}");
        if rising {
            assert!(before < altitude && after > altitude, "{before} -> {after}");
        } else {
            assert!(before > altitude && after < altitude, "{before} -> {after}");
        }
    }

    #[test]
    fn bennett_at_apparent_horizon_matches_meeus() {
        // Meeus ch. 16, eq. 16.3: 34.5 arcmin at apparent 0°, tolerance 0.1 arcmin.
        let refraction_arcmin = (0.0 - true_from_apparent(0.0)) * 60.0;
        assert!(
            (refraction_arcmin - 34.5).abs() <= 0.1,
            "Bennett refraction {refraction_arcmin} arcmin"
        );
    }

    #[test]
    fn saemundsson_at_true_forty_five_degrees_matches_meeus() {
        // Meeus ch. 16, eq. 16.4: 1.01 arcmin at true 45°, tolerance 0.01 arcmin.
        let refraction_arcmin = (apparent_from_true(45.0) - 45.0) * 60.0;
        assert!(
            (refraction_arcmin - 1.01).abs() <= 0.01,
            "Saemundsson refraction {refraction_arcmin} arcmin"
        );
    }

    #[test]
    fn refraction_round_trip_from_horizon_to_zenith() {
        // Meeus 16.3/16.4 are approximate inverses: recover the supplied true
        // altitude within 0.1 arcmin, including both endpoints, in 0.1° steps.
        for step in 0..=900 {
            let true_deg = f64::from(step) / 10.0;
            let apparent_deg = apparent_from_true(true_deg);
            let recovered_deg = true_from_apparent(apparent_deg);
            let error_arcmin = (recovered_deg - true_deg).abs() * 60.0;
            assert!(
                error_arcmin <= 0.1,
                "true {true_deg}°, apparent {apparent_deg}°, error {error_arcmin} arcmin"
            );
        }
    }

    #[test]
    fn sky_altitude_stays_geometric_for_meeus_example_13b() {
        // Meeus 13.b: Venus, 1987 April 10 19:21 UT, Washington. Convert the
        // example's place of date to J2000 for sky::alt_az, allowing 0.01° for
        // its mean sidereal time rather than the example's apparent time.
        let observer = Observer {
            lat: 38.921_389,
            lon: -77.065_556,
            time: unix(2_446_896.306_25),
        };
        let place_of_date = (347.319_337_5, -6.719_892);
        let mut j2000 = place_of_date;
        for _ in 0..10 {
            let precessed =
                sky::precess_from_j2000(j2000.0, j2000.1, sky::julian_date(observer.time));
            j2000.0 += place_of_date.0 - precessed.0;
            j2000.1 += place_of_date.1 - precessed.1;
        }
        let altitude_deg = sky::alt_az(j2000.0, j2000.1, &observer).0;
        assert!(
            (altitude_deg - 15.1249).abs() < 0.01,
            "geometric altitude {altitude_deg}°"
        );
    }

    #[test]
    fn sun_matches_meeus_example_25a() {
        // Meeus 25.a: 1992 October 13.0 TD, apparent equatorial coordinates.
        let (ra, dec) = sun(2_448_908.5);
        assert!((ra - 198.38083).abs() < 0.01, "RA {ra}");
        assert!((dec - -7.78507).abs() < 0.01, "Dec {dec}");
    }

    #[test]
    fn venus_matches_meeus_example_15a() {
        // Meeus's three 0h TD positions, March 19, 20 and 21, 1988; ΔT = 56 s.
        let jd = 2_447_240.5;
        let venus = Position::Daily {
            epoch_jd: jd,
            coordinates: [
                (40.68021, 18.04761),
                (41.73129, 18.44092),
                (42.78204, 18.82742),
            ],
            delta_t_seconds: 56.0,
        };
        let boston = Observer {
            lat: 42.3333,
            lon: -71.0833,
            time: unix(jd),
        };
        let times = on_date(venus, &boston, -0.5667);
        assert_eq!(times.state, AltitudeState::RisesAndSets);
        // Corrected m1=0.51766, m0=0.81980, m2=0.12130; tolerance one minute.
        for (actual, expected) in [
            (times.rise.unwrap(), 12.0 * 3600.0 + 25.0 * 60.0 + 26.0),
            (times.transit, 19.0 * 3600.0 + 40.0 * 60.0 + 31.0),
            (times.set.unwrap(), 2.0 * 3600.0 + 54.0 * 60.0 + 40.0),
        ] {
            assert!((actual - unix(jd) - expected).abs() < 60.0, "{times:?}");
            assert!((unix(jd)..unix(jd + 1.0)).contains(&actual));
        }
        // Date-bound results need not be in rising/transit/setting order.
        assert!(times.set.unwrap() < times.rise.unwrap());
    }

    #[test]
    fn circumpolar_position_stays_above_fifteen_degrees_and_has_a_transit() {
        let at = site(unix(2_461_056.5));
        let position = Position::Fixed {
            ra_deg: 120.0,
            dec_deg: 80.0,
        };
        let times = on_date(position, &at, 15.0);
        // Geometry: at 40° N a Dec +80° position has a lower culmination of 30°.
        assert_eq!(times.state, AltitudeState::AlwaysAbove);
        assert_eq!(times.rise, None);
        assert_eq!(times.set, None);
        assert!((at.time..at.time + DAY).contains(&times.transit));
        let (alt, _) = sky::alt_az(120.0, 80.0, &site(times.transit));
        assert!((alt - 50.0).abs() < 0.1, "upper culmination {alt}");
    }

    #[test]
    fn southern_position_never_reaches_the_horizon_and_has_a_transit() {
        let at = site(unix(2_461_056.5));
        let position = Position::Fixed {
            ra_deg: 120.0,
            dec_deg: -60.0,
        };
        let times = on_date(position, &at, 0.0);
        // Geometry: the upper culmination is 90° - |40° - (-60°)| = -10°.
        assert_eq!(times.state, AltitudeState::NeverRises);
        assert_eq!(times.rise, None);
        assert_eq!(times.set, None);
        assert!((at.time..at.time + DAY).contains(&times.transit));
        let (alt, _) = sky::alt_az(120.0, -60.0, &site(times.transit));
        assert!((alt - -10.0).abs() < 0.1, "upper culmination {alt}");
    }

    #[test]
    fn fixed_position_rising_agrees_with_sky_altitude() {
        let position = Position::Fixed {
            ra_deg: 85.0,
            dec_deg: -4.0,
        };
        let times = on_date(position, &site(unix(2_461_056.5)), 15.0);
        assert_eq!(times.state, AltitudeState::RisesAndSets);
        assert_crossing(85.0, -4.0, times.rise.unwrap(), 15.0, true);
    }

    #[test]
    fn m8_next_times_follow_the_rising_across_ut_midnight() {
        // OpenNGC M8 coordinates from the existing catalog; 2026-01-16 02:00 UT.
        let m8 = crate::catalog::find("M8").unwrap();
        let now = unix(2_461_056.5) + 2.0 * 3600.0;
        let times = next_times(
            Position::Fixed {
                ra_deg: m8.ra_deg,
                dec_deg: m8.dec_deg,
            },
            &site(now),
            15.0,
        );
        let (rise, set) = (times.rise.unwrap(), times.set.unwrap());
        assert_eq!(times.state, AltitudeState::RisesAndSets);
        assert!(now < rise && rise < times.transit && times.transit < set);
        assert!(rise <= now + 86_164.090_5);
        assert_crossing(m8.ra_deg, m8.dec_deg, rise, 15.0, true);
        assert_crossing(m8.ra_deg, m8.dec_deg, set, 15.0, false);
    }

    #[test]
    fn daily_ra_interpolation_unwraps_zero_and_uses_delta_t() {
        let position = Position::Daily {
            epoch_jd: sky::J2000,
            coordinates: [(358.0, 10.0), (359.0, 12.0), (0.0, 16.0)],
            delta_t_seconds: 60.0,
        };
        // Worked polynomial: RA = 359+n; Dec = 12+3n+n². With ΔT=60 s,
        // this UT instant is n=0.5 TT. Tolerance 1e-8°, including JD rounding.
        let (ra, dec) = position.at(sky::J2000 + 0.5 - 60.0 / DAY);
        assert!((ra - 359.5).abs() < 1e-8, "RA {ra}");
        assert!((dec - 13.75).abs() < 1e-8, "Dec {dec}");
        let (ra, dec) = position.at(sky::J2000 + 1.5 - 60.0 / DAY);
        assert!((ra - 0.5).abs() < 1e-8 && (dec - 18.75).abs() < 1e-8);
    }

    #[test]
    fn next_non_crossing_states_return_the_next_upper_transit() {
        for (dec, altitude, state) in [
            (80.0, 15.0, AltitudeState::AlwaysAbove),
            (-60.0, 0.0, AltitudeState::NeverRises),
        ] {
            let position = Position::Fixed {
                ra_deg: 120.0,
                dec_deg: dec,
            };
            let at = site(unix(2_461_056.5));
            let todays_transit = on_date(position, &at, altitude).transit;
            for now in [todays_transit - 60.0, todays_transit, todays_transit + 60.0] {
                let next = next_times(position, &site(now), altitude);
                assert_eq!(next.state, state);
                assert_eq!((next.rise, next.set), (None, None));
                assert!(next.transit >= now && next.transit < now + DAY);
                let before = sky::alt_az(120.0, dec, &site(next.transit - 60.0)).0;
                let at_transit = sky::alt_az(120.0, dec, &site(next.transit)).0;
                let after = sky::alt_az(120.0, dec, &site(next.transit + 60.0)).0;
                assert!(at_transit > before && at_transit > after);
            }
        }
    }

    #[test]
    fn next_rising_is_inclusive_and_rolls_forward_when_already_up() {
        let position = Position::Fixed {
            ra_deg: 85.0,
            dec_deg: -4.0,
        };
        let first = on_date(position, &site(unix(2_461_056.5)), 15.0)
            .rise
            .unwrap();
        let inclusive = next_times(position, &site(first), 15.0);
        assert_eq!(inclusive.rise, Some(first));
        let next = next_times(position, &site(first + 60.0), 15.0);
        let (rise, set) = (next.rise.unwrap(), next.set.unwrap());
        assert!(rise > first + 60.0 && rise <= first + 60.0 + 86_164.090_5);
        assert!(rise < next.transit && next.transit < set);
        assert_crossing(85.0, -4.0, rise, 15.0, true);
        assert_crossing(85.0, -4.0, set, 15.0, false);
    }

    #[test]
    fn date_events_and_next_sequence_remain_crossings_near_midnight() {
        // Sweep independent catalog directions across UT midnight. Earth turns
        // slightly more than 360° per UT day, so a day can have two risings.
        let start = unix(2_461_056.5);
        for ra in (0..720).map(|i| f64::from(i) / 2.0) {
            let position = Position::Fixed {
                ra_deg: ra,
                dec_deg: -4.0,
            };
            let day = on_date(position, &site(start), 15.0);
            for (event, rising) in [(day.rise.unwrap(), true), (day.set.unwrap(), false)] {
                assert!((start..start + DAY).contains(&event));
                assert_crossing(ra, -4.0, event, 15.0, rising);
            }
            for now in [start + 60.0, start + 23.0 * 3600.0 + 59.0 * 60.0] {
                let times = next_times(position, &site(now), 15.0);
                let (rise, set) = (times.rise.unwrap(), times.set.unwrap());
                assert!(rise >= now && rise <= now + 86_164.090_5);
                assert!(rise < times.transit && times.transit < set);
                assert_crossing(ra, -4.0, rise, 15.0, true);
                assert_crossing(ra, -4.0, set, 15.0, false);
            }
        }
    }
}
