//! The landing gear lever state (`0x1411878f0`), compared with the machine code in an emulator
//! (`tools/gen_small_vectors.py`, kind `gearlever`).

use crate::tire::time;
use crate::vm::{Callees, Vm};

/// `0x1411878f0(F)`: moves the gear position `F+0x220` by the lever mode `F+0x228`: mode 1 retracts it at 0.33
/// per second; mode -1 jumps it to 1 (and, with the position below 0.97, clears the mode and, with `B+0xe90`, sets
/// the latch `F+0x22c`); otherwise, with `B+0xe90` set and the latch clear, it falls by `dt / 3600`. The position is
/// held to `0..1`, a position below 0.001 is zero and ends the mode, and the timers `F+0x238`, `F+0x23c` count
/// down (a timer that is not positive is cleared).
pub fn gear_lever_step(vm: &mut Vm, env: &mut dyn Callees, f: u64) {
    let mode = vm.i32(f + 0x228);
    let b = vm.u64(f + 0x20);
    let mut run = false;
    let mut v = 0.0f64;
    if mode == 1 {
        let p = f64::from(vm.f32(f + 0x220));
        let dt = time(vm, env);
        v = p - dt * 0.33;
        run = true;
    } else if mode == -1 {
        if 0.97 > f64::from(vm.f32(f + 0x224)) {
            vm.set_f32(f + 0x220, 1.0);
        } else {
            vm.set_f32(f + 0x220, 1.0);
            vm.set_f32(f + 0x224, 1.0);
            if vm.i32(b + 0xe90) != 0 {
                vm.set_i32(f + 0x22c, 1);
            }
            vm.set_i32(f + 0x228, 0);
        }
    } else if vm.i32(b + 0xe90) != 0 && vm.i32(f + 0x22c) == 0 {
        let p = f64::from(vm.f32(f + 0x220));
        let dt = time(vm, env);
        v = p - dt / 3600.0;
        run = true;
    }
    if run {
        let x = v as f32;
        let held = if 0.0 > x { 0.0 } else { x };
        let value = if 1.0 < held { 1.0 } else { held };
        vm.set_f32(f + 0x220, value);
    }
    if 0.001 > f64::from(vm.f32(f + 0x220)) {
        vm.set_f32(f + 0x220, 0.0);
        vm.set_i32(f + 0x228, 0);
    }
    if vm.f32(f + 0x238) > 0.0 {
        let dt = time(vm, env);
        let n = (f64::from(vm.f32(f + 0x238)) - dt) as f32;
        vm.set_f32(f + 0x238, n);
    } else {
        vm.set_f32(f + 0x238, 0.0);
    }
    if vm.f32(f + 0x23c) > 0.0 {
        let dt = time(vm, env);
        let n = (f64::from(vm.f32(f + 0x23c)) - dt) as f32;
        vm.set_f32(f + 0x23c, n);
    } else {
        vm.set_f32(f + 0x23c, 0.0);
    }
}
