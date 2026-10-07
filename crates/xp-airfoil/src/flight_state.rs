//! The later blocks of the flight step (`0x1412656b0`): bookkeeping and derived quantities computed after the
//! state advance. Each function is a block of the original, compared with the machine code in an emulator
//! (`tools/gen_flight_block_vectors.py`), with the callees outside this port replayed.

use crate::flight_step::position_component;
use crate::vm::{CallArgs, Callees, Vm};

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
