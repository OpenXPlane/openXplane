//! Normalized angle and stall latch from 0x141a417c5..0x141a41907.
//! Excludes subsequent time-dependent coefficient perturbations.
use crate::airfoil::Polar;

#[derive(Debug, Clone, Copy)]
pub struct StallState {
    pub normalized_alpha: f32,
    pub stalled: bool,
}

/// Supply the lookup angle after the evaluator's -180/179 clamp.
/// The original writes this normalized value separately from Cl/Cd/Cm.
pub fn evaluate(
    polar: &Polar,
    lookup_alpha: f32,
    retain_state: bool,
    previously_stalled: bool,
) -> Result<StallState, String> {
    let negative_limit = polar.parameters[3];
    let positive_limit = polar.parameters[4];
    if ![lookup_alpha, negative_limit, positive_limit]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err("non-finite stall operand".into());
    }
    let normalized_alpha = if lookup_alpha > 0.0 {
        if positive_limit == 0.0 {
            0.5
        } else {
            let reciprocal = 1.0 / (positive_limit - 0.0);
            let offset = lookup_alpha - 0.0;
            let scaled = reciprocal * offset;
            scaled + 0.0
        }
    } else if negative_limit == 0.0 {
        -0.5
    } else {
        let denominator = negative_limit - 0.0;
        let reciprocal = 1.0 / denominator;
        let offset = lookup_alpha - 0.0;
        let scaled = reciprocal * offset;
        0.0 - scaled
    };
    if !normalized_alpha.is_finite() {
        return Err("non-finite normalized angle".into());
    }
    let magnitude = normalized_alpha.abs();
    let stalled = if retain_state {
        if magnitude > 1.0 {
            true
        } else if magnitude < 0.75 {
            false
        } else {
            previously_stalled
        }
    } else {
        magnitude > 1.0
    };
    Ok(StallState {
        normalized_alpha,
        stalled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn polar() -> Polar {
        let mut parameters = [0.0; 25];
        parameters[3] = -10.0;
        parameters[4] = 20.0;
        Polar {
            parameters,
            rows: vec![],
        }
    }
    #[test]
    fn uses_asymmetric_limits_and_strict_latch_thresholds() {
        let p = polar();
        assert_eq!(
            evaluate(&p, -5.0, false, false).unwrap().normalized_alpha,
            -0.5
        );
        assert_eq!(
            evaluate(&p, 10.0, false, false).unwrap().normalized_alpha,
            0.5
        );
        let mut state = false;
        for (alpha, expected) in [
            (20.0, false),
            (21.0, true),
            (20.0, true),
            (15.0, true),
            (14.0, false),
        ] {
            state = evaluate(&p, alpha, true, state).unwrap().stalled;
            assert_eq!(state, expected);
        }
        assert!(!evaluate(&p, 20.0, false, true).unwrap().stalled);
        assert!(evaluate(&p, -11.0, true, false).unwrap().stalled);
    }
    #[test]
    fn preserves_zero_limit_fallback_and_rejects_invalid_input() {
        let mut p = polar();
        p.parameters[3] = 0.0;
        p.parameters[4] = 0.0;
        assert_eq!(
            evaluate(&p, 1.0, false, false).unwrap().normalized_alpha,
            0.5
        );
        assert_eq!(
            evaluate(&p, -1.0, false, false).unwrap().normalized_alpha,
            -0.5
        );
        // At zero both original branches run; the negative branch writes last.
        assert_eq!(
            evaluate(&p, 0.0, false, false).unwrap().normalized_alpha,
            -0.5
        );
        assert!(evaluate(&p, f32::NAN, false, false).is_err());
    }
}
