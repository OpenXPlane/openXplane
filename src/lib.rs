//! Initial ACF reader. Preserve unknown properties rather than guessing their meaning.
pub mod aero;
pub mod aircraft;
pub mod airfoil;
pub mod apt;
pub mod buffet;
pub mod commands;
pub mod dataref;
pub mod discord;
pub mod flight;
pub mod install;
pub mod obj8;
pub mod profile;
pub mod regimes;
pub mod runtime;
pub mod stall;
pub mod wing;
pub mod world;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Property {
    pub key: String,
    pub value: String,
    pub line: usize,
}

#[derive(Debug)]
pub struct Aircraft {
    pub version: u32,
    pub properties: Vec<Property>,
}

impl Aircraft {
    /// Duplicate precedence in the original has not been established, so typed reads reject ambiguity.
    pub fn unique_property(&self, key: &str) -> Result<&Property, String> {
        let mut matches = self.properties.iter().filter(|p| p.key == key);
        let property = matches
            .next()
            .ok_or_else(|| format!("missing ACF property: {key}"))?;
        if matches.next().is_some() {
            return Err(format!("duplicate ACF property: {key}"));
        }
        Ok(property)
    }

    pub fn float_property(&self, key: &str) -> Result<f32, String> {
        let property = self.unique_property(key)?;
        let value: f32 = property
            .value
            .parse()
            .map_err(|_| format!("invalid float {key} at line {}", property.line))?;
        if !value.is_finite() {
            return Err(format!("non-finite float {key} at line {}", property.line));
        }
        Ok(value)
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let mut lines = text.lines().enumerate();
        let marker = lines
            .next()
            .ok_or("empty ACF")?
            .1
            .trim_start_matches('\u{feff}');
        if !matches!(marker, "I" | "A") {
            return Err("unsupported ACF header marker".into());
        }
        let version_line = lines.next().ok_or("missing ACF version")?.1;
        let (version, suffix) = version_line.split_once(' ').ok_or("invalid ACF version")?;
        if suffix != "Version" {
            return Err("invalid ACF version header".into());
        }
        let version = version.parse().map_err(|_| "invalid ACF version number")?;
        if lines.next().ok_or("missing ACF type")?.1 != "ACF" {
            return Err("file is not an ACF".into());
        }
        let mut properties = Vec::new();
        let mut begun = false;
        let mut ended = false;
        for (index, line) in lines {
            if line == "PROPERTIES_BEGIN" {
                if begun {
                    return Err("duplicate property section".into());
                }
                begun = true;
                continue;
            }
            if line == "PROPERTIES_END" {
                if !begun {
                    return Err("property section ends before it begins".into());
                }
                ended = true;
                break;
            }
            if begun {
                if line.is_empty() {
                    continue;
                }
                let rest = line
                    .strip_prefix("P ")
                    .ok_or_else(|| format!("invalid property at line {}", index + 1))?;
                let (key, value) = rest.split_once(' ').unwrap_or((rest, ""));
                if key.is_empty() {
                    return Err(format!("empty property key at line {}", index + 1));
                }
                properties.push(Property {
                    key: key.into(),
                    value: value.into(),
                    line: index + 1,
                });
            }
        }
        if !begun || !ended {
            return Err("incomplete ACF property section".into());
        }
        Ok(Self {
            version,
            properties,
        })
    }

    /// No duplicate precedence is assumed: all properties remain available in order.
    pub fn references(&self) -> impl Iterator<Item = &Property> {
        self.properties
            .iter()
            .filter(|p| p.key.contains("/_afl_file_") || p.key.ends_with("/_v10_att_file_stl"))
            .filter(|p| !p.value.is_empty())
    }
}

/// Candidates for the observed aircraft layout. Existence does not establish runtime semantics.
pub fn reference_candidates(
    aircraft_dir: &Path,
    content_root: &Path,
    property: &Property,
) -> Vec<PathBuf> {
    let value = property.value.replace('\\', "/");
    if property.key.contains("/_afl_file_") {
        vec![
            aircraft_dir.join("airfoils").join(&value),
            content_root.join("Airfoils").join(&value),
        ]
    } else {
        vec![aircraft_dir.join("objects").join(value)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_strings_empty_values_and_duplicate_keys() {
        let acf = Aircraft::parse("I\r\n1200 Version\r\nACF\r\n\r\nPROPERTIES_BEGIN\r\nP name Cessna 172 SP\r\nP empty \r\nP name second\r\nPROPERTIES_END\r\nPANEL_3D_BEGIN\r\n").unwrap();
        assert_eq!(acf.version, 1200);
        assert_eq!(acf.properties.len(), 3);
        assert_eq!(acf.properties[0].value, "Cessna 172 SP");
        assert_eq!(acf.properties[1].value, "");
        assert_eq!(acf.properties[2].key, "name");
    }
    #[test]
    fn refuses_truncated_or_wrong_format() {
        assert!(Aircraft::parse("I\n1200 Version\nACF\nPROPERTIES_BEGIN\nP key value\n").is_err());
        assert!(
            Aircraft::parse("I\n1200 Version\nOBJ\nPROPERTIES_BEGIN\nPROPERTIES_END\n").is_err()
        );
    }
    #[test]
    fn resolves_spaces_and_parent_object_paths() {
        let p = Property {
            key: "_obja/0/_v10_att_file_stl".into(),
            value: "../Cockpit with spaces.obj".into(),
            line: 1,
        };
        assert_eq!(
            reference_candidates(Path::new("root/plane"), Path::new("root"), &p)[0],
            PathBuf::from("root/plane/objects/../Cockpit with spaces.obj")
        );
        let p = Property {
            key: "_wing/0/_afl_file_1".into(),
            value: "NACA 2412 (popular).afl".into(),
            line: 1,
        };
        assert_eq!(
            reference_candidates(Path::new("root/plane"), Path::new("root"), &p)[1],
            PathBuf::from("root/Airfoils/NACA 2412 (popular).afl")
        );
    }
}
