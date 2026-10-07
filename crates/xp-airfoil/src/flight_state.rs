//! The later blocks of the flight step (`0x1412656b0`): bookkeeping and derived quantities computed after the
//! state advance. Each function is a block of the original, compared with the machine code in an emulator
//! (`tools/gen_flight_block_vectors.py`), with the callees outside this port replayed.

use crate::flight_step::position_component;
use crate::scalar::{clamp, lerp};
use crate::vm::{CallArgs, Callees, Vm};
use crate::wing_element::{interpolate_clamped, signed_sqrt};

fn frame_time(vm: &mut Vm, env: &mut dyn Callees) -> f64 {
    f64::from_bits(env.call(vm, 0x140c448c0, CallArgs::default()).xmm0)
}

fn planet(vm: &mut Vm, env: &mut dyn Callees) -> u64 {
    env.call(vm, 0x14193ae40, CallArgs::default()).rax
}

/// `0x1407d76f0(F, offset)`: the height of the aircraft (plus `offset` times the local vertical velocity
/// `F+0x3f8`) above the ground cell, replayed.
fn height_above_ground(vm: &mut Vm, env: &mut dyn Callees, f: u64) -> f32 {
    let mut args = CallArgs::ints(&[f]);
    args.xmm[1] = Some(0);
    f32::from_bits(env.call(vm, 0x1407d76f0, args).xmm0 as u32)
}

/// `0x141272a96..0x14127307c`: the path samples of the takeoff and landing record, kept while the simulation
/// speed `0x142f01920` exceeds 1. While parked (speed below one knot) the three start points `F+0x65b8/0x65d8/0x65f8`
/// (longitudes) and `F+0x65c8/0x65e8/0x6608` (latitudes) follow the position; when the aircraft is below 50 feet
/// they update, and once all six are set `F+0x65b0` is raised; the same for the second set
/// (`F+0x65c0/0x65e0/0x6600`, `F+0x65d0/0x65f0/0x6610`) with the flag `F+0x65b4`. The four distances between pairs
/// of the points (`0x1406e2be0`, replayed) in feet go to `F+0x6618/0x661c/0x6620/0x6624`. The registers at the
/// start of the block are `xmm7 = F+0x368`, `xmm9 = F+0x370` (floats) and `xmm11 = 0.0`.
pub fn path_samples(vm: &mut Vm, env: &mut dyn Callees, f: u64) {
    const KNOTS: f32 = f32::from_bits(0x3ff8_cfe5);
    const FEET: f32 = f32::from_bits(0x4051_f948);
    let g = vm.f64(0x1_42f0_1920);
    let speed_knots = |vm: &Vm| {
        let (a, b, c) = (vm.f32(f + 0x368), vm.f32(f + 0x36c), vm.f32(f + 0x370));
        let sum = a * a + b * b + c * c;
        (if 0.0 > sum { f32::NAN } else { sum.sqrt() }) * KNOTS
    };
    let zero = |vm: &Vm, offset: u64| vm.f64(f + offset) == 0.0;
    if g > 1.0 {
        let lon = |vm: &mut Vm, env: &mut dyn Callees| position_component(vm, env, f, 0x390);
        let lat = |vm: &mut Vm, env: &mut dyn Callees| position_component(vm, env, f, 0x398);
        if vm.i32(f + 0x65b0) == 0 {
            if 1.0 > speed_knots(vm) {
                let v = lon(vm, env);
                for offset in [0x65f8, 0x65d8, 0x65b8] {
                    vm.set_f64(f + offset, v);
                }
                let v = lat(vm, env);
                for offset in [0x6608, 0x65e8, 0x65c8] {
                    vm.set_f64(f + offset, v);
                }
            }
            if vm.i32(f + 0x24c) != 0 {
                let v = lon(vm, env);
                vm.set_f64(f + 0x65d8, v);
                let v = lat(vm, env);
                vm.set_f64(f + 0x65e8, v);
            }
            let height = height_above_ground(vm, env, f) * FEET;
            if 50.0 > height {
                let v = lon(vm, env);
                vm.set_f64(f + 0x65f8, v);
                let v = lat(vm, env);
                vm.set_f64(f + 0x6608, v);
            } else if ![0x65b8, 0x65d8, 0x65f8, 0x65c8, 0x65e8, 0x6608]
                .iter()
                .any(|o| zero(vm, *o))
            {
                vm.set_i32(f + 0x65b0, 1);
            }
        }
        if g > 1.0 && vm.i32(f + 0x65b4) == 0 {
            let height = height_above_ground(vm, env, f) * FEET;
            let (mut a, mut b);
            if height > 50.0 {
                let v = lon(vm, env);
                for offset in [0x6600, 0x65e0, 0x65c0] {
                    vm.set_f64(f + offset, v);
                }
                let v = lat(vm, env);
                for offset in [0x6610, 0x65f0, 0x65d0] {
                    vm.set_f64(f + offset, v);
                }
                a = v;
                b = v;
            } else {
                a = vm.f64(f + 0x65d0);
                b = vm.f64(f + 0x6610);
            }
            let c;
            if vm.i32(f + 0x24c) == 0 {
                let v = lon(vm, env);
                vm.set_f64(f + 0x65e0, v);
                c = lat(vm, env);
                vm.set_f64(f + 0x65f0, c);
                a = vm.f64(f + 0x65d0);
                b = vm.f64(f + 0x6610);
            } else {
                c = vm.f64(f + 0x65f0);
            }
            if speed_knots(vm) > 1.0 {
                let v = lon(vm, env);
                vm.set_f64(f + 0x65c0, v);
                let v = lat(vm, env);
                vm.set_f64(f + 0x65d0, v);
            } else {
                let stored = [0x65c0, 0x65e0, 0x6600].iter().any(|o| zero(vm, *o));
                if !stored && a != 0.0 && c != 0.0 && b != 0.0 {
                    vm.set_i32(f + 0x65b4, 1);
                }
            }
        }
    }
    for (target, points) in [
        (0x6618, [0x65b8, 0x65c8, 0x65d8, 0x65e8]),
        (0x661c, [0x65c0, 0x65d0, 0x65e0, 0x65f0]),
        (0x6620, [0x65b8, 0x65c8, 0x65f8, 0x6608]),
        (0x6624, [0x65c0, 0x65d0, 0x6600, 0x6610]),
    ] {
        let ctx = planet(vm, env);
        let bits = |vm: &Vm, k: usize| vm.f64(f + points[k]).to_bits();
        let mut args = CallArgs::ints(&[ctx]);
        args.xmm = [
            None,
            Some(bits(vm, 0) as u32),
            Some(bits(vm, 1) as u32),
            Some(bits(vm, 2) as u32),
        ];
        args.stack = [Some(bits(vm, 3)), Some(0), Some(0), None];
        let distance = f64::from_bits(env.call(vm, 0x1406e2be0, args).xmm0);
        vm.set_f32(f + target, (distance * 3.2808399200439453) as f32);
    }
}

/// A float function of the flight object that is replayed: `0x14076b5d0(0x145890720, lon, lat)`,
/// `0x141244b60(F, angle)`, `0x1407d7bc0(F, 2.0)` and `0x1407cc570(F, speed, 1)`.
fn replayed_float(vm: &mut Vm, env: &mut dyn Callees, address: u64, args: CallArgs) -> f32 {
    f32::from_bits(env.call(vm, address, args).xmm0 as u32)
}

/// `0x14127307c..0x141273779`: the cockpit quantities. Distances travelled (`F+0x6638` horizontal, `F+0x663c`
/// total, doubles added to floats), the equivalent airspeed in knots `F+0x41c`, the Mach number `F+0x420`, the
/// dynamic pressure `F+0x424`, the magnetic variation `F+0x428` and `F+0x42c` (replayed), and smoothed
/// instruments `F+0x500..0x514` (angles and loads blended with `lerp` at rates from the frame time and the aircraft's
/// time constants `B+0x24a4`, `B+0x2498`), then the rates of change of the pitch, roll, load and speed terms
/// `F+0x518..0x53c`. The register state at the block start is `xmm8 = 1.0` (double), `xmm11 = 0.0`.
pub fn instruments(vm: &mut Vm, env: &mut dyn Callees, f: u64) {
    const KNOTS: f32 = f32::from_bits(0x3ff8_cfe5);
    const DEG: f32 = f32::from_bits(0x4265_2ee0);
    let sqrt = |sum: f32| if 0.0 > sum { f32::NAN } else { sum.sqrt() };
    let b = vm.u64(f + 0x20);
    let (v0, v1, v2) = (vm.f32(f + 0x368), vm.f32(f + 0x36c), vm.f32(f + 0x370));
    let horizontal = sqrt(v0 * v0 + v2 * v2);
    let dt = frame_time(vm, env);
    let value = f64::from(vm.f32(f + 0x6638)) + dt * f64::from(horizontal);
    vm.set_f32(f + 0x6638, value as f32);
    let speed = vm.f32(f + 0x400);
    let dt = frame_time(vm, env);
    let value = f64::from(vm.f32(f + 0x663c)) + dt * f64::from(speed);
    vm.set_f32(f + 0x663c, value as f32);
    vm.set_f32(f + 0x41c, sqrt(vm.f32(f + 0x70)) * speed * KNOTS);
    vm.set_f32(f + 0x420, speed / vm.f32(f + 0x74));
    vm.set_f32(
        f + 0x424,
        (f64::from(speed * speed * vm.f32(f + 0x6c)) * 0.5) as f32,
    );
    let lat = position_component(vm, env, f, 0x398) as f32;
    let lon = position_component(vm, env, f, 0x390) as f32;
    let mut args = CallArgs::ints(&[0x1_4589_0720]);
    args.xmm[1] = Some(lon.to_bits());
    args.xmm[2] = Some(lat.to_bits());
    let variation = replayed_float(vm, env, 0x14076b5d0, args);
    vm.set_f32(f + 0x428, variation);
    let mut args = CallArgs::ints(&[f]);
    args.xmm[1] = Some(vm.f32(f + 0x358).to_bits());
    let value = replayed_float(vm, env, 0x141244b60, args);
    vm.set_f32(f + 0x42c, value);
    let dt = frame_time(vm, env);
    let factor = (dt / f64::from(vm.f32(b + 0x24a4))) as f32;
    let speed3 = sqrt(v0 * v0 + v1 * v1 + v2 * v2);
    let ramp = interpolate_clamped(5.0, 0.0, 10.0, 1.0, speed3);
    let target = ramp * vm.f32(f + 0x404);
    vm.set_f32(f + 0x500, lerp(vm.f32(f + 0x500), target, factor));
    let dt = frame_time(vm, env);
    let factor = dt as f32;
    let ramp = interpolate_clamped(5.0, 0.0, 10.0, 1.0, speed3);
    let target = ramp * vm.f32(f + 0x408);
    vm.set_f32(f + 0x504, lerp(vm.f32(f + 0x504), target, factor));
    let dt = frame_time(vm, env);
    let factor = (dt / f64::from(vm.f32(b + 0x2498))) as f32;
    let mut args = CallArgs::ints(&[f]);
    args.xmm[1] = Some(2.0f32.to_bits());
    let reference = replayed_float(vm, env, 0x1407d7bc0, args);
    let mut x = vm.f32(f + 0x358) - reference;
    while -180.0 > x {
        x += 360.0;
    }
    while x > 180.0 {
        x += -360.0;
    }
    let factor = clamp(factor, 0.0, 1.0);
    vm.set_f32(f + 0x508, (1.0 - factor) * vm.f32(f + 0x508) + factor * x);
    let dt = frame_time(vm, env);
    let factor = clamp((dt / f64::from(vm.f32(b + 0x24a4))) as f32, 0.0, 1.0);
    vm.set_f32(
        f + 0x50c,
        (1.0 - factor) * vm.f32(f + 0x50c) + factor * vm.f32(f + 0x350),
    );
    let dt = frame_time(vm, env);
    let factor = (dt * 0.2) as f32;
    let value = clamp(vm.f32(f + 0x344), -0.1, 1.0);
    vm.set_f32(f + 0x510, lerp(vm.f32(f + 0x510), value, factor));
    let dt = frame_time(vm, env);
    let factor = (dt + dt) as f32;
    vm.set_f32(
        f + 0x514,
        lerp(vm.f32(f + 0x514), vm.f32(f + 0x34c), factor),
    );
    let s = vm.f32(f + 0x400);
    let ramp = interpolate_clamped(1.0, 0.0, 5.0, 1.0, s * KNOTS);
    let a414 = ramp * vm.f32(f + 0x414);
    let a404 = ramp * vm.f32(f + 0x404);
    let a408 = ramp * vm.f32(f + 0x408);
    let mut args = CallArgs::ints(&[f]);
    args.xmm[1] = Some(s.to_bits());
    args.int[2] = Some(1);
    let load = replayed_float(vm, env, 0x1407cc570, args) * KNOTS;
    let ground = {
        let (g0, g2) = (vm.f32(f + 0x368), vm.f32(f + 0x370));
        sqrt(g0 * g0 + g2 * g2) * KNOTS
    };
    for (rate, previous, value) in [(0x51c, 0x518, a414), (0x52c, 0x528, a408)] {
        let dt = frame_time(vm, env);
        let factor = (dt * 10.0) as f32;
        let dt = frame_time(vm, env);
        let inverse = (1.0 / dt) as f32;
        let delta = (value - vm.f32(f + previous)) * inverse;
        vm.set_f32(f + rate, lerp(vm.f32(f + rate), delta, factor));
        vm.set_f32(f + previous, value);
        if previous == 0x518 {
            vm.set_f32(f + 0x524, vm.f32(f + 0x3d0) * DEG);
            vm.set_f32(f + 0x520, a404);
        }
    }
    for (rate, previous, value) in [(0x534, 0x530, load), (0x53c, 0x538, ground)] {
        let dt = frame_time(vm, env);
        let factor = dt as f32;
        let dt = frame_time(vm, env);
        let inverse = (1.0 / dt) as f32;
        let delta = (value - vm.f32(f + previous)) * inverse;
        vm.set_f32(f + rate, lerp(vm.f32(f + rate), delta, factor));
        vm.set_f32(f + previous, value);
    }
}

