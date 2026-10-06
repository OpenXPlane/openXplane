//! The engine control update `0x141260090`, ported in stages and compared with the original on random objects at
//! absolute addresses (`tools/gen_controls_vectors.py`). Objects: `F` (the flight object), `B = [F+0x20]`, the
//! engine records `E = [B+0x5ff8]` (stride `0x68`), the part records `P = [B+0x6010]` (stride `0x3770`), the engine
//! state records `M = [F+0x68b0]` (stride `0x2cc`) and the part state records `N = [F+0x68c8]` (stride `0x388`).
use crate::engine::{HoldInputs, engine_held_back, signed_pow};
use crate::vm::{CallArgs, Callees, Vm};
use crate::wing_element::interpolate_clamped;

const RAD: f32 = f32::from_bits(0x3c8efa36);

/// `0x1407ace10(F, 1, id, index, 1.0)`: an input-binding query.
fn bind(vm: &mut Vm, env: &mut dyn Callees, f: u64, id: u32, index: i32) -> bool {
    let mut args = CallArgs::ints(&[f, 1, u64::from(id), index as u32 as u64]);
    args.int[3] = Some(index as u32 as u64);
    env.call(vm, 0x1407ace10, args).rax as u32 != 0
}

/// `0x1411d9f60(F, index, mode)` through its ported rule.
fn held_back(vm: &mut Vm, env: &mut dyn Callees, f: u64, index: i32, mode: i32) -> bool {
    let b = vm.u64(f + 0x20);
    let part = vm.u64(b + 0x6010) + (i64::from(index) * 0x3770) as u64;
    let h = HoldInputs {
        kind: vm.i32(part),
        lever: vm.f32(f + 0x64b4),
        limit_low: vm.f32(part + 0x790),
        limit_high: vm.f32(part + 0x798),
        index,
        mode,
    };
    // the closures need the memory and the callees: the queries do not write memory, so a cell shares them
    let shared = std::cell::RefCell::new((&mut *vm, &mut *env));
    engine_held_back(
        h,
        |id, i| {
            let mut guard = shared.borrow_mut();
            let (vm, env) = &mut *guard;
            bind(vm, &mut **env, f, id, i)
        },
        |i, m| {
            let mut guard = shared.borrow_mut();
            let (vm, env) = &mut *guard;
            let args = CallArgs::ints(&[b, i as u32 as u64, m as u32 as u64]);
            env.call(vm, 0x140822620, args).rax as i32
        },
    ) != 0
}

/// Where to stop (for comparison with the original at a checkpoint).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Before the part dispatch at `0x14126059d`.
    Dispatch,
    /// Before the vectors at `0x1412609bb`.
    Blend,
}

