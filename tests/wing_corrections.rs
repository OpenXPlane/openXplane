use openxplane::wing_aero::{
    flap_slat_factor, ice_coefficients, ice_factors, induced_drag, reynolds,
};
#[test]
fn correction_blocks_match_original_machine_code() {
    let mut counts = [0; 5];
    for (line_index, line) in include_str!("data/wing_corrections.txt")
        .lines()
        .enumerate()
    {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        let kind = fields.next().unwrap();
        let bits: Vec<u32> = fields
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        let v: Vec<f32> = bits.iter().map(|b| f32::from_bits(*b)).collect();
        let (index, outputs, expected) = match kind {
            "R" => {
                assert_eq!(v.len(), 8);
                let r = reynolds(v[0], v[1], v[2], v[3], v[4]).unwrap();
                (
                    0,
                    vec![
                        r.viscosity.to_bits(),
                        r.chord.to_bits(),
                        r.millions.to_bits(),
                    ],
                    bits[5..].to_vec(),
                )
            }
            "S" => {
                assert_eq!(v.len(), 8);
                let r = flap_slat_factor(v[0] != 0.0, v[1] != 0.0, v[2], v[3], v[4], v[5], v[6])
                    .unwrap();
                (1, vec![r.to_bits()], bits[7..].to_vec())
            }
            "I" => {
                assert_eq!(v.len(), 7);
                let (a, b) = induced_drag(v[0], v[1], v[2], v[3], v[4]).unwrap();
                (2, vec![a.to_bits(), b.to_bits()], bits[5..].to_vec())
            }
            "C" => {
                assert_eq!(v.len(), 7);
                let (a, b) = ice_coefficients(v[0], v[1], v[2], v[3], v[4]);
                (3, vec![a.to_bits(), b.to_bits()], bits[5..].to_vec())
            }
            "A" => {
                assert_eq!(v.len(), 4);
                let r = ice_factors(v[0]).unwrap();
                (
                    4,
                    vec![
                        r.alpha_scale.to_bits(),
                        r.lift_multiplier.to_bits(),
                        r.drag_multiplier.to_bits(),
                    ],
                    bits[1..].to_vec(),
                )
            }
            _ => panic!("unknown fixture type"),
        };
        assert_eq!(
            outputs,
            expected,
            "original vector line {} ({kind})",
            line_index + 1
        );
        counts[index] += 1;
    }
    assert_eq!(counts, [1060, 1024, 1024, 1024, 1024]);
}
