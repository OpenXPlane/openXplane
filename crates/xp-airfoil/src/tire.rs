//! The motion of a tire contact point on the ground (`0x141182450`, with the limit `0x1412029d0`): the point
//! is carried along by the ground velocity, the surface friction pulls the aircraft back, and a point that grips
//! is snapped to its reference.

use crate::mass::{Balance, Sums, weight_and_balance};
use crate::vm::{CallArgs, Callees, Vm};

/// The table of flight objects (one per aircraft, `0x433c0` bytes each).
const FLIGHT_TABLE: u64 = 0x1_4612_43b0;
/// The table of aircraft objects (`0x6668` bytes each, `0x141a64260`).
const AIRCRAFT_TABLE: u64 = 0x1_4612_4398;
/// The frame time source `0x140c448c0(0x142f018b8)`.
const TIME_SOURCE: u64 = 0x1_42f0_18b8;
/// The scratch frame standing in for the locals of the original (`0x140 .. 0x1c8` of its frame).
const FRAME: u64 = 0x7e00_0200_0000;

/// The arguments of `0x141182450`.
#[derive(Clone, Copy, Debug)]
pub struct Tire {
    /// The wheel state record (`F+0x6958 + 0x90 i`): `+0x4c` flag, `+0x7c` gripping, doubles `+0x80`, `+0x88`.
    pub record: u64,
    /// The aircraft index (`F+0x28`).
    pub aircraft: i32,
    /// `xmm3`: added to the aircraft's heading before it is turned to radians.
    pub heading_offset: f64,
    /// Stack argument 5: the divisor scale (a quarter of it is used).
    pub scale: f64,
    /// Stack arguments 6 and 7: the reference point and the reference velocity of the first axis.
    pub reference_x: f64,
    pub reference_vx: f64,
    /// Stack argument 8: the velocity of the first axis (integrated into the record).
    pub vx: f64,
    /// Stack argument 9: float accumulator of the first force (`rbp-0x80` of the caller).
    pub force_a: u64,
    /// Stack argument 10: the friction coefficient.
    pub mu: f32,
    /// Stack argument 14: the force limit.
    pub limit: f32,
    /// Stack arguments 15 to 17: the reference point, the reference velocity and the velocity of the second axis.
    pub reference_z: f64,
    pub reference_vz: f64,
    pub vz: f64,
    /// Stack argument 18: float accumulator of the second force.
    pub force_b: u64,
}

pub(crate) fn aircraft_object(vm: &Vm, index: i32) -> u64 {
    vm.u64(FLIGHT_TABLE)
        .wrapping_add((i64::from(index) * 0x433c0) as u64)
}

pub(crate) fn time(vm: &mut Vm, env: &mut dyn Callees) -> f64 {
    f64::from_bits(
        env.call(vm, 0x140c448c0, CallArgs::ints(&[TIME_SOURCE]))
            .xmm0,
    )
}

/// `0x1412029d0(aircraft, x)`: the slip distance `max(dt * (|x| * B+0x2828 / F+0x288) * 2.5, 0.1)`.
pub fn slip_limit(vm: &mut Vm, env: &mut dyn Callees, aircraft: i32, x: f32) -> f32 {
    let f = aircraft_object(vm, aircraft);
    let b = vm
        .u64(AIRCRAFT_TABLE)
        .wrapping_add((i64::from(aircraft) * 0x6668) as u64);
    let t = x.abs() * vm.f32(b + 0x2828) / vm.f32(f + 0x288);
    let dt = time(vm, env);
    let v = (dt * f64::from(t)) as f32;
    let v = (f64::from(v) * 2.5) as f32;
    if v > 0.1 { v } else { 0.1 }
}

