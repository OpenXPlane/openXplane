//! The later blocks of the flight step (`0x1412656b0`): bookkeeping and derived quantities computed after the
//! state advance. Each function is a block of the original, compared with the machine code in an emulator
//! (`tools/gen_flight_block_vectors.py`), with the callees outside this port replayed.

use crate::flight_step::position_component;
use crate::scalar::{clamp, lerp};
use crate::vm::{CallArgs, Callees, Vm};
use crate::wing_element::interpolate_clamped;

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
