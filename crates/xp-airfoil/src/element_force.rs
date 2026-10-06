//! The per-element force function of the wing, `get_el_force` at `0x1411b9840`, ported with the helpers
//! of [`crate::wing_element`]. The original reads four objects at fixed offsets; here they are read through
//! [`Mem`] at the same offsets so the port can be compared against the original on the same data:
//! `F` (the flight object), `B` (the aircraft object at `F+0x20`), `W` (the wing) and `X` (the element
//! arrays).
//!
//! Ported: the control surface loop, the flap and slat terms, the dihedral geometry, the call to the wing
//! element function, the flow-separation weight, the output scaling and the per-element arrays. Not ported:
//! the alternative regime function `0x1411b8e00` (reached when the weight is positive, reported as an error)
//! and the structural-load section at the end (skipped by the original when `F+0x28` is nonzero).
use crate::wing_element::{
    Aircraft, Boundary, ControlSurface, ElementInputs, ElementState, Flow, FoilCall, FoilResult,
    WingFields, angle_floor, control_deflection, control_surface_terms, element_area, evaluate,
};

/// Read access to one object of the original by byte offset.
pub trait Mem {
    fn f32(&self, offset: usize) -> f32;
    fn i32(&self, offset: usize) -> i32;
}

pub struct Objects<'a> {
    pub f: &'a dyn Mem,
    pub b: &'a dyn Mem,
    pub w: &'a dyn Mem,
    pub x: &'a dyn Mem,
}

/// The call arguments that are not in the objects.
#[derive(Clone, Copy, Debug)]
pub struct Call<'a> {
    pub index: usize,
    /// The fourth argument (`R9`), passed on as the stall-memory request.
    pub retain: bool,
    /// The sixth argument: the ice factor.
    pub ice: f32,
    /// `[[W+0x3680]+0xd0]+0x10`, read through pointers in the original.
    pub g10: f32,
    /// The airfoil names at `W+0x3618`, `+0x3638`, `+0x3658`.
    pub names: [&'a str; 3],
}

/// What the function writes: the three outputs and the element arrays of `X`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForceOutput {
    pub out1: f32,
    pub out2: f32,
    pub out3: f32,
    /// `X+0xf4+4e`, `+0x11c`, `+0x144`, `+0x16c`.
    pub x_f4: f32,
    pub x_11c: f32,
    pub x_144: f32,
    pub x_16c: f32,
    /// The wing element result (its ratio is added to `X+0x1bc+4e`, its stall flag stored at `X+0x1e4+4e`).
    pub element: crate::wing_element::ElementOutput,
}

/// Per surface: code, gate offset in `W` (an int per element), angle offset in `X`.
const SURFACES: [(u32, usize, usize); 13] = [
    (0xb, 0x2fc, 0x29c),
    (0xc, 0x32c, 0x2a0),
    (0xd, 0x41c, 0x2a4),
    (0xe, 0x44c, 0x2a8),
    (0xf, 0x47c, 0x2b4),
    (0x10, 0x35c, 0x2b8),
    (0x11, 0x38c, 0x2bc),
    (0x12, 0x3bc, 0x2c0),
    (0x13, 0x3ec, 0x2c4),
    (0x14, 0x50c, 0x2c8),
    (0x15, 0x53c, 0x2cc),
    (0x16, 0x4ac, 0x2ac),
    (0x17, 0x4dc, 0x2b0),
];

const DEGREES: f32 = 57.295_776;
const RADIANS_PER_DEGREE: f32 = f32::from_bits(0x3c8e_fa36);
const SEPARATION_LIMIT: f32 = 0.99;

