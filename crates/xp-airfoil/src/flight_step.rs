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

/// `0x1411d9ec0(F, i)`: the enabled byte (`+0x588`) of body record `i` (`B+0x6040`, stride `0x34c8`), zero when the
/// binding `0x179` queried with the record's mode word (`+0x5f0`, when at most 0x26) is set or `+0x54` is nonzero.
pub fn body_enabled(vm: &Vm, env: &mut dyn Callees, f: u64, i: i32) -> u8 {
    let b = vm.u64(f + 0x20);
    let r = vm.u64(b + 0x6040) + (i64::from(i) * 0x34c8) as u64;
    let mode = vm.u32(r + 0x5f0);
    crate::engine::record_flag_6040(mode, vm.i32(r + 0x54), vm.u8(r + 0x588), |id, _| {
        bind(env, f, id, mode as i32)
    })
}

/// `0x141267978..0x1412686a9`: the body loop (39 bodies, `B+0x6040`): the body's reference point is rotated into
/// the aircraft frame (`0x14120cf60` with the lever curve of the record), the air velocity there
/// (`0x14121b580`, replayed) is turned back into the body's axes (`0x141291580`) and into the angles
/// (`0x141183bf0`); [`body_aero`](crate::body::body_aero) gives the three cross-flow results, blended with
/// the wave drag ([`body_wave_drag`](crate::body::body_wave_drag)) above the Mach-dependent blend factor
/// unless the record's engine index is a jet; the drag coefficient `+4` is stored and the force is applied at the
/// point by `0x140f26ef0` along the air direction.
pub fn body_pass(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) -> Result<(), String> {
    use crate::body::{body_aero, body_wave_drag};
    use crate::callees::{AeroForce, add_aero_force, body_blend, direction_angles, lever_curve};
    use crate::engine::root_ratio;
    use crate::transform::rotate_euler_offset;
    use crate::wing_element::rotate_euler;
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    let b = vm.u64(f + 0x20);
    const DEG: f32 = f32::from_bits(0x42652ee0);
    for i in 0..0x27 {
        if body_enabled(vm, env, f, i) == 0 {
            continue;
        }
        if debug_dump_active(vm, f) {
            return Err("debug dump not ported".into());
        }
        let r = vm.u64(b + 0x6040) + (i64::from(i) * 0x34c8) as u64;
        let obj = r + 0x588;
        let angles = [vm.f32(obj + 0x9c), vm.f32(obj + 0xa0), vm.f32(obj + 0xa4)];
        let offsets = [vm.f32(obj + 0x90), vm.f32(obj + 0x94), vm.f32(obj + 0x98)];
        let lever = lever_curve(vm, r, 0.0);
        let p = rotate_euler_offset(
            angles,
            offsets,
            true,
            vm.f32(r + 0x1c),
            vm.f32(r + 0x20),
            lever,
        );
        let mut args = CallArgs::ints(&[f, 0, slot(-0x70)]);
        args.int[1] = None;
        args.xmm[1] = Some(p[0].to_bits());
        args.xmm[3] = Some(p[1].to_bits());
        args.stack[0] = Some(slot(-0x60));
        args.stack[2] = Some(slot(0x1758));
        env.call(vm, 0x14121b580, args);
        let air = [
            vm.f32(slot(-0x70)),
            vm.f32(slot(-0x60)),
            vm.f32(slot(0x1758)),
        ];
        let q = rotate_euler(angles, air[0], air[1], air[2]);
        let [roll, pitch, yaw, speed] = direction_angles(q[0], q[1], q[2]);
        let (t_air, t_dir) = (vm.f32(f + 0x6c), vm.f32(f + 0x74));
        let forces = body_aero(vm, r, t_air, roll, pitch, yaw, speed);
        let (mut out1, mut out2, mut out3, magnitude) = match forces {
            Some(x) => (x.axial, x.side, x.normal, x.magnitude),
            // the outputs keep the zeros the original stored before the call
            None => (0.0, 0.0, 0.0, 0.0),
        };
        let mach = speed / t_dir;
        let ratio = root_ratio(
            vm.f32(r + 0x10),
            vm.f32(r + 0x58c + (i64::from(vm.i32(r + 0x654)) * 0xd8) as u64),
            vm.f32(r + 0x664),
        );
        let blend = body_blend(mach, ratio, magnitude);
        if blend > 0.0 {
            let index = vm.i32(r + 0x68);
            let jet = index >= 0 && {
                let engine = vm.u64(b + 0x5ff8) + (i64::from(index) * 0x68) as u64;
                (vm.i32(engine).wrapping_sub(5) as u32) <= 1
            };
            if !jet {
                let wave = body_wave_drag(vm, r, t_air, t_dir, speed);
                out3 = interpolate_clamped(0.0, out3, 1.0, wave, blend * vm.f32(r + 8));
            }
        }
        let dynamic = (f64::from(speed * speed * t_air) * 0.5) as f32;
        vm.set_f32(
            r + 4,
            out3 / sse_max(dynamic * vm.f32(r + 0x10), f32::from_bits(0x38d1b717)),
        );
        if f64::from(vm.f32(b + 0x2894)) > 0.01 && vm.i32(f + 0xdac) != 0 {
            for v in [&mut out1, &mut out3, &mut out2] {
                *v = (f64::from(*v) * 0.05) as f32;
            }
        }
        let lever = lever_curve(vm, r, yaw * DEG);
        let p = rotate_euler_offset(
            angles,
            offsets,
            true,
            vm.f32(r + 0x1c),
            vm.f32(r + 0x20),
            lever,
        );
        let force = AeroForce {
            a2: air[0],
            a3: out1,
            a5: air[1],
            a6: out3,
            a7: air[2],
            a8: out2,
            a9: p[0],
            a10: p[1],
            a11: p[2],
            a12: 0.0,
            a13: 0.0,
            a14: 0,
            a15: 0.0,
            a16: 0.0,
            a17: 0.0,
        };
        add_aero_force(vm, f, &force);
    }
    Ok(())
}

