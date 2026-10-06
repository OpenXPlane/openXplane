//! Pieces of the wing element aerodynamics function `0x1411b6630` (research/WING_ELEMENT.md), ported
//! one verified piece at a time. Inputs are named by what the code does with them; the field offsets
//! in the original object are noted for traceability. Float32 and double steps follow the original's
//! order. The transcendental functions (`atan2`, `tan`) come from the platform's libm here and from
//! the C runtime in the original, so results can differ in the last bit; the verification test allows
//! a few ulps for those.

/// Boundary points of the span elements of one wing (one entry per boundary, `elements + 1`):
/// the three coordinate arrays at `+0x5bc`, `+0x5e8`, `+0x614` and the chord at `+0x70`.
#[derive(Clone, Copy, Debug)]
pub struct Boundary<'a> {
    pub x: &'a [f32],
    pub y: &'a [f32],
    pub z: &'a [f32],
    pub chord: &'a [f32],
}

/// Radians to degrees as the original multiplies it (a double constant).
const DEGREES_PER_RADIAN: f64 = 57.295_776_367_187_5;
/// The float32 degrees to radians factor.
const RADIANS_PER_DEGREE: f32 = f32::from_bits(0x3c8e_fa36);

/// `0x1411a0230`: the sweep angle in degrees of the quarter-chord line of element `i`, the slope of
/// the quarter-chord point's z (the third coordinate minus a quarter of the chord) over the distance
/// between the first two coordinates of the element's two boundary points.
pub fn sweep_degrees(b: &Boundary, i: usize) -> f32 {
    // distance in the plane of the first two coordinate arrays (float32, as sqrtss)
    let d0 = b.x[i + 1] - b.x[i];
    let d1 = b.y[i + 1] - b.y[i];
    let distance = (d0 * d0 + d1 * d1).sqrt();
    let upper = f64::from(b.z[i + 1]) - f64::from(b.chord[i + 1]) * 0.25;
    let lower = f64::from(b.z[i]) - f64::from(b.chord[i]) * 0.25;
    let rise = upper - lower;
    (rise.atan2(f64::from(distance)) * DEGREES_PER_RADIAN) as f32
}

fn clamp01_low_high(v: f32) -> f32 {
    // `if 0 > v { 0 } else { min(1, v) }` as comiss/minss
    if 0.0 > v { 0.0 } else { 1.0f32.min(v) }
}

/// `0x1411a00f0`: the delta-wing weight of element `i`, the product of three factors in 0..1: a ramp
/// of the sweep over 40 degrees at 0.1 per degree, a ratio built from the object field at `+0x18`
/// and the tangent of the sweep (limited to 15..75 degrees), and `2 (1 - field at +0x1c)`.
pub fn delta_wing_weight(b: &Boundary, i: usize, field_18: f32, field_1c: f32) -> f32 {
    let s = sweep_degrees(b, i);
    let abs = s.abs();
    let limited = if 15.0 > abs { 15.0 } else { 75.0f32.min(abs) };
    let tangent = (limited * RADIANS_PER_DEGREE).tan();
    let c = (4.0f64 / f64::from(tangent)) as f32;

    let ramp = clamp01_low_high((sweep_degrees(b, i) - 40.0) * 0.1 + 0.0);

    let doubled = (f64::from(c) + f64::from(c)) as f32;
    let ratio = if doubled == c {
        0.5
    } else {
        let denominator = c - doubled;
        clamp01_low_high((1.0 / denominator) * (field_18 - doubled) + 0.0)
    };

    let y = 0.0 - ((field_1c - 1.0) + (field_1c - 1.0));
    let tail = if 0.0 > y { 0.0 } else { 1.0f32.min(y) };
    ramp * ratio * tail
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_wing(sweep_deg: f32) -> ([f32; 2], [f32; 2], [f32; 2], [f32; 2]) {
        // two boundary points 1 m apart along x, chord 1 m; the z of the second is shifted aft
        let rise = sweep_deg.to_radians().tan();
        ([0.0, 1.0], [0.0, 0.0], [0.0, rise], [1.0, 1.0])
    }

    #[test]
    fn sweep_is_the_slope_of_the_quarter_chord_line() {
        for deg in [0.0f32, 5.0, 25.0, 45.0, -30.0] {
            let (x, y, z, chord) = flat_wing(deg);
            let b = Boundary {
                x: &x,
                y: &y,
                z: &z,
                chord: &chord,
            };
            assert!((sweep_degrees(&b, 0) - deg).abs() < 1e-3, "{deg}");
        }
    }

    #[test]
    fn a_taper_moves_the_quarter_chord_line() {
        // the chord shrinks from 2 to 1 along 1 m, so the quarter-chord z moves by +0.25 even with z fixed
        let (x, y, z, chord) = ([0.0, 1.0], [0.0, 0.0], [0.0, 0.0], [2.0, 1.0]);
        let b = Boundary {
            x: &x,
            y: &y,
            z: &z,
            chord: &chord,
        };
        assert!((sweep_degrees(&b, 0) - 0.25f32.atan().to_degrees()).abs() < 1e-3);
    }

    #[test]
    fn delta_weight_is_zero_for_unswept_and_straight_wings() {
        let (x, y, z, chord) = flat_wing(5.0);
        let b = Boundary {
            x: &x,
            y: &y,
            z: &z,
            chord: &chord,
        };
        assert_eq!(delta_wing_weight(&b, 0, 1.0, 0.0), 0.0);
        // the ramp starts at 40 degrees; a 60 degree sweep reaches the full ramp
        let (x, y, z, chord) = flat_wing(60.0);
        let b = Boundary {
            x: &x,
            y: &y,
            z: &z,
            chord: &chord,
        };
        let w = delta_wing_weight(&b, 0, 1.0, 0.0);
        assert!((0.0..=1.0).contains(&w) && w > 0.0, "{w}");
        // the last factor 2 (1 - f) vanishes at f = 1
        assert_eq!(delta_wing_weight(&b, 0, 1.0, 1.0), 0.0);
    }
}
