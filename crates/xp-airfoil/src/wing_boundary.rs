//! Original wing boundary loop (0x14124e542) and point transform (0x141294240).
//! Inputs are stored runtime values; this does not reconstruct their update schedule.
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub span: f32,
    pub design_dihedral: f32,
    pub design_sweep: f32,
    pub rotation: f32,
    pub dihedral_delta: f32,
    pub side: f32,
    pub variable_sweep: i32,
    pub rotation_offset: f32,
    pub retract: f32,
    pub origin: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct Curvature {
    pub angle: f32,
    pub amplitude: f32,
    pub scale: f32,
}

/// Original CRT results: cos/sin design sweep and dihedral, sin/cos rotation,
/// sin/cos dihedral delta, and the final curvature cos/sin (point-dependent).
#[derive(Clone, Copy, Debug)]
pub struct Trig(pub [f32; 10]);
const RAD: f32 = f32::from_bits(0x3c8efa36);

fn sin(x: f32) -> f32 {
    f64::from(x).sin() as f32
}
fn cos(x: f32) -> f32 {
    f64::from(x).cos() as f32
}

fn transform_base(t: &Transform, point: [f32; 3], trig: Trig) -> [f32; 3] {
    let [cs, cd, sd, ss, sr, cr, sn, cn, _, _] = trig.0;
    let hinge = t.variable_sweep as f32 * t.rotation_offset;
    let hx = cd * (cs * hinge);
    let hy = sd * (cs * hinge);
    let hz = ss * hinge;
    let retract = t.retract * t.span;
    let rx = cs * retract;
    let a = (point[0] - cd * rx) + hx;
    let b = (point[2] - ss * retract) + hz;
    let u = cr * a - sr * b;
    let w = (point[1] - sd * rx) + hy;
    let v = (sr * a + cr * b) - hz;
    [
        ((cn * u - sn * w) - hx) * t.side + t.origin[0],
        ((cn * w + sn * u) - hy) + t.origin[1],
        v + t.origin[2],
    ]
}

/// A boundary point using explicitly recorded original CRT outputs.
pub fn boundary_point_with_trig(
    t: &Transform,
    c: Curvature,
    elements: usize,
    rib: usize,
    chord: f32,
    offset: f32,
    trig: Trig,
) -> [f32; 3] {
    assert!(elements <= 10 && rib <= elements);
    let [cs, cd, sd, ss, _, _, _, _, curve_cos, curve_sin] = trig.0;
    let distance = rib as f32 * (t.span / (elements as f32).max(1.0));
    let transverse = distance * cs;
    let point = [
        cd * transverse,
        sd * transverse,
        offset * chord + distance * ss,
    ];
    let mut output = transform_base(t, point, trig);
    if f64::from(c.angle.abs()) >= 0.001 {
        let dy = (f64::from(c.amplitude) * (1.0 - f64::from(curve_cos))) as f32;
        output[1] += dy;
        let dx = curve_sin * c.amplitude - output[0];
        output[0] += dx;
    } else {
        // Original adds a +0 delta even when the curvature branch is disabled.
        output[1] += 0.0;
        output[0] += 0.0;
    }
    output
}

/// Platform trigonometry, with the original float32 angles and operation order.
pub fn boundary_point(
    t: &Transform,
    c: Curvature,
    elements: usize,
    rib: usize,
    chord: f32,
    offset: f32,
) -> [f32; 3] {
    assert!(elements <= 10 && rib <= elements);
    let mut trig = Trig([
        cos(t.design_sweep * RAD),
        cos(t.design_dihedral * RAD),
        sin(t.design_dihedral * RAD),
        sin(t.design_sweep * RAD),
        sin(t.rotation * RAD),
        cos(t.rotation * RAD),
        sin(t.dihedral_delta * RAD),
        cos(t.dihedral_delta * RAD),
        0.0,
        0.0,
    ]);
    let distance = rib as f32 * (t.span / (elements as f32).max(1.0));
    let transverse = distance * trig.0[0];
    let raw = [
        trig.0[1] * transverse,
        trig.0[2] * transverse,
        offset * chord + distance * trig.0[3],
    ];
    let base = transform_base(t, raw, trig);
    if f64::from(c.angle.abs()) >= 0.001 {
        let angle = c.angle * base[0] / c.scale;
        trig.0[8] = cos(angle);
        trig.0[9] = sin(angle);
    }
    boundary_point_with_trig(t, c, elements, rib, chord, offset, trig)
}
