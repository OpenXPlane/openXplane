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

/// `0x1411a0080`: the planform area of span element `i` of a wing with `elements` elements, the
/// cosine of the sweep field (`+0x30`, degrees) times the span (`+0x10`) times the mean of the
/// element's two boundary chords (`+0x70`), divided by the element count (`+0x4`). The cosine comes
/// from the platform's libm here, so the last bit can differ.
pub fn element_area(sweep_field: f32, span: f32, chord: &[f32], i: usize, elements: i32) -> f32 {
    let cosine = f64::from((sweep_field * RADIANS_PER_DEGREE).cos());
    let mean = f64::from(chord[i] + chord[i + 1]) * 0.5 * f64::from(span);
    (cosine * mean / f64::from(elements as f32)) as f32
}

/// Control surface codes of `0x141192870` with the byte offsets it reads: the wing object fields that
/// hold the first and last element of the surface (float32 holding whole numbers), and the control
/// object fields that hold the deflection values at the two ends.
const CONTROL_SURFACES: [(u32, usize, usize, usize, usize); 13] = [
    (0xb, 0x324, 0x328, 0x1dfc, 0x1e00),
    (0xc, 0x354, 0x358, 0x1e0c, 0x1e10),
    (0xd, 0x444, 0x448, 0x1e18, 0x1e1c),
    (0xe, 0x474, 0x478, 0x1e28, 0x1e2c),
    (0xf, 0x4a4, 0x4a8, 0x1e3c, 0x1e40),
    (0x10, 0x384, 0x388, 0x1e4c, 0x1e50),
    (0x11, 0x3b4, 0x3b8, 0x1e60, 0x1e64),
    (0x12, 0x3e4, 0x3e8, 0x1e74, 0x1e78),
    (0x13, 0x414, 0x418, 0x1e88, 0x1e8c),
    (0x14, 0x534, 0x538, 0x1e9c, 0x1ea0),
    (0x15, 0x564, 0x568, 0x1ec4, 0x1ec8),
    (0x16, 0x4d4, 0x4d8, 0x1ee0, 0x1ee4),
    (0x17, 0x504, 0x508, 0x1ef0, 0x1ef4),
];

/// `0x141192870`: the deflection (times chord) of control surface `code` at span element `index`.
/// The surface covers elements `first..=last`; its end values are multiplied by the chords at those
/// elements (`+0x70`) and interpolated linearly over the element index (the plain mean when the
/// surface is one element). `wing` and `control` read float32 fields by byte offset. `None` for a
/// code outside `0xb..=0x17`.
pub fn control_deflection(
    code: u32,
    wing: &dyn Fn(usize) -> f32,
    control: &dyn Fn(usize) -> f32,
    index: i32,
) -> Option<f32> {
    let &(_, first_at, last_at, a_at, b_at) = CONTROL_SURFACES.iter().find(|c| c.0 == code)?;
    let first = wing(first_at) as i32;
    let last = wing(last_at) as i32;
    let chord = |i: i32| wing(0x70 + 4 * i as usize);
    let end = control(b_at) * chord(last);
    let start = control(a_at) * chord(first);
    let (first_f, last_f) = (first as f32, last as f32);
    Some(if first_f == last_f {
        (end + start) * 0.5
    } else {
        (end - start) / (last_f - first_f) * (index as f32 - first_f) + start
    })
}

/// `minss`: the first operand when it is smaller, otherwise the second (also for NaN).
fn sse_min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// `maxss`: the first operand when it is larger, otherwise the second.
fn sse_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// `0x1406ea0b0`: the value at `x` on the line through `(a0, v0)` and `(a1, v1)`, limited to the range of
/// the two values; the mean of the values when `a0 == a1`.
pub fn interpolate_clamped(a0: f32, v0: f32, a1: f32, v1: f32, x: f32) -> f32 {
    if a0 == a1 {
        return (v0 + v1) * 0.5;
    }
    let t = (v1 - v0) / (a1 - a0) * (x - a0) + v0;
    let low = sse_min(v0, v1);
    if low > t {
        low
    } else {
        sse_min(sse_max(v0, v1), t)
    }
}

