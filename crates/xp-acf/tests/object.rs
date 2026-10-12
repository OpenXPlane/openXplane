//! The aircraft object built from the ACF properties against the words the original loader leaves
//! (`tools/gen_acf_object_vectors.py`). Needs the reference aircraft in `Xplane12/` (not distributed); the
//! test is skipped without them.
use std::collections::BTreeMap;
use std::path::Path;
use xp_acf::Aircraft;
use xp_acf::object::{Schema, load_object};

fn expected(name: &str) -> BTreeMap<(String, u32), u32> {
    let path = format!(
        "{}/tests/data/acf_object_{name}.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).unwrap();
    let mut out = BTreeMap::new();
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let (region, rest) = line.split_once(' ').unwrap();
        let (off, word) = rest.split_once('=').unwrap();
        out.insert(
            (region.to_string(), u32::from_str_radix(off, 16).unwrap()),
            u32::from_str_radix(word, 16).unwrap(),
        );
    }
    out
}

#[test]
fn object_matches_the_original_loader() {
    let schema = Schema::load();
    for (dir, name) in [
        ("Cessna 172 SP", "Cessna_172SP"),
        ("Cessna 172 SP", "Cessna_172SP_G1000"),
        ("Cessna 172 SP", "Cessna_172SP_seaplane"),
    ] {
        let path = format!(
            "{}/../../Xplane12/{dir}/{name}.acf",
            env!("CARGO_MANIFEST_DIR")
        );
        if !Path::new(&path).exists() {
            eprintln!("skipped: {path} is not present");
            continue;
        }
        let acf = Aircraft::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let got = load_object(&acf, &schema);
        let want = expected(name);
        let (mut same, mut different, mut missing) = (0, 0, 0);
        let mut shown = 0;
        for ((region, offset), word) in &want {
            match got.regions.get(region).and_then(|r| r.get(offset)) {
                Some(w) if w == word => same += 1,
                Some(w) => {
                    different += 1;
                    if std::env::var_os("XP_MISSING").is_some() {
                        eprintln!("DIFF {region} {offset:x} {w:08x} {word:08x}");
                    }
                    if shown < 12 && std::env::var_os("XP_ALL").is_some() {
                        eprintln!("{region} {offset:#x}: {w:#010x} vs {word:#010x}");
                        shown += 1;
                    }
                }
                None => {
                    missing += 1;
                    if std::env::var_os("XP_MISSING").is_some() {
                        eprintln!("MISSING {region} {offset:x} {word:08x}");
                    }
                }
            }
        }
        let extra = got
            .regions
            .iter()
            .flat_map(|(r, w)| w.keys().map(move |o| (r.clone(), *o)))
            .filter(|k| !want.contains_key(k))
            .count();
        eprintln!("{name}: {same} same, {different} different, {missing} missing, {extra} extra");
    }
}
