//! The update of an engine of kinds 0 to 4 (`0x14119ac90(M, F, n, inputs)`): the manifold and density terms, the
//! power fraction, the throttle lever with its limits, the per-kind handlers, and the power from the altitude
//! and temperature tables. Objects as in [`crate::controls`]; `inputs` is the float array indexed by `B+0xd38+4n`.
//! Compared with the original on random objects at absolute addresses (`tools/gen_piston_vectors.py`).
use crate::callees::engine_ratio;
use crate::controls::ATMOSPHERE_TABLE;
use crate::engine::{curve, signed_pow};
use crate::engine_env::{starter_delay, starter_ready_of, thrust_term_of};
use crate::scalar::clamp;
use crate::vm::{CallArgs, Callees, Vm};
use crate::wing_element::interpolate_clamped;

/// `minss dst, src`: the first operand when it is smaller, otherwise the second.
fn sse_min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// `maxss dst, src`: the first operand when it is larger, otherwise the second.
fn sse_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

fn neg(v: f32) -> f32 {
    f32::from_bits(v.to_bits() ^ 0x8000_0000)
}

const FRAME_TIME_OWNER: u64 = 0x142f_018b8;
const ENGINE_FLAG_OWNER: u64 = 0x1424_f5648;
const SEED_GLOBAL: u64 = 0x142f_01918;

fn float_args(values: &[f32]) -> CallArgs {
    let mut a = CallArgs::default();
    for (i, v) in values.iter().enumerate() {
        a.xmm[i] = Some(v.to_bits());
    }
    a
}

fn reply_f32(r: crate::vm::Reply) -> f32 {
    f32::from_bits(r.xmm0 as u32)
}

/// `0x140c448c0`: the frame time in seconds.
fn frame_time(env: &mut dyn Callees) -> f64 {
    let args = CallArgs::ints(&[FRAME_TIME_OWNER]);
    f64::from_bits(env.call(&mut Vm::default(), 0x140c448c0, args).xmm0)
}

/// `0x1407ace10(F, 1, id, index, 1.0)`: an input-binding query.
fn bind(env: &mut dyn Callees, f: u64, id: u32, index: i32) -> bool {
    let args = CallArgs::ints(&[f, 1, u64::from(id), index as u32 as u64]);
    env.call(&mut Vm::default(), 0x1407ace10, args).rax as u32 != 0
}

/// The altitude argument of the temperature and density queries: `0` when `0x1417f12c0` is set, else `F+0x3a0`
/// narrowed to float32.
fn altitude(vm: &mut Vm, env: &mut dyn Callees, f: u64) -> f32 {
    let flag = env
        .call(vm, 0x1417f12c0, CallArgs::ints(&[ENGINE_FLAG_OWNER]))
        .rax as u8
        != 0;
    let d = if flag { 0.0 } else { vm.f64(f + 0x3a0) };
    d as f32
}

/// The position `(value + 5000) / 100` in the runtime atmosphere table, as the float32 the original computes.
fn table_position(value: f32) -> f32 {
    ((f64::from(value) + 5000.0) / 100.0) as f32
}

/// `cvttss2si`: out-of-range and NaN values give `i32::MIN`.
fn truncate(x: f32) -> i32 {
    if !(-2147483648.0..2147483648.0).contains(&x) {
        i32::MIN
    } else {
        x as i32
    }
}

fn table_lookup(vm: &Vm, position: f32) -> f32 {
    let index = truncate(position).clamp(0, 0x801);
    let v0 = vm.f32(ATMOSPHERE_TABLE + 8 * index as u64);
    let v1 = vm.f32(ATMOSPHERE_TABLE + 8 * (index as u64 + 1));
    interpolate_clamped(index as f32, v0, (index + 1) as f32, v1, position)
}

/// The `maxss` folds over the elements bound to engine `n` (`B+0xb7c+4n` equal to `B+0xbbc+4j`); `visit(N+j*0x388)`.
fn each_bound_element(vm: &Vm, f: u64, b: u64, n: i32, mut visit: impl FnMut(u64)) {
    let count = i64::from(vm.i32(b + 0x920));
    if count <= 0 {
        return;
    }
    let want = vm.i32(b + 0xb7c + 4 * n as i64 as u64);
    for j in 0..count {
        if vm.i32(b + 0xbbc + 4 * j as u64) == want {
            visit(vm.u64(f + 0x68c8) + (j * 0x388) as u64);
        }
    }
}

