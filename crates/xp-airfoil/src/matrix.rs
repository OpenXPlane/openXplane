//! The 4 x 4 matrices of doubles of the reference build (row-vector convention: the translation is in the last
//! row, `m[12..15]`). Each function is compared with the original machine code in an emulator
//! (`tools/gen_matrix_vectors.py`).

use crate::vm::Vm;

pub type Mat4 = [f64; 16];

pub fn identity() -> Mat4 {
    let mut m = [0.0; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    m
}

pub fn read(vm: &Vm, address: u64) -> Mat4 {
    std::array::from_fn(|k| vm.f64(address + 8 * k as u64))
}

pub fn write(vm: &mut Vm, address: u64, m: &Mat4) {
    for (k, v) in m.iter().enumerate() {
        vm.set_f64(address + 8 * k as u64, *v);
    }
}

/// `0x1407b33a0(out, a, b)`: `b * a` (row `r` of `b` against the columns of `a`, the four products added in order).
pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    std::array::from_fn(|k| {
        let (r, c) = (k / 4, k % 4);
        (((b[4 * r] * a[c]) + b[4 * r + 1] * a[4 + c]) + b[4 * r + 2] * a[8 + c])
            + b[4 * r + 3] * a[12 + c]
    })
}

/// `0x14093c4f0(out, m)`: the inverse of a rotation with a translation: the transposed rotation and the translation
/// `-t R^T`.
pub fn rigid_inverse(m: &Mat4) -> Mat4 {
    let mut o = [0.0; 16];
    for (to, from) in [
        (0, 0),
        (1, 4),
        (2, 8),
        (3, 3),
        (4, 1),
        (5, 5),
        (6, 9),
        (7, 7),
        (8, 2),
        (9, 6),
        (10, 10),
        (11, 11),
        (15, 15),
    ] {
        o[to] = m[from];
    }
    o[12] = -((m[13] * m[1] + m[12] * m[0]) + m[14] * m[2]);
    o[13] = -((m[5] * m[13] + m[4] * m[12]) + m[6] * m[14]);
    o[14] = -((m[9] * m[13] + m[8] * m[12]) + m[10] * m[14]);
    o
}

/// `0x140cbabc0(out, angle, x, y, z)`: the rotation by `angle` degrees about the axis `(x, y, z)`. An axis along one
/// coordinate (the other two exactly zero) is written directly, any other axis is normalised first; an axis shorter
/// than 0.0001 leaves the identity.
pub fn axis_rotation(angle: f64, x: f64, y: f64, z: f64) -> Mat4 {
    const RAD: f64 = f64::from_bits(0x3f91_df46_a252_9d39);
    let r = angle * RAD;
    let (s, c) = (r.sin(), r.cos());
    let mut m = identity();
    // `ucomisd` with a zero: equal only when ordered and zero
    let zero = |v: f64| v == 0.0;
    if zero(x) {
        if zero(y) {
            if !zero(z) {
                m[0] = c;
                m[5] = c;
                if 0.0 <= z || z.is_nan() {
                    m[1] = s;
                    m[4] = -s;
                } else {
                    m[4] = s;
                    m[1] = -s;
                }
                return m;
            }
        } else if zero(z) {
            m[0] = c;
            m[10] = c;
            if 0.0 <= y || y.is_nan() {
                m[8] = s;
                m[2] = -s;
            } else {
                m[2] = s;
                m[8] = -s;
            }
            return m;
        }
    } else if zero(y) && zero(z) {
        m[5] = c;
        m[10] = c;
        if 0.0 <= x || x.is_nan() {
            m[6] = s;
            m[9] = -s;
        } else {
            m[9] = s;
            m[6] = -s;
        }
        return m;
    }
    let len = ((x * x + y * y) + z * z).sqrt();
    if 0.0001 >= len {
        return m;
    }
    let (z, x, y) = (z / len, x / len, y / len);
    let t = 1.0 - c;
    let (xs, sz, sy) = (s * x, s * z, s * y);
    let xyt = (x * y) * t;
    let xzt = (x * z) * t;
    let yzt = (y * z) * t;
    m[0] = (x * x) * t + c;
    m[1] = xyt + sz;
    m[4] = xyt - sz;
    m[8] = xzt + sy;
    m[2] = xzt - sy;
    m[5] = (y * y) * t + c;
    m[10] = (z * z) * t + c;
    m[6] = yzt + xs;
    m[9] = yzt - xs;
    m
}

/// `0x140cb9860(m, angle, x, y, z)`: `m` becomes the rotation of [`axis_rotation`] times `m`.
pub fn rotate_by(m: &Mat4, angle: f64, x: f64, y: f64, z: f64) -> Mat4 {
    mul(m, &axis_rotation(angle, x, y, z))
}

/// `0x14089c7a0(out, a, b)`: [`mul`] for 4 x 4 matrices of floats.
pub fn mul_f32(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    std::array::from_fn(|k| {
        let (r, c) = (k / 4, k % 4);
        (((b[4 * r] * a[c]) + b[4 * r + 1] * a[4 + c]) + b[4 * r + 2] * a[8 + c])
            + b[4 * r + 3] * a[12 + c]
    })
}

/// `0x1419f6fd0(m, a1, a2, a3)`: the three angles (degrees) the rotation `T * m` has, where `T` is the rotation built
/// from the seed angles `a1`, `a2`, `a3` (their sines and cosines `s1 c1 s2 c2 s3 c3`: the rows of `T` are
/// `(s3 s2 s1 + c3 c1, -s3 c2, c3 s1 - s3 s2 c1)`, `(s3 c1 - c3 s2 s1, c3 c2, c3 s2 c1 + s3 s1)`, `(-c2 s1, -s2,
/// c2 c1)`). With `R = T * m` and `e = -R[2][1]` the second angle is `atan2(e, sqrt(1 - e^2))`; away from the pole the
/// first is `atan2(R[2][0] / -q, R[2][2] / q)` and the third `atan2(-R[0][1] / q, R[1][1] / q)` with `q` that
/// square root; at the pole (`q == 0`) the first is `atan2(R[0][2], R[0][0])` and the third zero.
pub fn euler_from_matrix(m: &[f32; 16], seeds: [f32; 3]) -> [f32; 3] {
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    const DEG: f32 = f32::from_bits(0x4265_2ee0);
    let (r1, r2, r3) = (seeds[0] * RAD, seeds[1] * RAD, seeds[2] * RAD);
    let (s1, c1) = (r1.sin(), r1.cos());
    let (s2, c2) = (r2.sin(), r2.cos());
    let (s3, c3) = (r3.sin(), r3.cos());
    let s3s2 = s3 * s2;
    let c3s2 = c3 * s2;
    let t = [
        (s3s2 * s1) + (c3 * c1),
        (-s3) * c2,
        (c3 * s1) - (s3s2 * c1),
        0.0,
        (s3 * c1) - (c3s2 * s1),
        c3 * c2,
        (c3s2 * c1) + (s3 * s1),
        0.0,
        (-c2) * s1,
        -s2,
        c2 * c1,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ];
    let r = mul_f32(m, &t);
    let e = -r[9];
    let v = 1.0 - e * e;
    let clamped = crate::scalar::clamp(v, 0.0, 1.0);
    let q = clamped.sqrt();
    if q == 0.0 {
        [r[2].atan2(r[0]) * DEG, e.atan2(q) * DEG, 0.0]
    } else {
        [
            (r[8] / -q).atan2(r[10] / q) * DEG,
            e.atan2(q) * DEG,
            ((-r[1]) / q).atan2(r[5] / q) * DEG,
        ]
    }
}
