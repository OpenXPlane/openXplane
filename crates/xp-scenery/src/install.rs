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
    /// Preview library: export replacement/extension/exclusion and backup fallback.
    /// Geographic predicates are applied when supplied; season/dataref and random
    /// selection remain incomplete.
    pub fn preview_library(&self) -> Result<std::collections::BTreeMap<String, PathBuf>, String> {
        self.preview_library_at(None)
    }

    /// Geographic DSF lookup; None retains a location-free catalog preview.
    pub fn preview_library_at(
        &self,
        location: Option<[f64; 2]>,
    ) -> Result<std::collections::BTreeMap<String, PathBuf>, String> {
        self.preview_library_for(location, None)
    }

    pub fn preview_library_for(
        &self,
        location: Option<[f64; 2]>,
        season: Option<&str>,
    ) -> Result<std::collections::BTreeMap<String, PathBuf>, String> {
        if season.is_some_and(|s| !["spr", "sum", "fal", "win"].contains(&s)) {
            return Err("invalid library season token".into());
        }
        let mut packs = self.scenery_packs()?;
        let defaults = self.root.join("Resources/default scenery");
        if let Ok(entries) = fs::read_dir(defaults) {
            let mut paths = entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect::<Vec<_>>();
            paths.sort();
            packs.extend(paths);
        }
        let mut index = crate::library::Index::default();
        for pack in packs {
            if let Ok(text) = fs::read_to_string(pack.join("library.txt")) {
                index.add_pack_for(&pack, &text, location, season);
            }
        }
        Ok(index.first_choices())
    }

    /// First usable base mesh in enabled custom-pack order, then global scenery.
    pub fn terrain_tile(&self, lat: f64, lon: f64) -> Option<PathBuf> {
        let (group, tile) = tile_names(lat, lon)?;
        for pack in self.scenery_packs().ok()? {
            let path = pack.join("Earth nav data").join(&group).join(&tile);
            if !path.is_file() {
                continue;
            }
            match crate::dsf::read_bytes(&path).and_then(|bytes| crate::dsf::is_overlay(&bytes)) {
                Ok(false) => return Some(path),
                Ok(true) => continue,
                Err(error) => eprintln!(
                    "Skipping unreadable scenery tile {}: {error}",
                    path.display()
                ),
            }
        }
        self.global_terrain_tile(lat, lon)
    }

    /// Global fallback, also used for elevation when a custom mesh has no raster.
    pub fn global_terrain_tile(&self, lat: f64, lon: f64) -> Option<PathBuf> {
        let (group, tile) = tile_names(lat, lon)?;
        let root = self.root.join("Global Scenery");
        ["X-Plane 12 Global Scenery", "X-Plane 12 Demo Areas"]
            .into_iter()
            .map(|name| {
                root.join(name)
                    .join("Earth nav data")
                    .join(&group)
                    .join(&tile)
            })
            .find(|p| p.is_file())
    }
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

fn tile_names(lat: f64, lon: f64) -> Option<(String, String)> {
    if !(-90.0..90.0).contains(&lat) || !(-180.0..180.0).contains(&lon) {
        return None;
    }
    let lat = lat.floor() as i32;
    let lon = lon.floor() as i32;
    Some((
        format!(
            "{:+03}{:+04}",
            lat.div_euclid(10) * 10,
            lon.div_euclid(10) * 10
        ),
        format!("{lat:+03}{lon:+04}.dsf"),
    ))
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
    fn preview_library_resolves_spaces_and_pack_priority() {
        let root = temp_root();
        write(
            &root.join("Custom Scenery/scenery_packs.ini"),
            "SCENERY_PACK Custom Scenery/Test/\n",
        );
        write(
            &root.join("Custom Scenery/Test/library.txt"),
            "A\n800\nLIBRARY\nEXPORT lib/test.obj objects/model with spaces.obj\n",
        );
        write(
            &root.join("Custom Scenery/Test/objects/model with spaces.obj"),
            "test",
        );
        write(
            &root.join("Resources/default scenery/Test/library.txt"),
            "EXPORT lib/test.obj fallback.obj\n",
        );
        write(
            &root.join("Resources/default scenery/Test/fallback.obj"),
            "test",
        );
        let index = Install::open(&root).unwrap().preview_library().unwrap();
        assert_eq!(
            index["lib/test.obj"],
            root.join("Custom Scenery/Test/objects/model with spaces.obj")
        );
        fs::remove_dir_all(root).unwrap();
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
            &apt("KAAA", "Custom Field").replacen("1200", "1130", 1),
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
    fn custom_base_mesh_precedes_global_but_overlays_and_disabled_packs_do_not() {
        fn dsf(overlay: bool) -> Vec<u8> {
            fn atom(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
                let mut result = tag.to_vec();
                result.extend(((body.len() + 8) as u32).to_le_bytes());
                result.extend(body);
                result
            }
            let mut prop = b"sim/overlay\0".to_vec();
            prop.extend(if overlay { &b"1\0"[..] } else { &b"0\0"[..] });
            let mut result = b"XPLNEDSF\x01\0\0\0".to_vec();
            result.extend(atom(b"DAEH", &atom(b"PORP", &prop)));
            result.extend([0; 16]);
            result
        }
        let root = temp_root();
        write(
            &root.join("Custom Scenery/scenery_packs.ini"),
            "SCENERY_PACK_DISABLED Custom Scenery/Off/\nSCENERY_PACK Custom Scenery/Overlay/\nSCENERY_PACK Custom Scenery/Mesh/\n",
        );
        let relative = "Earth nav data/+40-130/+47-123.dsf";
        for (name, overlay) in [("Off", false), ("Overlay", true), ("Mesh", false)] {
            let path = root.join("Custom Scenery").join(name).join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, dsf(overlay)).unwrap();
        }
        let global = root
            .join("Global Scenery/X-Plane 12 Global Scenery")
            .join(relative);
        fs::create_dir_all(global.parent().unwrap()).unwrap();
        fs::write(&global, dsf(false)).unwrap();
        let install = Install::open(&root).unwrap();
        assert_eq!(
            install.terrain_tile(47.4, -122.4),
            Some(root.join("Custom Scenery/Mesh").join(relative))
        );
        fs::remove_file(root.join("Custom Scenery/Mesh").join(relative)).unwrap();
        assert_eq!(install.terrain_tile(47.4, -122.4), Some(global));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_root_is_an_error() {
        assert!(Install::open(Path::new("/definitely/not/here")).is_err());
    }
}
