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
    let fraction = |offset: u64| clamp(vm.f32(obj + offset) / divisor, 0.0, 1.0);
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
    clamp((level - f32::from_bits(0x3dcccccd)) * 2.5 + 0.0, 0.0, 1.0)
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

fn finite_or_zero(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

/// `0x1411767f0(F, f, a2, a3)`: an axial force `f` (a non-finite value counts as zero) at the point whose other
/// two coordinates are `a2` and `a3`: `F+0x2bc += f`, `F+0x310 += f * a3`, `F+0x328 += f * a2`.
pub fn add_axial_force(vm: &mut Vm, f_addr: u64, f: f32, a2: f32, a3: f32) {
    let f = finite_or_zero(f);
    vm.set_f32(f_addr + 0x2bc, f + vm.f32(f_addr + 0x2bc));
    vm.set_f32(f_addr + 0x310, f * a3 + vm.f32(f_addr + 0x310));
    vm.set_f32(f_addr + 0x328, f * a2 + vm.f32(f_addr + 0x328));
}

/// `0x141176be0(F, f, a2, a3)`: a normal force: `F+0x2d0 += f`, `F+0x2f8 -= f * a2`, `F+0x310 -= f * a3`.
pub fn add_normal_force(vm: &mut Vm, f_addr: u64, f: f32, a2: f32, a3: f32) {
    let f = finite_or_zero(f);
    vm.set_f32(f_addr + 0x2d0, f + vm.f32(f_addr + 0x2d0));
    vm.set_f32(f_addr + 0x2f8, vm.f32(f_addr + 0x2f8) - f * a2);
    vm.set_f32(f_addr + 0x310, vm.f32(f_addr + 0x310) - f * a3);
}

/// `0x141176e30(F, f, a2, a3)`: a side force: `F+0x2e4 += f`, `F+0x2f8 += f * a2`, `F+0x328 -= f * a3`.
pub fn add_side_force(vm: &mut Vm, f_addr: u64, f: f32, a2: f32, a3: f32) {
    let f = finite_or_zero(f);
    vm.set_f32(f_addr + 0x2e4, f + vm.f32(f_addr + 0x2e4));
    vm.set_f32(f_addr + 0x2f8, f * a2 + vm.f32(f_addr + 0x2f8));
    vm.set_f32(f_addr + 0x328, vm.f32(f_addr + 0x328) - f * a3);
}

/// `0x141176a30(F, a, b, c, g5, g6, g7)`: the force `(g5, g6, g7)` given in the world axes is rotated into the
/// aircraft axes with the sine/cosine pairs of `F+0x430..0x454`, added at the point `(a, b, c)` as an axial, a
/// side and a normal force, and its magnitude is stored at `F+0x294`.
pub fn add_world_force(vm: &mut Vm, f_addr: u64, point: [f32; 3], force: [f32; 3]) {
    let [a, b, c] = point;
    let [g5, g6, g7] = force;
    let (s1, c1) = (vm.f32(f_addr + 0x440), vm.f32(f_addr + 0x444));
    let (s0, c0) = (vm.f32(f_addr + 0x430), vm.f32(f_addr + 0x434));
    let (s2, c2) = (vm.f32(f_addr + 0x450), vm.f32(f_addr + 0x454));
    let u = c1 * g7 - s1 * g5;
    let v = c1 * g5 + s1 * g7;
    let t = u * s0 + c0 * g6;
    let w3 = u * c0 - s0 * g6;
    let w1 = t * c2 + v * s2;
    let w2 = v * c2 - t * s2;
    add_axial_force(vm, f_addr, w3, a, b);
    add_side_force(vm, f_addr, w2, b, c);
    add_normal_force(vm, f_addr, w1, a, c);
    let sum = w2 * w2 + w1 * w1 + w3 * w3;
    vm.set_f32(
        f_addr + 0x294,
        if 0.0 > sum { f32::NAN } else { sum.sqrt() },
    );
}
