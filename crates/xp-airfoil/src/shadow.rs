//! The shadow of the bodies on a point of the airflow: the original's `0x141186930` and its ray-box test
//! `0x141296c40` (`research/FLIGHT_STEP.md`).

use crate::vm::{CallArgs, Callees, Vm};

/// `a > b`, false when unordered (`comiss; ja`).
fn above(a: f64, b: f64) -> bool {
    a > b
}

/// `comiss; jb`: below, or unordered.
fn below_or_unordered(a: f32, b: f32) -> bool {
    a < b || a.is_nan() || b.is_nan()
}

/// The size of a body record and the number of bodies (`B+0x6040`).
const BODY_STRIDE: u64 = 0x34c8;
const BODIES: u64 = 39;

/// The slab test with the original's operand selection: `lo = (a - o) * inv` and `hi = (b - o) * inv` per axis, the
/// entry time is the largest of the per-axis minima and the exit time the smallest of the per-axis maxima (every
/// `cmov` keeps the first operand when the comparison is unordered), and the ray hits when the exit time is not
/// negative and not before the entry time. The fourth component of the vectors never contributes.
pub fn slab(a: [f32; 3], b: [f32; 3], o: [f32; 3], inv: [f32; 3]) -> bool {
    let mut lo = [0.0f32; 3];
    let mut hi = [0.0f32; 3];
    for k in 0..3 {
        lo[k] = (a[k] - o[k]) * inv[k];
        hi[k] = (b[k] - o[k]) * inv[k];
    }
    let min = |k: usize| if lo[k] > hi[k] { hi[k] } else { lo[k] };
    let max = |k: usize| if hi[k] > lo[k] { hi[k] } else { lo[k] };
    let max12 = if max(1) > max(2) { max(2) } else { max(1) };
    let exit = if max12 < max(0) { max12 } else { max(0) };
    if below_or_unordered(exit, 0.0) {
        return false;
    }
    let min12 = if min(2) > min(1) { min(2) } else { min(1) };
    let enter = if min12 > min(0) { min12 } else { min(0) };
    !below_or_unordered(exit, enter)
}

/// `0x141296c40(a, b, o, inv)`: four floats at each address.
pub fn ray_box(vm: &Vm, a: u64, b: u64, o: u64, inv: u64) -> bool {
    let v = |p: u64| [vm.f32(p), vm.f32(p + 4), vm.f32(p + 8)];
    slab(v(a), v(b), v(o), v(inv))
}

fn bind(env: &mut dyn Callees, f: u64, id: u32, index: u32) -> bool {
    let args = CallArgs::ints(&[f, 1, u64::from(id), u64::from(index)]);
    env.call(&mut Vm::default(), 0x1407ace10, args).rax as u32 != 0
}

/// A hit of the ray `p + t d` with a triangle of nine floats (Moeller-Trumbore, float32 in the original's operation
/// order): 0 for a miss, 1 for `t >= 0`, 2 for `t < 0`.
fn triangle_hit(vm: &Vm, at: u64, p: [f32; 3], d: [f32; 3]) -> u32 {
    let t = |i: u64| vm.f32(at + 4 * i);
    let (t0, t1, t2) = (t(0), t(1), t(2));
    let e1 = [t(3) - t0, t(4) - t1, t(5) - t2];
    let e2 = [t(6) - t0, t(7) - t1, t(8) - t2];
    let p0 = e2[2] * d[1] - e2[1] * d[2];
    let p1 = e2[0] * d[2] - e2[2] * d[0];
    let p2 = e2[1] * d[0] - e2[0] * d[1];
    let det = p1 * e1[1] + p0 * e1[0] + p2 * e1[2];
    if det == 0.0 {
        return 0;
    }
    let inv = 1.0 / det;
    let (tx, ty, tz) = (p[0] - t0, p[1] - t1, p[2] - t2);
    let u = (tx * p0 + ty * p1 + tz * p2) * inv;
    #[allow(clippy::manual_range_contains)] // a NaN passes the original's comparisons
    let outside = 0.0 > u || u > 1.0;
    if outside {
        return 0;
    }
    let q0 = ty * e1[2] - tz * e1[1];
    let q1 = tz * e1[0] - tx * e1[2];
    let q2 = tx * e1[1] - ty * e1[0];
    let v = (q1 * d[1] + q0 * d[0] + q2 * d[2]) * inv;
    if 0.0 > v || v + u > 1.0 {
        return 0;
    }
    let s = (q1 * e2[1] + q0 * e2[0] + q2 * e2[2]) * inv;
    if 0.0 > s { 2 } else { 1 }
}

