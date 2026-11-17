//! Independent oracles: Meeus ch. 13/14 worked values in the tracking research
//! notes, section 3, plus explicitly worked sensor geometry and smear limits.

use finder::ephemeris::{
    altaz_rates, altaz_rates_from_alt_az, corner_radius_px, longest_exposure, Exposure,
};

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} +/- {tolerance}, got {actual}"
    );
}

#[test]
fn transit_rates_match_meeus_worked_values() {
    let rates = altaz_rates(40.0, 20.0, 0.0);
    assert_close(rates.alt_arcsec_per_s, 0.0, 0.01);
    assert_close(rates.az_arcsec_per_s, 41.33, 0.02);
    // Positive means that the parallactic angle increases, including south transit.
    assert_close(rates.field_rotation_deg_per_h, 33.69, 0.02);
}

#[test]
fn before_transit_rates_match_meeus_worked_values() {
    let rates = altaz_rates(40.0, 20.0, -30.0);
    assert_close(rates.alt_arcsec_per_s, 10.07, 0.02);
    assert_close(rates.az_arcsec_per_s, 18.45, 0.02);
}

#[test]
fn imaging_camera_corner_and_exposure_match_worked_example() {
    // Half the sensor diagonal, in micrometres, divided by the pixel pitch.
    // The research note rounds this to the nearest pixel: 3,751 px.
    assert_close(corner_radius_px(23.5, 15.6, 3.76), 3751.0, 0.5);
    let rotation = altaz_rates(40.0, 20.0, 0.0)
        .field_rotation_deg_per_h
        .to_radians()
        / 3600.0;
    let Exposure::Seconds(seconds) = longest_exposure(23.5, 15.6, 3.76, rotation, 1.0) else {
        panic!("nonzero field rotation must limit the exposure");
    };
    assert_close(seconds, 1.63, 0.01);
}

#[test]
fn due_east_and_west_have_no_field_rotation_limit() {
    for azimuth in [90.0, 270.0] {
        let rates = altaz_rates_from_alt_az(40.0, 30.0, azimuth);
        let rotation = rates.field_rotation_deg_per_h.to_radians() / 3600.0;
        // Meeus: dq/dt is proportional to cos(A), zero at both azimuths.
        assert_close(rotation, 0.0, 1e-9);
        // These are moving targets: only field rotation vanishes.
        let expected_alt_rate = if azimuth == 90.0 { 11.52 } else { -11.52 };
        assert_close(rates.alt_arcsec_per_s, expected_alt_rate, 0.02);
        assert_close(rates.az_arcsec_per_s, 9.67, 0.02);
        assert_eq!(
            longest_exposure(23.5, 15.6, 3.76, rotation, 1.0),
            Exposure::NoLimit
        );
    }
}

#[test]
fn horizontal_transit_and_northern_transit_preserve_rate_signs() {
    // The same worked transit is at h=70, A=180 in horizontal coordinates.
    let southern = altaz_rates_from_alt_az(40.0, 70.0, 180.0);
    assert_close(southern.alt_arcsec_per_s, 0.0, 0.01);
    assert_close(southern.az_arcsec_per_s, 41.33, 0.02);
    assert_close(southern.field_rotation_deg_per_h, 33.69, 0.02);

    // For delta=60 at this site, transit is instead at h=70, A=0.
    // Meeus: dA/dt = omega*cos(delta)/sin(phi-delta) = -21.99 arcsec/s;
    // dq/dt changes sign with cos(A), giving -33.69 deg/h.
    let northern = altaz_rates(40.0, 60.0, 0.0);
    assert_close(northern.alt_arcsec_per_s, 0.0, 0.01);
    assert_close(northern.az_arcsec_per_s, -21.99, 0.02);
    assert_close(northern.field_rotation_deg_per_h, -33.69, 0.02);
}

#[test]
fn exposure_uses_rotation_magnitude_smear_and_strict_zero_threshold() {
    // A 6 x 8 mm sensor at 10 um pitch has a 500 px corner radius.
    assert_close(corner_radius_px(6.0, 8.0, 10.0), 500.0, 1e-12);
    for rotation in [0.001, -0.001] {
        // 2 px / (500 px * 0.001 rad/s) = 4 seconds, either sign.
        assert_eq!(
            longest_exposure(6.0, 8.0, 10.0, rotation, 2.0),
            Exposure::Seconds(4.0)
        );
    }
    for rotation in [0.0, 0.999e-9, -0.999e-9] {
        assert_eq!(
            longest_exposure(6.0, 8.0, 10.0, rotation, 1.0),
            Exposure::NoLimit
        );
    }
    for rotation in [1e-9, -1e-9] {
        let Exposure::Seconds(seconds) = longest_exposure(6.0, 8.0, 10.0, rotation, 1.0) else {
            panic!("the threshold itself is a finite exposure");
        };
        assert_close(seconds, 2_000_000.0, 1e-9);
    }
}