/// `0x14119ac90(M, F, n, inputs)`; returns the value the original leaves in `xmm0` (`M+0x1d8`).
pub fn update_engine_piston(
    vm: &mut Vm,
    env: &mut dyn Callees,
    state: u64,
    f: u64,
    n: i32,
    inputs: u64,
) -> f32 {
    let b = vm.u64(f + 0x20);
    let engine = |vm: &Vm| vm.u64(b + 0x5ff8) + (i64::from(n) * 0x68) as u64;
    let kind = vm.i32(engine(vm));
    let m = state;
    if kind == 0 {
        vm.set_f32(m + 0x24c, 1.0);
        vm.set_f32(m + 0x254, 1.0);
        vm.set_f32(m + 0x258, 1.0);
    } else {
        piston_power(vm, env, m, f, b, n);
    }
    piston_levers(vm, env, m, f, b, n, inputs)
}

/// The first part of the update, for engines of a kind other than 0: the lag towards the commanded values
/// `M+0x58/0x60` and the power, density and manifold terms `M+0x24c/0x254/0x258`.
fn piston_power(vm: &mut Vm, env: &mut dyn Callees, m: u64, f: u64, b: u64, n: i32) {
    let step_toward = |vm: &mut Vm, env: &mut dyn Callees, target: u64, current: u64| {
        let current_value = vm.f32(current);
        let room = vm.f32(target) - current_value;
        let step = (frame_time(env) * 0.2) as f32;
        let t = if neg(step) > room {
            neg(step)
        } else {
            sse_min(step, room)
        };
        vm.set_f32(current, t + current_value);
    };
    step_toward(vm, env, m + 0x54, m + 0x58);
    step_toward(vm, env, m + 0x5c, m + 0x60);
    let index = n.clamp(0, 1);
    let mut x6 = 0.0f32;
    if !bind(env, f, 0xa8, index) {
        x6 = sse_max(vm.f32(m + 0x58), 0.0);
    }
    let held = sse_max(vm.f32(m + 0x60), x6);
    let d2 = f64::from(vm.f32(f + 0xb7a8 + 4 * n as i64 as u64));
    let d3 = f64::from(held);
    let x0 = vm.f32(b + 0x934) * vm.f32(f + 0x424);
    let divisor = sse_max(vm.f32(f + 0x68), 0.01);
    let power = (d2 * (1.0 - d3)) as f32;
    let ratio = ((f64::from(x0) + f64::from(x0)) / f64::from(divisor) + 1.0) as f32;
    let x0b = vm.f32(m + 0x90) - 0.0;
    let mut x4 = sse_min(1.0, ratio);
    let bound = (ratio - 1.0) / 100.0 * x0b + 1.0;
    if x4 <= bound || x4.is_nan() || bound.is_nan() {
        x4 = sse_min(sse_max(1.0, ratio), bound);
    }
    let mut x3 = ((1.0 - d3 * 0.15) * f64::from(ratio)) as f32;
    vm.set_f32(m + 0x24c, x3);
    let is_kind1 = |vm: &Vm| vm.i32(vm.u64(b + 0x5ff8) + (i64::from(n) * 0x68) as u64) == 1;
    if is_kind1(vm) {
        x3 = ((1.0 - f64::from(vm.f32(m + 0x50)) * 0.1) * f64::from(x3)) as f32;
        vm.set_f32(m + 0x24c, x3);
    }
    if is_kind1(vm) {
        let lean = (1.0 - f64::from(x4 * vm.f32(m + 0x2b4))) as f32;
        x3 *= clamp(lean, 0.0, 1.0);
    }
    let half = (f64::from(power) * 0.5) as f32;
    let squared = half * half * x4;
    let x3 = x3 * clamp((1.0 - f64::from(squared)) as f32, 0.0, 1.0);
    vm.set_f32(m + 0x24c, x3);
    let x254 = x3 * vm.f32(f + 0x68) / 101325.0;
    vm.set_f32(m + 0x254, x254);
    let fold = |vm: &Vm, offsets: [u64; 4]| {
        let a = sse_max(vm.f32(b + offsets[0]), vm.f32(b + offsets[1]));
        let c = sse_max(vm.f32(b + offsets[2]), vm.f32(b + offsets[3]));
        sse_max(a, c)
    };
    let x6 = fold(vm, [0xc7c, 0xc80, 0xc84, 0xc88]);
    let alt = altitude(vm, env, f);
    let args = {
        let mut a = float_args(&[0.0, alt]);
        a.int[0] = Some(f + 0xbfa8);
        a.xmm[0] = None;
        a
    };
    let temperature = reply_f32(env.call(vm, 0x141ba6750, args));
    let outside = temperature + x6 * vm.f32(m + 0x264);
    let alt = altitude(vm, env, f);
    let mut a = float_args(&[0.0, alt, outside]);
    a.int[0] = Some(f + 0xbfa8);
    a.xmm[0] = None;
    let density = reply_f32(env.call(vm, 0x141ba64e0, a));
    let x258 = density / 1.225 * vm.f32(m + 0x24c);
    vm.set_f32(m + 0x258, x258);
    if bind(env, f, 0x1a1, n) {
        let seed = vm.f64(SEED_GLOBAL) as f32;
        let noise = |vm: &mut Vm, env: &mut dyn Callees| {
            let mut a = float_args(&[seed, 0.0]);
            a.int[2] = Some(0);
            reply_f32(env.call(vm, 0x140984e50, a))
        };
        let v = noise(vm, env);
        let scaled = ((f64::from(v) * 0.1 + 0.75) * f64::from(vm.f32(m + 0x254))) as f32;
        vm.set_f32(m + 0x254, scaled);
        let v = noise(vm, env);
        let scaled = ((f64::from(v) * 0.1 + 0.75) * f64::from(vm.f32(m + 0x258))) as f32;
        vm.set_f32(m + 0x258, scaled);
    }
}