/// `update_flight` `0x141269920..0x14126a791`: the drag of the propeller-like parts. When the aircraft's
/// reference speed `B+0xc0c` (above 0.5) and the coefficients `B+0xc10`, `B+0xc14` are positive, every part
/// record (`B+0x6010`, stride `0x3770`) gets the aircraft velocity `F+0x29c/0x2a8/0x2b4` (non-finite
/// components count as zero), plus, when `B+0xc08` is set, the rotation of `(0, 0, N+0x84)` by the part's angles
/// (`0x14120cf60`); a force of magnitude `B+0xc14 * B+0xc10 * (speed / B+0xc0c)^2 * F+0x6c * 0.5 * M+0x44` along
/// that velocity is added at the part's offset by `0x140f26ef0`.
pub fn part_force_pass(vm: &mut Vm, f: u64, rbp: u64) -> Result<(), String> {
    use crate::callees::{AeroForce, add_aero_force};
    use crate::transform::rotate_euler_offset;
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    let b = vm.u64(f + 0x20);
    let (c0c, c10, c14) = (vm.f32(b + 0xc0c), vm.f32(b + 0xc10), vm.f32(b + 0xc14));
    let active = c0c > 0.5 && c10 > 0.0 && c14 > 0.0 && vm.i32(b + 0x91c) > 0;
    if !active {
        return Ok(());
    }
    let finite = |v: f32| if v.is_finite() { v } else { 0.0 };
    for i in 0..vm.i32(b + 0x91c) as u64 {
        let mut vx = finite(vm.f32(f + 0x29c));
        let mut vy = finite(vm.f32(f + 0x2a8));
        let mut vz = finite(vm.f32(f + 0x2b4));
        let p = vm.u64(b + 0x6010) + i * 0x3770;
        if vm.i32(b + 0xc08) != 0 {
            let n = vm.u64(f + 0x68c8) + i * 0x388;
            let obj = p + 0x700;
            let angles = [vm.f32(obj + 0x9c), vm.f32(obj + 0xa0), vm.f32(obj + 0xa4)];
            let r = rotate_euler_offset(angles, [0.0; 3], false, 0.0, 0.0, vm.f32(n + 0x84));
            vx += r[0];
            vy += r[1];
            vz += r[2];
        }
        vx = finite(vx);
        vy = finite(vy);
        vz = finite(vz);
        let speed = {
            let sum = vy * vy + vx * vx + vz * vz;
            if 0.0 > sum { f32::NAN } else { sum.sqrt() }
        };
        let ratio = speed / c0c;
        vm.set_f32(slot(-0x6c), ratio);
        let m = vm.u64(f + 0x68b0) + i * 0x2cc;
        let scale = c14 * c10 * (ratio * ratio) * vm.f32(f + 0x6c);
        let magnitude = (f64::from(scale) * 0.5 * f64::from(vm.f32(m + 0x44))) as f32;
        vm.set_f32(slot(0x1758), magnitude);
        if debug_dump_active(vm, f) {
            return Err("debug dump not ported".into());
        }
        let force = AeroForce {
            a2: vx,
            a3: 0.0,
            a5: vy,
            a6: magnitude,
            a7: vz,
            a8: 0.0,
            a9: vm.f32(p + 0x790),
            a10: vm.f32(p + 0x794),
            a11: vm.f32(p + 0x798),
            a12: 0.0,
            a13: 0.0,
            a14: 0,
            a15: 0.0,
            a16: 0.0,
            a17: 0.0,
        };
        add_aero_force(vm, f, &force);
    }
    Ok(())
}

