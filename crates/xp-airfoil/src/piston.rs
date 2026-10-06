//! The update of an engine of kinds 0 to 4 (`0x14119ac90(M, F, n, inputs)`): the manifold and density terms, the
//! power fraction, the throttle lever with its limits, the per-kind handlers, and the power from the altitude
//! and temperature tables. Objects as in [`crate::controls`]; `inputs` is the float array indexed by `B+0xd38+4n`.
//! Compared with the original on random objects at absolute addresses (`tools/gen_piston_vectors.py`).
use crate::callees::engine_ratio;
use crate::controls::ATMOSPHERE_TABLE;
use crate::engine::curve;
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
    let limited = env
        .call(vm, 0x1411a2e40, CallArgs::ints(&[f, u64::from(n as u32)]))
        .rax as u32
        != 0;
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
        call_void(vm, env, 0x14119c610, x6);
    }
    if vm.i32(engine(vm)) == 3 {
        call_void(vm, env, 0x14119cb70, x6);
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
