//! The power stage of the piston update (`piston::piston_power`, the first half of `0x14119ac90`) as plain
//! Rust over typed inputs, with no emulated memory. The arithmetic (float32 and float64 mixing, the
//! `minss`/`maxss` operand order) is the original's as the emulator-based code has it; the emulator adapter in
//! [`crate::piston`] calls this function, so the recorded original vectors
//! (`crates/xp-app/tests/data/piston.txt`) check it. The environment calls are made in the original's order.
use crate::scalar::clamp;

fn sse_min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

fn sse_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

fn neg(v: f32) -> f32 {
    f32::from_bits(v.to_bits() ^ 0x8000_0000)
}

/// What the stage asks of its surroundings, called in the original's order: the frame time twice, the
/// `0xa8` binding, the altitude, the temperature, the altitude again, the density, the `0x1a1` binding and,
/// when that is set, two noise samples.
pub trait PowerEnv {
    fn frame_time(&mut self) -> f64;
    /// The input binding `id` for engine index `index`.
    fn bound(&mut self, id: u32, index: i32) -> bool;
    fn altitude(&mut self) -> f32;
    fn temperature(&mut self, altitude: f32) -> f32;
    fn density(&mut self, altitude: f32, outside: f32) -> f32;
    fn noise(&mut self) -> f32;
}

