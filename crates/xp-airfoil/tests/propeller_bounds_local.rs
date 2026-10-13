use xp_airfoil::aircraft_bounds::{PropellerBoundary, propeller_bounds_points};
#[test]
#[ignore = "requires local original-code corpus; set OPENXPLANE_PROPELLER_BOUNDS_VECTORS"]
fn original_propeller_point_pass_matches_exactly() {
    let path = std::env::var("OPENXPLANE_PROPELLER_BOUNDS_VECTORS")
        .expect("set OPENXPLANE_PROPELLER_BOUNDS_VECTORS");
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains(
        "reference_sha256 2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"
    ));
    let mut cases = 0;
    let mut coordinates = 0;
    for line in text.lines().filter(|l| l.starts_with("P ")) {
        let (input, expected) = line.split_once(" -> ").unwrap();
        let f: Vec<_> = input.split_whitespace().collect();
        let count: usize = f[1].parse().unwrap();
        let w: Vec<u32> = f[2..]
            .iter()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(w.len(), count * 17);
        let parts: Vec<_> = w
            .as_chunks::<17>()
            .0
            .iter()
            .map(|s| PropellerBoundary {
                kind: s[0] as i32,
                elements: s[1] as usize,
                radii: std::array::from_fn(|i| f32::from_bits(s[2 + i])),
                center: std::array::from_fn(|i| f32::from_bits(s[14 + i])),
            })
            .collect();
        let points = propeller_bounds_points(&parts).unwrap();
        let out: Vec<_> = expected.split_whitespace().collect();
        let n: usize = out[0].parse().unwrap();
        assert_eq!(points.len(), n);
        assert_eq!(out.len(), n * 3 + 1);
        for (i, v) in points.iter().flatten().enumerate() {
            assert_eq!(
                v.to_bits(),
                u32::from_str_radix(out[i + 1], 16).unwrap(),
                "case {cases} coordinate {i}"
            );
            coordinates += 1;
        }
        cases += 1;
    }
    assert_eq!(cases, 400);
    eprintln!("{cases} cases; {coordinates} exact coordinate words");
}
#[test]
fn query_ten_is_not_radius_maximum_and_kind_nine_skips() {
    let mut radii = [100.0f32; 12];
    radii[1] = 2.;
    radii[2] = 1.;
    let parts = [
        PropellerBoundary {
            kind: 0,
            elements: 2,
            radii,
            center: [0.; 3],
        },
        PropellerBoundary {
            kind: 9,
            elements: 0,
            radii: [f32::NAN; 12],
            center: [f32::NAN; 3],
        },
    ];
    let points = propeller_bounds_points(&parts).unwrap();
    assert_eq!(
        points,
        vec![
            [-1., -1., -1.],
            [-1., -1., 1.],
            [-1., 1., -1.],
            [-1., 1., 1.],
            [1., -1., -1.],
            [1., -1., 1.],
            [1., 1., -1.],
            [1., 1., 1.]
        ]
    );
}
