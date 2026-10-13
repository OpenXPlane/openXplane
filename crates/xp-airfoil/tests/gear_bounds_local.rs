use xp_airfoil::aircraft_bounds::{GearBoundary, gear_bounds_points};
#[test]
#[ignore = "requires local original-code corpus; set OPENXPLANE_GEAR_BOUNDS_VECTORS"]
fn original_ten_slot_gear_bounds_match_exactly() {
    let path = std::env::var("OPENXPLANE_GEAR_BOUNDS_VECTORS")
        .expect("set OPENXPLANE_GEAR_BOUNDS_VECTORS");
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains(
        "reference_sha256 2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"
    ));
    let mut cases = 0;
    let mut coordinates = 0;
    for line in text.lines().filter(|l| l.starts_with("G ")) {
        let (input, expected) = line.split_once(" -> ").unwrap();
        let w: Vec<u32> = input
            .split_whitespace()
            .skip(1)
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(w.len(), 60);
        let gears: Vec<_> = w
            .as_chunks::<6>()
            .0
            .iter()
            .map(|s| GearBoundary {
                kind: s[0] as i32,
                leg_length: f32::from_bits(s[1]),
                tire_radius: f32::from_bits(s[2]),
                center: [
                    f32::from_bits(s[3]),
                    f32::from_bits(s[4]),
                    f32::from_bits(s[5]),
                ],
            })
            .collect();
        let points = gear_bounds_points(&gears).unwrap();
        let out: Vec<_> = expected.split_whitespace().collect();
        let count: usize = out[0].parse().unwrap();
        assert_eq!(points.len(), count);
        assert_eq!(out.len(), count * 3 + 1);
        for (i, v) in points.iter().flatten().enumerate() {
            assert_eq!(
                v.to_bits(),
                u32::from_str_radix(out[i + 1], 16).unwrap(),
                "case {cases} word {i}"
            );
            coordinates += 1;
        }
        cases += 1;
    }
    assert_eq!(cases, 500);
    eprintln!("{cases} cases, {coordinates} exact coordinate words");
}
#[test]
fn nonzero_kind_and_signed_extent_are_preserved() {
    let gears = [
        GearBoundary {
            kind: 0,
            leg_length: f32::NAN,
            tire_radius: 0.,
            center: [0.; 3],
        },
        GearBoundary {
            kind: -1,
            leg_length: 2.,
            tire_radius: -3.,
            center: [4., 5., 6.],
        },
    ];
    assert_eq!(
        gear_bounds_points(&gears).unwrap(),
        vec![[5., 6., 7.], [3., 4., 5.]]
    );
}
