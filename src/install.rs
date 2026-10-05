//! The folder layout of an X-Plane installation and the rules for which folder wins.
//!
//! Folder names are confirmed by strings in the reference build (research/INSTALL_LAYOUT.md):
//! `Aircraft/`, `Airfoils/`, `Custom Data/`, `Custom Scenery/`, `Global Scenery/`, `Output/`,
//! `Resources/default data/`, `Resources/default scenery/default apt dat/`. That `Custom Data/`
//! overrides `Resources/default data/` follows from the build's own warning text. Pack priority in
//! `scenery_packs.ini` (first line wins) is taken from X-Plane's public documentation and is NOT
//! confirmed in the reference build. The minimal "flat" layout used while only a few files were
//! provided (`Cessna 172 SP/`, `Airfoils/`, `Earth nav data/` next to the EXE) is also accepted.
use crate::apt::{self, Airport};
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

pub const APT_DAT: &str = "Earth nav data/apt.dat";

#[derive(Debug, Clone)]
pub struct Install {
    root: PathBuf,
}

impl Install {
    pub fn open(root: &Path) -> Result<Self, String> {
        if !root.is_dir() {
            return Err(format!("not a directory: {}", root.display()));
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn aircraft_dir(&self) -> PathBuf {
        self.root.join("Aircraft")
    }
    pub fn airfoils_dir(&self) -> PathBuf {
        self.root.join("Airfoils")
    }
    pub fn custom_data_dir(&self) -> PathBuf {
        self.root.join("Custom Data")
    }
    pub fn default_data_dir(&self) -> PathBuf {
        self.root.join("Resources").join("default data")
    }
    pub fn custom_scenery_dir(&self) -> PathBuf {
        self.root.join("Custom Scenery")
    }
    pub fn global_airports_dir(&self) -> PathBuf {
        self.root.join("Global Scenery").join("Global Airports")
    }
    pub fn default_apt_dir(&self) -> PathBuf {
        self.root
            .join("Resources")
            .join("default scenery")
            .join("default apt dat")
    }
    pub fn output_dir(&self) -> PathBuf {
        self.root.join("Output")
    }

    /// All `.acf` files: under `Aircraft/` in a standard installation, otherwise (flat layout)
    /// anywhere below the root. Sorted for stable output.
    pub fn find_acf(&self) -> Result<Vec<PathBuf>, String> {
        let start = if self.aircraft_dir().is_dir() {
            self.aircraft_dir()
        } else {
            self.root.clone()
        };
        let mut found = Vec::new();
        collect_acf(&start, &mut found).map_err(|e| format!("{}: {e}", start.display()))?;
        found.sort();
        Ok(found)
    }

    /// Navigation data file (`earth_nav.dat`, `earth_fix.dat`, `earth_awy.dat`): `Custom Data/` first,
    /// then `Resources/default data/`, then the flat `Earth nav data/` folder.
    pub fn nav_data_file(&self, name: &str) -> Option<PathBuf> {
        [
            self.custom_data_dir().join(name),
            self.default_data_dir().join(name),
            self.root.join("Earth nav data").join(name),
        ]
        .into_iter()
        .find(|p| p.is_file())
    }

    /// Candidate airfoil files for an ACF `_afl_file_*` value: the aircraft's own `airfoils/`
    /// first, then the shared `Airfoils/`.
    pub fn airfoil_candidates(&self, aircraft_dir: &Path, value: &str) -> Vec<PathBuf> {
        let value = value.replace('\\', "/");
        vec![
            aircraft_dir.join("airfoils").join(&value),
            self.airfoils_dir().join(&value),
        ]
    }

    /// Enabled scenery packs from `Custom Scenery/scenery_packs.ini`, in file order.
    /// Without the file, no pack is considered (X-Plane writes it itself).
    pub fn scenery_packs(&self) -> Result<Vec<PathBuf>, String> {
        let ini = self.custom_scenery_dir().join("scenery_packs.ini");
        let Ok(text) = fs::read_to_string(&ini) else {
            return Ok(Vec::new());
        };
        Ok(parse_scenery_packs(&text)
            .into_iter()
            .map(|p| self.root.join(p))
            .collect())
    }

    /// Every `apt.dat` that applies, highest priority first: enabled scenery packs in
    /// `scenery_packs.ini` order, then Global Airports, then the default apt.dat, then the
    /// flat `Earth nav data/apt.dat`. Only files that exist are returned.
    pub fn apt_dat_sources(&self) -> Result<Vec<PathBuf>, String> {
        let mut sources: Vec<PathBuf> = self
            .scenery_packs()?
            .into_iter()
            .map(|p| p.join(APT_DAT))
            .collect();
        sources.push(self.global_airports_dir().join(APT_DAT));
        sources.push(self.default_apt_dir().join(APT_DAT));
        sources.push(self.root.join(APT_DAT));
        sources.retain(|p| p.is_file());
        Ok(sources)
    }

    /// First airport with this ICAO id in priority order, with the file it came from.
    pub fn find_airport(&self, id: &str) -> Result<Option<(Airport, PathBuf)>, String> {
        for source in self.apt_dat_sources()? {
            let file = fs::File::open(&source).map_err(|e| format!("{}: {e}", source.display()))?;
            if let Some(airport) = apt::find_airport(BufReader::new(file), id)
                .map_err(|e| format!("{}: {e}", source.display()))?
            {
                return Ok(Some((airport, source)));
            }
        }
        Ok(None)
    }
}

fn collect_acf(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        if kind.is_dir() {
            collect_acf(&path, out)?;
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("acf"))
        {
            out.push(path);
        }
    }
    Ok(())
}

/// `SCENERY_PACK <path>` lines in file order; `SCENERY_PACK_DISABLED` lines are skipped.
/// Paths are relative to the installation root and end with `/` in the file.
pub fn parse_scenery_packs(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("SCENERY_PACK "))
        .map(|p| p.trim().replace('\\', "/"))
        .filter(|p| !p.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn temp_root() -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "openxplane-install-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn apt(id: &str, name: &str) -> String {
        format!("I\n1200 test\n\n1 0 0 0 {id} {name}\n99\n")
    }

