//! Unconditional library export precedence from the published library.txt
//! specification. Region/season/dataref evaluation is not implemented here.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Index {
    entries: BTreeMap<String, Vec<PathBuf>>,
    backups: BTreeMap<String, Vec<PathBuf>>,
    blocked: BTreeSet<String>,
}

impl Index {
    /// Add packages in descending priority. The caller owns package ordering.
    /// Keep all eligible variants; choosing a random one is a separate policy.
    pub fn add_pack(&mut self, pack: &Path, text: &str) {
        let mut replace = BTreeSet::new();
        let mut exclude = BTreeSet::new();
        for line in text.lines() {
            let mut fields = line.split('#').next().unwrap_or("").split_whitespace();
            let Some(command) = fields.next() else {
                continue;
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
        std::fs::remove_dir_all(root).unwrap();
    }
}
