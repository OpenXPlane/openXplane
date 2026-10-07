//! The terrain mesh probe of the reference build (`tools/gen_gear_drag_vectors.py`, `tools/gen_flight_block_vectors.py`).
//!
//! A mesh object holds an array of triangles of nine floats (`x y z` of three vertices, 36 bytes) at the pointer
//! `+0x28`: the ground triangles are the range `+0x7c..+0x80` and the hole triangles the range `+0x74..+0x78`
//! (their `x z` pairs are tested in double precision). `+0x84` is the top height of the mesh.

use crate::vm::{CallArgs, Callees, Vm};

const TRIANGLE: u64 = 0x24;

/// `0x14195cf10(mesh, origin, direction, t)`: whether the point `origin + t * direction` (the `x` and `z` words at
/// `+0`/`+8` of each vector, float32 arithmetic) lies inside one of the hole triangles; a triangle edge of zero
/// length rejects the triangle and the point is inside when all three edge products are not positive.
pub fn inside_hole(vm: &Vm, mesh: u64, origin: [f32; 3], direction: [f32; 3], t: f32) -> bool {
    let x = f64::from(t * direction[0] + origin[0]);
    let z = f64::from(t * direction[2] + origin[2]);
    let first = vm.i32(mesh + 0x74);
    let count = vm.i32(mesh + 0x78).wrapping_sub(first);
    if count == 0 {
        return false;
    }
    let base = vm
        .u64(mesh + 0x28)
        .wrapping_add((i64::from(first) * TRIANGLE as i64) as u64);
    // the loop is a do-while: a negative count wraps like the original's 32-bit counter
    for k in 0..u64::from(count as u32) {
        let t = base.wrapping_add(k * TRIANGLE);
        let c = |i: u64| f64::from(vm.f32(t + 4 * i));
        let (x0, z0, x1, z1, x2, z2) = (c(0), c(2), c(3), c(5), c(6), c(8));
        let edge = |ax: f64, az: f64, bx: f64, bz: f64| -> Option<f64> {
            let (dx, dz) = (bx - ax, bz - az);
            if dx == 0.0 && dz == 0.0 {
                return None;
            }
            Some((x - ax) * dz - (z - az) * dx)
        };
        let Some(e0) = edge(x0, z0, x1, z1) else {
            continue;
        };
        if e0 > 0.0 || e0.is_nan() {
            continue;
        }
        let Some(e1) = edge(x1, z1, x2, z2) else {
            continue;
        };
        if e1 > 0.0 || e1.is_nan() {
            continue;
        }
        let Some(e2) = edge(x2, z2, x0, z0) else {
            continue;
        };
        if 0.0 >= e2 {
            return true;
        }
    }
    false
}

/// `0x14194dbe0(grid, x, z)`: whether the point is within the cell size `+8` of the cell origin that `+0x48` points to.
pub fn in_cell(vm: &Vm, grid: u64, x: f32, z: f32) -> bool {
    let cell = vm.u64(grid + 0x48);
    if cell == 0 {
        return false;
    }
    let limit = vm.f32(grid + 8);
    // `comiss`/`ja` on x and `setbe` on z: an unordered (NaN) comparison passes both
    let outside = |d: f32| d > limit;
    !outside((x - vm.f32(cell)).abs()) && !outside((z - vm.f32(cell + 4)).abs())
}