/// Moves the contact point of a tire. Debug markers (`0x140cbc430`) are not ported.
pub fn tire_motion(vm: &mut Vm, env: &mut dyn Callees, t: &Tire) -> Result<(), String> {
    let rec = t.record;
    let dt = time(vm, env);
    vm.set_f64(rec + 0x80, dt * t.vx + vm.f64(rec + 0x80));
    let dt = time(vm, env);
    vm.set_f64(rec + 0x88, dt * t.vz + vm.f64(rec + 0x88));
    let f = aircraft_object(vm, t.aircraft);
    let rad = 0.017_453_292_519_943_295f64;
    let k = f32::from_bits(0x3c8e_fa36);
    let yaw = (f64::from(vm.f32(f + 0x3e0)) + t.heading_offset) * rad;
    let pitch = f64::from(vm.f32(f + 0x3dc) * k);
    let roll = f64::from(vm.f32(f + 0x3d8) * k);
    let dz = vm.f64(rec + 0x88) - t.reference_z;
    let dx = vm.f64(rec + 0x80) - t.reference_x;
    let (ca, sa) = (yaw.cos(), yaw.sin());
    let q1 = dz * ca - dx * sa;
    let (cp, sp) = (pitch.cos(), pitch.sin());
    let vz = t.vz - t.reference_vz;
    let vx = t.vx - t.reference_vx;
    let q2 = vz * ca - vx * sa;
    let cp0 = cp * 0.0;
    let q4 = roll.cos() * (vx * ca + vz * sa);
    let sp0 = sp * 0.0;
    let q5 = q2 * sp + cp0;
    let q6 = q2 * cp - sp0;
    let q7 = q4 - roll.sin() * q5;
    let gripping = vm.i32(rec + 0x7c) != 0;
    let attached = vm.i32(rec + 0x4c);
    if !gripping {
        let speed = {
            let (a, b) = (q6 as f32, q7 as f32);
            let s = a * a + b * b;
            if 0.0 > s { f32::NAN } else { s.sqrt() }
        };
        let limit = slip_limit(vm, env, t.aircraft, t.mu);
        if limit > speed {
            let n = |x: u64| vm.f32(f + x);
            let (a0, a1, a2) = (
                n(0x368) + n(0x35c),
                n(0x370) + n(0x364),
                n(0x36c) + n(0x360),
            );
            let (b0, b1, b2) = (
                n(0x364) * 0.0 + n(0x370),
                n(0x360) * 0.0 + n(0x36c),
                n(0x35c) * 0.0 + n(0x368),
            );
            let first = {
                let s = ((a0 * a0) + (a2 * a2)) + (a1 * a1);
                if 0.0 > s { f32::NAN } else { s.sqrt() }
            };
            let second = {
                let s = (b2 * b2 + b1 * b1) + b0 * b0;
                if 0.0 > s { f32::NAN } else { s.sqrt() }
            };
            if second > first || attached != 0 {
                vm.set_f64(rec + 0x80, t.reference_x);
                vm.set_f64(rec + 0x88, t.reference_z);
                vm.set_i32(rec + 0x7c, 1);
            }
        }
        return Ok(());
    }
    // the grip: the displacement of the point turned into forces
    let divisor = t.scale * 0.25;
    let cr = roll.cos();
    let p1 = dx * ca + dz * sa;
    let force_a =
        (((cr * p1) - (sp * q1 + cp0) * roll.sin()) + q7 * 0.005) / divisor * f64::from(t.limit);
    let mult = f64::from(i32::from(attached != 0) + 1);
    let force_b = ((((cp * q1) - sp0) + q6 * 0.005) / divisor * f64::from(t.mu)) * mult;
    let (fa, fb) = (force_a as f32, force_b as f32);
    let lim = t.limit;
    let mut apply = true;
    let mut ok = false;
    if attached != 0 {
        let aircraft = aircraft_object(vm, t.aircraft);
        for off in [0x140, 0x168, 0x170, 0x178, 0x1b8, 0x1c0, 0x1c8] {
            vm.set_u32(FRAME + off, 0);
        }
        let wb = Balance {
            sums: Sums {
                total: FRAME + 0x168,
                inertia: [FRAME + 0x140, FRAME + 0x1c0, FRAME + 0x178],
                moments: [FRAME + 0x170, FRAME + 0x1c8, FRAME + 0x1b8],
                weights: [0.0, 1.0, 0.0],
            },
            stop: false,
        };
        weight_and_balance(vm, env, aircraft, &wb)?;
        let s = fb * fb + fa * fa;
        let root = if 0.0 > s { f32::NAN } else { s.sqrt() };
        let ratio = (root / vm.f32(FRAME + 0x168)).abs();
        // `comiss`/`jbe`: an unordered ratio also takes the branch
        ok = ratio <= 1.0 || ratio.is_nan();
    }
    if !ok {
        if fa.abs() > lim {
            apply = false;
        } else {
            let wide = (f64::from(t.mu) * 1.2) as f32;
            let cap = if 0.0 > wide {
                0.0
            } else if lim < wide {
                lim
            } else {
                wide
            };
            if fb.abs() > cap {
                apply = false;
            }
        }
    }
    if !apply {
        vm.set_i32(rec + 0x7c, 0);
        return Ok(());
    }
    let a = (f64::from(vm.f32(t.force_a)) + force_a) as f32;
    vm.set_f32(t.force_a, a);
    let b = (f64::from(vm.f32(t.force_b)) + force_b) as f32;
    vm.set_f32(t.force_b, b);
    let drawing = env.call(vm, 0x140c51370, CallArgs::default()).rax as u32 != 0;
    if drawing {
        let mode = vm.u32(0x1_42f2_f21c).wrapping_sub(3);
        if mode <= 1 {
            return Err("debug markers not ported".into());
        }
    }
    Ok(())
}
