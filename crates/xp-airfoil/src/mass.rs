//! The weight and balance of the aircraft: point masses added to the total mass, the inertia sums and the moments
//! of the weight (`0x1411dd820`; the sources are summed by `0x1411ddfd0`).

use crate::vm::Vm;

/// The arguments of `0x1411dd820` that point into the flight object (the accumulators), with the three floats that
/// scale the moments.
#[derive(Clone, Copy, Debug)]
pub struct Sums {
    /// `arg5`: the total mass.
    pub total: u64,
    /// `arg7`, `arg10`, `arg13`: the inertia sums `sum m (c^2 + b^2)`, `sum m (a^2 + c^2)`, `sum m (a^2 + b^2)`.
    pub inertia: [u64; 3],
    /// `arg8`, `arg11`, `arg14`: the moments of the weight.
    pub moments: [u64; 3],
    /// `arg6`, `arg12`, `arg9`: the floats multiplied by the gravity constant into the moments.
    pub weights: [f32; 3],
}

const GRAVITY: f32 = f32::from_bits(0x411c_c5c1);

/// `0x1411dd820(v, a, b, c, ...)`: adds the point mass `v` at `(a, b, c)`. With `mode == 2` the inertia sums are
/// left alone. The moments change by `-w6 g b v`, `-w6 g a v` (the second and third accumulator), `-w9 g b v`,
/// `+w9 g c v`, `+w12 g a v` and `+w12 g c v`. The log branch (the flag argument) is not ported.
pub fn mass_point(
    vm: &mut Vm,
    s: &Sums,
    v: f32,
    (a, b, c): (f32, f32, f32),
    mode: i32,
    log: bool,
) -> Result<(), String> {
    if log {
        return Err("log line not ported".into());
    }
    let add = |vm: &mut Vm, address: u64, delta: f32| vm.set_f32(address, delta + vm.f32(address));
    vm.set_f32(s.total, v + vm.f32(s.total));
    if mode != 2 {
        let (a2, b2, c2) = (a * a, b * b, c * c);
        add(vm, s.inertia[0], (c2 + b2) * v);
        add(vm, s.inertia[1], (a2 + c2) * v);
        add(vm, s.inertia[2], (a2 + b2) * v);
    }
    let [w6, w12, w9] = s.weights.map(|w| w * GRAVITY);
    let [m8, m11, m14] = s.moments;
    // `*rdi -= w6 b v; *rsi -= w6 a v`
    vm.set_f32(m11, vm.f32(m11) - (w6 * b) * v);
    vm.set_f32(m14, vm.f32(m14) - (w6 * a) * v);
    vm.set_f32(m8, vm.f32(m8) - (w9 * b) * v);
    let c9 = (w9 * c) * v;
    let a12 = (w12 * a) * v;
    let c12 = (w12 * c) * v;
    vm.set_f32(m14, c9 + vm.f32(m14));
    vm.set_f32(m8, a12 + vm.f32(m8));
    vm.set_f32(m11, c12 + vm.f32(m11));
    Ok(())
}

const RADIANS: f32 = f32::from_bits(0x3c8e_fa36);

/// `0x1412941d0(spring, x)`: the lag term of a damped part: zero when `|spring+0| < 0.001`, else
/// `(1 - cos(spring+0 * x / spring+8)) * spring+4` (the last product in double precision).
pub fn spring_lag(vm: &Vm, spring: u64, x: f32) -> f32 {
    let s = vm.f32(spring);
    if f64::from(s.abs()) < 0.001 {
        return 0.0;
    }
    let c = (s * x / vm.f32(spring + 8)).cos();
    ((1.0 - f64::from(c)) * f64::from(vm.f32(spring + 4))) as f32
}

