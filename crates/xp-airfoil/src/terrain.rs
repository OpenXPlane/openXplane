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
        let tri = base.wrapping_add(k * TRIANGLE);
        if triangle_contains(vm, tri, x, z) {
            return true;
        }
    }
    false
}

/// The point `(x, z)` against the `x z` pairs of a triangle in double precision (the edge products must not be
/// positive; an all-zero edge rejects the triangle).
pub fn triangle_contains(vm: &Vm, tri: u64, x: f64, z: f64) -> bool {
    let c = |i: u64| f64::from(vm.f32(tri + 4 * i));
    let (x0, z0, x1, z1, x2, z2) = (c(0), c(2), c(3), c(5), c(6), c(8));
    let edge = |ax: f64, az: f64, bx: f64, bz: f64| -> Option<f64> {
        let (dx, dz) = (bx - ax, bz - az);
        if dx == 0.0 && dz == 0.0 {
            return None;
        }
        Some((x - ax) * dz - (z - az) * dx)
    };
    let Some(e0) = edge(x0, z0, x1, z1) else {
        return false;
    };
    if e0 > 0.0 || e0.is_nan() {
        return false;
    }
    let Some(e1) = edge(x1, z1, x2, z2) else {
        return false;
    };
    if e1 > 0.0 || e1.is_nan() {
        return false;
    }
    let Some(e2) = edge(x2, z2, x0, z0) else {
        return false;
    };
    0.0 >= e2
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

/// The vertical ray `(px, py, pz) + t (0, 1, 0)` against a triangle (Moeller-Trumbore, float32): `t` when the
/// determinant is nonzero and the barycentric coordinates lie in the unit triangle (a NaN coordinate passes).
fn vertical_t(vm: &Vm, tri: u64, px: f32, py: f32, pz: f32) -> Option<f32> {
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
        return None;
    }
    let inv = 1.0 / det;
    let dx = px - o(6);
    let dz = pz - o(8);
    let dy = py - o(7);
    let u = ((s * dy + dx * q) + dz * r) * inv;
    // a NaN `u` passes both `comiss` tests
    #[allow(clippy::manual_range_contains)]
    if 0.0 > u || u > 1.0 {
        return None;
    }
    let m5 = c2 * dy - dz * c1;
    let m4 = dx * c1 - b3 * dy;
    let m2 = dz * b3 - dx * c2;
    let v = ((m5 * 0.0 + m2) + m4 * 0.0) * inv;
    if 0.0 > v || v + u > 1.0 {
        return None;
    }
    Some(((m2 * a1 + m5 * b0) + m4 * a2) * inv)
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
        let Some(t) = vertical_t(vm, tri, px, origin_y, pz) else {
            continue;
        };
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

const NOISE: u64 = 0x1_4578_f1f0;
/// The global `0x14611ac88`: the surface records (36 bytes each) that the triangle ids select.
const SURFACES: u64 = 0x1_4611_ac88;

/// `t` passes the `comiss` tests `t < 0` and `1 < t` (a NaN fails the first).
fn in_unit(t: f32) -> bool {
    !(t < 0.0 || t.is_nan() || 1.0 < t)
}

fn point(vm: &Vm, address: u64) -> [f32; 3] {
    [vm.f32(address), vm.f32(address + 4), vm.f32(address + 8)]
}

/// `0x140984e50(x, y, seed)` on the process's noise table (see `buffet::NoiseTable::basis2`).
fn noise2(vm: &Vm, x: f32, y: f32, seed: i32) -> f32 {
    let (ax, ay) = (x.abs(), y.abs());
    let (ix, iy) = (ax as i64 as u32, ay as i64 as u32);
    let base = (iy << 9)
        .wrapping_add(ix)
        .wrapping_add((seed as u32).wrapping_mul(100));
    let v = |offset: u32| vm.f32(NOISE + 4 * u64::from(base.wrapping_add(offset) & 0x3ffff));
    let fx = ax - ix as f32;
    let fy = ay - iy as f32;
    let low = (fx * v(1) + (1.0 - fx) * v(0)) * (1.0 - fy);
    let high = ((1.0 - fx) * v(0x200) + fx * v(0x201)) * fy;
    (f64::from(low + high) * 2.0 - 1.0) as f32
}

/// `0x14195ecd0(record, x, z)`: the roughness height of a surface record: six octaves (frequencies 2, 1, 4, 8, 16,
/// 32 times `record+0` and weights 0.5, 1, 0.25, 0.125, 0.0625, 0.03125) of the noise table, scaled by `record+4`
/// (zero when that is zero).
pub fn roughness(vm: &Vm, record: u64, x: f32, z: f32) -> f32 {
    let scale = vm.f32(record + 4);
    if scale == 0.0 {
        return 0.0;
    }
    let freq = vm.f32(record);
    let (fz, fx) = (freq * z, freq * x);
    let (dz, dx) = (f64::from(fz), f64::from(fx));
    let mut sum = f64::from(noise2(vm, (dx + dx) as f32, (dz + dz) as f32, 0)) * 0.5;
    sum += f64::from(noise2(vm, fx, fz, 0));
    for (factor, weight) in [(4.0, 0.25), (8.0, 0.125), (16.0, 0.0625), (32.0, 0.03125)] {
        sum += f64::from(noise2(vm, (dx * factor) as f32, (dz * factor) as f32, 0)) * weight;
    }
    (sum as f32) * scale
}

/// `0x141962990(p0, p1, sphere)`: whether the segment `p0 -> p1` meets the sphere `(x y z r)`.
pub fn segment_sphere(vm: &Vm, p0: u64, p1: u64, sphere: u64) -> bool {
    let [px, py, pz] = point(vm, p0);
    let [cx, cy, cz] = point(vm, sphere);
    let r2 = vm.f32(sphere + 0xc) * vm.f32(sphere + 0xc);
    let (dx, dy, dz) = (cx - px, cy - py, cz - pz);
    if r2 > (dy * dy + dx * dx) + dz * dz {
        return true;
    }
    let [qx, qy, qz] = point(vm, p1);
    let (wx, wz, wy) = (px - cx, pz - cz, py - cy);
    let (sx, sz, sy) = (qx - px, qz - pz, qy - py);
    let b = (sy * wy + sx * wx) + sz * wz;
    let cc = ((wy * wy + wx * wx) + wz * wz) - r2;
    if cc > 0.0 && b > 0.0 {
        return false;
    }
    let disc = b * b - cc;
    if 0.0 > disc {
        return false;
    }
    let nb = -b;
    let t = if disc == 0.0 {
        if 0.0 > nb {
            return false;
        }
        nb
    } else {
        let root = if 0.0 > disc { f32::NAN } else { disc.sqrt() };
        let (t1, t2) = (nb - root, root - b);
        if 0.0 > t2 {
            return false;
        }
        if (t1 >= 0.0 || t1.is_nan()) && t2 >= 0.0 {
            t1
        } else if 0.0 <= t1 || t1.is_nan() || t2 < 0.0 {
            return false;
        } else {
            t2
        }
    };
    t <= 1.0 || t.is_nan()
}

/// `0x141964470(v2, v1, v0, p0, d)`: the parameter at which the segment `p0 + t d` crosses the triangle
/// (Moeller-Trumbore; a back-facing or degenerate triangle or a miss gives -1).
pub fn segment_triangle(vm: &Vm, v2: u64, v1: u64, v0: u64, p0: u64, d: [f32; 3]) -> f32 {
    let [v2x, v2y, v2z] = point(vm, v2);
    let [v1x, v1y, v1z] = point(vm, v1);
    let [v0x, v0y, v0z] = point(vm, v0);
    let (e1x, e1y, e1z) = (v0x - v2x, v0y - v2y, v0z - v2z);
    let (e2x, e2y, e2z) = (v1x - v2x, v1y - v2y, v1z - v2z);
    let [dx, dy, dz] = d;
    let cx = e1z * e2y - e1y * e2z;
    let cy = e1x * e2z - e1z * e2x;
    let cz = e1y * e2x - e1x * e2y;
    if (cx * dx + cy * dy) + cz * dz > 0.0 {
        return -1.0;
    }
    let hx = e1z * dy - e1y * dz;
    let hy = e1x * dz - e1z * dx;
    let hz = e1y * dx - e1x * dy;
    let det = (hy * e2y + hx * e2x) + hz * e2z;
    if det == 0.0 {
        return -1.0;
    }
    let inv = 1.0 / det;
    let [px, py, pz] = point(vm, p0);
    let (sx, sy, sz) = (px - v2x, py - v2y, pz - v2z);
    let u = ((sy * hy + sx * hx) + sz * hz) * inv;
    #[allow(clippy::manual_range_contains)]
    if 0.0 > u || u > 1.0 {
        return -1.0;
    }
    let qx = sy * e2z - sz * e2y;
    let qz = sx * e2y - sy * e2x;
    let qy = sz * e2x - sx * e2z;
    let v = ((qy * dy + qx * dx) + qz * dz) * inv;
    if 0.0 > v || v + u > 1.0 {
        return -1.0;
    }
    ((qy * e1y + qx * e1x) + qz * e1z) * inv
}

/// The cell triangle test of `0x14195ffc0`: the parameter of the segment `p0 + t d` (`t` in 0..1) when it enters the
/// front of the triangle, else `None`.
fn cell_t(vm: &Vm, tri: u64, p: [f32; 3], d: [f32; 3]) -> Option<f32> {
    let o = |i: u64| vm.f32(tri + 4 * i);
    let (e1x, e1y, e1z) = (o(0) - o(6), o(1) - o(7), o(2) - o(8));
    let (e2x, e2y, e2z) = (o(3) - o(6), o(4) - o(7), o(5) - o(8));
    let [dx, dy, dz] = d;
    let cx = e1z * e2y - e1y * e2z;
    let cy = e1x * e2z - e1z * e2x;
    let cz = e1y * e2x - e1x * e2y;
    if (cx * dx + cy * dy) + cz * dz > 0.0 {
        return None;
    }
    let hx = dy * e1z - dz * e1y;
    let hz = dx * e1y - dy * e1x;
    let hy = dz * e1x - dx * e1z;
    let det = (hy * e2y + hx * e2x) + hz * e2z;
    if det == 0.0 {
        return None;
    }
    let inv = 1.0 / det;
    let (sx, sy, sz) = (p[0] - o(6), p[1] - o(7), p[2] - o(8));
    let u = ((sx * hx + hy * sy) + sz * hz) * inv;
    #[allow(clippy::manual_range_contains)]
    if 0.0 > u || u > 1.0 {
        return None;
    }
    let qx = e2z * sy - sz * e2y;
    let qy = sz * e2x - sx * e2z;
    let qz = sx * e2y - e2x * sy;
    let v = ((qy * dy + qx * dx) + qz * dz) * inv;
    if 0.0 > v || v + u > 1.0 {
        return None;
    }
    let t = ((qy * e1y + qx * e1x) + qz * e1z) * inv;
    #[allow(clippy::manual_range_contains)]
    if !in_unit(t) {
        return None;
    }
    Some(t)
}

/// The id of the triangle at `tri` (index from the mesh's triangle array) from the id words at `mesh+0x40`.
fn triangle_index(vm: &Vm, mesh: u64, tri: u64) -> u64 {
    let diff = tri.wrapping_sub(vm.u64(mesh + 0x28)) as i64;
    ((diff >> 2) / 9) as u64
}

/// The arguments of `0x14195ffc0` beyond the segment: the outputs (each may be zero).
#[derive(Clone, Copy, Debug, Default)]
pub struct SegmentOut {
    /// The hit point (three floats).
    pub point: u64,
    /// The triangle normal (`0x1406ed6a0`, replayed).
    pub normal: u64,
    /// The hit point in the frame of the moving cell (three floats; zeros for the static mesh).
    pub moved: u64,
    /// The surface id (a 32-bit word).
    pub id: u64,
}

/// `0x14195ffc0(mesh, p0, p1, out, normal, moved, id)`: the first hit of the segment `p0 -> p1` with the mesh:
/// the moving cells (sphere-culled, records of 0x50 bytes at `+0xa8`, spheres at `+0x90`), then the ground
/// triangles `+0x78..+0x7c`, `+0x7c..+0x80` (accepted when the hole test passes) and `+0x70..+0x74`; with no hit
/// a downward segment is tried against the unflagged triangles `+0x70..+0x74` with a vertical ray.
pub fn segment_probe(
    vm: &mut Vm,
    env: &mut dyn Callees,
    mesh: u64,
    p0: u64,
    p1: u64,
    out: SegmentOut,
) -> bool {
    let [px, py, pz] = point(vm, p0);
    let [qx, qy, qz] = point(vm, p1);
    let top = vm.f32(mesh + 0x88);
    if py > top && qy > top {
        return false;
    }
    let d = [qx - px, qy - py, qz - pz];
    let mut best = 2.0f32;
    let mut hit: u64 = 0;
    let mut cell: u64 = 0;
    let cells = (vm.u64(mesh + 0xb0).wrapping_sub(vm.u64(mesh + 0xa8)) as i64) / 80;
    for i in 0..cells.max(0) as u64 {
        let sphere = vm.u64(mesh + 0x90).wrapping_add(16 * i);
        if !segment_sphere(vm, p0, p1, sphere) {
            continue;
        }
        let record = vm.u64(mesh + 0xa8).wrapping_add(0x50 * i);
        let first = vm.i32(record + 0x48);
        let count = vm.i32(record + 0x4c).wrapping_sub(first);
        let base = vm
            .u64(mesh + 0x28)
            .wrapping_add((i64::from(first.wrapping_mul(9)) * 4) as u64);
        for k in 0..u64::from(count as u32) {
            let tri = base.wrapping_add(k * TRIANGLE);
            if let Some(t) = cell_t(vm, tri, [px, py, pz], d)
                && best > t
            {
                best = t;
                hit = tri;
                cell = record;
            }
        }
    }
    let range = |vm: &Vm, at: u64| -> (u64, u32) {
        let first = vm.i32(mesh + at);
        let count = vm.i32(mesh + at + 4).wrapping_sub(first);
        let base = vm
            .u64(mesh + 0x28)
            .wrapping_add((i64::from(first.wrapping_mul(9)) * 4) as u64);
        (base, count as u32)
    };
    // `+0x78..+0x7c` clears the cell (r15) on every hit
    {
        let (base, count) = range(vm, 0x78);
        for k in 0..u64::from(count) {
            let tri = base.wrapping_add(k * TRIANGLE);
            let t = segment_triangle(vm, tri + 0x18, tri + 0xc, tri, p0, d);
            if in_unit(t) && best > t {
                best = t;
                hit = tri;
                cell = 0;
            }
        }
    }
    if hit != 0 {
        let at = |c: f32, o: f32| c * best + o;
        let (x, y0, z) = (at(d[0], px), at(d[1], py), at(d[2], pz));
        let idx = triangle_index(vm, mesh, hit);
        let id = vm.u32(vm.u64(mesh + 0x40).wrapping_add(4 * idx)) & 0x7fff;
        if out.point != 0 {
            vm.set_f32(out.point, x);
            vm.set_f32(out.point + 4, y0);
            vm.set_f32(out.point + 8, z);
        }
        if out.normal != 0 {
            env.call(
                vm,
                0x1406ed6a0,
                CallArgs::ints(&[hit, hit + 0xc, hit + 0x18, out.normal]),
            );
        }
        if out.id != 0 {
            vm.set_u32(out.id, id);
        }
        if out.point != 0 {
            let record = vm.u64(SURFACES).wrapping_add(36 * u64::from(id));
            let noise = roughness(vm, record, vm.f32(out.point), vm.f32(out.point + 8));
            vm.set_f32(out.point + 4, noise + vm.f32(out.point + 4));
        }
        if out.moved != 0 {
            if cell == 0 {
                vm.set_u32(out.moved, 0);
                vm.set_u64(out.moved + 4, 0);
            } else {
                moved_point(vm, cell, (x, y0, z), out.moved);
            }
        }
        return true;
    }
    // no hit among the cells and the first range: the second and third ranges
    let mut best8 = best;
    let mut found: u64 = 0;
    {
        let (base, count) = range(vm, 0x7c);
        for k in 0..u64::from(count) {
            let tri = base.wrapping_add(k * TRIANGLE);
            let t = segment_triangle(vm, tri + 0x18, tri + 0xc, tri, p0, d);
            if in_unit(t) && best8 > t {
                best8 = t;
                found = tri;
            }
        }
    }
    if found != 0 && inside_hole(vm, mesh, [px, py, pz], d, best8) {
        best = best8;
        hit = found;
    }
    {
        let (base, count) = range(vm, 0x70);
        for k in 0..u64::from(count) {
            let tri = base.wrapping_add(k * TRIANGLE);
            let t = segment_triangle(vm, tri + 0x18, tri + 0xc, tri, p0, d);
            if in_unit(t) && best > t {
                best = t;
                hit = tri;
            }
        }
    }
    if hit != 0 {
        let idx = triangle_index(vm, mesh, hit);
        let mut word = vm.u32(vm.u64(mesh + 0x40).wrapping_add(4 * idx));
        if out.point != 0 {
            vm.set_f32(out.point, d[0] * best + px);
            vm.set_f32(out.point + 4, d[1] * best + py);
            vm.set_f32(out.point + 8, d[2] * best + pz);
        }
        if out.normal != 0 {
            env.call(
                vm,
                0x1406ed6a0,
                CallArgs::ints(&[hit, hit + 0xc, hit + 0x18, out.normal]),
            );
        }
        if out.id != 0 {
            let (x, z) = (f64::from(d[0] * best + px), f64::from(d[2] * best + pz));
            let first = vm.i32(mesh + 0x74);
            let count = vm.i32(mesh + 0x78).wrapping_sub(first);
            let tris = vm
                .u64(mesh + 0x28)
                .wrapping_add((i64::from(first.wrapping_mul(9)) * 4) as u64);
            let flags = vm
                .u64(mesh + 0x40)
                .wrapping_add(4 * i64::from(first) as u64);
            let mut layer = 0i32;
            for k in 0..u64::from(count as u32) {
                let w = vm.u32(flags.wrapping_add(4 * k));
                let l = ((w >> 16) & 0x7fff) as i32;
                if l <= layer {
                    continue;
                }
                if triangle_contains(vm, tris.wrapping_add(k * TRIANGLE), x, z) {
                    layer = l;
                    word = w;
                }
            }
            vm.set_u32(out.id, word & 0x7fff);
        }
        if out.point != 0 && out.id != 0 {
            let record = vm.u64(SURFACES).wrapping_add(36 * u64::from(word & 0x7fff));
            let noise = roughness(vm, record, vm.f32(out.point), vm.f32(out.point + 8));
            vm.set_f32(out.point + 4, noise + vm.f32(out.point + 4));
        }
        if out.moved != 0 {
            vm.set_u32(out.moved, 0);
            vm.set_u64(out.moved + 4, 0);
        }
        return true;
    }
    if d[1] >= 0.0 || d[1].is_nan() {
        return false;
    }
    // a downward segment that missed everything: the vertical ray against the unflagged triangles
    let (base, count) = range(vm, 0x70);
    let flags = vm
        .u64(mesh + 0x40)
        .wrapping_add(4 * i64::from(vm.i32(mesh + 0x70)) as u64);
    let mut best = -1.0f32;
    let mut found: u64 = 0;
    let mut word = 0u32;
    for k in 0..u64::from(count) {
        let w = vm.u32(flags.wrapping_add(4 * k));
        if w & 0x8000 != 0 {
            continue;
        }
        let tri = base.wrapping_add(k * TRIANGLE);
        let Some(t) = vertical_t(vm, tri, px, py, pz) else {
            continue;
        };
        if 0.0 > t || t.is_nan() {
            continue;
        }
        if best == -1.0 || best > t {
            best = t;
            found = tri;
            word = w & 0x7fff;
        }
    }
    if found == 0 {
        return false;
    }
    if out.point != 0 {
        vm.set_f32(out.point, px);
        vm.set_f32(out.point + 4, py);
        vm.set_f32(out.point + 8, pz);
    }
    if out.normal != 0 {
        env.call(
            vm,
            0x1406ed6a0,
            CallArgs::ints(&[found, found + 0xc, found + 0x18, out.normal]),
        );
    }
    if out.id != 0 {
        vm.set_u32(out.id, word);
    }
    if out.moved != 0 {
        vm.set_u32(out.moved, 0);
        vm.set_u64(out.moved + 4, 0);
    }
    true
}

/// The hit point carried along a moving cell: relative to the cell origin `+0xc/+0x1c/+0x2c`, taken through the
/// rotation rows `+0/+4/+8`, `+0x10/+0x14/+0x18`, `+0x20/+0x24/+0x28`, turned by `+0x3c/+0x40/+0x44` and placed
/// again with the translation `+0x30/+0x34/+0x38` (all float32; the terms multiplied by zero are kept).
fn moved_point(vm: &mut Vm, cell: u64, hit: (f32, f32, f32), out: u64) {
    let f = |o: u64| vm.f32(cell + o);
    let (rx, ry, rz) = (hit.0 - f(0xc), hit.1 - f(0x1c), hit.2 - f(0x2c));
    let a = (f(0x10) * ry + f(0) * rx) + f(0x20) * rz;
    let b = (f(0x14) * ry + f(4) * rx) + f(0x24) * rz;
    let c = (f(0x18) * ry + f(8) * rx) + f(0x28) * rz;
    let s1 = b * f(0x3c) - c * f(0x44);
    let s2 = a * f(0x44) + b * f(0x40);
    let s3 = (-f(0x40)) * c - a * f(0x3c);
    let zero = 0.0f32;
    let x = ((s3 * f(4) + s1 * f(0)) + s2 * f(8)) + f(0xc) * zero + f(0x30);
    let y = ((s3 * f(0x14) + s1 * f(0x10)) + s2 * f(0x18)) + f(0x1c) * zero + f(0x34);
    let z = ((s3 * f(0x24) + s1 * f(0x20)) + s2 * f(0x28)) + f(0x2c) * zero + f(0x38);
    vm.set_f32(out, x);
    vm.set_f32(out + 4, y);
    vm.set_f32(out + 8, z);
}