    #[test]
    fn custom_data_overrides_default_data() {
        let root = temp_root();
        let install = Install::open(&root).unwrap();
        assert_eq!(install.nav_data_file("earth_nav.dat"), None);
        write(
            &root.join("Resources/default data/earth_nav.dat"),
            "default",
        );
        assert_eq!(
            install.nav_data_file("earth_nav.dat"),
            Some(root.join("Resources/default data/earth_nav.dat"))
        );
        write(&root.join("Custom Data/earth_nav.dat"), "custom");
        assert_eq!(
            install.nav_data_file("earth_nav.dat"),
            Some(root.join("Custom Data/earth_nav.dat"))
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn airports_resolve_in_priority_order_and_disabled_packs_are_skipped() {
        let root = temp_root();
        write(
            &root.join("Custom Scenery/scenery_packs.ini"),
            "I\n1000 Version\nSCENERY\n\nSCENERY_PACK_DISABLED Custom Scenery/Off/\nSCENERY_PACK Custom Scenery/Mine/\n",
        );
        write(
            &root.join("Custom Scenery/Mine/Earth nav data/apt.dat"),
            &apt("KAAA", "Custom Field"),
        );
        write(
            &root.join("Custom Scenery/Off/Earth nav data/apt.dat"),
            &apt("KAAA", "Disabled"),
        );
        write(
            &root.join("Global Scenery/Global Airports/Earth nav data/apt.dat"),
            "I\n1200 test\n\n1 0 0 0 KAAA Global Field\n1 0 0 0 KBBB Only Global\n99\n",
        );
        let install = Install::open(&root).unwrap();
        let sources = install.apt_dat_sources().unwrap();
        assert_eq!(sources.len(), 2);
        let (a, from) = install.find_airport("KAAA").unwrap().unwrap();
        assert_eq!(a.name, "Custom Field");
        assert!(from.starts_with(root.join("Custom Scenery/Mine")));
        let (b, from) = install.find_airport("KBBB").unwrap().unwrap();
        assert_eq!(b.name, "Only Global");
        assert!(from.starts_with(root.join("Global Scenery")));
        assert!(install.find_airport("KZZZ").unwrap().is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finds_aircraft_in_nested_folders_and_accepts_the_flat_layout() {
        let root = temp_root();
        write(
            &root.join("Aircraft/Laminar Research/Cessna 172 SP/Cessna_172SP.acf"),
            "x",
        );
        write(&root.join("Aircraft/Extra/Plane/other.ACF"), "x");
        write(&root.join("Resources/ignored.acf"), "x");
        let found = Install::open(&root).unwrap().find_acf().unwrap();
        assert_eq!(found.len(), 2);
        fs::remove_dir_all(&root).unwrap();

        let flat = temp_root();
        write(&flat.join("Cessna 172 SP/Cessna_172SP.acf"), "x");
        write(&flat.join("Earth nav data/apt.dat"), &apt("KFLT", "Flat"));
        let install = Install::open(&flat).unwrap();
        assert_eq!(install.find_acf().unwrap().len(), 1);
        assert_eq!(
            install.find_airport("KFLT").unwrap().unwrap().0.name,
            "Flat"
        );
        fs::remove_dir_all(flat).unwrap();
    }

    #[test]
    fn airfoil_candidates_prefer_the_aircraft_folder() {
        let install = Install {
            root: PathBuf::from("root"),
        };
        let c = install.airfoil_candidates(Path::new("root/Aircraft/A/B"), "NACA 2412.afl");
        assert_eq!(
            c[0],
            PathBuf::from("root/Aircraft/A/B/airfoils/NACA 2412.afl")
        );
        assert_eq!(c[1], PathBuf::from("root/Airfoils/NACA 2412.afl"));
    }

    #[test]
    fn scenery_pack_lines_keep_order() {
        let ini = "I\r\n1000 Version\r\nSCENERY\r\n\r\nSCENERY_PACK Custom Scenery/B/\r\nSCENERY_PACK_DISABLED Custom Scenery/X/\r\nSCENERY_PACK Custom Scenery/A/\r\n";
        assert_eq!(
            parse_scenery_packs(ini),
            ["Custom Scenery/B/", "Custom Scenery/A/"]
        );
    }

    #[test]
    fn missing_root_is_an_error() {
        assert!(Install::open(Path::new("/definitely/not/here")).is_err());
    }
}
