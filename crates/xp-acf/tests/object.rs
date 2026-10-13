//! The aircraft object built from the ACF properties against the words the original loader leaves
//! (`tools/gen_acf_object_vectors.py`). Needs the reference aircraft in `Xplane12/` (not distributed); the
//! run explicitly with OPENXPLANE_REFERENCE_ROOT. It is diagnostic, not a parity assertion.
use std::collections::BTreeMap;
use std::path::PathBuf;
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
#[ignore = "requires local reference aircraft; diagnostic mismatch report, not a parity assertion"]
fn object_loader_diagnostics() {
    let schema = Schema::load();
    let root = std::env::var_os("OPENXPLANE_REFERENCE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Xplane12"));
    for (dir, name) in [
        ("Cessna 172 SP", "Cessna_172SP"),
        ("Cessna 172 SP", "Cessna_172SP_G1000"),
        ("Cessna 172 SP", "Cessna_172SP_seaplane"),
    ] {
        let path = root.join(dir).join(format!("{name}.acf"));
        assert!(
            path.exists(),
            "reference aircraft is required: {}",
            path.display()
        );
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

#[test]
#[ignore = "requires all three local reference aircraft; strict body-grid field comparison"]
fn body_grid_coordinates_match_completed_original_loader() {
    let root = std::env::var_os("OPENXPLANE_REFERENCE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Xplane12"));
    let schema = Schema::load();
    for name in [
        "Cessna_172SP",
        "Cessna_172SP_G1000",
        "Cessna_172SP_seaplane",
    ] {
        let path = root.join("Cessna 172 SP").join(format!("{name}.acf"));
        let acf = Aircraft::parse(
            &std::fs::read_to_string(&path).expect("local reference aircraft required"),
        )
        .unwrap();
        let got = load_object(&acf, &schema);
        let want = expected(name);
        let mut checked = 0;
        for p in &acf.properties {
            let Some(rest) = p.key.strip_prefix("_body/") else {
                continue;
            };
            let Some((body, coordinates)) = rest.split_once("/_geo_xyz/") else {
                continue;
            };
            let Ok(body) = body.parse::<u32>() else {
                continue;
            };
            let Ok(indices) = coordinates
                .split(',')
                .map(str::parse::<u32>)
                .collect::<Result<Vec<_>, _>>()
            else {
                continue;
            };
            if indices.len() != 3 {
                continue;
            }
            assert!(body < 39 && indices[0] < 20 && indices[1] < 18 && indices[2] < 3);
            let offset =
                body * 0x34c8 + 0x65c + indices[0] * 216 + indices[1] * 12 + indices[2] * 4;
            let original = want
                .get(&("R6040".to_owned(), offset))
                .copied()
                .unwrap_or(0);
            assert_eq!(got.word("R6040", offset), original, "{name} {}", p.key);
            checked += 1;
        }
        assert!(checked > 0, "no body coordinates in {name}");
        eprintln!("{name}: {checked} exact body grid words");
    }
}
