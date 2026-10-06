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
    /// The global double at `0x142f01918` that the engine fuel check compares `+0x218` with.
    fn thrust_threshold(&mut self) -> f64;
    /// `0x14117c380(F+0xbd00, amount, interval, mode)`: draws fuel from the tanks (state outside the engine).
    fn fuel_draw(&mut self, amount: f32, interval: f32, mode: i32);
    /// `0x14067b2f0`: the next number of the simulation's random generator, in 0..1 (a Mersenne twister).
    fn random_unit(&mut self) -> f32;
}

/// `0x141170810`: the response curve of the aircraft: `100 * (0.01 x)^p` with the sign of `x` and the exponent
/// `B+0x9ac`.
pub fn response_curve(b: &dyn Mem, x: f32) -> f32 {
    let scaled = (f64::from(x) * 0.01) as f32;
    (f64::from(signed_pow(scaled, b.f32(0x9ac))) * 100.0) as f32
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
    descs: &dyn Mem,
    wings: &dyn Mem,
    rec: &mut Record,
    index: usize,
    env: &mut dyn EngineEnv,
) {
    let desc = &Shifted {
        mem: descs,
        base: 0x68 * index,
    };
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
    let gain = throttle_gain(f, b, rec);
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
    let before_running = response;
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

    // second part: the propeller and engine speeds
    let desc_kind = desc.i32(0);
    let (b9b0, b9b4) = (b.f32(0x9b0), b.f32(0x9b4));
    let rpm_now = rec.f32(0x98);
    let rec90 = rec.f32(0x90);
    let curve_90 = crate::wing_element::interpolate_clamped(0.0, b9b0, 100.0, b9b4, rec90);
    let mut pitch = 1.0f32;
    if desc_kind == 6 {
        pitch = b.f32(0x9ac);
    } else if desc_kind == 5 {
        pitch = curve_90;
    }
    let a2c = f64::from(b.f32(0xa2c));
    let rate_a = (100.0 / a2c) as f32;
    let rate_b = (300.0 / a2c) as f32;
    let b9f0 = f64::from(b.f32(0x9f0));
    let loss_a = (b9f0 * 0.0075) as f32;
    let loss_b = (b9f0 * 0.00025) as f32;
    let mixture = f64::from(b.f32(0x940)) + 1.0;
    let mut gain_c = ((1.0 - 1.0 / mixture) * 25.0 + 25.0) as f32;
    let shape_a = crate::wing_element::interpolate_clamped(0.0, 1.0, 2.0, pitch, rpm_now);
    let shape_b = crate::wing_element::interpolate_clamped(0.0, 1.0, 2.0, curve_90, rec90);
    let inverse_exponent = (1.0 / f64::from(exponent)) as f32;
    let scaled_idle = (f64::from(signed_pow(idle, inverse_exponent)) * 100.0) as f32;
    let idle_curve = response_curve(b, scaled_idle);
    let scaled_response = (f64::from(signed_pow(response, inverse_exponent)) * 100.0) as f32;
    let rpm_curve = response_curve(b, rpm_now);
    let mach_scaled = inverse_mach * 50.0;
    gain_c *= inverse_mach;
    let scaled_before = (f64::from(signed_pow(before_running, inverse_exponent)) * 100.0) as f32;
    rec.set_f32(0x28c, response_curve(b, scaled_before));
    let impact = b.f32(0x980) * f.f32(0x400);
    let mut q = (f64::from(impact * impact * f.f32(0x6c)) * 0.5) as f32;
    let f68 = f.f32(0x68);
    q += f68;
    q /= sse_max(f68, 0.01);
    q *= f.f32(0x70);
    let term_168 = (f64::from(signed_pow((f64::from(scaled_response) * 0.01) as f32, shape_a) * q)
        * (f64::from(loss_a) + 1.0)) as f32;
    let term_158 = signed_pow((f64::from(mach_scaled) * 0.01) as f32, shape_a) * q;
    let term_10 = signed_pow((f64::from(rpm_now) * 0.01) as f32, shape_a) * q;
    let sign_a = if 0.0 > rpm_now { -1.0f32 } else { 1.0 } * loss_a;
    let mut engine_speed =
        crate::wing_element::interpolate_clamped(0.0, 1.0, 100.0, 0.0, rpm_now) * term_158;
    engine_speed += term_168;
    engine_speed -= term_10;
    engine_speed -= sign_a;
    engine_speed += thrust_term(
        f,
        b,
        desc,
        &Shifted {
            mem: wings,
            base: 0x3770 * index,
        },
        rec,
        index as i32,
        env,
    );
    let spread = sse_max(rpm_curve - rec90, 0.0) * 0.1f32;
    let term_158b = ((f64::from(signed_pow((f64::from(rpm_curve) * 0.01) as f32, shape_b) * q))
        * (f64::from(loss_b) + 1.0)
        + f64::from(spread * q)) as f32;
    let term_160b = signed_pow((f64::from(gain_c) * 0.01) as f32, shape_b) * q;
    let term_168b = signed_pow((f64::from(rec90) * 0.01) as f32, shape_b) * q;
    let sign_b = if 0.0 > rec90 { -1.0f32 } else { 1.0 } * loss_b;
    let mut prop_speed =
        crate::wing_element::interpolate_clamped(0.0, 1.0, 100.0, 0.0, rec90) * term_160b;
    prop_speed += term_158b;
    prop_speed -= term_168b;
    prop_speed -= sign_b;
    let limit_ratio = rec.f32(0x268) / sse_max(desc.f32(0x20) * f.f32(0x74), 0.01);
    let friction = limit_ratio / sse_max((f64::from(rpm_now) * 0.01) as f32, 0.01);
    let bound = env.binding(0x1b1, index as i32);
    starter_timer(rec, bound, env);
    engine_speed -= friction;
    let lube = {
        let t = 0.0 - (rec.f32(0x2bc) - 1.0 + (rec.f32(0x2bc) - 1.0));
        if 0.0 > t { 0.0 } else { sse_min(2.0, t) }
    };
    let drag_a = {
        let x = (f64::from(rpm_now) * 100.0) as f32;
        if -1.0 > x { -1.0 } else { sse_min(1.0, x) }
    };
    engine_speed -= lube * drag_a;
    let drag_b = {
        let x = (f64::from(rec90) * 100.0) as f32;
        if -1.0 > x { -1.0 } else { sse_min(1.0, x) }
    };
    let factor = rec.f32(0x2c8);
    engine_speed *= factor;
    prop_speed -= lube * drag_b;
    prop_speed *= factor;
    let d64 = desc.f32(0x64);
    let clamp_d = if 0.1 > d64 { 0.1 } else { sse_min(2.0, d64) };
    engine_speed *= if 0.0 > clamp_d {
        f32::NAN
    } else {
        clamp_d.sqrt()
    };
    prop_speed *= if 0.0 > clamp_d {
        f32::NAN
    } else {
        clamp_d.sqrt()
    };
    let dt1 = env.frame_time();
    engine_speed *= rate_a;
    rec.set_f32(
        0x98,
        (f64::from(rec.f32(0x98)) + dt1 * f64::from(engine_speed)) as f32,
    );
    let dt2 = env.frame_time();
    prop_speed *= rate_b;
    let rpm_new = sse_max(rec.f32(0x98), 0.0);
    rec.set_f32(0x98, rpm_new);
    let mut prop_new = sse_max(
        (f64::from(rec.f32(0x90)) + dt2 * f64::from(prop_speed)) as f32,
        0.0,
    );
    rec.set_f32(0x90, prop_new);
    if desc_kind == 5 {
        rec.set_f32(0x90, rpm_new);
        prop_new = rpm_new;
    }
    rec.set_f32(
        0x78,
        (f64::from(prop_new) * 0.01 * f64::from(b.f32(0x96c)) * f64::from(0.104_719_77_f32)) as f32,
    );
    // the original zeroes the propeller speed when it is NaN or infinite
    if !prop_new.is_finite() {
        rec.set_f32(0x90, 0.0);
    }

    // third part: the power from the four engine map points, the thrust and the air flow lag
    if !rec.f32(0x258).is_finite() {
        rec.set_f32(0x258, 0.0);
    }
    let sigma = rec.f32(0x258);
    let d20 = desc.f32(0x20);
    let prop_speed_now = rec.f32(0x90);
    let mach_now = f.f32(0x420);
    let distance = |rpm_at: usize, mach_at: usize| -> f32 {
        let a = (f64::from(b.f32(rpm_at) - prop_speed_now) * 0.01) as f32;
        let m = b.f32(mach_at) - mach_now;
        let sum = m * m + a * a;
        if 0.0 > sum { f32::NAN } else { sum.sqrt() }
    };
    let mut d0 = distance(0xb38, 0xb3c);
    if !d0.is_finite() {
        d0 = 0.0;
    }
    let mut d1 = distance(0xb44, 0xb48);
    if !d1.is_finite() {
        d1 = 0.0;
    }
    let d2 = distance(0xb50, 0xb54);
    let d3 = distance(0xb5c, 0xb60);
    let inverse = |d: f32| (1.0 / f64::from(sse_max(d, 0.001))) as f32;
    let (i0, i1, i2, i3) = (inverse(d0), inverse(d1), inverse(d2), inverse(d3));
    let total = i3 + i2 + i1 + i0;
    let (w0, w1, w2, w3) = (i0 / total, i1 / total, i2 / total, i3 / total);
    let ambient = f.f32(0x424);
    let drag = ((f64::from(-b.f32(0x950)) * 0.3) * f64::from(ambient)) as f32;
    let pitch_curve = engine_curve(b, idle_curve);
    let idle_thrust = (f64::from(pitch_curve * (sigma * d20))
        - (f64::from(b.f32(0x950)) * 0.3) * f64::from(ambient)) as f32;
    let map_a = engine_curve(b, prop_speed_now);
    let map_b = engine_curve(b, gain_c);
    let drag_blend = if map_b == 1.0 {
        (drag + 0.0) * 0.5
    } else {
        ((0.0 - drag) / (1.0 - map_b)) * (map_a - map_b) + drag
    };
    let point = |at: usize, weight: f32| engine_curve(b, b.f32(at)) * ((sigma * d20) * weight);
    // the original adds the terms in this order: the second map point, then the first, the third, the fourth
    let power = {
        let t1 = point(0xb44, w1);
        let t0 = point(0xb38, w0);
        let t2 = point(0xb50, w2);
        let t3 = point(0xb5c, w3);
        t3 + (t2 + (t0 + t1)) + drag_blend
    };
    let q_term = b.f32(0xb64) * b.f32(0xb68) * (sigma * d20);
    let mut r_term = (w1 * b.f32(0xb4c)) * power;
    r_term += (w0 * b.f32(0xb40)) * power;
    r_term += (w2 * b.f32(0xb58)) * power;
    r_term += (b.f32(0xb64) * w3) * power;
    let sd = sigma * d20;
    let thrust_blend = if map_b == 1.0 {
        (sd + drag) * 0.5
    } else {
        (sd - drag) / (1.0 - map_b) * (map_a - map_b) + drag
    };
    rec.set_f32(0x270, thrust_blend);
    let live = f.i32(0x28) != 0 || f.i32(0x6880) == 0;
    if live {
        let lerp = linear(idle_thrust, q_term, drag_blend, r_term, rec.f32(0x270));
        rec.set_f32(0xc4, lerp * rec.i32(0x74) as f32);
    }
    rec.set_f32(0x274, rec.f32(0x270));
    let mut second = if live {
        let v = rec.f32(0xc4);
        rec.set_f32(0xcc, v);
        v
    } else {
        rec.f32(0xcc)
    };
    let first = second;
    let mut third = rec.f32(0x274);
    let fuel = rec.f32(0x234);
    if fuel > 0.0 {
        let extra = fuel * desc.f32(0x24) * rec.f32(0x258);
        third += extra;
        rec.set_f32(0x274, third);
        if live {
            second = extra * b.f32(0xb70) + first;
            rec.set_f32(0xcc, second);
        }
    }
    if live {
        let one = rec.i32(0x70) as f32;
        second *= one;
        rec.set_f32(0xcc, second);
        rec.set_f32(0xc4, one * rec.f32(0xc4));
    }
    rec.set_f32(0x1d8, second / third);
    rec.set_f32(0x1dc, -third / sse_max(b.f32(0x950) * f.f32(0x424), 0.01));
    let mut count = 0.0f32;
    for i in 0..b.i32(0x91c).max(0) as usize {
        let kind = descs.i32(0x68 * i);
        if kind == 5 || kind == 6 {
            count = (f64::from(count) + 1.0) as f32;
        }
    }
    let count = sse_max(count, 1.0);
    let lift = sse_max(rec.f32(0x270), 0.001);
    let ratio = lift / (sse_max(f.f32(0x6c), 0.001) * sse_max(b.f32(0x950), 0.001));
    let root = if ratio >= 0.0 {
        if 0.0 > ratio { f32::NAN } else { ratio.sqrt() }
    } else {
        -(-ratio).sqrt()
    };
    let spread = sse_max(count * root, 0.001);
    let limit = (f64::from(f.f32(0x64b8) / spread) * 1.15) as f32;
    let reduced = if 0.0 > limit {
        0.0
    } else {
        sse_min(lift, limit)
    };
    let mut net = rec.f32(0x274) - reduced;
    rec.set_f32(0x274, net);
    if rec.i32(0x298) == 3 {
        if net > 0.0 {
            net *= -0.5;
            rec.set_f32(0x274, net);
        }
        let reverse = f64::from(b.f32(0x954)) * 1.2 * f64::from(ambient);
        net = (f64::from(net) - reverse) as f32;
        rec.set_f32(0x274, net);
    }
    let dt = env.frame_time() as f32;
    let state = rec.f32(0x278);
    let half = (f64::from(state) * 0.5) as f32;
    let filtered = lag_filter(
        rec.f32(0x274),
        f.f32(0x6c),
        b.f32(0x950),
        f.f32(0x400),
        state,
        half,
        dt,
    );
    rec.set_f32(0x278, filtered);
    let thrust_total = f.f32(0x400) + rec.f32(0x278);
    rec.set_f32(0x27c, thrust_total);
    rec.set_f32(0x25c, thrust_total * rec.f32(0x274));
    let sign = if rec.i32(0x298) == 3 { -1.0f32 } else { 1.0 };
    rec.set_f32(
        0xb0,
        (f64::from(sign * rec.f32(0x274) / b.f32(0x950) / f.f32(0x68)) + 1.0) as f32,
    );
}

