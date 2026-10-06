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

/// What the engine update asks of the rest of the simulation (callees of `0x141197b00` that depend on state
/// outside the engine): the two atmosphere accessors (`0x141ba6750`, `0x141ba64e0`), the engine flag test
/// (`0x1417f12c0`), the frame time (`0x140c448c0`) and the input-binding query (`0x1407ace10`, by id).
pub trait EngineEnv {
    fn atmosphere_a(&mut self, time: f32) -> f32;
    fn atmosphere_b(&mut self, time: f32, value: f32) -> f32;
    fn engine_flag(&mut self) -> bool;
    fn frame_time(&mut self) -> f64;
    fn binding(&mut self, id: u32, index: i32) -> bool;
}

/// One engine record of `0x2cc` bytes (the array behind `F+0x68b0`).
#[derive(Clone, Debug)]
pub struct Record(pub Vec<u32>);

impl Record {
    pub const WORDS: usize = 0x2cc / 4;

    pub fn f32(&self, offset: usize) -> f32 {
        f32::from_bits(self.0[offset / 4])
    }
    pub fn i32(&self, offset: usize) -> i32 {
        self.0[offset / 4] as i32
    }
    pub fn set_f32(&mut self, offset: usize, value: f32) {
        self.0[offset / 4] = value.to_bits();
    }
    pub fn set_i32(&mut self, offset: usize, value: i32) {
        self.0[offset / 4] = value as u32;
    }
}

/// `0x1411a2d90`: the engine can be started: the aircraft allows it, the engine record has `+0x1e4` set, and
/// neither input-binding query (`0x2fb` for index 0, `0x239` for the engine) is active.
fn starter_ready(b: &dyn Mem, rec: &Record, index: i32, env: &mut dyn EngineEnv) -> bool {
    if b.i32(0xaa4) == 0 || rec.i32(0x1e4) == 0 {
        return false;
    }
    if env.binding(0x2fb, 0) {
        return false;
    }
    !env.binding(0x239, index)
}