/// `0x141273779..0x141273dfb`: the force coefficients and the trim estimate. `F+0x258` is set when the aircraft
/// type word `F+0x42f84` is 2 to 6 (`0x141964140`) and `F+0x25c` when the switch `F+0x24c` is on and it is not. The
/// forces `F+0x2c0/0x2d4/0x2e8` are turned into the wind axes with the sine and cosine pairs `F+0x460/0x464` and
/// `F+0x468/0x46c`: `F+0x298`, `F+0x2b0`, `F+0x2a4`; with the dynamic pressure `F+0x424` and the reference area
/// `B+0x2880` they give the coefficients `F+0x65a4`, `F+0x65a8` and the ratio `F+0x65ac`. For an aircraft heavier
/// than 0.99 of `B+0x288c` with the positive parameters `B+0x28e8/0x28ec/0x28f0`, the estimate `B+0x28f4` (clamped to
/// -100..100) is computed from the mix of the tail-surface fractions (nine records at `F+0xbbb4`, each queried
/// through `0x141a6ba30`, replayed) and the aircraft's coefficients, then integrated twice into `B+0x28f8` and
/// `B+0x28fc` (both clamped) with the frame time. The register state at the block start is `rdi = 0`, `rsi = 1`,
/// `xmm8 = 1.0` (double) and `xmm10 = 0.5` (double).
pub fn force_coefficients(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    rbp: u64,
) -> Result<(), String> {
    const G: f32 = f32::from_bits(0x411c_c5c1);
    const DEG: f32 = f32::from_bits(0x4265_2ee0);
    let b = vm.u64(f + 0x20);
    let kind = vm.i32(f + 0x42f84).wrapping_sub(2) as u32 <= 4;
    vm.set_i32(f + 0x258, i32::from(kind));
    vm.set_i32(f + 0x25c, i32::from(vm.i32(f + 0x24c) != 0 && !kind));
    let (c460, c464, c468, c46c) = (
        vm.f32(f + 0x460),
        vm.f32(f + 0x464),
        vm.f32(f + 0x468),
        vm.f32(f + 0x46c),
    );
    let (f2c0, f2d4, f2e8) = (vm.f32(f + 0x2c0), vm.f32(f + 0x2d4), vm.f32(f + 0x2e8));
    vm.set_f32(f + 0x298, f2d4 * c464 - f2c0 * c460);
    vm.set_f32(f + 0x2b0, f2e8 * c46c - f2c0 * c468);
    vm.set_f32(f + 0x2a4, f2c0 * c464 + f2d4 * c460 + f2e8 * c468);
    if vm.i32(f + 0xbcd0) != 0 || vm.i32(f + 0xbcc8) != 0 {
        return Err("debug dump not ported".into());
    }
    let (q, area) = (vm.f32(f + 0x424), vm.f32(b + 0x2880));
    let (drag, side) = (vm.f32(f + 0x298), vm.f32(f + 0x2a4));
    vm.set_f32(f + 0x65a4, drag / (q * area));
    vm.set_f32(f + 0x65a8, side / (q * area));
    vm.set_f32(f + 0x65ac, drag / side);
    let mass = vm.f32(f + 0x288);
    let heavy = f64::from(mass) > f64::from(vm.f32(b + 0x288c)) * 0.99;
    if !heavy {
        return Ok(());
    }
    let (p1, p2, p3) = (vm.f32(b + 0x28e8), vm.f32(b + 0x28ec), vm.f32(b + 0x28f0));
    let valid = p1 > 0.0 && p2 > 0.0 && p3 > 0.0;
    if !valid {
        return Ok(());
    }
    let mut x12 = (f64::from(mass * p1) * 0.5) as f32;
    let wind = f64::from(vm.f32(b + 0x64f4));
    let (mut x7, mut x11) = (0.0f32, 0.0f32);
    let mut x9;
    if vm.i32(b) < 0x4b0 {
        x9 = (wind * 0.5) as f32;
    } else {
        x9 = (wind * 0.33) as f32;
        for i in 0..9u64 {
            let w = vm.f32(f + 0xbbb4 + 4 * i);
            if w > 0.0 {
                vm.set_u32(rbp + 0x50, 0);
                vm.set_u32(rbp + 0x54, 0);
                vm.set_u32(rbp + 0x58, 0);
                env.call(vm, 0x141a6ba30, CallArgs::ints(&[b, i, rbp + 0x50]));
                let x = vm.f32(rbp + 0x50).abs();
                if x > 0.0 {
                    x7 += w;
                    x11 += x * w;
                }
            }
        }
    }
    let held = if x7 > 0.01 { x7 } else { 0.01 };
    x11 /= held;
    let sum = x7 + x12;
    x11 *= x7;
    x9 *= x12;
    x11 += x9;
    x11 /= sum;
    let b2898 = f64::from(vm.f32(b + 0x2898));
    let x9d = f64::from(x11);
    let g = f64::from(G);
    let t = (1.0 - f64::from(p1)) * b2898 * 0.5 * g * x9d / f64::from(p2);
    let x7 = t as f32;
    let x5 = (f64::from(p3) * (f64::from(x7) * 0.025)) as f32;
    let x8 = x11 * x11 * sum;
    let mut acc = f64::from(f2d4) * 0.5 * x9d;
    let weight = sum * vm.f32(f + 0x344) * G * x11;
    acc -= f64::from(weight);
    acc -= b2898 * 0.5 * g * x9d;
    let c = vm.f32(b + 0x28e4) * vm.f32(b + 0x2888);
    acc += f64::from(c) * 0.5 * g * x9d;
    x12 = x12 * G * x11;
    acc += f64::from(x12);
    acc -= f64::from(x7 * vm.f32(b + 0x28fc));
    acc -= f64::from(x5 * vm.f32(b + 0x28f8));
    let estimate = acc as f32 / x8 * DEG;
    let limited = |v: f32| clamp(v, -100.0, 100.0);
    vm.set_f32(b + 0x28f4, estimate);
    vm.set_f32(b + 0x28f4, limited(vm.f32(b + 0x28f4)));
    let dt = frame_time(vm, env);
    let value = f64::from(vm.f32(b + 0x28f8)) + dt * f64::from(vm.f32(b + 0x28f4));
    vm.set_f32(b + 0x28f8, value as f32);
    vm.set_f32(b + 0x28f8, limited(vm.f32(b + 0x28f8)));
    let dt = frame_time(vm, env);
    let value = f64::from(vm.f32(b + 0x28fc)) + dt * f64::from(vm.f32(b + 0x28f8));
    vm.set_f32(b + 0x28fc, value as f32);
    vm.set_f32(b + 0x28fc, limited(vm.f32(b + 0x28fc)));
    Ok(())
}

/// `0x1407ace10(F, mode, 0x100, 0, 1.0)`: an input-binding query with a mode word.
fn mode_query(vm: &mut Vm, env: &mut dyn Callees, f: u64, mode: u64) -> bool {
    let args = CallArgs::ints(&[f, mode, 0x100, 0]);
    env.call(vm, 0x1407ace10, args).rax as u32 != 0
}

