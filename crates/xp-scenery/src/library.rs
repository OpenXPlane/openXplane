//! Library export precedence and geographic region selection from the published
//! specification. Season/dataref evaluation remains incomplete.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Index {
    entries: BTreeMap<String, Vec<PathBuf>>,
    backups: BTreeMap<String, Vec<PathBuf>>,
    blocked: BTreeSet<String>,
    regions: BTreeMap<String, Region>,
}

#[derive(Default)]
struct Region {
    rects: Vec<[i32; 4]>,
    bitmap: Option<image::GrayImage>,
    unsupported_condition: bool,
}

impl Region {
    fn contains(&self, lat: f64, lon: f64) -> bool {
        if !lat.is_finite() || !lon.is_finite() || self.unsupported_condition {
            return false;
        }
        let (x, y) = (lon.floor() as i32, lat.floor() as i32);
        if let Some(bitmap) = &self.bitmap {
            if bitmap.width() != 360
                || bitmap.height() != 180
                || !(-180..180).contains(&x)
                || !(-90..90).contains(&y)
            {
                return false;
            }
            // Image rows start at the north; the specification's origin is SW.
            return bitmap.get_pixel((x + 180) as u32, (89 - y) as u32)[0] != 0;
        }
        self.rects
            .iter()
            .any(|r| x >= r[0] && y >= r[1] && x <= r[2] && y <= r[3])
    }
}

impl Index {
    /// Add packages in descending priority. The caller owns package ordering.
    /// Keep all eligible variants; choosing a random one is a separate policy.
    pub fn add_pack(&mut self, pack: &Path, text: &str) {
        self.add_pack_at(pack, text, None);
    }

    pub fn add_pack_at(&mut self, pack: &Path, text: &str, location: Option<[f64; 2]>) {
        self.add_pack_for(pack, text, location, None);
    }

