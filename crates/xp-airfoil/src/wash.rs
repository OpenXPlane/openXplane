//! The wash of the aircraft's engines, wings and bodies on the air at a point (`0x14117d970`): the jet exhaust of
//! the engines of kinds 5 and 6, the propeller slipstream of the parts, and the wakes of the wings and bodies. The
//! original's locals live at `rbp + offset` (the stack arguments at `rbp + 0x660..`); the port reads and writes them
//! as memory of [`Vm`] so that the environment calls and the checkpoints see the same words.
use crate::engine::signed_pow;
use crate::transform::rotate_euler_offset;
use crate::vm::{Callees, Vm};
use crate::wing_element::hypot2;

const RAD: f32 = f32::from_bits(0x3c8efa36);

/// `a > b`, false when unordered (the original's `comisd; jbe` skip).
fn above(a: f64, b: f64) -> bool {
    a > b
}

fn sse_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// `0x141296900(a, b, c, ..., p1..p6)`: rotates `(a, b, c)` by three sine/cosine pairs; returns the three results in
/// the order of the original's output pointers.
pub fn rotate_pairs_b(a: f32, b: f32, c: f32, p: [f32; 6]) -> [f32; 3] {
    let [p1, p2, p3, p4, p5, p6] = p;
    let t1 = a * p2 + c * p1;
    let t2 = c * p2 - a * p1;
    let u = t2 * p4 - b * p3;
    let v = t2 * p3 + b * p4;
    [t1 * p6 - v * p5, v * p6 + t1 * p5, u]
}

/// `0x141176330`: a non-finite float of memory is replaced by zero.
fn check(vm: &mut Vm, address: u64) {
    if !vm.f32(address).is_finite() {
        vm.set_f32(address, 0.0);
    }
}

/// Where to stop (for comparison with the original at a checkpoint).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// After the jet wash loop (`0x14117e266`).
    Jets,
    /// After the part loop (`0x14117f79f`).
    Parts,
}

#[allow(clippy::too_many_arguments)]
/// The arguments of `0x14117d970(F, x, &out1, z, &out2, y, &out3, ...)` that are not in registers are read from the
/// stack words around `rbp`; `x` and `z` are the register arguments the prologue stores at `rbp + 0x648/0x658`.
pub fn wash(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    rbp: u64,
    x: f32,
    z: f32,
    out1: u64,
    stop: Option<Stop>,
) -> Result<(), String> {
    let slot = |off: u64| rbp.wrapping_add(off);
    vm.set_f32(slot(0x648), x);
    vm.set_f32(slot(0x658), z);
    let b = vm.u64(f + 0x20);
    let out2 = vm.u64(slot(0x660));
    let out3 = vm.u64(slot(0x670));
    let y = vm.f32(slot(0x668));
    for e in 0..vm.i32(b + 0x91c) {
        let engine = vm.u64(b + 0x5ff8) + (i64::from(e) * 0x68) as u64;
        if (vm.i32(engine).wrapping_sub(5) as u32) > 1 {
            continue;
        }
        let part = vm.u64(b + 0x6010) + (i64::from(e) * 0x3770) as u64;
        let state = vm.u64(f + 0x68b0) + (i64::from(e) * 0x2cc) as u64;
        let (x, z) = (vm.f32(slot(0x648)), vm.f32(slot(0x658)));
        let rel = [
            x - vm.f32(engine + 0x4c),
            z - vm.f32(engine + 0x50),
            y - vm.f32(engine + 0x54),
        ];
        let (a4, a0, a9) = (
            vm.f32(part + 0x7a4) * RAD,
            vm.f32(part + 0x7a0) * RAD,
            vm.f32(part + 0x79c) * RAD,
        );
        let r = rotate_pairs_b(
            rel[0],
            rel[1],
            rel[2],
            [a9.sin(), a9.cos(), a0.sin(), a0.cos(), a4.sin(), a4.cos()],
        );
        vm.set_f32(slot(0x18), r[0]);
        vm.set_f32(slot(0x70u64.wrapping_neg()), r[1]);
        vm.set_f32(slot(0x650), r[2]);
        let u = r[2];
        if !above(f64::from(u.abs()), 0.01) {
            continue;
        }
        let m278 = vm.f32(state + 0x278);
        if !above(f64::from(m278.abs()), 0.01) {
            continue;
        }
        if crate::scalar::sign(u) != crate::scalar::sign(m278) {
            continue;
        }
        let ratio = vm.f32(b + 0x950) / f32::from_bits(0x40490fdb);
        let s9 = if 0.0 > ratio { f32::NAN } else { ratio.sqrt() };
        let d7 = f64::from(u.abs()) * 0.05;
        let x1 = (f64::from(s9) + d7) as f32;
        let x7 = d7 as f32;
        let x8 = s9 * s9 * m278 / (x1 * x1);
        let h = hypot2(r[0], r[1]);
        let spread = sse_max(h - s9, 0.0);
        let floor = (f64::from(s9) * 0.01) as f32;
        let width = sse_max(x7, floor);
        let decay = signed_pow(2.0, spread / width);
        let strength = x8 / decay;
        if !above(f64::from(strength), 0.1) {
            continue;
        }
        let obj = part + 0x700;
        let angles = [vm.f32(obj + 0x9c), vm.f32(obj + 0xa0), vm.f32(obj + 0xa4)];
        let offsets = [vm.f32(obj + 0x90), vm.f32(obj + 0x94), vm.f32(obj + 0x98)];
        let w = rotate_euler_offset(angles, offsets, false, 0.0, 0.0, strength);
        vm.set_f32(slot(0x670), w[0]);
        vm.set_f32(slot(0x660), w[1]);
        vm.set_f32(slot(0x640), w[2]);
        for s in [0x670, 0x660, 0x640] {
            check(vm, slot(s));
        }
        for (target, s) in [(out1, 0x670), (out2, 0x660), (out3, 0x640)] {
            vm.set_f32(target, vm.f32(slot(s)) + vm.f32(target));
        }
        for target in [out1, out2, out3] {
            check(vm, target);
        }
    }
    if stop == Some(Stop::Jets) {
        return Ok(());
    }
    parts(vm, env, f, rbp, out1, out2, out3)?;
    if stop == Some(Stop::Parts) {
        return Ok(());
    }
    Ok(())
}

