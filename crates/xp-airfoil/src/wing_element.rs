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

use crate::regimes::blend;

const PI: f32 = std::f32::consts::PI;
/// 5.729578: the lift slope of a thin airfoil per radian divided by ten, as a float32.
const SLOPE_OVER_TEN: f32 = 5.729_578;
const VISCOSITY: [f32; 3] = [1.463_8e-5, 1.723_1e-5, 1.960_8e-5];

/// Field values of the flow/aircraft context object (`RCX`), named by offset.
#[derive(Clone, Copy, Debug)]
pub struct Flow {
    /// `+0x5c`: the temperature-like value that selects the viscosity segment.
    pub f5c: f32,
    /// `+0x6c`: multiplies the element speed in the Reynolds number.
    pub f6c: f32,
    pub f1a0: f32,
    pub f1a4: f32,
    pub f408: f32,
    /// `+0xdac` is nonzero.
    pub flag_dac: bool,
    /// `+0xbcc8` or `+0xbcd0` is nonzero (diagnostic logging in the original).
    pub diagnostics: bool,
}

/// Field values of the aircraft object reached through `+0x20` of the flow object.
#[derive(Clone, Copy, Debug)]
pub struct Aircraft {
    pub f1f00: f32,
    pub f1f3c: f32,
    pub f64f4: f32,
    pub f64f8: f32,
    pub f64fc: f32,
}

/// The wing object (`RDX`): the ACF layout plus runtime arrays, named by offset.
#[derive(Clone, Copy, Debug)]
pub struct WingFields<'a> {
    /// `+0x00` `_is_right_mult`.
    pub is_right: f32,
    /// `+0x04` `_els`.
    pub elements: i32,
    pub f14: f32,
    pub f18: f32,
    pub f1c: f32,
    /// `+0x5c..+0x68`: `_foil_rat_rot`, `_foil_rat_mid_inner`, `_foil_rat_mid_outer`, `_foil_rat_tip`.
    pub ratios: [f32; 4],
    pub boundary: Boundary<'a>,
    /// `+0x56c`, `+0x594`: per element flap and slat flags.
    pub flap_flags: &'a [i32],
    pub slat_flags: &'a [i32],
    /// The airfoil names at `+0x3618`, `+0x3638`, `+0x3658` (root, middle, tip).
    pub names: [&'a str; 3],
}

/// Per-call values of the float array (`R8`), for the element being evaluated.
#[derive(Clone, Copy, Debug)]
pub struct ElementState {
    /// `R8[1 + e]`: speed-like value used in the flow factor and induced drag.
    pub v: f32,
    /// `R8[11 + e]` (`+0x2c`): added to the input angle.
    pub r11: f32,
    /// `R8[21 + e]` (`+0x54`): element speed in the Reynolds number.
    pub r21: f32,
    /// `R8[163]` (`+0x28c`): scales the induced drag.
    pub r163: f32,
    /// `R8[121 + e]` (`+0x1e4`): the persistent stall flag of the element.
    pub stall_flag: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ElementInputs<'a> {
    pub index: usize,
    /// `R9`: allow the stall memory to be kept.
    pub retain_request: bool,
    /// Stack argument at `+0x148`, passed through to the profile function.
    pub arg6: f32,
    /// Stack argument at `+0x150`: the ice factor.
    pub ice: f32,
    /// Stack argument at `+0x158`: the input angle.
    pub alpha_in: f32,
    /// Stack arguments at `+0x160`, `+0x168`, `+0x170`: added to the Cl, Cd and Cm accumulators.
    pub extra: [f32; 3],
    pub flow: Flow,
    pub aircraft: Aircraft,
    /// `[[mid airfoil + 0xd0] + 0x10]`.
    pub g10: f32,
    pub wing: WingFields<'a>,
    pub state: ElementState,
    /// Continue on the straight-wing path when the delta-wing weight is positive instead of failing
    /// (the delta-wing block is not ported; the result then ignores that effect).
    pub skip_delta_block: bool,
}

