//! Blocks of the flight step `update_flight` (`0x1412656b0`) that follow the original's objects ([`Vm`]); each
//! block is compared with the original on random objects by running it from its start address to its end
//! address in the emulator (`tools/gen_flight_block_vectors.py`).
use crate::callees::wing_area_factor;
use crate::element_force::boundary_ratio;
use crate::engine::signed_pow;
use crate::vm::{CallArgs, Callees, Vm};
use crate::wing_element::interpolate_clamped;

/// `maxss`: the first operand when it is larger, otherwise the second.
fn sse_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// The number of wing records of the aircraft (`B+0x6028`, stride `0x36c8`) and of the element state records
/// (`F+0x6940`, stride `0x2d8`) the step loops over.
pub const WINGS: i32 = 0x30;

/// `0x1407ace10(F, 1, id, index, 1.0)`: an input-binding query.
fn bind(env: &mut dyn Callees, f: u64, id: u32, index: i32) -> bool {
    let args = CallArgs::ints(&[f, 1, u64::from(id), index as u32 as u64]);
    env.call(&mut Vm::default(), 0x1407ace10, args).rax as u32 != 0
}

/// `0x1411da150(F, index)`: the enabled byte (`+0x678`) of wing record `index`, or zero when the binding `0x251`
/// queried with the index is set.
pub fn wing_enabled(vm: &Vm, env: &mut dyn Callees, f: u64, index: i32) -> u8 {
    if bind(env, f, 0x251, index) {
        return 0;
    }
    let b = vm.u64(f + 0x20);
    let wings = vm.u64(b + 0x6028);
    vm.u8(wings + (i64::from(index) * 0x36c8) as u64 + 0x678)
}

/// `0x14120c960(F+0xbcc8)`: whether the debug dump is active (either of the two words set).
pub fn debug_dump_active(vm: &Vm, f: u64) -> bool {
    vm.i32(f + 0xbcc8) != 0 || vm.i32(f + 0xbcd0) != 0
}

/// `0x141265f7d..0x14126644a`: the aspect-ratio factors of the wings. For each of the 48 wings that is enabled and
/// whose `|W+0xf4|` is below 45, the aspect ratio `ar` (the boundary ratio `0x14121b290`, at least 0) and the area
/// factor `0x141294180` give three blending factors, written to the element state `X+0x288`, `+0x28c`, `+0x290`:
/// clamped lines of the ratio `ar / (2 W+0x36a8)` between 1 and 2 (the first towards 1, the second and third
/// towards the values from `ar` and `ar / area`).
pub fn wing_aspect_pass(vm: &mut Vm, env: &mut dyn Callees, f: u64) -> Result<(), String> {
    if debug_dump_active(vm, f) {
        return Err("debug dump not ported".into());
    }
    let b = vm.u64(f + 0x20);
    for i in 0..WINGS {
        if wing_enabled(vm, env, f, i) == 0 {
            continue;
        }
        let w = vm.u64(b + 0x6028) + (i64::from(i) * 0x36c8) as u64;
        let x = vm.u64(f + 0x6940) + (i64::from(i) * 0x2d8) as u64;
        let incidence = vm.f32(w + 0xf4).abs();
        // `comiss 45.0, |incidence|; jbe`: skipped when 45 <= |incidence| or unordered
        if 45.0 <= incidence || incidence.is_nan() {
            continue;
        }
        // 0x14121b290 queries the engine flag three times; the second answer is the one it uses
        let mut flags = [false; 3];
        for flag in &mut flags {
            let args = CallArgs::ints(&[0x1424_f5648]);
            *flag = env.call(&mut Vm::default(), 0x1417f12c0, args).rax as u8 != 0;
        }
        let ar = sse_max(boundary_ratio(&(&*vm, w), &(&*vm, f), flags[1]), 0.0);
        let area = wing_area_factor(vm, w);
        let ar_over_area = ar / area;
        let w36a8 = vm.f32(w + 0x36a8);
        let half = (f64::from(ar) / (f64::from(w36a8) + f64::from(w36a8))) as f32;
        let q = sse_max(ar / w36a8, 0.1);
        let inverse = (0.020000000000000004 / f64::from(q)) as f32;
        let scaled = f64::from(inverse) * 8.0 / f64::from(sse_max(vm.f32(w + 0x18), 1.0)) + 1.0;
        let v8 = scaled as f32;
        let power = signed_pow(half, 1.5);
        let t = f64::from(power) * 33.0;
        let v6 = (t / (t + 1.0)) as f32;
        let m = sse_max(ar_over_area, 0.01);
        let v7 = (1.0 / (1.0 / f64::from(m) + 1.0)) as f32;
        vm.set_f32(x + 0x288, interpolate_clamped(1.0, v8, 2.0, 1.0, half));
        let second = interpolate_clamped(1.0, v6, 2.0, 1.0, half);
        vm.set_f32(x + 0x28c, second);
        let third = interpolate_clamped(1.0, v7, 2.0, 1.0, half);
        vm.set_f32(x + 0x290, ((f64::from(third + second)) * 0.5) as f32);
    }
    Ok(())
}