/// Engine and aircraft fields the stage reads (offsets of the original's objects in brackets).
#[derive(Clone, Copy, Debug)]
pub struct PowerInput {
    /// Engine kind `E+0` of engine `n`.
    pub kind: i32,
    /// Engine index `n`.
    pub n: i32,
    /// Targets `M+0x54`, `M+0x5c` and the current values `M+0x58`, `M+0x60`.
    pub target_a: f32,
    pub current_a: f32,
    pub target_b: f32,
    pub current_b: f32,
    /// `M+0x90` speed in percent, `M+0x50`, `M+0x2b4` (kind 1 lean terms), `M+0x264` heat offset.
    pub speed_percent: f32,
    pub m50: f32,
    pub m2b4: f32,
    pub heat_offset: f32,
    /// `B+0x934`, `F+0x424`, `F+0x68`, and `F+0xb7a8 + 4n`.
    pub b934: f32,
    pub f424: f32,
    pub f68: f32,
    pub power_input: f32,
    /// `B+0xc7c..0xc88`.
    pub fold: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerOutput {
    /// New `M+0x58`, `M+0x60`.
    pub current_a: f32,
    pub current_b: f32,
    /// `M+0x24c`, `M+0x254`, `M+0x258`.
    pub power_fraction: f32,
    pub pressure_term: f32,
    pub density_term: f32,
}

pub fn power_terms(i: &PowerInput, env: &mut dyn PowerEnv) -> PowerOutput {
    let step_toward = |env: &mut dyn PowerEnv, target: f32, current: f32| {
        let room = target - current;
        let step = (env.frame_time() * 0.2) as f32;
        let t = if neg(step) > room {
            neg(step)
        } else {
            sse_min(step, room)
        };
        t + current
    };
    let current_a = step_toward(env, i.target_a, i.current_a);
    let current_b = step_toward(env, i.target_b, i.current_b);
    let index = i.n.clamp(0, 1);
    let mut x6 = 0.0f32;
    if !env.bound(0xa8, index) {
        x6 = sse_max(current_a, 0.0);
    }
    let held = sse_max(current_b, x6);
    let d2 = f64::from(i.power_input);
    let d3 = f64::from(held);
    let x0 = i.b934 * i.f424;
    let divisor = sse_max(i.f68, 0.01);
    let power = (d2 * (1.0 - d3)) as f32;
    let ratio = ((f64::from(x0) + f64::from(x0)) / f64::from(divisor) + 1.0) as f32;
    let x0b = i.speed_percent - 0.0;
    let mut x4 = sse_min(1.0, ratio);
    let bound = (ratio - 1.0) / 100.0 * x0b + 1.0;
    if x4 <= bound || x4.is_nan() || bound.is_nan() {
        x4 = sse_min(sse_max(1.0, ratio), bound);
    }
    let mut x3 = ((1.0 - d3 * 0.15) * f64::from(ratio)) as f32;
    if i.kind == 1 {
        x3 = ((1.0 - f64::from(i.m50) * 0.1) * f64::from(x3)) as f32;
    }
    if i.kind == 1 {
        let lean = (1.0 - f64::from(x4 * i.m2b4)) as f32;
        x3 *= clamp(lean, 0.0, 1.0);
    }
    let half = (f64::from(power) * 0.5) as f32;
    let squared = half * half * x4;
    let x3 = x3 * clamp((1.0 - f64::from(squared)) as f32, 0.0, 1.0);
    let power_fraction = x3;
    let mut pressure_term = x3 * i.f68 / 101325.0;
    let fold = |o: [f32; 4]| {
        let a = sse_max(o[0], o[1]);
        let c = sse_max(o[2], o[3]);
        sse_max(a, c)
    };
    let x6 = fold(i.fold);
    let alt = env.altitude();
    let temperature = env.temperature(alt);
    let outside = temperature + x6 * i.heat_offset;
    let alt = env.altitude();
    let density = env.density(alt, outside);
    let mut density_term = density / 1.225 * power_fraction;
    if env.bound(0x1a1, i.n) {
        let v = env.noise();
        pressure_term = ((f64::from(v) * 0.1 + 0.75) * f64::from(pressure_term)) as f32;
        let v = env.noise();
        density_term = ((f64::from(v) * 0.1 + 0.75) * f64::from(density_term)) as f32;
    }
    PowerOutput {
        current_a,
        current_b,
        power_fraction,
        pressure_term,
        density_term,
    }
}

/// What the lever stage asks of its surroundings, in the original's order: the frame time (kinds 3 and 4
/// only) and then whether the throttle ramp limits the lever.
pub trait LeverEnv {
    fn frame_time(&mut self) -> f64;
    fn ramp_limited(&mut self) -> bool;
}

/// Fields of the lever stage (`piston_levers` before the per-kind handlers).
#[derive(Clone, Debug)]
pub struct LeverInput {
    /// Engine kind `E+0`.
    pub kind: i32,
    /// `B+0x93c`, `B+0xa18`, `B+0x9dc`, `B+0xc24`, `B+0x9b8`, `B+0xa9c`.
    pub b93c: f32,
    pub ba18: f32,
    pub b9dc: f32,
    pub bc24: i32,
    pub b9b8: f32,
    pub ba9c: i32,
    /// `M+4` (lever), `M+0x22c` (level), `M+0x228`, `M+0x70`, `M+0x78`, `M+0x90`, `M+0x298`.
    pub lever: f32,
    pub level: f32,
    pub m228: i32,
    pub m70: i32,
    pub m78: f32,
    pub m90: f32,
    pub m298: i32,
    /// `E+0x18`, the throttle ceiling and `engine_ratio` (both read from the objects, no environment call).
    pub e18: f32,
    pub ceiling: f32,
    pub engine_ratio: f32,
    /// `(p+0, p+0x1c)` of every element bound to the engine.
    pub elements: Vec<(f32, f32)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LeverOutput {
    /// New `M+0x22c`.
    pub level: f32,
    /// The commanded lever value after the limits.
    pub x6: f32,
    pub x11: f32,
}

pub fn lever_terms(i: &LeverInput, env: &mut dyn LeverEnv) -> LeverOutput {
    use crate::wing_element::interpolate_clamped;
    let x11 = ((f64::from(i.b93c) - 1.0) / 1.7 + 1.0) as f32;
    let (scale, level);
    if i.kind == 1 || i.kind == 2 || i.kind == 0 {
        scale = if i.kind == 0 {
            0.0
        } else {
            (0.13 / f64::from(x11)) as f32
        };
        level = 1.0;
    } else {
        scale = match i.kind {
            4 => f32::from_bits(0x3e23d70a),
            3 => f32::from_bits(0x3e4ccccd),
            _ => 0.0,
        };
        let mut dt = env.frame_time();
        if i.m228 != 0 {
            dt += dt;
        }
        let v = dt as f32 / i.ba18 + i.level;
        let mut l = if 0.0 > v { 0.0 } else { sse_min(1.0, v) };
        if i.m70 == 0 {
            l = 0.0;
        }
        if i.bc24 != 0 && i.b9b8 > i.m90 {
            l = 0.0;
        }
        level = l;
    }
    let x2 = i.lever;
    let x10 = i.ceiling * scale;
    let x7 = scale * i.b9dc;
    let mut x6 = (x2 - 0.0) * (1.0 - x10) + x10;
    if i.kind.wrapping_sub(3) as u32 <= 1 {
        x6 = sse_max(x2, x10);
        if i.kind == 4 && i.ba9c == 3 && i.m298 == 1 {
            x6 = (x2 - 0.0) * (1.0 - x7) + x7;
        }
    }
    x6 *= level;
    if env.ramp_limited() {
        let e18 = f64::from(i.e18);
        let a0 = (e18 + f64::from_bits(0x4014f1a6e9000000)) as f32;
        let a1 = (e18 + f64::from_bits(0x4024f1a6e9000000)) as f32;
        x6 = interpolate_clamped(a0, x6, a1, x10, i.m78);
    }
    if i.kind == 3 {
        if i.ba9c == 2 {
            let mut low = (f64::from(i.engine_ratio) + f64::from(i.engine_ratio)) as f32;
            let mut high = 0.0f32;
            for &(a, c) in &i.elements {
                low = sse_min(a, low);
                high = sse_max(c, high);
            }
            let exceeded = if i.m298 == 1 {
                let low = sse_min(low, i.engine_ratio);
                f64::from(high) > f64::from(low) * f64::from_bits(0x3ff0cccccccccccd)
            } else if i.m298 >= 2 {
                f64::from(high) > f64::from(i.engine_ratio) * 0.95
            } else {
                false
            };
            if exceeded {
                x6 = x10;
            }
        }
    } else if i.kind == 4 && i.ba9c == 3 {
        let limit = (f64::from(i.engine_ratio) * f64::from_bits(0x3ff0f5c28f5c28f6)) as f32;
        let mut acc = 0.0f32;
        for &(_, c) in &i.elements {
            acc = sse_max(c, acc);
        }
        if acc > limit {
            x6 = x10;
        }
    }
    LeverOutput { level, x6, x11 }
}

/// The calls of the head of the kind 1 and 2 handler, in the original's order: the flight altitude, then the
/// frame time twice (only when the lean/rich response `M+0x6c` is enabled).
pub trait Kind12HeadEnv {
    fn altitude(&mut self) -> f32;
    fn frame_time(&mut self) -> f64;
}

#[derive(Clone, Copy, Debug)]
pub struct Kind12HeadInput {
    /// Engine kind `E+0`, `E+0x14` and `E+0x18` (rated power and speed).
    pub kind: i32,
    pub e14: f32,
    pub e18: f32,
    /// `B+0x93c`, `B+0x9f4`.
    pub b93c: f32,
    pub b9f4: f32,
    /// `M+0x258`, `M+0x78`, `M+0x6c`, `M+0x20c`, `M+0x40`, `M+4`, `M+0x214`, `M+0x64`, `M+0x68`.
    pub m258: f32,
    pub m78: f32,
    pub m6c: i32,
    pub m20c: f32,
    pub m40: f32,
    pub lever: f32,
    pub m214: f32,
    pub m64: f32,
    pub m68: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kind12HeadOutput {
    /// Limited `B+0x93c * M+0x258`.
    pub a10: f32,
    /// Speed over the altitude-dependent rated speed.
    pub d15: f32,
    /// New `M+0x210` (written only when `M+0x6c` is set) and the final `M+0x214`.
    pub m210: Option<f32>,
    pub m214: f32,
}

fn limit_0_10(v: f32) -> f32 {
    if 0.0 > v { 0.0 } else { sse_min(10.0, v) }
}

fn line_between(v0: f32, v1: f32, t_line: f32) -> f32 {
    let lo = sse_min(v0, v1);
    if lo > t_line {
        lo
    } else {
        sse_min(sse_max(v0, v1), t_line)
    }
}

/// The head of `update_engine_kind12` before the running-engine part; the caller then sets `M+0x68 = M+0x64`.
pub fn kind12_head(i: &Kind12HeadInput, env: &mut dyn Kind12HeadEnv) -> Kind12HeadOutput {
    let x0 = i.b93c * i.m258;
    let a10 = if 0.0 > x0 { 0.0 } else { sse_min(1.0, x0) };
    let altitude = env.altitude() - 0.0;
    let w1 = sse_max(
        line_between(i.e14, i.e18, (i.e18 - i.e14) / 10000.0 * altitude + i.e14),
        1.0,
    );
    let d15 = i.m78 / w1;
    let mut m214 = i.m214;
    let mut m210 = None;
    if i.m6c != 0 {
        let clamp_ratio = |q: f32| {
            if 0.001 > q { 0.001 } else { sse_min(1000.0, q) }
        };
        let ca = clamp_ratio(i.m78 / sse_max(i.b9f4, 1.0));
        let cb = clamp_ratio(i.m78 / sse_max(i.e18, 1.0));
        let s11 = (2.0 / f64::from(ca)) as f32;
        let two = (cb - 0.0) + (cb - 0.0);
        let v = 1.0 - two;
        let c3 = if 0.0 > v { 0.0 } else { sse_min(1.0, v) };
        let rate = (f64::from(i.m20c)
            * 0.3
            * f64::from(i.m40)
            * (f64::from(i.lever) * 0.8 + 0.2)
            * f64::from(c3)) as f32;
        m210 = Some(rate);
        let dt = env.frame_time();
        let v = (f64::from(m214) + dt * f64::from(rate)) as f32;
        let held = limit_0_10(v);
        let dt = env.frame_time();
        let v = (f64::from(held) - dt / f64::from(s11)) as f32;
        m214 = limit_0_10(v);
    }
    if i.kind == 1 && i.m68 > i.m64 {
        let t = (i.m68 - i.m64) * 0.3 + m214;
        m214 = limit_0_10(t);
    }
    Kind12HeadOutput {
        a10,
        d15,
        m210,
        m214,
    }
}

/// Calls of the running-engine part of the kind 1 and 2 handler, in the original's order: the ramp bindings
/// (`0x2fb`, `0x239`), the noise binding `0x181` (then the clock and the wave noise), the magneto bindings
/// (`0x151`, `0x159`), and the flight altitude twice with the atmosphere table lookups after each.
pub trait Kind12RunEnv {
    fn bound(&mut self, id: u32, index: i32) -> bool;
    fn altitude(&mut self) -> f32;
    /// The value of the runtime atmosphere table at a position (`table_position` of a value).
    fn table(&mut self, position: f32) -> f32;
    /// The simulation clock in seconds (the global at `0x142f01910`).
    fn clock(&mut self) -> f64;
    /// The wave noise at `t` for engine `n`.
    fn wave_noise(&mut self, t: f32, n: i32) -> f32;
}

#[derive(Clone, Copy, Debug)]
pub struct Kind12RunInput {
    pub kind: i32,
    pub n: i32,
    pub level: f32,
    pub a10: f32,
    pub d15: f32,
    /// `B+0xb6c`, `B+0xab0`, and the `+0x1e4` field of the engine's flight state.
    pub b6c: f32,
    pub bab0: i32,
    pub ramp_flag: i32,
    /// `M+0x214`, `M+0x40`, `M+0` (lever), `M+0x48`, `M+0x258`, `M+0x25c`.
    pub m214: f32,
    pub m40: f32,
    pub m0: f32,
    pub m48: i32,
    pub m258: f32,
    pub m25c: f32,
    /// `B+0xb00..0xb18` (the table positions, the two power pairs and `ff_rat_idle`).
    pub bb00: f32,
    pub bb04: f32,
    pub bb08: f32,
    pub bb0c: f32,
    pub bb10: f32,
    pub bb14: f32,
    pub bb18: f32,
    /// `E+4` and `F+0x28`, `F+0x6880`.
    pub e4: f32,
    pub f28: i32,
    pub f6880: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kind12RunOutput {
    /// New `M+0x40` when the ramp rule set it.
    pub m40: Option<f32>,
    /// New `M+0x240` (= `M+0x244`) and `M+0x248` when the engine is above its threshold.
    pub manifold: Option<(f32, f32)>,
    /// New `M+0xcc`.
    pub power: Option<f32>,
    /// The magneto fuel factor returned to the handler.
    pub fuel_factor: f32,
}

fn table_position(value: f32) -> f32 {
    ((f64::from(value) + 5000.0) / 100.0) as f32
}

pub fn kind12_running_terms(i: &Kind12RunInput, env: &mut dyn Kind12RunEnv) -> Kind12RunOutput {
    use crate::engine::signed_pow;
    use crate::wing_element::interpolate_clamped;
    let k = f64::from(i.b6c) * 125.0;
    let a = f64::from(i.a10) * 0.8;
    let bterm = f64::from(i.d15) * 0.2;
    let breakpoint = |numerator: f64| (((numerator / k) + a) + bterm - 1.0) as f32;
    let (a1, a2, a3, a4, a5) = (
        breakpoint(125.0),
        breakpoint(105.0),
        breakpoint(110.0),
        breakpoint(85.0),
        breakpoint(95.0),
    );
    let mut m40 = i.m40;
    let mut m40_set = None;
    if i.bab0 != 0 && i.ramp_flag != 0 && !env.bound(0x2fb, 0) && !env.bound(0x239, i.n) {
        let l = i.level - 0.25;
        let t = (a1 - a5) / f32::from_bits(0x3f266666) * l + a5;
        let y = line_between(a5, a1, t);
        let v = if 0.0 > y { 0.0 } else { sse_min(1.0, y) };
        m40 = v;
        m40_set = Some(v);
    }
    let s = i.m214 + m40;
    let dd = f64::from(s - a5);
    let u2 = if s > a5 {
        ((dd * -0.9848) * dd - dd * 0.203) as f32
    } else {
        (dd * 3.2) as f32
    };
    let dd2 = f64::from(s - a2);
    let u12 = if s > a2 {
        (0.015 - dd2 * 0.243) as f32
    } else {
        (dd2 * 0.633 + 0.015) as f32
    };
    let x6 = (f64::from(u2) + 1.0) as f32;
    let s1 = signed_pow(i.m0, 0.5);
    let d15d = f64::from(i.d15);
    let inner = ((f64::from(s1) * 0.45 + 0.55) * f64::from(x6)) as f32;
    let v6 = ((0.25 - d15d * 0.25) + f64::from(inner)) as f32;
    let mut v6 = clamp(v6, 0.0, 2.0);
    let e = s - a3;
    let ed = f64::from(e);
    let mut q4 = (ed * 0.1235 - (ed * 1.3826) * ed) as f32;
    if e > 0.0 {
        q4 = sse_max((ed * -0.14) as f32, q4);
    }
    let threshold = match i.kind {
        1 => (f64::from(a5) * 0.95) as f32,
        2 => (f64::from(a4) * 0.95) as f32,
        _ => 0.0,
    };
    let mut x13 = 0.0f32;
    let mut manifold = None;
    if s > threshold {
        let v = (f64::from(q4) + 1.0) as f32;
        if v >= 0.0 || v.is_nan() {
            x13 = sse_min(2.0, v);
        }
        if env.bound(0x181, i.n) {
            let t = (env.clock() * 2.0) as f32;
            let noise = env.wave_noise(t, i.n);
            x13 = ((f64::from(noise) * 0.05 + 0.95) * f64::from(x13)) as f32;
        }
        let mut working = 0;
        if (i.m48.wrapping_sub(2) as u32) <= 1 && !env.bound(0x151, i.n) {
            working = 1;
        }
        if (i.m48.wrapping_sub(1) & !2) == 0 && !env.bound(0x159, i.n) {
            working += 1;
        }
        if working == 0 {
            x13 = (f64::from(x13) * 0.0) as f32;
            v6 = (f64::from(v6) * 0.0) as f32;
        } else if working == 1 {
            x13 = (f64::from(x13) * 0.95) as f32;
            v6 = (f64::from(v6) * 1.04) as f32;
        }
        manifold = Some((v6, (f64::from(u12) + 1.0) as f32));
    }
    let k208 = f32::from_bits(0x3e9c0ebf);
    let r1 = env.altitude();
    let la = env.table(table_position(i.bb04));
    let lb = env.table(table_position(i.bb00));
    let lc = env.table(table_position(r1 * k208));
    let h1 = interpolate_clamped(la, i.bb14, lb, i.bb10, lc);
    let r2 = env.altitude();
    let la = env.table(table_position(i.bb04));
    let lb = env.table(table_position(i.bb00));
    let lc = env.table(table_position(r2 * k208));
    let h2 = interpolate_clamped(la, i.bb0c, lb, i.bb08, lc);
    let h = if a3 == a4 {
        (h2 + h1) * 0.5
    } else {
        (h1 - h2) / (a4 - a3) * (s - a3) + h2
    };
    let v0 = i.e4 * i.bb18 * i.m258;
    let mut power = None;
    if i.f28 != 0 || i.f6880 == 0 {
        let a1b = (f64::from(v0) + f64::from(v0)) as f32;
        let r = interpolate_clamped(0.0, v0, a1b, i.m25c, i.m25c);
        power = Some(r * sse_max(h, 0.0));
    }
    Kind12RunOutput {
        m40: m40_set,
        manifold,
        power,
        fuel_factor: x13,
    }
}