/// `update_flight` `0x1412709ff..0x141271fa2`: the equations of motion. The totals in the aircraft axes
/// (`F+0x2f4` side, `+0x2e0` normal, `+0x2cc` axial, `+0x30c/0x324/0x33c` the moments) are divided by the mass
/// `F+0x288 * 9.798...` into the accelerations `+0x354/0x344/0x34c`, rotated into the world axes
/// (`from_aircraft_frame`, stored at `+0x35c/0x360/0x364`) with gravity toward the planet's centre subtracted
/// (`F+0x78 * position / radius`, radius from the position and `6378145`), and Euler's equations with the
/// inertias in the frame locals `rbp+0x1750`, `rbp+0x1758`, `rbp-0x78` give the angular accelerations
/// `+0x3c0/0x3c4/0x3c8`. The velocities `+0x368..0x370` and rates `+0x3cc..0x3d4` advance by the frame time
/// (doubles, six separate time queries). With the runaway-acceleration switches (`F+0x24c` and the globals
/// `0x145899fe0/4`) set, an acceleration above 15, or an altitude above the limit `F+0x42f5c` (with a speed
/// above 100 when the table flag is set), divides velocities and rates by the acceleration and sets `F+0xda8`.
pub fn rigid_body_step(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) -> Result<(), String> {
    use crate::transform::{Frame, from_aircraft_frame};
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    let engine_flag = |vm: &mut Vm, env: &mut dyn Callees| -> bool {
        env.call(vm, 0x1417f12c0, CallArgs::ints(&[0x1424_f5648]))
            .rax as u8
            != 0
    };
    let position = |vm: &mut Vm, env: &mut dyn Callees, offset: u64| -> f64 {
        if engine_flag(vm, env) {
            0.0
        } else {
            vm.f64(f + offset)
        }
    };
    let frame_time = |vm: &mut Vm, env: &mut dyn Callees| {
        f64::from_bits(env.call(vm, 0x140c448c0, CallArgs::default()).xmm0)
    };
    let check = |vm: &mut Vm, address: u64| {
        if !vm.f32(address).is_finite() {
            vm.set_f32(address, 0.0);
        }
    };
    const EARTH: f64 = 6378145.0;
    let z = position(vm, env, 0x388) as f32;
    let y = (position(vm, env, 0x380) + EARTH) as f32;
    let x = position(vm, env, 0x378) as f32;
    let radius = {
        let sum = y * y + x * x + z * z;
        if 0.0 > sum { f32::NAN } else { sum.sqrt() }
    };
    for off in [0x2f4, 0x2e0, 0x2cc] {
        check(vm, f + off);
    }
    let mass = vm.f32(f + 0x288) * f32::from_bits(0x411c_c5c1);
    let side = vm.f32(f + 0x2f4) / mass;
    vm.set_f32(f + 0x354, side);
    let normal = vm.f32(f + 0x2e0) / mass;
    vm.set_f32(f + 0x344, normal);
    let axial = vm.f32(f + 0x2cc) / mass;
    vm.set_f32(f + 0x34c, axial);
    let k = f32::from_bits(0x411c_c5c1);
    let frame = Frame {
        origin: [vm.f64(f + 0x378), vm.f64(f + 0x380), vm.f64(f + 0x388)],
        rotation: [
            [vm.f32(f + 0x430), vm.f32(f + 0x434)],
            [vm.f32(f + 0x440), vm.f32(f + 0x444)],
            [vm.f32(f + 0x450), vm.f32(f + 0x454)],
        ],
    };
    let world = from_aircraft_frame(&frame, [side * k, normal * k, axial * k], false, false);
    for (k, v) in world.iter().enumerate() {
        vm.set_f32(f + 0x35c + 4 * k as u64, *v);
    }
    for off in [0x35c, 0x360, 0x364] {
        check(vm, f + off);
    }
    let g = f64::from(vm.f32(f + 0x78));
    let r = f64::from(radius);
    let px = position(vm, env, 0x378);
    let gx = g * px / r;
    vm.set_f32(f + 0x35c, (f64::from(vm.f32(f + 0x35c)) - gx) as f32);
    let py = position(vm, env, 0x380) + EARTH;
    let gy = py * g / r;
    vm.set_f32(f + 0x360, (f64::from(vm.f32(f + 0x360)) - gy) as f32);
    let pz = position(vm, env, 0x388);
    let gz = g * pz / r;
    vm.set_f32(f + 0x364, (f64::from(vm.f32(f + 0x364)) - gz) as f32);
    for off in [0x35c, 0x360, 0x364] {
        check(vm, f + off);
    }
    let (w1, w2, w3) = (vm.f32(f + 0x3cc), vm.f32(f + 0x3d0), vm.f32(f + 0x3d4));
    let (i1, i2, i0) = (
        vm.f32(slot(0x1750)),
        vm.f32(slot(0x1758)),
        vm.f32(slot(-0x78)),
    );
    let (l, m, n) = (vm.f32(f + 0x30c), vm.f32(f + 0x324), vm.f32(f + 0x33c));
    vm.set_f32(f + 0x3c0, (l - (i1 - i2) * w2 * w3) / i0);
    vm.set_f32(f + 0x3c4, (m - (i0 - i1) * w1 * w3) / i2);
    vm.set_f32(f + 0x3c8, (n - (i2 - i0) * w1 * w2) / i1);
    if debug_dump_active(vm, f) {
        return Err("debug dump not ported".into());
    }
    for off in [0x368, 0x36c, 0x370] {
        check(vm, f + off);
    }
    for (target, rate) in [
        (0x368, 0x35c),
        (0x36c, 0x360),
        (0x370, 0x364),
        (0x3cc, 0x3c0),
        (0x3d0, 0x3c4),
        (0x3d4, 0x3c8),
    ] {
        let dt = frame_time(vm, env);
        let step = dt * f64::from(vm.f32(f + rate));
        vm.set_f32(f + target, (f64::from(vm.f32(f + target)) + step) as f32);
    }
    if vm.i32(f + 0x28) == 0 {
        let reading = vm.f32(f + 0x53c) * 0.0 + vm.f32(f + 0x538);
        let limit = 10.0f32;
        if limit > reading
            && (0x145899fd0u64..=0x145899fdc)
                .step_by(4)
                .any(|a| vm.i32(a) != 0)
        {
            vm.set_i32(f + 0x3cc, 0);
        }
    }
    // 0x141964300(0x14611ac80, F+0x42f84): a word of the table entry
    let table = vm.u64(0x1_4611_ac80 + 8);
    let flag_word = vm.i32(table + (i64::from(vm.i32(f + 0x42f84)) * 36) as u64 + 8);
    let (a1, a2, a3) = (vm.f32(f + 0x354), vm.f32(f + 0x34c), vm.f32(f + 0x344));
    let acceleration = {
        let sum = a3 * a3 + a2 * a2 + a1 * a1;
        if 0.0 > sum { f32::NAN } else { sum.sqrt() }
    };
    let mut clamp = false;
    if vm.i32(f + 0x24c) != 0 && vm.i32(0x1_4589_9fe0) == -1 && vm.i32(0x1_4589_9fe4) == -1 {
        if acceleration > 15.0 {
            clamp = true;
        } else {
            let limit = f64::from(vm.f32(f + 0x42f5c));
            let altitude = position(vm, env, 0x380);
            if flag_word == 0 {
                clamp = limit > altitude;
            } else if limit > altitude {
                let (v1, v2, v3) = (vm.f32(f + 0x368), vm.f32(f + 0x36c), vm.f32(f + 0x370));
                let speed = {
                    let sum = v1 * v1 + v2 * v2 + v3 * v3;
                    if 0.0 > sum { f32::NAN } else { sum.sqrt() }
                };
                clamp = speed > 100.0;
            }
        }
    }
    if clamp {
        if vm.i32(f + 0x24c) != 0 {
            for off in [0x368, 0x36c, 0x370, 0x3cc, 0x3d0, 0x3d4] {
                vm.set_f32(f + off, vm.f32(f + off) / acceleration);
            }
        }
        vm.set_i32(f + 0xda8, 1);
    }
    for off in [0x368, 0x36c, 0x370] {
        check(vm, f + off);
    }
    if debug_dump_active(vm, f) {
        return Err("debug dump not ported".into());
    }
    Ok(())
}

/// `0x1407ce820/0x1407ce850/0x1407ce880(F)`: the position doubles `F+0x378/0x380/0x388`, zero when the engine
/// flag is set.
pub(crate) fn position_component(vm: &mut Vm, env: &mut dyn Callees, f: u64, offset: u64) -> f64 {
    let flag = env
        .call(vm, 0x1417f12c0, CallArgs::ints(&[0x1424_f5648]))
        .rax as u8
        != 0;
    if flag { 0.0 } else { vm.f64(f + offset) }
}