/// `entry_rsp` is the stack pointer at the entry of the original (the temporary float array lives at
/// `entry_rsp - 0x208`).
pub fn engine_controls(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    entry_rsp: u64,
    stop: Option<Stop>,
) -> Result<(), String> {
    let temp = entry_rsp - 0x208;
    if vm.i32(f + 0xbcd0) != 0 || vm.i32(f + 0xbcc8) != 0 {
        return Err("debug dump not ported".into());
    }
    let b = vm.u64(f + 0x20);
    let zero = 0.0f32;
    if vm.i32(f + 0x6760) != 0 {
        // the override area replaces the propeller group totals
        for (from, to) in [
            (0x6780, 0x2d0),
            (0x6784, 0x2bc),
            (0x677c, 0x2e4),
            (0x6788, 0x2f8),
            (0x678c, 0x310),
            (0x6790, 0x328),
        ] {
            let w = vm.u32(f + from);
            vm.set_u32(f + to, w);
        }
    } else {
        for k in 0..3 {
            vm.set_u64(temp + 8 * k, 0);
        }
        // the throttle of each group: the weighted mean of the engines' levers
        for g in 0..vm.i32(b + 0xc90).max(0) {
            let mut sum = 0.0f32;
            for j in 0..vm.i32(b + 0xc9c).max(0) {
                let mut cl = vm.i32(f + 0x560) != 0 || vm.i32(b + 0xc94) != 0;
                for k in 0..vm.i32(b + 0xca0).max(0) {
                    if vm.i32(f + 0x6cc + 4 * k as u64) == 2 {
                        cl = true;
                    }
                }
                let masked = if cl {
                    (vm.i32(b + 0xc98) & 0x3f) as u32
                } else {
                    0
                };
                let args = CallArgs::ints(&[b, j as u64, g as u64, u64::from(masked)]);
                let ok = env.call(vm, 0x1411854a0, args).rax as u8 != 0;
                if ok && vm.i32(f + 0x764 + 4 * j as u64) == 1 {
                    let x6 = vm.f32(b + 0x1bb0) * vm.f32(b + 0x1b94);
                    let d0 = vm.f32(b + 0xd84);
                    let d0 = if d0 > 1.0 { d0 } else { 1.0 };
                    let table = vm.u64(b + 0x61f8);
                    let x3 = (vm.f64(table + 0xd68 + 8 * j as u64) / f64::from(d0)) as f32;
                    let contribution = if 0.001 > f64::from(x3) {
                        0.0
                    } else {
                        let scaled = x3 * 10.0;
                        let mut index = scaled as i32;
                        if index < 0 {
                            index = 0;
                        } else if index > 9 + j {
                            index = 9 + j;
                        }
                        let v0 = vm.f32(b + 0xd88 + 4 * index as u64);
                        let v1 = vm.f32(b + 0xd88 + 4 * (index as u64 + 1));
                        interpolate_clamped(index as f32, v0, (index + 1) as f32, v1, scaled) * x6
                    };
                    sum += contribution;
                }
            }
            let mut weight = 0.0f32;
            let engines = vm.i32(b + 0x91c);
            for e in 0..engines.max(0) {
                let rec = vm.u64(b + 0x5ff8) + 0x68 * e as u64;
                if vm.i32(rec) == 0 && vm.i32(b + 0xd38 + 4 * e as u64) == g {
                    let state = vm.u64(f + 0x68b0) + 0x2cc * e as u64;
                    let lever = vm.f32(state + 0x78);
                    let cap = vm.f32(rec + 0x18);
                    let limited = if zero > lever {
                        0.0
                    } else if cap < lever {
                        cap
                    } else {
                        lever
                    };
                    weight += vm.f32(state + 4) * vm.f32(rec + 0x10) * limited / vm.f32(b + 0xa38);
                }
            }
            let weight = if weight > 1.0 { weight } else { 1.0 };
            sum /= weight;
            let level = crate::scalar::clamp(sum, 0.0, 1.0);
            vm.set_f32(temp + 4 * g as u64, level);
        }
        // the engines: input and start handling, then the update of the engine's own kind
        for e in 0..vm.i32(b + 0x91c).max(0) {
            if bind(vm, env, f, 0x179, e) {
                continue;
            }
            let state = vm.u64(f + 0x68b0) + 0x2cc * e as u64;
            let ready = env.call(vm, 0x1411c5a90, CallArgs::ints(&[f])).rax as u32 != 0;
            if !ready {
                vm.set_i32(state + 0xcc, 0);
            }
            for off in [0x240, 0x244, 0x248] {
                vm.set_i32(state + off, 0);
            }
            if vm.i32(b + 0xc34) != 0 && vm.i32(state + 0x38) == 1 && 51.0 > vm.f32(state + 0x98) {
                env.call(vm, 0x1411da6c0, CallArgs::ints(&[f, e as u64]));
            }
            let kind = vm.i32(vm.u64(b + 0x5ff8) + 0x68 * e as u64) as u32;
            if kind <= 4 && vm.i32(f + 0x6764) == 0 {
                env.call(vm, 0x14119ac90, CallArgs::ints(&[state, f, e as u64, temp]));
            } else if kind.wrapping_sub(5) <= 1 {
                env.call(vm, 0x141197b00, CallArgs::ints(&[state, f, e as u64]));
            } else if kind == 7 {
                env.call(vm, 0x14119a570, CallArgs::ints(&[state, f, e as u64]));
            }
        }
    }
    // the propeller force of each part that is not held back
    for p in 0..vm.i32(b + 0x920).max(0) {
        if held_back(vm, env, f, p, 1) {
            let n = vm.u64(f + 0x68c8) + 0x388 * p as u64;
            env.call(vm, 0x1411bd470, CallArgs::ints(&[n, f, p as u64]));
        }
    }
    if stop == Some(Stop::Dispatch) {
        return Ok(());
    }
    // the jet and rocket engines (kinds 5 and 6) set the lever of the propeller parts of kind 6
    vm.set_u64(f + 0x64b4, 0);
    let (mut x6, mut x8) = (0.0f32, 0.0f32);
    let engines = vm.i32(b + 0x91c);
    let part_count = vm.i32(b + 0x920);
    let e_table = vm.u64(b + 0x5ff8);
    let p_table = vm.u64(b + 0x6010);
    for e in 0..engines.max(0) as u64 {
        if vm.i32(e_table + 0x68 * e).wrapping_sub(5) as u32 <= 1 {
            let state = vm.u64(f + 0x68b0) + 0x2cc * e;
            let s = (vm.f32(p_table + 0x3770 * e + 0x7a0) * RAD).sin();
            x6 += signed_pow(s, 0.25) * vm.f32(state + 0x90);
            x8 = (f64::from(x8) + 1.0) as f32;
        }
    }
    let state_of = |vm: &Vm, p: u64| vm.u64(f + 0x68b0) + 0x2cc * p;
    let part_of = |vm: &Vm, p: u64| vm.u64(f + 0x68c8) + 0x388 * p;
    if engines > 0 && x6 > 0.0 && x8 > 0.0 {
        x6 /= x8;
        vm.set_f32(f + 0x64b4, 1.0);
        for p in 0..part_count.max(0) as u64 {
            if vm.i32(p_table + 0x3770 * p) == 6 {
                let v = (f64::from(
                    vm.f32(p_table + 0x3770 * p + 0x2c) * f32::from_bits(0x3dd67751) * x6,
                ) * 0.01) as f32;
                let (m, n) = (state_of(vm, p), part_of(vm, p));
                vm.set_f32(m + 0x78, v);
                vm.set_f32(n + 0x1c, v);
                let w = vm.f32(n + 0x64);
                vm.set_f32(m + 0xb8, w);
                let power = f32::from_bits(vm.f32(n).to_bits() & 0x7fff_ffff) * vm.f32(n + 0x64);
                vm.set_f32(m + 0x25c, power);
                vm.set_f32(f + 0x64b8, power + vm.f32(f + 0x64b8));
            }
        }
    } else {
        for p in 0..part_count.max(0) as u64 {
            if vm.i32(p_table + 0x3770 * p) == 6 {
                let (m, n) = (state_of(vm, p), part_of(vm, p));
                vm.set_i32(m + 0x78, 0);
                vm.set_i32(n + 0x1c, 0);
                vm.set_i32(m + 0xb8, 0);
                vm.set_i32(m + 0x25c, 0);
                vm.set_i32(n + 0x84, 0);
            }
        }
    }
    // the engines' power eases toward the propeller's, by four times the frame time
    for e in 0..engines.max(0) {
        if held_back(vm, env, f, e, 3) {
            let t =
                (f64::from_bits(env.call(vm, 0x140c448c0, CallArgs::default()).xmm0) * 4.0) as f32;
            let m = state_of(vm, e as u64);
            let x2 = vm.f32(m + 0xb8) * vm.f32(m + 0x78);
            let x1 = crate::scalar::clamp(t, 0.0, 1.0);
            let blended = (1.0 - x1) * vm.f32(m + 0x25c) + x1 * x2;
            vm.set_f32(m + 0x25c, blended);
            env.call(vm, 0x1411975a0, CallArgs::ints(&[m, f, e as u64]));
        }
    }
    if stop == Some(Stop::Blend) {
        return Ok(());
    }
    let _ = Stop::Blend;
    Err("rest of the controls not ported".into())
}