/// `0x14081df10`: the linear interpolation between `(a0, v0)` and `(a1, v1)` at `x`, without limits; the mean
/// of the values when `a0 == a1`.
fn linear(a0: f32, v0: f32, a1: f32, v1: f32, x: f32) -> f32 {
    if a0 == a1 {
        (v0 + v1) * 0.5
    } else {
        (v1 - v0) / (a1 - a0) * (x - a0) + v0
    }
}

/// `0x1411e3700`: the engine map curve: `(x / B+0x9a8)` to the power read off a line from `B+0x9b0` at 0 to
/// `B+0x9b4` at 100 (limited to that range); zero when the result is not finite.
fn engine_curve(b: &dyn Mem, x: f32) -> f32 {
    let power = crate::wing_element::interpolate_clamped(0.0, b.f32(0x9b0), 100.0, b.f32(0x9b4), x);
    let r = signed_pow(x / b.f32(0x9a8), power);
    if r.is_finite() { r } else { 0.0 }
}

/// `0x1410c98f0`: one step of the air flow lag: the new value of the state is a blend of the old one and
/// `v` over the reference speed, weighted by the frame time limited to 0..1.
pub fn lag_filter(v: f32, f6c: f32, b950: f32, f400: f32, state: f32, half: f32, dt: f32) -> f32 {
    let reference = {
        let magnitude = sse_max(f400.abs(), ((f64::from(state) * 0.5) as f32).abs());
        let m = sse_max(1.0, magnitude);
        let x = f400 + half;
        let snapped = if -m > x || x > m {
            x
        } else if 0.0 > x {
            -m
        } else {
            m
        };
        snapped.abs()
    };
    let a = dt.clamp(0.0, 1.0);
    let rate = v / (reference * (sse_max(f6c, 0.001) * b950));
    (1.0 - a) * state + a * rate
}

