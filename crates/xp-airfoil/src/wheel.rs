//! Helpers of the wheel contact model `0x1411c8690`, compared with the machine code in an emulator
//! (`tools/gen_small_vectors.py`, kinds `caster` and `servo`).

use crate::binding::input_active;
use crate::flight_state::{add_plugin_force, local_to_geodetic, world_point_f64};
use crate::flight_step::position_component;
use crate::strut::record_contact;
use crate::terrain::{SegmentOut, noise2, segment_probe};
use crate::tire::{Tire, aircraft_object, slip_limit, time, tire_motion};
use crate::transform::{Frame, rotate_pairs_f64, to_aircraft_frame};
use crate::vm::{CallArgs, Callees, Vm};
use crate::wing_element::interpolate_clamped;

/// The scratch frame standing in for the locals of the original whose addresses go to callees.
const SCRATCH: u64 = 0x7e00_0400_0000;
const AIRCRAFT_TABLE: u64 = 0x1_4612_4398;
const SURFACES: u64 = 0x1_4611_ac80;
const NOISE: u64 = 0x1_4578_f1f0;
const RECORDED_AIRCRAFT: u64 = 0x1_42f2_e3dc;
/// The four wetness/roughness globals (`0x142fe8518 .. 0x142fe8524`).
const GLOBALS: [u64; 4] = [0x1_42fe_8518, 0x1_42fe_851c, 0x1_42fe_8520, 0x1_42fe_8524];
const RAD: f32 = f32::from_bits(0x3c8e_fa36);
const DEG: f32 = f32::from_bits(0x4265_2ee0);
const KG: f32 = f32::from_bits(0x411c_c5c1);
const KNOT: f32 = f32::from_bits(0x3f03_b2a2);
const KNOT_D: f64 = 0.514_444_470_405_578_6;

/// `minss` / `maxss`: the destination when it compares less / greater, otherwise the source.
fn minss(dst: f32, src: f32) -> f32 {
    if dst < src { dst } else { src }
}

fn maxss(dst: f32, src: f32) -> f32 {
    if dst > src { dst } else { src }
}

/// `sqrtss` with the original's slow path for a negative operand (a NaN).
fn sqrt_f(x: f32) -> f32 {
    if 0.0 > x { f32::NAN } else { x.sqrt() }
}

fn bobj(vm: &Vm, aircraft: i32) -> u64 {
    vm.u64(AIRCRAFT_TABLE)
        .wrapping_add((i64::from(aircraft) * 0x6668) as u64)
}

/// `0x141240d40(F, gear)`: whether the gear leg is steered by its caster/lock: false while `F+0x214` is set, for a
/// gear record (`[B+0x6080] + 0x88 gear`) of kind <= 1, and unless one of its two limits (`+0x50`, `+0x54`)
/// exceeds 0.01 or `+0xc` is set; then true for a negative `+0x7c` unless `+0xc` is clear while `F+0x204` is
/// set, and for a positive `+0x7c` when `F+0x210` is below 0.5.
pub fn caster_locked(vm: &Vm, f: u64, gear: i32) -> bool {
    if vm.i32(f + 0x214) != 0 {
        return false;
    }
    let b = vm.u64(f + 0x20);
    let g = vm
        .u64(b + 0x6080)
        .wrapping_add((i64::from(gear) * 0x88) as u64);
    if vm.u32(g) <= 1 {
        return false;
    }
    let limit = 0.01f64;
    if !(f64::from(vm.f32(g + 0x50)) > limit
        || f64::from(vm.f32(g + 0x54)) > limit
        || vm.i32(g + 0xc) != 0)
    {
        return false;
    }
    let x = vm.f32(g + 0x7c);
    if 0.0 > x && (vm.i32(g + 0xc) != 0 || vm.i32(f + 0x204) == 0) {
        return true;
    }
    if x > 0.0 {
        return 0.5 > vm.f32(f + 0x210);
    }
    false
}

/// `0x1410c9620(a, k, &state, b, lo, hi, free)`: one step of the servo that moves `state` toward `a`: the
/// value is `a + (state - b) k / 0.025` limited to `lo..hi`; the rate `a - value` (limited to the magnitude of
/// that correction unless `free`) divided by `k` is integrated into `state` over the frame time. Returns the
/// limited value.
#[allow(clippy::too_many_arguments)]
pub fn servo(
    vm: &mut Vm,
    env: &mut dyn Callees,
    a: f32,
    k: f32,
    state: u64,
    b: f32,
    lo: f32,
    hi: f32,
    free: i32,
) -> f32 {
    let diff = (vm.f32(state) - b) * k;
    let d = f64::from(diff) / 0.025;
    let v = (f64::from(a) + d) as f32;
    let value = if lo > v {
        lo
    } else if hi < v {
        hi
    } else {
        v
    };
    let rate = a - value;
    let magnitude = (d as f32).abs();
    let step = if free != 0 {
        rate
    } else if -magnitude > rate {
        -magnitude
    } else if magnitude < rate {
        magnitude
    } else {
        rate
    };
    let dt = time(vm, env);
    let moved = f64::from(step / k);
    vm.set_f32(state, (f64::from(vm.f32(state)) + dt * moved) as f32);
    value
}

