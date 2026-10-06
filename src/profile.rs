//! Composition of recovered polar evaluation and regime interpolation.
//! Upstream flow/correction factors and the runtime noise table remain explicit.
use crate::{
    aero::{self, TableSample},
    airfoil::{Airfoil, Coefficients, Polar},
    buffet::NoiseTable,
    regimes::{self, Selection},
    runtime::RunningTime,
    stall::{self, StallState},
};

#[derive(Clone, Copy, Debug)]
pub struct Inputs {
    pub alpha_deg: f32,
    pub multiplier: f32,
    pub divisor: f32,
    pub regime: f32,
    pub time: RunningTime,
    pub noise_coordinates: [f32; 3],
    pub retain_stall: bool,
}

#[derive(Debug)]
pub struct PolarEvaluation {
    pub table: TableSample,
    pub stall: StallState,
    pub coefficients: Coefficients,
}

#[derive(Debug)]
pub struct Evaluation {
    pub selection: Selection,
    pub lower: PolarEvaluation,
    pub upper: Option<PolarEvaluation>,
    pub coefficients: Coefficients,
    pub normalized_alpha: f32,
    pub stalled: bool,
}

/// Numerical outputs of 0x141a412c0, excluding its diagnostic logging.
pub fn evaluate_polar(
    polar: &Polar,
    input: Inputs,
    previously_stalled: bool,
    noise: Option<&NoiseTable>,
) -> Result<PolarEvaluation, String> {
    let table = aero::evaluate_table(polar, input.alpha_deg, input.multiplier, input.divisor)?;
    let stall = stall::evaluate(
        polar,
        table.lookup_alpha_deg,
        input.retain_stall,
        previously_stalled,
    )?;
    let coefficients = if stall.stalled {
        noise
            .ok_or("active stall requires an explicitly supplied runtime noise table")?
            .perturb_at_time(table.coefficients, input.noise_coordinates, input.time)?
    } else {
        table.coefficients
    };
    Ok(PolarEvaluation {
        table,
        stall,
        coefficients,
    })
}

/// The caller uses one shared stall pointer: lower writes it before upper reads it.
/// Blend numerical outputs after evaluating each polar independently.
/// This is not the full caller 0x141a44350 or a force/wing model.
pub fn evaluate(
    airfoil: &Airfoil,
    input: Inputs,
    previously_stalled: bool,
    noise: Option<&NoiseTable>,
) -> Result<Evaluation, String> {
    let selection = regimes::select(airfoil, input.regime)?;
    let a = &airfoil.polars[selection.lower];
    let lower = evaluate_polar(a, input, previously_stalled, noise)?;
    let Some(b) = (selection.lower != selection.upper).then(|| &airfoil.polars[selection.upper])
    else {
        return Ok(Evaluation {
            selection,
            coefficients: lower.coefficients,
            normalized_alpha: lower.stall.normalized_alpha,
            stalled: lower.stall.stalled,
            lower,
            upper: None,
        });
    };
    let upper = evaluate_polar(b, input, lower.stall.stalled, noise)?;
    let mix = |y0, y1| regimes::blend(a.parameters[0], y0, b.parameters[0], y1, input.regime);
    let coefficients = Coefficients {
        cl: mix(lower.coefficients.cl, upper.coefficients.cl)?,
        cd: mix(lower.coefficients.cd, upper.coefficients.cd)?,
        cm: mix(lower.coefficients.cm, upper.coefficients.cm)?,
    };
    let normalized_alpha = mix(lower.stall.normalized_alpha, upper.stall.normalized_alpha)?;
    Ok(Evaluation {
        selection,
        coefficients,
        normalized_alpha,
        stalled: upper.stall.stalled,
        lower,
        upper: Some(upper),
    })
}

