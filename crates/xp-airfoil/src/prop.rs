//! The per-engine propeller force function `0x1411bd470` (24 KB), ported in segments and checked against the
//! original at frame checkpoints (`tools/gen_prop_vectors.py`).
//!
//! Objects, by the original's registers: `F` (`r14`, the flight object), `B` (`F+0x20`, the aircraft object),
//! `E` (`B[0x5ff8] + n*0x68`, the engine record), `P` (`B[0x6010] + n*0x3770`, the propeller/part record) and
//! `R` (`r13`, the output record). Locals are kept in a [`Frame`] under the original's `rbp`-relative offsets
//! (`rsp+X` is `rbp+X-0x100`); the meaning of most of them is not established.
//!
//! The debug dump the original performs when the object at `F+0xbcc8` is nonzero (a stream write of values)
//! is not ported.
use crate::airflow::airflow;
use crate::forces::Words;
use crate::scalar::{clamp, kind_is_3_or_7, lerp, max3, sign, snap};
use crate::transform::rotate_euler_offset;
use crate::wing_element::{boundary_at, hypot2, hypot3, interpolate_clamped, rotate_euler};
use std::collections::BTreeMap;

const RAD: f32 = f32::from_bits(0x3c8efa36);
const DEG: f32 = f32::from_bits(0x42652ee0);
const ABS: u32 = 0x7fff_ffff;

/// Frame slots by `rbp` offset, as raw words.
#[derive(Clone, Debug, Default)]
pub struct Frame(pub BTreeMap<i32, u32>);

impl Frame {
    pub fn set(&mut self, offset: i32, value: f32) {
        self.0.insert(offset, value.to_bits());
    }
    pub fn set_i(&mut self, offset: i32, value: i32) {
        self.0.insert(offset, value as u32);
    }
    pub fn f(&self, offset: i32) -> f32 {
        f32::from_bits(self.0.get(&offset).copied().unwrap_or(0))
    }
    pub fn i(&self, offset: i32) -> i32 {
        self.0.get(&offset).copied().unwrap_or(0) as i32
    }
}

/// The callees outside this port.
pub trait PropEnv {
    /// `0x1417f12c0`: the engine flag that disables the origin offsets.
    fn engine_flag(&mut self) -> bool;
    /// `0x141ba80a0`: the wind at a world position.
    fn wind(&mut self, x: f64, y: f64, z: f64) -> [f32; 3];
    /// `0x140c448c0`: the time step in seconds (a per-thread constant divided by a per-thread integer).
    fn frame_time(&mut self) -> f64;
}

pub struct Objects<'a> {
    pub f: &'a mut Words,
    pub b: &'a Words,
    pub e: &'a Words,
    pub p: &'a Words,
    pub r: &'a mut Words,
}

/// Where to stop (for checkpoint comparison with the original).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Before the call at `0x1411bda66`.
    Segment1,
    /// Before the element-count test at `0x1411be62a` (the loop over elements).
    Segment2,
}

/// Registers that live across blocks (the `xmm` registers of the original), for checkpoints.
#[derive(Clone, Copy, Debug, Default)]
pub struct Regs {
    pub xmm6: f32,
    pub xmm7: f32,
    pub xmm8: f32,
    pub xmm9: f32,
    pub xmm11: f32,
    pub xmm13: f32,
    pub xmm15: f32,
    pub r15: i32,
}

