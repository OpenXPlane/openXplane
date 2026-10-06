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

/// `0x1406e2f20`: the cross product `a x b` normalised; returns the length of the cross product before
/// normalisation (the result is `[0, 1, 0]`-like `out = [0, 1, 0]` order `(+0, +4, +8) = (0, 1, 0)` for zero length).
fn cross_normalised(a: [f32; 3], b: [f32; 3]) -> ([f32; 3], f32) {
    let c0 = b[2] * a[1] - a[2] * b[1];
    let c1 = a[2] * b[0] - a[0] * b[2];
    let c2 = a[0] * b[1] - a[1] * b[0];
    let sum = c0 * c0 + c1 * c1 + c2 * c2;
    let len = if 0.0 > sum { f32::NAN } else { sum.sqrt() };
    if len == 0.0 {
        return ([0.0, 1.0, 0.0], len);
    }
    let inv = 1.0 / len;
    ([c0 * inv, c1 * inv, c2 * inv], len)
}

/// The inputs of [`add_aero_force`] (`0x140f26ef0`, "addFaero"), named by the original's argument positions.
#[derive(Clone, Copy, Debug)]
pub struct AeroForce {
    pub a2: f32,
    pub a3: f32,
    pub a5: f32,
    pub a6: f32,
    pub a7: f32,
    pub a8: f32,
    pub a9: f32,
    pub a10: f32,
    pub a11: f32,
    pub a12: f32,
    pub a13: f32,
    pub a14: i32,
    pub a15: f32,
    pub a16: f32,
    pub a17: f32,
}

const RECORD_VECTOR_END: u64 = 0x1461_25770;
const RECORD_VECTOR_CAP: u64 = 0x1461_25778;
const RECORDING_ID: u64 = 0x142f_2e3dc;

/// Appends a 0x50-byte record to the log vector at `0x146125768` (the inline path of the original; a full vector
/// would reallocate through `0x140f0f660`, which is not ported: `false` is returned then).
pub fn push_record(vm: &mut Vm, words: &[u32; 20]) -> bool {
    let end = vm.u64(RECORD_VECTOR_END);
    if end == vm.u64(RECORD_VECTOR_CAP) {
        return false;
    }
    for (i, w) in words.iter().enumerate() {
        vm.set_u32(end + 4 * i as u64, *w);
    }
    vm.set_u64(RECORD_VECTOR_END, end + 0x50);
    true
}

/// `0x140f26ef0(F, name, a2, a3, ...)`: adds an aerodynamic force given by its direction vector
/// `(a2, a5, a7)` (magnitudes in the original's argument slots) to the aerodynamic totals `F+0x2e8/0x2d4/0x2c0`
/// (forces) and `F+0x2fc/0x314/0x32c` (moments about the point `(a9, a10, a11)`), after building the frame from
/// the direction and logging the record when `F+0x28` names the recorded object. Returns false when the
/// record vector is full. A zero-length direction does nothing.
pub fn add_aero_force(vm: &mut Vm, f_addr: u64, a: &AeroForce) -> bool {
    let a3 = finite_or_zero(a.a3);
    let a6 = finite_or_zero(a.a6);
    let len = {
        let sum = a.a2 * a.a2 + a.a5 * a.a5 + a.a7 * a.a7;
        if 0.0 > sum { f32::NAN } else { sum.sqrt() }
    };
    let positive = len > 0.0;
    if !positive {
        return true;
    }
    let n = [a.a2 / len, a.a5 / len, a.a7 / len];
    let (c1, _) = cross_normalised(n, [1.0, 0.0, 0.0]);
    let (c2, _) = cross_normalised(c1, n);
    let mut record = [0u32; 20];
    record[1] = u32::from(a.a12 != 0.0 || a.a13 != 0.0);
    record[2] = a.a12.to_bits();
    record[3] = a.a13.to_bits();
    record[4] = a.a14 as u32;
    record[5] = (a.a9 + a.a15).to_bits();
    record[6] = (a.a10 + a.a16).to_bits();
    record[7] = (a.a11 + a.a17).to_bits();
    record[8] = a.a2.to_bits();
    record[9] = a.a5.to_bits();
    record[10] = a.a7.to_bits();
    let t1 = [c1[0] * a3, c1[1] * a3, c1[2] * a3];
    let t2 = [n[0] * a6, n[1] * a6, n[2] * a6];
    let t3 = [c2[0] * a.a8, c2[1] * a.a8, c2[2] * a.a8];
    for k in 0..3 {
        record[11 + k] = t1[k].to_bits();
        record[14 + k] = t2[k].to_bits();
        record[17 + k] = t3[k].to_bits();
    }
    let x7 = t1[0] + t2[0] + t3[0];
    let x8 = t1[1] + t2[1] + t3[1];
    let x6 = t2[2] + t1[2] + t3[2];
    let mut ok = true;
    if vm.i32(RECORDING_ID) == vm.i32(f_addr + 0x28) {
        ok = push_record(vm, &record);
    }
    let add =
        |vm: &mut Vm, offset: u64, v: f32| vm.set_f32(f_addr + offset, v + vm.f32(f_addr + offset));
    add(vm, 0x2e8, x7);
    add(vm, 0x2fc, x7 * a.a10 - x8 * a.a9);
    add(vm, 0x2d4, x8);
    add(vm, 0x314, x6 * a.a10 - x8 * a.a11);
    add(vm, 0x2c0, x6);
    add(vm, 0x32c, x6 * a.a9 - x7 * a.a11);
    ok
}
