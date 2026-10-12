use xp_airfoil::aircraft_bounds::wing_bounds_points;
use xp_airfoil::wing_element::Boundary;

#[test]
fn emitted_wing_bound_points_match_original_bits() {
    let mut count = 0;
    for (line_index, line) in include_str!("data/wing_bounds_points.txt")
        .lines()
        .enumerate()
    {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut tokens = line.split_whitespace();
        assert_eq!(tokens.next(), Some("P"));
        let n: usize = tokens.next().unwrap().parse().unwrap();
        let bits: Vec<u32> = tokens
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect();
        assert_eq!(bits.len(), 10 * (n + 1));
        let arrays: Vec<Vec<f32>> = bits[..4 * (n + 1)]
            .chunks_exact(n + 1)
            .map(|v| v.iter().map(|b| f32::from_bits(*b)).collect())
            .collect();
        let result = wing_bounds_points(&Boundary {
            chord: &arrays[0],
            x: &arrays[1],
            y: &arrays[2],
            z: &arrays[3],
        })
        .unwrap();
        let actual: Vec<u32> = result.iter().flatten().map(|v| v.to_bits()).collect();
        assert_eq!(
            actual,
            bits[4 * (n + 1)..],
            "original emitted points line {}",
            line_index + 1
        );
        count += 1;
    }
    assert_eq!(count, 512);
}
