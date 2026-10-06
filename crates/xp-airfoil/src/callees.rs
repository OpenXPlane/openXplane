//! Small callees of the propeller force function that follow pointers through the original's objects
//! ([`Vm`]), each checked against the machine code by `tools/gen_callee_vectors.py`.
use crate::scalar::clamp;
use crate::vm::Vm;

/// `0x1408154c0(B, n)`: the ratio of engine record `min(n, count - 1)` (`B+0x5ff8`, stride `0x68`, field `+0x18`) to
/// the part record `n` (`B+0x6010`, stride `0x3770`, field `+0x20`); zero when `n` is not below `B+0x920` or either
/// count is zero.
pub fn engine_ratio(vm: &Vm, b: u64, n: i32) -> f32 {
    let parts = vm.i32(b + 0x920);
    if n >= parts {
        return 0.0;
    }
    let engines = vm.i32(b + 0x91c);
    if engines == 0 || parts == 0 {
        return 0.0;
    }
    let index = if n < 0 { 0 } else { n.min(engines - 1) };
    let engine = vm.u64(b + 0x5ff8) + (index as i64 * 0x68) as u64;
    let part = vm
        .u64(b + 0x6010)
        .wrapping_add((i64::from(n) * 0x3770) as u64);
    vm.f32(engine + 0x18) / vm.f32(part + 0x20)
}

/// `0x1411daa80(obj, mask)`: three fractions `obj+0x60/0x64/0x68` over the float at `+0xdb4` of the object
/// `obj+8` points to, each held to `0..1`; the largest of those selected by `mask` (bit 0, 1, 2, accumulated in that
/// order), less 0.1, times 2.5, held to `0..1`. Without a mask bit the result is 1 for `mask == 0` at the start.
pub fn blend(vm: &Vm, obj: u64, mask: i32) -> f32 {
    if mask == 0 {
        return 1.0;
    }
    let divisor = vm.f32(vm.u64(obj + 8) + 0xdb4);
    let fraction = |offset: u64| {
        let v = vm.f32(obj + offset) / divisor;
        if 0.0 > v {
            0.0
        } else if 1.0 < v {
            1.0
        } else {
            v
        }
    };
    let (a, b, c) = (fraction(0x60), fraction(0x64), fraction(0x68));
    let sse_max = |x: f32, y: f32| if x > y { x } else { y };
    let mut level = 0.0;
    if mask & 1 != 0 {
        level = sse_max(a, 0.0);
    }
    if mask & 2 != 0 {
        level = sse_max(b, level);
    }
    if mask & 4 != 0 {
        level = sse_max(c, level);
    }
    let v = (level - f32::from_bits(0x3dcccccd)) * 2.5 + 0.0;
    if 0.0 > v {
        0.0
    } else if 1.0 < v {
        1.0
    } else {
        v
    }
}

/// `0x1411a0970(obj, i)` / `0x1411a0a10(obj, i)`: the positive and negative lever levels of engine `i`
/// (`binding(0x201, i)` is the input query `0x1407ace10`).
fn lever_levels_engine(
    vm: &Vm,
    obj: u64,
    i: i32,
    binding: &mut dyn FnMut(u32, i32) -> bool,
) -> (f32, f32) {
    let b = vm.u64(obj + 8);
    let record = vm.u64(b + 0x5ff8) + (i64::from(i) * 0x68) as u64;
    let state = vm.f32(obj + 0x18);
    let held = binding(0x201, i);
    let positive = if held {
        0.0
    } else if vm.i32(record + 0x2c) == 0 {
        1.0
    } else {
        state * state
    };
    let negative = if held || vm.i32(record + 0x28) != 0 {
        0.0
    } else if vm.i32(record + 0x2c) == 0 {
        -1.0
    } else {
        -(state * state)
    };
    (positive, negative)
}

/// `0x1411a0900(obj, i)`: whether engine `i`'s lever is fully forward (above 0.99) and the negative level below -0.99.
pub fn limit_a(vm: &Vm, obj: u64, i: i32, binding: &mut dyn FnMut(u32, i32) -> bool) -> bool {
    let (positive, _) = lever_levels_engine(vm, obj, i, binding);
    if f64::from(positive) > 0.99 {
        let (_, negative) = lever_levels_engine(vm, obj, i, binding);
        return -0.99 > f64::from(negative);
    }
    false
}

/// `0x1412180d0` / `0x141218180`: the same levels for part record `n` (part kind 5 asks `binding(0xd8, 0)` first).
fn lever_levels_part(
    vm: &Vm,
    obj: u64,
    n: i32,
    binding: &mut dyn FnMut(u32, i32) -> bool,
) -> (f32, f32) {
    let b = vm.u64(obj + 8);
    let part = vm.u64(b + 0x6010) + (i64::from(n) * 0x3770) as u64;
    let state = vm.f32(obj + 0x18);
    let held = vm.i32(part) == 5 && binding(0xd8, 0);
    let positive = if held {
        0.0
    } else if vm.i32(part + 0x1c) == 0 {
        1.0
    } else {
        state * state
    };
    let negative = if held {
        0.0
    } else if vm.i32(part + 0x1c) == 0 {
        -1.0
    } else {
        -(state * state)
    };
    (positive, negative)
}

/// `0x141218060(obj, n)`: [`limit_a`] for part record `n`.
pub fn limit_b(vm: &Vm, obj: u64, n: i32, binding: &mut dyn FnMut(u32, i32) -> bool) -> bool {
    let (positive, _) = lever_levels_part(vm, obj, n, binding);
    if f64::from(positive) > 0.99 {
        let (_, negative) = lever_levels_part(vm, obj, n, binding);
        return -0.99 > f64::from(negative);
    }
    false
}

#[allow(dead_code)]
fn unit(v: f32) -> f32 {
    clamp(v, 0.0, 1.0)
}
