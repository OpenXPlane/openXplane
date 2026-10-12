//! Bit comparisons against original instruction blocks executed by gen_wing_vectors.py.
use openxplane::wing_aero::{fade_to_one, flow_factors};

#[test]
fn wing_factor_blocks_match_original_instructions() {
    let mut counts = [0; 2];
    for (line_index, line) in include_str!("data/wing_factors.txt").lines().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        let kind = fields.next().unwrap();
        let values: Vec<u32> = fields
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        match kind {
            "F" => {
                assert_eq!(values.len(), 5);
                let result =
                    flow_factors(f32::from_bits(values[0]), f32::from_bits(values[1])).unwrap();
                assert_eq!(
                    [
                        result.flow.to_bits(),
                        result.lift_scale.to_bits(),
                        result.alpha_multiplier.to_bits()
                    ],
                    [values[2], values[3], values[4]],
                    "flow vector line {}",
                    line_index + 1
                );
                counts[0] += 1;
            }
            "D" => {
                assert_eq!(values.len(), 4);
                let result =
                    fade_to_one(f32::from_bits(values[0]), f32::from_bits(values[1])).to_bits();
                assert_eq!(
                    result,
                    values[2],
                    "multiplier vector line {}",
                    line_index + 1
                );
                assert_eq!(result, values[3], "divisor vector line {}", line_index + 1);
                counts[1] += 1;
            }
            _ => panic!("unknown vector type"),
        }
    }
    assert_eq!(counts, [1036, 1144]);
}
