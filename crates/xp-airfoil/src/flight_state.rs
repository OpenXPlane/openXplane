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
