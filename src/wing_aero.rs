//! Recovered numerical blocks from the wing-element function, not the full force model.

#[derive(Debug, Clone, Copy)]
pub struct FlowFactors {
    pub flow: f32,
    pub lift_scale: f32,
    pub alpha_multiplier: f32,
}

/// 0x1411b6706..0x1411b6776. Caller supplies the original's two denominator operands.
/// The origin/meaning of efficiency is not established; no default is implied here.
pub fn flow_factors(aspect_ratio: f32, efficiency: f32) -> Result<FlowFactors, String> {
    if ![aspect_ratio, efficiency]
        .iter()
        .all(|v| v.is_finite() && *v > 0.0)
    {
        return Err("flow operands must be finite and positive".into());
    }
    let pi_ar = aspect_ratio * f32::from_bits(0x40490fdb);
    let denominator = pi_ar * efficiency;
    let ratio = f32::from_bits(0x40b758b4) / denominator;
    let flow = (1.0 / (f64::from(ratio) + 1.0)) as f32;
    let radicand = f64::from(flow) * 8.0 + 1.0;
    let lift_scale = ((radicand.sqrt() + 1.0) * 0.25) as f32;
    let alpha_multiplier = flow / lift_scale;
    if ![flow, lift_scale, alpha_multiplier]
        .iter()
        .all(|v| v.is_finite())
        || flow == 0.0
    {
        return Err("flow-factor arithmetic outside supported domain".into());
    }
    Ok(FlowFactors {
        flow,
        lift_scale,
        alpha_multiplier,
    })
}

