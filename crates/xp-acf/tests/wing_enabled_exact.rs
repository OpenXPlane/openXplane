use xp_acf::wing::enabled_meters;

#[test]
fn original_enabled_byte_matches_for_metre_and_scaled_feet_inputs() {
    let mut count = 0;
    for (line_index, line) in include_str!("data/wing_enabled.txt").lines().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let t: Vec<_> = line.split_whitespace().collect();
        assert_eq!(t.len(), 5);
        assert_eq!(t[0], "E");
        let f = |i| f32::from_bits(u32::from_str_radix(t[i], 16).unwrap());
        assert_eq!(
            enabled_meters(f(1), f(2), f(3)),
            t[4] == "1",
            "original enabled line {}",
            line_index + 1
        );
        count += 1;
    }
    assert_eq!(count, 2072);
}
