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
