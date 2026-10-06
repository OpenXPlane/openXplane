//! The engine control update `0x141260090`, ported in stages and compared with the original on random objects at
//! absolute addresses (`tools/gen_controls_vectors.py`). Objects: `F` (the flight object), `B = [F+0x20]`, the
//! engine records `E = [B+0x5ff8]` (stride `0x68`), the part records `P = [B+0x6010]` (stride `0x3770`), the engine
//! state records `M = [F+0x68b0]` (stride `0x2cc`) and the part state records `N = [F+0x68c8]` (stride `0x388`).
use crate::callees::{
    engine_level_negative, engine_level_positive, limit_a, limit_b, part_level_negative,
    part_level_positive,
};
use crate::engine::{HoldInputs, engine_held_back, signed_pow};
use crate::scalar::clamp;
use crate::vm::{CallArgs, Callees, Vm};
use crate::wing_element::interpolate_clamped;

const RAD: f32 = f32::from_bits(0x3c8efa36);

/// `0x1407ace10(F, 1, id, index, 1.0)`: an input-binding query (it writes no memory, so the replay gets a scratch one).
fn bind(env: &mut dyn Callees, f: u64, id: u32, index: i32) -> bool {
    let mut args = CallArgs::ints(&[f, 1, u64::from(id), index as u32 as u64]);
    args.int[3] = Some(index as u32 as u64);
    env.call(&mut Vm::default(), 0x1407ace10, args).rax as u32 != 0
}

#[allow(dead_code)]
fn binder(env: &mut dyn Callees, f: u64) -> impl FnMut(u32, i32) -> bool + '_ {
    move |id, index| bind(env, f, id, index)
}