/// The second part: the lever level of the engine's kind, the limits against the bound elements, the per-kind
/// handlers, and the power from the tables.
fn piston_levers(
    vm: &mut Vm,
    env: &mut dyn Callees,
    m: u64,
    f: u64,
    b: u64,
    n: i32,
    inputs: u64,
) -> f32 {
    let engine = |vm: &Vm| vm.u64(b + 0x5ff8) + (i64::from(n) * 0x68) as u64;
    let kind0 = vm.i32(engine(vm));
    let x11 = ((f64::from(vm.f32(b + 0x93c)) - 1.0) / 1.7 + 1.0) as f32;
    let scale;
    if kind0 == 1 || kind0 == 2 || kind0 == 0 {
        // kinds 1 and 2 take the scale from the manifold term; kind 0 has none; all three set the level to 1
        scale = if kind0 == 0 {
            0.0
        } else {
            (0.13 / f64::from(x11)) as f32
        };
        vm.set_f32(m + 0x22c, 1.0);
    } else {
        let seed = match kind0 {
            4 => f32::from_bits(0x3e23d70a),
            3 => f32::from_bits(0x3e4ccccd),
            _ => 0.0,
        };
        scale = seed;
        let mut dt = frame_time(env);
        if vm.i32(m + 0x228) != 0 {
            dt += dt;
        }
        let v = dt as f32 / vm.f32(b + 0xa18) + vm.f32(m + 0x22c);
        let level = if 0.0 > v { 0.0 } else { sse_min(1.0, v) };
        vm.set_f32(m + 0x22c, level);
        if vm.i32(m + 0x70) == 0 {
            vm.set_i32(m + 0x22c, 0);
        }
        if vm.i32(b + 0xc24) != 0 && vm.f32(b + 0x9b8) > vm.f32(m + 0x90) {
            vm.set_i32(m + 0x22c, 0);
        }
    }
    let gain = reply_f32(env.call(vm, 0x1411dd610, CallArgs::ints(&[f, u64::from(n as u32)])));
    let x2 = vm.f32(m + 4);
    let x10 = gain * scale;
    let x7 = scale * vm.f32(b + 0x9dc);
    let mut x6 = (x2 - 0.0) * (1.0 - x10) + x10;
    let kind = vm.i32(engine(vm));
    if kind.wrapping_sub(3) as u32 <= 1 {
        x6 = sse_max(x2, x10);
        if kind == 4 && vm.i32(b + 0xa9c) == 3 && vm.i32(m + 0x298) == 1 {
            x6 = (x2 - 0.0) * (1.0 - x7) + x7;
        }
    }
    x6 *= vm.f32(m + 0x22c);
    let limited = engine_ramp_limited(vm, env, f, n);
    if limited {
        let e18 = f64::from(vm.f32(engine(vm) + 0x18));
        let a0 = (e18 + f64::from_bits(0x4014f1a6e9000000)) as f32;
        let a1 = (e18 + f64::from_bits(0x4024f1a6e9000000)) as f32;
        x6 = interpolate_clamped(a0, x6, a1, x10, vm.f32(m + 0x78));
    }
    let kind = vm.i32(engine(vm));
    if kind == 3 {
        if vm.i32(b + 0xa9c) == 2 {
            x6 = lever_limit_kind3(vm, f, b, n, m, x6, x10);
        }
    } else if kind == 4 && vm.i32(b + 0xa9c) == 3 {
        let limit = (f64::from(engine_ratio(vm, b, n)) * f64::from_bits(0x3ff0f5c28f5c28f6)) as f32;
        let mut acc = 0.0f32;
        each_bound_element(vm, f, b, n, |p| acc = sse_max(vm.f32(p + 0x1c), acc));
        if acc > limit {
            x6 = x10;
        }
    }
    vm.set_i32(m + 0x210, 0);
    let call_void = |vm: &mut Vm, env: &mut dyn Callees, address: u64, value: f32| {
        let mut a = CallArgs::ints(&[m, b, f, u64::from(n as u32)]);
        a.stack[0] = Some(u64::from(value.to_bits()));
        env.call(vm, address, a);
    };
    if vm.i32(engine(vm)) == 0 {
        let slot = i64::from(vm.i32(b + 0xd38 + 4 * n as i64 as u64));
        let input = vm.f32((inputs as i64 + slot * 4) as u64);
        call_void(vm, env, 0x14119bc00, x6 * input);
    }
    let scaled = x6 * x11;
    if vm.i32(engine(vm)) == 1 {
        call_void(vm, env, 0x14119d380, scaled);
    }
    if vm.i32(engine(vm)) == 2 {
        call_void(vm, env, 0x14119d380, scaled);
    }
    if vm.i32(engine(vm)) == 4 {
        update_engine_kind4(vm, env, m, b, f, n, x6);
    }
    if vm.i32(engine(vm)) == 3 {
        update_engine_kind3(vm, env, m, b, f, n, x6);
    }
    if vm.i32(f + 0x28) != 0 || vm.i32(f + 0x6880) == 0 {
        let kind = vm.i32(engine(vm));
        if kind.wrapping_sub(3) as u32 <= 1 {
            power_from_tables(vm, env, m, f, b, n);
        }
    }
    let out = vm.f32(m + 0xcc) / vm.f32(m + 0x25c);
    vm.set_f32(m + 0x1d8, out);
    out
}