/// `0x1411dd610`: the throttle gain between `B+0x9d8` and `B+0x9dc` for the record's `+0x4c`; the lever is
/// taken as 1.0 when the aircraft has `B+0xa98` set and `F+0x24c` is clear.
fn throttle_gain(f: &dyn Mem, b: &dyn Mem, rec: &Record) -> f32 {
    let mut t = rec.f32(0x4c);
    if b.i32(0xa98) != 0 && f.i32(0x24c) == 0 {
        t = 1.0;
    }
    let (low, high) = (b.f32(0x9d8), b.f32(0x9dc));
    let t = t - 0.0;
    // the clamped interpolation between (0, low) and (1, high), written as the original does
    let value = (high - low) * t + low;
    let lowest = sse_min(low, high);
    if lowest > value {
        lowest
    } else {
        sse_min(sse_max(low, high), value)
    }
}

/// A view of a memory starting at `base`.
struct Shifted<'a> {
    mem: &'a dyn Mem,
    base: usize,
}

impl Mem for Shifted<'_> {
    fn f32(&self, offset: usize) -> f32 {
        self.mem.f32(self.base + offset)
    }
    fn i32(&self, offset: usize) -> i32 {
        self.mem.i32(self.base + offset)
    }
}

/// `0x1411924e0`: the starter delay. Without the start command the delay state `+0x2c8` is reset to 1 and the
/// start flag `+0x2c4` cleared. With it, once `+0x2c8` is above the threshold, the start flag is set (and the
/// state cleared) when the frame time exceeds a random number; otherwise the state moves towards 1 at twice the
/// frame time.
const STARTER_THRESHOLD: f64 = 0.9;

