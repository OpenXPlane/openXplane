//! Exact original loop outputs through the public ACF object loader.
use xp_acf::{
    Aircraft,
    object::{Schema, load_object},
};

#[test]
fn original_chord_conversion_preserves_unused_ribs() {
    let schema = Schema::load();
    let mut cases = 0;
    for line in include_str!("data/wing_chord.txt")
        .lines()
        .filter(|s| !s.starts_with('#') && !s.is_empty())
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 23);
        let n: i32 = fields[0].parse().unwrap();
        let mut text = format!("I\n1200 Version\nACF\nPROPERTIES_BEGIN\nP _wing/0/_els {n}\n");
        for rib in 0..11 {
            let raw = u32::from_str_radix(fields[1 + rib], 16).unwrap();
            text.push_str(&format!(
                "P _wing/0/_chord_for_RIB/{rib} {}\n",
                f32::from_bits(raw)
            ));
        }
        text.push_str("PROPERTIES_END\n");
        let acf = Aircraft::parse(&text).unwrap();
        let object = load_object(&acf, &schema);
        for rib in 0..11 {
            let expected = u32::from_str_radix(fields[12 + rib], 16).unwrap();
            assert_eq!(
                object.word("R6028", 0x70 + rib as u32 * 4),
                expected,
                "case {cases} n={n} rib={rib}"
            );
        }
        cases += 1;
    }
    assert_eq!(cases, 312);
}
