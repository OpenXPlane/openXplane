//! The atmosphere accessors of the reference build, for the table path: the object at `F+0xbfa8` and the
//! table of 0x802 (temperature-pressure) pairs that the weather system fills at `0x14612bd90`.
//! The path taken when `obj+0x64` is above absolute zero (a layered temperature blend from `obj+0x44..0x58`)
//! is not ported; [`Atmosphere::temperature`] returns `None` then.
use crate::element_force::Mem;
use crate::wing_element::interpolate_clamped;

/// One table entry: `.0` is read by the density accessor (the pressure side), `.1` by the temperature
/// accessor (degrees Celsius).
pub type Entry = (f32, f32);

pub struct Atmosphere<'a> {
    pub object: &'a dyn Mem,
    pub table: &'a [Entry],
}

const KELVIN: f32 = 273.15;
const ISA_FEET: [f32; 13] = [
    0.0, 1772.0, 3243.0, 6392.0, 9878.0, 13801.0, 18281.0, 23564.0, 30053.0, 33999.0, 39000.0,
    44600.0, 53082.0,
];
const LAPSE: f64 = 0.006_500_000_134_110_451;
const EXPONENT: f64 = 5.251_390_945_716_319;
const INVERSE_EXPONENT: f64 = 0.190_425_738_692_283_63;