const RAD: f32 = f32::from_bits(0x3c8efa36);

fn neg(v: f32) -> f32 {
    f32::from_bits(v.to_bits() ^ 0x8000_0000)
}

/// `0x14126644a..0x141266b52`: the engine controls (`0x141260090`, called through the environment), the thrust
/// of the manoeuvring rockets, the pitch-tilt thrust, the blown-flap factors:
///
/// * while the rocket timer `F+0x6538` is ahead of the simulation time, the code `F+0x6518` selects one of the six
///   rocket forces (`0x264..0x269`: +/- normal `B+0x20d8`, +/- side `B+0x20d4`, +/- axial `B+0x20dc`) added at the
///   origin;
/// * while `F+0x6514` is set, the thrust `B+0x2920` at the angle `B+0x291c` is added as an axial and a normal
///   force (at the lever arms `B+0x2914`/`B+0x2918`) and the timer `F+0x6534` runs down;
/// * with `F+0x57c` set, the moments `B+0x20c8..0x20d0` are added scaled by `F+0x11c/0x118/0x120`;
/// * the blown-flap factor `F+0x64bc` from the flap setting `F+0x184` and, when `B+0x21e8` is positive, the mean of
///   the squared jet speeds (`F+0x64c0`).
pub fn thrust_effects_pass(vm: &mut Vm, env: &mut dyn Callees, f: u64) -> Result<(), String> {
    use crate::callees::{add_axial_force, add_normal_force, add_side_force};
    env.call(vm, 0x141260090, CallArgs::ints(&[f]));
    if debug_dump_active(vm, f) {
        return Err("debug dump not ported".into());
    }
    let b = vm.u64(f + 0x20);
    if f64::from(vm.f32(f + 0x6538)) > vm.f64(crate::callees::SIM_TIME) {
        let code = vm.i32(f + 0x6518);
        match code {
            0x264 => add_normal_force(vm, f, vm.f32(b + 0x20d8), 0.0, 0.0),
            0x265 => add_normal_force(vm, f, neg(vm.f32(b + 0x20d8)), 0.0, 0.0),
            0x266 => add_side_force(vm, f, neg(vm.f32(b + 0x20d4)), 0.0, 0.0),
            0x267 => add_side_force(vm, f, vm.f32(b + 0x20d4), 0.0, 0.0),
            0x268 => add_axial_force(vm, f, neg(vm.f32(b + 0x20dc)), 0.0, 0.0),
            0x269 => add_axial_force(vm, f, vm.f32(b + 0x20dc), 0.0, 0.0),
            _ => {}
        }
    }
    if vm.i32(f + 0x6514) != 0 {
        let angle = vm.f32(b + 0x291c) * RAD;
        let force = angle.cos() * neg(vm.f32(b + 0x2920));
        add_axial_force(vm, f, force, 0.0, vm.f32(b + 0x2914));
        let force = angle.sin() * vm.f32(b + 0x2920);
        add_normal_force(vm, f, force, 0.0, vm.f32(b + 0x2918));
        let dt = f64::from_bits(
            env.call(vm, 0x140c448c0, CallArgs::ints(&[0x1_42f0_18b8]))
                .xmm0,
        );
        let remaining = (f64::from(vm.f32(f + 0x6534)) - dt) as f32;
        vm.set_f32(f + 0x6534, remaining);
        if 0.0 > remaining {
            vm.set_i32(f + 0x6514, 0);
        }
    }
    if vm.i32(f + 0x57c) != 0 {
        let (x0, x1, x2) = (vm.f32(b + 0x20c8), vm.f32(b + 0x20cc), vm.f32(b + 0x20d0));
        if x0 > 0.0 || x1 > 0.0 || x2 > 0.0 {
            vm.set_f32(f + 0x2f8, x0 * vm.f32(f + 0x11c) + vm.f32(f + 0x2f8));
            vm.set_f32(f + 0x310, x1 * vm.f32(f + 0x118) + vm.f32(f + 0x310));
            vm.set_f32(f + 0x328, x2 * vm.f32(f + 0x120) + vm.f32(f + 0x328));
        }
    }
    let blown = interpolate_clamped(vm.f32(b + 0x21e4), 0.0, 1.0, 1.0, vm.f32(f + 0x184));
    vm.set_f32(f + 0x64bc, blown);
    vm.set_i32(f + 0x64c0, 0);
    let engines = vm.i32(b + 0x91c);
    if engines > 0 && vm.f32(b + 0x21e8) > 0.0 && blown > 0.0 {
        let (mut sum, mut count) = (0.0f32, 0.0f32);
        for e in 0..engines {
            if bind(env, f, 0x179, e) {
                continue;
            }
            let engine = vm.u64(b + 0x5ff8) + (i64::from(e) * 0x68) as u64;
            let jet = (vm.i32(engine).wrapping_sub(5) as u32) <= 1;
            let state = vm.u64(f + 0x68b0) + (i64::from(e) * 0x2cc) as u64;
            if jet && vm.i32(state + 0x298) != 3 {
                let n1 = (f64::from(vm.f32(state + 0x90)) * 0.01) as f32;
                sum += n1 * n1;
                count = (f64::from(count) + 1.0) as f32;
            }
        }
        if sum > 0.0 && count > 0.0 {
            vm.set_f32(f + 0x64c0, sum / count * blown * vm.f32(b + 0x21e8));
        }
    }
    Ok(())
}