/// `0x141a6ace0(F, x, y, z)`: sets the aircraft's position (doubles `F+0x378/0x380/0x388`; a component whose
/// float32 value is not finite becomes zero) and refreshes the ground probe state at it. With the speed over the
/// frame time and the length of the vector `B+0x64f4..0x64fc` it derives the probe lengths `F+0x42f50`
/// (`0.2 |b| + min(|b|, speed * dt)`), `F+0x42f54` (`2 |b|`) and `F+0x42f58`
/// (`1.01 (|b| + F+0x42f50)`), then queries the terrain object at `F+0x42e40` with a segment from `(x, y -
/// F+0x42f54, z)` to `(x, y + F+0x42f50, z)` (the terrain functions `0x141963d30`, `0x141962b00`, `0x14195f4b0` and
/// `0x141962750` are replayed): the ground height `F+0x42f5c` comes from the probe, or from the cached plane
/// (`F+0x42f60..0x42f74`, the slope terms) when the probe reports a cached hit. `rbp` locates the original's frame
/// locals, whose addresses are passed to the terrain functions.
#[allow(clippy::field_reassign_with_default)]
pub fn set_position(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64, position: [f64; 3]) {
    use crate::scalar::snap;
    let slot = |off: i64| rbp.wrapping_add(off as u64);
    let finite_f32 = |v: f64| (v as f32).is_finite();
    let x = if finite_f32(position[0]) {
        position[0]
    } else {
        0.0
    };
    let y = if finite_f32(position[1]) {
        position[1]
    } else {
        0.0
    };
    vm.set_f64(slot(0xd8), position[2]);
    if !finite_f32(position[2]) {
        vm.set_f64(slot(0xd8), 0.0);
    }
    vm.set_f64(f + 0x378, x);
    vm.set_f64(f + 0x380, y);
    let z = vm.f64(slot(0xd8));
    vm.set_f64(f + 0x388, z);
    let b = vm.u64(f + 0x20);
    let (w0, w1, w2) = (vm.f32(b + 0x64f4), vm.f32(b + 0x64f8), vm.f32(b + 0x64fc));
    let (v1, v2, v3) = (vm.f32(f + 0x368), vm.f32(f + 0x36c), vm.f32(f + 0x370));
    let dt = f64::from_bits(env.call(vm, 0x140c448c0, CallArgs::default()).xmm0);
    let root = |sum: f32| if 0.0 > sum { f32::NAN } else { sum.sqrt() };
    let speed = root(v1 * v1 + v2 * v2 + v3 * v3);
    let travelled = (dt * f64::from(speed)) as f32;
    let reach = if 0.0 > travelled {
        0.0
    } else {
        let norm = root(w0 * w0 + w1 * w1 + w2 * w2);
        if norm < travelled { norm } else { travelled }
    };
    let norm = root(w0 * w0 + w1 * w1 + w2 * w2);
    let f50 = (f64::from(norm) * 0.2 + f64::from(reach)) as f32;
    vm.set_f32(f + 0x42f50, f50);
    vm.set_f32(f + 0x42f54, (f64::from(norm) + f64::from(norm)) as f32);
    vm.set_f32(f + 0x42f58, (f64::from(norm + f50) * 1.01) as f32);
    let component =
        |vm: &mut Vm, env: &mut dyn Callees, offset: u64| position_component(vm, env, f, offset);
    let up = vm.f32(f + 0x42f50);
    let p70 = component(vm, env, 0x378) as f32;
    let p6c = (f64::from(up) + component(vm, env, 0x380)) as f32;
    let p68 = component(vm, env, 0x388) as f32;
    let p80 = component(vm, env, 0x378) as f32;
    let down = vm.f32(f + 0x42f54);
    let p7c = (component(vm, env, 0x380) - f64::from(down)) as f32;
    let p78 = component(vm, env, 0x388) as f32;
    for (off, v) in [
        (-0x70, p70),
        (-0x6c, p6c),
        (-0x68, p68),
        (-0x80, p80),
        (-0x7c, p7c),
        (-0x78, p78),
    ] {
        vm.set_f32(slot(off), v);
    }
    vm.set_i32(slot(-0x64), 0);
    vm.set_i32(slot(-0x74), 0);
    let terrain = f + 0x42e40;
    let (r12, r14, r15, r78) = (f + 0x42f60, f + 0x42f6c, f + 0x42f84, f + 0x42f78);
    let ground = f + 0x42f5c;
    let (p70_at, p80_at) = (slot(-0x70), slot(-0x80));
    let probe = |vm: &mut Vm, env: &mut dyn Callees, tail: [u64; 3]| -> u32 {
        let mut args = CallArgs::default();
        args.int = [Some(terrain), Some(p70_at), Some(p80_at), Some(ground)];
        args.stack = [Some(tail[0]), Some(tail[1]), Some(tail[2]), None];
        env.call(vm, 0x14195f4b0, args).rax as u32
    };
    if vm.i32(f + 0x42f88) != 0 {
        let mut args = CallArgs::default();
        args.int = [Some(terrain), Some(p80_at), Some(p70_at), None];
        args.xmm[3] = Some(vm.f32(f + 0x42f58).to_bits());
        args.stack = [
            Some(r12),
            Some(r14),
            Some(r78),
            Some(u64::from(vm.u32(r15))),
        ];
        env.call(vm, 0x141963d30, args);
    } else {
        let mut args = CallArgs::default();
        args.int[0] = Some(terrain);
        args.stack[0] = Some(u64::from(vm.u32(f + 0x28)));
        env.call(vm, 0x141962b00, args);
    }
    if vm.i32(f + 0x42f88) != 0 {
        if probe(vm, env, [0, 0, 0]) != 0 {
            return;
        }
        let v = snap(vm.f32(f + 0x42f70), f32::from_bits(0xbc23_d70a), 0.01);
        let dz = f64::from(vm.f32(f + 0x42f68)) - position_component(vm, env, f, 0x388);
        let t7 = f64::from(vm.f32(f + 0x42f74)) * dz;
        let dx = f64::from(vm.f32(r12)) - position_component(vm, env, f, 0x378);
        let t2 = f64::from(vm.f32(r14)) * dx + t7;
        let h = t2 / f64::from(v) + f64::from(vm.f32(f + 0x42f64));
        vm.set_f32(ground, h as f32);
    } else {
        vm.set_f32(ground, -500.0);
        vm.set_i32(r15, -1);
        let x0 = position_component(vm, env, f, 0x378) as f32;
        vm.set_f32(r12, x0);
        let g = vm.u32(ground);
        vm.set_u32(f + 0x42f64, g);
        let z0 = position_component(vm, env, f, 0x388) as f32;
        vm.set_f32(f + 0x42f68, z0);
        vm.set_i32(r14, 0);
        vm.set_u32(f + 0x42f70, 0x3f80_0000);
        vm.set_u32(f + 0x42f74, 0);
        vm.set_i32(r78, 0);
        vm.set_u32(f + 0x42f7c, 0);
        vm.set_u32(f + 0x42f80, 0);
        if probe(vm, env, [r14, r78, r15]) != 0 {
            return;
        }
        let cell = vm.u32(f + 0x28);
        let zf = position_component(vm, env, f, 0x388) as f32;
        let xf = position_component(vm, env, f, 0x378) as f32;
        let mut args = CallArgs::default();
        args.int = [
            Some(0x1_4611_ac80),
            None,
            None,
            Some(u64::from(cell.wrapping_add(0xc))),
        ];
        args.xmm[1] = Some(xf.to_bits());
        args.xmm[2] = Some(zf.to_bits());
        args.stack = [Some(r14), Some(r15), None, None];
        let h = f32::from_bits(env.call(vm, 0x141962750, args).xmm0 as u32);
        vm.set_f32(ground, h);
    }
}

