use xp_airfoil::wing_element::{Boundary, delta_wing_weight, sweep_degrees};
#[test]
fn geometry_helpers_match_original_bits() {
    let mut count = 0;
    for (line_index, line) in include_str!("data/wing_geometry.txt").lines().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        assert_eq!(fields.next(), Some("G"));
        let bits: Vec<u32> = fields
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(bits.len(), 12);
        let v: Vec<f32> = bits.iter().map(|v| f32::from_bits(*v)).collect();
        let x = [v[0], v[3]];
        let y = [v[1], v[4]];
        let z = [v[2], v[5]];
        let chord = [v[6], v[7]];
        let boundary = Boundary {
            x: &x,
            y: &y,
            z: &z,
            chord: &chord,
        };
        assert_eq!(
            sweep_degrees(&boundary, 0).to_bits(),
            bits[10],
            "sweep line {}",
            line_index + 1
        );
        assert_eq!(
            delta_wing_weight(&boundary, 0, v[8], v[9]).to_bits(),
            bits[11],
            "delta weight line {}",
            line_index + 1
        );
        count += 1;
    }
    assert_eq!(count, 4062);
}
