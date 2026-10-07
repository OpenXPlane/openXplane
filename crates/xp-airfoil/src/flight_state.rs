//! The later blocks of the flight step (`0x1412656b0`): bookkeeping and derived quantities computed after the
//! state advance. Each function is a block of the original, compared with the machine code in an emulator
//! (`tools/gen_flight_block_vectors.py`), with the callees outside this port replayed.

use crate::flight_step::position_component;
use crate::scalar::{clamp, lerp};
use crate::vm::{CallArgs, Callees, Vm};
use crate::wing_element::{hypot2, hypot3, interpolate_clamped, signed_sqrt};

fn frame_time(vm: &mut Vm, env: &mut dyn Callees) -> f64 {
    f64::from_bits(env.call(vm, 0x140c448c0, CallArgs::default()).xmm0)
}

fn planet(vm: &mut Vm, env: &mut dyn Callees) -> u64 {
    env.call(vm, 0x14193ae40, CallArgs::default()).rax
}

/// `0x1407d76f0(F, dt)`: the height of the aircraft above the ground: its altitude `F+0x3a0` plus `dt` times the
/// vertical velocity `F+0x3f8`, minus the altitude of the ground point `(F+0x378, F+0x42f5c, F+0x388)` (floats;
/// `local_to_geodetic`) and `B+0x65a4`.
fn height_above_ground(vm: &mut Vm, env: &mut dyn Callees, f: u64, dt: f32) -> f32 {
    let planet = planet(vm, env);
    let alt = position_component(vm, env, f, 0x3a0);
    let top = (f64::from(dt * vm.f32(f + 0x3f8)) + alt) as f32;
    let z = position_component(vm, env, f, 0x388) as f32;
    let ground = vm.f32(f + 0x42f5c);
    let x = position_component(vm, env, f, 0x378) as f32;
    let scratch = [0x7f00_0000u64, 0x7f00_0008, 0x7f00_0010];
    local_to_geodetic(
        vm,
        env,
        planet,
        scratch,
        [f64::from(x), f64::from(ground), f64::from(z)],
    );
    let out = vm.f64(scratch[2]) as f32;
    let b = vm.u64(f + 0x20);
    (top - out) - vm.f32(b + 0x65a4)
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
            let height = height_above_ground(vm, env, f, 0.0) * FEET;
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
            let height = height_above_ground(vm, env, f, 0.0) * FEET;
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
        planet(vm, env);
        let p = |vm: &Vm, k: usize| vm.f64(f + points[k]);
        let distance = great_circle(vm, p(vm, 0), p(vm, 1), p(vm, 2), p(vm, 3), 0, 0);
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
    let variation = magnetic_variation(vm, 0x1_4589_0720, lon, lat);
    vm.set_f32(f + 0x428, variation);
    // `0x141244b60(F, angle)`: `angle + F+0x428` wrapped to 0..360
    let mut value = vm.f32(f + 0x358) + vm.f32(f + 0x428);
    while 0.0 > value {
        value += 360.0;
    }
    while value > 360.0 {
        value += -360.0;
    }
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
    let reference = heading_blend(vm, f, 2.0);
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

/// `0x141273dfb..0x14127402d`: the last state updates. With `B+0xc54` the start sequence `0x141245750` (`start_sequence`)
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
        start_sequence(vm, env, f);
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
pub fn hook_state(vm: &mut Vm, env: &mut dyn Callees, f: u64, esi_in: u32) -> Result<(), String> {
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
        return Ok(());
    }
    let height = |vm: &mut Vm, _env: &mut dyn Callees| -> f32 {
        let value = hypot3(vm.f32(f + 0x368), vm.f32(f + 0x36c), vm.f32(f + 0x370));
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
        let planar = hypot2(x9 as f32, x6 as f32);
        let angle = x8.atan2(f64::from(planar));
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
        let (pa, pc, pp) = (vm.f32(b + 0x4440), vm.f32(b + 0x4444), vm.f32(b + 0x4448));
        add_plugin_force(vm, env, f, pa, x7, pc, x8n, pp, x2)?;
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
            return Ok(());
        }
    }
    if vm.i32(H4) < 0 {
        return Ok(());
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
    let ease = crate::engine::signed_pow(vm.f64(H8) as f32, 5.0);
    for (address, target) in targets {
        let value = interpolate_clamped(0.0, vm.f64(address) as f32, 1.0, target as f32, ease);
        vm.set_f64(address, f64::from(value));
    }
    Ok(())
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
pub fn gear_targets(vm: &mut Vm, _env: &mut dyn Callees, f: u64, rbp: u64) -> Result<(), String> {
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
            let scale = crate::callees::blend(vm, f + 0xbdd8, vm.i32(b + 0xe78));
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
        let pull =
            |_vm: &mut Vm, _env: &mut dyn Callees, x: f32| crate::engine::signed_pow(x, b2840);
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
            let floor = crate::callees::blend(vm, memory, vm.i32(owner + 0xe7c));
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
        let point = [0x280c, 0x2810, 0x2814].map(|o| f64::from(vm.f32(b + o)));
        world_point_f64(vm, env, f, point, [slot(-0x38), slot(-0x68), slot(0x1758)]);
        let ctx = env.call(vm, 0x14193ae40, CallArgs::default()).rax;
        let local = [slot(-0x38), slot(-0x68), slot(0x1758)].map(|a| vm.f64(a));
        local_to_geodetic(vm, env, ctx, [f + 0x64e8, f + 0x64f0, f + 0x64f8], local);
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

/// `0x14126d5d3..0x14126dd98`: the wing-strip contact near the ground. Returns `false` when the aircraft is too high
/// (`F+0x42ec8 <= altitude - |B+0x64f4..0x64fc|`; the original then skips to `0x14126ef6b`). Otherwise, for each of the
/// 48 wings whose binding `0x251` (queried with the wing index, replayed) is clear and whose enabled byte is set, the
/// unit span direction from the boundary arrays `+0x5bc/0x5e8/0x614` between the root and the tip, and the probe
/// distance `F+0x42f50` give three points that are handed to the strip contact function `0x1411c7a50` (replayed) with
/// the wing's accumulators `X+0x2d0/0x2d4`. The settings of the original are `B+0x2898 * 9.798 / (0.15 * B+0x64ac)`
/// (`rbp-0x10`) and a tenth of it (`rbp-0x8`).
#[allow(clippy::field_reassign_with_default)]
pub fn wing_ground_probe(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) -> bool {
    use crate::wing_element::boundary_at;
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    let sqrt = |v: f32| if 0.0 > v { f32::NAN } else { v.sqrt() };
    vm.set_i32(slot(-0x78), 0);
    let altitude = position_component(vm, env, f, 0x380);
    let b = vm.u64(f + 0x20);
    let (w0, w1, w2) = (vm.f32(b + 0x64f4), vm.f32(b + 0x64f8), vm.f32(b + 0x64fc));
    let wind = sqrt(w0 * w0 + w1 * w1 + w2 * w2);
    let limit = f64::from(vm.f32(f + 0x42ec8));
    let height = altitude - f64::from(wind);
    if limit.partial_cmp(&height) != Some(std::cmp::Ordering::Greater) {
        return false;
    }
    let x1 = (f64::from(vm.f32(b + 0x64ac)) * 0.15) as f32;
    let drag = vm.f32(b + 0x2898) * f32::from_bits(0x411c_c5c1) / x1;
    vm.set_f32(slot(-0x10), drag);
    let tenth = (f64::from(drag) * 0.1) as f32;
    vm.set_f32(slot(-0x8), tenth);
    vm.set_i32(slot(0x1750), 0);
    for i in 0..48u64 {
        let mut args = CallArgs::ints(&[f, 1, 0x251, i]);
        args.stack[0] = Some(u64::from(1.0f32.to_bits()));
        if env.call(vm, 0x1407ace10, args).rax as u32 != 0 {
            continue;
        }
        let b = vm.u64(f + 0x20);
        let wing = vm.u64(b + 0x6028) + i * 0x36c8;
        if vm.u8(wing + 0x678) == 0 {
            continue;
        }
        let xrec = vm.u64(f + 0x6940) + i * 0x2d8;
        let n = vm.i32(wing + 4);
        let read = |vm: &Vm, base: u64| -> Vec<f32> {
            (0..=n.max(0) as u64)
                .map(|k| vm.f32(wing + base + 4 * k))
                .collect()
        };
        let (ax, ay, az) = (read(vm, 0x5bc), read(vm, 0x5e8), read(vm, 0x614));
        let nf = n as f32;
        let d9 = boundary_at(&ax, n, nf) - boundary_at(&ax, n, 0.0);
        let d8 = boundary_at(&ay, n, nf) - boundary_at(&ay, n, 0.0);
        let d11 = boundary_at(&az, n, nf) - boundary_at(&az, n, 0.0);
        let len = sqrt(d9 * d9 + d8 * d8 + d11 * d11);
        let (d9, d8, d11) = (d9 / len, d8 / len, d11 / len);
        let reach = vm.f32(f + 0x42f50);
        let (drag, tenth) = (vm.f32(slot(-0x10)), vm.f32(slot(-0x8)));
        let ptrs = [Some(xrec + 0x2d0), Some(xrec + 0x2d4)];
        let call = |vm: &mut Vm, env: &mut dyn Callees, a: f32, b3: f32, s: [f32; 4]| {
            let mut args = CallArgs::default();
            args.int = [Some(f), Some(0xffff_ffff), None, None];
            args.xmm = [None, None, Some(a.to_bits()), Some(b3.to_bits())];
            args.stack = s.map(|v| Some(u64::from(v.to_bits())));
            let _ = ptrs;
            let _ = tenth;
            env.call(vm, 0x1411c7a50, args);
        };
        // first point
        let tz = boundary_at(&az, n, nf);
        let ty = boundary_at(&ay, n, nf);
        let tx = boundary_at(&ax, n, nf);
        let m_reach = -reach;
        call(vm, env, tx, m_reach * d9, [ty, m_reach * d8, drag, tz]);
        // second point
        let tz = boundary_at(&az, n, nf);
        let ty = boundary_at(&ay, n, nf);
        let tx = boundary_at(&ax, n, nf);
        call(vm, env, tx, m_reach * d8, [ty, reach * d9, drag, tz]);
        // third point
        let tz = boundary_at(&az, n, nf);
        let ty = boundary_at(&ay, n, nf);
        let tx = boundary_at(&ax, n, nf);
        call(vm, env, tx, d8 * reach, [ty, m_reach * d9, drag, tz]);
        let _ = (tenth, d11);
    }
    vm.set_i32(slot(0x1750), 48);
    true
}

/// `0x14126de36..0x14126e4c8`: the contact points of one body's surface. For a body whose `+0x5f4` is -1 and whose
/// last row sits above `0.05` of the wind height `B+0x64fc`, every point of every row of the body (`+0x660 + 0xd8 a`,
/// `+0x658` points of three floats) is moved by the body's Euler angles and offset and tested against the ground with
/// `0x1411c7a50`: the direction is the point's offset from the row's centroid scaled to half the body height
/// (`+0xc`; a point at the centroid uses the height above the body's middle `(+0x24 + +0x28) / 2` instead). A nonzero
/// answer raises `F+0x24c`. The body's `+0x2c` is cleared first. The drag and its tenth come from the frame slots
/// `rbp-0x10` and `rbp-0x8`.
pub fn body_surface_probe(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64, s: u64) {
    use crate::transform::rotate_pairs;
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    let slot = |o: i64| (rbp as i64 + o) as u64;
    vm.set_i32(s + 0x2c, 0);
    if vm.i32(s + 0x5f4) != -1 {
        return;
    }
    let rows = vm.i32(s + 0x654);
    let last = (s as i64 + 0x58c + i64::from(rows) * 0xd8) as u64;
    let rise = f64::from(vm.f32(last) - vm.f32(s + 0x664));
    let b = vm.u64(f + 0x20);
    let limit = f64::from(vm.f32(b + 0x64fc)) * 0.05;
    if rise.partial_cmp(&limit) != Some(std::cmp::Ordering::Greater) {
        return;
    }
    let middle = (f64::from(vm.f32(s + 0x28) + vm.f32(s + 0x24)) * 0.5) as f32;
    let half = (f64::from(vm.f32(s + 0xc)) * 0.5) as f32;
    if rows <= 0 {
        return;
    }
    let (drag, tenth) = (vm.f32(slot(-0x10)), vm.f32(slot(-0x8)));
    let _ = tenth;
    let g = s + 0x588;
    let angle = |o: u64| vm.f32(g + o) * RAD;
    let (r9c, ra0, ra4) = (angle(0x9c), angle(0xa0), angle(0xa4));
    let p = [
        r9c.sin(),
        r9c.cos(),
        ra0.sin(),
        ra0.cos(),
        ra4.sin(),
        ra4.cos(),
    ];
    let offsets = [vm.f32(g + 0x90), vm.f32(g + 0x94), vm.f32(g + 0x98)];
    let count = vm.i32(s + 0x658);
    for a in 0..rows as u64 {
        let row = s + 0x660 + a * 0xd8;
        let point = |vm: &Vm, m: u64| {
            let at = row - 4 + 12 * m;
            [vm.f32(at), vm.f32(at + 4), vm.f32(at + 8)]
        };
        if count <= 0 {
            continue;
        }
        let nf = count as f32;
        let mut c = [0.0f32; 3];
        for m in 0..count as u64 {
            let q = point(vm, m);
            for k in 0..3 {
                c[k] += q[k] / nf;
            }
        }
        for m in 0..count as u64 {
            let [x, y, z] = point(vm, m);
            let mut at = rotate_pairs(x, y, z, p);
            for k in 0..3 {
                at[k] += offsets[k];
            }
            let (dx, dy, dz) = (x - c[0], y - c[1], z - c[2]);
            let near = dy * dy + dx * dx;
            let len = (dz * dz + near).sqrt();
            let (ux, uy, uz) = if f64::from(len) > 0.01 {
                (-dx * half / len, -dy * half / len, -dz * half / len)
            } else {
                let dz2 = z - middle;
                let len2 = (dz2 * dz2 + near).sqrt();
                (-dx * half / len2, -dy * half / len2, -dz2 * half / len2)
            };
            let out = rotate_pairs(ux, uy, uz, p);
            let args = CallArgs {
                int: [Some(f), Some(0xffff_ffff), None, None],
                xmm: [None, None, Some(at[0].to_bits()), Some(out[0].to_bits())],
                stack: [at[1], out[1], drag, at[2]].map(|v| Some(u64::from(v.to_bits()))),
            };
            if env.call(vm, 0x1411c7a50, args).rax as u32 != 0 {
                vm.set_i32(f + 0x24c, 1);
            }
        }
    }
}

fn unit_clamp(v: f32) -> f32 {
    clamp(v, 0.0, 1.0)
}

/// `(1 - c) * old + c * new` with the blend factor `c = clamp(2 * frame time)` taken from a fresh time query.
fn blend_to(vm: &mut Vm, env: &mut dyn Callees, at: u64, new: f32) {
    let t = frame_time(vm, env);
    let c = unit_clamp((t + t) as f32);
    let kept = (1.0 - c) * vm.f32(at);
    vm.set_f32(at, kept + c * new);
}

/// `0x14126e4c8..0x14126e974`: the contact response of body `s` (index `j`, from the frame slot `rbp+0x1750`). The
/// callee `0x1411cb1e0` is asked for every pair `(edi, r12)` of the rows (`edi < +0x654 - 1`, `r12` from a quarter
/// to three quarters of the point count `+0x658`) and reports a contact through its return value and five float
/// outputs; a contact raises `F+0x24c`, `F+0x250`, `rbp-0x78` and the body's `+0x2c`, and tracks the contact with the
/// lowest and with the highest third output. When the body has a contact (`+0x2c`) its smoothed values `+0x30..+0x50`
/// are blended toward the new ones with `clamp(2 * frame time)`, otherwise `+0x30..+0x38` are cleared.
pub fn body_contact_blend(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64, s: u64) {
    let slot = |o: i64| (rbp as i64 + o) as u64;
    let g = s + 0x588;
    let r14 = s + 0x630;
    let j = vm.u32(slot(0x1750));
    let count = vm.i32(r14 + 0x28);
    let mut acc_a = 0.0f32; // xmm12, from rbp-0x1c
    let mut acc_b = 0.0f32; // xmm15, from rbp-0x7c
    let mut low = [0.0f32; 3]; // xmm8, xmm9, xmm6
    let mut high = [0.0f32; 3]; // xmm10, xmm11, xmm7
    vm.set_f32(slot(-0x1c), 0.0);
    vm.set_f32(slot(-0x6c), 0.0);
    vm.set_f32(slot(-0x7c), 0.0);
    let mut r12 = (f64::from(count) * 0.25) as i32;
    let reach = f64::from(count) * 0.75;
    if reach >= f64::from(r12) {
        loop {
            let mut edi = 0i32;
            while edi < vm.i32(r14 + 0x24) - 1 {
                vm.set_f32(slot(-0x6c), 0.0);
                vm.set_f32(slot(-0x30), 0.0);
                vm.set_f32(slot(-0x18), 0.0);
                let args = CallArgs {
                    int: [Some(f), Some(s), Some(g), Some(r14)],
                    xmm: [None; 4],
                    stack: [
                        u64::from(j),
                        edi as u32 as u64,
                        r12 as u32 as u64,
                        slot(-0x6c),
                    ]
                    .map(Some),
                };
                if env.call(vm, 0x1411cb1e0, args).rax as u32 != 0 {
                    vm.set_i32(f + 0x24c, 1);
                    vm.set_i32(f + 0x250, 1);
                    vm.set_i32(slot(-0x78), 1);
                    if vm.i32(s + 0x2c) == 0 {
                        vm.set_i32(s + 0x2c, 1);
                    }
                    let (o1, o2, o3) = (
                        vm.f32(slot(-0x6c)),
                        vm.f32(slot(-0x30)),
                        vm.f32(slot(-0x18)),
                    );
                    if low[2] > o3 {
                        low = [o1, o2, o3];
                    }
                    if o3 > high[2] {
                        high = [o1, o2, o3];
                    }
                }
                edi += 1;
            }
            r12 += 1;
            if reach < f64::from(r12) {
                break;
            }
        }
        acc_a = vm.f32(slot(-0x1c));
        acc_b = vm.f32(slot(-0x7c));
        vm.set_f32(slot(-0x6c), acc_b);
    }
    if vm.i32(s + 0x2c) == 0 {
        vm.set_u32(s + 0x30, 0);
        vm.set_u32(s + 0x34, 0);
        vm.set_u32(s + 0x38, 0);
        return;
    }
    blend_to(vm, env, s + 0x30, acc_a);
    blend_to(vm, env, s + 0x34, acc_b);
    let t = frame_time(vm, env);
    let t3 = (t + t) as f32;
    let (vx, vy) = (vm.f32(f + 0x368), vm.f32(f + 0x370));
    let speed = (vx * vx + vy * vy).sqrt();
    let w = unit_clamp((speed - 1.0) * f32::from_bits(0x3de3_8e39) + 0.0);
    let m = if acc_a > 0.0 { acc_a } else { 0.0 };
    let v = vm.f32(slot(-0x6c));
    let den = if v > f32::from_bits(0x3c23_d70a) {
        v
    } else {
        f32::from_bits(0x3c23_d70a)
    };
    let scaled = m / den * (w * w);
    let c3 = unit_clamp(t3);
    let kept = (1.0 - c3) * vm.f32(s + 0x38);
    vm.set_f32(s + 0x38, kept + c3 * scaled);
    for (offset, value) in [
        (0x3c, low[0]),
        (0x40, low[1]),
        (0x44, low[2]),
        (0x48, high[0]),
        (0x4c, high[1]),
        (0x50, high[2]),
    ] {
        blend_to(vm, env, s + offset, value);
    }
}

/// `0x14126dd98..0x14126e9a0`: the contact pass over the 39 bodies (`[B+0x6040]`, stride `0x34c8`). A body is
/// skipped when its model index `+0x5f0` (up to 0x26) is bound by `0x1407ace10(F, 1, 0x179, index)`, when `+0x54` is
/// nonzero or when its byte `+0x588` is clear; the others run [`body_surface_probe`] and [`body_contact_blend`]. The
/// body counter lives in the frame slot `rbp+0x1750`.
pub fn body_contact_loop(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) {
    let slot = |o: i64| (rbp as i64 + o) as u64;
    for j in 0..39u64 {
        vm.set_u32(slot(0x1750), j as u32);
        let bodies = vm.u64(vm.u64(f + 0x20) + 0x6040);
        let at = bodies + j * 0x34c8;
        let index = vm.u32(at + 0x5f0);
        if index <= 0x26 {
            let args = CallArgs::ints(&[f, 1, 0x179, u64::from(index)]);
            if env.call(vm, 0x1407ace10, args).rax as u32 != 0 {
                continue;
            }
        }
        if vm.i32(at + 0x54) != 0 {
            continue;
        }
        let bodies = vm.u64(vm.u64(f + 0x20) + 0x6040);
        if vm.u8(bodies + j * 0x34c8 + 0x588) == 0 {
            continue;
        }
        body_surface_probe(vm, env, f, rbp, at);
        body_contact_blend(vm, env, f, rbp, at);
    }
    vm.set_u32(slot(0x1750), 39);
}

/// `0x14126e9a0..0x14126ef74`: the ground forces after a body contact (`rbp-0x78` set). With the weights `B+0x2804`
/// and `B+0x2808` positive, the heading of the ground velocity relative to the aircraft (the point
/// `(-B+0x2800 * F+0x3d4, -B+0x2800 * F+0x3d0, 0)` moved out of the aircraft frame, added to `F+0x368/0x370`, through
/// `atan2`) gives a side force `clamp(f(speed) * speed * sin(angle) * |sin(angle)| ...)` applied at the contact point
/// with `0x1408e3230`; with the position `F+0x64e4` set, the drag toward the stored geographic point (converted to
/// the world through `0x140913e60`, `0x140816eb0` and `0x141296750`) is applied as a second force and a moment
/// (`0x14119e5b0`). Then, with `B+0x28dc` set and `F+0x64dc > 0.01`, a brake force is applied and the brake energy
/// `F+0x28c` grows by `frame time * ...` up to `B+0x2890`.
#[allow(clippy::field_reassign_with_default)]
pub fn ground_response(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) -> Result<(), String> {
    use crate::transform::{Frame, from_aircraft_frame};
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    const DEG: f32 = f32::from_bits(0x4265_2ee0);
    let slot = |o: i64| (rbp as i64 + o) as u64;
    let bits = |v: f32| u64::from(v.to_bits());
    if vm.i32(slot(-0x78)) == 0 {
        return Ok(());
    }
    let b = vm.u64(f + 0x20);
    if vm.f32(b + 0x2804) > 0.0 && vm.f32(b + 0x2808) > 0.0 {
        let m = -vm.f32(b + 0x2800);
        let (a, c3) = (m * vm.f32(f + 0x3d4), m * vm.f32(f + 0x3d0));
        let frame = Frame {
            origin: [0.0; 3],
            rotation: [
                [vm.f32(f + 0x430), vm.f32(f + 0x434)],
                [vm.f32(f + 0x440), vm.f32(f + 0x444)],
                [vm.f32(f + 0x450), vm.f32(f + 0x454)],
            ],
        };
        let w = from_aircraft_frame(&frame, [a, c3, 0.0], false, false);
        vm.set_f32(slot(0x1758), w[0]);
        vm.set_f32(slot(-0x58), w[1]);
        vm.set_f32(slot(0x1750), w[2]);
        let y = -(w[2] + vm.f32(f + 0x370));
        let x = w[0] + vm.f32(f + 0x368);
        let bearing = x.atan2(y) * DEG;
        let mut h = vm.f32(f + 0x3e0) - bearing;
        while -180.0 > h {
            h += 360.0;
        }
        while h > 180.0 {
            h += -360.0;
        }
        let angle = (vm.f32(f + 0x64cc) - h) * RAD;
        let sine = angle.sin();
        let (vx, vy, vz) = (vm.f32(f + 0x368), vm.f32(f + 0x36c), vm.f32(f + 0x370));
        let speed = ((vx * vx + vy * vy) + vz * vz).sqrt();
        let b = vm.u64(f + 0x20);
        let side = sine.abs() * speed * vm.f32(b + 0x2804) * 1000.0;
        let knots = speed * f32::from_bits(0x3ff8_cfe5);
        let factor = interpolate_clamped(0.0, 1.0, vm.f32(b + 0x7b0), 0.0, knots);
        let moment = factor * (speed * sine) * side * vm.f32(f + 0x64c8);
        let limit = vm.f32(b + 0x288c);
        let moment = clamp(moment, -limit, limit);
        let pp = vm.f32(b + 0x2800);
        add_plugin_force(vm, env, f, 0.0, -moment, 0.0, 0.0, pp, 0.0)?;
        if vm.i32(f + 0x64e4) != 0 {
            let planet = env.call(vm, 0x14193ae40, CallArgs::default()).rax;
            let geographic = [0x64e8, 0x64f0, 0x64f8].map(|o| vm.f64(f + o));
            geodetic_to_local(
                vm,
                env,
                planet,
                [slot(-0x38), rbp, slot(0x1750)],
                geographic,
            );
            let b = vm.u64(f + 0x20);
            let point = [0x280c, 0x2810, 0x2814].map(|o| f64::from(vm.f32(b + o)));
            world_point_f64(vm, env, f, point, [slot(-0x50), slot(-0x68), slot(0x1758)]);
            let d1 = (vm.f64(slot(0x1750)) - vm.f64(slot(0x1758))) as f32;
            let d2 = (vm.f64(rbp) - vm.f64(slot(-0x68))) as f32;
            let d3 = (vm.f64(slot(-0x38)) - vm.f64(slot(-0x50))) as f32;
            let frame = world_frame(vm, f);
            let [o1, o2, o3] =
                crate::transform::to_aircraft_frame(&frame, [d3, d2, d1], false, false);
            vm.set_f32(slot(0x1750), o1);
            vm.set_f32(slot(0x1758), o2);
            vm.set_f32(slot(-0x58), o3);
            let (x7, x8, x9) = (
                vm.f32(slot(0x1750)),
                vm.f32(slot(0x1758)),
                vm.f32(slot(-0x58)),
            );
            let len = ((x7 * x7 + x8 * x8) + x9 * x9).sqrt();
            let len = if len > f32::from_bits(0x3c23_d70a) {
                len
            } else {
                f32::from_bits(0x3c23_d70a)
            };
            let b = vm.u64(f + 0x20);
            let weight = vm.f32(b + 0x2898) * f32::from_bits(0x411c_c5c1);
            let half = (f64::from(weight) * 0.1) as f32;
            let k = interpolate_clamped(0.0, 0.0, 25.0, half, len);
            let (p9, p8, p7) = (k * x9 / len, k * x8 / len, k * x7 / len);
            let (pa, pc, pp) = (vm.f32(b + 0x280c), vm.f32(b + 0x2810), vm.f32(b + 0x2814));
            add_plugin_force(vm, env, f, pa, p7, pc, p8, pp, p9)?;
            let b = vm.u64(f + 0x20);
            let reach = (f64::from(vm.f32(b + 0x64ac)) * 0.1) as f32;
            let mut args = CallArgs::ints(&[f]);
            args.xmm = [
                None,
                Some(vm.f32(b + 0x280c).to_bits()),
                Some(vm.f32(b + 0x2810).to_bits()),
                Some(vm.f32(b + 0x2814).to_bits()),
            ];
            args.stack = [Some(bits(reach * reach)), None, None, None];
            env.call(vm, 0x14119e5b0, args);
        }
    }
    let b = vm.u64(f + 0x20);
    let slip = vm.f32(f + 0x64dc);
    if vm.i32(b + 0x28dc) != 0 && f64::from(slip) > 0.01 {
        let scale = vm.f32(b + 0x2890) / 10.0 * slip;
        let (vx, vy, vz) = (vm.f32(f + 0x368), vm.f32(f + 0x36c), vm.f32(f + 0x370));
        let speed = ((vx * vx + vy * vy) + vz * vz).sqrt();
        let force = (f64::from(speed * scale) * 1.25) as f32;
        let (pa, pc, pp) = (vm.f32(b + 0x28bc), vm.f32(b + 0x28c0), vm.f32(b + 0x28c4));
        add_plugin_force(vm, env, f, pa, 0.0, pc, 0.0, pp, force)?;
        let b = vm.u64(f + 0x20);
        let capacity = vm.f32(b + 0x2890);
        if capacity.partial_cmp(&vm.f32(f + 0x28c)) == Some(std::cmp::Ordering::Greater) {
            let t = frame_time(vm, env);
            let grown = (f64::from(vm.f32(f + 0x28c)) + t * f64::from(scale)) as f32;
            let capacity = vm.f32(vm.u64(f + 0x20) + 0x2890);
            let value = if 0.0 > grown {
                0.0
            } else if capacity < grown {
                capacity
            } else {
                grown
            };
            vm.set_f32(f + 0x28c, value);
        }
    }
    Ok(())
}

/// `0x14126ef74..0x14126f37f`: the gear drag and the brake energy. Every gear record whose state (`[B+0x6080] + 0x88 i`)
/// is above 1 and whose animation entry (`[F+0x6958] + 0x90 i`) has an extension `+0x10 > 0` places the foot of the
/// strut (the object at the entry's `+0` pointer: the strut length `+0x18 - +0x20 * (1 - extension)` rotated by the
/// entry's two angles `+0x14`, `+0x18` about the mount `+0x64/+0x70/+0x7c`) and applies a drag with `0x14119e5b0`,
/// from the tyre widths `+0x58`, `+0x5c` and the stroke `+0x18`. With `B+0x2890 > 0.01`, a brake (`B+0x28dc` or
/// `B+0x28e0`) and the brakes on `F+0x64d8`, the brake energy `F+0x28c` drains at the rate `F+0x290 = B+0x2890 *
/// scale / max(B+0x281c, 1)` (scale from two `0x1406ea0b0` lines of `F+0x28c / B+0x2890`) and the flag clears when
/// it is empty; otherwise the flag and rate are cleared. Finally `0x1411ddfd0` (replayed) is asked about the
/// aircraft's wheel state and the contact flag `rbp-0x78` is cleared first.
pub fn gear_drag_and_brake(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    rbp: u64,
) -> Result<(), String> {
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    let slot = |o: i64| (rbp as i64 + o) as u64;
    for i in 0..10u64 {
        let g = vm.u64(vm.u64(f + 0x20) + 0x6080) + i * 0x88;
        if vm.u32(g) <= 1 {
            continue;
        }
        let begin = vm.u64(f + 0x6958);
        let count = (vm.u64(f + 0x6960).wrapping_sub(begin) as i64) / 0x90;
        if i >= count as u64 {
            return Err("gear state index out of range".into());
        }
        let elem = begin + i * 0x90;
        let e = vm.f32(elem + 0x10);
        if e.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            continue;
        }
        let strut = vm.u64(elem);
        let ed = f64::from(e);
        let pitch = vm.f32(elem + 0x14) * RAD;
        let roll = vm.f32(elem + 0x18) * RAD;
        let r = f64::from(vm.f32(strut + 0x18)) - f64::from(vm.f32(strut + 0x20)) * (1.0 - ed);
        let cos_roll = f64::from(roll.cos());
        let x = (f64::from(pitch.sin()) * r * cos_roll + f64::from(vm.f32(strut + 0x64))) as f32;
        let y = (f64::from(vm.f32(strut + 0x70)) - f64::from(pitch.cos()) * r * cos_roll) as f32;
        let z = (f64::from(vm.f32(strut + 0x7c)) - f64::from(roll.sin()) * r) as f32;
        let wide = f64::from(vm.f32(g + 0x58));
        let narrow = vm.f32(g + 0x5c);
        let t1 = (((wide + wide) * f64::from(narrow)) * 2.0 * ed) as f32;
        let mut t2 = (wide * 0.2) as f32;
        if t2 > narrow {
            t2 = narrow;
        }
        let t2 = t2 * vm.f32(g + 0x18);
        let t2 = ((f64::from(t2) + f64::from(t2)) * ed) as f32;
        let mut args = CallArgs::ints(&[f]);
        args.xmm = [
            None,
            Some(x.to_bits()),
            Some(y.to_bits()),
            Some(z.to_bits()),
        ];
        args.stack = [Some(u64::from((t2 + t1).to_bits())), None, None, None];
        env.call(vm, 0x14119e5b0, args);
    }
    let b = vm.u64(f + 0x20);
    let capacity = vm.f32(b + 0x2890);
    let braking = f64::from(capacity) > 0.01
        && (vm.i32(b + 0x28dc) != 0 || vm.i32(b + 0x28e0) != 0)
        && vm.i32(f + 0x64d8) != 0;
    if braking {
        let energy = vm.f32(f + 0x28c);
        let ratio = energy / capacity;
        let low = interpolate_clamped(0.0, f32::from_bits(0x3727_c5ac), 0.1, 1.0, ratio);
        let high = interpolate_clamped(1.0, 0.1, f32::from_bits(0x3f73_3333), 1.0, ratio);
        let scale = low * high;
        let divisor = vm.f32(b + 0x281c);
        let divisor = if divisor > 1.0 { divisor } else { 1.0 };
        vm.set_f32(f + 0x290, capacity * scale / divisor);
        let t = frame_time(vm, env);
        let left = (f64::from(energy) - t * f64::from(vm.f32(f + 0x290))) as f32;
        let capacity = vm.f32(vm.u64(f + 0x20) + 0x2890);
        let value = if 0.0 > left {
            0.0
        } else if capacity < left {
            capacity
        } else {
            left
        };
        vm.set_f32(f + 0x28c, value);
        if value <= 0.0 {
            vm.set_u32(f + 0x64d8, 0);
        }
    } else {
        vm.set_u32(f + 0x64d8, 0);
        vm.set_u32(f + 0x290, 0);
    }
    if vm.i32(f + 0xbcd0) != 0 || vm.i32(f + 0xbcc8) != 0 {
        return Err("debug dump not ported".into());
    }
    vm.set_u32(slot(0x1758), 0);
    vm.set_u32(slot(0x1750), 0);
    vm.set_u32(slot(-0x78), 0);
    let mut args = CallArgs::ints(&[f, f + 0x288, f + 0x304, slot(0x1758)]);
    args.xmm[2] = Some(vm.f32(f + 0x34c).to_bits());
    args.stack = [
        Some(f + 0x304),
        Some(u64::from(vm.f32(f + 0x354).to_bits())),
        Some(slot(0x1750)),
        Some(f + 0x31c),
    ];
    env.call(vm, 0x1411ddfd0, args);
    Ok(())
}

/// `0x141265731..0x14126580a`: the start of the flight step. The log switch `0x14120c960(F+0xbcc8)` is queried (a
/// nonzero answer only writes a log line), and unless `F+0x675c` is set the six force totals are cleared; the
/// per-source force tables (`F+0x2bc..0x338` apart from the totals) and `F+0x294` are cleared; `0x1409830b0` is
/// called on `F+0x430d8` and `0x1412763c0` on the object (both replayed).
pub fn step_reset(vm: &mut Vm, env: &mut dyn Callees, f: u64) {
    // the log switch `0x14120c960` only decides whether a log line is written
    if vm.i32(f + 0x675c) == 0 {
        for offset in [0x30c, 0x2cc, 0x324, 0x2e0, 0x33c, 0x2f4] {
            vm.set_u32(f + offset, 0);
        }
    }
    for offset in [
        0x2c0, 0x2c4, 0x2bc, 0x2d4, 0x2d8, 0x2d0, 0x2e8, 0x2ec, 0x2e4, 0x300, 0x304, 0x2f8, 0x2fc,
        0x318, 0x31c, 0x310, 0x314, 0x330, 0x334, 0x328, 0x32c, 0x294,
    ] {
        vm.set_u32(f + offset, 0);
    }
    // `0x1409830b0`: the vector at `F+0x430d8` is cleared (its end returns to its begin)
    let begin = vm.u64(f + 0x430d8);
    if begin != vm.u64(f + 0x430e0) {
        vm.set_u64(f + 0x430e0, begin);
    }
    env.call(vm, 0x1412763c0, CallArgs::ints(&[f]));
}

/// `0x141265810..0x1412659b9`: with `F+0x28 == 0`, the ids that `0x1411bbfb0(F+0xb8a0, 0, 0, 1, 0, 0, 0, 0, 0, &ids)`
/// collects (a vector of 32-bit ids at `rbp+0x60`) are scanned in order: for each, the object `0x1407d6e70(F, id)`
/// is resolved through `0x1411b9460` (a temporary at `rbp+0x80`, released with `0x1407bfac0`), and when its record
/// has `+0x36e8 == 2` the same is done again to test `+0x36ec == 2`. The first id that passes is handed to
/// `0x140f39780` and, unless `F+0xb9a8` is already set, recorded in `F+0xb9a8` (1), `F+0xb9ac` (the result of
/// `0x140f44180`) and `F+0xb9b0` (`+0xfc` of its object); `F+0xb9a8` (8 bytes) is cleared and `F+0xb9b0` set to 180.0
/// beforehand. The vector is released with `0x140601360`. At the end `0x1411b10a0(F+0xb8a0)` is called.
pub fn find_marked_source(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) {
    let slot = |o: i64| (rbp as i64 + o) as u64;
    if vm.i32(f + 0x28) == 0 {
        for k in 0..3 {
            vm.set_u64(slot(0x60 + 8 * k), 0);
        }
        let mut args = CallArgs::ints(&[f + 0xb8a0, 0, 0, 1]);
        args.stack = [Some(0); 4];
        env.call(vm, 0x1411bbfb0, args);
        vm.set_u64(f + 0xb9a8, 0);
        vm.set_u32(f + 0xb9b0, 0x4334_0000);
        let object = |vm: &mut Vm, env: &mut dyn Callees, id: u32| {
            env.call(vm, 0x1407d6e70, CallArgs::ints(&[f, u64::from(id)]))
                .rax
        };
        let mut index = 0u32;
        let count = |vm: &Vm| (vm.u64(slot(0x68)).wrapping_sub(vm.u64(slot(0x60)))) as i64 >> 2;
        if count(vm) != 0 {
            loop {
                let id = vm.u32(vm.u64(slot(0x60)) + 4 * u64::from(index));
                let x = object(vm, env, id);
                let first = env
                    .call(vm, 0x1411b9460, CallArgs::ints(&[x, slot(0x80)]))
                    .rax;
                let record = vm.u64(first);
                let mut found = false;
                let mut second = false;
                if vm.i32(record + 0x36e8) == 2 {
                    let x = object(vm, env, id);
                    let again = env
                        .call(vm, 0x1411b9460, CallArgs::ints(&[x, slot(0x78)]))
                        .rax;
                    second = true;
                    found = vm.i32(vm.u64(again) + 0x36ec) == 2;
                }
                if second {
                    env.call(vm, 0x1407bfac0, CallArgs::ints(&[slot(0x78)]));
                }
                env.call(vm, 0x1407bfac0, CallArgs::ints(&[slot(0x80)]));
                if found {
                    let x = object(vm, env, id);
                    env.call(vm, 0x140f39780, CallArgs::ints(&[x]));
                    if vm.i32(f + 0xb9a8) == 0 {
                        vm.set_u32(f + 0xb9a8, 1);
                        let x = object(vm, env, id);
                        let flag = env.call(vm, 0x140f44180, CallArgs::ints(&[x])).rax as u8;
                        vm.set_u32(f + 0xb9ac, u32::from(flag));
                        let x = object(vm, env, id);
                        let value = vm.u32(x + 0xfc);
                        vm.set_u32(f + 0xb9b0, value);
                    }
                    break;
                }
                index = index.wrapping_add(1);
                if i64::from(index as i32) as u64 >= count(vm) as u64 {
                    break;
                }
            }
        }
        env.call(vm, 0x140601360, CallArgs::ints(&[slot(0x60)]));
    }
    env.call(vm, 0x1411b10a0, CallArgs::ints(&[f + 0xb8a0]));
}

/// `0x1412659b9..0x141265de3`: the part strips. For each of the `B+0x920` parts (`[B+0x6010] + 0x88 + 0x3770 i`)
/// that `0x1411d9f60(F, i, 1)` selects, four vectors `v = 0x1411b4730(F, i, j)` (`j` < 4) of floats are set up: the
/// record pointer and `v` are put into two single-element vectors (`0x140985d90`, iterated with `0x1405f3f30`) and
/// handed to `0x14121a9b0(record, v, begin_a, begin_b, log)`; when the part's `+4` is set `v[0]` is doubled; then
/// `v[1 + k] = part[+0xa0] * v[0]` for the `+0x8c` entries. The vectors are released with `0x1405ddb90`. The log
/// switch `0x14120c960(F+0xbcc8)` is queried around it (a nonzero answer would log values: not ported).
pub fn part_strips(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) -> Result<(), String> {
    let slot = |o: i64| (rbp as i64 + o) as u64;
    let log = |vm: &mut Vm, _env: &mut dyn Callees| -> Result<(), String> {
        if crate::flight_step::debug_dump_active(vm, f) {
            return Err("log output not ported".into());
        }
        Ok(())
    };
    let mut i = 0u32;
    if vm.i32(vm.u64(f + 0x20) + 0x920) > 0 {
        loop {
            let selected = crate::controls::held_back(vm, env, f, i as i32, 1);
            if selected {
                let offset = i64::from(i as i32) as u64 * 0x3770;
                for j in 0..4u64 {
                    let base = vm.u64(vm.u64(f + 0x20) + 0x6010);
                    let record = base + 0x88 + offset;
                    vm.set_u64(slot(-0x68), record);
                    // `0x1411b4730(F, i, j)`: element `i` of the vector of records number `j`
                    let v = vm.u64(f + 0x68e0 + 24 * j) + i64::from(i as i32) as u64 * 0x2d8;
                    vm.set_u64(slot(-0x38), v);
                    for vector in [0x88, 0xa0] {
                        for k in 0..3 {
                            vm.set_u64(slot(vector + 8 * k), 0);
                        }
                    }
                    env.call(vm, 0x140985d90, CallArgs::ints(&[slot(0x88), slot(-0x68)]));
                    env.call(vm, 0x140985d90, CallArgs::ints(&[slot(0xa0), slot(-0x38)]));
                    let begin_b = env
                        .call(vm, 0x1405f3f30, CallArgs::ints(&[slot(0x3d8), slot(0xa0)]))
                        .rax;
                    let begin_a = env
                        .call(vm, 0x1405f3f30, CallArgs::ints(&[slot(0x3f0), slot(0x88)]))
                        .rax;
                    crate::flight_step::wing_chain_factor(vm, v, begin_a, begin_b);
                    let part = base + offset;
                    if vm.i32(part + 4) != 0 {
                        let value = vm.f32(v);
                        vm.set_f32(v, (f64::from(value) + f64::from(value)) as f32);
                        log(vm, env)?;
                    }
                    let mut k = 0u64;
                    while (k as i32) < vm.i32(part + 0x8c) {
                        let scaled = vm.f32(part + 0xa0) * vm.f32(v);
                        vm.set_f32(v + 4 + 4 * k, scaled);
                        log(vm, env)?;
                        k += 1;
                    }
                    log(vm, env)?;
                    env.call(vm, 0x1405ddb90, CallArgs::ints(&[slot(0xa0)]));
                    env.call(vm, 0x1405ddb90, CallArgs::ints(&[slot(0x88)]));
                }
            }
            i += 1;
            if (i as i32) >= vm.i32(vm.u64(f + 0x20) + 0x920) {
                break;
            }
        }
    }
    Ok(())
}

/// `0x141265de3..0x141265f7d`: the wing strips. For each of the 48 wings (`[B+0x6028] + 0x36c8 w`) that
/// `0x1411da150(F, w)` selects, the wing's id vector (`+0x3690`..`+0x3698`, 32-bit ids) is turned into two pointer
/// vectors: the wing records `[B+0x6028] + 0x36c8 id` (at `rbp+0xd0`) and the element records `[F+0x6940] + 0x2d8 id`
/// (at `rbp+0xb8`), pushed with `0x140985d90`; then `0x14121a9b0(wing, [F+0x6940] + 0x2d8 w, begin_wings,
/// begin_elements, log)` is called (iterators from `0x1405f3f30`) and the vectors are released with `0x1405ddb90`.
pub fn wing_strips(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) {
    let slot = |o: i64| (rbp as i64 + o) as u64;
    for w in 0..0x30u64 {
        if crate::flight_step::wing_enabled(vm, env, f, w as i32) == 0 {
            continue;
        }
        let b = vm.u64(f + 0x20);
        let wing = vm.u64(b + 0x6028) + 0x36c8 * w;
        let xrec = vm.u64(f + 0x6940) + 0x2d8 * w;
        vm.set_u64(slot(-0x38), xrec);
        for vector in [0xd0, 0xb8] {
            for k in 0..3 {
                vm.set_u64(slot(vector + 8 * k), 0);
            }
        }
        let count =
            |vm: &Vm| (vm.u64(wing + 0x3698).wrapping_sub(vm.u64(wing + 0x3690))) as i64 >> 2;
        if count(vm) != 0 {
            let mut index = 0u32;
            loop {
                let ids = vm.u64(wing + 0x3690);
                let id = i64::from(vm.i32(ids + 4 * u64::from(index)));
                let table = vm.u64(vm.u64(f + 0x20) + 0x6028);
                vm.set_u64(slot(-0x68), (id * 0x36c8) as u64 + table);
                env.call(vm, 0x140985d90, CallArgs::ints(&[slot(0xd0), slot(-0x68)]));
                let id = i64::from(vm.i32(vm.u64(wing + 0x3690) + 4 * u64::from(index)));
                let table = vm.u64(f + 0x6940);
                vm.set_u64(slot(-0x68), (id * 0x2d8) as u64 + table);
                env.call(vm, 0x140985d90, CallArgs::ints(&[slot(0xb8), slot(-0x68)]));
                index = index.wrapping_add(1);
                if i64::from(index as i32) as u64 >= count(vm) as u64 {
                    break;
                }
            }
        }
        let begin_elements = env
            .call(vm, 0x1405f3f30, CallArgs::ints(&[slot(0x408), slot(0xb8)]))
            .rax;
        let begin_wings = env
            .call(vm, 0x1405f3f30, CallArgs::ints(&[slot(0x420), slot(0xd0)]))
            .rax;
        crate::flight_step::wing_chain_factor(vm, xrec, begin_wings, begin_elements);
        env.call(vm, 0x1405ddb90, CallArgs::ints(&[slot(0xb8)]));
        env.call(vm, 0x1405ddb90, CallArgs::ints(&[slot(0xd0)]));
    }
}

/// `0x14127402d..0x141274060`: the end of the step. With `rdi = 0` and `rsi = 1` at this point, the first key
/// index `0..=0x12` that `0x1417b2e70(0x1460ad818, index)` reports (low byte) decides what is asked of `0x1407debf0`
/// with the aircraft's name field `F+0x2c`: none reported, it gets the mode `F+0x28`; reported while the global
/// `0x1460b83b0` is the integer kind (2), it gets the low byte the import behind `0x1424e5fc8` returns for the
/// global's value (`0x1460b83b4`). With the other kind (a string) the original copies the last four characters into
/// `F+0x2c` and does not call `0x1407debf0`; that path is not ported.
pub fn late_tail(vm: &mut Vm, env: &mut dyn Callees, f: u64) -> Result<(), String> {
    const KIND: u64 = 0x1_460b_83b0;
    let call_end = |vm: &mut Vm, env: &mut dyn Callees, first: u64| {
        let mut args = CallArgs::ints(&[first, f + 0x2c, 1, 0]);
        args.stack[0] = Some(0);
        env.call(vm, 0x1407debf0, args);
    };
    // `0x1417b2e70(table, index)`: the key state byte `table[index + 0xaf2f]`
    let reported = (0..=0x12u64).any(|index| vm.u8(0x1_460a_d818 + index + 0xaf2f) != 0);
    if !reported {
        let mode = u64::from(vm.u32(f + 0x28));
        call_end(vm, env, mode);
        return Ok(());
    }
    if vm.i32(KIND) != 2 {
        return Err("string path not ported".into());
    }
    let value = u64::from(vm.u32(KIND + 4));
    let byte = env.call(vm, 0x1_424e_5fc8, CallArgs::ints(&[value])).rax as u8;
    call_end(vm, env, u64::from(byte));
    Ok(())
}

/// `0x1408e3230(F, a, b, c, d, p, e, name)`: the plug-in force `(b, d, e)` at the point `(a, c, p)` (arguments in the
/// original's order: `xmm1 = a`, `xmm2 = b`, `xmm3 = c`, then the stack `d`, `p`, `e`). A non-finite `b`, `d` or `e`
/// counts as zero (the original logs it). The force is added at `F+0x2ec` (`b`), `F+0x2d8` (`d`) and `F+0x2c4`
/// (`e`), the moments at `F+0x300` (`b c - a d`), `F+0x318` (`c e - d p`) and `F+0x330` (`a e - b p`), and `F+0x340`
/// (the integral of `F+0x318` over the frame time) advances. The debug dump (`F+0xbcd0`/`F+0xbcc8`) is not ported.
#[allow(clippy::too_many_arguments)]
pub fn add_plugin_force(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    p: f32,
    e: f32,
) -> Result<(), String> {
    let finite = |v: f32| if v.is_finite() { v } else { 0.0 };
    let (b, d, e) = (finite(b), finite(d), finite(e));
    let add = |vm: &mut Vm, offset: u64, value: f32| {
        let sum = value + vm.f32(f + offset);
        vm.set_f32(f + offset, sum);
    };
    add(vm, 0x2c4, e);
    add(vm, 0x2ec, b);
    add(vm, 0x2d8, d);
    add(vm, 0x300, b * c - a * d);
    add(vm, 0x318, c * e - d * p);
    add(vm, 0x330, a * e - b * p);
    let t = frame_time(vm, env);
    let moment = f64::from(vm.f32(f + 0x318));
    let advanced = (f64::from(vm.f32(f + 0x340)) + t * moment) as f32;
    vm.set_f32(f + 0x340, advanced);
    if vm.i32(f + 0xbcd0) != 0 || vm.i32(f + 0xbcc8) != 0 {
        return Err("debug dump not ported".into());
    }
    Ok(())
}

/// `0x140816eb0(F, _, out1, _, a, b, c, out2, out3, 1)`: the point `(a, b, c)` (doubles) moved out of the aircraft
/// frame into the world: the rotation of `F+0x430..0x454` (double precision) and then the position doubles
/// `F+0x378/0x380/0x388` added (one engine-flag query each). The three results are stored through the pointers.
pub fn world_point_f64(vm: &mut Vm, env: &mut dyn Callees, f: u64, point: [f64; 3], out: [u64; 3]) {
    let word = |vm: &Vm, o: u64| f64::from(vm.f32(f + o));
    let pairs = [
        word(vm, 0x440),
        word(vm, 0x444),
        word(vm, 0x430),
        word(vm, 0x434),
        word(vm, 0x450),
        word(vm, 0x454),
    ];
    let rotated = crate::transform::rotate_pairs_f64(point[0], point[1], point[2], pairs);
    for k in 0..3 {
        vm.set_f64(out[k], rotated[k]);
    }
    for (k, offset) in [0x378u64, 0x380, 0x388].into_iter().enumerate() {
        let shift = position_component(vm, env, f, offset);
        let sum = shift + vm.f64(out[k]);
        vm.set_f64(out[k], sum);
    }
}

/// `0x140913e60(planet, out1, out2, out3, lat, lon, alt)` with `0x1419f8ea0`: the geographic point (degrees, degrees,
/// metres) becomes earth-centred coordinates and then the local coordinates `m * p + t` of the planet object (matrix
/// at `+0x280..+0x2d0`, translation at `+0x2e0..+0x2f0`). The earth-centred step uses the ellipsoid at `+0xb0`
/// (`a`, `b` and the eccentricity squared at `+0xb0`, `+0xb8`, `+0xc8`) and the height above the ellipsoid is the
/// altitude plus the geoid height `0x1419f8ff0(ellipsoid, lat, lon)` (replayed). The sine and cosine are the platform's.
pub fn geodetic_to_local(
    vm: &mut Vm,
    env: &mut dyn Callees,
    planet: u64,
    out: [u64; 3],
    geographic: [f64; 3],
) {
    let [x, y, z] = geodetic_to_ecef(vm, env, planet + 0xb0, geographic);
    let m = |vm: &Vm, o: u64| vm.f64(planet + o);
    let local = [
        ((y * m(vm, 0x2a0) + x * m(vm, 0x280)) + z * m(vm, 0x2c0)) + m(vm, 0x2e0),
        ((y * m(vm, 0x2a8) + x * m(vm, 0x288)) + z * m(vm, 0x2c8)) + m(vm, 0x2e8),
        ((y * m(vm, 0x2b0) + x * m(vm, 0x290)) + z * m(vm, 0x2d0)) + m(vm, 0x2f0),
    ];
    for k in 0..3 {
        vm.set_f64(out[k], local[k]);
    }
}

/// `0x1419f8ea0(ellipsoid, x_out, y_out, z_out, lat, lon, alt)`: earth-centred coordinates of a geographic point
/// (see [`geodetic_to_local`]); the geoid height `0x1419f8ff0(ellipsoid, lat, lon)` is replayed.
pub fn geodetic_to_ecef(
    vm: &mut Vm,
    env: &mut dyn Callees,
    shape: u64,
    geographic: [f64; 3],
) -> [f64; 3] {
    const RAD: f64 = f64::from_bits(0x3f91_df46_a252_9d39);
    let [lat, lon, alt] = geographic;
    let geoid = f64::from_bits(env.call(vm, 0x1419f8ff0, CallArgs::ints(&[shape])).xmm0);
    let height = geoid + alt;
    let (lat_r, lon_r) = (lat * RAD, lon * RAD);
    let (a, b, e2) = (vm.f64(shape), vm.f64(shape + 8), vm.f64(shape + 0x18));
    let sin_lat = lat_r.sin();
    let w = (1.0 - sin_lat * e2 * sin_lat).sqrt();
    let n = a / w;
    let rho = lat_r.cos() * (n + height);
    let x = lon_r.cos() * rho;
    let y = lon_r.sin() * rho;
    let z = sin_lat * (b * b * n / (a * a) + height);
    [x, y, z]
}

/// `0x1406eaf20(planet, lat_out, lon_out, alt_out, x, y, z)`: the local point is moved to earth-centred coordinates
/// with the inverse affine map of the planet object (matrix `+0x200..+0x250`, translation `+0x260..+0x270`) and
/// `0x1419f9330` turns them into latitude and longitude (degrees) and the altitude above the geoid (the geoid height
/// `0x1419f8ff0` is replayed). Closed form: with `rho = sqrt(x^2 + y^2)`, `r = sqrt(rho^2 + (1.0026 z)^2)`, `c = rho
/// / r`, `s = 1.0026 z / r`, the numerator `n = z + e'^2 b s^3` and denominator `d = rho - a e^2 c^3` give the sine
/// and cosine of the latitude; the height is `rho / cos - N` (or from `z / sin` near the poles, `|cos| < 0.3827`).
/// On the rotation axis (`x == 0`) the longitude is 90, -90 or 0 degrees and the latitude +-90.
pub fn local_to_geodetic(
    vm: &mut Vm,
    env: &mut dyn Callees,
    planet: u64,
    out: [u64; 3],
    local: [f64; 3],
) {
    const DEG: f64 = f64::from_bits(0x404c_a5dc_1a63_c1f8);
    const SPLIT: f64 = f64::from_bits(0x3fd8_7de2_a6ae_a963);
    let [lx, ly, lz] = local;
    let m = |vm: &Vm, o: u64| vm.f64(planet + o);
    let x = ((ly * m(vm, 0x220) + lx * m(vm, 0x200)) + lz * m(vm, 0x240)) + m(vm, 0x260);
    let y = ((ly * m(vm, 0x228) + lx * m(vm, 0x208)) + lz * m(vm, 0x248)) + m(vm, 0x268);
    let z = ((ly * m(vm, 0x230) + lx * m(vm, 0x210)) + lz * m(vm, 0x250)) + m(vm, 0x270);
    let shape = planet + 0xb0;
    let mut latitude_set = false;
    if x == 0.0 {
        if y > 0.0 {
            vm.set_f64(out[1], 90.0);
        } else if 0.0 > y {
            vm.set_f64(out[1], -90.0);
        } else {
            latitude_set = true;
            vm.set_f64(out[1], 0.0);
            if z > 0.0 {
                vm.set_f64(out[0], 90.0);
            } else if 0.0 > z {
                vm.set_f64(out[0], -90.0);
            } else {
                vm.set_f64(out[0], 90.0);
                return;
            }
        }
    } else {
        vm.set_f64(out[1], y.atan2(x) * DEG);
    }
    let (a, b, e2, e2p) = (
        vm.f64(shape),
        vm.f64(shape + 8),
        vm.f64(shape + 0x18),
        vm.f64(shape + 0x20),
    );
    let rho_sq = x * x + y * y;
    let rho = rho_sq.sqrt();
    let zz = z * 1.0026;
    let r = (zz * zz + rho_sq).sqrt();
    let (c, s) = (rho / r, zz / r);
    let s3 = s * s * s;
    let n = e2p * b * s3 + z;
    let d = rho - a * e2 * c * c * c;
    let norm = (d * d + n * n).sqrt();
    let (sin_p, cos_p) = (n / norm, d / norm);
    let radius = a / (1.0 - e2 * sin_p * sin_p).sqrt();
    let height = if cos_p >= SPLIT {
        rho / cos_p - radius
    } else if cos_p <= -SPLIT {
        rho / -cos_p - radius
    } else {
        (e2 - 1.0) * radius + z / sin_p
    };
    if !latitude_set {
        vm.set_f64(out[0], (sin_p / cos_p).atan() * DEG);
    }
    let geoid = f64::from_bits(env.call(vm, 0x1419f8ff0, CallArgs::ints(&[shape])).xmm0);
    vm.set_f64(out[2], height - geoid);
}

/// `0x1419f7ee0(planet, out, lat, lon)`: the local-axes matrix for a geographic point (degrees) as floats at `out`
/// (three rows of three, 16 bytes apart; the double 4 x 4 behind it is left at `out + 0x40`). `0x1419f8960` builds
/// the east-north-up frame of the point on the ellipsoid (the rotation rows `(-sin lon, cos lon, 0)`,
/// `(-sin lat cos lon, -sin lat sin lon, cos lat)`, `(cos lon cos lat, sin lon cos lat, sin lat)` and the earth-centred
/// origin of the point at altitude zero as translation); `0x1419f8af0` turns it by 90 degrees about the x axis and
/// inverts it; the planet's inverse matrix (`planet + 0x200`) is multiplied on and the result narrowed to floats.
pub fn local_axes(vm: &mut Vm, env: &mut dyn Callees, planet: u64, out: u64, lat: f64, lon: f64) {
    use crate::matrix::{identity, mul, read, rigid_inverse, rotate_by};
    const RAD: f64 = f64::from_bits(0x3f91_df46_a252_9d39);
    let origin = geodetic_to_ecef(vm, env, planet + 0xb0, [lat, lon, 0.0]);
    let r_lat = lat * RAD;
    let (sin_lat, cos_lat) = (r_lat.sin(), r_lat.cos());
    let r_lon = lon * RAD;
    let (sin_lon, cos_lon) = (r_lon.sin(), r_lon.cos());
    let neg_sin_lat = -sin_lat;
    let enu = [
        -sin_lon,
        cos_lon,
        0.0,
        0.0,
        neg_sin_lat * cos_lon,
        neg_sin_lat * sin_lon,
        cos_lat,
        0.0,
        cos_lon * cos_lat,
        sin_lon * cos_lat,
        sin_lat,
        0.0,
        origin[0],
        origin[1],
        origin[2],
        1.0,
    ];
    let turned = rotate_by(&identity(), 90.0, 1.0, 0.0, 0.0);
    let inverse = rigid_inverse(&mul(&enu, &turned));
    let planet_inverse = read(vm, planet + 0x200);
    let product = mul(&inverse, &planet_inverse);
    crate::matrix::write(vm, out + 0x40, &product);
    for (k, v) in product.iter().enumerate() {
        vm.set_f32(out + 4 * k as u64, *v as f32);
    }
}

/// `0x1406e2be0(planet, a1, b1, a2, b2, bearing_out, distance_out)`: the great-circle distance in metres between
/// two points given as (`a`, `b`) pairs of degrees, with `a` the latitude and `b` the longitude counted positive to
/// the west: the haversine `h = cos a1 cos a2 sin^2(db / 2) + sin^2(da / 2)` (the two squared sines in float32),
/// the angle `c = 2 asin(sqrt(h))`, the distance `c * 6378145`. Optionally the initial bearing in degrees (0..360,
/// float32) through the first pointer and the distance as a float32 through the second. For `c == 0` both stores
/// are zero and the result is 0; within `cos a1 < 0.01` of a pole the bearing is 180 (north) or 360 (south).
pub fn great_circle(
    vm: &mut Vm,
    a1: f64,
    b1: f64,
    a2: f64,
    b2: f64,
    bearing_out: u64,
    distance_out: u64,
) -> f64 {
    const RAD: f64 = f64::from_bits(0x3f91_df46_a252_9d39);
    const DEG: f64 = f64::from_bits(0x404c_a5dc_1a63_c1f8);
    const RADIUS: f64 = 6378145.0;
    // the 15-digit constants of the original, not the exact values
    const HALF_TURN: f64 = f64::from_bits(0x400921fb54442d11);
    const TURN: f64 = f64::from_bits(0x401921fb54442d11);
    let (lat1, lat2) = (a1 * RAD, a2 * RAD);
    let lon2 = b2 * -RAD;
    let lon1 = b1 * -RAD;
    let cos1 = lat1.cos();
    let s_lat = ((lat1 - lat2) * 0.5).sin() as f32;
    let s_lon = ((lon1 - lon2) * 0.5).sin() as f32;
    let h = (lat2.cos() * cos1) * f64::from(s_lon * s_lon) + f64::from(s_lat * s_lat);
    let c = 2.0 * h.sqrt().asin();
    if c == 0.0 {
        if bearing_out != 0 {
            vm.set_u32(bearing_out, 0);
        }
        if distance_out != 0 {
            vm.set_u32(distance_out, 0);
        }
        return 0.0;
    }
    let bearing = if 0.01 > cos1 {
        if lat1 > 0.0 { HALF_TURN } else { TURN }
    } else {
        let cos_c = c.cos();
        let sin_c = c.sin();
        let along = (lat2.sin() - lat1.sin() * cos_c) / (sin_c * cos1);
        let sin_dlon = (lon2 - lon1).sin();
        let clamped = along.clamp(-1.0, 1.0);
        if 0.0 > sin_dlon {
            clamped.acos()
        } else {
            TURN - clamped.acos()
        }
    };
    if bearing_out != 0 {
        let mut v = (bearing * DEG) as f32;
        while 0.0 > v {
            v += 360.0;
        }
        while v > 360.0 {
            v += -360.0;
        }
        vm.set_f32(bearing_out, v);
    }
    let distance = c * RADIUS;
    if distance_out != 0 {
        vm.set_f32(distance_out, distance as f32);
    }
    distance
}

/// `0x14076b5d0(table, a, b)`: the magnetic variation at latitude `a` and longitude `b` (degrees) from the grid of
/// floats at `table + 4` (37 rows of 72 columns, 5 degrees apart: row `r` holds the latitude `90 - 5 r`, column `c`
/// the longitude `-180 + 5 c`, the stored values have the opposite sign). The cell corners come from the latitude
/// rounded down to a multiple of 5 (limited to -90..90) and the longitude likewise (-180..175, the column after 71
/// wrapping to 0); the result is the bilinear interpolation (the latitude first, in float32).
pub fn magnetic_variation(vm: &Vm, table: u64, a: f32, b: f32) -> f32 {
    let floor5 = |x: f32| -> i32 {
        let t = (f64::from(x) / 5.0 - 0.5) as f32;
        let t = if 0.0 > t { t - 0.5 } else { t + 0.5 };
        (t as i32).wrapping_mul(5)
    };
    let lat = floor5(a).clamp(-90, 90);
    let lon = {
        let v = floor5(b);
        if v < -180 { -180 } else { v.min(175) }
    };
    let row = ((90 - lat) / 5).clamp(1, 36) as u64;
    let col = ((lon + 180) / 5).clamp(0, 71) as u64;
    let at = |r: u64, c: u64| -vm.f32(table + 4 + 4 * (r * 72 + c % 72));
    let (v00, v10) = (at(row, col), at(row - 1, col));
    let (v01, v11) = (at(row, col + 1), at(row - 1, col + 1));
    let (lo, hi) = (lat as f32, (f64::from(lat) + 5.0) as f32);
    let along = |near: f32, far: f32| {
        if lo == hi {
            (far + near) * 0.5
        } else {
            (far - near) / (hi - lo) * (a - lo) + near
        }
    };
    let (l0, l1) = (along(v00, v10), along(v01, v11));
    let (lon_lo, lon_hi) = (lon as f32, (f64::from(lon) + 5.0) as f32);
    if lon_lo == lon_hi {
        (l1 + l0) * 0.5
    } else {
        (l1 - l0) / (lon_hi - lon_lo) * (b - lon_lo) + l0
    }
}

/// `0x1407d7bc0(F, x)`: the heading `F+0x358` moved toward `F+0x410` by the fraction `r` of the way, with `r` the
/// speed `|(F+0x368, F+0x36c, F+0x370)|` between `x / 2` (0) and `x` (1) held to `0..1` (0.5 for `x == 0`). The
/// difference of the headings is wrapped to -180..180 first and the result to 0..360.
pub fn heading_blend(vm: &Vm, f: u64, x: f32) -> f32 {
    let half = (f64::from(x) * 0.5) as f32;
    let ratio = if half == x {
        0.5
    } else {
        let speed = hypot3(vm.f32(f + 0x368), vm.f32(f + 0x36c), vm.f32(f + 0x370));
        clamp((speed - half) * (1.0 / (x - half)) + 0.0, 0.0, 1.0)
    };
    let base = vm.f32(f + 0x358);
    let mut d = vm.f32(f + 0x410) - base;
    while -180.0 > d {
        d += 360.0;
    }
    while d > 180.0 {
        d += -360.0;
    }
    let mut r = d * ratio + base;
    while 0.0 > r {
        r += 360.0;
    }
    while r > 360.0 {
        r += -360.0;
    }
    r
}

/// The four curves of the start sequence (24 floats each, `0x142660790`, `0x1426607f0`, `0x142660850`,
/// `0x1426608b0`): the gas generator speed (%) and the temperature (degrees) while starting, and the same while
/// running down.
const START_CURVES: [[f32; 24]; 4] = [
    [
        0.0, 8.0, 12.0, 18.0, 24.0, 29.0, 33.0, 38.0, 42.0, 47.0, 51.0, 56.0, 62.0, 66.0, 70.0,
        74.0, 77.0, 82.0, 85.0, 90.0, 94.0, 98.0, 100.0, 100.0,
    ],
    [
        40.0, 74.0, 159.0, 261.0, 361.0, 429.0, 461.0, 488.0, 507.0, 513.0, 510.0, 507.0, 490.0,
        479.0, 468.0, 455.0, 445.0, 432.0, 427.0, 419.0, 414.0, 411.0, 404.0, 397.0,
    ],
    [
        100.0, 99.0, 90.0, 75.0, 57.0, 48.0, 40.0, 33.0, 29.0, 25.0, 23.0, 20.0, 19.0, 17.0, 16.0,
        14.0, 13.0, 12.0, 11.0, 11.0, 10.0, 0.0, 0.0, 0.0,
    ],
    [
        316.0, 314.0, 310.0, 279.0, 244.0, 231.0, 218.0, 208.0, 203.0, 197.0, 194.0, 191.0, 189.0,
        186.0, 185.0, 183.0, 182.0, 181.0, 180.0, 180.0, 179.0, 0.0, 0.0, 0.0,
    ],
];

/// `0x141245750`: the start sequence state machine (`F+0x64a8` selects one of six states; the jet engine start:
/// 0 off, 1 winding up the starter, 2 and 3 light-off and acceleration along the curves of the gas generator speed
/// `F+0x6490` and temperature `F+0x6494`, 4 running, 5 running down). `F+0x6484` is the mode (0, 1 or 2), `F+0x6488`
/// follows it, `F+0x648c` a stop request, `F+0x6498` a blend, `F+0x649c` the time in the state and `F+0x64a4` the
/// temperature offset. The commands `0xd9`/`0xda` are input bindings (`0x1407ace10(F, 2, id, 0)`); entering
/// state 1 or 5 on the stop request ends them with `0x1407cdce0` and `0x1407d6a00` (replayed).
pub fn start_sequence(vm: &mut Vm, env: &mut dyn Callees, f: u64) {
    let (s90, s94, s98, s9c, sa4, sa8) = (0x6490, 0x6494, 0x6498, 0x649c, 0x64a4, 0x64a8);
    let bind = |vm: &mut Vm, env: &mut dyn Callees, id: u64| -> bool {
        let mut args = CallArgs::ints(&[f, 2, id, 0]);
        args.stack[0] = Some(u64::from(1.0f32.to_bits()));
        env.call(vm, 0x1407ace10, args).rax as u32 != 0
    };
    let tenth = |vm: &mut Vm, env: &mut dyn Callees| (frame_time(vm, env) / 10.0) as f32;
    // the position `x` on a curve: the interpolation between the neighbouring entries; `None` below zero
    let on_curve = |curve: usize, x: f32| -> Option<(f32, usize)> {
        if 0.0 > x {
            return None;
        }
        let (low, high) = (x.floor() as i32, x.ceil() as i32);
        if high > 23 {
            return Some((0.0, 24));
        }
        let t = &START_CURVES[curve];
        let (a, c) = (t[low as usize], t[high as usize]);
        Some(((x - low as f32) * (c - a) + a, 0))
    };
    let kind = vm.i32(f + 0x6484);
    if kind == 0 {
        vm.set_i32(f + 0x6488, 0);
    } else if kind == 2 {
        vm.set_i32(f + 0x6488, 1);
    }
    let state = vm.u32(f + sa8);
    let b = vm.u64(f + 0x20);
    match state {
        0 => {
            if 1.0 > vm.f32(f + s90) {
                let rate = (frame_time(vm, env) / f64::from(vm.f32(b + 0xc58))) as f32;
                let c = vm.f32(f + s98);
                let delta = if -rate > -c {
                    -rate
                } else if rate < -c {
                    rate
                } else {
                    -c
                };
                vm.set_f32(f + s98, delta + c);
            }
            let rate = tenth(vm, env);
            let v = lerp(vm.f32(f + s90), 0.0, rate);
            vm.set_f32(f + s90, v);
            let rate = tenth(vm, env);
            let v = lerp(vm.f32(f + s94), vm.f32(f + 0x5c), rate);
            vm.set_f32(f + s94, v);
            vm.set_u32(f + sa4, 0);
            if (vm.i32(f + 0x6488) != 0 || vm.i32(f + 0x6484) == 1) && !bind(vm, env, 0xd9) {
                vm.set_u32(f + sa8, 1);
            }
            vm.set_u32(f + s9c, 0);
            vm.set_u32(f + 0x648c, 0);
        }
        1 => {
            let rate = (frame_time(vm, env) / f64::from(vm.f32(b + 0xc58))) as f32;
            let c = vm.f32(f + s98);
            let room = 1.0 - c;
            let delta = if -rate > room {
                -rate
            } else if rate < room {
                rate
            } else {
                room
            };
            vm.set_f32(f + s98, c + delta);
            let r = clamp((frame_time(vm, env) / 10.0) as f32, 0.0, 1.0);
            let v = (1.0 - r) * vm.f32(f + s90) + r * 0.0;
            vm.set_f32(f + s90, v);
            let r = clamp((frame_time(vm, env) / 10.0) as f32, 0.0, 1.0);
            let v = (1.0 - r) * vm.f32(f + s94) + vm.f32(f + 0x5c) * r;
            vm.set_f32(f + s94, v);
            vm.set_u32(f + sa4, 0);
            if (vm.i32(f + 0x6488) != 0 || vm.i32(f + 0x6484) == 2)
                && f64::from(vm.f32(f + s98)) > 0.99
                && !bind(vm, env, 0xd9)
            {
                vm.set_u32(f + sa8, 2);
            }
            if vm.i32(f + 0x6484) == 0 {
                vm.set_u32(f + sa8, 0);
            }
            vm.set_u32(f + s9c, 0);
        }
        2 | 3 => {
            vm.set_f32(f + s98, 1.0);
            let timer = (f64::from(vm.f32(f + s9c)) + frame_time(vm, env)) as f32;
            vm.set_f32(f + s9c, timer);
            let mut held_b = 0.0;
            if state == 3 {
                let rate = tenth(vm, env);
                let big = |o: u64| vm.f32(b + o);
                let (m0, m1) = (big(0xc7c), big(0xc80));
                let first = if m0 > m1 { m0 } else { m1 };
                let (m2, m3) = (big(0xc84), big(0xc88));
                let second = if m2 > m3 { m2 } else { m3 };
                let top = if first > second { first } else { second };
                let pressed = bind(vm, env, 0xda);
                let boost = if pressed { 450.0 } else { 0.0 };
                let target = (f64::from(vm.f32(f + 0x64a0) * 4.0 * top)
                    + f64::from(vm.f32(f + 0x754)) * 0.5
                    + boost) as f32;
                held_b = lerp(vm.f32(f + sa4), target, rate);
                vm.set_f32(f + sa4, held_b);
            }
            let scale = 24.0 / vm.f32(b + 0xc60);
            let x = scale * timer;
            let n1 = match on_curve(0, x) {
                None => 0.0,
                Some((_, 24)) => 100.0,
                Some((v, _)) => v,
            };
            vm.set_f32(f + s90, n1);
            let base = if state == 2 {
                vm.f32(f + 0x5c) + vm.f32(f + sa4)
            } else {
                held_b + vm.f32(f + 0x5c)
            };
            let x = scale * timer;
            let temp = match on_curve(1, x) {
                None => base,
                Some((_, 24)) => {
                    let ramp = 397.0 - (x - 23.0) * 5.0;
                    let raised = base + 350.0;
                    if raised > ramp { raised } else { ramp }
                }
                Some((v, _)) => v,
            };
            vm.set_f32(f + s94, temp);
            let mut stop_calls = false;
            if state == 2 {
                let pressed = bind(vm, env, 0xda);
                vm.set_f32(f + sa4, if pressed { 450.0 } else { 0.0 });
                if vm.i32(f + 0x6484) == 0 && 12.0 > vm.f32(f + s90) {
                    vm.set_u32(f + sa8, 0);
                }
                if vm.f32(f + s90) > 99.0 {
                    vm.set_u32(f + sa8, 3);
                }
                if vm.i32(f + 0x648c) != 0 {
                    vm.set_u32(f + sa8, 1);
                    stop_calls = true;
                }
            } else {
                if vm.i32(f + 0x6484) == 0 {
                    vm.set_u32(f + sa8, 4);
                    vm.set_u32(f + s9c, 0);
                }
                if bind(vm, env, 0xd9) {
                    vm.set_u32(f + sa8, 5);
                    vm.set_u32(f + s9c, 0);
                }
                if vm.i32(f + 0x648c) != 0 {
                    vm.set_u32(f + 0x6f0, 0);
                    vm.set_u32(f + 0xbd20, 0);
                    vm.set_u32(f + sa8, 5);
                    vm.set_u32(f + s9c, 0);
                    env.call(vm, 0x1407d6a00, CallArgs::ints(&[f, 0xda, 0]));
                    stop_calls = true;
                }
            }
            if state == 2 && stop_calls {
                vm.set_u32(f + 0xbd20, 0);
                vm.set_u32(f + 0x6f0, 0);
                vm.set_u32(f + s9c, 0);
                vm.set_u32(f + 0x1128, 0);
                env.call(vm, 0x1407cdce0, CallArgs::ints(&[f, 0xd9, 0]));
            } else if state == 3 && stop_calls {
                env.call(vm, 0x1407cdce0, CallArgs::ints(&[f, 0xd9, 0]));
            }
        }
        4 => {
            vm.set_f32(f + s98, 1.0);
            let timer = (f64::from(vm.f32(f + s9c)) + frame_time(vm, env)) as f32;
            vm.set_f32(f + s9c, timer);
            vm.set_f32(f + s90, 100.0);
            vm.set_u32(f + 0xbd20, 0);
            let rate = tenth(vm, env);
            let v = lerp(vm.f32(f + sa4), 0.0, rate);
            vm.set_f32(f + sa4, v);
            let rate = (frame_time(vm, env) / f64::from(vm.f32(b + 0xc5c))) as f32;
            let v = lerp(vm.f32(f + s94), 316.0, rate);
            vm.set_f32(f + s94, v);
            let kind = vm.i32(f + 0x6484);
            if kind == 0 {
                if 317.0 > v {
                    vm.set_u32(f + sa8, 5);
                    vm.set_u32(f + s9c, 0);
                }
            } else if kind == 1 {
                vm.set_u32(f + sa8, 3);
            }
            if vm.i32(f + 0x648c) != 0 {
                vm.set_u32(f + sa8, 5);
                vm.set_u32(f + 0xbd20, 0);
                vm.set_u32(f + 0x6f0, 0);
                vm.set_u32(f + s9c, 0);
                vm.set_u32(f + 0x1128, 0);
                env.call(vm, 0x1407cdce0, CallArgs::ints(&[f, 0xd9, 0]));
            }
        }
        5 => {
            vm.set_f32(f + s98, 1.0);
            let timer = (f64::from(vm.f32(f + s9c)) + frame_time(vm, env)) as f32;
            vm.set_f32(f + s9c, timer);
            let scale = 24.0 / vm.f32(b + 0xc64);
            let x = scale * timer;
            let n1 = match on_curve(2, x) {
                None => 0.0,
                Some((_, 24)) => {
                    let ramp = 0.0 - (x - 20.0);
                    if 0.0 > ramp { 0.0 } else { ramp }
                }
                Some((v, _)) => v,
            };
            vm.set_f32(f + s90, n1);
            let base = vm.f32(f + 0x5c);
            let x = scale * timer;
            let temp = match on_curve(3, x) {
                None => base,
                Some((_, 24)) => {
                    let ramp = 0.0 - (x - 20.0) * 1.6;
                    if base > ramp { base } else { ramp }
                }
                Some((v, _)) => v,
            };
            vm.set_f32(f + s94, temp);
            vm.set_u32(f + sa4, 0);
            if vm.i32(f + 0x6484) == 0 && 11.0 >= n1 {
                vm.set_u32(f + sa8, 0);
            }
        }
        _ => {}
    }
}