/// `0x1411b1cf0`: a signed ramp of an angle in degrees, 0 to 0.9 over the first 20 degrees of magnitude
/// (slope 0.045), 0.9 to 1.0 up to 45 degrees (slope 0.004), then growing slowly to at most 1.2
/// (slope 0.004444), with the sign of the angle. The aircraft argument is not used.
pub fn angle_shape(x: f32) -> f32 {
    let sign = if 0.0 > x { -1.0f32 } else { 1.0 };
    let a = x.abs();
    let r = if 20.0 > a {
        let t = a * f32::from_bits(0x3d38_51eb) + 0.0;
        if 0.0 > t { 0.0 } else { sse_min(0.9, t) }
    } else if 45.0 > a {
        let t = (a - 20.0) * f32::from_bits(0x3b83_1271) + 0.9;
        if 0.9 > t { 0.9 } else { sse_min(1.0, t) }
    } else {
        let t = (a - 45.0) * f32::from_bits(0x3b91_a2b6) + 1.0;
        if 1.0 > t { 1.0 } else { sse_min(1.2, t) }
    };
    r * sign
}

/// Everything `0x141221220` reads, named by its offset in the original objects: `A` the aircraft/flight
/// object, `W` the wing object and `X` the wing-element array object.
#[derive(Clone, Copy, Debug)]
pub struct ControlSurface {
    /// Control surface code, `0xb..=0x17`.
    pub code: u32,
    /// The deflection from [`control_deflection`] and the chord at the element (`W+0x70+4i`).
    pub deflection: f32,
    pub chord: f32,
    /// `W+0x0` and `W+0x20`.
    pub wing_0: f32,
    pub wing_20: f32,
    /// `X+0x2c+4i` and `X+0x1bc+4i`.
    pub x_2c: f32,
    pub x_1bc: f32,
    /// The angle argument in degrees.
    pub angle_deg: f32,
    /// `A+0x1d20`, `A+0x1d24`, `A+0x1d28` and `A+0x1f74`.
    pub modes: [i32; 3],
    pub kind: i32,
    /// `A+0x1f84+4k` and `A+0x1fc4+4k` at the table index `A+0x1eb0`.
    pub table_a: f32,
    pub table_b: f32,
    /// `A+0x1f78`, `+0x1f7c`, `+0x1f80`, `+0x1fb8`, `+0x1fbc`, `+0x1fc0`.
    pub ratios: [f32; 6],
}

const DEGREES: f32 = 57.295_776;
const DEAD_ZONE: f32 = 0.01;

/// The magnitude floor the original applies to a divisor: values below -0.01 or above 0.01 stay,
/// the rest become -0.01 (negative input) or 0.01.
#[allow(clippy::manual_range_contains)]
pub fn angle_floor(v: f32) -> f32 {
    if -DEAD_ZONE > v || v > DEAD_ZONE {
        v
    } else if 0.0 > v {
        -DEAD_ZONE
    } else {
        DEAD_ZONE
    }
}

/// The original divides by the literal 0.7071, not by the square root of one half.
#[allow(clippy::approx_constant)]
fn sine_ratio(degrees: f32) -> f32 {
    ((degrees * RADIANS_PER_DEGREE).sin() as f64 / 0.7071) as f32
}

