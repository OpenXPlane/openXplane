//! Typed fields confirmed against the provided X-Plane 12.4.3.11 binary.
//! Addresses and instruction chains are recorded in research/ACF_LOADING.md.
use crate::Aircraft;

// Keep the actual float32 constants used by this reference build, including its rounding.
pub const ACF_LB_TO_KG: f32 = f32::from_bits(0x3ee83d47);
pub const ACF_FT_TO_M: f32 = f32::from_bits(0x3e9c0ebf);

#[derive(Debug)]
pub struct AircraftParameters {
    pub empty_mass_kg: f32,
    pub maximum_mass_kg: f32,
    pub maximum_fuel_mass_kg: f32,
    /// ACF reference coordinates, not transformed into a physics frame.
    pub cg_y_acf_m: f32,
    pub cg_z_acf_m: f32,
    pub engine_count: i32,
}

impl AircraftParameters {
    pub fn from_acf(aircraft: &Aircraft) -> Result<Self, String> {
        if aircraft.version != 1200 {
            return Err(format!(
                "typed parameters currently require ACF 1200, got {}",
                aircraft.version
            ));
        }
        let mass = |key| -> Result<f32, String> {
            let value = aircraft.float_property(key)? * ACF_LB_TO_KG;
            if !value.is_finite() || value < 0.0 {
                return Err(format!("invalid converted mass: {key}"));
            }
            Ok(value)
        };
        let coordinate = |key| -> Result<f32, String> {
            let value = aircraft.float_property(key)? * ACF_FT_TO_M;
            if !value.is_finite() {
                return Err(format!("invalid converted coordinate: {key}"));
            }
            Ok(value)
        };
        let engine_property = aircraft.unique_property("acf/_num_engn")?;
        let engine_count = engine_property
            .value
            .parse::<i32>()
            .map_err(|_| "invalid engine count")?;
        if engine_count < 0 {
            return Err("negative engine count".into());
        }
        Ok(Self {
            empty_mass_kg: mass("acf/_m_empty")?,
            maximum_mass_kg: mass("acf/_m_max")?,
            maximum_fuel_mass_kg: mass("acf/_m_fuel_max_tot")?,
            cg_y_acf_m: coordinate("acf/_cgY")?,
            cg_z_acf_m: coordinate("acf/_cgZ")?,
            engine_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(extra: &str) -> Aircraft {
        Aircraft::parse(&format!("I\n1200 Version\nACF\nPROPERTIES_BEGIN\nP acf/_m_empty 1721\nP acf/_m_max 2558\nP acf/_m_fuel_max_tot 318\nP acf/_cgY 0.3\nP acf/_cgZ 2.9\nP acf/_num_engn 1\n{extra}PROPERTIES_END\n")).unwrap()
    }
    #[test]
    fn uses_reference_float32_mass_and_length_scales() {
        let p = AircraftParameters::from_acf(&sample("")).unwrap();
        assert_eq!(
            p.empty_mass_kg.to_bits(),
            (1721_f32 * f32::from_bits(0x3ee83d47)).to_bits()
        );
        assert!((p.empty_mass_kg - 780.6327).abs() < 0.001);
        assert!((p.cg_z_acf_m - 0.88392).abs() < 0.00001);
        assert_eq!(p.engine_count, 1);
    }
    #[test]
    fn rejects_ambiguous_and_nonfinite_properties() {
        assert!(AircraftParameters::from_acf(&sample("P acf/_m_empty 999\n")).is_err());
        let mut a = sample("");
        a.properties[0].value = "NaN".into();
        assert!(AircraftParameters::from_acf(&a).is_err());
        a.properties[0].value = "-1".into();
        assert!(AircraftParameters::from_acf(&a).is_err());
        a.properties.remove(0);
        assert!(AircraftParameters::from_acf(&a).is_err());
    }
}
