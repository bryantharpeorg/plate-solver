//! 035 US2-S3 to S7. Oracles: the specified exact maps and the shared setup's
//! independent vector projection. Tolerances are in pixels unless stated otherwise.

use finder::imaging::{
    solver::ImagingSolve,
    stack::{Placement, CORNER_SLACK_PX},
};

#[derive(Clone, Copy)]
struct Field {
    ra: f64,
    dec: f64,
    up: f64,
    scale: f64,
    size: (u32, u32),
}

impl Field {
    fn solve(self) -> ImagingSolve {
        let (sin, cos) = self.up.to_radians().sin_cos();
        let k = self.scale / 3600.0;
        ImagingSolve::from_cd(
            self.ra,
            self.dec,
            [[-k * cos, -k * sin], [k * sin, -k * cos]],
            self.size.0,
            self.size.1,
        )
    }
}

const REFERENCE: Field = Field {
    ra: 83.8221,
    dec: -5.3911,
    up: 0.0,
    scale: 2.0,
    size: (1236, 822),
};

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} differs from {expected} by more than {tolerance}"
    );
}

fn point_close(actual: (f64, f64), expected: (f64, f64), tolerance: f64) {
    let error = (actual.0 - expected.0).hypot(actual.1 - expected.1);
    assert!(
        error <= tolerance,
        "{actual:?} vs {expected:?}, error {error}"
    );
}

fn map_close(actual: Placement, m: [[f64; 2]; 2], t: [f64; 2]) {
    for (row, values) in m.into_iter().enumerate() {
        for (column, expected) in values.into_iter().enumerate() {
            close(actual.m[row][column], expected, 1e-6);
        }
        close(actual.t[row], t[row], 1e-6);
    }
}

// Projecting by hand, spec shared setup: Cartesian directions b, e and n at a
// field's centre. No expected value calls ImagingSolve's sky_at or pixel_of.
const ARCSEC_PER_RADIAN: f64 = 206264.806;

