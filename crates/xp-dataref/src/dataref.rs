//! Dataref registry: named, typed, optionally writable values.
//! Access is strictly typed. Cross-type conversions of the original (e.g. reading a
//! float dataref as int) have not been established, so a mismatch is an error.
//! The one confirmed conversion is a float32-exposed dataref backed by a double.
use std::collections::BTreeMap;
use xp_acf::aircraft::AircraftParameters;
use xp_airfoil::runtime::{RUNNING_TIME_DATAREF, RunningTime};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Int,
    Float,
    Double,
    IntArray,
    FloatArray,
    Data,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    Float(f32),
    Double(f64),
    IntArray(Vec<i32>),
    FloatArray(Vec<f32>),
    Data(Vec<u8>),
    /// Exposed as float32, stored as double (see `runtime`).
    FloatOverDouble(f64),
}

impl Value {
    pub fn data_type(&self) -> DataType {
        match self {
            Value::Int(_) => DataType::Int,
            Value::Float(_) | Value::FloatOverDouble(_) => DataType::Float,
            Value::Double(_) => DataType::Double,
            Value::IntArray(_) => DataType::IntArray,
            Value::FloatArray(_) => DataType::FloatArray,
            Value::Data(_) => DataType::Data,
        }
    }
}

#[derive(Debug)]
struct Entry {
    value: Value,
    writable: bool,
}

#[derive(Debug, Default)]
pub struct Registry {
    entries: BTreeMap<String, Entry>,
}

/// Datarefs backed by the ACF fields studied in research/ACF_LOADING.md. Type and writability come
/// from the registration records of the reference build (research/DATAREFS.md); each dataref's
/// accessor reads the same aircraft-object field that the ACF loader stores into.
pub const ACF_MASS_EMPTY: &str = "sim/aircraft/weight/acf_m_empty";
pub const ACF_MASS_MAX: &str = "sim/aircraft/weight/acf_m_max";
pub const ACF_MASS_FUEL_TOTAL: &str = "sim/aircraft/weight/acf_m_fuel_tot";
pub const ACF_CG_Y_ORIGINAL: &str = "sim/aircraft/weight/acf_cgY_original";
pub const ACF_CG_Z_ORIGINAL: &str = "sim/aircraft/weight/acf_cgZ_original";
pub const ACF_ENGINE_COUNT: &str = "sim/aircraft/engine/acf_num_engines";

fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() || !name.contains('/') || name.chars().any(char::is_whitespace) {
        return Err(format!("invalid dataref name '{name}'"));
    }
    Ok(())
}