fn starter_timer(rec: &mut Record, start: bool, env: &mut dyn EngineEnv) {
    if !start {
        rec.set_i32(0x2c4, 0);
        rec.set_f32(0x2c8, 1.0);
        return;
    }
    if f64::from(rec.f32(0x2c8)) > STARTER_THRESHOLD {
        let random = f64::from(env.random_unit() + 0.0);
        if env.frame_time() > random {
            rec.set_i32(0x2c4, 1);
            rec.set_f32(0x2c8, 0.0);
            return;
        }
    }
    rec.set_i32(0x2c4, 0);
    let dt = env.frame_time();
    let c = ((dt + dt) as f32).clamp(0.0, 1.0);
    let state = rec.f32(0x2c8);
    rec.set_f32(0x2c8, (1.0 - c) * state + c);
}

fn byte(m: &dyn Mem, offset: usize) -> u32 {
    ((m.i32(offset & !3) as u32) >> (8 * (offset & 3))) & 0xff
}

/// `0x141189dc0`: the tank to draw from among the allowed ones (a bit mask): the one with the highest level
/// that is above the 0.99 offset, the first one when none qualifies.
fn select_tank(f: &dyn Mem, mask: u32) -> usize {
    let mut best = -1.0f32;
    let mut chosen = 0;
    if mask & 1 != 0 {
        let v = f.f32(0xd18);
        if f64::from(v) - 0.99 > -1.0 {
            best = v;
        }
    }
    for k in 1..=5usize {
        if mask & (1 << k) != 0 {
            let v = f.f32(0xd18 + 4 * k);
            if f64::from(v) - 0.99 > f64::from(best) {
                chosen = k;
                best = v;
            }
        }
    }
    chosen
}

