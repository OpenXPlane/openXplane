//! Requires an explicitly supplied local original-code corpus; none is distributed.
use xp_airfoil::wing_boundary::curvature_parameters;
#[test]
#[ignore = "requires local corpus generated from the reference EXE; set OPENXPLANE_CURVATURE_VECTORS"]
fn original_curvature_setup() {
    let path =
        std::env::var("OPENXPLANE_CURVATURE_VECTORS").expect("set OPENXPLANE_CURVATURE_VECTORS");
    let text = std::fs::read_to_string(path).unwrap();
    let mut cases = 0;
    let mut exact = 0;
    let mut worst = 0.0f32;
    assert!(text.contains(
        "reference_sha256 2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"
    ));
    for line in text.lines().filter(|s| s.starts_with("C ")) {
        let words: Vec<u32> = line
            .split_whitespace()
            .skip(1)
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(words.len(), 6);
        let f: Vec<f32> = words.iter().map(|w| f32::from_bits(*w)).collect();
        let got = curvature_parameters(f[0], f[1], f[2]);
        for (i, value) in [got.angle, got.amplitude, got.scale]
            .into_iter()
            .enumerate()
        {
            if value.to_bits() == words[3 + i] {
                exact += 1;
            }
            let err = (value - f[3 + i]).abs() / f[3 + i].abs().max(1.0);
            worst = worst.max(err);
            assert!(
                err <= 0.000002,
                "case {cases} field {i}: {value} != {} ({err})",
                f[3 + i]
            );
        }
        cases += 1;
    }
    assert_eq!(cases, 521);
    eprintln!(
        "{cases} cases; {exact}/{} exact words; worst scaled error {worst}",
        cases * 3
    );
}
