use xp_airfoil::buffet::{NoiseTable, TABLE_LEN};
use xp_airfoil::wing_element::{
    DeltaInputs, DeltaTrig, delta_coefficients, delta_coefficients_with_trig,
};

#[test]
fn delta_targets_match_original_with_recorded_crt_results() {
    let noise = NoiseTable::new(
        (0..TABLE_LEN)
            .map(|i| {
                let hash = (i as u32).wrapping_mul(1664525).wrapping_add(1013904223);
                (hash >> 8) as f32 / 16777216.0
            })
            .collect(),
    )
    .unwrap();
    let mut count = 0;
    let mut platform_exact = 0;
    let mut max_platform_error = 0.0_f32;
    for (line_index, line) in include_str!("data/delta_target.txt").lines().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        assert_eq!(fields.next(), Some("T"));
        let bits: Vec<u64> = fields
            .map(|v| u64::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(bits.len(), 19);
        let f = |index: usize| f32::from_bits(bits[index] as u32);
        let input = DeltaInputs {
            alpha_deg: f(0),
            sweep_deg: f(1),
            v: f(2),
            lift_scale: f(3),
            thickness: f(4),
            extra: [f(5), f(6), f(7)],
            running_time_seconds: f64::from_bits(bits[8]),
            x: f(9),
            z: f(10),
            span: f(11),
            z_reference: f(12),
        };
        let actual = delta_coefficients_with_trig(
            &input,
            &noise,
            DeltaTrig {
                cos_alpha: f(13),
                sin_abs_alpha: f(14),
            },
        )
        .unwrap();
        assert_eq!(
            actual.map(f32::to_bits),
            bits[15..]
                .iter()
                .map(|v| *v as u32)
                .collect::<Vec<_>>()
                .as_slice(),
            "original target line {}",
            line_index + 1
        );
        let platform = delta_coefficients(&input, &noise).unwrap();
        for index in 0..4 {
            platform_exact += usize::from(platform[index].to_bits() == actual[index].to_bits());
            let error = (platform[index] - actual[index]).abs();
            max_platform_error = max_platform_error.max(error);
            assert!(
                error <= 2e-6,
                "platform trig error {error} at line {} channel {index}",
                line_index + 1
            );
        }
        count += 1;
    }
    assert_eq!(count, 1896);
    eprintln!(
        "platform trig: {platform_exact}/{} exact, max absolute coefficient error {max_platform_error}",
        count * 4
    );
}