/// `0x141273dfb..0x14127402d`: the last state updates. With `B+0xc54` the fuel and load update `0x141245750` (replayed)
/// runs, otherwise `F+0x6490` and `F+0x64a8` are cleared and `F+0x6494` takes `F+0x5c`. When the aircraft has the
/// feature `B+0x8c4` and the global `0x142f01978` is set, the control assist `F+0x6594` is computed: with `B+0x8c8`
/// from the equivalent airspeed `signed_sqrt(F+0x41c / B+0x7b0)` and the angle `F+0x404 / B+0x8ec` (zero when the
/// binding with mode 2 is active), otherwise 1 when the binding with mode 1 is inactive and either the speed
/// `F+0x6c98` exceeds a quarter of `B+0x7b0` with the angle above `B+0x8ec` or the time `F+0x6f4c` is in the future,
/// else 0. With `F+0x28 == 0` the engine update `0x14125e4f0` (replayed) runs. With `F+0xdbc` the pitch and roll
/// `F+0x3d8` (to +-45) and `F+0x3dc` (to +-20) are limited by replacing an out-of-range value with the limit, and
/// the quaternion `F+0x3e4` is rebuilt from the heading `F+0x3e0` and the two angles. The register state at the block
/// start is `rdi = 0`, `rsi = 1`, `xmm13 = 0.0` and `xmm14 = 1.0`.
pub fn late_state(vm: &mut Vm, env: &mut dyn Callees, f: u64) {
    let b = vm.u64(f + 0x20);
    if vm.i32(b + 0xc54) != 0 {
        env.call(vm, 0x141245750, CallArgs::ints(&[f]));
    } else {
        vm.set_i32(f + 0x6490, 0);
        let word = vm.u32(f + 0x5c);
        vm.set_u32(f + 0x6494, word);
        vm.set_i32(f + 0x64a8, 0);
    }
    let b = vm.u64(f + 0x20);
    if vm.i32(b + 0x8c4) != 0 && vm.i32(0x1_42f0_1978) != 0 {
        if vm.i32(b + 0x8c8) != 0 {
            let mut assist = 0.0f32;
            if !mode_query(vm, env, f, 2) {
                let t = signed_sqrt(vm.f32(f + 0x41c) / vm.f32(b + 0x7b0));
                let scale = (t - 0.25) * f32::from_bits(0x3faa_aaab) + 0.0;
                let ramp =
                    interpolate_clamped(0.75, 0.0, 1.0, 1.0, vm.f32(f + 0x404) / vm.f32(b + 0x8ec));
                assist = ramp * scale;
            }
            vm.set_f32(f + 0x6594, assist);
        } else {
            let fast = f64::from(vm.f32(f + 0x6c98)) > f64::from(vm.f32(b + 0x7b0)) * 0.25
                && vm.f32(f + 0x404) > vm.f32(b + 0x8ec);
            let pending = f64::from(vm.f32(f + 0x6f4c)) > vm.f64(0x1_42f0_1918);
            let mut value = 0i32;
            if fast || pending {
                value = i32::from(!mode_query(vm, env, f, 1));
            }
            vm.set_f32(f + 0x6594, value as f32);
        }
    }
    if vm.i32(f + 0x28) == 0 {
        env.call(vm, 0x14125e4f0, CallArgs::ints(&[f]));
    }
    if vm.i32(f + 0xdbc) != 0 {
        let limit = |value: f32, lo: f32, hi: f32, bound: i32| -> f32 {
            if lo > value || value > hi {
                if value > hi {
                    bound as f32
                } else {
                    -bound as f32
                }
            } else {
                value
            }
        };
        let pitch = limit(vm.f32(f + 0x3d8), -45.0, 45.0, 45);
        vm.set_f32(f + 0x3d8, pitch);
        let roll = limit(vm.f32(f + 0x3dc), -20.0, 20.0, 20);
        vm.set_f32(f + 0x3dc, roll);
        let q = crate::attitude::euler_to_quaternion(vm.f32(f + 0x3e0), roll, pitch);
        for (k, v) in q.iter().enumerate() {
            vm.set_f32(f + 0x3e4 + 4 * k as u64, *v);
        }
    }
}

/// `0x14126a791..0x14126aad6`: the swing of the arm at `B+0x4440..0x4454` (a retractable or trailing member whose
/// angle `F+0x6548` follows the demand `F+0x6528` between the limits `B+0x444c` and `B+0x4450`). The angle becomes
/// the interpolation of the demand; with `F+0x28 == 0` and a positive arm length `B+0x4454`, five iterations place
/// the arm tip in the world (`rotate_pairs`, the position doubles, the probe offset `F+0x42f50` through
/// `rotate_pairs_f64`), ask the terrain probe `0x14195ffc0` (replayed) and, when it hits, lower the angle by the
/// penetration (`+0.1` of the probe height minus the tip height, over the arm length, in degrees) held to the limits.
/// Returns the original's `esi`: 1 once any iteration found the tip below the surface, else 0.
#[allow(clippy::field_reassign_with_default)]
pub fn arm_probe(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) -> u32 {
    use crate::transform::{rotate_pairs, rotate_pairs_f64};
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    const DEG: f32 = f32::from_bits(0x4265_2ee0);
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    let b = vm.u64(f + 0x20);
    let mut esi = 0u32;
    let angle = interpolate_clamped(
        0.0,
        vm.f32(b + 0x444c),
        1.0,
        vm.f32(b + 0x4450),
        vm.f32(f + 0x6528),
    );
    vm.set_f32(f + 0x6548, angle);
    let active = vm.i32(f + 0x28) == 0 && vm.f32(b + 0x4454) > 0.0;
    if !active {
        return esi;
    }
    let pairs = [
        vm.f32(f + 0x440),
        vm.f32(f + 0x444),
        vm.f32(f + 0x430),
        vm.f32(f + 0x434),
        vm.f32(f + 0x450),
        vm.f32(f + 0x454),
    ];
    for _ in 0..5 {
        vm.set_i32(slot(0x1758), 0);
        vm.set_i32(slot(-0x80), 0);
        vm.set_i32(slot(-0x7c), 0);
        let a7 = vm.f32(f + 0x6548) * RAD;
        let b = vm.u64(f + 0x20);
        let length = vm.f32(b + 0x4454);
        let x8 = length * a7.cos() + vm.f32(b + 0x4448);
        let x4 = vm.f32(b + 0x4444) - length * a7.sin();
        let [o1, o2, o3] = rotate_pairs(vm.f32(b + 0x4440), x4, x8, pairs);
        vm.set_f32(slot(0x1758), o1);
        vm.set_f32(slot(-0x80), o2);
        vm.set_f32(slot(-0x7c), o3);
        let x9 = (position_component(vm, env, f, 0x378) + f64::from(vm.f32(slot(0x1758)))) as f32;
        let y8 = (position_component(vm, env, f, 0x380) + f64::from(vm.f32(slot(-0x80)))) as f32;
        let z7 = (position_component(vm, env, f, 0x388) + f64::from(vm.f32(slot(-0x7c)))) as f32;
        let p = pairs.map(f64::from);
        let r = rotate_pairs_f64(0.0, f64::from(vm.f32(f + 0x42f50)), 0.0, p);
        vm.set_f64(slot(-0x68), r[0]);
        vm.set_f64(slot(-0x38), r[1]);
        vm.set_f64(slot(-0x50), r[2]);
        vm.set_f32(slot(0x40), (f64::from(x9) + r[0]) as f32);
        vm.set_f32(slot(0x44), (f64::from(y8) + r[1]) as f32);
        vm.set_f32(slot(0x48), (f64::from(z7) + r[2]) as f32);
        vm.set_f32(slot(0x30), x9);
        vm.set_f32(slot(0x34), y8);
        vm.set_f32(slot(0x38), z7);
        let mut args = CallArgs::default();
        args.int = [
            Some(f + 0x42e40),
            Some(slot(0x40)),
            Some(slot(0x30)),
            Some(slot(0x3a8)),
        ];
        args.stack = [Some(0), Some(0), Some(0), None];
        let hit = env.call(vm, 0x14195ffc0, args).rax as u8 != 0;
        if hit {
            let depth = (f64::from(vm.f32(slot(0x3ac))) + 0.1 - f64::from(y8)) as f32;
            if depth > 0.0 {
                esi = 1;
            }
            let b = vm.u64(f + 0x20);
            let lowered = vm.f32(f + 0x6548) - depth / vm.f32(b + 0x4454) * DEG;
            vm.set_f32(
                f + 0x6548,
                clamp(lowered, vm.f32(b + 0x444c), vm.f32(b + 0x4450)),
            );
        }
    }
    esi
}

/// `0x14126883f..0x141269920`: the aerodynamic drag of the landing gear. For each of the ten gear records
/// (`B+0x6080`, stride `0x88`) that has a kind word and is either extended (`state+0x10 >= 0.01`) or allowed by
/// `B+0x2830 > 0`, the matching animation state (`F+0x6958`, entries of `0x90` bytes; `0x1407d6c50`) and the
/// drag area from the record (`+0x58`, `+0x5c`, `+0x18`, scaled by the kind: wheel, door, strut, ...) are
/// multiplied by the dynamic pressure `F+0x424` and a power law of the extension (`|e|^0.1`, at least `B+0x2830`),
/// and the sum is applied by `0x140f26ef0` along the air velocity at a point blended from the record and the
/// state's own offsets. `found` tracks whether a live body record names the gear (`body+0x5f4 == index`);
/// it is cleared after a force is applied. The frame slot `rbp+0x1758` (computed before the loop from the wheel
/// groups) and the register state `xmm7 = 0`, `esi = 0` are inputs. Not ported: the debug log.
pub fn gear_aero(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) -> Result<(), String> {
    use crate::callees::{AeroForce, add_aero_force};
    use crate::engine::cosine_blend;
    use crate::flight_step::body_enabled;
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    let mut found = false;
    let power = |x: f32| -> f32 {
        if x > 0.0 {
            x.powf(0.1)
        } else if 0.0 > x {
            -(-x).powf(0.1)
        } else {
            0.0
        }
    };
    for i in 0..10u64 {
        let b = vm.u64(f + 0x20);
        let g = vm.u64(b + 0x6080) + i * 0x88;
        if vm.i32(g) == 0 {
            continue;
        }
        let begin = vm.u64(f + 0x6958);
        let count = (vm.u64(f + 0x6960).wrapping_sub(begin) as i64) / 0x90;
        if i >= count as u64 {
            return Err("gear state index out of range".into());
        }
        let elem = begin + i * 0x90;
        let ext = vm.f32(elem + 0x10);
        let allowed = vm.f32(b + 0x2830) > 0.0;
        if 0.01 > f64::from(ext) && !allowed {
            continue;
        }
        for j in 0..39 {
            if body_enabled(vm, env, f, j) != 0 {
                let body = vm.u64(b + 0x6040) + (j as u64) * 0x34c8;
                if vm.i32(body + 0x5f4) as u64 == i {
                    found = true;
                }
            }
        }
        let b = vm.u64(f + 0x20);
        let (kind, flag2) = (vm.i32(g), vm.i32(g + 4));
        let k74: f32 = if flag2 != 0 {
            f32::from_bits(0x3f95_c28f)
        } else {
            0.125
        };
        let d2 = if kind == 1 { 1.0 } else { 0.2 };
        let g58 = f64::from(vm.f32(g + 0x58));
        let g5c = vm.f32(g + 0x5c);
        let mut x2 = (g58 * d2) as f32;
        if x2 > g5c && kind != 1 {
            x2 = g5c;
        }
        x2 *= vm.f32(g + 0x18);
        let mut x10 = (f64::from(x2) + f64::from(x2)) as f32;
        let mut x6 = (((g58 + g58) * f64::from(g5c)) * 2.0) as f32;
        let (k70, mut x9): (f32, f32);
        if flag2 != 0 {
            k70 = 0.5;
            x9 = x10;
            x10 = (f64::from(x10) * 1.5) as f32;
        } else {
            k70 = 0.0;
            x9 = 0.0;
            if found {
                x6 = (f64::from(x6) * 0.25) as f32;
            }
        }
        x6 = match kind {
            3 => (f64::from(x6) + f64::from(x6)) as f32,
            4 => (f64::from(x6) * 1.2) as f32,
            5 => (f64::from(x6) * 2.4) as f32,
            6 => (f64::from(x6) * 2.8) as f32,
            7 => (f64::from(x6) * 4.0) as f32,
            1 => (f64::from(x6) * 0.0) as f32,
            _ => x6,
        };
        let b2830 = vm.f32(b + 0x2830);
        let pw = power(ext);
        let pmax = if pw > b2830 { pw } else { b2830 };
        let q = vm.f32(f + 0x424);
        x10 = x10 * k74 * q * pmax;
        let x7 = x6 * f32::from_bits(0x3ef0_a3d7) * q * pmax;
        x9 = x9 * k70 * q * pw;
        let x8 = if flag2 != 0 {
            (f64::from(vm.f32(b + 0x2834)) * f64::from(q) * f64::from(ext)
                / f64::from(vm.f32(rbp + 0x1758))) as f32
        } else {
            0.0
        };
        let sum = x7 + x10 + x9 + x8;
        vm.set_f32(rbp.wrapping_sub(0x78), sum);
        let p0 = vm.u64(elem);
        let a = vm.f32(elem + 0x18) * RAD;
        let lever =
            f64::from(vm.f32(p0 + 0x18)) - (1.0 - f64::from(ext)) * f64::from(vm.f32(p0 + 0x20));
        let along = f64::from(vm.f32(p0 + 0x7c)) - f64::from(a.sin()) * lever;
        let gt = vm.u64(b + 0x6080) + i * 0x88;
        let z = ((f64::from(vm.f32(gt + 0x7c)) + along) * 0.5) as f32;
        let cb = {
            // 0x14121b4d0(elem): the blend of the state and its target object
            let t = vm.u64(elem);
            cosine_blend(
                vm.f32(elem + 0x10),
                vm.f32(elem + 0x14),
                vm.f32(elem + 0x18),
                vm.f32(t + 0x18),
                vm.f32(t + 0x20),
                vm.f32(t + 0x70),
            )
        };
        let y =
            ((f64::from(vm.f32(gt + 0x70)) + (cb + f64::from(vm.f32(elem + 0x2c)))) * 0.5) as f32;
        let side = f64::from((vm.f32(elem + 0x14) * RAD).sin()) * lever * f64::from(a.cos())
            + f64::from(vm.f32(p0 + 0x64));
        let x = ((f64::from(vm.f32(gt + 0x64)) + side) * 0.5) as f32;
        let force = AeroForce {
            a2: vm.f32(f + 0x29c),
            a3: 0.0,
            a5: vm.f32(f + 0x2a8),
            a6: sum,
            a7: vm.f32(f + 0x2b4),
            a8: 0.0,
            a9: x,
            a10: y,
            a11: z,
            a12: 0.0,
            a13: 0.0,
            a14: 0,
            a15: 0.0,
            a16: 0.0,
            a17: 0.0,
        };
        add_aero_force(vm, f, &force);
        found = false;
    }
    Ok(())
}

