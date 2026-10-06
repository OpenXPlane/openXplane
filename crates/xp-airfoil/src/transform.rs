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

/// `0x1407ac180`: rotates `(a, b, c)` with three sine/cosine pairs `p = [p7 .. p12]` (the seventh to twelfth
/// arguments of the original): a rotation by the pair `(p11, p12)`, then by `(p9, p10)`, then by `(p7, p8)`.
/// Returns the three results in the order the original stores them.
pub fn rotate_pairs(a: f32, b: f32, c: f32, p: [f32; 6]) -> [f32; 3] {
    let [p7, p8, p9, p10, p11, p12] = p;
    let u = b * p12 - a * p11;
    let v = b * p11 + a * p12;
    let w = u * p9 + c * p10;
    [v * p8 - w * p7, u * p10 - c * p9, w * p8 + v * p7]
}

/// `0x14120cf60`: rotates `(a, b, c)` by three angles in degrees, `angles = [+0x9c, +0xa0, +0xa4]` of the
/// object (through [`rotate_pairs`] with their sines and cosines), and adds the offsets `+0x90`, `+0x94`,
/// `+0x98` when `add_offsets` is set. The trigonometric functions come from the platform's libm here.
pub fn rotate_euler_offset(
    angles: [f32; 3],
    offsets: [f32; 3],
    add_offsets: bool,
    a: f32,
    b: f32,
    c: f32,
) -> [f32; 3] {
    const RAD: f32 = f32::from_bits(0x3c8e_fa36);
    let (r0, r1, r2) = (angles[2] * RAD, angles[1] * RAD, angles[0] * RAD);
    let p = [r2.sin(), r2.cos(), r1.sin(), r1.cos(), r0.sin(), r0.cos()];
    let mut out = rotate_pairs(a, b, c, p);
    if add_offsets {
        for k in 0..3 {
            out[k] += offsets[k];
        }
    }
    out
}

/// `0x1407ac020`: the inverse of [`to_aircraft_frame`]'s order: rotates with the pairs of the frame
/// (`+0x440/0x444`, `+0x430/0x434`, `+0x450/0x454` as the pairs 7 to 12) and, when `shift` (the eighth
/// argument equals 1) and the engine flag is clear, adds the origin offsets in double precision.
pub fn from_aircraft_frame(
    f: &Frame,
    point: [f32; 3],
    shift: bool,
    origin_disabled: bool,
) -> [f32; 3] {
    let p = [
        f.rotation[1][0],
        f.rotation[1][1],
        f.rotation[0][0],
        f.rotation[0][1],
        f.rotation[2][0],
        f.rotation[2][1],
    ];
    let mut out = rotate_pairs(point[0], point[1], point[2], p);
    if shift {
        let off = if origin_disabled { [0.0; 3] } else { f.origin };
        for k in 0..3 {
            out[k] = (f64::from(out[k]) + off[k]) as f32;
        }
    }
    out
}