/// `0x141240fc0(aircraft, p, u, w, angle, dir, out1, out2, out3)`: the point `(p, u, w)` carried through the
/// orientation of a wheel axis and moved into the aircraft frame (`0x141296750` without the origin shift).
///
/// The orientation is the identity turned by `atan2(dir[2], dir[1])` degrees about `(1, 0, 0)`, by
/// `atan2(dir[0], dir[1])` degrees about `(0, 0, -1)` and by `angle` degrees about `(0, -1, 0)` (float matrices,
/// `0x140cb9900`). Three angles are taken from the result as in [`crate::matrix::euler_from_matrix`] (with the
/// pole case giving a zero third angle), each goes through degrees to radians in float32, and their sines and
/// cosines rotate the point; the rotated point goes to the aircraft frame of the flight object.
pub fn wheel_axis_point(
    vm: &Vm,
    aircraft: i32,
    p: f32,
    u: f32,
    w: f32,
    angle: f32,
    dir: [f64; 3],
) -> [f32; 3] {
    use crate::matrix::{identity_f32, rotate_by_f32};
    use crate::transform::{Frame, to_aircraft_frame};
    const DEGREES_D: f64 = 57.295_776_367_187_5;
    const DEGREES: f32 = f32::from_bits(0x4265_2ee0);
    const RADIANS: f32 = f32::from_bits(0x3c8e_fa36);
    let first = (dir[2].atan2(dir[1]) * DEGREES_D) as f32;
    let second = (dir[0].atan2(dir[1]) * DEGREES_D) as f32;
    let mut m = identity_f32();
    m = rotate_by_f32(&m, first, 1.0, 0.0, 0.0);
    m = rotate_by_f32(&m, second, 0.0, 0.0, -1.0);
    m = rotate_by_f32(&m, angle, 0.0, -1.0, 0.0);
    let e = -m[9];
    let q = crate::scalar::clamp(1.0 - e * e, 0.0, 1.0).sqrt();
    let pitch = e.atan2(q);
    let (yaw, roll) = if q == 0.0 {
        (m[2].atan2(m[0]), 0.0)
    } else {
        let roll = ((-m[1]) / q).atan2(m[5] / q) * DEGREES;
        ((m[8] / -q).atan2(m[10] / q), roll)
    };
    let (r1, r2, r3) = (
        yaw * DEGREES * RADIANS,
        pitch * DEGREES * RADIANS,
        roll * RADIANS,
    );
    let (c3, s3) = (r3.cos(), r3.sin());
    let (c2, s2) = (r2.cos(), r2.sin());
    let (c1, s1) = (r1.cos(), r1.sin());
    let a = c3 * u - s3 * p;
    let b = s3 * u + c3 * p;
    let c = s2 * a + c2 * w;
    let z = s1 * b + c1 * c;
    let y = c2 * a - s2 * w;
    let x = c1 * b - s1 * c;
    let f = crate::tire::aircraft_object(vm, aircraft);
    let frame = Frame {
        origin: [vm.f64(f + 0x378), vm.f64(f + 0x380), vm.f64(f + 0x388)],
        rotation: [
            [vm.f32(f + 0x430), vm.f32(f + 0x434)],
            [vm.f32(f + 0x440), vm.f32(f + 0x444)],
            [vm.f32(f + 0x450), vm.f32(f + 0x454)],
        ],
    };
    to_aircraft_frame(&frame, [x, y, z], false, false)
}

/// `0x1411c8690(wheel, aircraft, gear)`: one frame of a gear wheel's contact with the ground.
///
/// Locals of the original keep their frame names in the comments (`rbp-0x7c` and so on): pointers to some of
/// them go to callees, the others are plain variables here. Not ported: the debug dumps (`F+0xbcd0`/`F+0xbcc8`).
/// Returns whether the compression `+0x2c` of the record is positive.
pub fn wheel_model(
    vm: &mut Vm,
    env: &mut dyn Callees,
    w: u64,
    aircraft: i32,
    gear: i32,
) -> Result<bool, String> {
    wheel_model_in(vm, env, w, aircraft, gear, SCRATCH)
}

