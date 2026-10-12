use openxplane::{flight::foil_weights, wing_aero::span_weights};
#[test]
fn span_weights_match_original_code_including_name_comparisons() {
    let mut count = 0;
    let mut corrected = 0;
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
        let t = f32::from_bits(bits[0]);
        let r = std::array::from_fn(|i| f32::from_bits(bits[i + 1]));
        let weights = span_weights(t, r, bits[5] != 0, bits[6] != 0);
        assert_eq!(
            weights.map(f32::to_bits),
            bits[7..],
            "vector line {}",
            line_index + 1
        );
        let names = [
            if bits[5] != 0 { "middle" } else { "root" },
            "middle",
            if bits[6] != 0 { "middle" } else { "tip" },
        ];
        assert_eq!(foil_weights(t, r, names).map(f32::to_bits), bits[7..]);
        if (1.0 - weights[0] - weights[2]).to_bits() != weights[1].to_bits() {
            corrected += 1;
        }
        count += 1;
    }
    assert_eq!(count, 2440);
    assert!(
        corrected > 0,
        "fixture must expose float32 remainder regression"
    );
}