/// `0x1411d9f60(F, index, mode)` through its ported rule.
fn held_back(vm: &Vm, env: &mut dyn Callees, f: u64, index: i32, mode: i32) -> bool {
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
    let shared = std::cell::RefCell::new(env);
    engine_held_back(
        h,
        |id, i| bind(&mut **shared.borrow_mut(), f, id, i),
        |i, m| {
            let args = CallArgs::ints(&[b, i as u32 as u64, m as u32 as u64]);
            shared
                .borrow_mut()
                .call(&mut Vm::default(), 0x140822620, args)
                .rax as i32
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
    /// After the lever groups, before the follow-up of the part forces at `0x141262177`.
    Groups,
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
            if bind(env, f, 0x179, e) {
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
        if held_back(&*vm, env, f, p, 1) {
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
        if held_back(&*vm, env, f, e, 3) {
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
    // ---- the lever groups ----
    let time = |vm: &mut Vm, env: &mut dyn Callees| -> f64 {
        f64::from_bits(env.call(vm, 0x140c448c0, CallArgs::default()).xmm0)
    };
    let obj = f + 0xbce0;
    env.call(vm, 0x141190ed0, CallArgs::ints(&[obj]));
    let engine_vec = temp;
    let part_vec = entry_rsp - 0x1e8;
    for k in 0..3 {
        vm.set_u64(engine_vec + 8 * k, 0);
    }
    env.call(
        vm,
        0x1406fd4d0,
        CallArgs::ints(&[engine_vec, engines as i64 as u64]),
    );
    let begin = vm.u64(engine_vec);
    for e in 0..engines.max(0) as u64 {
        let w = vm.u32(state_of(vm, e) + 0x78);
        vm.set_u32(begin + 4 * e, w);
    }
    for k in 0..3 {
        vm.set_u64(part_vec + 8 * k, 0);
    }
    env.call(
        vm,
        0x1406fd4d0,
        CallArgs::ints(&[part_vec, part_count as i64 as u64]),
    );
    let begin = vm.u64(part_vec);
    for p in 0..part_count.max(0) as u64 {
        let w = vm.u32(part_of(vm, p) + 0x1c);
        vm.set_u32(begin + 4 * p, w);
    }
    let sign_zone = |v: f32| -> f32 {
        if v > f32::from_bits(0x3727c5ac) {
            1.0
        } else if f32::from_bits(0xb727c5ac) > v {
            -1.0
        } else {
            0.0
        }
    };
    let lever_kind6_idle =
        |vm: &Vm, p: u64| vm.i32(p_table + 0x3770 * p) == 6 && vm.f32(f + 0x64b4) == 0.0;
    let _ = lever_kind6_idle;
    for g in 0..vm.i32(b + 0xb78).max(0) {
        let (mut x11, mut x9, mut x8) = (0.0f32, 0.0f32, 0.0f32);
        let engines = vm.i32(b + 0x91c);
        let parts = vm.i32(b + 0x920);
        for e in 0..engines.max(0) {
            if held_back(&*vm, env, f, e, 3)
                && vm.i32(b + 0xb7c + 4 * e as u64) == g
                && limit_a(&*vm, obj, e, &mut binder(env, f))
            {
                x11 += vm.f32(e_table + 0x68 * e as u64 + 0x3c);
                x9 = vm.f32(state_of(vm, e as u64) + 0x78);
                x8 += vm.f32(state_of(vm, e as u64) + 0xb8);
            }
        }
        for p in 0..parts.max(0) {
            if held_back(&*vm, env, f, p, 1)
                && vm.i32(b + 0xbbc + 4 * p as u64) == g
                && limit_b(&*vm, obj, p, &mut binder(env, f))
            {
                let part = p_table + 0x3770 * p as u64;
                let x2 = vm.f32(part + 0x20);
                x11 += vm.f32(part + 0x18) / (x2 * x2);
                x9 = x2 * vm.f32(part_of(vm, p as u64) + 0x1c);
                x8 -= vm.f32(part_of(vm, p as u64) + 0x64) / x2;
            }
        }
        let mut x10 = 0.0f32;
        for e in 0..engines.max(0) {
            if held_back(&*vm, env, f, e, 3) && vm.i32(b + 0xb7c + 4 * e as u64) == g {
                x10 += vm.f32(e_table + 0x68 * e as u64 + 0xc);
            }
        }
        for e in 0..engines.max(0) {
            if held_back(&*vm, env, f, e, 3) && vm.i32(b + 0xb7c + 4 * e as u64) == g {
                let positive = engine_level_positive(&*vm, obj, e, &mut binder(env, f));
                if f64::from(positive) > 0.99 {
                    let negative = engine_level_negative(&*vm, obj, e, &mut binder(env, f));
                    if -0.99 > f64::from(negative) {
                        continue;
                    }
                }
                let negative = f64::from(engine_level_negative(&*vm, obj, e, &mut binder(env, f)));
                let power = f64::from(vm.f32(e_table + 0x68 * e as u64 + 0xc));
                let lo = (power * 1.25 * negative) as f32;
                let positive = f64::from(engine_level_positive(&*vm, obj, e, &mut binder(env, f)));
                let hi = (power * 1.25 * positive) as f32;
                let m = state_of(vm, e as u64);
                let dt = time(vm, env);
                x8 += rate_follow(
                    vm,
                    dt,
                    vm.f32(m + 0xb8),
                    vm.f32(e_table + 0x68 * e as u64 + 0x3c),
                    x9,
                    m + 0x78,
                    lo,
                    hi,
                    true,
                );
            }
        }
        for p in 0..parts.max(0) {
            if held_back(&*vm, env, f, p, 1)
                && vm.i32(b + 0xbbc + 4 * p as u64) == g
                && !limit_b(&*vm, obj, p, &mut binder(env, f))
            {
                let part = p_table + 0x3770 * p as u64;
                let negative = f64::from(part_level_negative(&*vm, obj, p, &mut binder(env, f)));
                let lo = (f64::from(x10 * vm.f32(part + 0x20)) * 1.25 * negative) as f32;
                let positive = f64::from(part_level_positive(&*vm, obj, p, &mut binder(env, f)));
                let hi = (f64::from(x10 * vm.f32(part + 0x20)) * 1.25 * positive) as f32;
                let n = part_of(vm, p as u64);
                let b_ref = x9 / max_ss(vm.f32(part + 0x20), f32::from_bits(0x3c23d70a));
                let dt = time(vm, env);
                let r = rate_follow(
                    vm,
                    dt,
                    neg(vm.f32(n + 0x64)),
                    vm.f32(part + 0x18),
                    b_ref,
                    n + 0x1c,
                    lo,
                    hi,
                    true,
                );
                x8 += r / vm.f32(part + 0x20);
            }
        }
        if x11 > 0.0 {
            if engines > 0 {
                for e in 0..engines {
                    let part = p_table + 0x3770 * e as u64;
                    let idle = vm.i32(part) == 6 && vm.f32(f + 0x64b4) == 0.0;
                    if idle {
                        continue;
                    }
                    bind(env, f, 0x179, e);
                    bind(env, f, 0x1f9, e);
                    if limits_block(&*vm, env, f, part) {
                        continue;
                    }
                    if vm.i32(e_table + 0x68 * e as u64) as u32 <= 4
                        && vm.i32(b + 0xb7c + 4 * e as u64) == g
                    {
                        let m = state_of(vm, e as u64);
                        let x6 = abs(vm.f32(m + 0xb8));
                        let t = sign_zone(x9);
                        let x1 = vm.f32(e_table + 0x68 * e as u64 + 0xc) * vm.f32(b + 0x964) * t;
                        x8 -= x1;
                        x8 -= x6 * vm.f32(b + 0x968) * t;
                    }
                }
            }
            let dt = time(vm, env);
            let delta = (dt * f64::from(x8 / x11)) as f32;
            let x9new = if x9 != 0.0 {
                let s1 = if 0.0 > x9 { -1.0f32 } else { 1.0 };
                let s2 = if 0.0 > delta + x9 { -1.0f32 } else { 1.0 };
                if s1 == s2 { x9 + delta } else { 0.0 }
            } else {
                x9 + delta
            };
            x9 = x9new;
            for e in 0..engines.max(0) {
                if vm.i32(b + 0xb7c + 4 * e as u64) == g
                    && held_back(&*vm, env, f, e, 3)
                    && limit_a(&*vm, obj, e, &mut binder(env, f))
                {
                    let m = state_of(vm, e as u64);
                    vm.set_f32(m + 0x78, x9);
                }
            }
            for p in 0..parts.max(0) {
                if vm.i32(b + 0xbbc + 4 * p as u64) == g
                    && held_back(&*vm, env, f, p, 1)
                    && limit_b(&*vm, obj, p, &mut binder(env, f))
                {
                    let n = part_of(vm, p as u64);
                    let part = p_table + 0x3770 * p as u64;
                    vm.set_f32(
                        n + 0x1c,
                        x9 / max_ss(vm.f32(part + 0x20), f32::from_bits(0x3c23d70a)),
                    );
                }
            }
        }
    }
    if stop == Some(Stop::Groups) {
        return Ok(());
    }
    // ---- the follow-up: rates of change of the lever positions, and the moments of the propeller parts ----
    let v3 = vm.u64(engine_vec);
    let v4 = vm.u64(part_vec);
    let engines = vm.i32(b + 0x91c);
    let part_count = vm.i32(b + 0x920);
    for i in 0..part_count.max(engines).max(0) {
        let proceed = i < engines && vm.i32(e_table + 0x68 * i as u64) as u32 <= 4;
        if !proceed {
            let args = CallArgs::ints(&[b, i as u64, 1]);
            if env.call(&mut Vm::default(), 0x140822620, args).rax as i32 == 0 {
                continue;
            }
        }
        if i < vm.i32(b + 0x91c) {
            let inv = (1.0 / time(vm, env)) as f32;
            let m = state_of(vm, i as u64);
            let previous = vm.f32(v3 + 4 * i as u64);
            vm.set_f32(m + 0x7c, (vm.f32(m + 0x78) - previous) * inv);
        }
        if i < vm.i32(b + 0x920) {
            let inv = (1.0 / time(vm, env)) as f32;
            let n = part_of(vm, i as u64);
            let previous = vm.f32(v4 + 4 * i as u64);
            vm.set_f32(n + 0x18, (vm.f32(n + 0x1c) - previous) * inv);
        }
        if i < vm.i32(b + 0x920) {
            let n = part_of(vm, i as u64);
            let part = p_table + 0x3770 * i as u64;
            vm.set_f32(
                n + 0x10,
                vm.f32(part + 0x18) * vm.f32(n + 0x18) + vm.f32(n + 0x64),
            );
            if vm.i32(e_table + 0x68 * i as u64) == 7 {
                vm.set_i32(n + 0x10, 0);
            }
        }
    }
    for p in 0..part_count.max(0) {
        if held_back(&*vm, env, f, p, 1) {
            let n = part_of(vm, p as u64);
            let part = p_table + 0x3770 * p as u64;
            let x11 = neg(vm.f32(n + 0x10)) * vm.f32(part + 0xc);
            let angle = |off: u64| vm.f32(part + off) * RAD;
            let (a4, a0, a9) = (angle(0x7a4), angle(0x7a0), angle(0x79c));
            let (c4, s4, c0, s0, c9, s9) =
                (a4.cos(), a4.sin(), a0.cos(), a0.sin(), a9.cos(), a9.sin());
            let (x10, x7) = (c4 * 0.0, s4 * 0.0);
            let x3 = x7 + x10;
            let x10 = x10 - x7;
            let x1 = c0 * x11 + s0 * x10;
            let x2 = c9 * x3 - s9 * x1;
            let x12 = c0 * x10 - s0 * x11;
            let x9 = c9 * x1 + s9 * x3;
            for (off, add) in [(0x2f8u64, x9), (0x310, x2), (0x328, x12)] {
                vm.set_f32(f + off, add + vm.f32(f + off));
            }
        }
    }
    // the temporary vectors are released
    for vec in [part_vec, engine_vec] {
        let begin = vm.u64(vec);
        if begin != 0 {
            let count = vm.u64(vec + 0x10).wrapping_sub(begin) >> 2;
            env.call(vm, 0x1406307a0, CallArgs::ints(&[vec, begin, count]));
        }
    }
    let _ = clamp(0.0, 0.0, 1.0);
    Ok(())
}

fn abs(v: f32) -> f32 {
    f32::from_bits(v.to_bits() & 0x7fff_ffff)
}

fn neg(v: f32) -> f32 {
    f32::from_bits(v.to_bits() ^ 0x8000_0000)
}

fn max_ss(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// The four limit-sign queries `0x2f6..0x2f9` (index 0) of the hold rule, on part record `part`: true when one
/// of them holds the part back.
fn limits_block(vm: &Vm, env: &mut dyn Callees, f: u64, part: u64) -> bool {
    let (low, high) = (vm.f32(part + 0x790), vm.f32(part + 0x798));
    (bind(env, f, 0x2f6, 0) && low < 0.0 && high < 0.0)
        || (bind(env, f, 0x2f7, 0) && low > 0.0 && high < 0.0)
        || (bind(env, f, 0x2f8, 0) && low < 0.0 && high > 0.0)
        || (bind(env, f, 0x2f9, 0) && low > 0.0 && high > 0.0)
}

/// `0x1410c9620(a, k, b, &state, lo, hi, flag)`: moves `*state` toward a target at a rate limited by the follow
/// term; returns the clamped target (`x7` of the original).
#[allow(clippy::too_many_arguments)]
fn rate_follow(
    vm: &mut Vm,
    dt: f64,
    a: f32,
    k: f32,
    b: f32,
    state: u64,
    lo: f32,
    hi: f32,
    flag: bool,
) -> f32 {
    let s = vm.f32(state);
    let x2 = (s - b) * k;
    let d3 = f64::from(x2) / 0.025;
    let x2f = (f64::from(a) + d3) as f32;
    let x7 = if lo > x2f {
        lo
    } else if hi < x2f {
        hi
    } else {
        x2f
    };
    let x4 = a - x7;
    let limit = abs(d3 as f32);
    let x6 = if flag {
        x4
    } else if neg(limit) > x4 {
        neg(limit)
    } else if limit < x4 {
        limit
    } else {
        x4
    };
    let x6 = x6 / k;
    vm.set_f32(state, (f64::from(x6) * dt + f64::from(s)) as f32);
    x7
}

/// `0x1411975a0(M, F, e)`: the thrust of engine `e` (`M+0x25c` scaled by two constants and `B+0x970`, floored at zero)
/// along the propeller axis, rotated by the part's angles and added at the part's position through the three
/// force sinks of [`crate::callees`].
pub fn apply_engine_thrust(vm: &mut Vm, state: u64, f: u64, e: i32) {
    const C1: f32 = f32::from_bits(0x3aafc53a);
    const C2: f32 = f32::from_bits(0x408e38be);
    let b = vm.u64(f + 0x20);
    let part = vm.u64(b + 0x6010) + (i64::from(e) * 0x3770) as u64;
    let raw = vm.f32(state + 0x25c) * C1 * vm.f32(b + 0x970) * C2;
    let thrust = if 0.0 > raw { 0.0 } else { raw };
    let t = neg(thrust);
    let a0 = vm.f32(part + 0x7a0) * RAD;
    let a4 = vm.f32(part + 0x7a4) * RAD;
    let a9 = vm.f32(part + 0x79c) * RAD;
    let (c0, s0) = (a0.cos(), a0.sin());
    let (c9, s9) = (a9.cos(), a9.sin());
    let c4 = a4.cos() * 0.0;
    let s4 = a4.sin() * 0.0;
    let x2 = s4 + c4;
    let x6 = c4 - s4;
    let x1 = s0 * x6 + c0 * t;
    let x12 = c0 * x6 - s0 * t;
    let x8 = c9 * x2 - s9 * x1;
    let x13 = c9 * x1 + s9 * x2;
    let (p0, p1, p2) = (
        vm.f32(part + 0x790),
        vm.f32(part + 0x794),
        vm.f32(part + 0x798),
    );
    crate::callees::add_side_force(vm, f, x8, p1, p2);
    crate::callees::add_normal_force(vm, f, x12, p0, p2);
    crate::callees::add_axial_force(vm, f, x13, p0, p1);
}
