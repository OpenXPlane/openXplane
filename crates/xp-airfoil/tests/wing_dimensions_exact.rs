use xp_airfoil::wing_dimensions::{WingGroupMember, wing_dimensions, wing_dimensions_with_trig};
use xp_airfoil::wing_element::Boundary;

#[test]
fn wing_group_dimensions_match_original_with_recorded_cosines() {
    let mut count = 0;
    let mut platform_exact = 0;
    let mut platform_worst = 0.0_f32;
    let parse = |value: &str| f32::from_bits(u32::from_str_radix(value, 16).unwrap());
    for (line_index, line) in include_str!("data/wing_dimensions.txt").lines().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut t = line.split_whitespace().filter(|v| *v != "|");
        assert_eq!(t.next(), Some("W"));
        let n: usize = t.next().unwrap().parse().unwrap();
        let endpoints = std::array::from_fn(|_| std::array::from_fn(|_| parse(t.next().unwrap())));
        let group_area = parse(t.next().unwrap());
        let owned: Vec<_> = (0..n)
            .map(|_| {
                let elements: usize = t.next().unwrap().parse().unwrap();
                let span = parse(t.next().unwrap());
                let sweep = parse(t.next().unwrap());
                let arrays: [Vec<f32>; 4] = std::array::from_fn(|_| {
                    (0..=elements).map(|_| parse(t.next().unwrap())).collect()
                });
                (span, sweep, arrays)
            })
            .collect();
        let members: Vec<_> = owned
            .iter()
            .map(|(span, sweep, a)| WingGroupMember {
                span: *span,
                sweep_deg: *sweep,
                boundary: Boundary {
                    chord: &a[0],
                    x: &a[1],
                    y: &a[2],
                    z: &a[3],
                },
            })
            .collect();
        let want: [f32; 7] = std::array::from_fn(|_| parse(t.next().unwrap()));
        let cosines: Vec<f32> = (0..n).map(|_| parse(t.next().unwrap())).collect();
        let projection_cosine = parse(t.next().unwrap());
        assert!(t.next().is_none());
        let words = |result: xp_airfoil::wing_dimensions::WingDimensions| {
            [
                result.aspect_parameter,
                result.shape_parameter,
                result.area_sum,
                result.weighted_radius_sum,
                result.mean_radius,
                result.centroid_fraction,
                result.length,
            ]
        };
        let actual = words(
            wing_dimensions_with_trig(&members, endpoints, group_area, &cosines, projection_cosine)
                .unwrap(),
        );
        assert_eq!(
            actual.map(f32::to_bits),
            want.map(f32::to_bits),
            "original wing dimensions line {}",
            line_index + 1
        );
        let platform = words(wing_dimensions(&members, endpoints, group_area).unwrap());
        for i in 0..7 {
            platform_exact += usize::from(platform[i].to_bits() == want[i].to_bits());
            let error = (platform[i] - want[i]).abs() / want[i].abs().max(1.0);
            platform_worst = platform_worst.max(error);
            assert!(
                error <= 2e-6,
                "platform error {error} at line {} channel {i}",
                line_index + 1
            );
        }
        count += 1;
    }
    assert_eq!(count, 635);
    eprintln!(
        "platform cosine path: {platform_exact}/{} exact words, worst scaled absolute error {platform_worst}",
        count * 7
    );
}
