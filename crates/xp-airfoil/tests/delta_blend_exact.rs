use xp_airfoil::wing_element::blend_delta_coefficients;

#[test]
fn delta_coefficients_and_stall_match_original_bits() {
    let mut count = 0;
    for (line_index, line) in include_str!("data/delta_blend.txt").lines().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        assert_eq!(fields.next(), Some("B"));
        let bits: Vec<u32> = fields
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(bits.len(), 14);
        let base = std::array::from_fn(|i| f32::from_bits(bits[i]));
        let delta = std::array::from_fn(|i| f32::from_bits(bits[i + 4]));
        let result = blend_delta_coefficients(base, delta, f32::from_bits(bits[8]));
        assert_eq!(
            [
                result.cl.to_bits(),
                result.cd.to_bits(),
                result.cm.to_bits(),
                result.ratio.to_bits(),
                u32::from(result.stalled)
            ],
            bits[9..],
            "original delta blend line {}",
            line_index + 1
        );
        count += 1;
    }
    assert_eq!(count, 2228);
}