/// The limit of the kind 3 engine (for `B+0xa9c == 2`): the lever value `x6`, or `x10` when the bound elements'
/// extremes exceed the engine's ratio by more than the margin of the mode `M+0x298`.
fn lever_limit_kind3(vm: &Vm, f: u64, b: u64, n: i32, m: u64, x6: f32, x10: f32) -> f32 {
    let ratio = engine_ratio(vm, b, n);
    let mut low = (f64::from(ratio) + f64::from(ratio)) as f32;
    let mut high = 0.0f32;
    each_bound_element(vm, f, b, n, |p| {
        let a = vm.f32(p);
        let c = vm.f32(p + 0x1c);
        low = sse_min(a, low);
        high = sse_max(c, high);
    });
    let mode = vm.i32(m + 0x298);
    let exceeded = if mode == 1 {
        let limit = engine_ratio(vm, b, n);
        let low = sse_min(low, limit);
        f64::from(high) > f64::from(low) * f64::from_bits(0x3ff0cccccccccccd)
    } else if mode >= 2 {
        f64::from(high) > f64::from(engine_ratio(vm, b, n)) * 0.95
    } else {
        false
    };
    if exceeded { x10 } else { x6 }
}

/// The power written to `M+0xcc` for kinds 3 and 4 from the altitude/temperature tables (`B+0xb1c..0xb34`) and
/// the engine speed `M+0x90`.
fn power_from_tables(vm: &mut Vm, env: &mut dyn Callees, m: u64, f: u64, b: u64, n: i32) {
    let engine = vm.u64(b + 0x5ff8) + (i64::from(n) * 0x68) as u64;
    let alt = altitude(vm, env, f);
    let l1 = table_lookup(vm, table_position(vm.f32(b + 0xb20)));
    let l2 = table_lookup(vm, table_position(vm.f32(b + 0xb1c)));
    let l3 = table_lookup(vm, table_position(alt));
    let high = interpolate_clamped(l1, vm.f32(b + 0xb30), l2, vm.f32(b + 0xb2c), l3);
    let alt = altitude(vm, env, f);
    let l1 = table_lookup(vm, table_position(vm.f32(b + 0xb20)));
    let l2 = table_lookup(vm, table_position(vm.f32(b + 0xb1c)));
    let l3 = table_lookup(vm, table_position(alt));
    let speed = vm.f32(m + 0x90);
    let low = interpolate_clamped(l1, vm.f32(b + 0xb28), l2, vm.f32(b + 0xb24), l3);
    let c1 = curve(80.0, high, 95.0, low, speed, f32::from_bits(0x3eaaa64c));
    let c2 = curve(
        0.0,
        vm.f32(engine + 8) * vm.f32(m + 0x258) * vm.f32(b + 0xb34),
        80.0,
        vm.f32(m + 0x25c),
        speed,
        2.0,
    );
    let running = vm.i32(m + 0x74) as f32 * vm.f32(m + 0x22c);
    vm.set_f32(m + 0xcc, c1 * c2 * running);
}

