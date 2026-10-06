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
use crate::engine::signed_pow;
use crate::forces::Words;
use crate::scalar::{clamp, kind_is_3_or_7, lerp, max3, sign, snap};
use crate::transform::rotate_euler_offset;
use crate::wing_element::{
    Boundary, boundary_at, element_dihedral, hypot2, hypot3, interpolate_clamped, rotate_euler,
};
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
    /// `0x140c81ea0(&0x142f01928)`: a double derived from the simulation time (used for the turbulence phase).
    fn time_phase(&mut self) -> f64;
    /// `0x140984e50`: the 2-D noise table lookup `(x, y, seed)`.
    fn noise2(&mut self, x: f32, y: f32, seed: i32) -> f32;
    /// `0x14195f4b0(F+0x42e40, a, b, &height, 0, 0, &flag)`: the terrain probe between two points; the height
    /// starts at -500 and the probe may store a new one.
    fn terrain(&mut self, a: [f32; 3], b: [f32; 3], height: f32) -> f32;
}

pub struct Objects<'a> {
    pub f: &'a mut Words,
    pub b: &'a Words,
    pub e: &'a Words,
    pub p: &'a Words,
    pub r: &'a mut Words,
    /// The element-state record of engine `n` for each station `b` (`F[0x68e0 + b*0x18] + n*0x2d8`).
    pub x: &'a mut [Words],
}

/// Where to stop (for checkpoint comparison with the original).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Before the call at `0x1411bda66`.
    Segment1,
    /// Before the element-count test at `0x1411be62a` (the loop over elements).
    Segment2,
    /// First pass of the loop, before the second airflow call at `0x1411bf1c8`.
    Segment3,
}

/// Registers that live across blocks (the `xmm` registers of the original, low 32 bits), for checkpoints. Only
/// the registers the port models at a checkpoint are `Some`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Regs {
    pub xmm: [Option<f32>; 16],
    pub r15: i32,
}

impl Regs {
    fn with(values: &[(usize, f32)], r15: i32) -> Self {
        let mut regs = Regs {
            xmm: [None; 16],
            r15,
        };
        for (i, v) in values {
            regs.xmm[*i] = Some(*v);
        }
        regs
    }
}

