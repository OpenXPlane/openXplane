use xp_airfoil::wing_element::profile_weights;

#[test]
fn profile_weights_match_original_bits() {
    let mut count = 0;
    for (line_index, line) in include_str!("data/wing_span.txt").lines().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        assert_eq!(fields.next(), Some("W"));
        let bits: Vec<u32> = fields
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(bits.len(), 10);
        let fraction = f32::from_bits(bits[0]);
        let ratios = std::array::from_fn(|i| f32::from_bits(bits[i + 1]));
        let names = [
            if bits[5] != 0 { "middle" } else { "root" },
            "middle",
            if bits[6] != 0 { "middle" } else { "tip" },
        ];
        let weights = profile_weights(fraction, ratios, names);
        assert_eq!(
            weights.map(f32::to_bits),
            bits[7..],
            "original span line {}",
            line_index + 1
        );
        count += 1;
    }
    assert_eq!(count, 2440);
}
