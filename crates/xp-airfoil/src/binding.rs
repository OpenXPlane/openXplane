//! The input-binding query of the flight object (`0x1407ace10`): whether input `a + b` is active for the
//! given mode, with its side effects on the analog accumulators (`tools/gen_binding_vectors.py`).

use crate::vm::{CallArgs, Callees, Vm};

/// The static table of ints indexed by input id (`0x142515560`) and the table of 40-byte rows (`0x142515366`).
const ID_TABLE: u64 = 0x1_4251_5560;
const ROW_TABLE: u64 = 0x1_4251_5366;

/// `0x1407ace10(F, mode, a, b, amount)`: `mode` 1 is the press, 2 the release query; returns the original's `eax`.
/// `0x14082be10` is replayed.
pub fn input_active(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    mode: i32,
    a: i32,
    b: i32,
    amount: f32,
) -> Result<bool, String> {
    let id = a.wrapping_add(b);
    if (id as u32) > 0x301 {
        return Ok(false);
    }
    let idx = id as i64 as u64;
    if vm.u32(f + 0xdc0 + 4 * idx) == 6 {
        return Ok(true);
    }
    if (mode.wrapping_sub(1) as u32) > 1 {
        return Ok(false);
    }
    let aircraft = |vm: &Vm| vm.u64(f + 0x20);
    // esi: whether the input is available to this aircraft
    let mut esi = true;
    if vm.i32(f + 0x55c) == 0 {
        esi = false;
    } else {
        let b0 = aircraft(vm);
        if vm.i32(b0 + 0x918) != 0 {
            // `0x141238b40(F, 0)`: the lever setting of engine 0 above the global double `0x142f01918`
            let lever = vm.f32(vm.u64(f + 0x68b0) + 0x218);
            let answer = f64::from(lever) > vm.f64(0x1_42f0_1918);
            if answer && vm.i32(b0 + 0x91c) == 1 {
                let engines = vm.u64(b0 + 0x5ff8);
                if (vm.u32(engines).wrapping_sub(1)) <= 1 {
                    esi = false;
                }
            }
        }
    }
    // the second lookup used by several inputs: `0x14082be10([B+0x1d18], 2)`
    let object_missing = |vm: &mut Vm, env: &mut dyn Callees| -> Option<bool> {
        let b0 = vm.u64(f + 0x20);
        let obj = vm.u64(b0 + 0x1d18);
        if obj == 0 {
            return None;
        }
        Some(env.call(vm, 0x14082be10, CallArgs::ints(&[obj, 2])).rax == 0)
    };
    let word = |vm: &Vm, o: u64| vm.i32(f + o);
    // the outcome of the dispatch: Some(true) returns 1, Some(false) returns 0, None goes on to the common part
    let mut after: Option<bool> = None;
    match id {
        0x45 => {
            let mut args = CallArgs::ints(&[f, 2, 0xfa, 0]);
            args.stack[0] = Some(u64::from(1.0f32.to_bits()));
            let again = input_active(vm, env, f, 2, 0xfa, 0, 1.0)?;
            let _ = args;
            let mut ebp = 0;
            let b0 = aircraft(vm);
            let slot = vm.i32(b0 + 0xad8);
            if again && slot > -1 && (slot as u32 as u64) < 0xd {
                let row = vm.u8(ROW_TABLE + 40 * slot as u64);
                if row == 0 {
                    ebp = 1;
                }
            }
            if !esi || ebp != 0 {
                after = Some(true);
            }
        }
        0x47 | 0x48 | 0x49 | 0x44 | 0x4a | 0x43 | 0x112 | 0x113 | 0x2fa | 0x103 => {
            if !esi {
                after = Some(true);
            }
        }
        0xfa => {
            if !esi && vm.i32(aircraft(vm) + 0x918) != 0 {
                after = Some(true);
            }
        }
        0x10f..=0x111 => {
            if !esi {
                match object_missing(vm, env) {
                    None | Some(true) => after = Some(true),
                    Some(false) => {}
                }
            }
        }
        0x102 => {
            if !esi && word(vm, 0x564) == 0 {
                match object_missing(vm, env) {
                    None | Some(true) => after = Some(true),
                    Some(false) => {}
                }
            }
        }
        0x104 | 0x10a => {
            let offset = if id == 0x104 { 0x5ac } else { 0x5bc };
            let go_object = if !esi { true } else { word(vm, offset) == 0 };
            if go_object {
                match object_missing(vm, env) {
                    None | Some(true) => after = Some(true),
                    Some(false) => {}
                }
            }
        }
        0x105 | 0x107 | 0x108 | 0x109 | 0x10b | 0x12f | 0x130 | 0x10e => {
            let offset = match id {
                0x105 => 0x5b0,
                0x107 => 0x5f4,
                0x108 => 0x5b4,
                0x109 => 0x5b8,
                0x10b => 0x5c0,
                0x12f => 0x5bc,
                0x130 => 0x5c0,
                _ => 0x5c4,
            };
            if !esi || word(vm, offset) == 0 {
                after = Some(true);
            }
        }
        0x106 => {
            let mut done = false;
            if esi && word(vm, 0x5f0) != 0 {
                done = true;
            }
            if !done && let Some(false) = object_missing(vm, env) {
                done = true;
            }
            if !done && word(vm, 0x564) == 0 {
                after = Some(true);
            }
        }
        _ => {}
    }
    if let Some(v) = after {
        return Ok(v);
    }
    // the common part
    let b0 = aircraft(vm);
    if vm.i32(ID_TABLE + 4 * idx) == 0 && (id.wrapping_sub(0x2fc) as u32) > 5 {
        return Ok(false);
    }
    if vm.u8(b0 + 0x2998 + idx) == 0 {
        return Ok(false);
    }
    if vm.u8(f + 0x55f0 + idx) == 0 && mode == 1 {
        vm.set_u8(f + 0x55f0 + idx, mode as u8);
        let value = match id {
            0x102 => {
                let s = word(vm, 0x5f0).wrapping_add(word(vm, 0x5ac));
                (f64::from(s) * 0.5) as f32
            }
            0x103 => {
                let s = word(vm, 0x5f4).wrapping_add(word(vm, 0x5b0));
                (f64::from(s) * 0.5) as f32
            }
            _ => amount,
        };
        let b0 = aircraft(vm);
        let mask = vm.u8(b0 + 0x2998 + idx);
        let level = |vm: &Vm, k: u64| vm.f32(f + 0xd18 + 4 * k);
        let mut best = -1.0f32;
        let mut slot = 0u64;
        for k in 0..6u64 {
            if mask & (1 << k) == 0 {
                continue;
            }
            let l = level(vm, k);
            let beats = if k == 0 {
                f64::from(l) - 0.99 > -1.0
            } else {
                f64::from(l) - 0.99 > f64::from(best)
            };
            if beats {
                slot = k;
                best = l;
            }
        }
        let scaled = value * vm.f32(b0 + 0x2c9c + 4 * idx);
        vm.set_f32(f + 0xd30 + 4 * slot, scaled + vm.f32(f + 0xd30 + 4 * slot));
    }
    // the final checks of the analog levels against the limit at `B+0x1b7c`
    let b0 = aircraft(vm);
    let mut mask = u32::from(vm.u8(b0 + 0x2998 + idx));
    for (k, bit) in [(5u64, 0x20u32), (4, 0x10), (3, 8), (2, 4), (1, 2), (0, 1)] {
        if mask >= bit {
            mask -= bit;
            if vm.i32(b0 + 0xc90) > k as i32 && vm.f32(f + 0xd18 + 4 * k) > vm.f32(b0 + 0x1b7c) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// The query as the flight step makes it: `0x1407ace10(F, mode, id, index, 1.0)` on the real flight object.
pub fn bound(vm: &mut Vm, env: &mut dyn Callees, f: u64, mode: i32, id: i32, index: i32) -> bool {
    input_active(vm, env, f, mode, id, index, 1.0).unwrap_or(false)
}