/// `0x1412686a9..0x14126883f`: the number of wheel groups, a float stored at `rbp+0x1758`: the gear records
/// (`B+0x6080`, stride `0x88`) whose first two words (kind and second flag) are both nonzero.
pub fn wheel_groups(vm: &mut Vm, f: u64, rbp: u64) {
    let b = vm.u64(f + 0x20);
    let table = vm.u64(b + 0x6080);
    let mut count = 0.0f32;
    for i in 0..10u64 {
        if vm.i32(table + i * 0x88) != 0 && vm.i32(table + i * 0x88 + 4) != 0 {
            count = (f64::from(count) + 1.0) as f32;
        }
    }
    vm.set_f32(rbp.wrapping_add(0x1758), count);
}

fn world_frame(vm: &Vm, f: u64) -> crate::transform::Frame {
    crate::transform::Frame {
        origin: [vm.f64(f + 0x378), vm.f64(f + 0x380), vm.f64(f + 0x388)],
        rotation: [
            [vm.f32(f + 0x430), vm.f32(f + 0x434)],
            [vm.f32(f + 0x440), vm.f32(f + 0x444)],
            [vm.f32(f + 0x450), vm.f32(f + 0x454)],
        ],
    }
}

/// `0x1407ac020(F, point, ..., shift = 1)`: the point rotated out of the aircraft frame, then each component moved by
/// the position double (three engine-flag queries, x, y, z).
fn to_world(vm: &mut Vm, env: &mut dyn Callees, f: u64, point: [f32; 3]) -> [f32; 3] {
    let frame = world_frame(vm, f);
    let mut out = crate::transform::from_aircraft_frame(&frame, point, false, false);
    for (k, offset) in [0x378u64, 0x380, 0x388].into_iter().enumerate() {
        let shift = position_component(vm, env, f, offset);
        out[k] = (f64::from(out[k]) + shift) as f32;
    }
    out
}

/// `0x14126aad6..0x14126b3ee`: the hook. A static object (`0x1411b63a0` of the table at `0x14578b040`, replayed)
/// holds the hook geometry (rotated through `0x140f33900`); the globals at `0x14589a000..0x14589a020` are the
/// engaged hook's state: the wire index `H` (negative while free), a second index, a progress double and an
/// engagement point of three doubles. With `F+0x28 == 0`, the object and `B+0x4454 > 0`, an engaged hook pulls the
/// arm angle `F+0x6548` toward the wire and applies the pull (`0x1408e3230`, replayed); the tip positions
/// `F+0x654c..0x655c` and `F+0x6550..0x6560` are placed in the world; and, when `esi` (the arm found the surface)
/// and the demand `F+0x6528 > 0.9` allow, each of the three wires is tested for engagement with the probe
/// `0x1411e14d0` (replayed), recording the first engaged wire; a hold timer then eases the engagement point.
/// The register state at the block start is `xmm8 = 0.5` (double), `xmm10 = 0.5`, `xmm12 = 1.0` (double),
/// `r13 = -1`.
#[allow(clippy::field_reassign_with_default)]
pub fn hook_state(vm: &mut Vm, env: &mut dyn Callees, f: u64, esi_in: u32) {
    use crate::transform::{rotate_euler_f64, to_aircraft_frame};
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    const DEG: f32 = f32::from_bits(0x4265_2ee0);
    const TABLE: u64 = 0x1_4578_b040;
    const H: u64 = 0x1_4589_a000;
    const H4: u64 = 0x1_4589_a004;
    const H8: u64 = 0x1_4589_a008;
    const WEIGHT: u64 = 0x1_4589_9ffc;
    let object = env.call(vm, 0x1411b63a0, CallArgs::ints(&[TABLE])).rax;
    let b0 = vm.u64(f + 0x20);
    let usable = vm.i32(f + 0x28) == 0 && object != 0 && vm.f32(b0 + 0x4454) > 0.0;
    if !usable {
        return;
    }
    let height = |vm: &mut Vm, env: &mut dyn Callees| -> f32 {
        let value = f32::from_bits(env.call(vm, 0x1407cd810, CallArgs::ints(&[f])).xmm0 as u32);
        value - vm.f32(TABLE + 0xc8)
    };
    let entry = |vm: &Vm, offset: u64, index: i32| {
        f64::from(vm.f32((TABLE + offset).wrapping_add((i64::from(index) * 8) as u64)))
    };
    let g070 = f64::from(vm.f32(TABLE + 0x30));
    let rotate = |vm: &Vm, a: f64, bb: f64, c: f64, add: bool| {
        let angles = [
            vm.f32(object + 0x80),
            vm.f32(object + 0x84),
            vm.f32(object + 0x88),
        ];
        let offsets = [
            vm.f32(object + 0x64),
            vm.f32(object + 0x68),
            vm.f32(object + 0x6c),
        ];
        rotate_euler_f64(angles, offsets, add, [a, bb, c])
    };
    let frame_time = |vm: &mut Vm, env: &mut dyn Callees| frame_time(vm, env);
    let hook_index = vm.i32(H);
    if hook_index >= 0 {
        let r1 = rotate(
            vm,
            entry(vm, 0x8c, hook_index),
            g070,
            entry(vm, 0xa4, hook_index),
            true,
        );
        let again = vm.i32(H);
        let r2 = rotate(
            vm,
            entry(vm, 0x90, again),
            g070,
            entry(vm, 0xa8, again),
            true,
        );
        let x9 = f64::from(vm.f32(f + 0x654c)) - (r2[0] + r1[0]) * 0.5;
        let x8 = f64::from(vm.f32(f + 0x6554)) - (r2[1] + r1[1]) * 0.5;
        let x6 = f64::from(vm.f32(f + 0x655c)) - (r2[2] + r1[2]) * 0.5;
        let h = height(vm, env);
        let weight = interpolate_clamped(0.0, 0.01, 15.0, 1.0, h);
        let mut args = CallArgs::default();
        args.xmm = [
            Some((x9 as f32).to_bits()),
            Some((x6 as f32).to_bits()),
            None,
            None,
        ];
        let planar = f32::from_bits(env.call(vm, 0x1408be280, args).xmm0 as u32);
        let mut args = CallArgs::default();
        args.xmm = [
            Some(x8.to_bits() as u32),
            Some(planar.to_bits()),
            None,
            None,
        ];
        let angle = f64::from_bits(env.call(vm, 0x1408ce690, args).xmm0);
        let target = (angle * 57.2957763671875) as f32;
        let arm = interpolate_clamped(0.01, vm.f32(f + 0x6548), 1.0, target, weight);
        vm.set_f32(f + 0x6548, arm);
        let x8w = vm.f32(WEIGHT) * vm.f32(f + 0x288) * weight;
        let idx = vm.i32(H);
        let dy = entry(vm, 0xa4, idx) as f32 - entry(vm, 0xa8, idx) as f32;
        let dx = entry(vm, 0x90, idx) as f32 - entry(vm, 0x8c, idx) as f32;
        let heading = dx.atan2(dy) * DEG;
        let a6 = ((f64::from(heading) + 90.0) as f32) * RAD;
        let first = a6.sin() * x8w;
        let neg = -x8w;
        let third = neg * a6.cos();
        let o = rotate(vm, f64::from(first), 0.0, f64::from(third), false);
        let frame = world_frame(vm, f);
        let [t1, _t2, t3] = to_aircraft_frame(
            &frame,
            [o[0] as f32, o[1] as f32, o[2] as f32],
            false,
            false,
        );
        let b = vm.u64(f + 0x20);
        let a = vm.f32(f + 0x6548) * RAD;
        let x7 = a.cos() * t1;
        let x8n = neg * a.sin();
        let x2 = t3 * a.cos();
        let mut args = CallArgs::ints(&[f]);
        args.xmm = [
            None,
            Some(vm.f32(b + 0x4440).to_bits()),
            Some(x7.to_bits()),
            Some(vm.f32(b + 0x4444).to_bits()),
        ];
        args.stack = [
            Some(u64::from(x8n.to_bits())),
            Some(u64::from(vm.f32(b + 0x4448).to_bits())),
            Some(u64::from(x2.to_bits())),
            None,
        ];
        env.call(vm, 0x1408e3230, args);
    }
    // the tips of the arm
    let b = vm.u64(f + 0x20);
    let base = [vm.f32(b + 0x4440), vm.f32(b + 0x4444), vm.f32(b + 0x4448)];
    let out = to_world(vm, env, f, base);
    vm.set_f32(f + 0x654c, out[0]);
    vm.set_f32(f + 0x6554, out[1]);
    vm.set_f32(f + 0x655c, out[2]);
    let a7 = vm.f32(f + 0x6548) * RAD;
    let length = vm.f32(b + 0x4454);
    let tip = [
        vm.f32(b + 0x4440),
        vm.f32(b + 0x4444) - length * a7.sin(),
        length * a7.cos() + vm.f32(b + 0x4448),
    ];
    let out = to_world(vm, env, f, tip);
    vm.set_f32(f + 0x6550, out[0]);
    vm.set_f32(f + 0x6558, out[1]);
    vm.set_f32(f + 0x6560, out[2]);
    if vm.i32(H) >= 0 {
        let dt = frame_time(vm, env);
        let v = f64::from(vm.f32(f + 0x6550)) + dt * f64::from(vm.f32(f + 0x368));
        vm.set_f64(H + 0x10, v);
        let dt = frame_time(vm, env);
        let v = f64::from(vm.f32(f + 0x6558)) + dt * f64::from(vm.f32(f + 0x36c));
        vm.set_f64(H + 0x18, v);
        let dt = frame_time(vm, env);
        let v = f64::from(vm.f32(f + 0x6560)) + dt * f64::from(vm.f32(f + 0x370));
        vm.set_f64(H + 0x20, v);
    }
    let probe = |vm: &mut Vm,
                 env: &mut dyn Callees,
                 a: u64,
                 bb: u64,
                 c: u64,
                 d: u64,
                 x: f32,
                 z: f32|
     -> bool {
        let mut args = CallArgs::ints(&[a, bb, c, d]);
        args.stack = [
            Some(u64::from(x.to_bits())),
            Some(u64::from(z.to_bits())),
            None,
            None,
        ];
        env.call(vm, 0x1411e14d0, args).rax as u32 != 0
    };
    // 0 = fall to the release check, 1 = the engaged check (`0x14126b1c5`)
    let mut to_release_check = false;
    if esi_in != 0 && f64::from(vm.f32(f + 0x6528)) > 0.9 {
        if vm.i32(H) >= 0 {
            to_release_check = true;
        } else {
            let h = height(vm, env);
            if h > 10.0 {
                let (x, z) = (vm.f32(f + 0x6550), vm.f32(f + 0x6560));
                if probe(vm, env, 0, 0, 2, 0, x, z) {
                    let (x, z) = (vm.f32(f + 0x6550), vm.f32(f + 0x6560));
                    if !probe(vm, env, 0, 1, 2, 1, x, z) {
                        for wire in 0..=2u64 {
                            let (x, z) = (vm.f32(f + 0x6550), vm.f32(f + 0x6560));
                            if !probe(vm, env, wire, 0, wire, 1, x, z) {
                                continue;
                            }
                            let dt = frame_time(vm, env);
                            let z2 = (f64::from(vm.f32(f + 0x6560))
                                + dt * f64::from(vm.f32(f + 0x370)))
                                as f32;
                            let dt = frame_time(vm, env);
                            let x2 = (f64::from(vm.f32(f + 0x6550))
                                + dt * f64::from(vm.f32(f + 0x368)))
                                as f32;
                            if probe(vm, env, wire, 0, wire, 1, x2, z2) {
                                continue;
                            }
                            let reach = height(vm, env);
                            vm.set_f32(WEIGHT, ((f64::from(reach * reach)) / 200.0) as f32);
                            vm.set_i32(H, wire as i32);
                            vm.set_i32(H4, wire as i32);
                            vm.set_f64(H8, 0.0);
                        }
                    }
                }
            }
        }
    }
    if to_release_check || vm.i32(H) >= 0 {
        // the engaged check
        let h = height(vm, env);
        if 1.0 > h && 0.5 > vm.f32(f + 0x6528) {
            vm.set_i32(H, -1);
        } else if vm.i32(H) >= 0 {
            return;
        }
    }
    if vm.i32(H4) < 0 {
        return;
    }
    let dt = frame_time(vm, env);
    let progress = vm.f64(H8) + dt * 0.1;
    vm.set_f64(H8, progress);
    if progress > 1.0 {
        vm.set_i32(H4, -1);
    }
    let idx = vm.i32(H4);
    let r1 = rotate(vm, entry(vm, 0x8c, idx), g070, entry(vm, 0xa4, idx), true);
    let idx = vm.i32(H4);
    let r2 = rotate(vm, entry(vm, 0x90, idx), g070, entry(vm, 0xa8, idx), true);
    let targets = [
        (H + 0x10, (r2[0] + r1[0]) * 0.5),
        (H + 0x18, (r2[1] + r1[1]) * 0.5),
        (H + 0x20, (r2[2] + r1[2]) * 0.5),
    ];
    let mut args = CallArgs::default();
    args.xmm = [
        Some((vm.f64(H8) as f32).to_bits()),
        Some(5.0f32.to_bits()),
        None,
        None,
    ];
    let ease = f32::from_bits(env.call(vm, 0x1408625a0, args).xmm0 as u32);
    for (address, target) in targets {
        let value = interpolate_clamped(0.0, vm.f64(address) as f32, 1.0, target as f32, ease);
        vm.set_f64(address, f64::from(value));
    }
}

