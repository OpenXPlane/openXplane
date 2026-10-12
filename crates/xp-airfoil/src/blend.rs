//! The contact response of one quad of a body surface (`0x1411cb1e0`), compared with the machine code in an
//! emulator (`tools/gen_blend_vectors.py`). The NaN diagnostics (log strings) are not ported: they return an error.

use crate::engine::signed_pow;
use crate::flight_state::{add_plugin_force, local_axes_forward, to_world};
use crate::flight_step::position_component;
use crate::terrain::probe;
use crate::transform::{Frame, rotate_euler_offset, rotate_pairs, to_aircraft_frame};
use crate::vm::{CallArgs, Callees, Vm};
use crate::wing_element::interpolate_clamped;

const SCRATCH: u64 = 0x7e00_0500_0000;
const RAD: f32 = f32::from_bits(0x3c8e_fa36);
const DEG: f32 = f32::from_bits(0x4265_2ee0);
const KG: f32 = f32::from_bits(0x411c_c5c1);
const RECORDED_AIRCRAFT: u64 = 0x1_42f2_e3dc;
const LINES: u64 = 0x1_4612_5780;
const AIRCRAFT_TABLE: u64 = 0x1_4612_4398;

fn minss(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

fn maxss(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

fn sqrt_f(x: f32) -> f32 {
    if 0.0 > x { f32::NAN } else { x.sqrt() }
}

/// `0x1411764a0` / `0x1408e25a0`: a NaN or infinity at `address` is replaced by zero (the log message is not
/// ported).
fn repair(vm: &mut Vm, address: u64) {
    if !vm.f32(address).is_finite() {
        vm.set_f32(address, 0.0);
    }
}

/// `0x1411cb1e0(F, s, g, base, j, i0, i1, outs)`: the quad of the surface points `(i0, i1)` of `base` (vertices of
/// 12 bytes, 18 per row): its centre goes through the euler offsets of `g` and to the world, the terrain is
/// probed there and, one hundredth of a second ahead with the velocity of the point; a point below the ground
/// gets a force along the surface that is added as a plug-in force and to the outputs `outs[3]`, `outs[4]`; the
/// body point and the force position go to `outs[0..3]`. Returns whether there was a contact.
#[allow(clippy::too_many_arguments)]
pub fn blend_contact(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    s: u64,
    g: u64,
    base: u64,
    i0: i32,
    i1: i32,
    outs: [u64; 5],
) -> Result<bool, String> {
    blend_contact_in(vm, env, f, s, g, base, i0, i1, outs, SCRATCH)
}

/// [`blend_contact`] with the address of the frame (`rbp` of the original) given.
#[allow(clippy::too_many_arguments)]
pub fn blend_contact_in(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    s: u64,
    g: u64,
    base: u64,
    i0: i32,
    i1: i32,
    outs: [u64; 5],
    rbp: u64,
) -> Result<bool, String> {
    let n1 = 18 * (i64::from(i0) + 1) + i64::from(i1);
    let n0 = 18 * i64::from(i0) + i64::from(i1);
    let at = |vm: &Vm, idx: i64, off: u64| {
        vm.f32(base.wrapping_add(off).wrapping_add((idx * 12) as u64))
    };
    let (t1, t0, t4) = (n1, n0, n1 + 4);
    let half = |a: f32, b: f32| (f64::from(a + b) * 0.5) as f32;
    // the centre of the quad: y from entries +4, x from entry n1+4 and n0+4, z from entries +3
    let my = half(at(vm, t1, 0x34), at(vm, t0, 0x34));
    let mx = half(at(vm, t4, 0), at(vm, t0, 0x30));
    let mz = half(at(vm, t1, 0x2c), at(vm, t0, 0x2c));
    let angles = [vm.f32(g + 0x9c), vm.f32(g + 0xa0), vm.f32(g + 0xa4)];
    let offsets = [vm.f32(g + 0x90), vm.f32(g + 0x94), vm.f32(g + 0x98)];
    let [la, lb, lc] = rotate_euler_offset(angles, offsets, true, mz, mx, my);
    let pw = to_world(vm, env, f, [la, lb, lc]);
    for (k, v) in pw.iter().enumerate() {
        vm.set_f32(rbp - 0x70 + 4 * k as u64, *v);
    }
    let (p, hit1, normal1) = (rbp - 0x70, rbp - 0x60, rbp + 0x58);
    for k in 0..3 {
        vm.set_f32(hit1 + 4 * k, 0.0);
        vm.set_f32(normal1 + 4 * k, 0.0);
    }
    let grid = vm.u64(0x1461_179f8);
    let mesh = f + 0x42e40;
    if !probe(vm, env, mesh, p, hit1, normal1, 0, grid) {
        return Ok(false);
    }
    // the velocity of the point
    let w = |vm: &Vm, o: u64| vm.f32(f + o);
    let (w3cc, w3d4, w3d0) = (w(vm, 0x3cc), w(vm, 0x3d4), w(vm, 0x3d0));
    let m3 = w3cc * lb - w3d4 * lc;
    let m2 = ((-la) * w3cc) - w3d0 * lc;
    let m1 = w3d0 * lb + w3d4 * la;
    let pairs = [
        w(vm, 0x440),
        w(vm, 0x444),
        w(vm, 0x430),
        w(vm, 0x434),
        w(vm, 0x450),
        w(vm, 0x454),
    ];
    let rot = rotate_pairs(m3, m2, m1, pairs);
    let vx = rot[0] + w(vm, 0x368);
    let mut vy = rot[1] + w(vm, 0x36c);
    let vz = rot[2] + w(vm, 0x370);
    let p2 = [
        (f64::from(vx) * 0.01 + f64::from(pw[0])) as f32,
        (f64::from(vy) * 0.01 + f64::from(pw[1])) as f32,
        (f64::from(vz) * 0.01 + f64::from(pw[2])) as f32,
    ];
    let (p2a, hit2, normal2) = (rbp + 0x20, rbp + 0x48, rbp + 0x68);
    for (k, v) in p2.iter().enumerate() {
        vm.set_f32(p2a + 4 * k as u64, *v);
    }
    for k in 0..3 {
        vm.set_f32(hit2 + 4 * k, 0.0);
        vm.set_f32(normal2 + 4 * k, 0.0);
    }
    let grid = vm.u64(0x1461_179f8);
    let mut slope = 0.0f32;
    let mut hit1_y = vm.f32(hit1 + 4);
    let wy = pw[1];
    if probe(vm, env, mesh, p2a, hit2, normal2, 0, grid) {
        let d1 = hit1_y - wy;
        let rate = ((vm.f32(hit2 + 4) - p2[1]) - d1) as f64 * 100.0;
        let rate = rate as f32;
        let dy = (p2[1] - wy).abs();
        let (dz, dx) = (p2[2] - pw[2], p2[0] - pw[0]);
        let len = maxss(sqrt_f(dz * dz + dx * dx), f32::from_bits(0x3c23_d70a));
        slope = dy / len;
        vy = -rate;
    }
    if vm.i32(s + 0x2c) != 0 {
        let hs = sqrt_f(vx * vx + vz * vz);
        let denom = maxss(hs, 1.0);
        let k = vm.f32(s + 0x38) / denom;
        let lag = maxss((lc - vm.f32(s + 0x44)) * k, 0.0);
        if !lc.is_finite() {
            return Err("NaN diagnostics not ported".into());
        }
        repair(vm, s + 0x44);
        repair(vm, hit1 + 4);
        hit1_y = vm.f32(hit1 + 4) - lag;
        vm.set_f32(hit1 + 4, hit1_y);
        vy += vm.f32(s + 0x38);
        repair(vm, hit1 + 4);
        hit1_y = vm.f32(hit1 + 4);
    }
    if !hit1_y.is_finite() || !wy.is_finite() {
        return Err("NaN diagnostics not ported".into());
    }
    let d = hit1_y - wy;
    if 0.0 >= d {
        return Ok(false);
    }
    // the size of the quad in the aircraft frame
    let q = |a: f32, b: f32| (f64::from(a + b) * 0.25) as f32;
    let (r11, r9, r10, r8) = (n1, n1 + 1, n0, n0 + 1);
    let za = q(
        at(vm, r10, 0x20) - at(vm, r8, 0x2c),
        at(vm, r11, 0x20) - at(vm, r9, 0x2c),
    );
    let xa = q(
        at(vm, r10, 0x24) - at(vm, r8, 0x30),
        at(vm, r11, 0x24) - at(vm, r9, 0x30),
    );
    let ya =
        (f64::from((at(vm, r9, 0x34) - at(vm, r8, 0x34)) + (at(vm, r11, 0x28) - at(vm, r10, 0x28)))
            * 0.5) as f32;
    let [e1, e2, e3] = rotate_euler_offset(angles, offsets, false, za, xa, ya).map(f32::abs);
    let min11 = if 0.0 > e2 { 0.0 } else { minss(d, e2) };
    let yb = at(vm, t1, 0x34) - at(vm, t0, 0x34);
    let xb = at(vm, t4, 0) - at(vm, t0, 0x30);
    let zb = at(vm, t1, 0x2c) - at(vm, t0, 0x2c);
    let [o1, o2, o3] = rotate_euler_offset(angles, offsets, false, zb, xb, yb);
    let (c454, s450, c434, s430, c444, s440) = (
        w(vm, 0x454),
        w(vm, 0x450),
        w(vm, 0x434),
        w(vm, 0x430),
        w(vm, 0x444),
        w(vm, 0x440),
    );
    let pp = c454 * o1 + s450 * o2;
    let qq = c454 * o2 - s450 * o1;
    let rr = c434 * o3 + qq * s430;
    let outx = pp * c444 - rr * s440;
    let outy = qq * c434 - o3 * s430;
    let outz = rr * c444 + pp * s440;
    let hs = sqrt_f(vx * vx + vz * vz);
    let mut heading = (vx).atan2(-vz) * DEG;
    while 0.0 > heading {
        heading += 360.0;
    }
    while heading > 360.0 {
        heading += -360.0;
    }
    let pwr = if hs > 0.0 {
        hs.powf(1.75)
    } else if 0.0 > hs {
        -((-hs).powf(1.75))
    } else {
        0.0
    };
    let pw_speed = (f64::from(pwr * 1000.0) * 0.5) as f32;
    let ratio = outy
        / maxss(
            sqrt_f(outx * outx + outz * outz),
            f32::from_bits(0x3c23_d70a),
        );
    let clampv = if -0.25 > ratio {
        -0.25
    } else {
        minss(0.25, ratio)
    };
    let mut hrel = vm.f32(f + 0x3e0) - heading;
    while -180.0 > hrel {
        hrel += 360.0;
    }
    while hrel > 180.0 {
        hrel += -360.0;
    }
    let ang = e2.atan2(e1);
    let cosabs = ang.cos().abs();
    let sinabs = ang.sin().abs();
    let a12 = (min11 * e1).abs();
    let a8 = (e1 * e3).abs();
    let a11 = (min11 * e3).abs();
    let len12 = sqrt_f(e1 * e1 + e2 * e2);
    let ratio2 = d / e2;
    let r2 = if 0.0 > ratio2 {
        0.0
    } else {
        minss(1.0, ratio2)
    };
    let slide = len12 * e3 * r2;
    let a12 = a12 * hs * 1000.0;
    let fy = vy.abs() * a8 * 1000.0;
    let absclamp = clampv.abs();
    let t14 = (absclamp * hs - vy) * cosabs;
    let lat = signed_pow(cosabs, 0.5) * (-vy);
    let p3 = f64::from(signed_pow(absclamp, 3.0));
    let interp = interpolate_clamped(
        0.0,
        f32::from_bits(0x3c23_d70a),
        f32::from_bits(0x3d4c_cccd),
        f32::from_bits(0x3ca3_d70a),
        slope,
    );
    let mut x13 = (f64::from(interp) + (p3 + p3)) as f32;
    let r7 = hrel * RAD;
    let sn = r7.sin();
    let sq = sn * sn;
    let sg = if sn >= 0.0 { sq } else { -sq };
    let cs = r7.cos().abs();
    let mut x11 = a11 * pw_speed;
    x11 *= sinabs;
    x11 *= sg;
    let x7 = cs * x11;
    let mut x8 = a8 * d;
    x8 *= 1000.0;
    x8 *= KG;
    x8 = maxss(x8, 0.0);
    let m = lat * fy + t14 * a12;
    x13 *= slide;
    x13 *= pw_speed;
    if !x7.is_finite() || !x8.is_finite() {
        return Err("NaN diagnostics not ported".into());
    }
    let slot = |vm: &mut Vm, off: u64, v: f32| {
        vm.set_f32(rbp + off, v);
        repair(vm, rbp + off);
        vm.f32(rbp + off)
    };
    let _ = slot(vm, 0x248, m);
    let _ = slot(vm, 0x250, x13);
    let n = vm.f32(rbp + 0x250);
    let mval = vm.f32(rbp + 0x248);
    let a = x7;
    vm.set_f32(rbp + 0x220, a);
    let pval = maxss(mval + x8, 0.0);
    vm.set_f32(rbp + 0x238, pval);
    vm.set_f32(rbp + 0x248, n);
    repair(vm, rbp + 0x220);
    repair(vm, rbp + 0x238);
    repair(vm, rbp + 0x248);
    let (a, pval, n) = (
        vm.f32(rbp + 0x220),
        vm.f32(rbp + 0x238),
        vm.f32(rbp + 0x248),
    );
    let hr = heading * RAD;
    let (ch, sh) = (hr.cos(), hr.sin());
    let new248 = ch * a - sh * n;
    let new250 = pval;
    let new220 = ch * n + sh * a;
    vm.set_f32(rbp + 0x248, new248);
    vm.set_f32(rbp + 0x250, new250);
    vm.set_f32(rbp + 0x220, new220);
    repair(vm, rbp + 0x248);
    repair(vm, rbp + 0x250);
    repair(vm, rbp + 0x220);
    let (u, v, wv) = (
        vm.f32(rbp + 0x248),
        vm.f32(rbp + 0x250),
        vm.f32(rbp + 0x220),
    );
    // the world axes (the planet matrix at the latitude and longitude of the aircraft)
    let planet = env.call(vm, 0x14193ae40, CallArgs::default()).rax;
    let lon = position_component(vm, env, f, 0x398);
    let latd = position_component(vm, env, f, 0x390);
    let mat = rbp + 0x80;
    local_axes_forward(vm, env, planet, mat, latd, lon);
    let m = |vm: &Vm, r: u64, c: u64| vm.f32(mat + 16 * r + 4 * c);
    let a1 = v * m(vm, 1, 0) + m(vm, 0, 0) * u + wv * m(vm, 2, 0);
    let a2 = v * m(vm, 1, 1) + m(vm, 0, 1) * u + wv * m(vm, 2, 1);
    let a3 = m(vm, 1, 2) * v + m(vm, 0, 2) * u + wv * m(vm, 2, 2);
    vm.set_f32(rbp + 0x248, a1);
    vm.set_f32(rbp + 0x250, a2);
    vm.set_f32(rbp + 0x220, a3);
    let frame = Frame {
        origin: [vm.f64(f + 0x378), vm.f64(f + 0x380), vm.f64(f + 0x388)],
        rotation: [
            [vm.f32(f + 0x430), vm.f32(f + 0x434)],
            [vm.f32(f + 0x440), vm.f32(f + 0x444)],
            [vm.f32(f + 0x450), vm.f32(f + 0x454)],
        ],
    };
    let fo = to_aircraft_frame(&frame, [a1, a2, a3], false, false);
    repair(vm, rbp + 0x248);
    repair(vm, rbp + 0x250);
    repair(vm, rbp + 0x220);
    add_plugin_force(vm, env, f, la, fo[0], lb, fo[1], lc, fo[2])?;
    if vm.i32(f + 0x28) == vm.i32(RECORDED_AIRCRAFT) {
        let idx = vm.i32(0x1_4265_52ac);
        let b = vm
            .u64(AIRCRAFT_TABLE)
            .wrapping_add((i64::from(idx) * 0x6668) as u64);
        let sp = sqrt_f(
            (vm.f32(b + 0x64f8) * vm.f32(b + 0x64f8) + vm.f32(b + 0x64f4) * vm.f32(b + 0x64f4))
                + vm.f32(b + 0x64fc) * vm.f32(b + 0x64fc),
        );
        let scale = (f64::from(sp) * 10.0 / f64::from(vm.f32(b + 0x2898) * KG)) as f32;
        let (hx, hy, hz) = (vm.f32(hit1), vm.f32(hit1 + 4), vm.f32(hit1 + 8));
        let tail = [0u32, 1.0f32.to_bits(), 1.0f32.to_bits(), 1.0f32.to_bits()];
        let tail3 = [0u32, 1.0f32.to_bits(), 0, 1.0f32.to_bits()];
        let lines = [
            ([hx - 0.5, hy, hz, hx + 0.5, hy, hz], tail),
            ([hx, hy, hz - 0.5, hx, hy, hz + 0.5], tail),
            (
                [
                    hx,
                    hy,
                    hz,
                    scale * vm.f32(rbp + 0x248) + hx,
                    scale * vm.f32(rbp + 0x250) + hy,
                    scale * vm.f32(rbp + 0x220) + hz,
                ],
                tail3,
            ),
        ];
        for (floats, words) in lines {
            let mut record = [0u32; 10];
            for k in 0..6 {
                record[k] = floats[k].to_bits();
            }
            record[6..].copy_from_slice(&words);
            let cursor = vm.u64(LINES + 8);
            if cursor != vm.u64(LINES + 16) {
                for (k, w) in record.iter().enumerate() {
                    vm.set_u32(cursor + 4 * k as u64, *w);
                }
                vm.set_u64(LINES + 8, cursor + 0x28);
            } else {
                let tmp = rbp + 0x48;
                for (k, w) in record.iter().enumerate() {
                    vm.set_u32(tmp + 4 * k as u64, *w);
                }
                let mut args = CallArgs::ints(&[LINES, cursor]);
                args.int[2] = None;
                env.call(vm, 0x141167000, args);
            }
        }
    }
    vm.set_f32(outs[0], la);
    vm.set_f32(outs[1], lb);
    vm.set_f32(outs[2], lc);
    let o3 = vm.f32(outs[3]) + mval;
    vm.set_f32(outs[3], o3);
    let o4 = vm.f32(outs[4]) + a12;
    vm.set_f32(outs[4], o4);
    Ok(true)
}
