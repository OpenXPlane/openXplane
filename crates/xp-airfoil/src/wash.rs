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
    /// After the wing loop (`0x1411819c2`).
    Wings,
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
    wings(vm, f, rbp, out1, out2, out3)?;
    if stop == Some(Stop::Wings) {
        return Ok(());
    }
    shadow(vm, env, f, rbp, out1, out2, out3)
}

/// `0x1411819c2..0x141181f0d`: the shadow of the bodies (`0x141186930`, `shadow::body_shadow`: it stores a flag at
/// `rbp+0x670` and a value at `rbp+0x674`) scales the three outputs: by `sqrt(1/2)` when the point is inside a body,
/// otherwise by `sqrt(1 - min(1, value))` when the value is positive; a factor of 1 or more changes nothing.
fn shadow(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    rbp: u64,
    out1: u64,
    out2: u64,
    out3: u64,
) -> Result<(), String> {
    let slot = |off: u64| rbp.wrapping_add(off);
    let (x, z, y) = (
        vm.f32(slot(0x648)),
        vm.f32(slot(0x658)),
        vm.f32(slot(0x668)),
    );
    let body = vm.i32(slot(0x688));
    crate::shadow::body_shadow(vm, env, f, slot(0x670), x, out1, z, out2, y, out3, body)?;
    shadow_scale(vm, rbp, out1, out2, out3);
    Ok(())
}

/// The part of the tail after the shadow function: the scale factor from the flag and value it stored.
pub fn shadow_scale(vm: &mut Vm, rbp: u64, out1: u64, out2: u64, out3: u64) {
    let slot = |off: u64| rbp.wrapping_add(off);
    let inside = vm.i32(slot(0x670)) != 0;
    let value = vm.f32(slot(0x674));
    let factor = if inside {
        (0.5f64.sqrt()) as f32
    } else if above(f64::from(value), 0.0) {
        let m = if 1.0 < value { 1.0 } else { value };
        let d = 1.0 - f64::from(m);
        (if d < 0.0 { f64::NAN } else { d.sqrt() }) as f32
    } else {
        return;
    };
    if !above(1.0, f64::from(factor)) {
        return;
    }
    for target in [out1, out2, out3] {
        vm.set_f32(target, factor * vm.f32(target));
        check(vm, target);
    }
}