/// `0x14119c610(M, B, F, n, level)`: the kind 4 handler: the engine's torque from the speed `M+0x78` against its
/// rated speed (`E+0x18`), the thrust term of the fuel supply, the starter delay, the noise, the speeds
/// `M+0x90/0x98` and, for a running engine, the manifold-pressure terms `M+0x240/0x244/0x248`.
pub fn update_engine_kind4(
    vm: &mut Vm,
    env: &mut dyn Callees,
    m: u64,
    b: u64,
    f: u64,
    n: i32,
    level: f32,
) {
    let engine = |vm: &Vm| vm.u64(b + 0x5ff8) + (i64::from(n) * 0x68) as u64;
    let rated = sse_max(vm.f32(engine(vm) + 0x18), 1.0);
    let x6 = vm.f32(m + 0x78) / rated;
    let root = signed_pow(x6, 0.5);
    let m258 = f64::from(vm.f32(m + 0x258));
    let running = vm.i32(m + 0x74);
    let h = (f64::from(root) * 1.26 * m258 * f64::from(running) * f64::from(level)) as f32;
    let squared = signed_pow(x6, 2.0);
    let w = (f64::from(squared) * 0.25 * m258) as f32;
    let s = (f64::from(x6) * 100.0) as f32;
    let u = if -1.0 > s { -1.0 } else { sse_min(1.0, s) };
    let v6 = ((f64::from(vm.f32(b + 0x9f0)) * 0.02 + 1.0 - f64::from(vm.f32(m + 0x2bc)))
        * f64::from(u)) as f32;
    let thrust = thrust_term_of(vm, env, f, n);
    let power = thrust + (sse_max(0.0, h) - w - v6);
    vm.set_f32(m + 0xb8, power * vm.f32(engine(vm) + 0x10));
    let start = bind(env, f, 0x1b1, n);
    starter_delay(vm, env, f, m, start);
    let speed = sse_max(vm.f32(m + 0x78), 0.01);
    let drag = vm.f32(m + 0x268) / speed;
    let start_state = vm.f32(m + 0x2c8);
    let ramp = if 1.0 > start_state {
        vm.i32(m + 0x74) as f32 * start_state
    } else {
        1.0
    };
    let b8 = ramp * (vm.f32(m + 0xb8) - drag);
    vm.set_f32(m + 0xb8, b8);
    if bind(env, f, 0x181, n) {
        let t = (vm.f64(0x142f01910) * 2.0) as f32;
        let mut a = CallArgs::ints(&[0, u64::from(n as u32)]);
        a.xmm[0] = Some(t.to_bits());
        a.int[0] = None;
        let noise = reply_f32(env.call(vm, 0x1408bd9d0, a));
        let v = ((f64::from(noise) * 0.05 + 0.95) * f64::from(vm.f32(m + 0xb8))) as f32;
        vm.set_f32(m + 0xb8, v);
    }
    let rated_ratio = vm.f32(engine(vm) + 0x64);
    let g = if 0.1 > rated_ratio {
        0.1
    } else {
        sse_min(2.0, rated_ratio)
    };
    let sqrt = g.sqrt();
    let s_pos = sse_max(s, 0.0);
    let b8 = sqrt * vm.f32(m + 0xb8);
    vm.set_f32(m + 0x90, s_pos);
    vm.set_f32(m + 0x98, s_pos);
    vm.set_f32(m + 0xb8, b8);
    if running == 0 {
        return;
    }
    let low = f64::from(vm.f32(b + 0x9c4));
    let q = ((f64::from(x6) * 100.0 - low) / (100.0 - low)) as f32;
    let q3 = signed_pow(q, 3.0);
    let lever = running as f32 * level;
    let lever_a = signed_pow(lever, 0.1);
    let lever_b = signed_pow(lever, 5.0);
    let blend = (f64::from(lever_b) * 0.3 + f64::from(lever_a) * 1.7) as f32;
    let m258 = vm.f32(m + 0x258);
    let blend = (blend - q3) / vm.f32(m + 0x24c);
    let density = sse_max(signed_pow(m258, 0.25), 0.01);
    let o = sse_max(0.0, blend / density);
    vm.set_f32(m + 0x240, o);
    vm.set_f32(m + 0x244, o);
    let x0 =
        (vm.f32(m + 0x25c) / vm.f32(engine(vm) + 4) - 0.05) * f32::from_bits(0x3f4a1af3) + 0.25;
    let y = if 0.25 > x0 { 0.25 } else { sse_min(1.0, x0) };
    vm.set_f32(m + 0x248, y * o);
}