impl Atmosphere<'_> {
    /// The table position of an altitude in metres: the float position and the clamped entry index (steps of
    /// 100 m from -5000 m).
    fn position(altitude: f32) -> (f32, usize) {
        let position = ((f64::from(altitude) + 5000.0) / 100.0) as f32;
        // `cvttss2si`: NaN and values outside the 32-bit range become 0x80000000
        let index = if position > -2147483648.0 && position < 2147483648.0 {
            position as i32
        } else {
            i32::MIN
        };
        (position, index.clamp(0, 0x801) as usize)
    }

    /// The table's temperature at an altitude (the first call of `0x141ba6750` in its layered path).
    fn table_temperature(&self, altitude: f32) -> f32 {
        let (position, i) = Self::position(altitude);
        interpolate_clamped(
            i as f32,
            self.table[i].1,
            (i + 1) as f32,
            self.table[i + 1].1,
            position,
        )
    }

    /// `0x1409e7540(obj, v)`: the offset curve `obj+0x64+4k` over the layer altitudes `obj+0x30+4k` (13 each)
    /// at `v`: the two layers around `v` (the first altitude at or above `v`, the last below it), linear
    /// between them and held to their values.
    fn layer_offset(&self, v: f32) -> f32 {
        let p = |k: i32| self.object.f32(0x30 + 4 * k as usize);
        let q = |k: i32| self.object.f32(0x64 + 4 * k as usize);
        let mut hi = -1;
        for k in (0..=12).rev() {
            if p(k) >= v {
                hi = k;
            }
        }
        let mut lo = -1;
        for k in 0..=12 {
            if v > p(k) {
                lo = k;
            }
        }
        if hi == -1 {
            if lo == -1 {
                return q(0);
            }
            hi = lo;
        } else if lo == -1 {
            lo = hi;
        }
        let (p_hi, q_hi, p_lo, q_lo) = (p(hi), q(hi), p(lo), q(lo));
        if p_lo == p_hi {
            return (q_lo + q_hi) * 0.5;
        }
        let low = if q_lo < q_hi { q_lo } else { q_hi };
        let t = (q_hi - q_lo) / (p_hi - p_lo) * (v - p_lo) + q_lo;
        if low > t {
            return low;
        }
        let high = if q_lo > q_hi { q_lo } else { q_hi };
        if high < t { high } else { t }
    }

    /// `0x141ba6290(obj, v)`: the offset of the standard temperature by the altitude `v` (metres, turned into
    /// feet): a piecewise-linear curve of `obj+0xa0+4k` over the 13 altitudes of `ISA_FEET`.
    pub fn isa_offset(&self, v: f32) -> f32 {
        const FEET: f32 = f32::from_bits(0x4051_f948);
        let x = v * FEET;
        let a = |k: usize| self.object.f32(0xa0 + 4 * k);
        if 0.0 > x || x.partial_cmp(&ISA_FEET[1]) != Some(std::cmp::Ordering::Greater) {
            return interpolate_clamped(0.0, a(0), ISA_FEET[1], a(1), x);
        }
        if ISA_FEET[11].partial_cmp(&x) != Some(std::cmp::Ordering::Greater) {
            return interpolate_clamped(ISA_FEET[11], a(11), ISA_FEET[12], a(12), x);
        }
        for i in 1..=10 {
            if ISA_FEET[i] > x
                || !matches!(
                    x.partial_cmp(&ISA_FEET[i + 1]),
                    Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                )
            {
                continue;
            }
            return interpolate_clamped(ISA_FEET[i], a(i), ISA_FEET[i + 1], a(i + 1), x);
        }
        0.0
    }

    /// `0x141ba6750`: the temperature at an altitude (Celsius). With `obj+0x64` at or below absolute zero it is
    /// the table's. Otherwise the layered path: the top layer `k` (the highest of `obj+0x34..0x60` that is
    /// positive), the target `obj+0x24c` and the end value `obj+0x250` give a blend between the table
    /// temperature plus the offset curve ([`Self::layer_offset`]) at the layer altitude and the end value;
    /// without a target, or when the altitude is outside the blend, the table temperature plus the offset
    /// curve at the altitude itself.
    pub fn temperature(&self, altitude: f32) -> Option<f32> {
        if -273.15f32 >= self.object.f32(0x64) {
            return Some(self.table_temperature(altitude));
        }
        let p = |k: usize| self.object.f32(0x30 + 4 * k);
        let (x, end) = (self.object.f32(0x24c), self.object.f32(0x250));
        let plain = |s: &Self, v: f32| s.table_temperature(v) + s.layer_offset(v);
        let mut top = 0;
        for k in 1..=12 {
            if p(k) > 0.0 {
                top = k;
            }
        }
        let top_value = p(top);
        if x == 0.0 {
            return Some(plain(self, altitude));
        }
        if x >= top_value {
            // at or above the top layer
            if top_value >= altitude {
                return Some(plain(self, altitude));
            }
            if x >= altitude {
                let sum = plain(self, top_value);
                return Some(interpolate_clamped(top_value, sum, x, end, altitude));
            }
            let x2 = x / 11019.0;
            let end2 = end / -56.6;
            let (position, i) = Self::position(altitude / x2);
            let v = interpolate_clamped(
                i as f32,
                self.table[i].1,
                (i + 1) as f32,
                self.table[i + 1].1,
                position,
            );
            return Some(v * end2);
        }
        let mut layer = -1i32;
        for k in 0..=11 {
            if x >= p(k) && p(k + 1) > x {
                layer = k as i32;
            }
        }
        if layer < 0 {
            return Some(plain(self, altitude));
        }
        let (lower, upper) = (p(layer as usize), p(layer as usize + 1));
        if lower >= altitude {
            return Some(plain(self, altitude));
        }
        if x >= altitude {
            let sum = plain(self, lower);
            return Some(interpolate_clamped(lower, sum, x, end, altitude));
        }
        if !matches!(
            upper.partial_cmp(&altitude),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ) {
            return Some(plain(self, altitude));
        }
        let sum = plain(self, upper);
        Some(interpolate_clamped(x, end, upper, sum, altitude))
    }

    /// `0x141baf820`: the pressure at an altitude from the pressure setting `obj+0x98` and the temperature
    /// profile (barometric formula).
    pub fn pressure(&self, altitude: f32) -> Option<f32> {
        let kelvin = self.temperature(altitude)? + KELVIN;
        let setting = f64::from(self.object.f32(0x98));
        let lifted = f64::from(altitude * LAPSE as f32);
        let ratio = lifted / (lifted + f64::from(kelvin));
        let lowered = (1.0 - ratio).powf(-EXPONENT);
        let adjusted = setting / lowered / 100.0 - 0.3;
        let standard = 1013.25f64.powf(INVERSE_EXPONENT);
        let scaled = standard * LAPSE / f64::from(288.15f32) * f64::from(altitude);
        let corrected = (scaled / adjusted.powf(INVERSE_EXPONENT) + 1.0).powf(EXPONENT);
        Some((corrected * (adjusted * 100.0)) as f32)
    }

    /// `0x141ba64e0`: the density ratio at an altitude for the outside temperature `celsius`: the table's
    /// pressure over the standard pressure, divided by the ratio of the temperatures.
    pub fn density_ratio(&self, altitude: f32, celsius: f32) -> Option<f32> {
        let reference = self.pressure(self.object.f32(0x9c))? / 101_325.0f32;
        let (position, i) = Self::position(altitude);
        let table_temperature = interpolate_clamped(
            i as f32,
            self.table[i].1,
            (i + 1) as f32,
            self.table[i + 1].1,
            position,
        );
        let temperature_ratio = (celsius + KELVIN) / (table_temperature + KELVIN);
        let pressure = interpolate_clamped(
            i as f32,
            self.table[i].0,
            (i + 1) as f32,
            self.table[i + 1].0,
            position,
        );
        Some(pressure * reference / temperature_ratio)
    }
}