/// `0x14126b3e8..0x14126b548`: the towing drag and the record update. With a positive drag coefficient `B+0x2894`
/// and `F+0xdac == 0` a force `B+0x2894 * 9.798 * F+0x6590 * ramp(F+0x70)` (the ramp falls from 1 at `0.01` to 0 at
/// 0) acts along the aircraft's second axis; it is moved into the aircraft frame and added as axial (with arm
/// `B+0x289c`), side and normal forces. Then every live record of the list `F+0x69b8` is passed to the singleton's
/// `0x140f39ae0` (replayed, after `0x140f42620`). `rbp` is the frame of `update_flight`; the register state at the
/// block start is `xmm13 = 0.0`, `xmm14 = 1.0`.
#[allow(clippy::field_reassign_with_default)]
pub fn tow_and_records(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) {
    use crate::callees::{add_axial_force, add_normal_force, add_side_force};
    use crate::transform::to_aircraft_frame;
    let b = vm.u64(f + 0x20);
    let coefficient = vm.f32(b + 0x2894);
    if coefficient > 0.0 && vm.i32(f + 0xdac) == 0 {
        let ramp =
            interpolate_clamped(f32::from_bits(0x3c23_d70a), 1.0, 0.0, 0.0, vm.f32(f + 0x70));
        let pull = coefficient * f32::from_bits(0x411c_c5c1) * vm.f32(f + 0x6590) * ramp;
        let frame = world_frame(vm, f);
        let [out1, out2, out3] = to_aircraft_frame(&frame, [0.0, pull, 0.0], false, false);
        let b = vm.u64(f + 0x20);
        let arm = vm.f32(b + 0x289c);
        add_axial_force(vm, f, out3, 0.0, arm);
        add_side_force(vm, f, out1, arm, 0.0);
        add_normal_force(vm, f, out2, 0.0, 0.0);
    }
    let count = |vm: &Vm| vm.u64(f + 0x69c0).wrapping_sub(vm.u64(f + 0x69b8)) as i64 >> 4;
    if count(vm) != 0 {
        let mut i = 0i32;
        loop {
            if crate::flight_step::record_live(vm, env, f, i) {
                let entry = vm.u64(f + 0x69b8) + (i as u64) * 16;
                for k in 0..4 {
                    let word = vm.u32(entry + 4 * k);
                    vm.set_u32(rbp + 4 * k, word);
                }
                let mut args = CallArgs::ints(&[0x1_4578_b780]);
                args.int[1] = None;
                let object = env.call(vm, 0x140f42620, args).rax;
                env.call(vm, 0x140f39ae0, CallArgs::ints(&[object]));
            }
            i += 1;
            if (i64::from(i) as u64) >= count(vm) as u64 {
                break;
            }
        }
    }
}

/// `0x14126b548..0x14126b757`: the drag of the float sections. While `F+0x148 > 0.01`, each of the four records at
/// `B+0x6098` (stride `0x1c8`) with a nonzero first word gets the air velocity at its point (`+0xc/+0x10/+0x14`,
/// `0x14121b580` replayed, with the wash switched on) and a drag along it of `|v|^2 * 1.23 * sin(clamp(|+0x1bc - +0x1b0| - 1,
/// 0, 180) deg) * (+8) * F+0x6c / 2` applied by `0x140f26ef0` at the point. The register state at the block start is
/// `xmm11 = rad`, `xmm12 = 1.0` (double), `xmm13 = 0`, `r13 = -1`, `r14 = 1`, `esi = 0`.
#[allow(clippy::field_reassign_with_default)]
pub fn float_drag(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) {
    use crate::callees::{AeroForce, add_aero_force};
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    if f64::from(vm.f32(f + 0x148)) <= 0.01 || vm.f32(f + 0x148).is_nan() {
        return;
    }
    for k in 0..4u64 {
        let b = vm.u64(f + 0x20);
        let rec = vm.u64(b + 0x6098) + k * 0x1c8;
        if vm.i32(rec) == 0 {
            continue;
        }
        vm.set_i32(slot(0x1758), 0);
        let mut args = CallArgs::default();
        args.int = [Some(f), None, Some(slot(-0x78)), None];
        args.xmm = [
            None,
            Some(vm.f32(rec + 0xc).to_bits()),
            None,
            Some(vm.f32(rec + 0x10).to_bits()),
        ];
        args.stack = [
            Some(slot(-0x80)),
            Some(u64::from(vm.f32(rec + 0x14).to_bits())),
            Some(slot(0x1758)),
            None,
        ];
        env.call(vm, 0x14121b580, args);
        let (a, bb, c) = (
            vm.f32(slot(-0x78)),
            vm.f32(slot(-0x80)),
            vm.f32(slot(0x1758)),
        );
        let sum = a * a + bb * bb + c * c;
        let speed = if 0.0 > sum { f32::NAN } else { sum.sqrt() };
        let b = vm.u64(f + 0x20);
        let rec = vm.u64(b + 0x6098) + k * 0x1c8;
        let spread = (vm.f32(rec + 0x1bc) - vm.f32(rec + 0x1b0)).abs();
        let angle = clamp((f64::from(spread) - 1.0) as f32, 0.0, 180.0);
        let area = (angle * RAD).sin() * vm.f32(rec + 8);
        let drag = (f64::from(speed * speed)
            * (f64::from(area) * 1.23)
            * f64::from(vm.f32(f + 0x6c))
            * 0.5) as f32;
        let force = AeroForce {
            a2: a,
            a3: 0.0,
            a5: bb,
            a6: drag,
            a7: c,
            a8: 0.0,
            a9: vm.f32(rec + 0xc),
            a10: vm.f32(rec + 0x10),
            a11: vm.f32(rec + 0x14),
            a12: 0.0,
            a13: 0.0,
            a14: 0,
            a15: 0.0,
            a16: 0.0,
            a17: 0.0,
        };
        add_aero_force(vm, f, &force);
    }
}

