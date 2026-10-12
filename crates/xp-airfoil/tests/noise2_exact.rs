use xp_airfoil::buffet::{NoiseTable, TABLE_LEN};

#[test]
fn six_octave_noise2_matches_original_bits() {
    let values = (0..TABLE_LEN)
        .map(|i| {
            let hash = (i as u32).wrapping_mul(1664525).wrapping_add(1013904223);
            (hash >> 8) as f32 / 16777216.0
        })
        .collect();
    let noise = NoiseTable::new(values).unwrap();
    let mut count = 0;
    for (line_index, line) in include_str!("data/noise2.txt").lines().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        assert_eq!(fields.next(), Some("N"));
        let bits: Vec<u32> = fields
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(bits.len(), 4);
        let actual = noise
            .fractal2(
                f32::from_bits(bits[0]),
                f32::from_bits(bits[1]),
                bits[2] as i32,
            )
            .unwrap();
        assert_eq!(
            actual.to_bits(),
            bits[3],
            "original noise2 line {}",
            line_index + 1
        );
        count += 1;
    }
    assert_eq!(count, 1756);
    for coordinate in [f32::NAN, f32::INFINITY, 134217728.0] {
        assert!(noise.fractal2(coordinate, 0.0, 0).is_err());
        assert!(noise.fractal2(0.0, coordinate, 0).is_err());
    }
}