/// `0x1407d6ce0(x, lo, hi)`: `x` moved by multiples of `hi - lo` into `lo..=hi` (a NaN is returned unchanged).
pub fn wrap_range(x: f32, lo: f32, hi: f32) -> f32 {
    let span = hi - lo;
    let mut x = x;
    while lo > x {
        x += span;
    }
    while x > hi {
        x -= span;
    }
    x
}

/// The floats `+0x5bc/0x5e8/0x614` (the boundary coordinates) and `+0x70` (the chord) of wing record `w`, one entry
/// per boundary (`elements + 1`).
fn boundary_arrays(vm: &Vm, w: u64, elements: i32) -> [Vec<f32>; 4] {
    let read = |base: u64| -> Vec<f32> {
        (0..=elements.max(0) as u64)
            .map(|i| vm.f32(w + base + 4 * i))
            .collect()
    };
    [read(0x5bc), read(0x5e8), read(0x614), read(0x70)]
}

/// The element loop of the flight step (`0x141266b52..0x141267978`): for each enabled wing record `W` and each of
/// its span elements `e`, the element's boundary point `(a0, a1, a2)` gives the air velocity `(A, B, C)` there
/// (`0x14121b580`, called through the environment, results in the frame slots), which is resolved along and across
/// the element's dihedral; the relative speed `X+0x50` (and its blend by `W+0x54`), the angle `X+0x28`, the
/// sweep-corrected `X+0` factor and the control-surface counters `F+0x544..0x558` follow; the element force
/// (`0x1411b9840`, through the environment) is added to the wing's sums `X+0x294/0x298` and the moments
/// `F+0x314/0x32c`, and the aerodynamic force `0x140f26ef0` is applied at the point.
///
/// The original's locals live at `rbp + offset`; `rbp` is where the caller keeps its frame, and the slots the
/// environment calls write are read back from there.
pub fn element_pass(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) -> Result<(), String> {
    use crate::callees::{AeroForce, add_aero_force};
    use crate::scalar::{clamp, sign};
    use crate::wing_element::{
        Boundary, boundary_at, delta_wing_weight, element_dihedral, hypot2, hypot3, signed_sqrt,
    };
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    for v in [0x544, 0x54c, 0x554] {
        vm.set_u64(f + v, 0);
    }
    const DEG: f32 = f32::from_bits(0x42652ee0);
    const KNOTS: f32 = f32::from_bits(0x3ff8cfe5);
    let b = vm.u64(f + 0x20);
    for j in 0..WINGS {
        if wing_enabled(vm, env, f, j) == 0 {
            continue;
        }
        let x = vm.u64(f + 0x6940) + (i64::from(j) * 0x2d8) as u64;
        let w = vm.u64(b + 0x6028) + (i64::from(j) * 0x36c8) as u64;
        let elements = vm.i32(w + 4);
        if elements <= 0 {
            continue;
        }
        let [bx, by, bz, bc] = boundary_arrays(vm, w, elements);
        let bound = Boundary {
            x: &bx,
            y: &by,
            z: &bz,
            chord: &bc,
        };
        for e in 0..elements {
            let (we, xe) = (w + 4 + 4 * e as u64, x + 4 + 4 * e as u64);
            let e_f = e as f32;
            let u = e_f + 0.5;
            let a0 = boundary_at(&bx, elements, u);
            let a1 = boundary_at(&by, elements, u);
            let a2 = boundary_at(&bz, elements, u);
            if debug_dump_active(vm, f) {
                return Err("debug dump not ported".into());
            }
            let mut args = CallArgs::ints(&[f, 0, slot(-0x5c)]);
            args.int[1] = None;
            args.xmm[1] = Some(a0.to_bits());
            args.xmm[3] = Some(a1.to_bits());
            args.stack[0] = Some(slot(-0x74));
            args.stack[1] = None;
            args.stack[2] = Some(slot(-0x44));
            env.call(vm, 0x14121b580, args);
            let (air_a, air_b, air_c) = (
                vm.f32(slot(-0x5c)),
                vm.f32(slot(-0x74)),
                vm.f32(slot(-0x44)),
            );
            let dihedral = element_dihedral(&bound, e as usize);
            let cs = signed_sqrt((dihedral * RAD).cos());
            let w0 = vm.f32(w);
            let sg = sign(element_dihedral(&bound, e as usize));
            let t = (1.0 - f64::from(cs * cs)) as f32;
            let sn = sg * (signed_sqrt(t) * w0);
            let chord_l = w0 * vm.f32(we + 0x118);
            let s140 = vm.f32(we + 0x140);
            let along = s140 * air_a + chord_l * air_b;
            let across = s140 * air_b - chord_l * air_a;
            let v1 = along * cs + sn * air_c;
            let v2 = cs * air_c - along * sn;
            let angle_a = across.atan2(air_c) * DEG;
            let h1 = hypot2(across, v2);
            let n3 = hypot3(v1, across, v2);
            let weight = delta_wing_weight(&bound, e as usize, vm.f32(w + 0x18), vm.f32(w + 0x1c));
            let relative = interpolate_clamped(0.0, h1, 1.0, n3, weight);
            vm.set_f32(xe + 0x50, relative);
            let limited = clamp(element_dihedral(&bound, e as usize), -60.0, 60.0).abs();
            let slope = clamp(v1.abs().atan2(v2) * DEG, -60.0, 60.0).abs();
            let ratio = (slope * RAD).cos() / (limited * RAD).cos();
            let ratio = ratio * ratio;
            vm.set_f32(xe, vm.f32(x) * vm.f32(w + 0x18) * ratio);
            let angle = angle_a + vm.f32(we + 0x168) + vm.f32(we + 0x190);
            vm.set_f32(xe + 0x28, wrap_range(angle, -180.0, 180.0));
            if vm.i32(w + 0x58) != 0 {
                let position = clamp(vm.i32(w + 4) as f32 * vm.f32(w + 0x54) - e_f, 0.0, 1.0);
                let blended = ((1.0 - f64::from(position)) * f64::from(relative)) as f32;
                vm.set_f32(xe + 0x50, blended);
            }
            let any = |vm: &Vm, offsets: [u64; 4]| offsets.iter().any(|o| vm.i32(we + o) != 0);
            for (flags, count, sum) in [
                ([0x2f8, 0x328, 0x1b8, 0x1e0], 0x544u64, 0x548u64),
                ([0x358, 0x388, 0x208, 0x230], 0x54c, 0x550),
                ([0x3b8, 0x3e8, 0x258, 0x280], 0x554, 0x558),
            ] {
                if any(vm, flags) {
                    vm.set_f32(f + count, (f64::from(vm.f32(f + count)) + 1.0) as f32);
                    let v = vm.f32(xe + 0x50) * KNOTS + vm.f32(f + sum);
                    vm.set_f32(f + sum, v);
                }
            }
            let ice = if j < 7 {
                if 0.0 > vm.f32(w) {
                    vm.f32(f + 0xb798)
                } else {
                    vm.f32(f + 0xb79c)
                }
            } else if j < 11 {
                if 0.0 > vm.f32(w) {
                    vm.f32(f + 0xb7a0)
                } else {
                    vm.f32(f + 0xb7a4)
                }
            } else {
                0.0
            };
            let _ = ice;
            let mut args = CallArgs::ints(&[f, w, x, 1]);
            // (the element index and the ice factor share their 8-byte stack slots with unwritten words)
            args.stack[2] = Some(slot(-0x6c));
            args.stack[3] = Some(slot(-0x30));
            env.call(vm, 0x1411b9840, args);
            let (o1, o2, o3) = (
                vm.f32(slot(-0x6c)),
                vm.f32(slot(-0x30)),
                vm.f32(slot(-0x18)),
            );
            vm.set_f32(x + 0x294, o1 + vm.f32(x + 0x294));
            vm.set_f32(x + 0x298, o2 + vm.f32(x + 0x298));
            vm.set_f32(f + 0x314, o3 * s140 + vm.f32(f + 0x314));
            vm.set_f32(f + 0x32c, vm.f32(f + 0x32c) - o3 * chord_l);
            let force = AeroForce {
                a2: air_a,
                a3: o1 * s140,
                a5: air_b,
                a6: o2,
                a7: air_c,
                a8: neg(o1) * chord_l,
                a9: a0,
                a10: a1,
                a11: a2,
                a12: vm.f32(xe + 0x28),
                a13: vm.f32(xe + 0x1b8),
                a14: vm.i32(xe + 0x1e0),
                a15: 0.0,
                a16: 0.0,
                a17: 0.0,
            };
            add_aero_force(vm, f, &force);
        }
    }
    for (count, sum) in [(0x544u64, 0x548u64), (0x54c, 0x550), (0x554, 0x558)] {
        let divisor = sse_max(vm.f32(f + count), 1.0);
        vm.set_f32(f + sum, vm.f32(f + sum) / divisor);
    }
    Ok(())
}