/// 0x1411b68fc..0x1411b6979. Both factors use the same float32 chain.
/// Alpha supplied after the caller's wrap; 20..90 degrees is the transition.
pub fn fade_to_one(factor: f32, alpha_deg: f32) -> f32 {
    let remaining = 1.0 - factor;
    let slope = remaining / 70.0;
    let offset = alpha_deg.abs() - 20.0;
    let delta = slope * offset;
    let value = delta + factor;
    let lower = factor.min(1.0);
    if lower > value {
        lower
    } else {
        factor.max(1.0).min(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsupported_flow_operands() {
        for (a, e) in [(0.0, 1.0), (-1.0, 1.0), (1.0, 0.0), (f32::NAN, 1.0)] {
            assert!(flow_factors(a, e).is_err());
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Reynolds {
    pub viscosity: f32,
    pub chord: f32,
    pub millions: f32,
}

/// Original 0x1411b6979..0x1411b6a44, including the original interpolation helper.
/// Raw operands: wing +0x70/+0x74, flow +0x54 and object +0x6c/+0x5c.
/// Their production and units are not recovered; callers must supply them explicitly.
pub fn reynolds(
    temperature: f32,
    component_70: f32,
    component_74: f32,
    flow_54: f32,
    field_6c: f32,
) -> Result<Reynolds, String> {
    if ![temperature, component_70, component_74, flow_54, field_6c]
        .iter()
        .all(|v| v.is_finite())
        || component_70 == 0.0
    {
        return Err("invalid Reynolds operands".into());
    }
    let middle = f32::from_bits(0x37908b4a);
    let viscosity = if temperature < 0.0 {
        crate::regimes::blend(-50.0, f32::from_bits(0x377595bb), 0.0, middle, temperature)?
    } else {
        crate::regimes::blend(0.0, middle, 50.0, f32::from_bits(0x37a47bdc), temperature)?
    };
    let ratio = component_74 / component_70;
    let sum = f64::from(ratio) + 1.0;
    let square = ratio * ratio;
    let numerator = f64::from(square) + sum;
    let normalized = numerator / sum;
    let extent = f64::from(component_70) * f64::from_bits(0x3fe5555555555555);
    let chord = (normalized * extent) as f32;
    let flow = flow_54 * field_6c;
    let product = chord * flow;
    let millions = (f64::from(product) / (f64::from(viscosity) * 1_000_000.0)) as f32;
    if ![chord, millions].iter().all(|v| v.is_finite()) {
        return Err("Reynolds arithmetic outside supported domain".into());
    }
    Ok(Reynolds {
        viscosity,
        chord,
        millions,
    })
}

/// Original 0x1411b6817..0x1411b689b, before the already verified angle fade.
/// Flap/slat flags and raw reference operands are explicit; no guessed defaults.
pub fn flap_slat_factor(
    flap: bool,
    slat: bool,
    flap_state: f32,
    slat_state: f32,
    reference: f32,
    flap_extra: f32,
    slat_extra: f32,
) -> Result<f32, String> {
    if ![flap_state, slat_state, reference, flap_extra, slat_extra]
        .iter()
        .all(|v| v.is_finite())
        || ((flap || slat) && reference == 0.0)
    {
        return Err("invalid flap/slat operands".into());
    }
    let mut factor = 1.0;
    if flap {
        let state = flap_state - 0.0;
        let sum = reference + flap_extra;
        let ratio = sum / reference;
        let delta = ratio - 1.0;
        factor = state * delta + 1.0;
    }
    if slat {
        let state = slat_state - 0.0;
        let sum = reference + slat_extra;
        let ratio = sum / reference;
        let delta = ratio - 1.0;
        let slat_factor = state * delta + 1.0;
        factor *= slat_factor;
    }
    if !factor.is_finite() {
        return Err("flap/slat arithmetic overflow".into());
    }
    Ok(factor)
}

/// 0x1411b7c14..0x1411b7c42 plus 0x1411b7d87..0x1411b7da3.
pub fn induced_drag(
    cl: f32,
    cd: f32,
    aspect: f32,
    efficiency: f32,
    scale: f32,
) -> Result<(f32, f32), String> {
    if ![cl, cd, aspect, efficiency, scale]
        .iter()
        .all(|v| v.is_finite())
        || aspect <= 0.0
        || efficiency <= 0.0
    {
        return Err("invalid induced-drag operands".into());
    }
    let denominator = aspect * f32::from_bits(0x40490fdb);
    let denominator = denominator * efficiency;
    let squared = cl * cl;
    let raw = squared / denominator;
    let scaled = raw * scale;
    let total = scaled + cd;
    if ![scaled, total].iter().all(|v| v.is_finite()) {
        return Err("induced drag overflow".into());
    }
    Ok((scaled, total))
}

/// 0x1411b8d15..0x1411b8da0 with logging flags zero; original positive-condition branch.
pub fn ice_coefficients(
    cl: f32,
    cd: f32,
    amount: f32,
    cl_multiplier: f32,
    cd_multiplier: f32,
) -> (f32, f32) {
    if amount > 0.0 {
        (cl_multiplier * cl, cd_multiplier * cd)
    } else {
        (cl, cd)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IceFactors {
    pub alpha_scale: f32,
    pub lift_multiplier: f32,
    pub drag_multiplier: f32,
}
/// 0x1411b67b4..0x1411b6817. Mixed float64/float32 order is intentional.
pub fn ice_factors(amount: f32) -> Result<IceFactors, String> {
    if !amount.is_finite() {
        return Err("nonfinite ice operand".into());
    }
    let amount = f64::from(amount);
    let alpha_scale = (amount + 1.0) as f32;
    let lift_denominator = amount * f64::from_bits(0x3ff199999999999a) + 1.0;
    let lift_multiplier = (1.0 / lift_denominator) as f32;
    let drag_multiplier = (amount * 4.0 + 1.0) as f32;
    if ![alpha_scale, lift_multiplier, drag_multiplier]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err("ice-factor overflow".into());
    }
    Ok(IceFactors {
        alpha_scale,
        lift_multiplier,
        drag_multiplier,
    })
}

/// 0x1411b6d3b..0x1411b6e8f. Flags represent exact equality of adjacent name bytes.
/// Unordered/overlapping ratios are preserved; the middle weight may be negative.
// clamp preserves an input -0, unlike this recovered ADDSS/clamp chain.
#[allow(clippy::manual_clamp)]
pub fn span_weights(
    t: f32,
    ratios: [f32; 4],
    root_is_middle: bool,
    middle_is_tip: bool,
) -> [f32; 3] {
    let ramp = |from: f32, to: f32| {
        if from == to {
            0.5
        } else {
            let denominator = to - from;
            let reciprocal = 1.0 / denominator;
            let offset = t - from;
            let product = reciprocal * offset;
            let value = product + 0.0;
            // Original ADDSS +0 produces +0; do not let optimized min retain -0.
            if value <= 0.0 {
                0.0
            } else if value > 1.0 {
                1.0
            } else {
                value
            }
        }
    };
    let root = if root_is_middle {
        0.0
    } else {
        ramp(ratios[1], ratios[0])
    };
    let tip = if middle_is_tip {
        0.0
    } else {
        ramp(ratios[2], ratios[3])
    };
    let remainder = 1.0_f64 - f64::from(root);
    let middle = (remainder - f64::from(tip)) as f32;
    [root, middle, tip]
}

#[derive(Debug, Clone, Copy)]
pub struct SpanInputs {
    pub fraction: f32,
    pub normalized_position: [f32; 3],
}
/// 0x1411b6c10..0x1411b6d60. Original corner arrays, not invented element centres.
/// Positions use the integer element index; the weight fraction alone uses index + 0.5.
pub fn span_inputs(
    index: i32,
    points: &[[f32; 3]],
    reference: [f32; 3],
) -> Result<SpanInputs, String> {
    if !(2..=11).contains(&points.len())
        || !(-16_777_215..=16_777_215).contains(&index)
        || !points.iter().flatten().all(|v| v.is_finite())
        || !reference.iter().all(|v| v.is_finite() && *v != 0.0)
    {
        return Err("unsupported wing coordinate inputs".into());
    }
    let elements = (points.len() - 1) as i32;
    let index_float = index as f32;
    let midpoint = (f64::from(index_float) + 0.5) as f32;
    let fraction = midpoint / elements as f32;
    let segment = index.clamp(0, elements - 1) as usize;
    let mut normalized_position = [0.0; 3];
    for axis in 0..3 {
        let position = crate::regimes::blend(
            segment as f32,
            points[segment][axis],
            (segment + 1) as f32,
            points[segment + 1][axis],
            index_float,
        )?;
        normalized_position[axis] = position / reference[axis];
        if !normalized_position[axis].is_finite() {
            return Err("wing coordinate overflow".into());
        }
    }
    Ok(SpanInputs {
        fraction,
        normalized_position,
    })
}
