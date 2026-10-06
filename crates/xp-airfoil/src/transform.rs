//! Coordinate transforms of the flight object used by the wing force function.

/// The flight object fields `0x141296750` reads: the origin offsets (doubles at `+0x378`, `+0x380`,
/// `+0x388`) and the three sine/cosine pairs at `+0x430/+0x434`, `+0x440/+0x444`, `+0x450/+0x454`.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub origin: [f64; 3],
    /// `[+0x430, +0x434]`, `[+0x440, +0x444]`, `[+0x450, +0x454]`.
    pub rotation: [[f32; 2]; 3],
}

/// `0x141296750`: moves a point from the world frame into the aircraft frame. When `shift` (the eighth
/// argument equals 1) and the engine flag tested by `0x1417f12c0` is clear (`origin_disabled` false), the
/// origin offsets are subtracted first (in double precision); then the point is rotated by the three
/// sine/cosine pairs. Returns the three results in the order the original stores them.
pub fn to_aircraft_frame(
    f: &Frame,
    point: [f32; 3],
    shift: bool,
    origin_disabled: bool,
) -> [f32; 3] {
    let [mut x, mut y, mut z] = point;
    if shift {
        let off = if origin_disabled { [0.0; 3] } else { f.origin };
        x = (f64::from(x) - off[0]) as f32;
        y = (f64::from(y) - off[1]) as f32;
        z = (f64::from(z) - off[2]) as f32;
    }
    let [[s0, c0], [s1, c1], [s2, c2]] = f.rotation;
    // the original names: xmm7 = +0x430, xmm6 = +0x434, xmm2 = +0x440, xmm1 = +0x444, xmm5 = +0x450, xmm4 = +0x454
    let a = z * c1 - x * s1;
    let b = x * c1 + z * s1;
    let out3 = a * c0 - y * s0;
    let c = y * c0 + a * s0;
    let out2 = c * c2 + b * s2;
    let out1 = b * c2 - c * s2;
    [out1, out2, out3]
}