/// `0x141238c20`: the thrust term of the engine from its fuel supply, written smoothed to `+0x21c` and returned.
/// Zero (and `+0x21c` cleared) when `+0x218` is not above the global threshold or the cut-off input binding is
/// active. The fuel is drawn from the tanks through the environment.
fn thrust_term(
    f: &dyn Mem,
    b: &dyn Mem,
    desc: &dyn Mem,
    wing: &dyn Mem,
    rec: &mut Record,
    index: i32,
    env: &mut dyn EngineEnv,
) -> f32 {
    if f64::from(rec.f32(0x218)) <= env.thrust_threshold() {
        rec.set_i32(0x21c, 0);
        return 0.0;
    }
    if env.binding(0x1b9, index) {
        rec.set_i32(0x21c, 0);
        return 0.0;
    }
    let mut level = b.f32(0xc18);
    if env.binding(0x1c9, index) {
        let t = (rec.f32(0x90) - 0.0) * 0.1f32;
        let u = 1.0 - t;
        level *= if 0.0 > u { 0.0 } else { sse_min(1.0, u) };
    }
    let speed = match desc.i32(0) {
        1..=3 => (f64::from(rec.f32(0x90)) * 0.01) as f32,
        4..=6 => (f64::from(rec.f32(0x98)) * 0.01) as f32,
        _ => 0.0,
    };
    let flag = byte(f, 0x58d);
    let reference = b.f32(0xc1c);
    let mut limit = f64::from(reference);
    if flag == 1 {
        limit *= 1.7;
    }
    let limit = limit as f32;
    let ratio = limit / sse_max(speed, limit);
    level *= ratio * ratio;
    if b.i32(0xc20) != 0 {
        let (tank_level, mode) = if b.i32(4) > 0x1bb5c {
            let w = wing.f32(0x790);
            if 0.0 > w {
                (f.f32(0xbd2c), 1)
            } else if w > 0.0 {
                (f.f32(0xbd30), 3)
            } else {
                (f.f32(0xbd28), 2)
            }
        } else {
            (f.f32(0xbd28), 2)
        };
        let fraction = if 0.0 > tank_level {
            0.0
        } else {
            sse_min(1.0, tank_level)
        };
        let heights = sse_max(
            sse_max(b.f32(0xc7c), b.f32(0xc80)),
            sse_max(b.f32(0xc84), b.f32(0xc88)),
        );
        let scaled_speed = speed / reference;
        level *= fraction;
        let amount = (f64::from(b.f32(0xc7c) / heights) * (1.0 - f64::from(scaled_speed))) as f32;
        env.fuel_draw(amount, 1.0, mode);
    } else {
        let tank = select_tank(f, byte(b, 0x2b51 + index as usize));
        let tank_ratio = f.f32(0xd18 + 4 * tank) / b.f32(0x1b94);
        let squared = tank_ratio * tank_ratio;
        let fraction = if 0.0 > squared {
            0.0
        } else {
            sse_min(1.2, squared)
        };
        level *= fraction;
        // the original also passes the flow request (`level / B+0xc18`, doubled for flag 1) with this query
        let _ = env.binding(0x1b9, index);
    }
    let smoothed = level * 0.7f32 + rec.f32(0x21c) * 0.3f32;
    rec.set_f32(0x21c, smoothed);
    level
}