fn finite(value: f64, name: &str) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!("non-finite value for dataref {name}"))
    }
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, name: &str, value: Value, writable: bool) -> Result<(), String> {
        check_name(name)?;
        match &value {
            Value::Float(v) => finite(f64::from(*v), name)?,
            Value::Double(v) | Value::FloatOverDouble(v) => finite(*v, name)?,
            Value::FloatArray(v) => v.iter().try_for_each(|x| finite(f64::from(*x), name))?,
            _ => {}
        }
        if self.entries.contains_key(name) {
            return Err(format!("duplicate dataref: {name}"));
        }
        self.entries
            .insert(name.to_string(), Entry { value, writable });
        Ok(())
    }

    /// Registers `sim/time/total_running_time_sec` as a float dataref over the double snapshot.
    pub fn register_running_time(
        &mut self,
        time: RunningTime,
        writable: bool,
    ) -> Result<(), String> {
        self.register(
            RUNNING_TIME_DATAREF,
            Value::FloatOverDouble(time.seconds()),
            writable,
        )
    }

    /// Registers the datarefs whose backing ACF fields are confirmed. All or nothing: a name
    /// that already exists leaves the registry unchanged.
    ///
    /// The ACF loader stores `_num_engn` into the engine-count field only when a flag in its load
    /// context is zero; that condition is not modelled, so the value is always stored here.
    pub fn register_aircraft(&mut self, p: &AircraftParameters) -> Result<(), String> {
        let entries = [
            (ACF_MASS_EMPTY, Value::Float(p.empty_mass_kg), true),
            (ACF_MASS_MAX, Value::Float(p.maximum_mass_kg), true),
            (
                ACF_MASS_FUEL_TOTAL,
                Value::Float(p.maximum_fuel_mass_kg),
                true,
            ),
            (ACF_CG_Y_ORIGINAL, Value::Float(p.cg_y_acf_m), false),
            (ACF_CG_Z_ORIGINAL, Value::Float(p.cg_z_acf_m), false),
            (ACF_ENGINE_COUNT, Value::Int(p.engine_count), true),
        ];
        if let Some((name, _, _)) = entries
            .iter()
            .find(|(n, _, _)| self.entries.contains_key(*n))
        {
            return Err(format!("duplicate dataref: {name}"));
        }
        for (name, value, writable) in entries {
            self.register(name, value, writable)?;
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    fn entry(&self, name: &str) -> Result<&Entry, String> {
        self.entries
            .get(name)
            .ok_or_else(|| format!("unknown dataref: {name}"))
    }

    fn entry_mut(&mut self, name: &str) -> Result<&mut Entry, String> {
        let entry = self
            .entries
            .get_mut(name)
            .ok_or_else(|| format!("unknown dataref: {name}"))?;
        if !entry.writable {
            return Err(format!("dataref is read-only: {name}"));
        }
        Ok(entry)
    }

    pub fn data_type(&self, name: &str) -> Result<DataType, String> {
        Ok(self.entry(name)?.value.data_type())
    }

    pub fn is_writable(&self, name: &str) -> Result<bool, String> {
        Ok(self.entry(name)?.writable)
    }

    pub fn get_int(&self, name: &str) -> Result<i32, String> {
        match self.entry(name)?.value {
            Value::Int(v) => Ok(v),
            _ => Err(mismatch(name, DataType::Int, self.data_type(name)?)),
        }
    }

    pub fn get_float(&self, name: &str) -> Result<f32, String> {
        match self.entry(name)?.value {
            Value::Float(v) => Ok(v),
            Value::FloatOverDouble(v) => Ok(v as f32),
            _ => Err(mismatch(name, DataType::Float, self.data_type(name)?)),
        }
    }

    pub fn get_double(&self, name: &str) -> Result<f64, String> {
        match self.entry(name)?.value {
            Value::Double(v) => Ok(v),
            _ => Err(mismatch(name, DataType::Double, self.data_type(name)?)),
        }
    }

    pub fn get_int_at(&self, name: &str, index: usize) -> Result<i32, String> {
        match &self.entry(name)?.value {
            Value::IntArray(v) => v
                .get(index)
                .copied()
                .ok_or_else(|| range(name, index, v.len())),
            _ => Err(mismatch(name, DataType::IntArray, self.data_type(name)?)),
        }
    }

    pub fn get_float_at(&self, name: &str, index: usize) -> Result<f32, String> {
        match &self.entry(name)?.value {
            Value::FloatArray(v) => v
                .get(index)
                .copied()
                .ok_or_else(|| range(name, index, v.len())),
            _ => Err(mismatch(name, DataType::FloatArray, self.data_type(name)?)),
        }
    }

    pub fn get_data(&self, name: &str) -> Result<&[u8], String> {
        match &self.entry(name)?.value {
            Value::Data(v) => Ok(v),
            _ => Err(mismatch(name, DataType::Data, self.data_type(name)?)),
        }
    }

    pub fn set_int(&mut self, name: &str, value: i32) -> Result<(), String> {
        let entry = self.entry_mut(name)?;
        match &mut entry.value {
            Value::Int(v) => *v = value,
            other => return Err(mismatch(name, DataType::Int, other.data_type())),
        }
        Ok(())
    }

    /// Setter widens to double for double-backed float datarefs; non-finite input is rejected.
    pub fn set_float(&mut self, name: &str, value: f32) -> Result<(), String> {
        finite(f64::from(value), name)?;
        let entry = self.entry_mut(name)?;
        match &mut entry.value {
            Value::Float(v) => *v = value,
            Value::FloatOverDouble(v) => *v = f64::from(value),
            other => return Err(mismatch(name, DataType::Float, other.data_type())),
        }
        Ok(())
    }

    pub fn set_double(&mut self, name: &str, value: f64) -> Result<(), String> {
        finite(value, name)?;
        let entry = self.entry_mut(name)?;
        match &mut entry.value {
            Value::Double(v) => *v = value,
            other => return Err(mismatch(name, DataType::Double, other.data_type())),
        }
        Ok(())
    }

    pub fn set_int_at(&mut self, name: &str, index: usize, value: i32) -> Result<(), String> {
        let entry = self.entry_mut(name)?;
        match &mut entry.value {
            Value::IntArray(v) => {
                let len = v.len();
                *v.get_mut(index).ok_or_else(|| range(name, index, len))? = value;
            }
            other => return Err(mismatch(name, DataType::IntArray, other.data_type())),
        }
        Ok(())
    }

    pub fn set_float_at(&mut self, name: &str, index: usize, value: f32) -> Result<(), String> {
        finite(f64::from(value), name)?;
        let entry = self.entry_mut(name)?;
        match &mut entry.value {
            Value::FloatArray(v) => {
                let len = v.len();
                *v.get_mut(index).ok_or_else(|| range(name, index, len))? = value;
            }
            other => return Err(mismatch(name, DataType::FloatArray, other.data_type())),
        }
        Ok(())
    }
}

fn mismatch(name: &str, wanted: DataType, actual: DataType) -> String {
    format!("dataref {name} is {actual:?}, not {wanted:?}")
}

fn range(name: &str, index: usize, len: usize) -> String {
    format!("index {index} out of range for dataref {name} (length {len})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_access_and_writability() {
        let mut r = Registry::new();
        r.register("sim/test/int", Value::Int(3), true).unwrap();
        r.register("sim/test/ro", Value::Float(1.5), false).unwrap();
        r.register("sim/test/arr", Value::FloatArray(vec![0.0; 2]), true)
            .unwrap();
        r.register("sim/test/blob", Value::Data(b"hi".to_vec()), false)
            .unwrap();
        assert_eq!(r.get_int("sim/test/int"), Ok(3));
        r.set_int("sim/test/int", 7).unwrap();
        assert_eq!(r.get_int("sim/test/int"), Ok(7));
        assert!(r.get_float("sim/test/int").is_err());
        assert!(
            r.set_float("sim/test/ro", 2.0)
                .unwrap_err()
                .contains("read-only")
        );
        r.set_float_at("sim/test/arr", 1, 4.0).unwrap();
        assert_eq!(r.get_float_at("sim/test/arr", 1), Ok(4.0));
        assert!(
            r.get_float_at("sim/test/arr", 2)
                .unwrap_err()
                .contains("out of range")
        );
        assert_eq!(r.get_data("sim/test/blob"), Ok(&b"hi"[..]));
        assert_eq!(r.data_type("sim/test/arr"), Ok(DataType::FloatArray));
        assert!(r.get_int("sim/missing").unwrap_err().contains("unknown"));
    }

    #[test]
    fn rejects_bad_names_duplicates_and_non_finite() {
        let mut r = Registry::new();
        assert!(r.register("noslash", Value::Int(0), true).is_err());
        assert!(r.register("sim/has space", Value::Int(0), true).is_err());
        assert!(r.register("sim/nan", Value::Float(f32::NAN), true).is_err());
        r.register("sim/x", Value::Int(0), true).unwrap();
        assert!(
            r.register("sim/x", Value::Int(1), true)
                .unwrap_err()
                .contains("duplicate")
        );
        r.register("sim/f", Value::Float(0.0), true).unwrap();
        assert!(r.set_float("sim/f", f32::INFINITY).is_err());
        assert_eq!(r.get_float("sim/f"), Ok(0.0));
    }

    fn params() -> AircraftParameters {
        AircraftParameters {
            empty_mass_kg: 780.5,
            maximum_mass_kg: 1160.25,
            maximum_fuel_mass_kg: 158.75,
            cg_y_acf_m: 0.09144,
            cg_z_acf_m: 0.88392,
            engine_count: 1,
        }
    }

    #[test]
    fn aircraft_datarefs_match_confirmed_types_and_writability() {
        let mut r = Registry::new();
        r.register_aircraft(&params()).unwrap();
        assert_eq!(r.get_float(ACF_MASS_EMPTY), Ok(780.5));
        assert_eq!(r.get_float(ACF_MASS_MAX), Ok(1160.25));
        assert_eq!(r.get_float(ACF_MASS_FUEL_TOTAL), Ok(158.75));
        assert_eq!(r.get_int(ACF_ENGINE_COUNT), Ok(1));
        assert_eq!(r.data_type(ACF_ENGINE_COUNT), Ok(DataType::Int));
        // cgY/cgZ "original" are read-only in the reference registration table.
        assert_eq!(r.is_writable(ACF_CG_Y_ORIGINAL), Ok(false));
        assert!(
            r.set_float(ACF_CG_Z_ORIGINAL, 1.0)
                .unwrap_err()
                .contains("read-only")
        );
        r.set_float(ACF_MASS_EMPTY, 800.0).unwrap();
        assert_eq!(r.get_float(ACF_MASS_EMPTY), Ok(800.0));
    }

    #[test]
    fn register_aircraft_is_all_or_nothing() {
        let mut r = Registry::new();
        r.register(ACF_MASS_MAX, Value::Float(1.0), true).unwrap();
        assert!(
            r.register_aircraft(&params())
                .unwrap_err()
                .contains("duplicate")
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r.get_float(ACF_MASS_MAX), Ok(1.0));
    }

    #[test]
    fn running_time_keeps_double_storage_behind_float_dataref() {
        let mut r = Registry::new();
        let time = RunningTime::from_snapshot(16_777_217.0).unwrap();
        r.register_running_time(time, true).unwrap();
        assert_eq!(r.get_float(RUNNING_TIME_DATAREF), Ok(16_777_216.0));
        assert_eq!(r.data_type(RUNNING_TIME_DATAREF), Ok(DataType::Float));
        r.set_float(RUNNING_TIME_DATAREF, 1.25).unwrap();
        assert_eq!(r.get_float(RUNNING_TIME_DATAREF), Ok(1.25));
        assert!(r.get_double(RUNNING_TIME_DATAREF).is_err());
    }
}
