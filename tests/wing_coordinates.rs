use openxplane::wing_aero::span_inputs;
#[test]
fn wing_coordinates_and_fraction_match_original_instructions() {
    let mut count = 0;
    for (line_index, line) in include_str!("data/wing_coordinates.txt")
        .lines()
        .enumerate()
    {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        assert_eq!(fields.next(), Some("P"));
        let elements: usize = fields.next().unwrap().parse().unwrap();
        let index: i32 = fields.next().unwrap().parse().unwrap();
        let bits: Vec<u32> = fields
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(bits.len(), 3 + 3 * (elements + 1) + 4);
        let refs = std::array::from_fn(|i| f32::from_bits(bits[i]));
        let points: Vec<[f32; 3]> = bits[3..3 + 3 * (elements + 1)]
            .chunks_exact(3)
            .map(|p| std::array::from_fn(|i| f32::from_bits(p[i])))
            .collect();
        let r = span_inputs(index, &points, refs).unwrap();
        assert_eq!(
            [
                r.fraction.to_bits(),
                r.normalized_position[0].to_bits(),
                r.normalized_position[1].to_bits(),
                r.normalized_position[2].to_bits()
            ],
            bits[bits.len() - 4..],
            "vector line {}",
            line_index + 1
        );
        count += 1;
    }
    assert_eq!(count, 1105);
}