/// `0x1411d9e50(F, i)`: whether the `i`-th entry of the record list at `F+0x69b8` (16-byte entries) is live: it
/// must differ from the empty key held in the globals `0x142f03778/0x142f03780` and be accepted by the lookup
/// `0x140f5c540` on the singleton at `0x14578b780` (replayed).
pub(crate) fn record_live(vm: &mut Vm, env: &mut dyn Callees, f: u64, index: i32) -> bool {
    let begin = vm.u64(f + 0x69b8);
    let count = vm.u64(f + 0x69c0).wrapping_sub(begin) as i64 >> 4;
    if (i64::from(index) as u64) >= count as u64 {
        return false;
    }
    let entry = begin + (i64::from(index) as u64) * 16;
    let empty =
        vm.u32(entry) == vm.u32(0x1_42f0_3778) && vm.u64(entry + 8) == vm.u64(0x1_42f0_3780);
    if empty {
        return false;
    }
    env.call(vm, 0x140f5c540, CallArgs::ints(&[0x1_4578_b780]))
        .rax as u8
        != 0
}

/// `update_flight` `0x141271fa2..0x14127223b`: advances the state. The position is advanced by
/// `dt * velocity * scale` (the double at `0x142f01898`; three separate frame-time queries) through
/// [`set_position`]; the rates `F+0x3cc/0x3d0/0x3d4` times the frame time rotate the quaternion at `F+0x3e4`
/// ([`integrate_attitude`](crate::attitude::integrate_attitude)), giving the Euler angles `F+0x3e0/0x3dc/0x3d8` whose
/// sines and cosines fill the frame pairs `F+0x430/0x434`, `0x440/0x444`, `0x450/0x454`; every live entry of the
/// record list `F+0x69b8` is advanced by the frame time through the singleton (`0x140f42620`, `0x140f3c2d0`,
/// replayed). `rbp` is the frame of `update_flight` (the entry copy lives at `rbp`), `callee_frame` the frame of
/// [`set_position`] when called from here.
pub fn integrate_motion(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64, callee_frame: u64) {
    use crate::attitude::integrate_attitude;
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    let frame_time = |vm: &mut Vm, env: &mut dyn Callees| {
        f64::from_bits(env.call(vm, 0x140c448c0, CallArgs::default()).xmm0)
    };
    let scale = vm.f64(0x1_42f0_1898);
    let z0 = position_component(vm, env, f, 0x388);
    let dt = frame_time(vm, env);
    let z = dt * f64::from(vm.f32(f + 0x370)) * scale + z0;
    let y0 = position_component(vm, env, f, 0x380);
    let dt = frame_time(vm, env);
    let y = dt * f64::from(vm.f32(f + 0x36c)) * scale + y0;
    let x0 = position_component(vm, env, f, 0x378);
    let dt = frame_time(vm, env);
    let x = dt * f64::from(vm.f32(f + 0x368)) * scale + x0;
    set_position(vm, env, f, callee_frame, [x, y, z]);
    let dt = frame_time(vm, env);
    let w3 = (dt * f64::from(vm.f32(f + 0x3d4))) as f32;
    let dt = frame_time(vm, env);
    let w2 = (dt * f64::from(vm.f32(f + 0x3d0))) as f32;
    let dt = frame_time(vm, env);
    let w1 = (dt * f64::from(vm.f32(f + 0x3cc))) as f32;
    integrate_attitude(vm, [w1, w2, w3], f + 0x3e4, f + 0x3e0, f + 0x3dc, f + 0x3d8);
    for (angle, pair) in [(0x3dc, 0x430), (0x3e0, 0x440), (0x3d8, 0x450)] {
        let a = vm.f32(f + angle) * RAD;
        vm.set_f32(f + pair, a.sin());
        vm.set_f32(f + pair + 4, a.cos());
    }
    let mut i = 0i32;
    let count = |vm: &Vm| vm.u64(f + 0x69c0).wrapping_sub(vm.u64(f + 0x69b8)) as i64 >> 4;
    if count(vm) != 0 {
        loop {
            if record_live(vm, env, f, i) {
                let entry = vm.u64(f + 0x69b8) + (i as u64) * 16;
                for k in 0..4 {
                    let word = vm.u32(entry + 4 * k);
                    vm.set_u32(rbp + 4 * k, word);
                }
                let mut args = CallArgs::ints(&[0x1_4578_b780]);
                args.int[1] = None;
                let object = env.call(vm, 0x140f42620, args).rax;
                let dt = frame_time(vm, env) as f32;
                let mut args = CallArgs::ints(&[object]);
                args.xmm[1] = Some(dt.to_bits());
                env.call(vm, 0x140f3c2d0, args);
            }
            i += 1;
            if (i64::from(i) as u64) >= count(vm) as u64 {
                break;
            }
        }
    }
}

