//! Helpers of the wheel contact model `0x1411c8690`, compared with the machine code in an emulator
//! (`tools/gen_small_vectors.py`, kinds `caster` and `servo`).

use crate::tire::time;
use crate::vm::{Callees, Vm};

/// `0x141240d40(F, gear)`: whether the gear leg is steered by its caster/lock: false while `F+0x214` is set, for a
/// gear record (`[B+0x6080] + 0x88 gear`) of kind <= 1, and unless one of its two limits (`+0x50`, `+0x54`)
/// exceeds 0.01 or `+0xc` is set; then true for a negative `+0x7c` unless `+0xc` is clear while `F+0x204` is
/// set, and for a positive `+0x7c` when `F+0x210` is below 0.5.
pub fn caster_locked(vm: &Vm, f: u64, gear: i32) -> bool {
    if vm.i32(f + 0x214) != 0 {
        return false;
    }
    let b = vm.u64(f + 0x20);
    let g = vm
        .u64(b + 0x6080)
        .wrapping_add((i64::from(gear) * 0x88) as u64);
    if vm.u32(g) <= 1 {
        return false;
    }
    let limit = 0.01f64;
    if !(f64::from(vm.f32(g + 0x50)) > limit
        || f64::from(vm.f32(g + 0x54)) > limit
        || vm.i32(g + 0xc) != 0)
    {
        return false;
    }
    let x = vm.f32(g + 0x7c);
    if 0.0 > x && (vm.i32(g + 0xc) != 0 || vm.i32(f + 0x204) == 0) {
        return true;
    }
    if x > 0.0 {
        return 0.5 > vm.f32(f + 0x210);
    }
    false
}

/// `0x1410c9620(a, k, &state, b, lo, hi, free)`: one step of the servo that moves `state` toward `a`: the
/// value is `a + (state - b) k / 0.025` limited to `lo..hi`; the rate `a - value` (limited to the magnitude of
/// that correction unless `free`) divided by `k` is integrated into `state` over the frame time. Returns the
/// limited value.
#[allow(clippy::too_many_arguments)]
pub fn servo(
    vm: &mut Vm,
    env: &mut dyn Callees,
    a: f32,
    k: f32,
    state: u64,
    b: f32,
    lo: f32,
    hi: f32,
    free: i32,
) -> f32 {
    let diff = (vm.f32(state) - b) * k;
    let d = f64::from(diff) / 0.025;
    let v = (f64::from(a) + d) as f32;
    let value = if lo > v {
        lo
    } else if hi < v {
        hi
    } else {
        v
    };
    let rate = a - value;
    let magnitude = (d as f32).abs();
    let step = if free != 0 {
        rate
    } else if -magnitude > rate {
        -magnitude
    } else if magnitude < rate {
        magnitude
    } else {
        rate
    };
    let dt = time(vm, env);
    let moved = f64::from(step / k);
    vm.set_f32(state, (f64::from(vm.f32(state)) + dt * moved) as f32);
    value
}
