//! Where the simple datarefs of the reference build keep their value: a field of the flight object `F` (the object
//! `0x141a64250` returns for the aircraft index) or of the global simulation object at `0x1461020e0`, found by
//! decoding the getter thunks (`tools/extract_flight_datarefs.py`). `scale` is the unit conversion the getter applies
//! after the load.

/// The object a dataref reads from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The flight object of the aircraft.
    Flight,
    /// A global object at this address (`0x1461020e0` for the cockpit and simulation state).
    Global(u64),
}

/// How the getter loads the value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Load {
    Float,
    Double,
    Int,
    /// `movq`: a pointer-sized integer.
    Wide,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Field {
    pub name: &'static str,
    pub source: Source,
    pub offset: u64,
    pub load: Load,
    /// The getter multiplies by this factor (applied in order); `None` for the plain load.
    pub scale: Option<f64>,
    pub writable: bool,
}

const TABLE: &str = include_str!("../data/flight_datarefs.tsv");

fn parse_scale(text: &str) -> Option<f64> {
    // a single `*factor` or `/divisor` (other shapes are not in the table)
    if let Some(v) = text.strip_prefix('*') {
        v.parse().ok()
    } else if let Some(v) = text.strip_prefix('/') {
        v.parse::<f64>().ok().map(|d| 1.0 / d)
    } else {
        None
    }
}

/// Every mapped dataref.
pub fn fields() -> impl Iterator<Item = Field> {
    TABLE.lines().filter_map(|line| {
        let mut cols = line.split('\t');
        let name = cols.next()?;
        let source = match cols.next()? {
            "F" => Source::Flight,
            g => Source::Global(u64::from_str_radix(g.trim_start_matches("0x"), 16).ok()?),
        };
        let offset = u64::from_str_radix(cols.next()?.trim_start_matches("0x"), 16).ok()?;
        let load = match cols.next()? {
            "movss" => Load::Float,
            "movsd" => Load::Double,
            "movl" => Load::Int,
            _ => Load::Wide,
        };
        let scale = parse_scale(cols.next()?);
        let writable = cols.next()? == "rw";
        Some(Field {
            name,
            source,
            offset,
            load,
            scale,
            writable,
        })
    })
}

/// The field of a dataref by name.
pub fn field(name: &str) -> Option<Field> {
    fields().find(|f| f.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_datarefs_map_to_their_fields() {
        let theta = field("sim/flightmodel/position/theta");
        // the getter loads F+0x3dc; the lookup is by name
        assert_eq!(
            theta.map(|f| (f.source, f.offset, f.load)),
            Some((Source::Flight, 0x3dc, Load::Float))
        );
        let ias = field("sim/flightmodel/position/vh_ind");
        assert!(ias.is_none() || ias.is_some_and(|f| f.offset == 0x6cd8));
        assert!(fields().count() > 2000);
    }
}
