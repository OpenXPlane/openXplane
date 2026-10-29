//! Named-light table (`Resources/bitmaps/world/lites/lights.txt`). Parsing only: nothing renders it yet.
//! Column layouts follow the header table in the installed file.
use std::collections::BTreeMap;

/// One table cell: a number, a candela intensity (`5000cd`) or a parameter placeholder such as `DX`.
#[derive(Clone, Debug, PartialEq)]
pub enum Field {
    Number(f32),
    Candela(f32),
    Param(String),
}

impl Field {
    fn parse(token: &str) -> Self {
        if let Some(value) = token.strip_suffix("cd").and_then(|v| v.parse::<f32>().ok()) {
            Self::Candela(value)
        } else if let Ok(value) = token.parse::<f32>() {
            Self::Number(value)
        } else {
            Self::Param(token.to_string())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overload {
    BillboardHw,
    BillboardSw,
    SpillHwDir,
    SpillHwFla,
    SpillSw,
    SpillGnd,
    SpillGndRev,
}

#[derive(Clone, Debug)]
pub struct LightSpec {
    pub overload: Overload,
    pub name: String,
    /// Columns after the name, in file order, named by [`Overload::columns`].
    pub fields: Vec<Field>,
}

impl Overload {
    fn from_command(command: &str) -> Option<Self> {
        Some(match command {
            "BILLBOARD_HW" => Self::BillboardHw,
            "BILLBOARD_SW" => Self::BillboardSw,
            "SPILL_HW_DIR" => Self::SpillHwDir,
            "SPILL_HW_FLA" => Self::SpillHwFla,
            "SPILL_SW" => Self::SpillSw,
            "SPILL_GND" => Self::SpillGnd,
            "SPILL_GND_REV" => Self::SpillGndRev,
            _ => return None,
        })
    }

    /// Column names after the light name, from the header table of the installed file.
    pub fn columns(self) -> &'static [&'static str] {
        match self {
            Self::BillboardHw => &[
                "R",
                "G",
                "B",
                "A",
                "SIZE",
                "CELL_SIZE",
                "CELL_ROW",
                "CELL_COL",
                "DX",
                "DY",
                "DZ",
                "WIDTH",
                "FREQ",
                "PHASE",
                "AMP",
                "DAY",
            ],
            Self::BillboardSw => &[
                "R",
                "G",
                "B",
                "A",
                "SIZE",
                "CELL_SIZE",
                "CELL_ROW",
                "CELL_COL",
                "DX",
                "DY",
                "DZ",
                "WIDTH",
                "DREF",
            ],
            Self::SpillHwDir => &["R", "G", "B", "A", "SIZE", "DX", "DY", "DZ", "WIDTH", "DAY"],
            Self::SpillHwFla => &["R", "G", "B", "A", "SIZE", "FREQ", "PHASE", "AMP", "DAY"],
            Self::SpillSw => &[
                "R", "G", "B", "A", "SIZE", "DX", "DY", "DZ", "WIDTH", "DREF",
            ],
            Self::SpillGnd | Self::SpillGndRev => &["SIZE", "CELL_SIZE", "CELL_ROW", "CELL_COL"],
        }
    }
}

#[derive(Debug, Default)]
pub struct LightTable {
    pub texture: Option<String>,
    pub x_divisions: Option<u32>,
    pub y_divisions: Option<u32>,
    pub specs: Vec<LightSpec>,
    /// Records whose column count differs from the header layout (kept verbatim, not dropped).
    pub irregular: Vec<String>,
    index: BTreeMap<(String, bool), usize>,
}

impl LightTable {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut table = Self::default();
        let mut records = text
            .lines()
            .map(|l| l.split('#').next().unwrap().trim())
            .filter(|l| !l.is_empty());
        let marker = records.next().ok_or("empty light table")?;
        if !matches!(marker.trim_start_matches('\u{feff}'), "A" | "I")
            || records.next() != Some("850")
            || records.next() != Some("LIGHT_SPECS")
        {
            return Err("unsupported light table header".into());
        }
        for record in records {
            let tokens: Vec<_> = record.split_whitespace().collect();
            match tokens[0] {
                "TEXTURE" => table.texture = tokens.get(1).map(|t| (*t).to_string()),
                "X_DIVISIONS" => table.x_divisions = tokens.get(1).and_then(|t| t.parse().ok()),
                "Y_DIVISIONS" => table.y_divisions = tokens.get(1).and_then(|t| t.parse().ok()),
                command => {
                    let Some(overload) = Overload::from_command(command) else {
                        continue;
                    };
                    let name = tokens.get(1).ok_or("light record without a name")?;
                    let fields: Vec<_> = tokens[2..].iter().map(|t| Field::parse(t)).collect();
                    if fields.len() != overload.columns().len() {
                        table.irregular.push(record.to_string());
                    }
                    table
                        .index
                        .insert(((*name).to_string(), is_spill(overload)), table.specs.len());
                    table.specs.push(LightSpec {
                        overload,
                        name: (*name).to_string(),
                        fields,
                    });
                }
            }
        }
        Ok(table)
    }

    /// Last definition of `name` of the billboard (`spill == false`) or spill kind.
    pub fn get(&self, name: &str, spill: bool) -> Option<&LightSpec> {
        self.index
            .get(&(name.to_string(), spill))
            .map(|&i| &self.specs[i])
    }
}

impl LightTable {
    /// Billboard and spill definitions of a named light (a name may have both).
    pub fn named(&self, name: &str) -> (Option<&LightSpec>, Option<&LightSpec>) {
        (self.get(name, false), self.get(name, true))
    }
}

/// Flash brightness in 0..=1 at `time` seconds, following the rules documented in the header of the
/// installed `lights.txt` (triangle wave; amplitude > 1 subtracts `amp - 1` for a strobe; amplitude < 1
/// lifts the dark level to `amp`; amplitude > 1000 is a square wave with pulse width `(amp - 1000) / 1000`).
/// These are the documented semantics, not yet compared with the original code.
pub fn flash_level(freq: f32, phase: f32, amp: f32, time: f32) -> f32 {
    if freq <= 0.0 {
        return 1.0;
    }
    let cycle = ((time - phase) * freq).rem_euclid(1.0);
    if amp > 1000.0 {
        return if cycle < (amp - 1000.0) / 1000.0 {
            1.0
        } else {
            0.0
        };
    }
    let triangle = 1.0 - (2.0 * cycle - 1.0).abs();
    let level = if amp > 1.0 {
        triangle * amp - (amp - 1.0)
    } else if amp < 1.0 {
        amp + triangle * (1.0 - amp)
    } else {
        triangle
    };
    level.clamp(0.0, 1.0)
}

fn is_spill(overload: Overload) -> bool {
    !matches!(overload, Overload::BillboardHw | Overload::BillboardSw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_levels_follow_documented_amplitude_rules() {
        // Pulsing triangle: dark at the cycle start, full at the middle.
        assert_eq!(flash_level(1.0, 0.0, 1.0, 0.0), 0.0);
        assert!((flash_level(1.0, 0.0, 1.0, 0.5) - 1.0).abs() < 1e-6);
        // Strobe: amplitude 10 is lit only near the peak.
        assert_eq!(flash_level(1.0, 0.0, 10.0, 0.25), 0.0);
        assert!(flash_level(1.0, 0.0, 10.0, 0.5) > 0.99);
        // Amplitude 0.5 never drops below 50 %.
        assert!((flash_level(1.0, 0.0, 0.5, 0.0) - 0.5).abs() < 1e-6);
        // Square wave, 50 % width, and phase shift.
        assert_eq!(flash_level(1.0, 0.0, 1500.0, 0.25), 1.0);
        assert_eq!(flash_level(1.0, 0.0, 1500.0, 0.75), 0.0);
        assert_eq!(flash_level(1.0, 0.5, 1500.0, 0.75), 1.0);
        // No frequency means steady.
        assert_eq!(flash_level(0.0, 0.0, 1.0, 3.0), 1.0);
    }

    const SAMPLE: &str = "A\n850\nLIGHT_SPECS\nTEXTURE lights.dds\nX_DIVISIONS 16\nY_DIVISIONS 8\n\
BILLBOARD_HW\ttaillight\t1.0\t0.05\t0.00\t1.0\t100cd\t1 6 6\t0 0 0.99 -0.4 0 0 0 0\n\
SPILL_HW_FLA\tCL_r_3\t0.997 0.05 0.05 1 45cd 0.008 0.25 1750 1\n\
SPILL_GND\theli_y_halo\t1 1 1 3\n\
SPILL_HW_DIR\tarea_lt_param_sp\t0.85 0.75 1.0 1.0 SIZE DX DY DZ 0.3\n";

    #[test]
    fn parses_columns_candela_and_parameters() {
        let t = LightTable::parse(SAMPLE).unwrap();
        assert_eq!(t.texture.as_deref(), Some("lights.dds"));
        assert_eq!((t.x_divisions, t.y_divisions), (Some(16), Some(8)));
        assert_eq!(t.specs.len(), 4);
        let tail = t.get("taillight", false).unwrap();
        assert_eq!(tail.fields.len(), Overload::BillboardHw.columns().len());
        assert_eq!(tail.fields[4], Field::Candela(100.0));
        assert_eq!(tail.fields[5], Field::Number(1.0));
        let flash = t.get("CL_r_3", true).unwrap();
        assert_eq!(flash.fields[4], Field::Candela(45.0));
        let param = t.get("area_lt_param_sp", true).unwrap();
        assert_eq!(param.fields[4], Field::Param("SIZE".into()));
        // The nine-column SPILL_HW_DIR record is kept but flagged irregular.
        assert_eq!(t.irregular.len(), 1);
    }

    #[test]
    #[ignore = "requires read-only installed X-Plane via OPENXPLANE_REFERENCE_ROOT"]
    fn installed_light_table_parses() {
        let root = std::env::var_os("OPENXPLANE_REFERENCE_ROOT").expect("reference root required");
        let path = std::path::Path::new(&root).join("Resources/bitmaps/world/lites/lights.txt");
        let t = LightTable::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
        println!("{} specs, {} irregular", t.specs.len(), t.irregular.len());
        assert!(t.specs.len() > 800);
        assert!(t.get("taillight", false).is_some());
    }

    #[test]
    #[ignore = "requires read-only installed UHWW scenery via OPENXPLANE_REFERENCE_ROOT"]
    fn installed_uhww_named_lights_resolve_in_the_table() {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("obj"))
                {
                    out.push(path);
                }
            }
        }
        let root = std::path::PathBuf::from(
            std::env::var_os("OPENXPLANE_REFERENCE_ROOT").expect("reference root required"),
        );
        let table = LightTable::parse(
            &std::fs::read_to_string(root.join("Resources/bitmaps/world/lites/lights.txt"))
                .unwrap(),
        )
        .unwrap();
        let mut files = Vec::new();
        walk(&root.join("Custom Scenery/1_UHWW"), &mut files);
        let (mut named, mut missing) = (0, std::collections::BTreeSet::new());
        for path in files {
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Ok(object) = crate::Object::parse_bytes(&bytes) else {
                continue;
            };
            for light in &object.lights {
                if matches!(
                    light.kind,
                    crate::LightKind::Named | crate::LightKind::Param
                ) {
                    named += 1;
                    let name = light.name.as_deref().unwrap();
                    if matches!(table.named(name), (None, None)) {
                        missing.insert(name.to_string());
                    }
                }
            }
        }
        println!("{named} named lights, unresolved names: {missing:?}");
        assert!(named > 0);
    }
}
