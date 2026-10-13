use xp_acf::{
    Aircraft,
    object::{Schema, load_object},
};
#[test]
#[ignore = "requires local original-code corpus; set OPENXPLANE_BODY_GRID_UNITS_VECTORS"]
fn full_original_body_grid_conversion_matches_exactly() {
    let path = std::env::var("OPENXPLANE_BODY_GRID_UNITS_VECTORS")
        .expect("set OPENXPLANE_BODY_GRID_UNITS_VECTORS");
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains(
        "reference_sha256 2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"
    ));
    let schema = Schema::load();
    let mut cases = 0;
    for line in text.lines().filter(|l| l.starts_with("U ")) {
        let (input, output) = line.split_once(" -> ").unwrap();
        let words: Vec<u32> = input
            .split_whitespace()
            .skip(1)
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        let want: Vec<u32> = output
            .split_whitespace()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(words.len(), 1080);
        assert_eq!(want.len(), 1080);
        let body = cases % 39;
        let mut acf = String::from("I\n1200 Version\nACF\nPROPERTIES_BEGIN\n");
        for (i, word) in words.iter().enumerate() {
            acf.push_str(&format!(
                "P _body/{body}/_geo_xyz/{},{},{} {}\n",
                i / 54,
                i % 54 / 3,
                i % 3,
                f32::from_bits(*word)
            ));
        }
        acf.push_str("PROPERTIES_END\n");
        let object = load_object(&Aircraft::parse(&acf).unwrap(), &schema);
        for (i, expected) in want.into_iter().enumerate() {
            assert_eq!(
                object.word("R6040", body as u32 * 0x34c8 + 0x65c + i as u32 * 4),
                expected,
                "case {cases} coordinate {i}"
            );
        }
        cases += 1;
    }
    assert_eq!(cases, 64);
    eprintln!("{cases} cases, {} exact coordinate words", cases * 1080);
}