/// `0x141221220`: the four terms the control surface of `code` adds to the element's force and moment
/// accumulators (the original does `out += term` on four float32 outputs), or `None` when it adds
/// nothing. `driven(id)` answers the original's input-binding queries (`0x1407ace10`, ids
/// `0x2d9..=0x2f0`). The diagnostic output of the original is not ported. Trigonometric functions come
/// from the platform's libm, so the last bits can differ.
pub fn control_surface_terms(i: &ControlSurface, driven: &dyn Fn(u32) -> bool) -> Option<[f32; 4]> {
    let c = i.deflection / i.chord;
    let w0 = i.wing_0;
    // which of the two driven paths applies, or neither
    #[derive(PartialEq)]
    enum Path {
        None,
        A,
        B,
    }
    let flagged = |first: u32, second: u32, path: Path| -> Path {
        let flag = 0.0 > w0 && driven(first);
        let direct = w0 > 0.0 && driven(second);
        if flag || direct { path } else { Path::None }
    };
    let path = match i.code {
        0xb => flagged(0x2d9, 0x2da, Path::B),
        0xc => flagged(0x2db, 0x2dc, Path::B),
        0x10 => flagged(0x2dd, 0x2de, Path::B),
        0x11 => flagged(0x2df, 0x2e0, Path::B),
        0xf => flagged(0x2e7, 0x2e8, Path::B),
        0xd => flagged(0x2e3, 0x2e4, Path::A),
        0xe => flagged(0x2e5, 0x2e6, Path::A),
        0x16 => flagged(0x2e9, 0x2ea, Path::A),
        0x17 => flagged(0x2eb, 0x2ec, Path::A),
        0x12 if driven(0x2e1) => Path::B,
        0x13 if driven(0x2e2) => Path::B,
        0x14 => {
            let (mut flag, mut direct) = (false, false);
            if 0.0 > w0 {
                flag = driven(0x2ed);
                direct = driven(0x2ef);
            }
            if flag || direct { Path::B } else { Path::None }
        }
        0x15 => {
            let (mut flag, mut direct) = (false, false);
            if w0 > 0.0 {
                flag = driven(0x2ee);
                direct = driven(0x2f0);
            }
            if flag || direct { Path::B } else { Path::None }
        }
        _ => Path::None,
    };
    match path {
        Path::A => {
            let t = (-i.x_2c * c) as f64 * 0.1;
            return Some([t as f32, 0.0, 0.05, 0.0]);
        }
        Path::B => {
            let v = f64::from(-i.x_2c * c);
            let last = ((0.75 - f64::from(c) * 0.5) * -0.0) as f32;
            return Some([(v + v) as f32, 0.0, 0.1, last]);
        }
        Path::None => {}
    }

    let angle = i.angle_deg;
    if angle.is_nan() || angle.abs() <= 0.0 {
        return None;
    }
    let preamble = matches!(i.code, 0xb | 0xc | 0x10..=0x15);
    let first = if preamble {
        let r = angle * RADIANS_PER_DEGREE;
        let d = ((r.cos() * c) as f64 + (1.0 - f64::from(c))) as f32;
        let s = r.sin() * c;
        let degrees = f64::from(s.atan2(d) * DEGREES);
        let scale = match i.kind {
            5 => 0.54,
            1..=4 => 0.48,
            _ => 0.6,
        };
        (degrees * scale) as f32
    } else {
        0.0
    };
    let radians = angle * RADIANS_PER_DEGREE;
    let abs_radians = angle.abs() * RADIANS_PER_DEGREE;
    let flap_like = |first: f32| {
        let s = f64::from(radians.sin());
        let t = s * (f64::from(c) * 1.23);
        [first, 0.0, (t + t) as f32, 0.0]
    };
    let terms = match i.code {
        0xf => flap_like(first),
        0x16 | 0x17 if i.wing_20.abs() > 45.0 => flap_like(first),
        0x16 | 0x17 | 0xd | 0xe => {
            let r = (1.0 - f64::from((i.x_1bc * i.x_1bc).abs())) as f32;
            let g = if 0.0 > r { 0.0 } else { sse_min(1.0, r) };
            let s = abs_radians.sin() * c;
            let second = (f64::from(-s * g) * 8.0) as f32;
            let third = s * g;
            [first, second, third, third]
        }
        0xb | 0xc | 0x10..=0x13 => {
            let r1 = interpolate_clamped(0.0, 1.0, 15.0, 0.75, angle.abs());
            let mut second = (f64::from(r1) * (f64::from(c * angle) * 0.1)) as f32;
            let v = i.x_1bc;
            let sign_a = if 0.0 > angle { -1.0f32 } else { 1.0 };
            let sign_v = if 0.0 > v { -1.0f32 } else { 1.0 };
            let limit = if sign_a == sign_v {
                let f = (1.0 - f64::from(v.abs())) as f32;
                if 0.0 > f { 0.0 } else { sse_min(1.0, f) }
            } else {
                1.0
            };
            second *= limit;
            let mode = match i.code {
                0xb | 0xc => i.modes[0],
                0x10 | 0x11 => i.modes[1],
                _ => i.modes[2],
            };
            let gain = match mode {
                0 => 0.8,
                1 => 0.9,
                3 => 1.5,
                _ => 1.0,
            };
            second *= gain;
            let ratio = sine_ratio(angle.abs()) * interpolate_clamped(0.0, 0.0, 0.35, 1.0, c);
            let third = (f64::from(ratio) * 0.14) as f32;
            let fourth = ((0.75 - f64::from(c) * 0.5) * f64::from(-second)) as f32;
            [first, second, third, fourth]
        }
        0x14 => {
            let shape = angle_shape(i.table_a);
            let floor = angle_floor(shape);
            let applied = angle_shape(angle);
            let second = applied * i.ratios[0] / floor;
            let divisor = angle_floor(sine_ratio(i.table_a.abs()));
            let third = sine_ratio(angle.abs()) * i.ratios[1] / divisor;
            let fourth = applied * i.ratios[2] / angle_floor(shape);
            [first, second, third, fourth]
        }
        0x15 => {
            let shape = angle_shape(i.table_b);
            let floor = angle_floor(shape);
            let applied = angle_shape(angle);
            let second = applied * i.ratios[3] / floor;
            let divisor = angle_floor(sine_ratio(i.table_b.abs()));
            let third = sine_ratio(angle.abs()) * i.ratios[4] / divisor;
            let fourth = applied * i.ratios[5] / angle_floor(shape);
            [first, second, third, fourth]
        }
        _ => [0.0; 4],
    };
    Some(terms)
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

/// `0x1408be280`: `sqrt(x^2 + y^2)` in float32.
pub fn hypot2(x: f32, y: f32) -> f32 {
    let sum = x * x + y * y;
    if 0.0 > sum { f32::NAN } else { sum.sqrt() }
}

/// `0x14090e310`: `sqrt(x^2 + y^2 + z^2)` in float32.
pub fn hypot3(x: f32, y: f32, z: f32) -> f32 {
    let sum = x * x + y * y + z * z;
    if 0.0 > sum { f32::NAN } else { sum.sqrt() }
}

/// `0x141291580`: rotates `(a, b, c)` with three angles in degrees, the object floats at `+0x9c`, `+0xa0`
/// and `+0xa4` (`angles[0]`, `angles[1]`, `angles[2]`), and returns the three results in the order the
/// original stores them.
pub fn rotate_euler(angles: [f32; 3], a: f32, b: f32, c: f32) -> [f32; 3] {
    let (r0, r1, r2) = (
        angles[2] * RADIANS_PER_DEGREE,
        angles[1] * RADIANS_PER_DEGREE,
        angles[0] * RADIANS_PER_DEGREE,
    );
    let (cp, sp) = (r0.cos(), r0.sin());
    let (cq, sq) = (r1.cos(), r1.sin());
    let (cr, sr) = (r2.cos(), r2.sin());
    let u = cr * c - sr * a;
    let v = cr * a + sr * c;
    let w = sq * u + cq * b;
    [cp * v - sp * w, cp * w + sp * v, cq * u - sq * b]
}
