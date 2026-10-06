//! The force and moment totals of the flight model (`0x1412656b0`, the block at `0x14126f37f`): the six
//! contributions (propeller, aerodynamics, landing gear, aircraft mass, plug-in) of the three forces and three
//! moments in aircraft axes, their optional replacement from the override area at `F+0x6780..0x67c0`, and the
//! totals. Offsets are those of the flight object `F` (research/FLIGHT_LAYOUT.md).
use std::collections::HashMap;

/// A flight object seen as float and integer words at byte offsets; unset words read as zero.
#[derive(Clone, Debug, Default)]
pub struct Words(pub HashMap<usize, u32>);

impl Words {
    pub fn f32(&self, offset: usize) -> f32 {
        f32::from_bits(self.0.get(&offset).copied().unwrap_or(0))
    }
    pub fn i32(&self, offset: usize) -> i32 {
        self.0.get(&offset).copied().unwrap_or(0) as i32
    }
    pub fn set_f32(&mut self, offset: usize, value: f32) {
        self.0.insert(offset, value.to_bits());
    }
    pub fn set_i32(&mut self, offset: usize, value: i32) {
        self.0.insert(offset, value as u32);
    }
    /// A double at `offset` (two consecutive words, low word first).
    pub fn f64(&self, offset: usize) -> f64 {
        let low = u64::from(self.0.get(&offset).copied().unwrap_or(0));
        let high = u64::from(self.0.get(&(offset + 4)).copied().unwrap_or(0));
        f64::from_bits(high << 32 | low)
    }
    pub fn set_f64(&mut self, offset: usize, value: f64) {
        let bits = value.to_bits();
        self.0.insert(offset, bits as u32);
        self.0.insert(offset + 4, (bits >> 32) as u32);
    }
    fn copy(&mut self, from: usize, to: usize) {
        let value = self.0.get(&from).copied().unwrap_or(0);
        self.0.insert(to, value);
    }
}

/// Per group (propeller, aerodynamics, gear): the flag offset and, for each of the six outputs
/// (normal, axial, side, L, M, N), the (source, destination) offsets of the override copy.
const OVERRIDES: [(usize, [(usize, usize); 6]); 3] = [
    (
        0x676c,
        [
            (0x6780, 0x2d0),
            (0x6784, 0x2bc),
            (0x677c, 0x2e4),
            (0x6788, 0x2f8),
            (0x678c, 0x310),
            (0x6790, 0x328),
        ],
    ),
    (
        0x6768,
        [
            (0x6798, 0x2d4),
            (0x679c, 0x2c0),
            (0x6794, 0x2e8),
            (0x67a0, 0x2fc),
            (0x67a4, 0x314),
            (0x67a8, 0x32c),
        ],
    ),
    (
        0x6770,
        [
            (0x67b0, 0x2d8),
            (0x67b4, 0x2c4),
            (0x67ac, 0x2ec),
            (0x67b8, 0x300),
            (0x67bc, 0x318),
            (0x67c0, 0x330),
        ],
    ),
];

/// Replaces the contributions of the groups whose override flag is set and computes the totals
/// (`+0x2cc` axial, `+0x2e0` normal, `+0x2f4` side, `+0x30c` L, `+0x324` M, `+0x33c` N). Nothing is done when
/// `F+0x675c` is set (the totals are then supplied from outside).
pub fn force_totals(f: &mut Words) {
    if f.i32(0x675c) != 0 {
        return;
    }
    for (flag, copies) in OVERRIDES {
        if f.i32(flag) != 0 {
            for (from, to) in copies {
                f.copy(from, to);
            }
        }
    }
    // each total adds the aerodynamic term first, then the propeller, gear, mass and plug-in terms
    let sum = |f: &Words, first: usize, rest: &[usize]| {
        rest.iter().fold(f.f32(first), |acc, &o| acc + f.f32(o))
    };
    let axial = sum(f, 0x2c0, &[0x2bc, 0x2c4, 0x2c8]);
    let normal = sum(f, 0x2d4, &[0x2d0, 0x2d8, 0x2dc]);
    let side = sum(f, 0x2e8, &[0x2e4, 0x2ec, 0x2f0]);
    let roll = sum(f, 0x2fc, &[0x2f8, 0x300, 0x304, 0x308]);
    let pitch = sum(f, 0x314, &[0x310, 0x318, 0x31c, 0x320]);
    let yaw = sum(f, 0x32c, &[0x328, 0x330, 0x334, 0x338]);
    f.set_f32(0x2e0, normal);
    f.set_f32(0x2cc, axial);
    f.set_f32(0x2f4, side);
    f.set_f32(0x30c, roll);
    f.set_f32(0x324, pitch);
    f.set_f32(0x33c, yaw);
}

impl crate::element_force::Mem for Words {
    fn f32(&self, offset: usize) -> f32 {
        Words::f32(self, offset)
    }
    fn i32(&self, offset: usize) -> i32 {
        Words::i32(self, offset)
    }
}