/// `0x14117f79f..0x1411819c2`: the wake of the wings: for every wing but the excluded one (`rbp+0x680`) that is
/// enabled and lies above the point, the elements whose chordwise extent contains the point's `x` give a downwash
/// (tangents of two angles from the profile thickness and the wing's loading, blended by the body blend factor)
/// subtracted from the second output and a decay factor scaling all three.
fn wings(vm: &mut Vm, f: u64, rbp: u64, out1: u64, out2: u64, out3: u64) -> Result<(), String> {
    use crate::callees::{body_blend, wing_area_factor};
    use crate::scalar::{clamp, snap};
    use crate::wing_element::boundary_at;
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    let b = vm.u64(f + 0x20);
    let speed = {
        let (a, c, d) = (vm.f32(out1), vm.f32(out2), vm.f32(out3));
        let sum = a * a + c * c + d * d;
        if 0.0 > sum { f32::NAN } else { sum.sqrt() }
    };
    vm.set_f32(slot(0x660), speed);
    let ex_wing = i64::from(vm.i32(slot(0x680)));
    let ex_body = vm.i32(slot(0x688));
    let (x, z, y) = (
        vm.f32(slot(0x648)),
        vm.f32(slot(0x658)),
        vm.f32(slot(0x668)),
    );
    for j in 0..0x30i64 {
        if j == ex_wing {
            continue;
        }
        let w = vm.u64(b + 0x6028) + (j * 0x36c8) as u64;
        if vm.u8(w + 0x678) == 0 || !above(f64::from(y), f64::from(vm.f32(w + 0x710))) {
            continue;
        }
        let xs = vm.u64(f + 0x6940) + (j * 0x2d8) as u64;
        let n = vm.i32(w + 4);
        if n <= 0 {
            continue;
        }
        let read = |vm: &Vm, base: u64| -> Vec<f32> {
            (0..=n as u64).map(|i| vm.f32(w + base + 4 * i)).collect()
        };
        let (ax, ay, az) = (read(vm, 0x5bc), read(vm, 0x5e8), read(vm, 0x614));
        for e in 0..n {
            let e_f = e as f32;
            let x0 = boundary_at(&ax, n, e_f);
            let x1 = boundary_at(&ax, n, (f64::from(e_f) + 1.0) as f32);
            let lo = if x0 < x1 { x0 } else { x1 };
            let hi = if x0 > x1 { x0 } else { x1 };
            if x < lo || x.is_nan() || lo.is_nan() || hi <= x || hi.is_nan() {
                continue;
            }
            let chord = vm.f32(w + 0x70 + 4 * e as u64);
            let ratio = vm.f32(w + 0x74 + 4 * e as u64) / chord;
            let theta = vm.f32(w + 0x16c + 4 * e as u64) * RAD;
            let cos_t = theta.cos();
            let pos = (f64::from(e_f) + 0.5) as f32;
            let r1 = f64::from(ratio) + 1.0;
            let taper = (f64::from(ratio * ratio) + r1) / r1;
            let mean = (taper * (f64::from(chord) * f64::from_bits(0x3fe5555555555555))) as f32;
            let reach = f64::from(mean) * 0.75;
            let zc = (f64::from(boundary_at(&az, n, pos)) + f64::from(cos_t) * reach) as f32;
            if !above(f64::from(y), f64::from(zc)) {
                continue;
            }
            let yc = (f64::from(boundary_at(&ay, n, pos)) - f64::from(theta.sin()) * reach) as f32;
            vm.set_f32(slot(0x640), yc);
            let span = vm.f32(w + 0x36a8);
            let dyn_ = f64::from((y - zc) / span);
            let f4 = vm.f32(xs + 0xf4 + 4 * e as u64);
            let dzn = f64::from((z - yc) / span);
            let s = (dzn + dyn_ * 0.1 * f64::from(f4)) as f32;
            let w650 = (1.0 / f64::from(2.0f32.powf((dyn_ / 1.5) as f32))) as f32;
            let w28 = (1.0 / f64::from(2.0f32.powf((f64::from(s.abs()) / 0.45) as f32))) as f32;
            let mut sq = 1.0f32;
            let area = |vm: &Vm, wing: u64| wing_area_factor(vm, wing);
            if ex_wing >= 0 {
                let w2 = vm.u64(b + 0x6028) + (ex_wing * 0x36c8) as u64;
                let t = area(vm, w) / area(vm, w2);
                let t = clamp(t, 0.0, 1.0);
                sq = if 0.0 > t { f32::NAN } else { t.sqrt() };
            }
            if ex_body >= 0 {
                let r = vm.u64(b + 0x6040) + (i64::from(ex_body) * 0x34c8) as u64;
                let t = area(vm, w) / (vm.f32(r + 0x6c) - vm.f32(r + 0x60));
                let t = clamp(t, 0.0, 1.0);
                sq = if 0.0 > t { f32::NAN } else { t.sqrt() };
            }
            let w24 = vm.f32(w + 0x144 + 4 * e as u64);
            let t290 = vm.f32(xs + 0x290);
            let amp = (f64::from(w650)
                * 0.75
                * f64::from(w28)
                * f64::from(sq)
                * f64::from(w24)
                * f64::from(t290)) as f32;
            let tilt = vm.f32(f + 0x524);
            let limit = clamp(tilt, -90.0, 90.0);
            let mut w1 = amp * limit;
            let distance = (y - zc).abs();
            let q = distance / snap(vm.f32(out3), f32::from_bits(0xbc23d70a), 0.01);
            let t1 = clamp(q, 0.0, 1.0);
            w1 *= t1;
            let q2 = q - 0.5;
            let u = 1.0 - (q2 + q2);
            let t2 = clamp(u, 0.0, 1.0);
            let k10 = f64::from(f4) * 10.0;
            let v0 = (f64::from(t2) * k10) as f32;
            let v1 = ((f64::from(amp) * k10 - f64::from(w1)) * f64::from(t2)) as f32;
            let xa = vm.f32(xs + 4 + 4 * e as u64);
            let denom = (if xa > 1.0 { xa } else { 1.0 }) * f32::from_bits(0x40490fdb);
            let v6 = (f64::from(w24 * f4) * 1.62 / f64::from(denom)) as f32;
            let e_n = ((f64::from(e as f32) + 0.5) as f32) / (n as f32);
            let foil = |vm: &Vm, offset: u64| -> u64 { vm.u64(w + offset) };
            let (p1, p2, p3) = (foil(vm, 0x3678), foil(vm, 0x3680), foil(vm, 0x3688));
            let thickness = if vm.f32(w + 0x60) > e_n && p1 != 0 && p2 != 0 {
                crate::wing_element::interpolate_clamped(
                    vm.f32(w + 0x5c),
                    vm.f32(p1 + 0x58),
                    vm.f32(w + 0x60),
                    vm.f32(p2 + 0x58),
                    e_n,
                )
            } else if e_n > vm.f32(w + 0x64) && p2 != 0 && p3 != 0 {
                crate::wing_element::interpolate_clamped(
                    vm.f32(w + 0x64),
                    vm.f32(p2 + 0x58),
                    vm.f32(w + 0x68),
                    vm.f32(p3 + 0x58),
                    e_n,
                )
            } else if p2 != 0 {
                vm.f32(p2 + 0x58)
            } else {
                0.0
            };
            let mach = speed / vm.f32(f + 0x74);
            let blend = body_blend(mach, thickness, f4);
            let lerp = |a: f32, c: f32| {
                let low = if a < c { a } else { c };
                let high = if a > c { a } else { c };
                let t = (c - a) * blend + a;
                if low > t {
                    low
                } else if high < t {
                    high
                } else {
                    t
                }
            };
            let angle_a = finite(lerp(v0, v6));
            let angle_b = finite(lerp(v1, v6));
            let tan_a = (angle_a * RAD).tan();
            let tan_b = (angle_b * RAD).tan();
            vm.set_f32(out2, vm.f32(out2) - tan_b * speed);
            let (tan_a, tan_b) = (finite(tan_a), finite(tan_b));
            for target in [out1, out2, out3] {
                check(vm, target);
            }
            if vm.i32(f + 0xbcc8) != 0 || vm.i32(f + 0xbcd0) != 0 {
                return Err("debug dump not ported".into());
            }
            let x16c = vm.f32(xs + 0x16c + 4 * e as u64);
            if !above(f64::from(x16c), 0.0) {
                continue;
            }
            let sum = (tan_b + tan_a) * (y - zc);
            let w650b = (f64::from(yc) - f64::from(sum) * 0.5) as f32;
            vm.set_f32(slot(0x650), w650b);
            let chord_e = vm.f32(w + 0x70 + 4 * e as u64);
            let dz = (z - w650b).abs() / chord_e;
            let r6 = (y - zc) / chord_e;
            let root = |v: f32| if 0.0 > v { f32::NAN } else { v.sqrt() };
            let x11 = (f64::from(root(x16c)) * 1.25 * f64::from(root(r6))) as f32;
            if !above(f64::from(r6), 0.0) || !above(f64::from(x11), f64::from(dz)) {
                continue;
            }
            let k = (f64::from(root(clamp(x16c, 0.0, 1.0))) / (f64::from(r6) * 4.0 + 1.0)) as f32;
            let c = (dz * f32::from_bits(0x3fc90fdb) / x11).cos();
            let d = 1.0 - f64::from(c * k);
            let factor = (if d < 0.0 { f64::NAN } else { d.sqrt() }) as f32;
            for target in [out1, out2, out3] {
                vm.set_f32(target, factor * vm.f32(target));
                check(vm, target);
            }
        }
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
