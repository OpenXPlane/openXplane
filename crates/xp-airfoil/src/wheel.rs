//! Helpers of the wheel contact model `0x1411c8690`, compared with the machine code in an emulator
//! (`tools/gen_small_vectors.py`, kinds `caster` and `servo`).

use crate::tire::time;
use crate::vm::{Callees, Vm};

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