/// `0x14126b757..0x14126bf30`: the float sections' wave interaction. Three sections (`B+0x3f40/0x3f4c/0x3f58`: the
/// point; `B+0x3f64`: the area; `B+0x3f70`: the strength) keep a smoothed air velocity at `F+0x6564/0x6570/0x657c`
/// (three floats each). With the water switch `F+0x650c` clear the stored values follow the points; with it set
/// `F+0x652c` rises at 0.5 per second to 1 and, for every section with strength, the air velocity at its point
/// (`0x14121b580` replayed, wash off) is pushed by the neighbouring sections (the wave function `0x1408bd9d0`
/// replayed, a spacing from the radii `sqrt(area / pi)`), the result is blended into the stored velocity (factor
/// `dt`) and applied as a drag `1.2 * area * F+0x652c * |v|^2 * F+0x6c / 2` by `0x140f26ef0`. Register state:
/// `xmm12 = 1.0` (double), `r13 = -1`, `esi = 0`.
#[allow(clippy::field_reassign_with_default)]
pub fn float_waves(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) {
    use crate::callees::{AeroForce, add_aero_force};
    const PI: f32 = f32::from_bits(0x4049_0fdb);
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    let sqrt = |v: f32| if 0.0 > v { f32::NAN } else { v.sqrt() };
    if vm.i32(f + 0x650c) == 0 {
        vm.set_i32(f + 0x652c, 0);
        let b = vm.u64(f + 0x20);
        for k in 0..3u64 {
            if vm.f32(b + 0x3f64 + 4 * k) > 0.0 {
                for (to, from) in [(0x6564, 0x3f40), (0x6570, 0x3f4c), (0x657c, 0x3f58)] {
                    let word = vm.u32(b + from + 4 * k);
                    vm.set_u32(f + to + 4 * k, word);
                }
            }
        }
        return;
    }
    let dt = frame_time(vm, env);
    let level = (f64::from(vm.f32(f + 0x652c)) + dt * 0.5) as f32;
    vm.set_f32(f + 0x652c, clamp(level, 0.0, 1.0));
    for k in 0..3u64 {
        let b = vm.u64(f + 0x20);
        let strength = vm.f32(b + 0x3f70 + 4 * k);
        // the loop head tests the area, not the strength
        let gate = vm.f32(b + 0x3f64 + 4 * k);
        if gate <= 0.0 || gate.is_nan() {
            continue;
        }
        vm.set_i32(slot(-0x80), 0);
        let mut args = CallArgs::default();
        args.int = [Some(f), None, Some(slot(0x1758)), None];
        args.xmm = [
            None,
            Some(vm.f32(b + 0x3f40 + 4 * k).to_bits()),
            None,
            Some(vm.f32(b + 0x3f4c + 4 * k).to_bits()),
        ];
        args.stack = [
            Some(slot(-0x74)),
            Some(u64::from(vm.f32(b + 0x3f58 + 4 * k).to_bits())),
            Some(slot(-0x80)),
            None,
        ];
        env.call(vm, 0x14121b580, args);
        let mut a = vm.f32(slot(0x1758));
        let mut bb = vm.f32(slot(-0x74));
        let mut c = vm.f32(slot(-0x80));
        let speed = sqrt(a * a + bb * bb + c * c);
        let b = vm.u64(f + 0x20);
        for m in 0..3u64 {
            let area_m = vm.f32(b + 0x3f64 + 4 * m);
            if area_m <= 0.0 || area_m.is_nan() || m == k {
                continue;
            }
            let rsum = sqrt(vm.f32(b + 0x3f64 + 4 * k) / PI) + sqrt(area_m / PI);
            let time = vm.f64(0x1_42f0_1918) as f32;
            let wave = |vm: &mut Vm, env: &mut dyn Callees, index: u64| -> f64 {
                let mut args = CallArgs::default();
                args.xmm[0] = Some(time.to_bits());
                args.int[1] = Some(index);
                let w = f32::from_bits(env.call(vm, 0x1408bd9d0, args).xmm0 as u32);
                f64::from(w) * 0.5 + 1.0
            };
            let pk = |vm: &Vm, base: u64, i: u64| vm.f32(f + base + 4 * i);
            let w = wave(vm, env, 3 * k);
            let dx = (w * f64::from(pk(vm, 0x6564, k) - pk(vm, 0x6564, m))) as f32;
            let w = wave(vm, env, 3 * k + 1);
            let dy = (w * f64::from(pk(vm, 0x6570, k) - pk(vm, 0x6570, m))) as f32;
            let w = wave(vm, env, 3 * k + 2);
            let dz = (w * f64::from(pk(vm, 0x657c, k) - pk(vm, 0x657c, m))) as f32;
            let dist = sqrt(dy * dy + dx * dx + dz * dz);
            let d5 = if dist > 0.01 { dist } else { 0.01 };
            let x4 = (f64::from(rsum) + f64::from(rsum)) as f32;
            let fac = if x4 == 0.0 {
                0.5
            } else {
                clamp(1.0 - (1.0 / (x4 - 0.0)) * (d5 - 0.0), 0.0, 1.0)
            };
            let sc = f64::from(speed) * 0.1;
            a = (f64::from(a) + f64::from(dx / d5) * sc * f64::from(fac)) as f32;
            bb = (f64::from(bb) + f64::from(dy / d5) * sc * f64::from(fac)) as f32;
            c = (f64::from(c) + f64::from(dz / d5) * sc * f64::from(fac)) as f32;
        }
        let speed2 = sqrt(a * a + bb * bb + c * c);
        let level = vm.f32(f + 0x652c);
        let s8 = level * strength;
        let b = vm.u64(f + 0x20);
        let area_k = vm.f32(b + 0x3f64 + 4 * k);
        let q = speed2 * speed2 * vm.f32(f + 0x6c);
        let magnitude = (f64::from(area_k) * 1.2 * f64::from(level) * f64::from(q) * 0.5) as f32;
        vm.set_f32(slot(-0x78), magnitude);
        let dt = frame_time(vm, env);
        let t = clamp(dt as f32, 0.0, 1.0);
        let held = if speed2 > 1.0 { speed2 } else { 1.0 };
        let x5 = s8 * a / held;
        let x2 = ((f64::from(bb) - 1.0) * f64::from(s8) / f64::from(held)) as f32;
        let x8 = s8 * c / held;
        for (base, value) in [(0x6564u64, x5), (0x6570, x2), (0x657c, x8)] {
            let old = vm.f32(f + base + 4 * k);
            vm.set_f32(f + base + 4 * k, (1.0 - t) * old + t * value);
        }
        let b = vm.u64(f + 0x20);
        let force = AeroForce {
            a2: a,
            a3: 0.0,
            a5: bb,
            a6: magnitude,
            a7: c,
            a8: 0.0,
            a9: vm.f32(b + 0x3f40 + 4 * k),
            a10: vm.f32(b + 0x3f4c + 4 * k),
            a11: vm.f32(b + 0x3f58 + 4 * k),
            a12: 0.0,
            a13: 0.0,
            a14: 0,
            a15: 0.0,
            a16: 0.0,
            a17: 0.0,
        };
        add_aero_force(vm, f, &force);
    }
}

/// `0x14126bf30..0x14126c034`: the external pull and the pedal sum. With `F+0x28 == 0` the two interface updates
/// `0x1411e4bd0(0x145899fa0)` and `0x1409057f0(0x142edf4b0)` run (replayed); with `F+0x28 == 1` and one of the globals
/// `0x145899fd0/fd4` set, the force `-(0x145899fc4, fc8, fcc)` (world axes) is added at `(0, B+0x2638, B+0x263c)`.
/// The larger in magnitude of `F+0xf0` and `F+0xfc` (the second wins a tie) plus `F+0x108` is stored as a float at
/// `rbp+0x1750`; when `F+0x3c == 1` and `0x1411d9d80(F)` (replayed) is zero it is `F+0x108 + F+0xec` instead.
pub fn world_pull(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) {
    use crate::callees::add_world_force;
    match vm.i32(f + 0x28) {
        0 => {
            env.call(vm, 0x1411e4bd0, CallArgs::ints(&[0x1_4589_9fa0]));
            env.call(vm, 0x1409057f0, CallArgs::ints(&[0x1_42ed_f4b0]));
        }
        1 if vm.i32(0x1_4589_9fd0) != 0 || vm.i32(0x1_4589_9fd4) != 0 => {
            let b = vm.u64(f + 0x20);
            let force = [
                -vm.f32(0x1_4589_9fc4),
                -vm.f32(0x1_4589_9fc8),
                -vm.f32(0x1_4589_9fcc),
            ];
            add_world_force(vm, f, [0.0, vm.f32(b + 0x2638), vm.f32(b + 0x263c)], force);
        }
        _ => {}
    }
    let (second, first) = (vm.f32(f + 0xfc), vm.f32(f + 0xf0));
    let mut pick = if first.abs() > second.abs() {
        first
    } else {
        second
    };
    let f108 = vm.f32(f + 0x108);
    pick += f108;
    vm.set_f32(rbp.wrapping_add(0x1750), pick);
    if vm.i32(f + 0x3c) == 1 && env.call(vm, 0x1411d9d80, CallArgs::ints(&[f])).rax as u32 == 0 {
        vm.set_f32(rbp.wrapping_add(0x1750), f108 + vm.f32(f + 0xec));
    }
}