/// What the original passes to the profile function `0x141a44350` for one of the three airfoils.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoilCall {
    /// 0 root, 1 middle, 2 tip.
    pub slot: usize,
    pub x_norm: f32,
    pub y_norm: f32,
    pub z_norm: f32,
    pub retain: bool,
    pub diagnostics: bool,
    pub re_meg: f32,
    pub arg6: f32,
    pub alpha: f32,
    pub multiplier: f32,
    pub divisor: f32,
    pub flag_dac: bool,
    /// The element's stall flag as it stands when the call is made.
    pub stalled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoilResult {
    /// Returned in `XMM0`: blended with the weights into the function's own return value.
    pub ret: f32,
    pub cl: f32,
    pub cd: f32,
    pub cm: f32,
    /// The fourth output, accumulated into `R8[111 + e]` (`+0x1bc`).
    pub ratio: f32,
    pub stalled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElementOutput {
    pub ret: f32,
    pub cl: f32,
    pub cd: f32,
    pub cm: f32,
    pub ratio: f32,
    pub induced_drag: f32,
    pub stall_flag: bool,
}

fn clamp_zero_one(v: f32) -> f32 {
    // comiss / minss in the original; the same for every input, NaN and -0 included
    v.clamp(0.0, 1.0)
}

fn ramp(from: f32, to: f32, t: f32) -> f32 {
    if from == to {
        0.5
    } else {
        clamp_zero_one((1.0 / (to - from)) * (t - from) + 0.0)
    }
}

/// The angle factor fade of the caller: moves `x` towards 1 as the absolute angle grows past 20
/// degrees, limited to the range between `x` and 1.
fn fade(x: f32, abs_angle_minus_20: f32) -> f32 {
    let lo = if x < 1.0 { x } else { 1.0 };
    let moved = (1.0 - x) / 70.0 * abs_angle_minus_20 + x;
    if lo > moved {
        lo
    } else {
        let hi = if x > 1.0 { x } else { 1.0 };
        if hi < moved { hi } else { moved }
    }
}

/// The straight-wing path of `0x1411b6630`. `profile` stands for `0x141a44350`: it receives what the
/// original passes and returns what it writes. A positive delta-wing weight selects a block that is
/// not ported yet and is an error.
pub fn evaluate<F>(inp: &ElementInputs, mut profile: F) -> Result<ElementOutput, String>
where
    F: FnMut(&FoilCall) -> Result<FoilResult, String>,
{
    let e = inp.index;
    let w = &inp.wing;
    let v = inp.state.v;

    let sweep = sweep_degrees(&w.boundary, e);
    let _lateral_sweep = sweep + inp.flow.f408 * w.is_right;
    let slope = SLOPE_OVER_TEN / ((w.f14 * PI) * v);
    let flow_factor = (1.0f64 / (f64::from(slope) + 1.0)) as f32;
    let root = f64::from(flow_factor) * 8.0 + 1.0;
    let k = ((root.sqrt() + 1.0) * 0.25) as f32;
    let multiplier0 = flow_factor / k;
    let delta_weight = delta_wing_weight(&w.boundary, e, w.f18, w.f1c);
    let retain = inp.retain_request && 0.01 > f64::from(delta_weight);

    let ice = f64::from(inp.ice);
    let alpha_scale = (ice + 1.0) as f32;
    let ice_cl = (1.0 / (ice * 1.1 + 1.0)) as f32;
    let ice_cd = (ice * 4.0 + 1.0) as f32;

    // flap and slat influence on the divisor
    let mut flap = 1.0f32;
    if w.flap_flags[e] != 0 {
        flap = (inp.flow.f1a0 - 0.0) * ((inp.g10 + inp.aircraft.f1f00) / inp.g10 - 1.0) + 1.0;
    }
    if w.slat_flags[e] != 0 {
        let slat = (inp.flow.f1a4 - 0.0) * ((inp.g10 + inp.aircraft.f1f3c) / inp.g10 - 1.0) + 1.0;
        flap *= slat;
    }

    // the element angle wrapped into -180..180
    let mut alpha = (inp.alpha_in + inp.state.r11) * alpha_scale;
    let mut guard = 0u32;
    while -180.0 > alpha {
        alpha += 360.0;
        guard += 1;
        if guard > 1_000_000 {
            return Err("element angle does not converge".into());
        }
    }
    while alpha > 180.0 {
        alpha += -360.0;
        guard += 1;
        if guard > 1_000_000 {
            return Err("element angle does not converge".into());
        }
    }
    let beyond = alpha.abs() - 20.0;
    let divisor = fade(flap, beyond);
    let multiplier = fade(multiplier0, beyond);

    // dynamic viscosity by linear interpolation through three temperature points
    let t = inp.flow.f5c;
    let (x0, y0, x1, y1) = if 0.0 > t {
        (-50.0, VISCOSITY[0], 0.0, VISCOSITY[1])
    } else {
        (0.0, VISCOSITY[1], 50.0, VISCOSITY[2])
    };
    let mu = blend(x0, y0, x1, y1, t)?;

    // chord Reynolds number in millions, from the mean aerodynamic chord of the element
    let (c0, c1) = (w.boundary.chord[e], w.boundary.chord[e + 1]);
    let ratio = c1 / c0;
    let r1 = f64::from(ratio) + 1.0;
    let mu_e6 = f64::from(mu) * 1_000_000.0;
    let ratio_sq = ratio * ratio;
    let mac = ((f64::from(ratio_sq) + r1) / r1) * (f64::from(c0) * 0.666_666_666_666_666_6);
    let speed = inp.state.r21 * inp.flow.f6c;
    let product = (mac as f32) * speed;
    let re_meg = (f64::from(product) / mu_e6) as f32;

    // normalised position of the element centre
    let n = w.elements;
    let eb = e as i32;
    let i0 = if eb < 0 {
        0
    } else if eb > n - 1 {
        n - 1
    } else {
        eb
    } as usize;
    let xe = e as f32;
    let t_span = ((f64::from(xe) + 0.5) as f32) / (n as f32);
    let at = |a: &[f32]| blend(i0 as f32, a[i0], (i0 + 1) as f32, a[i0 + 1], xe);
    let x_norm = at(w.boundary.x)? / inp.aircraft.f64f4;
    let y_norm = at(w.boundary.y)? / inp.aircraft.f64f8;
    let z_norm = at(w.boundary.z)? / inp.aircraft.f64fc;

    // weights of the root, middle and tip airfoils
    let mut w_root = ramp(w.ratios[1], w.ratios[0], t_span);
    if w.names[0] == w.names[1] {
        w_root = 0.0;
    }
    let mut w_tip = ramp(w.ratios[2], w.ratios[3], t_span);
    if w.names[1] == w.names[2] {
        w_tip = 0.0;
    }
    let w_mid = (1.0f64 - f64::from(w_root) - f64::from(w_tip)) as f32;
    let weights = [w_root, w_mid, w_tip];

    let (mut cl, mut cd, mut cm, mut ratio_acc, mut ret_acc) =
        (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let mut stall_flag = inp.state.stall_flag;
    for (slot, weight) in weights.into_iter().enumerate() {
        // comiss/jbe: a zero, negative or NaN weight skips the call
        if weight.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            continue;
        }
        let out = profile(&FoilCall {
            slot,
            x_norm,
            y_norm,
            z_norm,
            retain,
            diagnostics: inp.flow.diagnostics,
            re_meg,
            arg6: inp.arg6,
            alpha,
            multiplier,
            divisor,
            flag_dac: inp.flow.flag_dac,
            stalled: stall_flag,
        })?;
        stall_flag = out.stalled;
        ret_acc = if slot == 0 {
            out.ret * weight + 0.0
        } else {
            ret_acc + out.ret * weight
        };
        cl += out.cl * weight;
        cd += out.cd * weight;
        cm += out.cm * weight;
        ratio_acc += out.ratio * weight;
    }

    cl += inp.extra[0];
    cd += inp.extra[1];
    cm += inp.extra[2];
    cl *= k;

    if delta_weight > 0.0 && !inp.skip_delta_block {
        return Err("the delta-wing block of the wing element function is not ported yet".into());
    }

    let induced = (cl * cl) / ((w.f14 * PI) * v);
    let induced = induced * inp.state.r163;
    cd += induced;

    if inp.ice > 0.0 {
        cl *= ice_cl;
        cd *= ice_cd;
    }
    Ok(ElementOutput {
        ret: ret_acc,
        cl,
        cd,
        cm,
        ratio: ratio_acc,
        induced_drag: induced,
        stall_flag,
    })
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