/// [`wheel_model`] with the address of the frame (`rbp` of the original) given: the vectors of the function are
/// made with the frame of the emulator, whose addresses the replayed callees see.
pub fn wheel_model_in(
    vm: &mut Vm,
    env: &mut dyn Callees,
    w: u64,
    aircraft: i32,
    gear: i32,
    frame_base: u64,
) -> Result<bool, String> {
    let rbp = frame_base;
    let f = aircraft_object(vm, aircraft);
    if vm.i32(f + 0xbcd0) != 0 || vm.i32(f + 0xbcc8) != 0 {
        return Err("debug dump not ported".into());
    }
    let b = bobj(vm, aircraft);
    let g = vm
        .u64(b + 0x6080)
        .wrapping_add((i64::from(gear) * 0x88) as u64);
    // the wheel size (`G+0x58`; slot 0x1e0, xmm15), reshaped while the input is held
    let mut size = vm.f32(g + 0x58);
    let (mut m7c, mut m80) = (0.0f32, 0.0f32);
    if input_active(vm, env, f, 1, 0x88, gear, 1.0)? {
        let kind = vm.u32(g);
        let k = match kind {
            3 => 0.75f32,
            5..=7 => 0.5,
            _ => 1.0,
        };
        let r = vm.f32(w + 0x48) * RAD;
        let sa = ((f64::from(r) * 1.8) as f32).sin();
        let sb = r.sin();
        let v = (f64::from(sa) * 0.25 + f64::from(sb) * 0.75) as f32;
        let wv = f64::from((f64::from(v) * 0.5 + 0.5) as f32);
        let x2 = (1.0 - wv) as f32;
        let kd = f64::from(k);
        m7c = (kd * wv + 0.0) as f32;
        m80 = (wv * 0.5 * kd + 0.0) as f32;
        size = (((f64::from(x2) * 0.3 + 0.6) * kd) * f64::from(size)) as f32;
    }
    // the direction of the strut in the world (rbp-0x... 0x28/0x30/0x38) and the lowest point of the wheel
    let pitch = vm.f32(b + 0x65a8) * RAD;
    let (neg_sin, cos) = (-pitch.sin(), pitch.cos());
    let at = |vm: &Vm, o: u64| f64::from(vm.f32(f + o));
    let pairs = [
        at(vm, 0x440),
        at(vm, 0x444),
        at(vm, 0x430),
        at(vm, 0x434),
        at(vm, 0x450),
        at(vm, 0x454),
    ];
    let dir = rotate_pairs_f64(0.0, f64::from(cos), f64::from(neg_sin), pairs);
    let ra = vm.f32(w + 0x14) * RAD;
    let q = vm.u64(w);
    let l = f64::from(vm.f32(q + 0x18))
        - f64::from(vm.f32(q + 0x20)) * (1.0 - f64::from(vm.f32(w + 0x10)));
    let rb = vm.f32(w + 0x18) * RAD;
    let cos_rb = f64::from(rb.cos());
    let h = f64::from(size);
    let px = f64::from(ra.sin()) * l * cos_rb + f64::from(vm.f32(q + 0x64)) - dir[0] * h;
    let py = f64::from(vm.f32(q + 0x70)) - f64::from(ra.cos()) * l * cos_rb - dir[1] * h;
    let pz = f64::from(vm.f32(q + 0x7c)) - f64::from(rb.sin()) * l - dir[2] * h;
    let world = [rbp + 0x28, rbp + 0x30, rbp + 0x38];
    world_point_f64(vm, env, f, [px, py, pz], world);
    let pw = [vm.f64(world[0]), vm.f64(world[1]), vm.f64(world[2])];
    vm.set_i32(w + 0x78, 0);
    let reach = f64::from(vm.f32(f + 0x42f50)) + h * 5.0;
    // a 3 x 3 grid of vertical probes around the point; the centre one is kept (its normal and the velocity of
    // the surface), the highest hit is the ground height
    let (y_top, y_bot) = ((pw[1] + reach) as f32, (pw[1] - reach) as f32);
    let mut best = -99999.0f64;
    let (mut ground_y, mut hit_x, mut hit_z) = (-99999.0f64, 0.0f64, 0.0f64);
    let mut normal = [0.0f64, 1.0, 0.0]; // rbp+0x68 .. 0x78
    let mut moved = [0.0f64; 3]; // rbp+0x60, 0x58, 0x50
    let (p0, p1, out_point, out_normal, out_moved, out_id) = (
        rbp - 0x40,
        rbp - 0x18,
        rbp - 0x8,
        rbp + 0x10,
        rbp - 0x30,
        rbp + 0x1e8,
    );
    let mut j = -0.25f64;
    loop {
        let mut i = -0.25f64;
        loop {
            let sx = h * j + pw[0];
            let sz = h * i + pw[2];
            for (base, y) in [(p0, y_top), (p1, y_bot)] {
                vm.set_f32(base, sx as f32);
                vm.set_f32(base + 4, y);
                vm.set_f32(base + 8, sz as f32);
            }
            vm.set_f32(out_point, 0.0);
            vm.set_u32(out_point + 4, 0xc7c3_4f80);
            vm.set_f32(out_point + 8, 0.0);
            vm.set_f32(out_normal, 0.0);
            vm.set_f32(out_normal + 4, 1.0);
            vm.set_f32(out_normal + 8, 0.0);
            for k in 0..3 {
                vm.set_f32(out_moved + 4 * k, 0.0);
            }
            vm.set_u32(out_id, 0);
            segment_probe(
                vm,
                env,
                f + 0x42e40,
                p0,
                p1,
                SegmentOut {
                    point: out_point,
                    normal: out_normal,
                    moved: out_moved,
                    id: out_id,
                },
            );
            let id = vm.i32(out_id);
            if id != 0 {
                let hit_y = vm.f32(out_point + 4);
                best = f64::from(maxss(best as f32, hit_y));
                if 0.01 > f64::from((j as f32).abs()) && 0.01 > f64::from((i as f32).abs()) {
                    vm.set_i32(w + 0x78, id);
                    ground_y = f64::from(hit_y);
                    for k in 0..3 {
                        normal[k] = f64::from(vm.f32(out_normal + 4 * k as u64));
                        moved[k] = f64::from(vm.f32(out_moved + 4 * k as u64));
                    }
                    hit_x = sx;
                    hit_z = sz;
                }
            }
            i += 0.25;
            if 0.26 <= i {
                break;
            }
        }
        j += 0.25;
        if 0.26 <= j {
            break;
        }
    }
    if vm.i32(w + 0x78) == 0 {
        vm.set_i32(w + 0x4c, 0);
    } else {
        ground_y = best;
    }
    // the height of the ground point above the geoid
    let planet = env.call(vm, 0x14193ae40, CallArgs::default()).rax;
    vm.set_f64(rbp + 0x90, 0.0);
    vm.set_f64(rbp + 0x88, 0.0);
    local_to_geodetic(
        vm,
        env,
        planet,
        [rbp + 0x90, rbp + 0x88, w + 0x70],
        [hit_x, ground_y, hit_z],
    );
    // the ground point in the aircraft frame (double precision), and its velocity
    let x1 = (f64::from(hit_x as f32) - position_component(vm, env, f, 0x378)) as f32;
    let y1 = (f64::from(ground_y as f32) - position_component(vm, env, f, 0x380)) as f32;
    let z1 = (f64::from(hit_z as f32) - position_component(vm, env, f, 0x388)) as f32;
    let (s430, c434, s440, c444, s450, c454) = (
        at(vm, 0x430),
        at(vm, 0x434),
        at(vm, 0x440),
        at(vm, 0x444),
        at(vm, 0x450),
        at(vm, 0x454),
    );
    let (xd, yd, zd) = (f64::from(x1), f64::from(y1), f64::from(z1));
    let qa = xd * c444 + zd * s440;
    let qb = zd * c444 - xd * s440;
    let qc = qb * s430 + yd * c434;
    let o1 = qa * c454 - qc * s450;
    let o2 = qc * c454 + qa * s450;
    let o3 = qb * c434 - yd * s430;
    let (w3cc, w3d0, w3d4) = (at(vm, 0x3cc), at(vm, 0x3d0), at(vm, 0x3d4));
    let m1 = w3d4 * o1 + w3d0 * o2;
    let m2 = w3cc * (-o1) - w3d0 * o3;
    let m3 = w3cc * o2 - w3d4 * o3;
    let rot = rotate_pairs_f64(m3, m2, m1, [s440, c444, s430, c434, s450, c454]);
    let v1 = rot[0] + at(vm, 0x368);
    let v2 = rot[1] + at(vm, 0x36c);
    let v3 = rot[2] + at(vm, 0x370);
    let rel = [
        (v1 - moved[0]) as f32,
        (v2 - moved[1]) as f32,
        (v3 - moved[2]) as f32,
    ];
    let frame = Frame {
        origin: [vm.f64(f + 0x378), vm.f64(f + 0x380), vm.f64(f + 0x388)],
        rotation: [
            [vm.f32(f + 0x430), vm.f32(f + 0x434)],
            [vm.f32(f + 0x440), vm.f32(f + 0x444)],
            [vm.f32(f + 0x450), vm.f32(f + 0x454)],
        ],
    };
    // rbp-0x70 / rbp-0x78: the relative velocity along two aircraft axes, ey (xmm10) the third
    let [ex, ey, ez] = to_aircraft_frame(&frame, rel, false, false);
    let (mut m70, mut m78) = (ex, ez);
    let nl = to_aircraft_frame(
        &frame,
        [normal[0] as f32, normal[1] as f32, normal[2] as f32],
        false,
        false,
    );
    let m68 = -ex;
    let vn = (nl[0] * m68 - nl[1] * ey) - nl[2] * ez;
    let mut s1f0 = vn;
    vm.set_f32(w + 0x30, vn);
    let depth_in = (ground_y - pw[1]) as f32;
    let mut dep = depth_in;
    let mut s1e8 = 0.0f32;
    // the limits of the wheel travel by the kind of gear
    let size_eff = size;
    let kind = vm.u32(g);
    let kb = vm.f32(b + 0x27cc);
    let t7 = match kind {
        4 | 5 | 8 => (f64::from(size_eff * kb) * 0.5) as f32,
        6 => size_eff * kb,
        _ => 0.0,
    };
    if t7 > 0.0 {
        let a24 = vm.f32(g + 0x24) * RAD;
        let lim_a = t7 * a24.sin();
        let lim_b = (-t7) * a24.sin();
        let f430 = vm.f32(f + 0x430);
        let d_lo = depth_in - t7 * f430;
        let d_hi = t7 * f430 + depth_in;
        let mut x9 = maxss(d_lo - lim_a, 0.0);
        let mut x8 = maxss(d_hi - lim_b, 0.0);
        if x8 > 0.0 || x9 > 0.0 {
            let kk = vm.f32(b + 0x283c);
            let w30 = vm.f32(w + 0x30);
            if x8 > x9 {
                let top = x8 + lim_b;
                s1f0 = kk * w30;
                dep = x8 * kk + 0.0;
                let t = f64::from(x8 * kk);
                let u = f64::from(lim_a - x8);
                x8 = (u + (t + t)) as f32;
                if d_lo > x8 {
                    s1f0 = w30;
                    let d = d_lo - x8;
                    dep = (f64::from(d) * 0.5 + f64::from(dep)) as f32;
                    x8 += d;
                }
                x9 = top;
            } else {
                let top = x9 + lim_a;
                s1f0 = kk * w30;
                dep = x9 * kk + 0.0;
                let t = f64::from(x9 * kk);
                let u = f64::from(lim_b - x9);
                x9 = (u + (t + t)) as f32;
                if d_hi > x9 {
                    s1f0 = w30;
                    let d = d_hi - x9;
                    dep = (f64::from(d) * 0.5 + f64::from(dep)) as f32;
                    x9 += d;
                }
                x8 = top;
            }
            let ratio = f64::from(x8 - x9) / (f64::from(t7) + f64::from(t7));
            vm.set_f32(w + 0x20, (ratio.asin() * 57.295_776_367_187_5) as f32);
        }
    }
    // the compression and the spring force
    if dep > 0.0 {
        vm.set_f32(w + 0x2c, maxss(dep, 0.0));
        let cap = (f64::from(vm.f32(b + 0x2898) * KG) * 2.0) as f32;
        let lo = vm.f32(g + 0x28);
        let hi = vm.f32(g + 0x2c);
        let mut acc = interpolate_clamped(0.0, 0.0, hi, lo, dep) + 0.0;
        if dep > hi {
            let mut t = vm.f32(g + 0x30) - lo;
            let g34 = vm.f32(g + 0x34);
            if hi == g34 {
                t = (t + 0.0) * 0.5;
            } else {
                t -= 0.0;
                t /= g34 - hi;
                t *= dep - hi;
                t += 0.0;
            }
            acc += t;
        }
        let v = maxss(s1f0 * vm.f32(g + 0x38) + acc, 0.0);
        if v >= 0.0 || v.is_nan() {
            s1e8 = minss(cap, v);
        }
    }
    // the surface under the wheel: its friction factors and the patches of wetness
    let id = vm.i32(w + 0x78);
    let row = vm
        .u64(SURFACES + 8)
        .wrapping_add((i64::from(id) * 36) as u64);
    let surf40 = vm.f32(b + 0x27c0) * vm.f32(row + 0x18);
    let surf44 = vm.f32(b + 0x27c4) * vm.f32(row + 0x1c);
    let mut s1e0 = surf40 + m7c;
    let x9m = surf44 + m80;
    let mut m6c = x9m;
    let x8m = (f64::from(surf44) * 0.75 + f64::from(m80)) as f32;
    m80 = x8m;
    let zs = (hit_z * 0.05) as f32;
    let xs = (hit_x * 0.05) as f32;
    let noise = noise2(vm, xs, zs, 0);
    let v5 = f64::from(noise) * 0.5 + 0.5;
    let gl = [
        vm.f32(GLOBALS[0]),
        vm.f32(GLOBALS[1]),
        vm.f32(GLOBALS[2]),
        vm.f32(GLOBALS[3]),
    ];
    let mut r8 = f64::from(gl[0]) > v5;
    let r9 = f64::from(gl[1]) > v5;
    // the second lookup of the original is the same noise value
    if f64::from(gl[2]) > v5 {
        r8 = true;
    }
    if f64::from(gl[3]) > v5 || r8 {
        let cl = |value: f32, scale: f32, floor: f32| {
            let a = 1.0 - (value - 0.0) * scale;
            if floor > a { floor } else { minss(1.0, a) }
        };
        let c4 = cl(
            gl[3],
            f32::from_bits(0x3f4c_cccd),
            f32::from_bits(0x3e4c_cccd),
        );
        let c1 = cl(
            gl[2],
            f32::from_bits(0x3f19_999a),
            f32::from_bits(0x3ecc_cccd),
        );
        let c2 = cl(
            gl[1],
            f32::from_bits(0x3ecc_cccc),
            f32::from_bits(0x3f19_999a),
        );
        let c0 = cl(
            gl[0],
            f32::from_bits(0x3e4c_cccc),
            f32::from_bits(0x3f4c_cccd),
        );
        let m = minss(minss(c0, c2), minss(c1, c4));
        m6c = x9m * m;
        m80 = x8m * m;
    }
    if r9 {
        let r = f64::from(sqrt_f(vm.f32(g + 0x3c))) * 9.0 * KNOT_D;
        let thr = r as f32;
        let (a, c, d) = (vm.f32(f + 0x368), vm.f32(f + 0x36c), vm.f32(f + 0x370));
        let speed = sqrt_f((a * a + c * c) + d * d);
        if speed > thr {
            let v = f32::from_bits(0x3d4c_cccd);
            m6c = v;
            m80 = v;
        }
    }
    // the slip angle of the relative velocity
    let mut s1f8 = sqrt_f(m70 * m70 + m78 * m78);
    let (neg_lim, pos_lim) = (f32::from_bits(0xba83_126f), f32::from_bits(0x3a83_126f));
    let z6 = m78;
    let kept = if neg_lim > z6 || z6 > pos_lim {
        z6
    } else if 0.0 > z6 {
        neg_lim
    } else {
        pos_lim
    };
    let mut m74 = m68.atan2(-kept) * DEG;
    // whether the wheel can steer, and the steering angle (the record's `+0x28`)
    let gk = vm.u32(g);
    let mut steer = false;
    if gk > 1
        && (f64::from(vm.f32(g + 0x50)) > 0.01
            || f64::from(vm.f32(g + 0x54)) > 0.01
            || vm.i32(g + 0xc) != 0)
    {
        steer = !caster_locked(vm, f, gear);
    }
    let locked = caster_locked(vm, f, gear);
    let mut w28 = 0.0f32;
    let mut skip_limits = false;
    if vm.f32(f + 0x218) == 0.0 {
        if steer {
            let g50 = vm.f32(g + 0x50);
            if vm.f32(b + 0x27bc) > g50 {
                let d = (vm.f32(f + 0x240) - vm.f32(f + 0x244)).abs();
                let e = (1.0 - f64::from(d)) as f32;
                w28 = e * vm.f32(w + 0x24) - d * m74;
            } else {
                w28 = vm.f32(w + 0x24);
            }
        }
    } else if steer || locked {
        w28 = vm.f32(w + 0x24);
        skip_limits = true;
    }
    if !skip_limits {
        let k27bc = vm.f32(b + 0x27bc);
        if k27bc > 0.0 {
            let lo = -k27bc;
            if lo > w28 {
                w28 = lo;
            } else if w28 > k27bc {
                w28 = k27bc;
            }
        }
        if vm.f32(b + 0x1e90) > 0.0 {
            let rate = vm.f32(g + 0x50) / vm.f32(b + 0x1e90);
            let dt = time(vm, env);
            let hi = (f64::from(vm.f32(w + 0x28)) + dt * f64::from(rate)) as f32;
            let rate2 = vm.f32(g + 0x50) / vm.f32(b + 0x1e90);
            let dt2 = time(vm, env);
            let lo = (f64::from(vm.f32(w + 0x28)) - dt2 * f64::from(rate2)) as f32;
            if lo > w28 {
                w28 = lo;
            } else if w28 > hi {
                w28 = hi;
            }
        }
    }
    if locked {
        if s1e8 > 0.0 {
            let r = vm.f32(w + 0x28) * RAD;
            let xs8 = r.sin() * size_eff;
            let xs7 = (-size_eff) * r.cos();
            let dt = time(vm, env);
            let a = (dt * f64::from(m70)) as f32;
            let dt2 = time(vm, env);
            let c = (dt2 * f64::from(m78)) as f32;
            let y = a + xs8;
            let x = -xs7 - c;
            w28 = y.atan2(x) * DEG;
        } else {
            w28 = -vm.f32(f + 0x408);
        }
    }
    vm.set_f32(w + 0x28, w28);
    // the direction of the contact force in the wheel plane
    let r7 = (m74 + w28) * RAD;
    let v48 = r7.cos() * s1f8;
    let size_c = maxss(size_eff, f32::from_bits(0x3c23_d70a));
    let mut x4 = 0.0f64;
    if s1e8 > 0.0 {
        x4 = f64::from(v48 / size_c);
    }
    vm.set_f32(w + 0x40, x4 as f32);
    m78 = r7.sin() * s1f8;
    let m50 = v48 - size_eff * vm.f32(w + 0x44);
    m70 = sqrt_f(m50 * m50 + m78 * m78);
    let mut x7 = 0.0f32;
    if vm.i32(g + 4) != 0 {
        let w10 = vm.f32(w + 0x10);
        if 0.1 > f64::from(w10) {
            let t = (w10 - f32::from_bits(0x3dcc_cccd)) * 10.0;
            let u = 0.0 - t;
            if u >= 0.0 || u.is_nan() {
                x7 = minss(1.0, u);
            }
        }
    }
    if vm.i32(w + 0x4c) != 0 {
        m7c = 1.0;
    } else {
        let t = vm.f32(w + 0x50) * vm.f32(w + 0x54) + x7;
        m7c = if 0.0 > t { 0.0 } else { minss(1.0, t) };
    }
    let x8 = m6c;
    let mut x7b = vm.f32(b + 0x2898) * KG;
    x7b *= x8;
    x7b *= size_eff;
    x7b /= vm.f32(b + 0x282c);
    let inv = (1.028_888_940_811_157_2f64 / f64::from(size_c)) as f32;
    let r12 = inv > vm.f32(w + 0x40)
        && inv > vm.f32(w + 0x44)
        && (f64::from(vm.f32(w + 0x50)) > 0.1 || vm.i32(w + 0x4c) != 0);
    let sl = slip_limit(vm, env, aircraft, x8 * s1e8);
    let s_k = f32::from_bits(0x3e06_0a93).sin();
    let m74n = maxss(s_k * s1f8, sl);
    m74 = m74n;
    let ax = m70.abs();
    let twice = (f64::from(m74n) + f64::from(m74n)) as f32;
    m6c = if m74n == twice {
        0.5
    } else {
        let t = 1.0 / (twice - m74n);
        let t = t * (ax - m74n) + 0.0;
        if 0.0 > t { 0.0 } else { minss(1.0, t) }
    };
    let s3 = s1e8;
    s1e0 *= s3;
    let m6c_abs = m6c.abs();
    let up = maxss(x8, m80);
    let low = minss(x8, m80);
    let lerp_v = (m80 - x8) * (m6c_abs - 0.0) + x8;
    let clamped = if low > lerp_v { low } else { minss(up, lerp_v) };
    let lat = s3 * clamped;
    // the aerodynamic-looking correction from the speed of the wheel record's body
    let bw = vm.u64(w + 8);
    let t0 = vm.f32(bw + 0x2838) * KNOT;
    let prod = f64::from(t0 * t0) * (f64::from(vm.f32(bw + 0x2898)) * 0.5);
    let q = (prod / f64::from(vm.f32(bw + 0x282c))) as f32;
    let t3 = (vm.f32(w + 0x68) / q - 1.0) * f32::from_bits(0x3ffa_e148);
    let x0 = 1.0 - t3;
    let floor = f32::from_bits(0x3ca3_d70a);
    let x6c = if floor > x0 { floor } else { minss(1.0, x0) };
    let sign = if 0.0 > vm.f32(w + 0x44) {
        -1.0f32
    } else {
        1.0f32
    };
    let x4c = maxss(size_eff, f32::from_bits(0x3c23_d70a));
    let g60 = f64::from(vm.f32(g + 0x60));
    let mut m64 = (-x7b) * m7c;
    m64 *= x6c;
    m64 *= sign;
    m64 = (f64::from(m64) - (g60 + g60) / f64::from(x4c) * f64::from(sign)) as f32;
    // the stick-slip of the grip (`+0x54`)
    let w54_clear =
        0.1 > f64::from(vm.f32(w + 0x50)) || inv > vm.f32(w + 0x40) || vm.i32(b + 0xe98) == 0;
    let x0 = if w54_clear {
        1.0f32
    } else {
        let t = f64::from(-((lat * size_eff) / m64));
        let dt = time(vm, env);
        let w54 = f64::from(vm.f32(w + 0x54));
        let moved_w54 = if t > 1.2 {
            w54 + dt * 3.0
        } else {
            w54 - dt * 15.0
        };
        let w54n = moved_w54 as f32;
        vm.set_f32(w + 0x54, w54n);
        let tt = (f64::from_bits(vm.u64(0x1_42f0_1910)) * 2.0) as f32;
        let ta = tt.abs();
        let low32 = (ta as i64) as u32;
        let frac = ta - (low32 as f32);
        let base = (low32 as i32).wrapping_add(gear.wrapping_mul(100)) as u32;
        let n0 = vm.f32(NOISE + 4 * u64::from(base & 0x3ffff));
        let n1 = vm.f32(NOISE + 4 * u64::from(base.wrapping_add(1) & 0x3ffff));
        let v = (1.0 - frac) * n0 + n1 * frac;
        let vd = f64::from(v);
        let vm1 = ((vd + vd) - 1.0) as f32;
        let sc = f64::from(vm1) * 0.05 + 1.0;
        let w54m = (f64::from(w54n) * sc) as f32;
        vm.set_f32(w + 0x54, w54m);
        let low = f32::from_bits(0x3dcc_cccd);
        if low > w54m { low } else { minss(1.0, w54m) }
    };
    vm.set_f32(w + 0x54, x0);
    if !r12 && vm.f32(w + 0x50) > 0.0 {
        let a = (m64 * vm.f32(w + 0x40)).abs();
        let dt = time(vm, env);
        let n = (f64::from(vm.f32(w + 0x68)) + dt * f64::from(a)) as f32;
        vm.set_f32(w + 0x68, n);
    }
    let dt = time(vm, env);
    let fac = 1.0 - dt * 0.001;
    vm.set_f32(w + 0x68, (f64::from(vm.f32(w + 0x68)) * fac) as f32);
    if vm.i32(g + 4) != 0 {
        let w10 = vm.f32(w + 0x10);
        let c = f64::from(vm.f32(f + 0x41c));
        let dt = time(vm, env);
        let v = ((dt * c) * f64::from(w10)) / 10000.0;
        vm.set_f32(w + 0x68, (f64::from(vm.f32(w + 0x68)) * (1.0 - v)) as f32);
    }
    if vm.i32(f + 0x200) != 0 {
        let dt = time(vm, env);
        let fac = 1.0 - dt * 0.01;
        vm.set_f32(w + 0x68, (f64::from(vm.f32(w + 0x68)) * fac) as f32);
    }
    if s1e8 > 0.0 {
        let sx = {
            let x1 = m64 / maxss(size_eff, f32::from_bits(0x3c23_d70a));
            let n = s1e0 - x1;
            s1e0 = n;
            n
        };
        let m = m74;
        let neg_m = -m;
        s1f8 = if neg_m == m {
            0.0
        } else {
            let t = 2.0f32 / (m - neg_m);
            let t = t * (m78 - neg_m) - 1.0;
            if -1.0 > t { -1.0 } else { minss(1.0, t) }
        };
        let sl_a = slip_limit(vm, env, aircraft, sx);
        let sl_b = slip_limit(vm, env, aircraft, sx);
        let neg = -sl_b;
        let x9b = if neg == sl_a {
            0.0
        } else {
            let t = 2.0f32 / (sl_a - neg);
            let t = t * (v48 - neg) - 1.0;
            if -1.0 > t { -1.0 } else { minss(1.0, t) }
        };
        m7c = s1f8 * lat;
        let n0 = -lat;
        let x1 = x9b * s1e0;
        m80 = if n0 > x1 { n0 } else { minss(lat, x1) };
        if r12 || m6c > 0.0 {
            let sl = slip_limit(vm, env, aircraft, lat);
            let ratio = m70 / sl;
            let y7 = if 0.0 > ratio { 0.0 } else { minss(1.0, ratio) };
            let ang = m78.atan2(m50) * DEG * RAD;
            let y7 = y7 * lat;
            m7c = ang.sin() * y7;
            m80 = ang.cos() * y7;
        }
        let sq = sqrt_f(m7c * m7c + m80 * m80);
        let denom = maxss(lat, f32::from_bits(0x3c23_d70a));
        vm.set_f32(w + 0x58, sq / denom);
        // the contact point moves with the ground; the forces are collected in rbp-0x7c / rbp-0x80
        vm.set_f32(rbp - 0x7c, m7c);
        vm.set_f32(rbp - 0x80, m80);
        let tire = Tire {
            record: w,
            aircraft,
            heading_offset: f64::from(vm.f32(w + 0x28)),
            scale: f64::from(size_eff),
            reference_x: hit_x,
            reference_vx: v1,
            vx: moved[0],
            force_a: rbp - 0x7c,
            mu: s1e0,
            limit: lat,
            reference_z: hit_z,
            reference_vz: v3,
            vz: moved[2],
            force_b: rbp - 0x80,
        };
        tire_motion(vm, env, &tire)?;
        let p = if !locked { vm.f32(rbp - 0x7c) } else { 0.0 };
        let angle = vm.f32(f + 0x358) + vm.f32(w + 0x28);
        let out = wheel_axis_point(vm, aircraft, p, s1e8, vm.f32(rbp - 0x80), angle, normal);
        let (out1, out2, out3) = (out[0], out[1], out[2]);
        let (a, c, d) = (o1 as f32, o2 as f32, o3 as f32);
        add_plugin_force(vm, env, f, a, out1, c, out2, d, out3)?;
        vm.set_f32(w + 0x38, out1);
        vm.set_f32(w + 0x34, out2);
        vm.set_f32(w + 0x3c, out3);
        if aircraft == vm.i32(RECORDED_AIRCRAFT) {
            record_contact(
                vm,
                env,
                rbp + 0xa0,
                [
                    (0x14, a.to_bits()),
                    (0x18, c.to_bits()),
                    (0x1c, d.to_bits()),
                    (0x2c, out1.to_bits()),
                    (0x30, out2.to_bits()),
                    (0x34, out3.to_bits()),
                ],
            );
        }
    }
    if r12 {
        vm.set_i32(w + 0x44, 0);
    } else {
        let w40 = vm.f32(w + 0x40);
        let k = vm.f32(g + 0x60);
        servo(
            vm,
            env,
            m64,
            k,
            w + 0x44,
            w40,
            (-lat) * size_eff,
            lat * size_eff,
            0,
        );
    }
    if s1e8 > 0.0 {
        if aircraft == 0 && f64::from_bits(vm.u64(0x1_42f0_1920)) > 1.0 {
            let x6 = s1f0;
            let code = if x6 > 4.0 && vm.f32(f + 0x3d8).abs() > 90.0 {
                Some(aircraft + 0x3d)
            } else if x6 > 3.0 {
                Some(0x3c)
            } else if x6 > 2.0 {
                Some(0x3b)
            } else if x6 > 1.0 {
                Some(0x3a)
            } else {
                None
            };
            if let Some(code) = code {
                let mut args = CallArgs::ints(&[0x1_4612_0f08, code as u64]);
                args.xmm = [None, None, Some(1.0f32.to_bits()), Some(1.0f32.to_bits())];
                env.call(vm, 0x141a21200, args);
            }
        }
        vm.set_f32(w + 0x5c, maxss(m70 - m74, 0.0));
        if GLOBALS.iter().all(|a| vm.f32(*a) == 0.0) {
            let x7 = vm.f32(b + 0x2898) * KG / vm.f32(b + 0x282c);
            let mut x6 = vm.f32(w + 0x58);
            x6 *= x6;
            x6 *= vm.f32(w + 0x5c);
            x6 *= s1e8;
            x6 /= x7;
            let dt = time(vm, env);
            let mut v = (f64::from(vm.f32(w + 0x60)) + dt * f64::from(x6)) as f32;
            vm.set_f32(w + 0x60, v);
            if v > 0.0 {
                let dt = time(vm, env);
                v = (f64::from(vm.f32(w + 0x60)) - dt / 60.0) as f32;
                vm.set_f32(w + 0x60, v);
            }
            if f64::from(v) > f64::from(sqrt_f(size_eff)) * 300.0 {
                vm.set_i32(f + 0xfe0 + 4 * gear as u64, 6);
            }
        }
        let t = vm.f32(w + 0x58) - 0.5;
        let t = (t + t) + 0.0;
        let x14 = if 0.0 > t { 0.0 } else { minss(1.0, t) };
        let x6 = if x14 > 0.0 {
            x14.powf(4.0)
        } else if 0.0 > x14 {
            -((-x14).powf(4.0))
        } else {
            0.0
        };
        let sq = f64::from(sqrt_f(vm.f32(w + 0x5c)));
        vm.set_f32(w + 0x64, (sq * 0.2 * f64::from(x6)) as f32);
    }
    if vm.i32(f + 0xbcd0) != 0 || vm.i32(f + 0xbcc8) != 0 {
        return Err("debug dump not ported".into());
    }
    Ok(vm.f32(w + 0x2c) > 0.0)
}