    /// Season tokens follow library.txt: spr, sum, fal, win. Without a season,
    /// seasonal commands are excluded instead of merging incompatible artwork.
    pub fn add_pack_for(
        &mut self,
        pack: &Path,
        text: &str,
        location: Option<[f64; 2]>,
        season: Option<&str>,
    ) {
        let mut replace = BTreeSet::new();
        let mut exclude = BTreeSet::new();
        let mut defining: Option<String> = None;
        let mut active: Option<String> = None;
        for line in text.lines() {
            let mut fields = line.split('#').next().unwrap_or("").split_whitespace();
            let Some(command) = fields.next() else {
                continue;
            };
            match command {
                "REGION_DEFINE" => {
                    defining = fields.next().map(String::from);
                    if let Some(name) = &defining {
                        self.regions.insert(name.clone(), Region::default());
                    }
                    continue;
                }
                "REGION_RECT" | "REGION_ALL" | "REGION_BITMAP" | "REGION_DREF" => {
                    if let Some(region) = defining.as_ref().and_then(|n| self.regions.get_mut(n)) {
                        match command {
                            "REGION_ALL" => region.rects.push([-180, -90, 179, 89]),
                            "REGION_RECT" => {
                                let r =
                                    fields.map(str::parse::<i32>).collect::<Result<Vec<_>, _>>();
                                if let Ok(r) = r
                                    && let Ok(r) = <[i32; 4]>::try_from(r)
                                {
                                    region.rects.push(r);
                                }
                            }
                            "REGION_BITMAP" => {
                                region.rects.clear();
                                region.bitmap =
                                    image::open(pack.join(fields.collect::<Vec<_>>().join(" ")))
                                        .ok()
                                        .map(|im| im.to_luma8());
                                if region.bitmap.is_none() {
                                    region.unsupported_condition = true;
                                }
                            }
                            _ => region.unsupported_condition = true,
                        }
                    }
                    continue;
                }
                "REGION" => {
                    active = fields.next().map(String::from);
                    continue;
                }
                _ => {}
            }
            if let (Some(name), Some([lat, lon])) = (&active, location)
                && !self.regions.get(name).is_some_and(|r| r.contains(lat, lon))
            {
                continue;
            }
            let command = if let Some(base) = command.strip_suffix("_SEASON") {
                if !["EXPORT", "EXPORT_RATIO", "EXPORT_EXTEND", "EXPORT_EXCLUDE"].contains(&base) {
                    continue;
                }
                let Some(tokens) = fields.next() else {
                    continue;
                };
                if !season.is_some_and(|s| tokens.split(',').any(|t| t == s)) {
                    continue;
                }
                base
            } else {
                command
            };
            if ![
                "EXPORT",
                "EXPORT_RATIO",
                "EXPORT_EXTEND",
                "EXPORT_EXCLUDE",
                "EXPORT_BACKUP",
            ]
            .contains(&command)
            {
                continue;
            }
            if command == "EXPORT_RATIO" {
                // Ratios are retained only as eligibility here, not sampled.
                let Some(ratio) = fields.next().and_then(|s| s.parse::<f32>().ok()) else {
                    continue;
                };
                if !ratio.is_finite() || ratio <= 0.0 {
                    continue;
                }
            }
            let Some(name) = fields.next() else { continue };
            let physical = fields.collect::<Vec<_>>().join(" ").replace('\\', "/");
            // The original library contract disallows references outside a pack.
            if physical.is_empty()
                || physical.starts_with('/')
                || physical.contains(':')
                || physical.split('/').any(|p| p == "..")
            {
                continue;
            }
            let target = pack.join(physical);
            if !target.is_file() {
                continue;
            }
            if command == "EXPORT_BACKUP" {
                self.backups.entry(name.into()).or_default().push(target);
                continue;
            }
            if self.blocked.contains(name) || exclude.contains(name) {
                continue;
            }
            self.entries.entry(name.into()).or_default().push(target);
            match command {
                "EXPORT" | "EXPORT_RATIO" => {
                    replace.insert(name.into());
                }
                "EXPORT_EXCLUDE" => {
                    exclude.insert(name.into());
                }
                _ => {}
            }
        }
        // Ordinary EXPORT replaces lower packs, but not sibling definitions.
        self.blocked.extend(replace);
        self.blocked.extend(exclude);
    }