/// `0x1411a2e40(F, n)`: whether the engine's lever is limited by its ramp: the aircraft has `B+0xaac` set, the
/// engine's state has `+0x1e4` set, and neither input binding (`0x2fb` for index 0, `0x239` for the engine) is active.
pub fn engine_ramp_limited(vm: &Vm, env: &mut dyn Callees, f: u64, n: i32) -> bool {
    let b = vm.u64(f + 0x20);
    if vm.i32(b + 0xaac) == 0 {
        return false;
    }
    let state = vm.u64(f + 0x68b0) + (i64::from(n) * 0x2cc) as u64;
    if vm.i32(state + 0x1e4) == 0 {
        return false;
    }
    !bind(env, f, 0x2fb, 0) && !bind(env, f, 0x239, n)
}

/// `0x14119cb70(M, B, F, n, level)`: the kind 3 handler (a carburetted engine with a variable load): the
/// lever limited by the starter and the throttle ramp, the manifold terms `M+0x240/0x244/0x248`, the load terms
/// from the speed, the fuel thrust term, the integration of the engine speed `M+0x90` by the frame time, the speed
/// ratio `M+0x98` and the power `M+0xb8`.
pub fn update_engine_kind3(
    vm: &mut Vm,
    env: &mut dyn Callees,
    m: u64,
    b: u64,
    f: u64,
    n: i32,
    level: f32,
) {
    let engine = |vm: &Vm| vm.u64(b + 0x5ff8) + (i64::from(n) * 0x68) as u64;
    let running = vm.i32(m + 0x74);
    let mut level = level;
    if running != 0 {
        let speed = (f64::from(vm.f32(m + 0x90)) * 0.01) as f32;
        let v0 = (f64::from(vm.f32(b + 0x9d8)) * 0.2) as f32;
        let a0 = (f64::from(vm.f32(b + 0x9c4)) * 0.01) as f32;
        let mut k = curve(a0, v0, 1.0, 1.0, speed, 1.75);
        if starter_ready_of(vm, env, f, n) {
            let d4 = vm.f32(b + 0x1a74) / vm.f32(b + 0x1a80);
            let low = sse_min(1.0, d4);
            let t = (d4 - 1.0) * (level - 0.0) + 1.0;
            let mut c = if low > t {
                low
            } else {
                sse_min(sse_max(1.0, d4), t)
            };
            c *= k;
            if 0.0 > level {
                level = 0.0;
            } else if level > c {
                level = c;
            }
        }
        k = level / k;
        let e4 = vm.f32(engine(vm) + 4);
        let manifold =
            (f64::from(k) * 1.05 / ((f64::from(vm.f32(m + 0x24c)) - 1.0) * 0.25 + 1.0)) as f32;
        vm.set_f32(m + 0x244, manifold);
        vm.set_f32(m + 0x240, manifold);
        let x1 = (vm.f32(m + 0x25c) / e4 - 0.05) * f32::from_bits(0x3f4a1af3) + 0.25;
        let y = if 0.25 > x1 { 0.25 } else { sse_min(1.0, x1) };
        vm.set_f32(m + 0x248, y * manifold);
    }
    let r13 = running as f32 * vm.f32(m + 0x22c);
    let feedback = vm.f32(b + 0x9d0);
    if feedback > 0.0 && f64::from(vm.f32(m + 0x22c)) > 0.99 {
        let t = (vm.f32(m + 0x90) / feedback - 0.5) * ((level - r13) + (level - r13)) + r13;
        let low = sse_min(r13, level);
        level = if low > t {
            low
        } else {
            sse_min(sse_max(r13, level), t)
        };
    }
    let p1 = signed_pow(level, f32::from_bits(0x3e924925));
    let p1 = (f64::from((f64::from(p1) * 100.0) as f32) * 0.01) as f32;
    let pw1 = signed_pow(p1, 3.5);
    let m258 = vm.f32(m + 0x258);
    let m90 = vm.f32(m + 0x90);
    let mut x7 = m258 * pw1 * r13;
    let pw2 = signed_pow((f64::from(m90) * 0.01) as f32, 3.5);
    let a = (f64::from(vm.f32(f + 0x400)) * 0.002) as f32;
    let mut a2 = a * a;
    if a < 0.0 || a.is_nan() {
        a2 = neg(a2);
    }
    let x5 = (f64::from(vm.f32(b + 0x9f0)) * 0.01) as f32;
    let lo = sse_min(x5, 2.0);
    let t = x5 - (vm.f32(m + 0x2bc) - 1.0) * (2.0 - x5);
    let y2 = if lo > t {
        lo
    } else {
        sse_min(sse_max(x5, 2.0), t)
    };
    let s = (f64::from(m90) * 100.0) as f32;
    let u = if -1.0 > s { -1.0 } else { sse_min(1.0, s) };
    let x15 = m258 * pw2 - m258 * a2 + y2 * u;
    if bind(env, f, 0x1d1, n) {
        let x1 = vm.f32(m + 0x90) - 10.0;
        let low = sse_min(x7, 0.0);
        let t = (0.0 - x7) / 10.0 * x1 + x7;
        x7 = if low > t {
            low
        } else {
            sse_min(sse_max(x7, 0.0), t)
        };
    }
    if bind(env, f, 0x181, n) {
        let t = (vm.f64(0x142f01910) * 2.0) as f32;
        let mut a = CallArgs::ints(&[0, u64::from(n as u32)]);
        a.xmm[0] = Some(t.to_bits());
        a.int[0] = None;
        let noise = reply_f32(env.call(vm, 0x1408bd9d0, a));
        x7 = ((f64::from(noise) * 0.05 + 0.95) * f64::from(x7)) as f32;
    }
    if bind(env, f, 0x171, n) {
        x7 = 0.0;
    }
    let start = bind(env, f, 0x1b1, n);
    starter_delay(vm, env, f, m, start);
    x7 *= vm.f32(m + 0x2c8);
    let speed = (f64::from(vm.f32(m + 0x90)) * 0.01) as f32;
    let pw = signed_pow(speed, 4.5) * vm.f32(engine(vm) + 8) / 0.95;
    let x11 = vm.f32(m + 0x268) / sse_max(pw, 1.0);
    let pw3 = signed_pow(speed, 2.0);
    let x11 = if 0.0 > x11 {
        0.0
    } else if x11 > pw3 {
        pw3
    } else {
        x11
    };
    let e64 = vm.f32(engine(vm) + 0x64);
    let g = if 0.1 > e64 { 0.1 } else { sse_min(2.0, e64) };
    let thrust = thrust_term_of(vm, env, f, n);
    let power = thrust + g.sqrt() * x7 - x15 - x11;
    let dt = frame_time(env);
    let power = power / vm.f32(b + 0xa20);
    let m78 = vm.f32(m + 0x78);
    let integrated = (f64::from(vm.f32(m + 0x90)) + dt * f64::from(power) * 200.0) as f32;
    let speed_new = sse_max(integrated, 0.0);
    vm.set_f32(m + 0x90, speed_new);
    let rated = sse_max(vm.f32(engine(vm) + 0x18), 1.0);
    let ratio = m78 / rated;
    let m98 = (f64::from(ratio) * 100.0) as f32;
    let v = if -1.5 > m98 { -1.5 } else { sse_min(1.5, m98) };
    let e64 = vm.f32(engine(vm) + 0x64);
    let g2 = if 0.1 > e64 { 0.1 } else { sse_min(2.0, e64) };
    let efficiency = propeller_curve(vm, b, speed_new, ratio, vm.f32(m + 0x258));
    let d7 = f64::from(efficiency) - f64::from(v) * 0.01 * (1.0 - f64::from(r13));
    vm.set_f32(m + 0x98, m98);
    let e10 = f64::from(vm.f32(engine(vm) + 0x10));
    vm.set_f32(m + 0xb8, (f64::from(g2.sqrt()) * (e10 * d7)) as f32);
}