/// `0x14126c034..0x14126c4d7`: the gear animation targets. Unless `F+0x28 == 0` and `F+0x689c` is set, each gear
/// record (`B+0x6080`, stride `0x88`, ten of them) with a kind other than 0 and 1 and a live target (`+0x50` or `+0x54`
/// above 0.01 or `+0xc` set) writes the target of its animation state (`F+0x6958`, `+0x24` of the `0x90`-byte entry):
/// with `F+0x214` set the value `F+0x218`; with a held-down key among the slots of the table at `0x1460e9708` (kind `0x25`
/// or `0x49`, enabled by the parallel table `+0xba44`) the lever `F+0x20c + F+0x208` times `+0x50` plus the pedal sum
/// `rbp+0x1750` times `+0x54`, held to `+-(+0x50)`; otherwise `+0x50` moved toward `+0x54` by `sqrt(clamp(1 -
/// (1 / B+0x1e94) * (max(+0x58, 0.01) * entry0+0x40 * 1.9438445), 0, 1)) - 1` (`0.5` when `B+0x1e94` is zero), held
/// between them and multiplied by the pedal sum. Then, with `B+0xe78`, the value is multiplied by `0x1411daa80`
/// (replayed), by the extension `+0x10` of the entry, and by `-1` when the entry's own height term is positive.
/// The register state at the block start is `xmm7 = 0.01` (double), `xmm10 = 0.5`, `xmm12 = 1.0` (double),
/// `xmm8 = 0`, and the slot `rbp+0x1750`.
pub fn gear_targets(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) -> Result<(), String> {
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    const KNOTS: f32 = f32::from_bits(0x3ff8_cfe5);
    const TABLE: u64 = 0x1_460e_9708;
    let slot = |off: u64| rbp.wrapping_add(off);
    if vm.i32(f + 0x28) == 0 && vm.i32(f + 0x689c) != 0 {
        return Ok(());
    }
    let sqrt = |v: f32| if 0.0 > v { f32::NAN } else { v.sqrt() };
    for i in 0..10u64 {
        let b = vm.u64(f + 0x20);
        let g = vm.u64(b + 0x6080) + i * 0x88;
        let kind = vm.i32(g);
        if kind == 0 || kind == 1 {
            continue;
        }
        let live = f64::from(vm.f32(g + 0x50)) > 0.01
            || f64::from(vm.f32(g + 0x54)) > 0.01
            || vm.i32(g + 0xc) != 0;
        if !live {
            continue;
        }
        let begin = vm.u64(f + 0x6958);
        let count = (vm.u64(f + 0x6960).wrapping_sub(begin) as i64) / 0x90;
        let elem = begin + i * 0x90;
        let in_range = (i as i64) < count && count > 0;
        if vm.i32(f + 0x214) != 0 {
            if !in_range {
                return Err("gear state index out of range".into());
            }
            vm.set_f32(elem + 0x24, vm.f32(f + 0x218));
            continue;
        }
        let pressed = |code: i32| {
            (0..500u64)
                .any(|k| vm.i32(TABLE + 4 * k + 0xba44) != 0 && vm.i32(TABLE + 4 * k) == code)
        };
        let pedal = vm.f32(slot(0x1750));
        let mut value;
        if pressed(0x25) || pressed(0x49) {
            let (g50, g54) = (vm.f32(g + 0x50), vm.f32(g + 0x54));
            if !in_range {
                return Err("gear state index out of range".into());
            }
            vm.set_f32(
                elem + 0x24,
                (vm.f32(f + 0x20c) + vm.f32(f + 0x208)) * g50 + pedal * g54,
            );
            let v = vm.f32(elem + 0x24);
            let neg = -g50;
            value = if neg > v {
                neg
            } else if g50 < v {
                g50
            } else {
                v
            };
        } else {
            if count == 0 {
                return Err("gear state vector empty".into());
            }
            let first = vm.u64(b + 0x6080);
            let reach = {
                let w = vm.f32(first + 0x58);
                (if w > 0.01 { w } else { 0.01 }) * vm.f32(begin + 0x40)
            };
            let lever = vm.f32(b + 0x1e94);
            let held = if lever == 0.0 {
                0.5
            } else {
                let t = (1.0 / (lever - 0.0)) * (reach * KNOTS - 0.0);
                let v = 1.0 - t;
                clamp(v, 0.0, 1.0)
            };
            let s = sqrt(held);
            let (g50, g54) = (vm.f32(g + 0x50), vm.f32(g + 0x54));
            let hi = if g50 > g54 { g50 } else { g54 };
            let lo = if g50 < g54 { g50 } else { g54 };
            let v = g50 - (g54 - g50) * (s - 1.0);
            let moved = if lo > v {
                lo
            } else if hi < v {
                hi
            } else {
                v
            };
            value = pedal * moved;
            if !in_range {
                return Err("gear state index out of range".into());
            }
        }
        vm.set_f32(elem + 0x24, value);
        let b = vm.u64(f + 0x20);
        if vm.i32(b + 0xe78) != 0 {
            let scale = f32::from_bits(
                env.call(vm, 0x1411daa80, CallArgs::ints(&[f + 0xbdd8]))
                    .xmm0 as u32,
            );
            value = scale * vm.f32(elem + 0x24);
            vm.set_f32(elem + 0x24, value);
        }
        let ext = vm.f32(elem + 0x10);
        vm.set_f32(slot(0x1758), ext);
        let scaled = vm.f32(elem + 0x24) * ext;
        vm.set_f32(elem + 0x24, scaled);
        let p0 = vm.u64(elem);
        let a = vm.f32(elem + 0x18) * RAD;
        let lever =
            f64::from(vm.f32(p0 + 0x18)) - (1.0 - f64::from(ext)) * f64::from(vm.f32(p0 + 0x20));
        let along = f64::from(vm.f32(p0 + 0x7c)) - f64::from(a.sin()) * lever;
        let factor = if along > 0.0 { -1.0 } else { 1.0 };
        vm.set_f32(elem + 0x24, (f64::from(scaled) * factor) as f32);
    }
    Ok(())
}

/// `0x14126c4e4..0x14126c7dc`: the nose-wheel and brake demand, only while `F+0x3c == 1`. The demands `F+0x240` and
/// `F+0x244` follow the pedal `rbp+0x1750` unless the brake or the pause conditions hold (`F+0x28 == 0` with
/// `F+0x6824`, the globals `0x142fe2a48`, `0x1460e0a1c`, `0x1460e0a6c`, a held key of code 6 or 7 in the key table,
/// the queries `0x1417dacf0(0x1460e0a10, 0x1d9/0x1da)` (replayed), `B+0x2840 < 0.01` or `F+0x23c != 0`): the pedal
/// ramp `interpolate(|p|, 0.5 to 1)` signed like the pedal is combined with the speed ramp from `0x14123d110`
/// (replayed) and `0x1408625a0(pedal, B+0x2840)` (replayed) either into the left demand alone or as a split of the
/// average of both, and held to `0..1`. Finally, with `B+0xe94`, `F+0x238 == 0` and both demands above 0.9 the
/// latches `F+0x228`, `F+0x22c`, `F+0x220` are cleared (the last two only with `B+0xe90`), and with `B+0xe90 == 1`
/// and `F+0x22c == 1` the maximum of the demands and `F+0x224` becomes `F+0x220` and `F+0x224`. The register state
/// is `xmm7 = 0.01` (double), `xmm10 = 0.5`.
#[allow(clippy::field_reassign_with_default, clippy::needless_late_init)]
pub fn steering_state(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) {
    use crate::scalar::sign;
    const TABLE: u64 = 0x1_460e_9708;
    let slot = |off: u64| rbp.wrapping_add(off);
    if vm.i32(f + 0x3c) != 1 {
        return;
    }
    let b = vm.u64(f + 0x20);
    let held = |vm: &Vm, code: i32| {
        (0..500u64).any(|k| vm.i32(TABLE + 4 * k + 0xba44) != 0 && vm.i32(TABLE + 4 * k) == code)
    };
    let blocked = (vm.i32(f + 0x28) == 0 && vm.i32(f + 0x6824) != 0)
        || vm.i32(0x1_42fe_2a48) != 0
        || vm.i32(0x1_460e_0a1c) != 0
        || vm.i32(0x1_460e_0a6c) != 0
        || held(vm, 6)
        || held(vm, 7);
    let mut proceed = !blocked;
    for id in [0x1d9u64, 0x1da] {
        if proceed
            && env
                .call(vm, 0x1417dacf0, CallArgs::ints(&[0x1_460e_0a10, id]))
                .rax as u8
                != 0
        {
            proceed = false;
        }
    }
    if proceed {
        let b2840 = vm.f32(b + 0x2840);
        proceed = f64::from(b2840) >= 0.01 && vm.f32(f + 0x23c) == 0.0;
    }
    let float_call = |vm: &mut Vm, env: &mut dyn Callees, address: u64, args: CallArgs| {
        f32::from_bits(env.call(vm, address, args).xmm0 as u32)
    };
    if proceed {
        let pedal = vm.f32(slot(0x1750));
        let ramp = interpolate_clamped(0.5, 0.0, 1.0, 1.0, pedal.abs());
        let signed = ramp * sign(pedal);
        vm.set_f32(slot(0x1758), signed);
        let speed = vm.f32(f + 0x41c);
        let reach = float_call(vm, env, 0x14123d110, CallArgs::ints(&[f]));
        let speed_ramp = interpolate_clamped(0.0, 1.0, reach, 0.0, speed);
        let root = if 0.0 > speed_ramp {
            f32::NAN
        } else {
            speed_ramp.sqrt()
        };
        let toggle = env
            .call(vm, 0x1411e3ee0, CallArgs::ints(&[0x1_460e_0a10, 0x4e]))
            .rax as u8
            != 0;
        let b2840 = vm.f32(b + 0x2840);
        let pull = |vm: &mut Vm, env: &mut dyn Callees, x: f32| {
            let mut args = CallArgs::default();
            args.xmm = [Some(x.to_bits()), Some(b2840.to_bits()), None, None];
            float_call(vm, env, 0x1408625a0, args)
        };
        let first = pull(vm, env, signed);
        let value;
        if !toggle {
            let left = clamp(-first * root, 0.0, 1.0);
            vm.set_f32(f + 0x240, left);
            let second = pull(vm, env, vm.f32(slot(0x1758)));
            value = second * root;
        } else {
            let mid = (vm.f32(f + 0x244) + vm.f32(f + 0x240)) * 0.5;
            let left = clamp(mid - first * root, 0.0, 1.0);
            vm.set_f32(f + 0x240, left);
            let second = pull(vm, env, vm.f32(slot(0x1758)));
            value = second * root + mid;
        }
        vm.set_f32(f + 0x244, clamp(value, 0.0, 1.0));
    }
    let b = vm.u64(f + 0x20);
    if vm.i32(b + 0xe94) != 0
        && vm.f32(f + 0x238) == 0.0
        && f64::from(vm.f32(f + 0x240)) > 0.9
        && f64::from(vm.f32(f + 0x244)) > 0.9
    {
        vm.set_i32(f + 0x228, 0);
        if vm.i32(b + 0xe90) != 0 {
            vm.set_i32(f + 0x22c, 0);
        }
        vm.set_i32(f + 0x220, 0);
    }
    if vm.i32(b + 0xe90) == 1 && vm.i32(f + 0x22c) == 1 {
        let (d244, d240, d224) = (vm.f32(f + 0x244), vm.f32(f + 0x240), vm.f32(f + 0x224));
        let m = if d224 > d240 { d224 } else { d240 };
        let m = if d244 > m { d244 } else { m };
        vm.set_f32(f + 0x220, m);
        vm.set_f32(f + 0x224, m);
    }
}