    pub fn candidates(&self, name: &str) -> &[PathBuf] {
        self.entries
            .get(name)
            .or_else(|| self.backups.get(name))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Deterministic preview selection, not original random/weighted selection.
    pub fn first_choices(&self) -> BTreeMap<String, PathBuf> {
        self.entries
            .keys()
            .chain(self.backups.keys())
            .filter_map(|name| {
                self.candidates(name)
                    .first()
                    .map(|p| (name.clone(), p.clone()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn geographic_conditions_choose_override_without_blocking_fallback() {
        let root = std::env::temp_dir().join(format!("openxplane-regions-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for name in ["local", "world"] {
            std::fs::write(root.join(name), "fixture").unwrap();
        }
        let text = "REGION_DEFINE area\nREGION_RECT -123 47 -122 48\nREGION_DEFINE all\nREGION_ALL\nREGION area\nEXPORT_EXCLUDE lib/test local\nREGION all\nEXPORT lib/test world\n";
        for (location, expected) in [
            ([48.999, -121.001], "local"),
            ([49.0, -122.0], "world"),
            ([-1.0, -1.0], "world"),
        ] {
            let mut i = Index::default();
            i.add_pack_at(&root, text, Some(location));
            assert_eq!(i.candidates("lib/test"), &[root.join(expected)]);
        }
        let mut bitmap = image::GrayImage::new(360, 180);
        bitmap.put_pixel(57, 42, image::Luma([255])); // DSF +47-123
        bitmap.save(root.join("region.png")).unwrap();
        let text = "REGION_DEFINE bitmap\nREGION_RECT -180 -90 179 89\nREGION_BITMAP region.png\nREGION bitmap\nEXPORT lib/test local\n";
        let mut i = Index::default();
        i.add_pack_at(&root, text, Some([47.5, -122.5]));
        assert_eq!(i.candidates("lib/test"), &[root.join("local")]);
        let mut i = Index::default();
        i.add_pack_at(&root, text, Some([46.5, -122.5]));
        assert!(i.candidates("lib/test").is_empty());
        let mut i = Index::default();
        i.add_pack_at(&root, "REGION_DEFINE all\nREGION_ALL\nREGION_DREF sim/test > 0\nREGION all\nEXPORT lib/test local", Some([47.5, -122.5]));
        assert!(i.candidates("lib/test").is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn replacement_extension_exclusion_and_backup_across_packs() {
        let root = std::env::temp_dir().join(format!("openxplane-library-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for name in ["a", "b", "c", "d"] {
            std::fs::write(root.join(name), "fixture").unwrap();
        }
        let mut i = Index::default();
        i.add_pack(&root, "EXPORT_BACKUP lib/backup a\nEXPORT_EXTEND lib/merge a\nEXPORT lib/replace a\nEXPORT lib/replace b\nEXPORT_EXCLUDE lib/exclude a\nEXPORT lib/exclude b\nEXPORT_BACKUP lib/only a\n");
        i.add_pack(&root, "EXPORT lib/backup b\nEXPORT lib/merge b\nEXPORT_EXTEND lib/merge c\nEXPORT lib/replace c\nEXPORT lib/exclude c\nEXPORT_BACKUP lib/only b\n");
        i.add_pack(&root, "EXPORT lib/merge d\n");
        let paths = |names: &[&str]| names.iter().map(|n| root.join(n)).collect::<Vec<_>>();
        assert_eq!(i.candidates("lib/backup"), paths(&["b"]));
        assert_eq!(i.candidates("lib/merge"), paths(&["a", "b", "c"]));
        assert_eq!(i.candidates("lib/replace"), paths(&["a", "b"]));
        assert_eq!(i.candidates("lib/exclude"), paths(&["a"]));
        assert_eq!(i.first_choices()["lib/only"], root.join("a"));
        i.add_pack(
            &root,
            "EXPORT lib/missing absent\nEXPORT_RATIO 0 lib/zero a\nEXPORT lib/outside ../a\n",
        );
        assert!(i.candidates("lib/missing").is_empty());
        assert!(i.candidates("lib/zero").is_empty());
        assert!(i.candidates("lib/outside").is_empty());
        i.add_pack(
            &root,
            "EXPORT_RATIO 2.5 lib/ratio a\nEXPORT lib/ratio b # sibling variant\n",
        );
        assert_eq!(i.candidates("lib/ratio"), paths(&["a", "b"]));
        let seasonal = "EXPORT_SEASON spr,sum lib/season a\nEXPORT_EXCLUDE_SEASON fal,win lib/season b\nEXPORT_RATIO_SEASON win 2 lib/ratio-season c\nEXPORT_EXTEND_SEASON sum lib/extend a\n";
        let mut summer = Index::default();
        summer.add_pack_for(&root, seasonal, None, Some("sum"));
        summer.add_pack(&root, "EXPORT lib/extend d\n");
        assert_eq!(summer.candidates("lib/season"), paths(&["a"]));
        assert_eq!(summer.candidates("lib/extend"), paths(&["a", "d"]));
        let mut winter = Index::default();
        winter.add_pack_for(&root, seasonal, None, Some("win"));
        assert_eq!(winter.candidates("lib/season"), paths(&["b"]));
        assert_eq!(winter.candidates("lib/ratio-season"), paths(&["c"]));
        let mut unknown = Index::default();
        unknown.add_pack(&root, seasonal);
        assert!(unknown.candidates("lib/season").is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