/// `0x141960dc0(mesh, p, out, normal, extra, grid)`: a vertical ray through `p` (`x y z` words) from 10 below its
/// height is intersected with the ground triangles (Moeller-Trumbore with the direction `(0, 1, 0)`, the nearest
/// non-negative hit). `false` when `p.y` is above the mesh top `+0x84`, no triangle is hit, the hit lies in a hole
/// (`inside_hole`), or the hit (with the grid height `0x14194dc20` added when `grid` covers it, replayed) is below
/// `p.y`. Otherwise `out` receives the hit point and `normal` (when nonzero) the triangle normal `0x1406ed6a0`
/// (replayed); a nonzero `extra` has its word `+8` copied to `+4` and `+0`.
#[allow(clippy::too_many_arguments)]
pub fn probe(
    vm: &mut Vm,
    env: &mut dyn Callees,
    mesh: u64,
    p: u64,
    out: u64,
    normal: u64,
    extra: u64,
    grid: u64,
) -> bool {
    let (px, py, pz) = (vm.f32(p), vm.f32(p + 4), vm.f32(p + 8));
    if py > vm.f32(mesh + 0x84) {
        return false;
    }
    let origin_y = py - 10.0;
    let first = vm.i32(mesh + 0x7c);
    let count = vm.i32(mesh + 0x80).wrapping_sub(first);
    if count == 0 {
        return false;
    }
    let base = vm
        .u64(mesh + 0x28)
        .wrapping_add((i64::from(first) * TRIANGLE as i64) as u64);
    let mut best = -1.0f32;
    let mut hit: Option<u64> = None;
    for k in 0..u64::from(count as u32) {
        let tri = base.wrapping_add(k * TRIANGLE);
        let o = |i: u64| vm.f32(tri + 4 * i);
        let a1 = o(1) - o(7);
        let a2 = o(2) - o(8);
        let b0 = o(0) - o(6);
        let b3 = o(3) - o(6);
        let c1 = o(4) - o(7);
        let c2 = o(5) - o(8);
        let t0 = a1 * 0.0;
        let t1 = a2 * 0.0;
        let q = a2 - t0;
        let r = t0 - b0;
        let s = b0 * 0.0 - t1;
        let det = (s * c1 + b3 * q) + r * c2;
        if det == 0.0 {
            continue;
        }
        let inv = 1.0 / det;
        let dx = px - o(6);
        let dz = pz - o(8);
        let dy = origin_y - o(7);
        let u = ((s * dy + dx * q) + dz * r) * inv;
        // a NaN `u` passes both `comiss` tests
        #[allow(clippy::manual_range_contains)]
        if 0.0 > u || u > 1.0 {
            continue;
        }
        let m5 = c2 * dy - dz * c1;
        let m4 = dx * c1 - b3 * dy;
        let m2 = dz * b3 - dx * c2;
        let v = ((m5 * 0.0 + m2) + m4 * 0.0) * inv;
        if 0.0 > v || v + u > 1.0 {
            continue;
        }
        let t = ((m2 * a1 + m5 * b0) + m4 * a2) * inv;
        if 0.0 > t || t.is_nan() {
            continue;
        }
        if t > best {
            best = t;
            hit = Some(tri);
        }
    }
    let Some(tri) = hit else { return false };
    if inside_hole(vm, mesh, [px, origin_y, pz], [0.0, 1.0, 0.0], best) {
        return false;
    }
    if out != 0 {
        let zero = best * 0.0;
        let height = best + origin_y;
        let (x, z) = (zero + px, zero + pz);
        vm.set_f32(out + 4, height);
        vm.set_f32(out, x);
        vm.set_f32(out + 8, z);
        let mut y = height;
        if grid != 0 && in_cell(vm, grid, x, z) {
            let mut args = CallArgs::ints(&[grid]);
            args.xmm = [None, Some(x.to_bits()), Some(z.to_bits()), None];
            let reply = env.call(vm, 0x14194dc20, args);
            y = f32::from_bits(reply.xmm0 as u32) + height;
        }
        vm.set_f32(out + 4, y);
        if py > y {
            return false;
        }
    }
    if normal != 0 {
        let args = CallArgs::ints(&[tri, tri + 0xc, tri + 0x18, normal]);
        env.call(vm, 0x1406ed6a0, args);
    }
    if extra != 0 {
        let w = vm.f32(extra + 8);
        vm.set_f32(extra + 4, w);
        vm.set_f32(extra, w);
    }
    true
}