/// Inputs of [`engine_held_back`] read from the objects: `kind` is the engine record's word at `+0`, `lever` is
/// `F+0x64b4`, `limit_low`/`limit_high` the record's floats at `+0x790`/`+0x798`.
#[derive(Clone, Copy)]
pub struct HoldInputs {
    pub kind: i32,
    pub lever: f32,
    pub limit_low: f32,
    pub limit_high: f32,
    pub index: i32,
    pub mode: i32,
}

/// `0x1411d9f60`: whether engine `index` is held back for input mode `mode` (1 or 2 select the two directions
/// of the `0x179`/`0x1f9` binding pair). `binding(id, index)` is the input query `0x1407ace10` (its other
/// arguments are the fixed 1 and the float `0x1425036a4`), `fallback(index, mode)` the function `0x140822620`
/// that decides when no rule applies.
pub fn engine_held_back(
    h: HoldInputs,
    mut binding: impl FnMut(u32, i32) -> bool,
    fallback: impl FnOnce(i32, i32) -> i32,
) -> i32 {
    if h.kind == 6 && h.lever == 0.0 {
        return 0;
    }
    let (low, high) = (h.limit_low, h.limit_high);
    if binding(0x179, h.index) && h.mode == 2 {
        return 0;
    }
    if binding(0x1f9, h.index) && h.mode == 1 {
        return 0;
    }
    if binding(0x2f6, 0) && low < 0.0 && high < 0.0 {
        return 0;
    }
    if binding(0x2f7, 0) && low > 0.0 && high < 0.0 {
        return 0;
    }
    if binding(0x2f8, 0) && low < 0.0 && high > 0.0 {
        return 0;
    }
    if binding(0x2f9, 0) && low > 0.0 && high > 0.0 {
        return 0;
    }
    fallback(h.index, h.mode)
}

