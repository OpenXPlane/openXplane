use xp_airfoil::aircraft_bounds::aircraft_bounds;

#[test]
fn bounds_and_reference_sizes_match_original_bits() {
    let mut count = 0;
    for (line_index, line) in include_str!("data/aircraft_bounds.txt").lines().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut tokens = line.split_whitespace();
        assert_eq!(tokens.next(), Some("B"));
        let len: usize = tokens.next().unwrap().parse().unwrap();
        let raw: Vec<u32> = tokens
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(raw.len(), len * 3 + 10);
        let points: Vec<[f32; 3]> = raw[..len * 3]
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| std::array::from_fn(|i| f32::from_bits(p[i])))
            .collect();
        let result = aircraft_bounds(&points).unwrap();
        let have: Vec<u32> = result
            .min
            .into_iter()
            .chain(result.max)
            .chain([result.planar_halfspan])
            .chain(result.size)
            .map(f32::to_bits)
            .collect();
        assert_eq!(
            have,
            raw[len * 3..],
            "original bounds line {}",
            line_index + 1
        );
        count += 1;
    }
    assert_eq!(count, 1013);
    assert!(aircraft_bounds(&[[f32::NAN, 0.0, 0.0]]).is_err());
}