/// `update_flight` `0x14127223b..0x141272395`: the velocities in the aircraft axes. The world velocity
/// `F+0x368/0x36c/0x370` moved into the aircraft frame (`0x141296750`, no origin shift) is stored at
/// `F+0x2a0/0x2ac/0x2b8`; the air velocity at the centre of gravity (`0x14121b580` at the origin, with the wash
/// switched off; replayed) lands in `F+0x29c/0x2a8/0x2b4`; its direction angles (`0x141183bf0`, radians) are stored at
/// `F+0x404/0x408/0x40c` and its speed at `F+0x400`, and the three angles are then converted to degrees and scaled
/// by the speed held to `0..1`.
pub fn body_velocities(vm: &mut Vm, env: &mut dyn Callees, f: u64) {
    use crate::callees::direction_angles;
    use crate::transform::{Frame, to_aircraft_frame};
    let frame = Frame {
        origin: [vm.f64(f + 0x378), vm.f64(f + 0x380), vm.f64(f + 0x388)],
        rotation: [
            [vm.f32(f + 0x430), vm.f32(f + 0x434)],
            [vm.f32(f + 0x440), vm.f32(f + 0x444)],
            [vm.f32(f + 0x450), vm.f32(f + 0x454)],
        ],
    };
    let velocity = [vm.f32(f + 0x368), vm.f32(f + 0x36c), vm.f32(f + 0x370)];
    let local = to_aircraft_frame(&frame, velocity, false, false);
    for (offset, v) in [(0x2a0, local[0]), (0x2ac, local[1]), (0x2b8, local[2])] {
        vm.set_f32(f + offset, v);
    }
    let mut args = CallArgs::ints(&[f, 0, f + 0x29c]);
    args.int[1] = None;
    args.xmm[1] = Some(0);
    args.xmm[3] = Some(0);
    args.stack[0] = Some(f + 0x2a8);
    args.stack[1] = None;
    args.stack[2] = Some(f + 0x2b4);
    env.call(vm, 0x14121b580, args);
    let air = [vm.f32(f + 0x29c), vm.f32(f + 0x2a8), vm.f32(f + 0x2b4)];
    let [roll, pitch, yaw, speed] = direction_angles(air[0], air[1], air[2]);
    vm.set_f32(f + 0x404, roll);
    vm.set_f32(f + 0x408, pitch);
    vm.set_f32(f + 0x40c, yaw);
    vm.set_f32(f + 0x400, speed);
    let factor = crate::scalar::clamp(speed, 0.0, 1.0);
    for offset in [0x404, 0x408, 0x40c] {
        let scaled = factor * f32::from_bits(0x4265_2ee0);
        vm.set_f32(f + offset, scaled * vm.f32(f + offset));
    }
}

/// `update_flight` `0x141272395..0x1412728f7`: the geographic state. With the planet object (`0x14193ae40`, a
/// function-local static; replayed) the position becomes the double triple `F+0x390/0x398/0x3a0`
/// (`0x1406eaf20`, replayed), the local-axes matrix for that position (`0x1419f7ee0`, replayed, three rows of three
/// floats at `rbp+0x15a0` with a 16-byte stride) turns the world velocity into `F+0x3f4/0x3f8/0x3fc`, the matrix
/// then gives the Euler angles `F+0x358/0x350/0x348` (`0x1419f6fd0`, replayed, seeded with `F+0x3e0/0x3dc/0x3d8`)
/// whose change over the frame time (in radians per second) is stored at `F+0x438/0x448/0x458` and smoothed into the
/// accelerations `F+0x43c/0x44c/0x45c` (`lerp` with the factor `20 dt`); finally the position goes through the
/// planet object's 4 x 4 double matrix into `F+0x3a8/0x3b0/0x3b8`.
pub fn geodetic_state(vm: &mut Vm, env: &mut dyn Callees, f: u64, rbp: u64) {
    use crate::scalar::lerp;
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    let frame_time = |vm: &mut Vm, env: &mut dyn Callees| {
        f64::from_bits(env.call(vm, 0x140c448c0, CallArgs::default()).xmm0)
    };
    let planet =
        |vm: &mut Vm, env: &mut dyn Callees| env.call(vm, 0x14193ae40, CallArgs::default()).rax;
    let ctx = planet(vm, env);
    let z = position_component(vm, env, f, 0x388);
    let y = position_component(vm, env, f, 0x380);
    let x = position_component(vm, env, f, 0x378);
    crate::flight_state::local_to_geodetic(
        vm,
        env,
        ctx,
        [f + 0x390, f + 0x398, f + 0x3a0],
        [x, y, z],
    );
    let ctx2 = planet(vm, env);
    let a398 = position_component(vm, env, f, 0x398);
    let a390 = position_component(vm, env, f, 0x390);
    let matrix = rbp + 0x15a0;
    let mut args = CallArgs::ints(&[ctx2, matrix]);
    args.xmm[2] = Some(a390.to_bits() as u32);
    args.xmm[3] = Some(a398.to_bits() as u32);
    env.call(vm, 0x1419f7ee0, args);
    let m = |vm: &Vm, row: u64, col: u64| vm.f32(matrix + 0x10 * row + 4 * col);
    let (v0, v1, v2) = (vm.f32(f + 0x368), vm.f32(f + 0x36c), vm.f32(f + 0x370));
    for (offset, col) in [(0x3f4, 0), (0x3f8, 1), (0x3fc, 2)] {
        let value = m(vm, 1, col) * v1 + m(vm, 0, col) * v0 + m(vm, 2, col) * v2;
        vm.set_f32(f + offset, value);
    }
    let (old350, old358, old348) = (vm.f32(f + 0x350), vm.f32(f + 0x358), vm.f32(f + 0x348));
    let (old438, old448, old458) = (vm.f32(f + 0x438), vm.f32(f + 0x448), vm.f32(f + 0x458));
    for (to, from) in [(0x358, 0x3e0), (0x350, 0x3dc), (0x348, 0x3d8)] {
        let word = vm.u32(f + from);
        vm.set_u32(f + to, word);
    }
    let args = CallArgs::ints(&[matrix, f + 0x358, f + 0x350, f + 0x348]);
    env.call(vm, 0x1419f6fd0, args);
    for (target, angle, old) in [
        (0x438, 0x350, old350),
        (0x448, 0x358, old358),
        (0x458, 0x348, old348),
    ] {
        let dt = frame_time(vm, env);
        let inverse = (1.0 / dt) as f32;
        let rate = (vm.f32(f + angle) - old) * RAD * inverse;
        vm.set_f32(f + target, rate);
    }
    for (acceleration, rate, old) in [
        (0x43c, 0x438, old438),
        (0x44c, 0x448, old448),
        (0x45c, 0x458, old458),
    ] {
        let dt = frame_time(vm, env);
        let factor = (dt * 20.0) as f32;
        let dt = frame_time(vm, env);
        let inverse = (1.0 / dt) as f32;
        let delta = (vm.f32(f + rate) - old) * inverse;
        let smoothed = lerp(vm.f32(f + acceleration), delta, factor);
        vm.set_f32(f + acceleration, smoothed);
    }
    let ctx3 = planet(vm, env);
    let pz = position_component(vm, env, f, 0x388);
    let py = position_component(vm, env, f, 0x380);
    let px = position_component(vm, env, f, 0x378);
    let c = |vm: &Vm, offset: u64| vm.f64(ctx3 + offset);
    vm.set_f64(
        f + 0x3a8,
        ((py * c(vm, 0x220) + px * c(vm, 0x200)) + pz * c(vm, 0x240)) + c(vm, 0x260),
    );
    vm.set_f64(
        f + 0x3b0,
        ((c(vm, 0x228) * py + c(vm, 0x208) * px) + c(vm, 0x248) * pz) + c(vm, 0x268),
    );
    vm.set_f64(
        f + 0x3b8,
        ((px * c(vm, 0x210) + py * c(vm, 0x230)) + c(vm, 0x250) * pz) + c(vm, 0x270),
    );
}

