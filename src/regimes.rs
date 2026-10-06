//! Reference table selection and interpolation; see research/AFL_REGIMES.md.
use crate::{
    aero,
    airfoil::{Airfoil, Coefficients},
};

#[derive(Debug, PartialEq)]
pub struct Selection {
    pub lower: usize,
    pub upper: usize,
}

pub fn select(airfoil: &Airfoil, parameter: f32) -> Result<Selection, String> {
    if !parameter.is_finite() || airfoil.polars.is_empty() {
        return Err("invalid regime input or empty airfoil".into());
    }
    let mut lower = None;
    let mut upper = None;
    let mut lower_distance = f32::MAX;
    let mut upper_distance = f32::MAX;
    for (index, polar) in airfoil.polars.iter().enumerate() {
        let value = polar.parameters[0];
        if !value.is_finite() {
            return Err("non-finite regime parameter".into());
        }
        let distance = (value - parameter).abs();
        if !distance.is_finite() {
            return Err("regime distance overflow".into());
        }
        // Strict comparisons retain the first equal candidate in source order.
        if parameter >= value && distance < lower_distance {
            lower = Some(index);
            lower_distance = distance;
        }
        if value >= parameter && distance < upper_distance {
            upper = Some(index);
            upper_distance = distance;
        }
    }
    let lower = lower.or(upper).ok_or("no regime candidate")?;
    let upper = upper.unwrap_or(lower);
    Ok(Selection { lower, upper })
}

/// Instruction order of helper 0x1406ea0b0; equal abscissae average ordinates.
pub fn blend(x0: f32, y0: f32, x1: f32, y1: f32, x: f32) -> Result<f32, String> {
    if ![x0, y0, x1, y1, x].iter().all(|v| v.is_finite()) {
        return Err("non-finite blend operand".into());
    }
    let value = if x0 == x1 {
        (y0 + y1) * 0.5
    } else {
        let difference_y = y1 - y0;
        let difference_x = x1 - x0;
        let offset = x - x0;
        let slope = difference_y / difference_x;
        let delta = slope * offset;
        let interpolated = delta + y0;
        if !interpolated.is_finite() {
            return Err("blend arithmetic overflow".into());
        }
        interpolated.clamp(y0.min(y1), y0.max(y1))
    };
    if !value.is_finite() {
        return Err("non-finite blend result".into());
    }
    Ok(value)
}

/// Diagnostic composition of the recovered table stage and regime blend.
/// The original blends full evaluator outputs, including later corrections available separately in crate::profile.
pub fn evaluate_table_stage(
    airfoil: &Airfoil,
    parameter: f32,
    alpha: f32,
    multiplier: f32,
    divisor: f32,
) -> Result<(Selection, Coefficients), String> {
    let selection = select(airfoil, parameter)?;
    let a = &airfoil.polars[selection.lower];
    let ca = aero::evaluate_table(a, alpha, multiplier, divisor)?.coefficients;
    if selection.lower == selection.upper {
        return Ok((selection, ca));
    }
    let b = &airfoil.polars[selection.upper];
    let cb = aero::evaluate_table(b, alpha, multiplier, divisor)?.coefficients;
    let mix = |y0, y1| blend(a.parameters[0], y0, b.parameters[0], y1, parameter);
    let coefficients = Coefficients {
        cl: mix(ca.cl, cb.cl)?,
        cd: mix(ca.cd, cb.cd)?,
        cm: mix(ca.cm, cb.cm)?,
    };
    Ok((selection, coefficients))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::airfoil::Polar;
    fn airfoil(values: &[f32]) -> Airfoil {
        Airfoil {
            version: 1110,
            header_scalars: [0.0; 2],
            shape_points: [[0.0; 2]; 14],
            polars: values
                .iter()
                .map(|v| {
                    let mut parameters = [0.0; 25];
                    parameters[0] = *v;
                    Polar {
                        parameters,
                        rows: vec![],
                    }
                })
                .collect(),
        }
    }
    #[test]
    fn brackets_unsorted_regimes_and_clamps_endpoints() {
        let a = airfoil(&[3.1, 0.5, 0.1, 1.0, 0.5]);
        for (x, lower, upper) in [
            (0.0, 2, 2),
            (0.3, 2, 1),
            (0.5, 1, 1),
            (0.8, 1, 3),
            (2.0, 3, 0),
            (4.0, 0, 0),
        ] {
            assert_eq!(select(&a, x).unwrap(), Selection { lower, upper });
        }
        assert_eq!(
            select(&airfoil(&[6.0]), 0.2).unwrap(),
            Selection { lower: 0, upper: 0 }
        );
        assert!(select(&airfoil(&[]), 1.0).is_err());
        assert!(select(&a, f32::NAN).is_err());
    }
    #[test]
    fn helper_blends_in_both_directions_and_averages_equal_abscissae() {
        assert_eq!(blend(0.0, 2.0, 4.0, 10.0, 1.0).unwrap(), 4.0);
        assert_eq!(blend(4.0, 10.0, 0.0, 2.0, 1.0).unwrap(), 4.0);
        assert_eq!(blend(0.0, 2.0, 4.0, 10.0, -1.0).unwrap(), 2.0);
        assert_eq!(blend(0.0, 10.0, 4.0, 2.0, 5.0).unwrap(), 2.0);
        assert_eq!(blend(1.0, 2.0, 1.0, 10.0, 9.0).unwrap(), 6.0);
        assert!(blend(0.0, 2.0, 4.0, 10.0, f32::INFINITY).is_err());
    }
}