/// `get_el_force` for one element. `foil` is the profile function (see [`evaluate`]); `driven` answers the
/// input-binding queries of the control surface function.
pub fn element_force<F>(
    o: &Objects,
    call: &Call,
    foil: F,
    driven: &dyn Fn(u32) -> bool,
) -> Result<ForceOutput, String>
where
    F: FnMut(&FoilCall) -> Result<FoilResult, String>,
{
    let (f, b, w, x) = (o.f, o.b, o.w, o.x);
    let e = call.index;
    let at = |base: usize| base + 4 * e;

    // control surfaces: angle, Cl, Cd and Cm terms
    let mut acc = [0.0f32; 4];
    for (code, gate, angle_at) in SURFACES {
        if w.i32(at(gate)) == 0 {
            continue;
        }
        let deflection = control_deflection(code, &|o| w.f32(o), &|o| b.f32(o), e as i32)
            .ok_or("control surface code")?;
        let table = b.i32(0x1eb0) as usize;
        let surface = ControlSurface {
            code,
            deflection,
            chord: w.f32(at(0x70)),
            wing_0: w.f32(0),
            wing_20: w.f32(0x20),
            x_2c: x.f32(at(0x2c)),
            x_1bc: x.f32(at(0x1bc)),
            angle_deg: x.f32(angle_at),
            modes: [b.i32(0x1d20), b.i32(0x1d24), b.i32(0x1d28)],
            kind: b.i32(0x1f74),
            table_a: b.f32(0x1f84 + 4 * table),
            table_b: b.f32(0x1fc4 + 4 * table),
            ratios: [
                b.f32(0x1f78),
                b.f32(0x1f7c),
                b.f32(0x1f80),
                b.f32(0x1fb8),
                b.f32(0x1fbc),
                b.f32(0x1fc0),
            ],
        };
        if let Some(t) = control_surface_terms(&surface, driven) {
            for k in 0..4 {
                acc[k] += t[k];
            }
        }
    }
    if w.i32(at(0x56c)) != 0 {
        acc[2] += b.f32(0x1f04) * f.f32(0x1a0);
    }
    if w.i32(at(0x594)) != 0 {
        acc[2] += b.f32(0x1f40) * f.f32(0x1a4);
    }
    if w.i32(at(0x50c)) != 0 {
        acc[1] += f.f32(0x64c0);
    }
    if w.i32(at(0x53c)) != 0 {
        acc[1] += f.f32(0x64c0);
    }

    // dihedral of the element and the cosine floor used as a divisor
    let dz = w.f32(at(0x618)) - w.f32(at(0x614));
    let d1 = w.f32(at(0x5c0)) - w.f32(at(0x5bc));
    let d0 = w.f32(at(0x5ec)) - w.f32(at(0x5e8));
    let sum = d1 * d1 + d0 * d0;
    let distance = if 0.0 > sum { f32::NAN } else { sum.sqrt() };
    let dihedral = dz.atan2(distance) * DEGREES;
    let angle = w.f32(0) * f.f32(0x408) + dihedral;
    let floor = angle_floor((angle * RADIANS_PER_DEGREE).cos()).abs();
    let speed_ratio = x.f32(at(0x54)) / f.f32(0x74);

    // the wing element
    let elements = w.i32(4);
    let count = (elements.max(0) as usize) + 1;
    let read = |base: usize| -> Vec<f32> { (0..count).map(|i| w.f32(base + 4 * i)).collect() };
    let (bx, by, bz, chord) = (read(0x5bc), read(0x5e8), read(0x614), read(0x70));
    let flaps: Vec<i32> = (0..count).map(|i| w.i32(0x56c + 4 * i)).collect();
    let slats: Vec<i32> = (0..count).map(|i| w.i32(0x594 + 4 * i)).collect();
    let input = ElementInputs {
        index: e,
        retain_request: call.retain,
        arg6: speed_ratio,
        ice: call.ice,
        alpha_in: acc[0],
        extra: [acc[1], acc[2], acc[3]],
        flow: Flow {
            f5c: f.f32(0x5c),
            f6c: f.f32(0x6c),
            f1a0: f.f32(0x1a0),
            f1a4: f.f32(0x1a4),
            f408: f.f32(0x408),
            flag_dac: f.i32(0xdac) != 0,
            diagnostics: false,
        },
        aircraft: Aircraft {
            f1f00: b.f32(0x1f00),
            f1f3c: b.f32(0x1f3c),
            f64f4: b.f32(0x64f4),
            f64f8: b.f32(0x64f8),
            f64fc: b.f32(0x64fc),
        },
        g10: call.g10,
        wing: WingFields {
            is_right: w.f32(0),
            elements,
            f14: w.f32(0x14),
            f18: w.f32(0x18),
            f1c: w.f32(0x1c),
            ratios: [w.f32(0x5c), w.f32(0x60), w.f32(0x64), w.f32(0x68)],
            boundary: Boundary {
                x: &bx,
                y: &by,
                z: &bz,
                chord: &chord,
            },
            flap_flags: &flaps,
            slat_flags: &slats,
            names: call.names,
        },
        state: ElementState {
            v: x.f32(4 + 4 * e),
            r11: x.f32(at(0x2c)),
            r21: x.f32(at(0x54)),
            r163: x.f32(0x28c),
            stall_flag: x.i32(at(0x1e4)) != 0,
        },
        skip_delta_block: false,
    };
    let element = evaluate(&input, foil)?;

    // flow separation weight
    let ratio = element.ret / floor.sqrt();
    let limited = ratio.clamp(0.01, SEPARATION_LIMIT);
    let weight = if limited == 1.0 {
        0.5
    } else {
        ((1.0 / (1.0 - limited)) * (f.f32(0x420) - limited) + 0.0).clamp(0.0, 1.0)
    };
    if weight > 0.0 {
        return Err(format!(
            "the alternative regime function 0x1411b8e00 is not ported (weight {weight})"
        ));
    }

    // scale and offsets
    let scale = x.f32(0x288);
    let scaled = if element.cl > 0.0 {
        scale * element.cl
    } else {
        element.cl / scale
    };
    let cl = scaled + x.f32(at(0x7c));
    let cd = element.cd + x.f32(at(0xa4));
    let cm = element.cm + x.f32(at(0xcc));
    let dynamic = ((x.f32(at(0x54)) * x.f32(at(0x54)) * f.f32(0x6c)) as f64 * 0.5) as f32;
    let area = element_area(w.f32(0x30), w.f32(0x10), &chord, e, elements);
    let out1 = area * (dynamic * cl);
    let out2 = area * (dynamic * cd);
    let taper = chord[e + 1] / chord[e];
    let plus_one = f64::from(taper) + 1.0;
    let mean =
        ((f64::from(taper * taper) + plus_one) / plus_one) * (f64::from(chord[e]) * (2.0 / 3.0));
    let out3 = area * (dynamic * cm) * (mean as f32);
    Ok(ForceOutput {
        out1,
        out2,
        out3,
        x_f4: cl,
        x_11c: cd,
        x_144: cm,
        x_16c: cd - element.induced_drag,
        element,
    })
}
