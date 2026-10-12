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

use crate::vm::{CallArgs, Callees};

/// The accumulators and weights of `0x1411ddfd0`.
#[derive(Clone, Copy, Debug)]
pub struct Balance {
    pub sums: Sums,
    /// The original's byte argument: when set, only the aircraft's own mass and the payload stations are summed.
    pub stop: bool,
}

const REGISTRY: u64 = 0x1_4578_b780;
const NULL_KEY: u64 = 0x1_42f0_3778;

fn sqrt_or_nan(v: f32) -> f32 {
    if 0.0 > v { f32::NAN } else { v.sqrt() }
}

/// `0x1411ddfd0(F, ...)`: sums the mass, the inertia and the moments of the weight. The total mass and the base
/// inertia come from the dynamic-pressure part (`F+0xb588` velocity, the four sums at `F+0xb798..0xb7a4`,
/// `F+0xb838`) and the aircraft's `B+0x288c`; then the point masses are added with [`mass_point`]: the nine
/// payload stations (`B+0x412c`), the jet fuel (`F+0x28c`), JATO (`F+0x6534`), the mass `F+0x284`, the nine fuel
/// tanks (`F+0xbbb4`, at [`tank_position`]), the removed items (`B+0x91c` of them, bound by `0x1407ace10`) and
/// the attached objects of the registry at `F+0x69b8` (lookups `0x140f5c540`/`0x140f42620` replayed, the
/// reference counting and the label strings skipped).
pub fn weight_and_balance(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    wb: &Balance,
) -> Result<(), String> {
    let s = &wb.sums;
    let b = vm.u64(f + 0x20);
    let log = vm.i32(f + 0xbcd0) != 0 || vm.i32(f + 0xbcc8) != 0;
    // the aircraft's own mass
    let flow = vm.u64(f + 0xb588);
    let (vx, vy, vz) = (
        vm.f32(flow + 0x64f4),
        vm.f32(flow + 0x64f8),
        vm.f32(flow + 0x64fc),
    );
    let speed = sqrt_or_nan((vx * vx + vy * vy) + vz * vz);
    let sum4 =
        ((vm.f32(f + 0xb79c) + vm.f32(f + 0xb798)) + vm.f32(f + 0xb7a0)) + vm.f32(f + 0xb7a4);
    let pressure = f64::from(speed * speed) * 1000.0 * 0.025 * 0.25;
    let part = ((f64::from(sum4) * pressure) * 0.25) as f32 + vm.f32(b + 0x288c);
    let total = (f64::from(part) + f64::from(vm.f32(f + 0xb838)) * 1.1) as f32;
    vm.set_f32(s.total, total);
    vm.set_f32(s.inertia[0], total * vm.f32(b + 0x28ac));
    vm.set_f32(s.inertia[1], vm.f32(b + 0x28b0) * total);
    vm.set_f32(s.inertia[2], vm.f32(b + 0x28b4) * total);
    let add =
        |vm: &mut Vm, v: f32, p: (f32, f32, f32), mode: i32| mass_point(vm, s, v, p, mode, log);
    for k in 0..9u64 {
        if vm.f32(b + 0x412c + 4 * k) > 0.0 {
            let mode = vm.i32(b + 0x4108 + 4 * k);
            let p = (
                vm.f32(b + 0x4150 + 12 * k),
                vm.f32(b + 0x4154 + 12 * k),
                vm.f32(b + 0x4158 + 12 * k),
            );
            add(vm, vm.f32(f + 0x260 + 4 * k), p, mode)?;
        }
    }
    if wb.stop {
        return Ok(());
    }
    if vm.i32(b + 0x28d4) == 0 && vm.i32(b + 0x28d8) == 0 && vm.f32(f + 0x28c) > 0.0 {
        let p = (vm.f32(b + 0x28c8), vm.f32(b + 0x28cc), vm.f32(b + 0x28d0));
        add(vm, vm.f32(f + 0x28c), p, 1)?;
    }
    if vm.f32(f + 0x6534) > 0.0 {
        let flow = vm.f32(f + 0x6534) * vm.f32(b + 0x2920) * vm.f32(b + 0x2928);
        let v = (f64::from(flow) / 3600.0 / f64::from(f32::from_bits(0x411c_c5c1))) as f32;
        add(vm, v, (0.0, vm.f32(b + 0x2914), vm.f32(b + 0x2918)), 1)?;
    }
    if vm.f32(f + 0x284) > 0.0 {
        let p = (
            vm.f32(b + 0x2268) * vm.f32(f + 0x11c),
            0.0,
            vm.f32(b + 0x226c) * vm.f32(f + 0x118),
        );
        add(vm, vm.f32(f + 0x284), p, 1)?;
    }
    for k in 0..9u64 {
        let level = vm.f32(f + 0xbbb4 + 4 * k);
        if level > 0.0 {
            let [x, y, z] = tank_position(vm, b, k as i32);
            add(vm, level, (x, y, z), 1)?;
        }
    }
    let mut i = 0u64;
    while (i as i64) < i64::from(vm.i32(b + 0x91c)) {
        // `0x1407ace10(F, 0, 0x179, i, 1.0)`: mode 0 only answers yes for a slot held at 6
        if crate::binding::bound(vm, env, f, 0, 0x179, i as i32) {
            let item = vm.u64(b + 0x5ff8) + 0x68 * i;
            let at = vm.u64(b + 0x6010) + 0x3770 * i;
            let v = -vm.f32(item + 0x38);
            let p = (vm.f32(at + 0x790), vm.f32(at + 0x794), vm.f32(at + 0x798));
            add(vm, v, p, 1)?;
        }
        i += 1;
    }
    // the attached objects
    let mut n = 0u64;
    loop {
        let begin = vm.u64(f + 0x69b8);
        let count = (vm.u64(f + 0x69c0).wrapping_sub(begin) as i64) >> 4;
        if n as i64 >= count {
            break;
        }
        let key = begin + 16 * n;
        let present = if vm.u32(key) == vm.u32(NULL_KEY) && vm.u64(key + 8) == vm.u64(NULL_KEY + 8)
        {
            false
        } else {
            let args = CallArgs::ints(&[REGISTRY, key]);
            env.call(vm, 0x140f5c540, args).rax as u8 != 0
        };
        if present {
            let lookup = |vm: &mut Vm, env: &mut dyn Callees| -> u64 {
                env.call(vm, 0x140f42620, CallArgs::ints(&[REGISTRY, key]))
                    .rax
            };
            let rec = vm.u64(b + 0x60c8) + 0x90 * n;
            // the first lookup only supplies the label string
            lookup(vm, env);
            let e2 = lookup(vm, env);
            let z = vm.f32(rec + 0x7c) - vm.f32(vm.u64(e2 + 0x178) + 0x790);
            let e3 = lookup(vm, env);
            let y = vm.f32(rec + 0x74) - vm.f32(vm.u64(e3 + 0x178) + 0x78c);
            let e4 = lookup(vm, env);
            let x = vm.f32(rec + 0x6c) - vm.f32(vm.u64(e4 + 0x178) + 0x788);
            let e5 = lookup(vm, env);
            add(vm, vm.f32(e5 + 0xbc), (x, y, z), 1)?;
        }
        let rec = vm.u64(b + 0x60c8) + 0x90 * n;
        if vm.u64(rec + 0x30) != 0 && vm.u64(rec + 0x88) != 0 {
            let obj = vm.u64(rec + 0x88);
            let mut v = vm.f32(obj + 0x37d8);
            let kind = vm.i32(obj + 0x36e8);
            if (kind.wrapping_sub(5) as u32) > 1 && matches!(kind, 1 | 4 | 7 | 8 | 9) {
                let count = vm.i32(vm.u64(f + 0x69a0) + 16 * n) as f32;
                v += count * vm.f32(obj + 0xa8);
            }
            let p = (
                vm.f32(rec + 0x6c) - vm.f32(obj + 0x3e08),
                vm.f32(rec + 0x74) - vm.f32(obj + 0x3e0c),
                vm.f32(rec + 0x7c) - vm.f32(obj + 0x3e10),
            );
            add(vm, v, p, 1)?;
        }
        n += 1;
    }
    Ok(())
}