/// `0x1411d9ec0`: the enabled flag of record `index` of the table at `B+0x6040` (stride `0x34c8`): zero when the
/// record's mode word (`+0x5f0`) is at most `0x26` and the binding `0x179` queried with that mode is set, or when
/// the record's word at `+0x54` is nonzero; otherwise the byte at `+0x588`.
pub fn record_flag_6040(
    mode: u32,
    blocked_word: i32,
    flag: u8,
    mut binding: impl FnMut(u32, i32) -> bool,
) -> u8 {
    if (mode <= 0x26 && binding(0x179, mode as i32)) || blocked_word != 0 {
        return 0;
    }
    flag
}

/// `maxss`/`minss` operand semantics: the second operand wins when either is NaN or both are equal.
fn max_ss(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

fn min_ss(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// `0x14121b4d0`: `r70 - cos(a14 * rad) * (r18 - (1 - a10) * r20) * cos(a18 * rad)` in double precision, with the
/// two cosines taken in float32 (`0x14230b380` is `cosf`). `a` are the floats of the object at `+0x10/+0x14/+0x18`
/// and `r` the floats of the object it points to at `+0x18/+0x20/+0x70`.
pub fn cosine_blend(a10: f32, a14: f32, a18: f32, r18: f32, r20: f32, r70: f32) -> f64 {
    const RAD: f32 = f32::from_bits(0x3c8efa36);
    let first = f64::from((a14 * RAD).cos());
    let second = (a18 * RAD).cos();
    let inner = f64::from(r18) - (1.0 - f64::from(a10)) * f64::from(r20);
    f64::from(r70) - first * inner * f64::from(second)
}

/// `0x1411b5ee0`: `2 * sqrt(v10 / pi) / max(record - f664, 0.01)`, held to `0..=1` (`v10` is the object's float at
/// `+0x10`, `record` the float at `+0x58c + index * 0xd8` with the index at `+0x654`, `f664` the float at `+0x664`).
pub fn root_ratio(v10: f32, record: f32, f664: f32) -> f32 {
    let root = f64::from((v10 / f32::from_bits(0x40490fdb)).sqrt());
    let divisor = f64::from(max_ss(record - f664, f32::from_bits(0x3c23d70a)));
    let ratio = (root * 2.0 / divisor) as f32;
    if 0.0 > ratio {
        return 0.0;
    }
    min_ss(1.0, ratio)
}

/// `0x1411da150`: the byte at `+0x678` of record `index` of the `B+0x6028` table (stride `0x36c8`), or zero when
/// the binding `0x251` queried with the index is set.
pub fn record_flag_6028(flag: u8, bound: bool) -> u8 {
    if bound { 0 } else { flag }
}
