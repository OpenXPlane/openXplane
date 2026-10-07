//! The attitude of the flight object as a quaternion: Euler angles (degrees) to a quaternion (`0x1408816e0`), the
//! integration of body rates into the quaternion (`0x140f5fb10`) and the conversion back to Euler angles
//! (`0x140889cb0`). float32 throughout, with the platform's libm for the trigonometry.

use crate::vm::Vm;

const RAD: f32 = f32::from_bits(0x3c8e_fa36);
/// Degrees per radian as the original holds it (`0x14250e640`).
const DEG: f32 = f32::from_bits(0x4265_2ee0);

/// `0x1408816e0(a, b, c, out)`: the quaternion `[w, x, y, z]`-style four floats of three half angles.
pub fn euler_to_quaternion(a: f32, b: f32, c: f32) -> [f32; 4] {
    let half = |v: f32| v * RAD * 0.5;
    let (hb, ha, hc) = (half(b), half(a), half(c));
    let (sb, sa, sc) = (hb.sin(), ha.sin(), hc.sin());
    let (cb, ca, cc) = (hb.cos(), ha.cos(), hc.cos());
    let x5 = cc * cb;
    let x7 = cb * sc;
    let x3 = cc * sb;
    let x4 = sc * sb;
    let q0 = x5 * ca + x4 * sa;
    let x0 = x3 * sa;
    let x3 = x3 * ca;
    let q3 = x5 * sa - x4 * ca;
    let q2 = x3 + x7 * sa;
    let q1 = x7 * ca - x0;
    [q0, q1, q2, q3]
}

/// `0x140889cb0(q, &a, &b, &c)`: normalises the quaternion in place (`1 / sqrt(max(|q|^2, 0.1))`) and returns the
/// three Euler angles in degrees: `a` in `0..=360` and `b`, `c` in `-180..=180` (`b` from an arcsine of the
/// clamped middle term, the others from arctangents).
pub fn quaternion_to_euler(q: &mut [f32; 4]) -> [f32; 3] {
    let sum = q[1] * q[1] + q[0] * q[0] + q[2] * q[2] + q[3] * q[3];
    let held = if sum > 0.1 { sum } else { 0.1 };
    let inverse = 1.0 / held;
    let scale = if 0.0 > inverse {
        f32::NAN
    } else {
        inverse.sqrt()
    };
    for v in q.iter_mut() {
        *v *= scale;
    }
    let (q0, q1, q2, q3) = (q[0], q[1], q[2], q[3]);
    let (x4, x2, x1, x3) = (q1 * q1, q2 * q2, q3 * q3, q0 * q0);
    let x0 = q2 * q0;
    let x12 = q3 * q2;
    let x8 = q2 * q1;
    let x9 = q3 * q1;
    let x6 = q3 * q0;
    let x13 = x3 - x4;
    let x9 = x9 - x0;
    let x0 = q1 * q0;
    let x4 = x4 + x3;
    let x8 = x8 + x6;
    let x13 = x13 - x2;
    let x12 = x12 + x0;
    let x4 = x4 - x2;
    let x9 = x9 + x9;
    let x8 = x8 + x8;
    let x13 = x13 + x1;
    let x12 = x12 + x12;
    let x4 = x4 - x1;
    let mut a = x8.atan2(x4) * DEG;
    while 0.0 > a {
        a += 360.0;
    }
    while a > 360.0 {
        a += -360.0;
    }
    let arg = crate::scalar::clamp(x9, -1.0, 1.0);
    let mut b = -arg.asin() * DEG;
    while -180.0 > b {
        b += 360.0;
    }
    while b > 180.0 {
        b += -360.0;
    }
    let mut c = x12.atan2(x13) * DEG;
    while -180.0 > c {
        c += 360.0;
    }
    while c > 180.0 {
        c += -360.0;
    }
    [a, b, c]
}

/// `0x140f5fb10(w1, w2, w3, &a, &b, &c, q)`: multiplies the quaternion at `q` by the small rotation of the
/// three angles (radians, scaled to degrees) and stores the Euler angles of the result: `a` at `out_a`, `b` at
/// `out_b`, `c` at `out_c`.
pub fn integrate_attitude(vm: &mut Vm, w: [f32; 3], q: u64, out_a: u64, out_b: u64, out_c: u64) {
    let inc = euler_to_quaternion(w[2] * DEG, w[1] * DEG, w[0] * DEG);
    let cur = [vm.f32(q), vm.f32(q + 4), vm.f32(q + 8), vm.f32(q + 12)];
    let (q0, q1, q2, q3) = (cur[0], cur[1], cur[2], cur[3]);
    let (i0, i1, i2, i3) = (inc[0], inc[1], inc[2], inc[3]);
    let n0 = q0 * i0 - q1 * i1 - q2 * i2 - q3 * i3;
    let n1 = q1 * i0 + q0 * i1 + q2 * i3 - q3 * i2;
    let n2 = q0 * i2 - q1 * i3 + q2 * i0 + q3 * i1;
    let n3 = q1 * i2 + q0 * i3 - q2 * i1 + q3 * i0;
    let mut next = [n0, n1, n2, n3];
    for (k, v) in next.iter().enumerate() {
        vm.set_f32(q + 4 * k as u64, *v);
    }
    let [a, b, c] = quaternion_to_euler(&mut next);
    for (k, v) in next.iter().enumerate() {
        vm.set_f32(q + 4 * k as u64, *v);
    }
    vm.set_f32(out_a, a);
    vm.set_f32(out_b, b);
    vm.set_f32(out_c, c);
}
