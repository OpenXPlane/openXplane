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
const LAPSE: f64 = 0.006_500_000_134_110_451;
const EXPONENT: f64 = 5.251_390_945_716_319;
const INVERSE_EXPONENT: f64 = 0.190_425_738_692_283_63;

impl Atmosphere<'_> {
    /// The table position of an altitude in metres: the float position and the clamped entry index (steps of
    /// 100 m from -5000 m).
    fn position(altitude: f32) -> (f32, usize) {
        let position = ((f64::from(altitude) + 5000.0) / 100.0) as f32;
        let index = position as i32;
        (position, index.clamp(0, 0x801) as usize)
    }

    /// `0x141ba6750`, table path: the temperature at an altitude (Celsius), or `None` for the layered path.
    pub fn temperature(&self, altitude: f32) -> Option<f32> {
        if -273.15f32 >= self.object.f32(0x64) {
            let (position, i) = Self::position(altitude);
            Some(interpolate_clamped(
                i as f32,
                self.table[i].1,
                (i + 1) as f32,
                self.table[i + 1].1,
                position,
            ))
        } else {
            None
        }
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
