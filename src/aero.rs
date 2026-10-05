//! Isolated angle correction and table lookup recovered from the reference EXE.
//! Not a complete wing model: callers supply upstream correction factors.
use crate::airfoil::{Coefficients, POLAR_ROWS, Polar};

#[derive(Debug)]
pub struct TableSample {
    pub effective_alpha_deg: f32,
    pub lookup_alpha_deg: f32,
    pub lower: usize,
    pub upper: usize,
    pub lower_weight: f32,
    pub upper_weight: f32,
    pub coefficients: Coefficients,
}

// Original uses repeated addss, including distinct -180 and +180 endpoints.
fn wrap_angle(mut angle: f32) -> Result<f32, String> {
    if !angle.is_finite() {
        return Err("non-finite intermediate angle".into());
    }
    // Bound diagnostic work for pathological inputs; not an original format limit.
    for _ in 0..4096 {
        if angle < -180.0 {
            angle += 360.0;
        } else if angle > 180.0 {
            angle += -360.0;
        } else {
            return Ok(angle);
        }
    }
    Err("angle exceeds diagnostic wrap budget".into())
}

/// Instruction chain 0x141a412df..0x141a41419.
/// parameter[1] is the slope operand; parameter[2] is the zero-angle lift operand.
pub fn effective_alpha(
    polar: &Polar,
    induced_alpha_deg: f32,
    alpha_multiplier: f32,
    alpha_divisor: f32,
) -> Result<f32, String> {
    let slope = polar.parameters[1];
    let cl_zero = polar.parameters[2];
    if ![
        slope,
        cl_zero,
        induced_alpha_deg,
        alpha_multiplier,
        alpha_divisor,
    ]
    .iter()
    .all(|x| x.is_finite())
        || alpha_divisor == 0.0
    {
        return Err("invalid angle-correction inputs".into());
    }
    let zero_lift_alpha = if slope > 0.0 { -cl_zero / slope } else { 0.0 };
    let offset = induced_alpha_deg - zero_lift_alpha;
    let scaled = offset * alpha_multiplier;
    let divided = scaled / alpha_divisor;
    let base = divided + zero_lift_alpha;
    let fade_start = induced_alpha_deg.abs() - 30.0;
    let fade_scaled = fade_start * f32::from_bits(0x3d088889);
    let fade_offset = fade_scaled + 0.0;
    let fade = fade_offset.clamp(0.0, 1.0);
    let delta = wrap_angle(induced_alpha_deg - base)?;
    let correction = fade * delta;
    wrap_angle(correction + base)
}

/// Table portion 0x141a414ae..0x141a417ba, with upstream effective angle supplied.
/// The divisor also multiplies the interpolated Cl in the original instruction chain.
/// Excludes the subsequent stall perturbation branch; see crate::profile.
pub fn sample_table(
    polar: &Polar,
    effective_alpha_deg: f32,
    alpha_divisor: f32,
) -> Result<TableSample, String> {
    if !effective_alpha_deg.is_finite() || !alpha_divisor.is_finite() {
        return Err("non-finite table inputs".into());
    }
    if polar.rows.len() != POLAR_ROWS {
        return Err("reference lookup requires 721 rows".into());
    }
    let alpha = effective_alpha_deg.clamp(-180.0, 179.0);
    let coordinate = if alpha < -20.0 {
        let shifted = alpha - (-180.0);
        let added = shifted + 0.0;
        added.clamp(0.0, 160.0)
    } else if alpha < 20.0 {
        let shifted = alpha - (-20.0);
        let scaled = shifted * 10.0;
        let added = scaled + 160.0;
        added.clamp(160.0, 560.0)
    } else {
        let shifted = alpha - 20.0;
        let added = shifted + 560.0;
        added.clamp(560.0, 719.0)
    };
    let lower = coordinate as usize; // cvttss2si: truncation of nonnegative coordinate
    let upper = lower + 1;
    let lower_weight = upper as f32 - coordinate;
    let upper_weight = 1.0 - lower_weight;
    let a = polar.rows[lower].coefficients;
    let b = polar.rows[upper].coefficients;
    let mix = |x: f32, y: f32| {
        let left = lower_weight * x;
        let right = upper_weight * y;
        left + right
    };
    let lift = mix(a.cl, b.cl);
    let coefficients = Coefficients {
        cl: lift * alpha_divisor,
        cd: mix(a.cd, b.cd),
        cm: mix(a.cm, b.cm),
    };
    if ![coefficients.cl, coefficients.cd, coefficients.cm]
        .iter()
        .all(|x| x.is_finite())
    {
        return Err("non-finite interpolated coefficients".into());
    }
    Ok(TableSample {
        effective_alpha_deg,
        lookup_alpha_deg: alpha,
        lower,
        upper,
        lower_weight,
        upper_weight,
        coefficients,
    })
}

