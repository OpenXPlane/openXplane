//! The airflow velocity at a point of the aircraft, `0x14121b580`.
//!
//! The point `(x, z, y)` (the order of the original's arguments: `xmm1`, `xmm3`, then the stack float) is turned
//! into the world frame in double precision, the wind sampler (`0x141ba80a0`, outside this port: passed in) is
//! asked for the wind there, the wind is moved back into the aircraft frame and the velocity of the point from
//! the angular rates `F+0x3cc/0x3d0/0x3d4` is added. Every result is checked by `0x141176330`, which replaces a
//! non-finite value by zero.
//!
//! The original's last argument switches on a call of `0x14117d970` (18 KB), which is
//! not ported (the caller passes zero there). The eighth argument of the frame transform is `r12d`, which the
//! function's first finite check clears (`xorl %r12d, %r12d` at `0x14121b735`) whatever the caller had in the
//! register, so the origin offsets are never subtracted from the wind.
use crate::element_force::Mem;
use crate::transform::{Frame, to_aircraft_frame};

/// `0x141176330`: a non-finite float is replaced by zero (and logged).
fn checked(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

/// `0x140983ae0(v, -200, 200)`: false only when `v` is below -200 or above 200 (NaN passes).
fn in_range(v: f32) -> bool {
    let below = -200.0 > v;
    let above = v > 200.0;
    !below && !above
}

/// Results: the three velocity components in the aircraft frame (the original's `r14`, `rsi`, `rdi` outputs).
/// `origin_disabled` is the engine flag tested by `0x1417f12c0`; `wind(x, y, z)` is the sampler `0x141ba80a0`
/// for the world position and returns the three floats it stores (`+0xd8`, `+0xe8`, `+0xd0` of the frame).
pub fn airflow(
    f: &dyn Mem,
    point: [f32; 3],
    origin_disabled: bool,
    wind: impl FnOnce(f64, f64, f64) -> [f32; 3],
) -> Result<[f32; 3], &'static str> {
    let [x, z, y] = point;
    let (a, b, c) = (f64::from(x), f64::from(z), f64::from(y));
    let (s0, c0) = (f64::from(f.f32(0x430)), f64::from(f.f32(0x434)));
    let (s1, c1) = (f64::from(f.f32(0x440)), f64::from(f.f32(0x444)));
    let (s2, c2) = (f64::from(f.f32(0x450)), f64::from(f.f32(0x454)));
    let t10 = c2 * a + s2 * b;
    let mut t11 = c2 * b - s2 * a;
    let mut t12 = t11 * s0 + c0 * c;
    let mut t13 = t10 * c1 - t12 * s1;
    t11 = t11 * c0 - s0 * c;
    t12 = t12 * c1 + s1 * t10;
    let origin = |offset: usize| if origin_disabled { 0.0 } else { f.f64(offset) };
    t13 += origin(0x378);
    t11 += origin(0x380);
    t12 += origin(0x388);
    let finite = |v: f64| if (v as f32).is_finite() { v } else { 0.0 };
    let (world_x, world_y, world_z) = (finite(t13), finite(t11), finite(t12));
    let reference = [
        checked(f.f32(0x368)),
        checked(f.f32(0x36c)),
        checked(f.f32(0x370)),
    ];
    let w = wind(world_x, world_y, world_z);
    if !(in_range(w[0]) && in_range(w[1]) && in_range(w[2])) {
        return Err("wind outside the accepted range");
    }
    let relative = [
        w[0] - reference[0],
        w[1] - reference[1],
        w[2] - reference[2],
    ];
    let frame = Frame {
        origin: [f.f64(0x378), f.f64(0x380), f.f64(0x388)],
        rotation: [
            [f.f32(0x430), f.f32(0x434)],
            [f.f32(0x440), f.f32(0x444)],
            [f.f32(0x450), f.f32(0x454)],
        ],
    };
    let [o1, o2, o3] = to_aircraft_frame(&frame, relative, false, origin_disabled);
    let [o1, o2, o3] = [checked(o1), checked(o2), checked(o3)];
    let (rx, ry, rz) = (f.f32(0x3cc), f.f32(0x3d0), f.f32(0x3d4));
    let out1 = o1 - z * rx + y * rz;
    let out2 = y * ry + o2 + x * rx;
    let out3 = o3 - z * ry - x * rz;
    Ok([checked(out1), checked(out2), checked(out3)])
}
