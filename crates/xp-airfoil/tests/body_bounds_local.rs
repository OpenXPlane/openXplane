use xp_airfoil::aircraft_bounds::{BodyBoundary, body_bounds_points};
#[test]
#[ignore = "requires original-code local corpus; set OPENXPLANE_BODY_BOUNDS_VECTORS"]
fn original_body_point_list_matches_exactly() {
    let path = std::env::var("OPENXPLANE_BODY_BOUNDS_VECTORS")
        .expect("set OPENXPLANE_BODY_BOUNDS_VECTORS");
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains(
        "reference_sha256 2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"
    ));
    let mut cases = 0;
    let mut coordinates = 0;
    for line in text.lines().filter(|l| l.starts_with("P ")) {
        let (inputs, expected) = line.split_once(" -> ").unwrap();
        let f: Vec<_> = inputs.split_whitespace().collect();
        let enabled = f[1] == "1";
        let ns: usize = f[2].parse().unwrap();
        let rings: usize = f[3].parse().unwrap();
        let values: Vec<f32> = f[5..]
            .iter()
            .map(|s| f32::from_bits(u32::from_str_radix(s, 16).unwrap()))
            .collect();
        assert_eq!(values.len(), 3 + ns * 54);
        let mut grid = vec![[[0.0f32; 3]; 18]; ns];
        for (station, row) in grid.iter_mut().enumerate() {
            for (ring, point) in row.iter_mut().enumerate() {
                *point = values[3 + station * 54 + ring * 3..3 + station * 54 + ring * 3 + 3]
                    .try_into()
                    .unwrap();
            }
        }
        let got = body_bounds_points(&BodyBoundary {
            enabled,
            origin: values[..3].try_into().unwrap(),
            rings,
            stations: &grid,
        })
        .unwrap();
        let out: Vec<_> = expected.split_whitespace().collect();
        let count: usize = out[0].parse().unwrap();
        assert_eq!(got.len(), count);
        assert_eq!(out.len(), 1 + count * 3);
        for (i, v) in got.iter().flatten().enumerate() {
            assert_eq!(
                v.to_bits(),
                u32::from_str_radix(out[1 + i], 16).unwrap(),
                "case {cases} coordinate {i}"
            );
            coordinates += 1;
        }
        cases += 1;
    }
    assert_eq!(cases, 120);
    eprintln!("{cases} cases, {coordinates} exact coordinate words");
}

#[test]
fn quad_order_wrap_and_fixed_eighteen_point_stride() {
    let mut grid = [[[0.0f32; 3]; 18]; 2];
    grid[0][0] = [1., 2., 3.];
    grid[0][1] = [4., 5., 6.];
    grid[1][0] = [7., 8., 9.];
    grid[1][1] = [10., 11., 12.];
    let got = body_bounds_points(&BodyBoundary {
        enabled: true,
        origin: [0., 0., 0.],
        rings: 2,
        stations: &grid,
    })
    .unwrap();
    assert_eq!(
        got,
        vec![
            grid[0][0], grid[1][0], grid[1][1], grid[0][1], grid[0][1], grid[1][1], grid[1][0],
            grid[0][0]
        ]
    );
    assert!(
        body_bounds_points(&BodyBoundary {
            enabled: false,
            origin: [0.; 3],
            rings: 2,
            stations: &grid
        })
        .unwrap()
        .is_empty()
    );
    assert!(
        body_bounds_points(&BodyBoundary {
            enabled: true,
            origin: [0.; 3],
            rings: 19,
            stations: &grid
        })
        .is_err()
    );
}