pub fn evaluate_table(
    polar: &Polar,
    induced_alpha_deg: f32,
    alpha_multiplier: f32,
    alpha_divisor: f32,
) -> Result<TableSample, String> {
    let alpha = effective_alpha(polar, induced_alpha_deg, alpha_multiplier, alpha_divisor)?;
    sample_table(polar, alpha, alpha_divisor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::airfoil::PolarRow;

    fn polar() -> Polar {
        let mut parameters = [0.0; 25];
        parameters[1] = 0.1;
        parameters[2] = 0.2;
        Polar {
            parameters,
            rows: (0..POLAR_ROWS)
                .map(|i| PolarRow {
                    // Deliberately unrelated: reference lookup ignores the alpha column.
                    alpha_deg: i as f32,
                    coefficients: Coefficients {
                        cl: i as f32,
                        cd: i as f32 * 2.0,
                        cm: -(i as f32),
                    },
                })
                .collect(),
        }
    }

    #[test]
    fn uses_fixed_grid_and_reference_endpoint_clamp() {
        let p = polar();
        for (angle, index) in [
            (-180.0, 0.0),
            (-20.5, 159.5),
            (-20.0, 160.0),
            (0.0, 360.0),
            (0.05, 360.5),
            (20.0, 560.0),
            (20.5, 560.5),
            (179.0, 719.0),
            (180.0, 719.0),
        ] {
            let s = sample_table(&p, angle, 1.0).unwrap();
            assert_eq!(s.coefficients.cl, index);
            assert_eq!(s.coefficients.cd, index * 2.0);
            assert_eq!(s.upper, s.lower + 1);
            assert_eq!(s.lower_weight + s.upper_weight, 1.0);
        }
    }

    #[test]
    fn corrects_about_zero_lift_and_fades_at_large_angles() {
        let p = polar();
        // Zero-lift angle -2; correction factor 1/2 around that point.
        assert_eq!(effective_alpha(&p, 10.0, 1.0, 2.0).unwrap(), 4.0);
        assert_eq!(effective_alpha(&p, 30.0, 1.0, 2.0).unwrap(), 14.0);
        assert!((effective_alpha(&p, 45.0, 1.0, 2.0).unwrap() - 33.25).abs() < 0.00001);
        assert_eq!(effective_alpha(&p, 60.0, 1.0, 2.0).unwrap(), 60.0);
        assert_eq!(effective_alpha(&p, -60.0, 1.0, 2.0).unwrap(), -60.0);
        let s = evaluate_table(&p, 10.0, 1.0, 2.0).unwrap();
        assert_eq!(s.coefficients.cl, 800.0);
        assert_eq!(s.coefficients.cd, 800.0); // Cd stays unscaled: index 400 * 2
        assert_eq!(s.coefficients.cm, -400.0);
    }

    #[test]
    fn wraps_angles_and_rejects_invalid_operands() {
        let mut p = polar();
        assert_eq!(wrap_angle(180.0).unwrap(), 180.0);
        assert_eq!(wrap_angle(-180.0).unwrap(), -180.0);
        assert_eq!(wrap_angle(181.0).unwrap(), -179.0);
        assert_eq!(wrap_angle(-181.0).unwrap(), 179.0);
        assert!(effective_alpha(&p, 0.0, 1.0, 0.0).is_err());
        assert!(effective_alpha(&p, f32::NAN, 1.0, 1.0).is_err());
        assert!(sample_table(&p, f32::INFINITY, 1.0).is_err());
        p.parameters[1] = 0.0;
        assert_eq!(effective_alpha(&p, 10.0, 1.0, 2.0).unwrap(), 5.0);
        p.rows.pop();
        assert!(sample_table(&p, 0.0, 1.0).is_err());
    }
}