/// The engine update `0x141197b00`, first part: the intake power factor and the throttle and
/// mixture response written to `+0x258`, `+0x22c`, `+0x240`, `+0x244` and `+0x248` of the record. The rest of
/// the function is not ported yet (research/ENGINE.md).
pub fn engine_update(
    f: &dyn Mem,
    b: &dyn Mem,
    desc: &dyn Mem,
    rec: &mut Record,
    index: usize,
    env: &mut dyn EngineEnv,
) {
    let mach = f.f32(0x420);
    let inverse_mach = if 1.0 > mach {
        f64::from(mach)
    } else {
        1.0 / f64::from(mach)
    } as f32;

    let time_a = if env.engine_flag() { 0.0 } else { f.f64(0x3a0) } as f32;
    let heights = sse_max(
        sse_max(b.f32(0xc7c), b.f32(0xc80)),
        sse_max(b.f32(0xc84), b.f32(0xc88)),
    );
    let pressure = env.atmosphere_a(time_a);
    let shifted = pressure + heights * rec.f32(0x264);
    let time_b = if env.engine_flag() { 0.0 } else { f.f64(0x3a0) } as f32;
    let density = env.atmosphere_b(time_b, shifted);
    rec.set_f32(0x258, density / 1.225f32);
    let power = ram_power_factor(
        b,
        desc.f32(0x1c),
        rec.f32(0x258),
        f.f32(0x64),
        f.f32(0x400),
        f.f32(0x41c),
        f.f32(0x420),
    );
    rec.set_f32(0x258, power);

    // the throttle spool-up timer at +0x22c
    let mut dt = env.frame_time();
    if rec.i32(0x228) != 0 {
        dt += dt;
    }
    let dt = dt as f32;
    let timer = rec.f32(0x22c);
    if 1.0 > timer {
        let t = dt / b.f32(0xa24) + timer;
        rec.set_f32(0x22c, if 0.0 > t { 0.0 } else { sse_min(1.0, t) });
    }
    if (f.i32(0x28) != 0 || f.i32(0x6880) == 0) && b.f32(0x9bc) > rec.f32(0x98) {
        rec.set_i32(0x22c, 0);
    }

    let idle = ((f64::from(b.f32(0x9d8)) * 0.05) as f32, 0.0f32).0;
    let gain = throttle_gain(b, rec);
    let gain_scaled = (f64::from(gain) * 0.05) as f32;
    let lever = rec.f32(0x4) - 0.0;
    let mut response = sse_max(gain_scaled, (1.0 - idle) * lever + idle);
    let mut exponent = ((f64::from(b.f32(0x9b4) + b.f32(0x9b0))) * 0.5) as f32;
    if desc.i32(0) == 6 {
        exponent *= b.f32(0x9ac);
    }
    let mut zero_or_one = 0.0f64;
    if b.i32(0x978) != 0 {
        let inverse = (1.0 / f64::from(exponent)) as f32;
        let a = (f64::from(signed_pow(idle, inverse)) * 100.0) as f32;
        let c = (f64::from(signed_pow(gain_scaled, inverse)) * 100.0) as f32;
        let blended = sse_max((100.0 - a) * lever + a, c);
        let scaled = (f64::from(blended) * 0.01) as f32;
        response = signed_pow(scaled, exponent);
        zero_or_one = 0.0;
    }
    if rec.i32(0x74) != 0 {
        zero_or_one = 1.0;
    }
    let running = (f64::from(rec.f32(0x22c)) * zero_or_one) as f32;
    response *= running;
    let rpm = rec.f32(0x98);
    if env.binding(0x1d1, index as i32) {
        response = crate::wing_element::interpolate_clamped(10.0, response, 20.0, 0.0, rpm);
    }
    if rec.i32(0x74) != 0 {
        let b9d4 = b.f32(0x9d4);
        if b9d4 > 0.0 {
            let ratio = rpm / b9d4;
            if f64::from(rec.f32(0x22c)) > 0.99 {
                response =
                    crate::wing_element::interpolate_clamped(0.5, running, 1.0, response, ratio);
            }
        }
        let shape = (f64::from(exponent) * 0.5) as f32;
        let position = (f64::from(rpm) * 0.01) as f32;
        let low_a = (f64::from(b.f32(0x9c8)) * 0.01) as f32;
        let first = curve(low_a, idle, 1.0, 1.0, position, shape);
        let low_b = (f64::from(b.f32(0x9cc)) * 0.01) as f32;
        let second = curve(low_b, idle, 1.0, 1.0, position, shape);
        let limit = |response: &mut f32, factor: f32, bound: f32, slope: f32, scale: f32| {
            let mut x = f64::from(f.f32(0x60));
            if bound != 0.0 {
                x *= 1.8;
            }
            let x0 = x as f32;
            let t1 = crate::wing_element::interpolate_clamped(
                0.0,
                1.0,
                1.0,
                (factor - x0) / slope,
                lever,
            );
            let t2 = crate::wing_element::interpolate_clamped(0.0, 1.0, 1.0, t1, *response) * scale;
            *response = if 0.0 > *response {
                0.0
            } else if *response > t2 {
                t2
            } else {
                *response
            };
        };
        if starter_ready(b, rec, index as i32, env) {
            limit(
                &mut response,
                b.f32(0x1a74),
                if b.i32(0x1a84) == 0 { 1.0 } else { 0.0 },
                b.f32(0x1a80),
                first,
            );
        }
        if b.i32(0xaa8) != 0
            && rec.i32(0x1e4) != 0
            && !env.binding(0x2fb, 0)
            && !env.binding(0x239, index as i32)
        {
            limit(
                &mut response,
                b.f32(0x1a94),
                if b.i32(0x1aa4) == 0 { 1.0 } else { 0.0 },
                b.f32(0x1aa0),
                second,
            );
        }
        let divisor = f64::from(inverse_mach) * f64::from(0.2f32) + 1.0;
        let a = (f64::from(response / first) / divisor) as f32;
        rec.set_f32(0x240, a);
        let c = (f64::from(response / second) / divisor) as f32;
        rec.set_f32(0x244, c);
        rec.set_f32(0x248, ((f64::from(c + a)) * 0.5) as f32);
    }
}

/// `0x1411dd610`: the throttle gain between `B+0x9d8` and `B+0x9dc`, from the record's `+0x4c` (1.0 when the
/// aircraft has `B+0xa98` set and `F+0x24c` is clear is not modelled here: the caller passes the record).
fn throttle_gain(b: &dyn Mem, rec: &Record) -> f32 {
    let t = rec.f32(0x4c);
    let (low, high) = (b.f32(0x9d8), b.f32(0x9dc));
    crate::wing_element::interpolate_clamped(0.0, low, 1.0, high, t)
}
