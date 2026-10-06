//! Pieces of the engine model of the reference build (research/ENGINE.md), ported one verified function at
//! a time against the original machine code.
use crate::element_force::Mem;

const RADIANS_PER_DEGREE: f32 = f32::from_bits(0x3c8e_fa36);

/// `minss`: the first operand when it is smaller, otherwise the second.
fn sse_min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// `maxss`: the first operand when it is larger, otherwise the second.
fn sse_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// `0x1408625a0`: `x^p` with the sign of `x` (`-(-x)^p` for negative `x`, 0 for zero). The power comes
/// from the platform's libm here and from the C runtime in the original.
pub fn signed_pow(x: f32, p: f32) -> f32 {
    if x > 0.0 {
        x.powf(p)
    } else if 0.0 > x {
        -((-x).powf(p))
    } else {
        0.0
    }
}

/// `0x14082b800` and `0x1411a2bf0` (two copies of the same code): `v0 + (v1 - v0) * t^p` where `t` is the
/// position of `x` between `a0` and `a1` limited to 0..1; for `p == 1` the clamped linear interpolation
/// of [`crate::wing_element::interpolate_clamped`]; for `a0 == a1`, `t = 0.5^p`.
pub fn curve(a0: f32, v0: f32, a1: f32, v1: f32, x: f32, p: f32) -> f32 {
    if p == 1.0 {
        return crate::wing_element::interpolate_clamped(a0, v0, a1, v1, x);
    }
    let t = if a0 == a1 {
        signed_pow(0.5, p)
    } else {
        let raw = (1.0 / (a1 - a0)) * (x - a0) + 0.0;
        let clamped = if 0.0 > raw { 0.0 } else { sse_min(1.0, raw) };
        signed_pow(clamped, p)
    };
    (v1 - v0) * (t - 0.0) + v0
}

/// `0x1411e39a0`: the intake and propeller power factor of one engine: the ram pressure rise of the Mach
/// number (with the shock recovery above the critical Mach number `B+0x984`), the propeller slipstream
/// speed from a five-step square root iteration, and the resulting fraction times the density ratio.
/// `desc_1c` is the float at `+0x1c` of the engine's descriptor (`B+0x5ff8 + 0x68 * index`).
pub fn ram_power_factor(
    b: &dyn Mem,
    desc_1c: f32,
    sigma: f32,
    f64_value: f32,
    f400: f32,
    f41c: f32,
    mach: f32,
) -> f32 {
    let mach_crit = b.f32(0x984);
    let efficiency = b.f32(0x980);
    let heat = (f64::from(mach * mach) * f64::from(0.2f32) + 1.0) as f32;
    let ram = if mach_crit < 1.0 {
        let rise = signed_pow(heat, 3.5);
        let mut ram = (f64::from((f64::from(rise) - 1.0) as f32 * efficiency) + 1.0) as f32;
        if mach > mach_crit {
            let ratio = mach / mach_crit;
            let square = ratio * ratio;
            let numerator = square * 2.4f32;
            let denominator = square * 0.4f32;
            let first = signed_pow(
                (f64::from(numerator) / (f64::from(denominator) + 2.0)) as f32,
                3.5,
            );
            let k = f64::from(square) * f64::from(2.8f32) - f64::from(0.4f32);
            let second = signed_pow((f64::from(2.4f32) / k) as f32, 2.5);
            let recovery = second * first;
            let degrees = if mach_crit == 1.0 {
                90.0f32
            } else {
                let t = 180.0 / (1.0 - mach_crit) * (mach - mach_crit) + 0.0;
                if 0.0 > t { 0.0 } else { sse_min(180.0, t) }
            };
            let low = sse_min(ram, recovery);
            let blend =
                ram - ((degrees * RADIANS_PER_DEGREE).cos() - 1.0) * ((recovery - ram) * 0.5);
            ram = if low > blend {
                low
            } else {
                sse_min(sse_max(ram, recovery), blend)
            };
        }
        ram
    } else {
        let weight = curve(1.0, 1.0, mach_crit, 0.0, mach, 1.25) * efficiency;
        let rise = signed_pow(heat, 3.5);
        (f64::from((f64::from(rise) - 1.0) as f32 * weight) + 1.0) as f32
    };
    let density = b.f32(0x950) * 1.225f32;
    let speed = mach_crit * 340.29f32;
    let d = f64::from(desc_1c);
    let mut g = 170.145f32;
    for _ in 0..5 {
        let m = (d / (f64::from(density * g) * 0.5)) as f32;
        g = m * 0.5 + g * 0.5;
    }
    let mix = b.f32(0x940);
    let combined = ((f64::from(mix * g) + f64::from(speed)) / (f64::from(mix) + 1.0)) as f32;
    let mut fraction = combined / (combined + f400);
    fraction *= ram;
    fraction *= b.f32(0x988) / sse_max(b.f32(0x988), f41c);
    let tail = b.f32(0x98c) / sse_max(b.f32(0x98c), f64_value);
    signed_pow(tail, 0.5) * fraction * sigma
}