fn basis(field: Field) -> [[f64; 3]; 3] {
    let (sa, ca) = field.ra.to_radians().sin_cos();
    let (sd, cd) = field.dec.to_radians().sin_cos();
    [
        [cd * ca, cd * sa, sd],
        [-sa, ca, 0.0],
        [-sd * ca, -sd * sa, cd],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}

fn sky_by_hand(field: Field, p: (f64, f64)) -> [f64; 3] {
    let dx = p.0 - f64::from(field.size.0) / 2.0;
    let dy = p.1 - f64::from(field.size.1) / 2.0;
    let (sin, cos) = field.up.to_radians().sin_cos();
    let xi = (-dx * cos - dy * sin) * field.scale / ARCSEC_PER_RADIAN;
    let eta = (dx * sin - dy * cos) * field.scale / ARCSEC_PER_RADIAN;
    let [b, e, n] = basis(field);
    let v = std::array::from_fn(|i| b[i] + xi * e[i] + eta * n[i]);
    let length = dot(v, v).sqrt();
    v.map(|component| component / length)
}

fn pixel_by_hand(field: Field, v: [f64; 3]) -> (f64, f64) {
    let [b, e, n] = basis(field);
    let denominator = dot(v, b);
    assert!(
        denominator > 0.0,
        "oracle place must be on this tangent plane"
    );
    let xi = dot(v, e) / denominator;
    let eta = dot(v, n) / denominator;
    let (sin, cos) = field.up.to_radians().sin_cos();
    (
        f64::from(field.size.0) / 2.0 + (-xi * cos + eta * sin) * ARCSEC_PER_RADIAN / field.scale,
        f64::from(field.size.1) / 2.0 + (-xi * sin - eta * cos) * ARCSEC_PER_RADIAN / field.scale,
    )
}

fn projected(reference: Field, sub: Field, p: (f64, f64)) -> (f64, f64) {
    pixel_by_hand(sub, sky_by_hand(reference, p))
}

#[test]
fn identity_quarter_turn_half_turn_scale_and_mirror_have_the_worked_maps() {
    let reference = REFERENCE.solve();
    map_close(Placement::identity(), [[1.0, 0.0], [0.0, 1.0]], [0.0, 0.0]);
    map_close(
        Placement::between(&reference, &reference).unwrap(),
        [[1.0, 0.0], [0.0, 1.0]],
        [0.0, 0.0],
    );

    let quarter = Placement::between(
        &reference,
        &Field {
            up: 90.0,
            ..REFERENCE
        }
        .solve(),
    )
    .unwrap();
    map_close(quarter, [[0.0, -1.0], [1.0, 0.0]], [1029.0, -207.0]);
    point_close(quarter.apply((718.0, 411.0)), (618.0, 511.0), 1e-6);
    close(quarter.turn_deg(), 90.0, 1e-6);
    close(quarter.area_scale(), 1.0, 1e-6);
    point_close(
        quarter
            .centre_shift(REFERENCE.size, REFERENCE.size)
            .unwrap(),
        (0.0, 0.0),
        1e-6,
    );

    let half = Placement::between(
        &reference,
        &Field {
            up: 180.0,
            ..REFERENCE
        }
        .solve(),
    )
    .unwrap();
    map_close(half, [[-1.0, 0.0], [0.0, -1.0]], [1236.0, 822.0]);
    point_close(half.apply((0.0, 0.0)), (1236.0, 822.0), 1e-6);
    close(half.turn_deg().abs(), 180.0, 1e-6);

    let scaled = Placement::between(
        &reference,
        &Field {
            scale: 4.0,
            size: (618, 411),
            ..REFERENCE
        }
        .solve(),
    )
    .unwrap();
    map_close(scaled, [[0.5, 0.0], [0.0, 0.5]], [0.0, 0.0]);
    close(scaled.area_scale(), 0.25, 1e-6);
    point_close(
        scaled.centre_shift(REFERENCE.size, (618, 411)).unwrap(),
        (0.0, 0.0),
        1e-6,
    );

    let k = 2.0 / 3600.0;
    let mirrored = ImagingSolve::from_cd(83.8221, -5.3911, [[k, 0.0], [0.0, -k]], 1236, 822);
    let mirror = Placement::between(&reference, &mirrored).unwrap();
    map_close(mirror, [[-1.0, 0.0], [0.0, 1.0]], [1236.0, 0.0]);
    close(mirror.area_scale(), -1.0, 1e-6);
}

#[test]
fn a_sub_east_of_the_reference_shifts_its_pixels_right_and_its_centre_left() {
    let reference = Field {
        ra: 10.0,
        dec: 0.0,
        ..REFERENCE
    };
    let sub = Field {
        ra: 10.01,
        ..reference
    };
    let placement = Placement::between(&reference.solve(), &sub.solve()).unwrap();
    for (row, values) in placement.m.into_iter().enumerate() {
        for (column, actual) in values.into_iter().enumerate() {
            close(actual, if row == column { 1.0 } else { 0.0 }, 1e-6);
        }
    }
    close(placement.t[0], 18.0, 1e-4);
    close(placement.t[1], 0.0, 1e-4);
    point_close(placement.apply((618.0, 411.0)), (636.0, 411.0), 1e-4);
    point_close(
        placement.centre_shift(reference.size, sub.size).unwrap(),
        (-18.0, 0.0),
        1e-4,
    );
}

#[test]
fn meridian_convergence_is_carried_through_the_sky() {
    let reference = Field {
        ra: 30.0,
        dec: 60.0,
        ..REFERENCE
    };
    let sub = Field {
        ra: 30.04,
        ..reference
    };
    let placement = Placement::between(&reference.solve(), &sub.solve()).unwrap();
    for p in [
        (118.0, 411.0),
        (1118.0, 411.0),
        (618.0, 111.0),
        (618.0, 711.0),
        (1118.0, 711.0),
    ] {
        point_close(placement.apply(p), projected(reference, sub, p), 0.01);
    }
    close(placement.turn_deg(), -0.03464, 0.0005); // degrees

    // Pin the prescribed five-point central differences, not an angle-only map.
    let c = (618.0, 411.0);
    let q = 822.0 / 4.0;
    let fc = projected(reference, sub, c);
    let xp = projected(reference, sub, (c.0 + q, c.1));
    let xm = projected(reference, sub, (c.0 - q, c.1));
    let yp = projected(reference, sub, (c.0, c.1 + q));
    let ym = projected(reference, sub, (c.0, c.1 - q));
    let m = [
        [(xp.0 - xm.0) / (2.0 * q), (yp.0 - ym.0) / (2.0 * q)],
        [(xp.1 - xm.1) / (2.0 * q), (yp.1 - ym.1) / (2.0 * q)],
    ];
    map_close(
        placement,
        m,
        [
            fc.0 - m[0][0] * c.0 - m[0][1] * c.1,
            fc.1 - m[1][0] * c.0 - m[1][1] * c.1,
        ],
    );
}

#[test]
fn a_place_beyond_a_quarter_turn_and_a_singular_map_have_no_inverse_place() {
    let sub = Field {
        ra: 203.8,
        dec: 5.4,
        ..REFERENCE
    };
    assert_eq!(Placement::between(&REFERENCE.solve(), &sub.solve()), None);
    assert_eq!(
        Placement::identity().centre_shift((64, 48), (64, 48)),
        Some((0.0, 0.0))
    );
    let singular = Placement {
        m: [[0.0; 2]; 2],
        t: [0.0; 2],
    };
    assert_eq!(singular.centre_shift((64, 48), (64, 48)), None);
    let almost_singular = Placement {
        m: [[1.0, 0.0], [0.0, 1e-13]],
        t: [0.0; 2],
    };
    assert_eq!(almost_singular.centre_shift((64, 48), (64, 48)), None);
}

#[test]
fn wide_field_near_sub_is_approximate_and_far_or_diagonal_subs_fail_the_corners() {
    let reference = Field {
        scale: 3.0,
        size: (3600, 2400),
        ..REFERENCE
    };
    let near = Field {
        ra: 84.0721,
        ..reference
    };
    assert_eq!(CORNER_SLACK_PX, 0.5);
    let placement = Placement::between(&reference.solve(), &near.solve()).unwrap();
    for corner in [(0.0, 0.0), (3600.0, 0.0), (0.0, 2400.0), (3600.0, 2400.0)] {
        let exact = projected(reference, near, corner);
        let mapped = placement.apply(corner);
        let error = (mapped.0 - exact.0).hypot(mapped.1 - exact.1);
        assert!(
            (0.2..0.3).contains(&error),
            "corner {corner:?}, error {error}"
        );
    }
    for (ra, dec) in [(84.5721, -5.3911), (84.4606, -4.7544), (83.1836, -4.7544)] {
        let sub = Field {
            ra,
            dec,
            ..reference
        };
        assert_eq!(
            Placement::between(&reference.solve(), &sub.solve()),
            None,
            "sub ({ra}, {dec}) must fail the corner check"
        );
    }
}

#[test]
fn the_whole_cd_matrix_carries_shear() {
    // Same tangent plane: inv([[-k, k/2], [0, -k]]) * diag(-k, -k)
    // is [[1, 1/2], [0, 1]], about the two grids' common centre.
    let k = 2.0 / 3600.0;
    let sub = ImagingSolve::from_cd(
        REFERENCE.ra,
        REFERENCE.dec,
        [[-k, k / 2.0], [0.0, -k]],
        1236,
        822,
    );
    let placement = Placement::between(&REFERENCE.solve(), &sub).unwrap();
    map_close(placement, [[1.0, 0.5], [0.0, 1.0]], [-205.5, 0.0]);
    close(placement.area_scale(), 1.0, 1e-6);
    close(
        placement.turn_deg(),
        (-0.5_f64).atan2(2.0).to_degrees(),
        1e-6,
    );
}