/// `0x14126c7dc..0x14126d1f2`: the gear brake state. For each of ten gears (`B+0x6080` field `+8` set) the brake
/// target `+0x50` of its animation entry (`F+0x6958`, `0x90` bytes) takes the demand `F+0x224`, adds the left demand
/// `F+0x240` when the wheel's lateral offset `sin(entry+0x14) * (p0+0x18 - (1 - ext) * p0+0x20) * cos(entry+0x18) +
/// p0+0x64` is below -0.01 and the right demand `F+0x244` when it is above 0.01, each cleared by its binding
/// (`0x72`, `0x73`); with `B+0xe7c` a decaying memory (`F+0xbe28` and the history `F+0xbe00..`) limits it through
/// `0x1411daa80` (replayed). Unless the sim speed `0x142f01920` is above 1 with `F+0x24c` set and `F+0x64e4` clear, the
/// world frame of the point `(B+0x280c, y, B+0x2810)` (`0x140816eb0`, replayed) is converted to the geographic doubles
/// `F+0x64e8/0x64f0/0x64f8` (`0x1406eaf20`, replayed). Then `F+0x24c/0x250` are cleared, the two floats `+0x2d0/+0x2d4`
/// of the element record of every enabled wing (`F+0x6940`, stride `0x2d8`) are cleared, and for each gear with a kind
/// the entry's `+0x20` takes the record's `+0x24` and `+0x2c..0x40`, `+0x64`, `+0x5c` are cleared. The register
/// state at the block start is `xmm7 = 0.01` (double), `xmm9 = 0x7fffffff`, `xmm12 = 1.0` (double), `r14 = 0`.
#[allow(clippy::field_reassign_with_default)]
pub fn gear_state_update(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    rbp: u64,
) -> Result<(), String> {
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    let in_range = |vm: &Vm, i: u64| {
        let begin = vm.u64(f + 0x6958);
        (i as i64) < (vm.u64(f + 0x6960).wrapping_sub(begin) as i64) / 0x90
    };
    let side = |vm: &Vm, elem: u64| -> f64 {
        let p0 = vm.u64(elem);
        let a = f64::from((vm.f32(elem + 0x14) * RAD).sin());
        let lever = f64::from(vm.f32(p0 + 0x18))
            - (1.0 - f64::from(vm.f32(elem + 0x10))) * f64::from(vm.f32(p0 + 0x20));
        a * lever * f64::from((vm.f32(elem + 0x18) * RAD).cos()) + f64::from(vm.f32(p0 + 0x64))
    };
    let bind = |vm: &mut Vm, env: &mut dyn Callees, id: u64| {
        let mut args = CallArgs::ints(&[f, 1, id, 0]);
        args.stack[0] = Some(u64::from(1.0f32.to_bits()));
        env.call(vm, 0x1407ace10, args).rax as u32 != 0
    };
    for i in 0..10u64 {
        if !in_range(vm, i) {
            return Err("gear state index out of range".into());
        }
        let begin = vm.u64(f + 0x6958);
        let elem = begin + i * 0x90;
        vm.set_i32(elem + 0x50, 0);
        let b = vm.u64(f + 0x20);
        if vm.i32(vm.u64(b + 0x6080) + i * 0x88 + 8) == 0 {
            continue;
        }
        let base = vm.f32(f + 0x224) + vm.f32(elem + 0x50);
        vm.set_f32(elem + 0x50, base);
        if -0.01 > side(vm, elem) {
            vm.set_f32(elem + 0x50, vm.f32(f + 0x240) + base);
            if bind(vm, env, 0x72) {
                vm.set_i32(elem + 0x50, 0);
            }
        }
        if side(vm, elem) > 0.01 {
            vm.set_f32(elem + 0x50, vm.f32(f + 0x244) + vm.f32(elem + 0x50));
            if bind(vm, env, 0x73) {
                vm.set_i32(elem + 0x50, 0);
            }
        }
        let b = vm.u64(f + 0x20);
        if vm.i32(b + 0xe7c) != 0 {
            let memory = f + 0xbdd8;
            let held = vm.f32(elem + 0x50);
            let owner = vm.u64(memory + 8);
            let mut args = CallArgs::ints(&[memory, u64::from(vm.u32(owner + 0xe7c))]);
            args.int[1] = Some(u64::from(vm.u32(owner + 0xe7c)));
            let floor = f32::from_bits(env.call(vm, 0x1411daa80, args).xmm0 as u32);
            let decay = if vm.i32(0x1_42f0_1968) != 0 {
                f32::from_bits(0x3681_742e)
            } else {
                0.0
            };
            let lowered = vm.f32(memory + 0x50) - decay;
            let level = if lowered > floor { lowered } else { floor };
            vm.set_f32(memory + 0x50, level);
            let change = (vm.f32(memory + 0x28 + 4 * i) - held).abs();
            let eased = (f64::from(level) - f64::from(change) * 0.25) as f32;
            vm.set_f32(memory + 0x50, eased);
            vm.set_f32(memory + 0x28 + 4 * i, held);
            let x1 = vm.f32(memory + 0x50);
            let first = clamp(x1, 0.0, 1.0);
            vm.set_f32(memory + 0x50, first);
            let e = vm.f32(elem + 0x50);
            let result = if 0.0 > e {
                0.0
            } else if first < e {
                first
            } else {
                e
            };
            vm.set_f32(elem + 0x50, result);
        }
        let e = vm.f32(elem + 0x50);
        vm.set_f32(elem + 0x50, clamp(e, 0.0, 1.0));
    }
    let sim_speed = vm.f64(0x1_42f0_1920);
    if 1.0 > sim_speed || vm.i32(f + 0x24c) == 0 || vm.i32(f + 0x64e4) == 0 {
        let b = vm.u64(f + 0x20);
        let mut args = CallArgs::default();
        args.int[0] = Some(f);
        args.int[2] = Some(slot(-0x38));
        args.xmm[1] = Some(f64::from(vm.f32(b + 0x280c)).to_bits() as u32);
        args.xmm[3] = Some(f64::from(vm.f32(b + 0x2810)).to_bits() as u32);
        args.stack = [
            Some(slot(-0x68)),
            Some(f64::from(vm.f32(b + 0x2814)).to_bits()),
            Some(slot(0x1758)),
            Some(1),
        ];
        env.call(vm, 0x140816eb0, args);
        let ctx = env.call(vm, 0x14193ae40, CallArgs::default()).rax;
        let mut args = CallArgs::default();
        args.int = [
            Some(ctx),
            Some(f + 0x64e8),
            Some(f + 0x64f0),
            Some(f + 0x64f8),
        ];
        args.stack = [
            Some(vm.f64(slot(-0x38)).to_bits()),
            Some(vm.f64(slot(-0x68)).to_bits()),
            Some(vm.f64(slot(0x1758)).to_bits()),
            None,
        ];
        env.call(vm, 0x1406eaf20, args);
    }
    vm.set_u32(f + 0x24c, 0);
    vm.set_u32(f + 0x250, 0);
    for w in 0..48u64 {
        let b = vm.u64(f + 0x20);
        if vm.u8(vm.u64(b + 0x6028) + w * 0x36c8 + 0x678) != 0 {
            let x = vm.u64(f + 0x6940) + w * 0x2d8;
            vm.set_u32(x + 0x2d0, 0);
            vm.set_u32(x + 0x2d4, 0);
        }
    }
    for i in 0..10u64 {
        let b = vm.u64(f + 0x20);
        let g = vm.u64(b + 0x6080) + i * 0x88;
        if vm.i32(g) == 0 {
            continue;
        }
        if !in_range(vm, i) {
            return Err("gear state index out of range".into());
        }
        let elem = vm.u64(f + 0x6958) + i * 0x90;
        vm.set_u32(elem + 0x20, vm.u32(g + 0x24));
        for off in [0x2c, 0x30, 0x34, 0x38, 0x3c, 0x40, 0x64, 0x5c] {
            vm.set_u32(elem + off, 0);
        }
    }
    Ok(())
}

/// `0x14126d1f2..0x14126d5d3`: the wheel contact. The latch value `F+0x224` moves toward `F+0x220` (at 1 per second,
/// 10 per second when it is above the target and the latches `B+0xe90` / `F+0x22c` do not both hold; not at all while
/// both hold and the target is below it or `B+0xe90 >= 2`); `0x1411878f0(F)` runs (replayed) and `F+0x64c8` eases
/// toward `F+0x64c4` by `2 dt`. Then each of the ten gears with a kind is handed to the contact function (replayed):
/// the tire function `0x1411c8690(entry, F+0x28, i)` when the gear's kind is not 1 and `|B+0x65a8|`, `|F+0x348|`,
/// `|F+0x350|` are below 45 degrees, otherwise the strut function `0x1411c7a50` with the foot point computed from the
/// entry's pose. The contact counts add up in `rbp+0x1758`; afterwards `F+0x24c = count > 0`, `F+0x250 = count >= 3`
/// and `F+0x64cc = pedal * B+0x2808`. Register state: `xmm9 = 0x7fffffff`, `xmm11 = rad`, `xmm12 = 1.0` (double).
#[allow(clippy::field_reassign_with_default)]
pub fn wheel_contact(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) -> Result<(), String> {
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    let b = vm.u64(f + 0x20);
    let rising = vm.f32(f + 0x224) > vm.f32(f + 0x220);
    let (latch, held) = (vm.i32(b + 0xe90), vm.i32(f + 0x22c) != 0);
    let mut rate = 1.0f64;
    let adjust = if latch != 0 && held {
        !rising && latch < 2
    } else {
        if rising {
            rate = 10.0;
        }
        true
    };
    if adjust {
        let dt = frame_time(vm, env);
        let step = (dt * rate) as f32;
        let (target, current) = (vm.f32(f + 0x220), vm.f32(f + 0x224));
        let gap = target - current;
        let lowered = -step;
        let delta = if lowered > gap {
            lowered
        } else if step < gap {
            step
        } else {
            gap
        };
        vm.set_f32(f + 0x224, current + delta);
    }
    env.call(vm, 0x1411878f0, CallArgs::ints(&[f]));
    let dt = frame_time(vm, env);
    let factor = (dt + dt) as f32;
    vm.set_f32(
        f + 0x64c8,
        lerp(vm.f32(f + 0x64c8), vm.f32(f + 0x64c4), factor),
    );
    let mut contacts = 0i32;
    vm.set_i32(slot(0x1758), 0);
    for i in 0..10u64 {
        let b = vm.u64(f + 0x20);
        let g = vm.u64(b + 0x6080) + i * 0x88;
        let kind = vm.i32(g);
        if kind == 0 {
            continue;
        }
        let begin = vm.u64(f + 0x6958);
        let count = (vm.u64(f + 0x6960).wrapping_sub(begin) as i64) / 0x90;
        if (i as i64) >= count {
            return Err("gear state index out of range".into());
        }
        let elem = begin + i * 0x90;
        let tilt_ok = vm.f32(b + 0x65a8).abs() < 45.0
            && vm.f32(f + 0x348).abs() < 45.0
            && vm.f32(f + 0x350).abs() < 45.0;
        let result = if kind != 1 && tilt_ok {
            env.call(
                vm,
                0x1411c8690,
                CallArgs::ints(&[elem, u64::from(vm.u32(f + 0x28)), i]),
            )
            .rax as i32
        } else {
            let a7 = vm.f32(b + 0x65a8) * RAD;
            let drop = -vm.f32(f + 0x42f50) * a7.sin();
            let p0 = vm.u64(elem);
            let a8 = vm.f32(elem + 0x18) * RAD;
            let lever = f64::from(vm.f32(p0 + 0x18))
                - (1.0 - f64::from(vm.f32(elem + 0x10))) * f64::from(vm.f32(p0 + 0x20));
            let x11 = (f64::from(vm.f32(p0 + 0x7c)) - f64::from(a8.sin()) * lever) as f32;
            let b = vm.u64(f + 0x20);
            let g = vm.u64(b + 0x6080) + i * 0x88;
            let x10 = vm.f32(g + 0x30) / vm.f32(g + 0x34);
            let x9 = a7.cos() * vm.f32(f + 0x42f50);
            let a14 = vm.f32(elem + 0x14) * RAD;
            let cos8 = f64::from(a8.cos());
            let x6 = (f64::from(vm.f32(p0 + 0x70)) - f64::from(a14.cos()) * lever * cos8) as f32;
            let x2 = (f64::from(a14.sin()) * lever * cos8 + f64::from(vm.f32(p0 + 0x64))) as f32;
            let mut args = CallArgs::default();
            args.int = [Some(f), Some(i), None, None];
            args.xmm = [None, None, Some(x2.to_bits()), Some(0)];
            args.stack = [
                Some(u64::from(x6.to_bits())),
                Some(u64::from(x9.to_bits())),
                Some(u64::from(x10.to_bits())),
                Some(u64::from(x11.to_bits())),
            ];
            let _ = drop;
            env.call(vm, 0x1411c7a50, args).rax as i32
        };
        contacts = vm.i32(slot(0x1758)).wrapping_add(result);
        vm.set_i32(slot(0x1758), contacts);
    }
    vm.set_i32(f + 0x24c, i32::from(contacts > 0));
    vm.set_i32(f + 0x250, i32::from(contacts >= 3));
    let b = vm.u64(f + 0x20);
    vm.set_f32(f + 0x64cc, vm.f32(slot(0x1750)) * vm.f32(b + 0x2808));
    vm.set_i32(slot(-0x78), 0);
    Ok(())
}