/// `update_flight` `0x1412728f7..0x141272a96`: the flight angles from the local velocity `F+0x3f4/0x3f8/0x3fc`:
/// the flight path angle `F+0x414 = atan(F+0x3f8 / max(sqrt(F+0x3f4^2 + F+0x3fc^2), 0.01))` and the track
/// `F+0x410 = atan2(F+0x3f4, -F+0x3fc)` in degrees (`0..=360`), the track of the world velocity
/// `F+0x418 = atan2(F+0x368, -F+0x370)` in degrees (`0..=360`), and the sine and cosine of the angles
/// `F+0x404` and `F+0x408` (taken as degrees) at `F+0x460/0x464` and `F+0x468/0x46c`.
pub fn flight_angles(vm: &mut Vm, f: u64) {
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    const DEG: f32 = f32::from_bits(0x4265_2ee0);
    let wrap = |mut a: f32| {
        while 0.0 > a {
            a += 360.0;
        }
        while a > 360.0 {
            a += -360.0;
        }
        a
    };
    let neg_z = -vm.f32(f + 0x3fc);
    let east = vm.f32(f + 0x3f4);
    let sum = east * east + neg_z * neg_z;
    let horizontal = if 0.0 > sum { f32::NAN } else { sum.sqrt() };
    let held = if horizontal > 0.01 { horizontal } else { 0.01 };
    vm.set_f32(f + 0x414, (vm.f32(f + 0x3f8) / held).atan() * DEG);
    vm.set_f32(f + 0x410, wrap(east.atan2(neg_z) * DEG));
    let (v0, v2) = (vm.f32(f + 0x368), vm.f32(f + 0x370));
    vm.set_f32(f + 0x418, wrap(v0.atan2(-v2) * DEG));
    for (angle, pair) in [(0x404, 0x460), (0x408, 0x468)] {
        let a = vm.f32(f + angle) * RAD;
        vm.set_f32(f + pair, a.sin());
        vm.set_f32(f + pair + 4, a.cos());
    }
}