/// `0x141186930(F, result, x, &out1, z, &out2, y, &out3, excluded_body)`: casts the ray from the point `(x, z, y)`
/// against the bodies (all but the excluded one, and only those whose `+0x10` is above the excluded body's). The
/// ray runs along `-out`: a body whose box contains the point or is crossed by the ray is tested with the triangles
/// of its 32 mesh boxes; crossings forward and backward both found (3) put the point inside, which sets
/// `result[0] = 1`, while one forward crossing only (1) adds the body's `+4` to the accumulated value. The value is
/// stored at `result[1]`.
#[allow(clippy::too_many_arguments)]
pub fn body_shadow(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    result: u64,
    x: f32,
    out1: u64,
    z: f32,
    out2: u64,
    y: f32,
    out3: u64,
    excluded: i32,
) -> Result<(), String> {
    let d = [-vm.f32(out1), -vm.f32(out2), -vm.f32(out3)];
    let inv = [1.0 / d[0], 1.0 / d[1], 1.0 / d[2]];
    let neg = [-inv[0], -inv[1], -inv[2]];
    let mut value = 0.0f32;
    let bodies = vm.u64(vm.u64(f + 0x20) + 0x6040);
    let debug = |vm: &Vm| vm.i32(f + 0xbcd0) != 0 || vm.i32(f + 0xbcc8) != 0;
    for j in 0..BODIES {
        let body = bodies + j * BODY_STRIDE;
        let index = vm.u32(body + 0x5f0);
        if index <= 0x26 && bind(env, f, 0x179, index) {
            continue;
        }
        if vm.i32(body + 0x54) != 0 || vm.u8(body + 0x588) == 0 || j as i64 == i64::from(excluded) {
            continue;
        }
        if excluded >= 0 {
            let other = bodies + (excluded as u64) * BODY_STRIDE;
            if !above(
                f64::from(vm.f32(body + 0x10)),
                f64::from(vm.f32(other + 0x10)),
            ) {
                continue;
            }
        }
        let min = [
            vm.f32(body + 0x58),
            vm.f32(body + 0x5c),
            vm.f32(body + 0x60),
        ];
        let max = [
            vm.f32(body + 0x64),
            vm.f32(body + 0x68),
            vm.f32(body + 0x6c),
        ];
        let p = [
            x - vm.f32(body + 0x618),
            z - vm.f32(body + 0x61c),
            y - vm.f32(body + 0x620),
        ];
        let outside = min[1] > p[1]
            || min[2] > p[2]
            || min[0] > p[0]
            || p[1] > max[1]
            || p[2] > max[2]
            || p[0] > max[0];
        if outside && !slab(min, max, p, inv) {
            continue;
        }
        let inside = !outside;
        let mut bits = 0u32;
        'boxes: for k in 0..32u64 {
            let r = body + 0xa0 + 0x28 * k;
            let count = vm.i32(r + 0xc);
            if count == 0 {
                continue;
            }
            let lo = [vm.f32(r - 0x18), vm.f32(r - 0x14), vm.f32(r - 0x10)];
            let hi = [vm.f32(r - 8), vm.f32(r - 4), vm.f32(r)];
            let hit = slab(lo, hi, p, inv) || (inside && slab(lo, hi, p, neg));
            if !hit {
                continue;
            }
            let start = vm.i32(r + 8).wrapping_mul(9);
            let mut at = vm
                .u64(body + 0x70)
                .wrapping_add((i64::from(start) * 4) as u64);
            for _ in 0..count {
                bits |= triangle_hit(vm, at, p, d);
                if bits == 3 {
                    if debug(vm) {
                        return Err("debug log not ported".into());
                    }
                    vm.set_i32(result, 1);
                    vm.set_f32(result + 4, value);
                    return Ok(());
                }
                if bits == 1 && !inside {
                    break 'boxes;
                }
                at += 0x24;
            }
        }
        if bits == 1 {
            if debug(vm) {
                return Err("debug log not ported".into());
            }
            value += vm.f32(body + 4);
        }
    }
    vm.set_i32(result, 0);
    vm.set_f32(result + 4, value);
    Ok(())
}