/// Inputs of the profile function `0x141a44350`, which wraps `evaluate` with a compressibility
/// correction. `mach` is the stack argument the wing element function passes at `+0x148`.
#[derive(Clone, Copy, Debug)]
pub struct OuterInputs {
    pub alpha_deg: f32,
    pub multiplier: f32,
    pub divisor: f32,
    pub regime: f32,
    pub mach: f32,
    pub time: RunningTime,
    pub noise_coordinates: [f32; 3],
    pub retain_stall: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OuterOutput {
    pub cl: f32,
    pub cd: f32,
    pub cm: f32,
    pub normalized_alpha: f32,
    pub stalled: bool,
    /// Returned in `XMM0`: the second header scalar minus a tenth of the absolute Cl.
    pub ret: f32,
}

/// Prandtl-Glauert style factor `1 / sqrt(1 - m^2)` with `m` the Mach argument limited to 0..0.7.
pub fn compressibility_factor(mach: f32) -> f32 {
    let m = mach.clamp(0.0, 0.7);
    let m2 = m * m;
    (1.0f64 / (1.0f64 - f64::from(m2)).sqrt()) as f32
}

/// `0x141a44350`: the profile evaluation, then Cl and Cm multiplied by the compressibility factor.
pub fn outer(
    airfoil: &Airfoil,
    input: OuterInputs,
    previously_stalled: bool,
    noise: Option<&NoiseTable>,
) -> Result<OuterOutput, String> {
    let e = evaluate(
        airfoil,
        Inputs {
            alpha_deg: input.alpha_deg,
            multiplier: input.multiplier,
            divisor: input.divisor,
            regime: input.regime,
            time: input.time,
            noise_coordinates: input.noise_coordinates,
            retain_stall: input.retain_stall,
        },
        previously_stalled,
        noise,
    )?;
    let factor = compressibility_factor(input.mach);
    let cl = factor * e.coefficients.cl;
    let cm = factor * e.coefficients.cm;
    let ret = (f64::from(airfoil.header_scalars[1]) - f64::from(cl.abs()) * 0.1) as f32;
    Ok(OuterOutput {
        cl,
        cd: e.coefficients.cd,
        cm,
        normalized_alpha: e.normalized_alpha,
        stalled: e.stalled,
        ret,
    })
}

pub const REPLAY_HEADER: &str = "alpha,multiplier,divisor,regime,time,x,y,phase,retain";

/// Strict numeric CSV: nine columns, no quoted fields; no implicit clock steps.
pub fn parse_replay(text: &str) -> Result<Vec<Inputs>, String> {
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    if lines.next() != Some(REPLAY_HEADER) {
        return Err(format!("expected replay header: {REPLAY_HEADER}"));
    }
    let mut result = Vec::new();
    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        let fields: Vec<_> = line.split(',').map(str::trim).collect();
        let parse = || -> Result<Inputs, String> {
            if fields.len() != 9 {
                return Err("expected nine numeric columns".into());
            }
            let float = |i: usize| -> Result<f32, String> {
                fields[i]
                    .parse::<f32>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| format!("invalid finite float32 in column {}", i + 1))
            };
            let retain_stall = match fields[8] {
                "0" => false,
                "1" => true,
                _ => return Err("retain must be 0 or 1".into()),
            };
            Ok(Inputs {
                alpha_deg: float(0)?,
                multiplier: float(1)?,
                divisor: float(2)?,
                regime: float(3)?,
                time: RunningTime::from_snapshot(
                    fields[4].parse::<f64>().map_err(|_| "invalid time")?,
                )?,
                noise_coordinates: [float(5)?, float(6)?, float(7)?],
                retain_stall,
            })
        };
        result.push(parse().map_err(|error| format!("replay line {line_number}: {error}"))?);
    }
    if result.is_empty() {
        return Err("replay requires at least one sample".into());
    }
    Ok(result)
}

