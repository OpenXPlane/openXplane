//! The ground contact of a landing-gear strut or a body point (`0x1411c7a50`): the point is placed in the world,
//! a segment through the point is tested against the terrain mesh, and a hit gives the contact force (a spring
//! and a damper along the surface normal plus the tire friction), the tire motion and the plug-in force.

use crate::flight_state::add_plugin_force;
use crate::flight_step::position_component;
use crate::terrain::{SegmentOut, segment_probe};
use crate::tire::{Tire, slip_limit, tire_motion};
use crate::transform::{Frame, to_aircraft_frame};
use crate::vm::{CallArgs, Callees, Vm};

/// The scratch frame standing in for the original's locals (offsets as in the original's frame).
const FRAME: u64 = 0x7e00_0300_0000;
/// `0x14611ac80`: the surface table (records of 36 bytes behind `+8`).
const SURFACES: u64 = 0x1_4611_ac80;
/// The recorder vector (`begin`, `end`, `capacity` words) and the aircraft index that is recorded.
const RECORDER: u64 = 0x1_4612_5768;
const RECORDED_AIRCRAFT: u64 = 0x1_42f2_e3dc;

/// The arguments of `0x1411c7a50`.
#[derive(Clone, Copy, Debug)]
pub struct Strut {
    /// The wheel index (`edx`, negative for a body point that has no wheel record).
    pub wheel: i32,
    /// `xmm2`, `xmm3` and the stack floats 5..10: the arm in the aircraft frame (`x, a4`), the offsets
    /// `a5 .. a8`, `a9` and the damping `a10`.
    pub x: f32,
    pub a4: f32,
    pub a5: f32,
    pub a6: f32,
    pub a7: f32,
    pub a8: f32,
    pub a9: f32,
    pub a10: f32,
    /// The two float outputs (addresses, may be zero): the speed over the ground and the contact force.
    pub speed_out: u64,
    pub force_out: u64,
}

fn clamp_between(lo: f32, v: f32) -> f32 {
    // `comiss v, lo; ja` keeps `lo`; otherwise `minss(1.0, v)`
    if lo > v {
        lo
    } else if 1.0 < v {
        1.0
    } else {
        v
    }
}

/// `0x1407d6c50(F, index)`: entry `index` of the vector at `F+0x6958` (entries of 0x90 bytes); out of range is the
/// original's exception.
fn wheel_entry(vm: &Vm, f: u64, index: i32) -> Result<u64, String> {
    let begin = vm.u64(f + 0x6958);
    let count = (vm.u64(f + 0x6960).wrapping_sub(begin) as i64) / 0x90;
    if index as i64 as u64 >= count as u64 {
        return Err("wheel index out of range".into());
    }
    Ok(begin + 0x90 * index as i64 as u64)
}

