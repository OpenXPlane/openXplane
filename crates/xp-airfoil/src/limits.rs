//! Small queries of the part and engine records used by the engine control: the ratio of an engine record to its
//! part (`0x1408154c0`), the blend factor of three components (`0x1411daa80`) and the two limit tests
//! (`0x1411a0900`, `0x141218060`). Compared with the machine code in an emulator
//! (`tools/gen_small_vectors.py`, kinds `ratio`, `blend`, `limita`, `limitb`).

use crate::binding::input_active;
use crate::scalar::clamp;
use crate::vm::{Callees, Vm};

/// `0x1408154c0(B, n)`: `engine[clamp(n)].+0x18 / part[n].+0x20`, zero when `n` is not below `B+0x920`, or when
/// `B+0x91c` or `B+0x920` is zero.
pub fn engine_ratio(vm: &Vm, b: u64, n: i32) -> f32 {
    let (count, engines) = (vm.i32(b + 0x920), vm.i32(b + 0x91c));
    if n >= count || engines == 0 || count == 0 {
        return 0.0;
    }
    let index = if n < 0 { 0 } else { n.min(engines - 1) };
    let engine = vm
        .u64(b + 0x5ff8)
        .wrapping_add((i64::from(index) * 0x68) as u64);
    let part = vm
        .u64(b + 0x6010)
        .wrapping_add((i64::from(n) * 0x3770) as u64);
    vm.f32(engine + 0x18) / vm.f32(part + 0x20)
}

/// `0x1411daa80(obj, mask)`: one for a zero mask; otherwise the components `+0x60`, `+0x64`, `+0x68` of the
/// object, each divided by `[[obj+8]+0xdb4]` and held to `0..1`, the ones selected by the bits of `mask` are
/// combined by the larger value, then `(v - 0.1) * 2.5` is held to `0..1`.
pub fn blend_factor(vm: &Vm, obj: u64, mask: i32) -> f32 {
    if mask == 0 {
        return 1.0;
    }
    let scale = vm.f32(vm.u64(obj + 8) + 0xdb4);
    let held = |x: f32| clamp(x, 0.0, 1.0);
    let (a, b, c) = (
        held(vm.f32(obj + 0x60) / scale),
        held(vm.f32(obj + 0x64) / scale),
        held(vm.f32(obj + 0x68) / scale),
    );
    let mut v = 0.0f32;
    if mask & 1 != 0 {
        v = if a > 0.0 { a } else { 0.0 };
    }
    if mask & 2 != 0 {
        v = if b > v { b } else { v };
    }
    if mask & 4 != 0 {
        v = if c > v { c } else { v };
    }
    let r = (v - f32::from_bits(0x3dcc_cccd)) * 2.5 + 0.0;
    clamp(r, 0.0, 1.0)
}

/// `0x1411a0900(obj, i)`: whether the limit of engine `i` is active: its `+0x2c` flag is set, the binding
/// `(0x201, i)` is not held and `obj+0x18` squared exceeds 0.99 while the negative test `0x1411a0a10` is below
/// -0.99.
pub fn limit_a(vm: &mut Vm, env: &mut dyn Callees, obj: u64, i: i32) -> Result<bool, String> {
    let f = vm.u64(obj);
    if input_active(vm, env, f, 1, 0x201, i, 1.0)? {
        return Ok(false);
    }
    let engine = vm
        .u64(vm.u64(obj + 8) + 0x5ff8)
        .wrapping_add((i64::from(i) * 0x68) as u64);
    let square = vm.f32(obj + 0x18) * vm.f32(obj + 0x18);
    let first = if vm.i32(engine + 0x2c) == 0 {
        1.0
    } else {
        square
    };
    let active = f64::from(first) > 0.99;
    if !active {
        return Ok(false);
    }
    if input_active(vm, env, f, 1, 0x201, i, 1.0)? {
        return Ok(false);
    }
    let second = if vm.i32(engine + 0x28) != 0 {
        return Ok(false);
    } else if vm.i32(engine + 0x2c) == 0 {
        -1.0
    } else {
        -square
    };
    Ok(-0.99 > f64::from(second))
}

/// `0x141218060(obj, n)`: [`limit_a`] for the part records `[B+0x6010] + 0x3770 n`: a part of kind 5 is skipped
/// while the binding `(0xd8, 0)` is held, and `+0x1c` selects the squared value.
pub fn limit_b(vm: &mut Vm, env: &mut dyn Callees, obj: u64, n: i32) -> Result<bool, String> {
    let f = vm.u64(obj);
    let part = |vm: &Vm| {
        vm.u64(vm.u64(obj + 8) + 0x6010)
            .wrapping_add((i64::from(n) * 0x3770) as u64)
    };
    let square = vm.f32(obj + 0x18) * vm.f32(obj + 0x18);
    let first = {
        let p = part(vm);
        if vm.i32(p) == 5 && input_active(vm, env, f, 1, 0xd8, 0, 1.0)? {
            0.0
        } else if vm.i32(p + 0x1c) == 0 {
            1.0
        } else {
            square
        }
    };
    let active = f64::from(first) > 0.99;
    if !active {
        return Ok(false);
    }
    let second = {
        let p = part(vm);
        if vm.i32(p) == 5 && input_active(vm, env, f, 1, 0xd8, 0, 1.0)? {
            0.0
        } else if vm.i32(p + 0x1c) == 0 {
            -1.0
        } else {
            -square
        }
    };
    Ok(-0.99 > f64::from(second))
}