/// `0x1412763c0(F)`: the atmosphere of the step at the aircraft's altitude `F+0x3a0` (zero when the engine flag
/// `0x1417f12c0` is set): the gravity `F+0x78` (`GM / (r + h)^2`), the temperature `F+0x5c` (`0x141ba6750`), the
/// offsets from the standard profile `F+0x58` (`0x141ba6290`) and `F+0x60` (the table temperature), the density
/// ratio and pressure `F+0x6c/0x68`, `F+0x70` (the density over 1.225), the speed of sound `F+0x74` and the total
/// temperature `F+0x64`. The accessors of the weather object at `F+0xbfa8` are called through the environment.
pub fn atmosphere_step(vm: &mut Vm, env: &mut dyn Callees, f: u64) {
    let object = f + 0xbfa8;
    let altitude = |vm: &mut Vm, env: &mut dyn Callees| -> f64 {
        let flag = env
            .call(vm, 0x1417f12c0, CallArgs::ints(&[0x1424_f5648]))
            .rax as u8
            != 0;
        if flag { 0.0 } else { vm.f64(f + 0x3a0) }
    };
    let accessor = |vm: &mut Vm, env: &mut dyn Callees, address: u64, value: f32| -> f32 {
        let mut args = CallArgs::ints(&[object]);
        args.xmm[1] = Some(value.to_bits());
        f32::from_bits(env.call(vm, address, args).xmm0 as u32)
    };
    let h = altitude(vm, env);
    let r = (h + 6378145.0) as f32;
    vm.set_f32(f + 0x78, (398601200000000.0 / f64::from(r * r)) as f32);
    let a = altitude(vm, env);
    let temperature = accessor(vm, env, 0x141ba6750, a as f32);
    vm.set_f32(f + 0x5c, temperature);
    let a8 = altitude(vm, env);
    let a = altitude(vm, env);
    let t2 = accessor(vm, env, 0x141ba6750, a as f32);
    let offset = accessor(vm, env, 0x141ba6290, a8 as f32);
    vm.set_f32(f + 0x58, t2 - offset);
    // the table temperature at the altitude (the second float of the entries at 0x14612bd90)
    let flag = env
        .call(vm, 0x1417f12c0, CallArgs::ints(&[0x1424_f5648]))
        .rax as u8
        != 0;
    let probe = if flag { 0.0 } else { vm.f64(f + 0x3a0) };
    let position = ((f64::from(probe as f32) + 5000.0) / 100.0) as f32;
    // `cvttss2si`: out-of-range and NaN positions give `i32::MIN`, which the original's sign test sends to 0
    let truncated = if (-2147483648.0..2147483648.0).contains(&position) {
        position as i32
    } else {
        i32::MIN
    };
    let index = if truncated < 0 {
        0
    } else {
        truncated.min(0x801)
    };
    let entry = |i: i32| vm.f32(0x1_4612_bd90 + 8 * i as u64 + 4);
    let table = interpolate_clamped(
        index as f32,
        entry(index),
        (index + 1) as f32,
        entry(index + 1),
        position,
    );
    vm.set_f32(f + 0x60, vm.f32(f + 0x5c) - table);
    let a6 = altitude(vm, env);
    let a7 = altitude(vm, env);
    let scale = accessor(vm, env, 0x141ba6df0, a6 as f32);
    let d = accessor(
        vm,
        env,
        0x141ba63a0,
        (f64::from(scale * 0.3048f32) + a7) as f32,
    );
    let kelvin = vm.f32(f + 0x5c) + 273.15f32;
    vm.set_f32(f + 0x6c, d);
    vm.set_f32(f + 0x70, d / f32::from_bits(0x3f9ccccd));
    vm.set_f32(f + 0x68, d * f32::from_bits(0x438f86c9) * kelvin);
    let tk = f64::from(kelvin);
    let sqrt = tk * f64::from_bits(0x40791dfcc6666666);
    vm.set_f32(
        f + 0x74,
        (if sqrt < 0.0 { f64::NAN } else { sqrt.sqrt() }) as f32,
    );
    let mach = vm.f32(f + 0x420);
    let total = (f64::from(mach * mach) * 0.2 + 1.0) * tk - f64::from_bits(0x4071126660000000);
    vm.set_f32(f + 0x64, total as f32);
}

/// `0x14121a9b0(W, X, &list_a, &list_b, log)`: the wing-chain factor of the wing state `X`: over the wings of the
/// list `a` (records `W_k`) and the matching state lists `b`, every element contributes the weight
/// `s^2 |f|` (its relative speed `X_k+0x54+4e` squared and the factor at `+0xf4+4e`) and the ratio of its distance
/// from the wing's reference point `(W_k+0x36b0, +0x36b4)` to the distance from `(+0x36bc, +0x36c0)`; the clamped
/// mean ratio `r` gives `X+0 = 2 (1 - r)` held to 0.5..2. The aerodynamic state fields `X+0x25c..0x294` are cleared
/// and `X+0x288/0x28c/0x290` set to 1; the two lists are released (their begin, end and capacity words zeroed).
pub fn wing_chain_factor(vm: &mut Vm, x: u64, list_a: u64, list_b: u64) {
    for off in (0x25c..0x294).step_by(4) {
        vm.set_u32(x + off, 0);
    }
    for off in [0x288, 0x28c, 0x290] {
        vm.set_f32(x + off, 1.0);
    }
    let (mut sum_dist, mut sum_weight) = (0.005f32, 0.01f32);
    let begin = vm.u64(list_a);
    let count = (vm.u64(list_a + 8).wrapping_sub(begin) as i64) >> 3;
    let b_begin = vm.u64(list_b);
    for k in 0..count {
        let wk = vm.u64(begin + 8 * k as u64);
        let xk = vm.u64(b_begin + 8 * k as u64);
        let elements = vm.i32(wk + 4);
        let [ax, ay, _, _] = boundary_arrays(vm, wk, elements);
        for e in 0..elements.max(0) {
            let cell = xk + 0x54 + 4 * e as u64;
            let weight = vm.f32(cell) * vm.f32(cell) * vm.f32(cell + 0xa0).abs();
            let u = (f64::from(e as f32) + 0.5) as f32;
            let dy = crate::wing_element::boundary_at(&ay, elements, u) - vm.f32(wk + 0x36b4);
            let dx = crate::wing_element::boundary_at(&ax, elements, u) - vm.f32(wk + 0x36b0);
            let near = {
                let sum = dy * dy + dx * dx;
                if 0.0 > sum { f32::NAN } else { sum.sqrt() }
            };
            let far = {
                let ey = vm.f32(wk + 0x36c0) - vm.f32(wk + 0x36b4);
                let ex = vm.f32(wk + 0x36bc) - vm.f32(wk + 0x36b0);
                let sum = ex * ex + ey * ey;
                if 0.0 > sum { f32::NAN } else { sum.sqrt() }
            };
            sum_weight += weight;
            sum_dist += near * weight / far;
        }
    }
    let ratio = sum_dist / sum_weight;
    let ratio = crate::scalar::clamp(ratio, 0.0, 1.0);
    let twice = (f64::from((1.0 - f64::from(ratio)) as f32) * 2.0) as f32;
    let value = crate::scalar::clamp(twice, 0.5, 2.0);
    vm.set_f32(x, value);
    for list in [list_a, list_b] {
        if vm.u64(list) != 0 {
            for off in [0, 8, 0x10] {
                vm.set_u64(list + off, 0);
            }
        }
    }
}