/// A finite-value guard of the original: a non-finite value becomes zero.
fn finite(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

/// `0x14117e266..0x14117f79f`: the propeller slipstream of the parts: the point relative to the part is rotated into
/// the propeller's axes (`rotate_pairs_b`), the normalized radius `rho` and axial position `zeta` give the swirl and
/// thrust velocity profiles scaled by the part state `N+0x80/0x84` (`N = F+0x68c8`), and the result is rotated back
/// (`0x14120cf60`) and added to the three outputs.
fn parts(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    rbp: u64,
    out1: u64,
    out2: u64,
    out3: u64,
) -> Result<(), String> {
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    let b = vm.u64(f + 0x20);
    let (excluded_part, excluded_body) = (vm.i32(slot(0x678)), vm.i32(slot(0x688)));
    for p in 0..vm.i32(b + 0x920) {
        if p == excluded_part {
            continue;
        }
        let part = vm.u64(b + 0x6010) + (i64::from(p) * 0x3770) as u64;
        if vm.i32(part) == 6 {
            continue;
        }
        if !crate::controls::held_back(vm, env, f, p, 1) {
            continue;
        }
        let part = vm.u64(b + 0x6010) + (i64::from(p) * 0x3770) as u64;
        let (x, z, y) = (
            vm.f32(slot(0x648)),
            vm.f32(slot(0x658)),
            vm.f32(slot(0x668)),
        );
        let rel = [
            x - vm.f32(part + 0x790),
            z - vm.f32(part + 0x794),
            y - vm.f32(part + 0x798),
        ];
        let (a4, a0, a9) = (
            vm.f32(part + 0x7a4) * RAD,
            vm.f32(part + 0x7a0) * RAD,
            vm.f32(part + 0x79c) * RAD,
        );
        let (c4, s4, c0, s0, c9, s9) = (a4.cos(), a4.sin(), a0.cos(), a0.sin(), a9.cos(), a9.sin());
        let r = rotate_pairs_b(rel[0], rel[1], rel[2], [s9, c9, s0, c0, s4, c4]);
        vm.set_f32(slot(0x18), r[0]);
        vm.set_f32(slot(0xb0), r[1]);
        vm.set_f32(slot(-0x70), r[2]);
        let w = vm.f32(slot(0x18));
        let v6 = vm.f32(slot(0xb0));
        let perp = {
            let sum = w * w + v6 * v6;
            if 0.0 > sum { f32::NAN } else { sum.sqrt() }
        };
        vm.set_f32(slot(0xa0), perp);
        let radius = vm.f32(part + 0x98);
        let rho = perp / radius;
        vm.set_f32(slot(0x28), rho);
        let zeta = vm.f32(slot(-0x70)) / radius;
        vm.set_f32(slot(0x670), zeta);
        if 1.25 <= rho || rho.is_nan() {
            continue;
        }
        let q = ((1.0 - f64::from(rho)) * f64::from(rho)) as f32;
        let shaped = if 0.0 > q {
            0.0
        } else {
            signed_pow(if 1.0 < q { 1.0 } else { q }, 0.25)
        };
        let tau = (f64::from(shaped) * 1.25) as f32;
        vm.set_f32(slot(0x24), tau);
        let n = vm.u64(f + 0x68c8) + (i64::from(p) * 0x388) as u64;
        let mut thrust = tau * vm.f32(n + 0x80);
        if excluded_body != -1 {
            thrust = 0.0;
        }
        thrust = finite(thrust);
        vm.set_f32(slot(0x640), vm.f32(n + 0x84));
        check(vm, slot(0x640));
        let edge = 1.0 - (rho - 0.75 + (rho - 0.75));
        let fade = crate::scalar::clamp(edge, 0.0, 1.0);
        vm.set_f32(slot(0x30), fade);
        let rot = crate::transform::rotate_pairs(
            vm.f32(out1),
            vm.f32(out2),
            vm.f32(out3),
            [s9, c9, s0, c0, s4, c4],
        );
        vm.set_f32(slot(-0x80), rot[0]);
        vm.set_f32(slot(0x80), rot[1]);
        vm.set_f32(slot(0x20), rot[2]);
        let swirl = vm.f32(slot(0x20));
        let swirl_n = vm.f32(slot(0x640));
        let blend = |source: f32| {
            let s = if 0.0 > source { -1.0f32 } else { 1.0 };
            let (lo, hi) = (-s, s);
            let t = 1.0 / (hi - lo) * (zeta - lo) + 0.0;
            crate::scalar::clamp(t, 0.0, 1.0)
        };
        let a = blend(swirl);
        let bb = blend(swirl_n);
        let sq_a = swirl * swirl;
        let sq_b = swirl_n * swirl_n;
        let denom = sse_max(sq_b + sq_a, 0.01);
        let k = if swirl_n == 0.0 {
            27.5
        } else {
            let t = 45.0 / swirl_n * swirl + 5.0;
            crate::scalar::clamp(t, 5.0, 50.0)
        };
        let decay = 2.0f32.powf((zeta / k).abs());
        let value = finite((sq_b * bb / denom + sq_a * a / denom) / decay);
        for target in [out1, out2, out3] {
            if !vm.f32(target).is_finite() {
                vm.set_f32(target, 0.0);
            }
        }
        let snapped = crate::scalar::snap(vm.f32(slot(0xa0)), f32::from_bits(0xbc23d70a), 0.01);
        let fade = vm.f32(slot(0x30));
        let v670 = thrust * vm.f32(slot(0xb0)) / snapped * fade * value;
        vm.set_f32(slot(0x670), v670);
        let v640 = neg32(thrust) * vm.f32(slot(0x18)) / snapped * fade * value;
        vm.set_f32(slot(0x640), v640);
        vm.set_f32(slot(0x30), fade * swirl_n * value);
        for s in [0x670, 0x640, 0x30] {
            check(vm, slot(s));
        }
        let obj = part + 0x700;
        let angles = [vm.f32(obj + 0x9c), vm.f32(obj + 0xa0), vm.f32(obj + 0xa4)];
        let offsets = [vm.f32(obj + 0x90), vm.f32(obj + 0x94), vm.f32(obj + 0x98)];
        let o = rotate_euler_offset(
            angles,
            offsets,
            false,
            vm.f32(slot(0x670)),
            vm.f32(slot(0x640)),
            vm.f32(slot(0x30)),
        );
        vm.set_f32(slot(0x660), o[0]);
        vm.set_f32(slot(0x650), o[1]);
        vm.set_f32(slot(0x90), o[2]);
        for s in [0x660, 0x650, 0x90] {
            check(vm, slot(s));
        }
        for (target, s) in [(out1, 0x660), (out2, 0x650), (out3, 0x90)] {
            vm.set_f32(target, vm.f32(slot(s)) + vm.f32(target));
        }
        for target in [out1, out2, out3] {
            check(vm, target);
        }
    }
    Ok(())
}

fn neg32(v: f32) -> f32 {
    f32::from_bits(v.to_bits() ^ 0x8000_0000)
}
