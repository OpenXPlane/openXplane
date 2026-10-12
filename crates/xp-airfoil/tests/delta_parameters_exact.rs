use xp_airfoil::wing_element::delta_parameters;

#[test]
fn delta_parameter_curves_match_original_bits() {
    let mut count = 0;
    for (line_index, line) in include_str!("data/delta_parameters.txt")
        .lines()
        .enumerate()
    {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        assert_eq!(fields.next(), Some("P"));
        let bits: Vec<u32> = fields
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(bits.len(), 4);
        let parameters = delta_parameters(f32::from_bits(bits[0]));
        assert_eq!(
            [
                parameters.lift_slope.to_bits(),
                parameters.peak_alpha_deg.to_bits(),
                parameters.peak_lift.to_bits()
            ],
            bits[1..],
            "original delta parameter line {}",
            line_index + 1
        );
        count += 1;
    }
    assert_eq!(count, 2027);
}