/// State is local; an error does not mutate any caller-owned stall state.
pub fn replay(
    airfoil: &Airfoil,
    inputs: &[Inputs],
    initially_stalled: bool,
    noise: Option<&NoiseTable>,
) -> Result<Vec<Evaluation>, String> {
    let mut stalled = initially_stalled;
    let mut outputs = Vec::with_capacity(inputs.len());
    for (index, input) in inputs.iter().enumerate() {
        let output = evaluate(airfoil, *input, stalled, noise)
            .map_err(|error| format!("sample {}: {error}", index + 1))?;
        stalled = output.stalled;
        outputs.push(output);
    }
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::airfoil::{POLAR_ROWS, PolarRow};
    use crate::buffet::TABLE_LEN;

    fn airfoil() -> Airfoil {
        Airfoil {
            version: 1110,
            header_scalars: [0.0; 2],
            shape_points: [[0.0; 2]; 14],
            polars: [(0.0, 10.0), (1.0, 12.0)]
                .into_iter()
                .map(|(regime, limit)| {
                    let mut parameters = [0.0; 25];
                    parameters[0] = regime;
                    parameters[3] = -limit;
                    parameters[4] = limit;
                    Polar {
                        parameters,
                        rows: (0..POLAR_ROWS)
                            .map(|_| PolarRow {
                                alpha_deg: 0.0,
                                coefficients: Coefficients {
                                    cl: 2.0,
                                    cd: 0.25,
                                    cm: -0.1,
                                },
                            })
                            .collect(),
                    }
                })
                .collect(),
        }
    }

    fn input(alpha_deg: f32) -> Inputs {
        Inputs {
            alpha_deg,
            multiplier: 1.0,
            divisor: 1.0,
            regime: 0.5,
            time: RunningTime::from_snapshot(0.0).unwrap(),
            noise_coordinates: [0.0; 3],
            retain_stall: true,
        }
    }

    #[test]
    fn shared_latch_reaches_upper_polar_before_blending() {
        let a = airfoil();
        let noise = NoiseTable::new(vec![1.0; TABLE_LEN]).unwrap();
        // Lower exceeds 1; upper is below 1 but inside the hysteresis band.
        let result = evaluate(&a, input(11.0), false, Some(&noise)).unwrap();
        assert!(result.lower.stall.stalled);
        assert!(result.upper.as_ref().unwrap().stall.stalled);
        assert!(result.stalled);
        assert_eq!(result.coefficients.cl, 2.984375);
        assert_eq!(result.coefficients.cd, 255.0 / 512.0);
        assert_eq!(result.coefficients.cm, -0.1);
        // No retention: upper independently clears; blend includes one perturbed result.
        let mut stateless = input(11.0);
        stateless.retain_stall = false;
        let result = evaluate(&a, stateless, false, Some(&noise)).unwrap();
        assert!(!result.stalled);
        assert_eq!(result.coefficients.cl, 2.4921875);
        assert_eq!(result.coefficients.cd, 383.0 / 1024.0);
    }

    #[test]
    fn replay_retains_then_clears_and_requires_noise_only_on_active_branch() {
        let a = airfoil();
        assert!(evaluate(&a, input(0.0), false, None).is_ok());
        assert!(evaluate(&a, input(11.0), false, None).is_err());
        let noise = NoiseTable::new(vec![1.0; TABLE_LEN]).unwrap();
        let outputs = replay(
            &a,
            &[input(11.0), input(9.5), input(7.0)],
            false,
            Some(&noise),
        )
        .unwrap();
        assert_eq!(
            outputs.iter().map(|o| o.stalled).collect::<Vec<_>>(),
            [true, true, false]
        );
        assert_eq!(outputs[2].coefficients.cl, 2.0);
        let mut exact = input(11.0);
        exact.regime = 0.0;
        assert!(
            evaluate(&a, exact, false, Some(&noise))
                .unwrap()
                .upper
                .is_none()
        );
    }

    #[test]
    fn replay_parser_preserves_double_time_and_rejects_bad_rows() {
        let rows = parse_replay(&format!(
            "{REPLAY_HEADER}\r\n0,1,1,0.5,16777217,0,0,0,1\r\n"
        ))
        .unwrap();
        assert_eq!(rows[0].time.seconds(), 16_777_217.0);
        assert!(parse_replay(REPLAY_HEADER).is_err());
        assert!(parse_replay(&format!("{REPLAY_HEADER}\n0,1,1,0.5,NaN,0,0,0,1")).is_err());
        assert!(parse_replay(&format!("{REPLAY_HEADER}\n0,1,1,0.5,0,0,0,0,2")).is_err());
        assert!(
            parse_replay(&format!("{REPLAY_HEADER}\n0,1,1"))
                .unwrap_err()
                .contains("line 2")
        );
    }
}
