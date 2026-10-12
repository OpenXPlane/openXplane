//! The wind at a point (`0x141ba80a0`) with its noise helpers, compared with the machine code in an emulator
//! (`tools/gen_weather_vectors.py`).

use crate::engine::signed_pow;
use crate::flight_state::geodetic_to_local;
use crate::vm::{CallArgs, Callees, Vm};

const NOISE: u64 = 0x1_4578_f1f0;
const SCRATCH: u64 = 0x7e00_0600_0000;

fn minss(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// `x` held to `0..1` as the original does: `comiss; ja` against zero, then `minss` with one.
fn clamp01(x: f32) -> f32 {
    if 0.0 > x { 0.0 } else { minss(1.0, x) }
}

/// `0x1411e17a0(x, y, z, seed)`: the three-dimensional noise on the process's table: the cell is
/// `(|z| << 12) + (|y| << 6) + |x| + 100 seed` (indices wrap at 0x40000), the eight corners are blended
/// trilinearly and mapped from `0..1` to `-1..1` in double precision.
pub fn noise3(vm: &Vm, x: f32, y: f32, z: f32, seed: i32) -> f32 {
    let (ax, ay, az) = (x.abs(), y.abs(), z.abs());
    let (ix, iy, iz) = (ax as i64 as u32, ay as i64 as u32, az as i64 as u32);
    let fx = ax - ix as f32;
    let fy = ay - iy as f32;
    let fz = az - iz as f32;
    let (ox, oy, oz) = (1.0 - fx, 1.0 - fy, 1.0 - fz);
    let base = (iz << 12)
        .wrapping_add(iy << 6)
        .wrapping_add((seed as u32).wrapping_mul(100))
        .wrapping_add(ix);
    let v = |offset: u32| vm.f32(NOISE + 4 * u64::from(base.wrapping_add(offset) & 0x3ffff));
    let row = |dz: u32, dy: u32| v(dz + dy) * ox + v(dz + dy + 1) * fx;
    let p1 = row(0x1000, 0) * oy + row(0x1000, 0x40) * fy;
    let p0 = row(0, 0x40) * fy + row(0, 0) * oy;
    let r = p1 * fz + p0 * oz;
    (f64::from(r) * 2.0 - 1.0) as f32
}

/// `0x141244c10(_, a, b, c, seed)`: three octaves of [`noise3`]: `0.5 n(2p) + n(p) + 0.25 n(4p)` (double
/// accumulation).
pub fn turbulence(vm: &Vm, a: f32, b: f32, c: f32, seed: i32) -> f32 {
    let (da, db, dc) = (f64::from(a), f64::from(b), f64::from(c));
    let n1 = noise3(
        vm,
        (da + da) as f32,
        (db + db) as f32,
        (dc + dc) as f32,
        seed,
    );
    let mut sum = f64::from(n1) * 0.5;
    sum += f64::from(noise3(vm, a, b, c, seed));
    let n4 = noise3(
        vm,
        (da * 4.0) as f32,
        (db * 4.0) as f32,
        (dc * 4.0) as f32,
        seed,
    );
    sum += f64::from(n4) * 0.25;
    sum as f32
}

fn ssqrt(x: f32) -> f32 {
    if x >= 0.0 {
        if x.is_nan() { f32::NAN } else { x.sqrt() }
    } else {
        -((-x).sqrt())
    }
}

/// `0x141ba80a0(W, x, y, z, out1, out2, out3)`: the wind at the world position: the base wind `W+0x14..0x1c`, the
/// rotating gust around the centre of the wake `W+0x30..` while the runtime flag `0x142f01898` exceeds 0.99,
/// the two turbulence scales scaled by the gust factor from `W+4` and the globals `0x14632b870` / `0x14632b820`,
/// and the storm cell at the geographic point `0x142fe8500..` (influence by the time globals `0x142fe84e8`,
/// `0x142fe84f0` and `0x142f01918`). The three results go to the pointers (added to the base wind).
#[allow(clippy::too_many_arguments)]
pub fn wind_at(vm: &mut Vm, env: &mut dyn Callees, w: u64, x: f64, y: f64, z: f64, out: [u64; 3]) {
    let d = |vm: &Vm, o: u64| f64::from(vm.f32(w + o));
    for (k, o) in [0x14u64, 0x18, 0x1c].into_iter().enumerate() {
        vm.set_u32(out[k], vm.u32(w + o));
    }
    let add = |vm: &mut Vm, k: usize, v: f64| {
        let sum = (v + f64::from(vm.f32(out[k]))) as f32;
        vm.set_f32(out[k], sum);
    };
    if vm.i32(w + 0x2c) != 0 && f64::from_bits(vm.u64(0x1_42f0_1898)) > 0.99 {
        let (c48, c4c, c50, c54, c58, c5c) = (
            d(vm, 0x48),
            d(vm, 0x4c),
            d(vm, 0x50),
            d(vm, 0x54),
            d(vm, 0x58),
            d(vm, 0x5c),
        );
        let dz = z - d(vm, 0x40);
        let dx = x - d(vm, 0x30);
        let u1 = dz * c48 + dx * c4c;
        let u2 = dz * c4c - dx * c48;
        let u3 = u2 * c50 + (y - d(vm, 0x38)) * c54;
        let v1 = u1 * c58 + u3 * c5c;
        let v2 = u1 * c5c - u3 * c58;
        let (f1, f2) = (v1 as f32, v2 as f32);
        let len = {
            let s = f2 * f2 + f1 * f1;
            if 0.0 > s { f32::NAN } else { s.sqrt() }
        };
        let r = len / vm.f32(w + 0x60);
        if 1.0 > r {
            let g = vm.f32(0x1_42fe_7be8);
            let clamped = clamp01(g);
            let q = (1.0 - f64::from(r)) as f32;
            let p = signed_pow(q, 0.5);
            let mag = f64::from(clamped * vm.f32(w + 0x64) * p);
            let (nv2, a1) = (-v2 * mag, v1 * mag);
            let w1 = nv2 * c58 + a1 * c5c;
            let w2 = nv2 * c5c - a1 * c58;
            let w3 = w2 * c50 + c54 * 0.0;
            let w4 = w2 * c54 - c50 * 0.0;
            let ox = w3 * c4c + w1 * c48;
            let oz = w1 * c4c - w3 * c48;
            add(vm, 0, oz);
            add(vm, 1, w4);
            add(vm, 2, ox);
        }
    }
    let k5 = f32::from_bits(0x3f8c_cccd);
    let s = clamp01(vm.f32(w + 4));
    let g2 = vm.f32(0x1_4632_b870);
    let t = if 0.0 > g2 {
        Some(0.0f32)
    } else {
        let t = minss(1.0, g2);
        if t == k5 { None } else { Some(t) }
    };
    let base = match t {
        None => f32::from_bits(0x3f0c_cccd),
        Some(t) => {
            let v = k5 / (k5 - t) * (s - t) + 0.0;
            if 0.0 > v { 0.0 } else { minss(k5, v) }
        }
    };
    let factor = base.powf(vm.f32(0x1_4632_b820));
    if factor > 0.0 {
        let (px, py, pz) = (x, y, z);
        let a = (((px - d(vm, 0x20)) as f32).abs() as f64 * 0.01) as f32;
        let b = (((py - d(vm, 0x24)) as f32).abs() as f64 * 0.01) as f32;
        let c = (((pz - d(vm, 0x28)) as f32).abs() as f64 * 0.01) as f32;
        let r3 = turbulence(vm, a, b, c, 4);
        let r4 = turbulence(vm, a, b, c, 5);
        let r5 = turbulence(vm, a, b, c, 3);
        let (r3, r4, r5) = (r3 * factor, r4 * factor, r5 * factor);
        add(vm, 0, f64::from(r5) * 10.0);
        add(vm, 1, f64::from(r3) * 10.0);
        add(vm, 2, f64::from(r4) * 10.0);
        let a = (((px - d(vm, 0x20)) as f32).abs() as f64 * 0.05) as f32;
        let b = (((py - d(vm, 0x24)) as f32).abs() as f64 * 0.05) as f32;
        let c = (((pz - d(vm, 0x28)) as f32).abs() as f64 * 0.05) as f32;
        let r1 = turbulence(vm, a, b, c, 1);
        let r2 = turbulence(vm, a, b, c, 2);
        let r0 = turbulence(vm, a, b, c, 0);
        let (r0, r1, r2) = (r0 * factor, r1 * factor, r2 * factor);
        add(vm, 0, f64::from(r0) * 5.0);
        add(vm, 1, f64::from(r1) * 5.0);
        add(vm, 2, f64::from(r2) * 5.0);
    }
    let start = f64::from_bits(vm.u64(0x1_42fe_84e8));
    if start > 1.0 {
        let now = f64::from_bits(vm.u64(0x1_42f0_1918));
        let end = f64::from_bits(vm.u64(0x1_42fe_84f0));
        let outside = (start as f32) > (now as f32) || (now as f32) > (end as f32);
        if !outside {
            let planet = env.call(vm, 0x14193ae40, CallArgs::default()).rax;
            let geographic = [
                f64::from_bits(vm.u64(0x1_42fe_8500)),
                f64::from_bits(vm.u64(0x1_42fe_8508)),
                f64::from_bits(vm.u64(0x1_42fe_8510)),
            ];
            let slots = [SCRATCH, SCRATCH + 8, SCRATCH + 0x10];
            geodetic_to_local(vm, env, planet, slots, geographic);
            let local = [vm.f64(slots[0]), vm.f64(slots[1]), vm.f64(slots[2])];
            let dx = (x - local[0]) as f32;
            let dy = (y - local[1]) as f32;
            let dz = (z - local[2]) as f32;
            let h = {
                let s = dz * dz + dx * dx;
                if 0.0 > s { f32::NAN } else { s.sqrt() }
            };
            let t_in = clamp01(((now - start) / 15.0) as f32);
            let t_out = clamp01(((end - now) / 60.0) as f32);
            let strength = ssqrt(t_out * t_in);
            let height_term = clamp01(1.0 - (dy - 0.0) * f32::from_bits(0x39ae_c33e));
            let radial = clamp01(1.0 - (h - 0.0) * f32::from_bits(0x3979_a934));
            let amp = (f64::from(ssqrt(height_term))
                * (f64::from(strength) * 30.866_668_224_334_717)
                * f64::from(radial)) as f32;
            let k3 = f32::from_bits(0x453b_8000);
            let hs = (f64::from(h) * 0.1) as f32;
            let sh = if k3 == hs {
                0.5f32
            } else {
                clamp01(1.0 - 1.0 / (hs - k3) * (dy - k3))
            };
            let z = (h / k3 - 0.0) * f32::from_bits(0x3f66_6666) + f32::from_bits(0x3dcc_cccd);
            let e = if f32::from_bits(0x3dcc_cccd) > z {
                f32::from_bits(0x3dcc_cccd)
            } else {
                minss(1.0, z)
            };
            let pw = signed_pow(sh, e);
            let k8 = f32::from_bits(0xbfc9_0fdb);
            let t = k8 - (pw - 1.0) * f32::from_bits(0x3fc9_0fdb);
            let lat = if k8 > t {
                k8
            } else {
                if 0.0 < t { 0.0 } else { t }
            };
            let heading = dx.atan2(-dz);
            let horizontal = lat.cos() * amp;
            let vx = heading.sin() * horizontal;
            let vy = lat.sin() * amp;
            let vz = heading.cos() * horizontal;
            let o1 = vm.f32(out[0]) + vx;
            vm.set_f32(out[0], o1);
            let o2 = vm.f32(out[1]) + vy;
            vm.set_f32(out[1], o2);
            let o3 = vm.f32(out[2]) - vz;
            vm.set_f32(out[2], o3);
        }
    }
}