/// Returns whether the strut touches the ground (the original's `al`).
pub fn strut_contact(
    vm: &mut Vm,
    env: &mut dyn Callees,
    f: u64,
    s: &Strut,
) -> Result<bool, String> {
    let m = |vm: &Vm, o: u64| vm.f32(f + o);
    let md = |vm: &Vm, o: u64| f64::from(vm.f32(f + o));
    let (f450, f454, f430, f434, f440, f444) = (
        md(vm, 0x450),
        md(vm, 0x454),
        md(vm, 0x430),
        md(vm, 0x434),
        md(vm, 0x440),
        md(vm, 0x444),
    );
    let (x, a5d, a8d) = (f64::from(s.x), f64::from(s.a5), f64::from(s.a8));
    // the foot point rotated into the world frame and moved by the position doubles
    let ma = f454 * x + f450 * a5d;
    let mb = f454 * a5d - f450 * x;
    let mc = mb * f430 + f434 * a8d;
    let wz = mc * f444 + ma * f440;
    let wx = ma * f444 - mc * f440;
    let wy = mb * f434 - f430 * a8d;
    let px = wx + position_component(vm, env, f, 0x378);
    let py = wy + position_component(vm, env, f, 0x380);
    let pz = wz + position_component(vm, env, f, 0x388);
    // the second end of the probe segment: the offset (a4, a6, a9) rotated the same way
    let (a4d, a6d, a9d) = (f64::from(s.a4), f64::from(s.a6), f64::from(s.a9));
    let na = f454 * a4d + f450 * a6d;
    let nb = f454 * a6d - f450 * a4d;
    let nc = f434 * a9d + nb * f430;
    let ny = nb * f434 - f430 * a9d;
    let nx = na * f444 - nc * f440;
    let nz = nc * f444 + na * f440;
    let (p0, p1) = (FRAME + 0x8, FRAME - 0x8);
    for (k, v) in [nx + px, ny + py, nz + pz].into_iter().enumerate() {
        vm.set_f32(p0 + 4 * k as u64, v as f32);
    }
    for (k, v) in [px, py, pz].into_iter().enumerate() {
        vm.set_f32(p1 + 4 * k as u64, v as f32);
    }
    let (out, normal, moved, id) = (FRAME - 0x48, FRAME - 0x20, FRAME - 0x68, FRAME + 0x158);
    for off in [
        out,
        out + 4,
        out + 8,
        normal,
        normal + 4,
        normal + 8,
        moved,
        moved + 4,
        moved + 8,
        id,
    ] {
        vm.set_u32(off, 0);
    }
    let hit = segment_probe(
        vm,
        env,
        f + 0x42e40,
        p0,
        p1,
        SegmentOut {
            point: out,
            normal,
            moved,
            id,
        },
    );
    if !hit {
        return Ok(false);
    }
    let depth = (f64::from(vm.f32(out + 4)) - py) as f32;
    if 0.0 >= depth {
        return Ok(false);
    }
    // the velocity of the contact relative to the surface
    let (w3, w5, w2) = (m(vm, 0x3cc), m(vm, 0x3d4), m(vm, 0x3d0));
    let g1 = w2 * s.a5 + w5 * s.x;
    let g2 = (-s.x) * w3 - w2 * s.a8;
    let g3 = w3 * s.a5 - w5 * s.a8;
    let (g1, g2, g3) = (f64::from(g1), f64::from(g2), f64::from(g3));
    let ha = f454 * g2 - f450 * g3;
    let hb = f454 * g3 + f450 * g2;
    let hc = ha * f430 + f434 * g1;
    let u = m(vm, 0x368) - vm.f32(moved);
    let v = m(vm, 0x36c) - vm.f32(moved + 4);
    let w = m(vm, 0x370) - vm.f32(moved + 8);
    let vel_x = (hb * f444 - hc * f440) + f64::from(u);
    let vel_y = (ha * f434 - f430 * g1) + f64::from(v);
    let vel_z = (hc * f444 + hb * f440) + f64::from(w);
    let (nxf, nyf, nzf) = (vm.f32(normal), vm.f32(normal + 4), vm.f32(normal + 8));
    let vn_d = (f64::from(nxf) * (-vel_x) - f64::from(nyf) * vel_y) - f64::from(nzf) * vel_z;
    let vn = vn_d as f32;
    let b = vm.u64(f + 0x20);
    let spring = depth * s.a7 + vn * s.a10;
    if 0.0 > spring {
        return Ok(false);
    }
    let cap = {
        let v = ((f64::from(vm.f32(b + 0x2898) * f32::from_bits(0x411c_c5c1))) * 4.0) as f32;
        if v < spring { v } else { spring }
    };
    if 0.0 >= cap {
        return Ok(false);
    }
    // the surface row of the contact: the friction scale at `+0x20`
    let id_value = vm.u32(id);
    let row = vm.u64(SURFACES + 8).wrapping_add(36 * u64::from(id_value));
    let half = (f64::from(vm.f32(row + 0x20)) * 0.5) as f32;
    let (fx0, fy0, fz0) = (nxf * cap, nyf * cap, nzf * cap);
    // the wetness of the surface scales the friction
    let wet = |address: u64, scale: f32, floor: f32| {
        let w = vm.f32(address) - 0.0;
        let one = 1.0 - w * scale;
        clamp_between(floor, one)
    };
    let c15 = wet(
        0x1_42fe_8524,
        f32::from_bits(0x3f66_6666),
        f32::from_bits(0x3dcc_cccd),
    );
    let c8 = wet(
        0x1_42fe_8520,
        f32::from_bits(0x3f4c_cccd),
        f32::from_bits(0x3e4c_cccd),
    );
    let c14 = wet(
        0x1_42fe_851c,
        f32::from_bits(0x3f19_999a),
        f32::from_bits(0x3ecc_cccd),
    );
    let c7 = wet(
        0x1_42fe_8518,
        f32::from_bits(0x3ecc_cccc),
        f32::from_bits(0x3f19_999a),
    );
    let (vxf, vyf, vzf) = (vel_x as f32, vel_y as f32, vel_z as f32);
    let sum = (vxf * vxf + vyf * vyf) + vzf * vzf;
    let speed = if 0.0 > sum { f32::NAN } else { sum.sqrt() };
    let min = |a: f32, b: f32| if a < b { a } else { b };
    let m1 = min(c7, c14);
    let m2 = min(c8, c15);
    let held = if speed > f32::from_bits(0x3c23_d70a) {
        speed
    } else {
        f32::from_bits(0x3c23_d70a)
    };
    let scale = (min(m1, m2) * half) * cap;
    let limit = slip_limit(vm, env, vm.i32(f + 0x28), scale);
    let ratio_raw = held / limit;
    let ratio = ratio_raw.clamp(0.0, 1.0);
    let magnitude = f64::from(-(ratio * scale));
    let (hd, vyd, vzd) = (f64::from(held), vel_y, vel_z);
    let fx = (magnitude * vel_x / hd) as f32 + fx0;
    let fz = (magnitude * vzd / hd) as f32 + fz0;
    let fy = (magnitude * vyd / hd) as f32 + fy0;
    // the force into the aircraft frame
    let frame = Frame {
        origin: [vm.f64(f + 0x378), vm.f64(f + 0x380), vm.f64(f + 0x388)],
        rotation: [
            [m(vm, 0x430), m(vm, 0x434)],
            [m(vm, 0x440), m(vm, 0x444)],
            [m(vm, 0x450), m(vm, 0x454)],
        ],
    };
    let force = to_aircraft_frame(&frame, [fx, fy, fz], false, false);
    let (fa_addr, fb_addr, fc_addr) = (FRAME - 0x80, FRAME - 0x7c, FRAME - 0x78);
    for (k, v) in force.into_iter().enumerate() {
        vm.set_f32(fa_addr + 4 * k as u64, v);
    }
    if s.wheel >= 0 {
        let entry = wheel_entry(vm, f, s.wheel)?;
        let b = vm.u64(f + 0x20);
        let tire = Tire {
            record: entry,
            aircraft: vm.i32(f + 0x28),
            heading_offset: 0.0,
            scale: f64::from(vm.f32(b + 0x64d8)) * 0.01,
            reference_x: px,
            reference_vx: vel_x,
            vx: f64::from(vm.f32(moved)),
            force_a: fa_addr,
            mu: scale,
            limit: scale,
            reference_z: pz,
            reference_vz: vel_z,
            vz: f64::from(vm.f32(moved + 8)),
            force_b: fc_addr,
        };
        tire_motion(vm, env, &tire)?;
        let entry = wheel_entry(vm, f, s.wheel)?;
        vm.set_u32(entry + 0x78, id_value);
        vm.set_f32(entry + 0x2c, depth);
        vm.set_f32(entry + 0x30, vn);
        vm.set_f32(entry + 0x38, vm.f32(fa_addr));
        vm.set_f32(entry + 0x34, vm.f32(fb_addr));
        vm.set_f32(entry + 0x3c, vm.f32(fc_addr));
        vm.set_f32(entry + 0x58, ratio);
        vm.set_f32(entry + 0x5c, held);
    }
    let (ca, cb, cc) = (vm.f32(fa_addr), vm.f32(fb_addr), vm.f32(fc_addr));
    add_plugin_force(vm, env, f, s.x, ca, s.a5, cb, s.a8, cc)?;
    let sum = (cb * cb + ca * ca) + cc * cc;
    let magnitude_out = if 0.0 > sum { f32::NAN } else { sum.sqrt() };
    if s.speed_out != 0 {
        vm.set_f32(s.speed_out, speed);
    }
    if s.force_out != 0 {
        vm.set_f32(s.force_out, magnitude_out);
    }
    // the contact point in the aircraft frame, with the origin shift, joins the contact list of the object
    let mut shifted = [vm.f32(out), vm.f32(out + 4), vm.f32(out + 8)];
    for (k, offset) in [0x378u64, 0x380, 0x388].into_iter().enumerate() {
        let origin = position_component(vm, env, f, offset);
        shifted[k] = (f64::from(shifted[k]) - origin) as f32;
    }
    let local = to_aircraft_frame(&frame, shifted, false, false);
    let record = [
        id_value,
        local[0].to_bits(),
        local[1].to_bits(),
        local[2].to_bits(),
        speed.to_bits(),
        magnitude_out.to_bits(),
    ];
    let end = vm.u64(f + 0x430e0);
    if end != vm.u64(f + 0x430e8) {
        for (k, w) in record.iter().enumerate() {
            vm.set_u32(end + 4 * k as u64, *w);
        }
        vm.set_u64(f + 0x430e0, end + 0x18);
    } else {
        for (k, w) in record.iter().enumerate() {
            vm.set_u32(FRAME - 0x80 + 4 * k as u64, *w);
        }
        let args = CallArgs::ints(&[f + 0x430d8, end, FRAME - 0x80]);
        env.call(vm, 0x1405e73d0, args);
    }
    if vm.i32(f + 0x28) == vm.i32(RECORDED_AIRCRAFT) {
        let cursor = vm.u64(RECORDER + 8);
        let fields = [
            (0x14u64, s.x.to_bits()),
            (0x18, s.a5.to_bits()),
            (0x1c, s.a8.to_bits()),
            (0x2c, ca.to_bits()),
            (0x30, cb.to_bits()),
            (0x34, cc.to_bits()),
        ];
        if cursor != vm.u64(RECORDER + 16) {
            for off in (0..0x50).step_by(4) {
                vm.set_u32(cursor + off, 0);
            }
            for (off, w) in fields {
                vm.set_u32(cursor + off, w);
            }
            vm.set_u64(RECORDER + 8, cursor + 0x50);
        } else {
            let tmp = FRAME + 0x20;
            for off in (0..0x50).step_by(4) {
                vm.set_u32(tmp + off, 0);
            }
            for (off, w) in fields {
                vm.set_u32(tmp + off, w);
            }
            env.call(vm, 0x140f0f660, CallArgs::ints(&[RECORDER, cursor, tmp]));
        }
    }
    if vm.i32(f + 0x28) == 0 && vm.f64(0x1_42f0_1920) > 1.0 {
        let code = if vn > 4.0 {
            Some(0x3d)
        } else if vn > 3.0 {
            Some(0x3c)
        } else if vn > 2.0 {
            Some(0x3b)
        } else if vn > 1.0 {
            Some(0x3a)
        } else {
            None
        };
        if let Some(code) = code {
            let mut args = CallArgs::ints(&[0x1_4612_0f08, code]);
            args.xmm = [None, None, Some(1.0f32.to_bits()), Some(1.0f32.to_bits())];
            env.call(vm, 0x141a21200, args);
        }
    }
    Ok(depth > 0.0)
}