/// `0x141294240(B, part, x, y, z) -> (x', y', z')`: the point `(x, y, z)` of the part `part` (records of 0x36c8
/// bytes at `B+0x6028`) carried through the part's four angles (`+0x30`, `+0x20`, `+0x38`, `+0x28` in degrees; the
/// first two scale the arm `+0x3c * +0x40` and `+0x54 * +0x10`), its scale `+0` and offset `+0x6f0..`, with the
/// lag of the spring at `B+0x6070` added to the second component and the spring's swing to the first.
pub fn part_point(vm: &mut Vm, b: u64, part: i32, (x, y, z): (f32, f32, f32)) -> [f32; 3] {
    let w = vm
        .u64(b + 0x6028)
        .wrapping_add((i64::from(part) * 0x36c8) as u64);
    let f = |o: u64| vm.f32(w + o);
    let a1 = f(0x30) * RADIANS;
    let arm = (vm.i32(w + 0x3c) as f32) * f(0x40);
    let c1a = a1.cos() * arm;
    let a2 = f(0x20) * RADIANS;
    let c2c1 = a2.cos() * c1a;
    let sa = a2.sin() * c1a;
    let arm2 = f(0x54) * f(0x10);
    let sb = a1.sin() * arm;
    let c1b = a1.cos() * arm2;
    let t1 = (x - a2.cos() * c1b) + c2c1;
    let t3 = (z - a1.sin() * arm2) + sb;
    let a3 = f(0x38) * RADIANS;
    let (s3, c3) = (a3.sin(), a3.cos());
    let a4 = f(0x28) * RADIANS;
    let (s4, c4) = (a4.sin(), a4.cos());
    let u1 = c3 * t1 - s3 * t3;
    let s2c1b = a2.sin() * c1b;
    let u3 = (s3 * t1 + c3 * t3) - sb;
    let u2 = (y - s2c1b) + sa;
    let out0 = (u1 * c4 - u2 * s4 - c2c1) * f(0) + f(0x6f0);
    let mut out1 = (u2 * c4 + u1 * s4 - sa) + f(0x6f4);
    let out2 = u3 + f(0x6f8);
    out1 += spring_lag(vm, b + 0x6070, out0);
    let spring = vm.f32(b + 0x6070);
    let swing = if f64::from(spring.abs()) < 0.001 {
        0.0
    } else {
        (spring * out0 / vm.f32(b + 0x6078)).sin() * vm.f32(b + 0x6074) - out0
    };
    [out0 + swing, out1, out2]
}

/// `0x141a6ba30(B, tank) -> (x, y, z)`: the centre of a fuel tank for its fill fraction (`F`-side fill at
/// `[B+0x61f8] + 0xbbb4` over the capacity `B+0x3f7c * B+0x2888`, at least 0.01), interpolated between the empty
/// position (`B+0x400c + 12 tank`) and the full one (`B+0x4078 + 12 tank`) and, when the tank sits on a part
/// (`B+0x40e4 + 4 tank >= 0`), carried by [`part_point`].
pub fn tank_position(vm: &mut Vm, b: u64, tank: i32) -> [f32; 3] {
    let i = tank as i64 as u64;
    let level = vm.u64(b + 0x61f8);
    let capacity = vm.f32(b + 0x3f7c + 4 * i) * vm.f32(b + 0x2888);
    let floor = f32::from_bits(0x3c23_d70a);
    let capacity = if capacity > floor { capacity } else { floor };
    let t = vm.f32(level + 0xbbb4 + 4 * i) / capacity;
    let lerp = |empty: f32, full: f32| {
        if t == 0.0 {
            empty
        } else if t == 1.0 {
            full
        } else {
            (1.0 - t) * empty + full * t
        }
    };
    let row = 12 * i;
    let x = lerp(vm.f32(b + 0x400c + row), vm.f32(b + 0x4078 + row));
    let y = lerp(vm.f32(b + 0x4010 + row), vm.f32(b + 0x407c + row));
    let z = lerp(vm.f32(b + 0x4014 + row), vm.f32(b + 0x4080 + row));
    let part = vm.i32(b + 0x40e4 + 4 * i);
    if part >= 0 {
        return part_point(vm, b, part, (x, y, z));
    }
    [x, y, z]
}