/// The runtime values the propeller curve of `B+0x990 == 1` interpolates between (two floats at `0x14612c1c8`
/// and `0x14612c1d0`; zero in the reference build until the weather system fills them).
pub const PROPELLER_REFERENCE: u64 = 0x1_4612_c1c8;

/// `0x141a6a650(B, a, r, d)`: the propeller power coefficient from the speed `a` (percent), the speed ratio `r`
/// and the density factor `d`, by the model selected with `B+0x990` (0: the power laws of the speed, 1: the
/// density-scaled model with the exponents `B+0x998..0x9a4`, anything else 0).
pub fn propeller_curve(vm: &Vm, b: u64, a: f32, r: f32, d: f32) -> f32 {
    match vm.i32(b + 0x990) {
        0 => {
            let h = (f64::from(a) * 0.01) as f32;
            let p3 = signed_pow(h, 3.0);
            let p14 = signed_pow(h, f32::from_bits(0x3fb33333));
            let cc = interpolate_clamped(25.0, p3, 50.0, p14, a);
            let s1 = (f64::from(cc) * f64::from_bits(0x3ff33b645a1cac08)) as f32;
            let s2 = (f64::from(cc) * -0.2) as f32;
            let p4 = signed_pow(h, 4.0);
            let r2 = signed_pow(r, 2.0);
            let rr = (f64::from(r) * 100.0) as f32;
            let q = if -1.0 > rr { -1.0 } else { sse_min(1.0, rr) };
            let e1 = r2 * s2 * d + p4 * s1 * d - q * f32::from_bits(0x3b03126f);
            let h2 = signed_pow(h, 2.0);
            let r2b = signed_pow(r, 2.0);
            let diff = f64::from(h2 - r2b);
            let t = (diff / (f64::from(a) * 5.0 * 0.01 + 1.0)) as f32;
            let t = (f64::from(t) / (f64::from(r) * 5.0 + 1.0)) as f32;
            sse_max(0.0, t) * d + e1
        }
        1 => {
            let rho = interpolate_clamped(
                135.0,
                vm.f32(PROPELLER_REFERENCE),
                136.0,
                vm.f32(PROPELLER_REFERENCE + 8),
                f32::from_bits(0x43075811),
            ) / 1.225;
            let line = |v0: f32, v1: f32| {
                if rho == 1.0 {
                    (v0 + v1) * 0.5
                } else {
                    (v1 - v0) / (1.0 - rho) * (d - rho) + v0
                }
            };
            let k1 = line(vm.f32(b + 0x998), vm.f32(b + 0x99c));
            let k2 = line(vm.f32(b + 0x9a0), vm.f32(b + 0x9a4));
            let ah = (f64::from(a) * 0.01) as f32;
            let pa = signed_pow(ah, k1);
            let k994 = vm.f32(b + 0x994);
            let t12 = ((f64::from(k994) + 1.0) * f64::from(d) * f64::from(pa)) as f32;
            let pr = signed_pow(r, k2);
            let abs_r = r.abs();
            let v = neg(d) * k994 * pr + t12;
            let root = f64::from(abs_r.sqrt());
            (f64::from(v) / (root * 0.95 + 0.05)) as f32
        }
        _ => 0.0,
    }
}