pub fn prop_force(
    o: &mut Objects,
    n: i32,
    env: &mut dyn PropEnv,
    stop: Stop,
) -> Result<(Frame, Regs), String> {
    let (f, b, e, p, r) = (&mut *o.f, o.b, o.e, o.p, &mut *o.r);
    let xs = &mut *o.x;
    let mut fr = Frame::default();
    // 0x1411bd4da..0x1411bd54e
    r.set_i32(0x60, 0);
    r.set_i32(0x64, 0);
    fr.set(0x10c, 0.0);
    fr.set(0x118, 0.0);
    fr.set(0xe0, 0.0);
    fr.set_i(0x8e0, n);
    let one = 1.0f32;
    let zero0 = 0.0f32;
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
    let regs1 = Regs::with(
        &[
            (6, x6),
            (7, x7),
            (8, x8),
            (9, x9),
            (10, one),
            (11, x11),
            (12, RAD),
            (13, x13),
            (14, zero0),
            (15, x15),
        ],
        r15,
    );
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
    if stop == Stop::Segment2 {
        let regs = Regs::with(
            &[
                (6, x6),
                (7, x7),
                (8, x8),
                (9, x9),
                (10, one),
                (11, x11),
                (12, RAD),
                (13, x13),
                (14, zero0),
                (15, f32::from_bits(ABS)),
            ],
            r15,
        );
        return Ok((fr, regs));
    }

    // ---- segment 3: the loop over `k` (0x1411be62a ..), first pass up to 0x1411bf1c8 ----
    let count = p.i32(0x8c);
    if count <= 0 {
        return Err("loop skipped: post-loop code not ported".into());
    }
    let one = 1.0f32;
    let half = 0.5f32;
    let zero = 0.0f32;
    let k = 0i32;
    // outer setup
    let mut x10: f32;
    fr.set(
        0x18c,
        snap(
            fr.f(-0xa0),
            f32::from_bits(0xbc23d70a),
            f32::from_bits(0x3c23d70a),
        ),
    );
    x8 = fr.f(-0x10);
    fr.set(0x150, (f64::from(x8) + f64::from(x8)) as f32);
    let mut x2 = fr.f(0x8e8) * f32::from_bits(0x3ff8cfe5);
    x2 = (f64::from(x2) - 15.0) as f32;
    x2 = abs(x2);
    x2 -= zero;
    x2 *= f32::from_bits(0x3e4ccccd);
    x10 = one - x2;
    fr.set(0x158, x10);
    x6 = fr.f(0x80);
    fr.set(0x184, x6.cos());
    fr.set(0x160, x6.sin());
    let x3 = f64::from(x13);
    fr.set(0x13c, (x3 + 1.0) as f32);
    fr.set(0x134, (1.0 - x3) as f32);
    x13 = f32::from_bits(0x8000_0000);
    fr.set(0x164, f32::from_bits(x8.to_bits() ^ 0x8000_0000));
    let rbx = n as u8;
    let _ = rbx;
    fr.set_i(0x248, n);
    fr.set_i(0x24c, 0);
    fr.set_i(0x28, n.wrapping_mul(0x3770));
    fr.set_i(0x2c, 0);
    fr.set_i(0x208, n);
    fr.set_i(0x20c, 0);
    let _ = (x13, x10);
    let edx = k;
    let inner = 0i32;
    // inner setup (0x1411be756)
    fr.set_i(0x8, 0);
    fr.set_i(0x30, 0);
    fr.set_i(0x34, 0);
    fr.set(0x154, edx as f32);
    x7 = (f64::from(edx as f32) + 0.5) as f32;
    fr.set(-0xc, x7);
    fr.set_i(0x240, k * 16);
    fr.set_i(0x244, 0);
    fr.set_i(0x200, k * 4);
    fr.set_i(0x204, 0);
    let q = (f64::from(x7) * 0.1) as f32;
    fr.set(0x178, q);
    let d2 = f64::from(q);
    fr.set(0x168, (d2 + d2) as f32);
    fr.set(0x17c, (d2 * 4.0) as f32);
    fr.set(0x180, (d2 * 8.0) as f32);
    fr.set(0x15c, (d2 * 16.0) as f32);
    fr.set(0x188, (d2 * 32.0) as f32);
    // station body (0x1411be836)
    let _ = (inner, k, &xs);
    let values_x: Vec<f32> = (0..=elements.max(0) as usize)
        .map(|i| p.f32(0x88 + 0x5bc + 4 * i))
        .collect();
    let values_y: Vec<f32> = (0..=elements.max(0) as usize)
        .map(|i| p.f32(0x88 + 0x5e8 + 4 * i))
        .collect();
    let values_z: Vec<f32> = (0..=elements.max(0) as usize)
        .map(|i| p.f32(0x88 + 0x614 + 4 * i))
        .collect();
    x8 = boundary_at(&values_x, elements, x7);
    fr.set(0x120, x8);
    x6 = boundary_at(&values_z, elements, x7);
    fr.set(0x78, x6);
    x9 = boundary_at(&values_y, elements, x7);
    let chord: Vec<f32> = vec![0.0; values_x.len()];
    let bnd = Boundary {
        x: &values_x,
        y: &values_y,
        z: &values_z,
        chord: &chord,
    };
    x7 = element_dihedral(&bnd, fr.i(0x8d8).max(0) as usize);
    x10 = DEG;
    x7 -= x6.atan2(x8) * x10;
    fr.set(0, x7);
    fr.set(0x18, (x7 * RAD).cos());
    let mut x1 = inner as f32;
    x1 -= zero;
    x1 *= f32::from_bits(0x3fc90fdb);
    x1 += zero;
    x6 = if 0.0 > x1 {
        zero
    } else if f32::from_bits(0x40c90fdb) < x1 {
        f32::from_bits(0x40c90fdb)
    } else {
        x1
    };
    x11 = x6.sin();
    x7 = x6.cos();
    fr.set(4, x7);
    let mut x15 = x11 * x8;
    fr.set(0x100, x15);
    fr.set(0x6c, x7 * x8);
    x9 = f32::from_bits(x9.to_bits() ^ 0x8000_0000);
    fr.set(0x138, x9);
    let (c4, s4) = ((p.f32(0x7a4) * RAD).cos(), (p.f32(0x7a4) * RAD).sin());
    let (c0, s0) = ((p.f32(0x7a0) * RAD).cos(), (p.f32(0x7a0) * RAD).sin());
    let (c9, s9) = ((p.f32(0x79c) * RAD).cos(), (p.f32(0x79c) * RAD).sin());
    x13 = c4;
    x7 = s4;
    x8 = c0;
    x9 = s0;
    x10 = c9;
    let mut x3 = s9;
    let mut x2 = x13 * x15 + x7 * fr.f(0x6c);
    x13 *= fr.f(0x6c);
    x7 *= x15;
    x13 -= x7;
    x15 = x13 * x9;
    x15 += x8 * fr.f(0x138);
    let mut x12 = x10 * x2;
    x12 -= x15 * x3;
    x12 += p.f32(0x790);
    fr.set(0x108, x12);
    x13 *= x8;
    x9 *= fr.f(0x138);
    x13 -= x9;
    x13 += p.f32(0x794);
    fr.set(0x190, x13);
    x15 *= x10;
    x3 *= x2;
    x15 += x3;
    x15 += p.f32(0x798);
    fr.set(0x80, x15);
    // the point in single precision into the world frame, then the origin offsets in double precision
    let (c2, s2, c0f, s0f, c1f, s1f) = (
        f.f32(0x454),
        f.f32(0x450),
        f.f32(0x434),
        f.f32(0x430),
        f.f32(0x444),
        f.f32(0x440),
    );
    x6 = x12 * c2 + s2 * x13;
    x7 = c2 * x13 - x12 * s2;
    x8 = x7 * s0f + c0f * x15;
    x9 = x6 * c1f - x8 * s1f;
    x7 = x7 * c0f - s0f * x15;
    x8 = x8 * c1f + x6 * s1f;
    let flag = env.engine_flag();
    let origin = |f: &Words, off: usize| if flag { 0.0 } else { f.f64(off) };
    x6 = (f64::from(x9) + origin(f, 0x378)) as f32;
    x7 = (f64::from(x7) + origin(f, 0x380)) as f32;
    fr.set(0x20, x7);
    x2 = (f64::from(x8) + origin(f, 0x388)) as f32;
    fr.set(0xb8, x6);
    fr.set(0xbc, x7 + p.f32(0x3730));
    fr.set(0xc0, x2);
    fr.set(0x198, x6);
    fr.set(0x19c, x7 - f.f32(0x42f54));
    fr.set(0x1a0, x2);
    let height = env.terrain(
        [x6, x7 + p.f32(0x3730), x2],
        [x6, x7 - f.f32(0x42f54), x2],
        -500.0,
    );
    fr.set(0x104, height);
    fr.set_i(-0xa0, 0);
    let ratio = abs((x7 - height) / fr.f(0x18c));
    let x2d = f64::from(ratio);
    let r3730 = f64::from(p.f32(0x3730));
    let x4 = (x2d / (r3730 + r3730)) as f32;
    let s30 = 0usize;
    let mut x1 = r.f32(0x284 + 4 * (s30 + k as usize * 4));
    if (-0.01..=0.01).contains(&x1) || x1.is_nan() {
        x1 = if 0.0 > x1 { -0.01 } else { 0.01 };
    }
    let x3 = ((x2d + x2d) / f64::from(x1)) as f32 * fr.f(0x8e8);
    let f150 = fr.f(0x150);
    x7 = one;
    let mut x6;
    if f150 == 0.0 {
        x8 = half;
        x6 = half;
    } else {
        let x0 = f150 - zero;
        let x1 = x7 / x0;
        let x3 = x3 - zero;
        let x0 = x7 - x1 * x3;
        x6 = if 0.0 > x0 {
            zero
        } else if x7 < x0 {
            x7
        } else {
            x0
        };
        x8 = half;
    }
    x6 *= x6;
    let rdi = s30 + k as usize * 4;
    r.set_f32(0xa4 + 4 * rdi, one);
    let mut x0 = x7;
    if r.f32(0x28c + 16 * k as usize) > 0.0 {
        x0 = signed_pow(clamp(x4, zero, x7), 0.25);
    }
    let x2m = if x0 > x7 { x0 } else { x7 };
    x12 = if x0 < x7 { x0 } else { x7 };
    x6 -= x7;
    let x1 = x7 - x0;
    x6 *= x1;
    x0 -= x6;
    let keep_min = x12 > x0;
    if !keep_min {
        x12 = if x2m < x0 { x2m } else { x0 };
    }
    r.set_f32(0xa4 + 4 * rdi, x12);
    x2 = r.f32(0xa0);
    r.set_f32(0x144 + 4 * rdi, x2);
    if r.f32(0xa0) > x8 {
        x2 -= x8;
        x6 = x2;
        r.set_f32(0x144 + 4 * rdi, x6);
        let top = (p.i32(0x8c) - 1) as f32;
        let ramp = interpolate_clamped(zero, zero, top, x7, fr.f(0x154));
        x2 = ramp * x6;
        x2 += x8;
        r.set_f32(0x144 + 4 * rdi, x2);
    }
    let x1 = fr.f(0x158);
    let x0 = if 0.0 > x1 {
        zero
    } else if x7 < x1 {
        x7
    } else {
        x1
    };
    x9 = (f64::from(x0) * 0.2) as f32;
    fr.set(0x98, x9);
    let x4 = fr.f(0x184) * fr.f(0x6c) + fr.f(0x160) * fr.f(0x100);
    let a7 = fr.f(0x164);
    x10 = fr.f(-0x10);
    let x1 = interpolate_clamped(a7, fr.f(0x134), x10, fr.f(0x13c), x4);
    x12 = x12 * x2 * x1;
    let phase = if env.engine_flag() {
        0.0
    } else {
        env.time_phase()
    };
    x6 = (phase * 4.0) as f32;
    let x7d = f64::from(x6);
    let seed = fr.i(8);
    let n1 = env.noise2(fr.f(0x168), (x7d + x7d) as f32, seed);
    let mut x8d = f64::from(n1) * 0.5;
    let n2 = env.noise2(fr.f(0x178), x6, seed);
    x8d += f64::from(n2);
    let n3 = env.noise2(fr.f(0x17c), (x7d * 4.0) as f32, seed);
    x8d += f64::from(n3) * 0.25;
    let n4 = env.noise2(fr.f(0x180), (x7d * 8.0) as f32, seed);
    x8d += f64::from(n4) * 0.125;
    let n5 = env.noise2(fr.f(0x15c), (x7d * 16.0) as f32, seed);
    x8d += f64::from(n5) * 0.0625;
    let n6 = env.noise2(fr.f(0x188), (x7d * 32.0) as f32, seed);
    x8d += f64::from(n6) * 0.031_25;
    x2 = x8d as f32;
    let x0 = x12 * r.f32(0x284 + 4 * (s30 + k as usize * 4));
    x2 *= x9;
    let x1d = f64::from(x0) * (f64::from(x2) + 1.0);
    r.set_f32(0x1e4 + 4 * rdi, x1d as f32);
    let regs = Regs::with(
        &[
            (6, x6),
            (9, x9),
            (10, x10),
            (11, x11),
            (12, x12),
            (13, x13),
            (15, x15),
        ],
        r15,
    );
    if stop == Stop::Segment3 {
        return Ok((fr, regs));
    }
    Err("rest of the loop not ported".into())
}