pub fn prop_force(
    o: &mut Objects,
    n: i32,
    env: &mut dyn PropEnv,
    stop: Stop,
) -> Result<(Frame, Regs), String> {
    let (f, b, e, p, r) = (&mut *o.f, o.b, o.e, o.p, &mut *o.r);
    let mut fr = Frame::default();
    // 0x1411bd4da..0x1411bd54e
    r.set_i32(0x60, 0);
    r.set_i32(0x64, 0);
    fr.set(0x10c, 0.0);
    fr.set(0x118, 0.0);
    fr.set(0xe0, 0.0);
    fr.set_i(0x8e0, n);
    let one = 1.0f32;
    fr.set(0x38, one);
    // 0x1411bd557: the two ramps on F+0x170 while the engine type word is set
    if b.i32(0x2140) != 0 && e.i32(0x30) != 0 {
        let v = interpolate_clamped(f32::from_bits(0x3f666666), one, one, 0.0, f.f32(0x170));
        fr.set(0x38, v);
    }
    if b.i32(0x2144) != 0 && e.i32(0x30) != 0 {
        let v = interpolate_clamped(f32::from_bits(0x3dcccccd), one, 0.0, 0.0, f.f32(0x170));
        fr.set(0x38, v);
    }
    // 0x1411bd5c9
    if b.i32(0xa78) != 0 {
        let ratio = b.f32(0x9f4) / p.f32(0x20);
        let half = (f64::from(ratio) * 0.5) as f32;
        let v = interpolate_clamped(ratio, one, half, 0.0, r.f32(0x1c));
        fr.set(0x38, v);
    }
    // 0x1411bd61f: the function ends here when the factor is below 0.01
    if 0.01f64 > f64::from(fr.f(0x38)) {
        return Err("factor below 0.01".into());
    }
    // 0x1411bd638: the airflow at the part's point
    let point = [p.f32(0x790), p.f32(0x794), p.f32(0x798)];
    let flag = env.engine_flag();
    let wind = airflow(f, point, flag, |x, y, z| env.wind(x, y, z))?;
    fr.set(0x84, wind[0]);
    fr.set(0x8e8, wind[1]);
    fr.set(0x8d8, wind[2]);
    // 0x1411bd6a6: rotate by the angles of the part record at +0x79c, +0x7a0, +0x7a4
    let angles = [p.f32(0x79c), p.f32(0x7a0), p.f32(0x7a4)];
    let rot = rotate_euler(angles, fr.f(0x84), fr.f(0x8e8), fr.f(0x8d8));
    fr.set(0x8e8, rot[0]);
    fr.set(0x8d8, rot[1]);
    fr.set(-0x18, rot[2]);
    let (x15, x7) = if p.i32(0) == 6 {
        (0.0, 0.0)
    } else {
        (fr.f(0x8e8), fr.f(0x8d8))
    };
    fr.set(0x8d8, x7);
    fr.set(0x8e8, hypot2(x15, x7));
    let x0 = fr.f(-0x18);
    let x11 = f32::from_bits(x0.to_bits() & ABS);
    fr.set(-0x14, x11);
    fr.set(-0xa0, hypot3(x15, x7, x0));
    fr.set(0x80, x15.atan2(x7));
    let mut x6 = f.f32(0x1cc);
    x6 = x6
        * if 0.0 > x6 {
            b.f32(0x20b0)
        } else {
            b.f32(0x20b4)
        };
    x6 += f.f32(0x11c);
    let mut x9 = f.f32(0x1d0);
    x9 = x9
        * if 0.0 > x9 {
            b.f32(0x20b8)
        } else {
            b.f32(0x20bc)
        };
    x9 += f.f32(0x118);
    let mut x13 = f.f32(0x120);
    // 0x1411bd7d7: the part's last boundary coordinate, and a flag
    let elements = p.i32(0x8c);
    let values: Vec<f32> = (0..=elements.max(0) as usize)
        .map(|i| p.f32(0x88 + 0x5bc + 4 * i))
        .collect();
    fr.set(-0x10, boundary_at(&values, elements, elements as f32));
    fr.set_i(0x84, i32::from(p.f32(0xc) > 0.0));
    let mut x8 = f32::from_bits(0x7fff_ffff);
    let one_f = 1.0f32;
    let mut r15 = 0;
    fr.set(0xc, one_f);
    if kind_is_3_or_7(p.i32(0)) {
        let lever = [0x2080, 0x2084, 0x2088, 0x208c]
            .iter()
            .any(|&o| b.f32(o) > 0.0)
            || [0x209c, 0x20a0, 0x212c]
                .iter()
                .any(|&o| f32::from_bits(b.f32(o).to_bits() & ABS) > 0.0);
        if lever {
            let mut x7 = if b.i32(0x296c) != 0 {
                let s = (p.f32(0x88 + 0x718) * RAD).sin();
                let c = clamp(s, 0.0, one_f);
                if 0.0 > c { f32::NAN } else { c.sqrt() }
            } else {
                one_f
            };
            fr.set(0xc, x7);
            let mut x8v = if 0.0 > x6 {
                b.f32(0x2088)
            } else {
                b.f32(0x208c)
            };
            let mut x11v = if 0.0 > x9 {
                b.f32(0x2080)
            } else {
                b.f32(0x2084)
            };
            x8v *= x6;
            x8v *= x7;
            let mut x1 = x13 * b.f32(0x20a0);
            x1 *= x7;
            let x0 = sign(p.f32(0x88 + 0x710)) * x1;
            x8v -= x0;
            r.set_f32(0x34, x8v);
            let x0 = sign(p.f32(0x88 + 0x708));
            x13 *= b.f32(0x209c);
            x13 *= fr.f(0xc);
            x7 = x0 * x13;
            x11v *= x9;
            x11v *= fr.f(0xc);
            x7 += x11v;
            let ramp = interpolate_clamped(
                b.f32(0x211c),
                0.0,
                b.f32(0x2124),
                b.f32(0x212c),
                f.f32(0x41c),
            );
            x7 += ramp;
            r.set_f32(0x30, x7);
            x8v -= r.f32(0x44);
            x7 -= r.f32(0x40);
            let x6t = (x7 * RAD).tan();
            let s = (b.f32(0x2138) * RAD).sin() * x6t;
            let d = s.atan() * DEG + x8v;
            r.set_f32(0x3c, d);
            let x6t = (x8v * RAD).tan();
            x6 = x6t;
            let s = (b.f32(0x2138) * RAD).sin() * x6t;
            x7 -= s.atan() * DEG;
            r.set_f32(0x38, x7);
            r15 = 1;
            x8 = f32::from_bits(ABS);
            let _ = x8v;
        }
    }
    let _ = x8;
    let mut x7 = fr.f(0x8d8);
    let mut x11 = fr.f(-0x14);
    let mut x8 = f32::from_bits(ABS);
    let regs1 = Regs {
        xmm6: x6,
        xmm7: x7,
        xmm8: x8,
        xmm9: x9,
        xmm11: x11,
        xmm13: x13,
        xmm15: x15,
        r15,
    };
    if stop == Stop::Segment1 {
        return Ok((fr, regs1));
    }

    // ---- segment 2: 0x1411bda66 .. 0x1411be62a ----
    if f.i32(0xbcc8) != 0 || f.i32(0xbcd0) != 0 {
        return Err("debug dump not ported".into());
    }
    x6 = fr.f(-0x18);
    let abs = |v: f32| f32::from_bits(v.to_bits() & ABS);
    let a6 = abs(x6);
    x7 = r.f32(0x84);
    x8 = abs(x7);
    let m = max3(x8, a6, one);
    x13 = interpolate_clamped(0.0, 0.0, m, one, fr.f(0x8e8));
    x6 = f32::from_bits(x6.to_bits() ^ 0x8000_0000);
    let d = snap(x7, f32::from_bits(0xbc23d70a), f32::from_bits(0x3c23d70a));
    x6 /= d;
    x6 = clamp(x6, -2.0, 2.0);
    r.set_f32(0x50, x6);
    let half = 0.5f32;
    x7 = -0.5;
    x9 = interpolate_clamped(-0.75, half, x7, half, x6);
    if x6 >= x7 {
        x9 = interpolate_clamped(x7, half, half, half, x6);
    }
    x7 = 4.0;
    if x6 >= half {
        x9 = interpolate_clamped(half, half, one, x7, x6);
    }
    if x6 >= one {
        x9 = interpolate_clamped(one, x7, 1.25, one, x6);
    }
    x8 = (f64::from(x8) * 10.0).sqrt() as f32;
    let d7 = f64::from(x8);
    let x2 = (f64::from(x11) + f64::from(x11)) as f32;
    x6 = interpolate_clamped(x11, 0.0, x2, one, fr.f(0x8e8));
    x11 = interpolate_clamped((d7 * 0.5) as f32, 0.0, x8, one, fr.f(0x8e8));
    x11 *= x6;
    x7 = interpolate_clamped(x8, 0.0, (d7 * 5.0) as f32, one, fr.f(0x8e8));
    x7 *= x6;
    if x11 > 0.0 {
        x9 = interpolate_clamped(0.0, x9, one, half, x11);
    }
    if x7 > 0.0 {
        x9 = interpolate_clamped(0.0, x9, one, 0.25, x7);
    }
    let dt = env.frame_time();
    let t2 = (dt + dt) as f32;
    r.set_f32(0xa0, lerp(r.f32(0xa0), x9, t2));
    x8 = interpolate_clamped(0.0, fr.f(-0x18), one, fr.f(-0xa0), x11);
    fr.set(-0x14, x8);
    let p10 = p.f32(0x10);
    let quarter = (f64::from(p10) * 0.25) as f32;
    x11 = quarter;
    let x0;
    if p10 == 2.0 && (b.i32(0x214c) != 0 || b.i32(0x2150) != 0) {
        let inv = (1.0 / env.frame_time()) as f32;
        x9 = (f64::from(inv) * f64::from_bits(0x3fc6_5718_7000_0000)) as f32;
        let x8l = r.f32(0x1c);
        x7 = quarter;
        x6 = r.f32(0x24) * RAD;
        x11 = interpolate_clamped(0.0, abs(x6.sin()), x9, x7, x8l);
        x0 = interpolate_clamped(0.0, abs(x6.cos()), x9, x7, x8l);
        x8 = fr.f(-0x14);
    } else {
        x0 = quarter;
    }
    fr.set(0x48, x0);
    fr.set(0x4c, x11);
    fr.set(0x50, x0);
    fr.set(0x54, x11);
    // 0x1411be54c: the rotation of the point (0, 0, -1) by the part's angles, with its offsets
    let angles = [p.f32(0x79c), p.f32(0x7a0), p.f32(0x7a4)];
    let offsets = [p.f32(0x790), p.f32(0x794), p.f32(0x798)];
    let rot = rotate_euler_offset(angles, offsets, false, 0.0, 0.0, -1.0);
    fr.set(-0xc, rot[0]);
    fr.set(-0xa0, rot[1]);
    fr.set(0x8d8, rot[2]);
    for off in [
        0xcc, 0xd0, 0xdc, 0xd4, 0xd8, 0x70, 0x110, 0xc8, 0xe4, 0xe8, 0xec,
    ] {
        fr.set(off, 0.0);
    }
    fr.set(0x114, f32::from_bits(0x3c23d70a));
    fr.set(0x10, f32::from_bits(0x3c23d70a));
    fr.set_i(0x8d8, 0);
    x6 = 0.0;
    x7 = 0.0;
    x9 = 0.0;
    x11 = 0.0;
    let regs = Regs {
        xmm6: x6,
        xmm7: x7,
        xmm8: x8,
        xmm9: x9,
        xmm11: x11,
        xmm13: x13,
        xmm15: f32::from_bits(ABS),
        r15,
    };
    Ok((fr, regs))
}
