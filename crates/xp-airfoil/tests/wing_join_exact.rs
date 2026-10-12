use xp_airfoil::wing_dimensions::{CandidateEnd, WingJoinReference, can_join_wing};
use xp_airfoil::wing_element::Boundary;

#[test]
fn endpoint_join_predicates_match_original() {
    let mut count = 0;
    for (line_index, line) in include_str!("data/wing_join.txt").lines().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let t: Vec<_> = line.split_whitespace().collect();
        assert_eq!(t.len(), 16);
        assert_eq!(t[0], "J");
        let f = |i| f32::from_bits(u32::from_str_radix(t[i], 16).unwrap());
        let x = [f(1), f(4)];
        let y = [f(2), f(5)];
        let z = [f(3), f(6)];
        let candidate = Boundary {
            x: &x,
            y: &y,
            z: &z,
            chord: &[0.0, 0.0],
        };
        let reference = WingJoinReference {
            point: [f(8), f(9), f(10)],
            chord: f(11),
            dihedral_deg: f(12),
            late_pass: t[13] == "1",
        };
        for (end, want) in [(CandidateEnd::Root, t[14]), (CandidateEnd::Tip, t[15])] {
            assert_eq!(
                can_join_wing(&candidate, f(7), &reference, end).unwrap(),
                want == "1",
                "original join line {}",
                line_index + 1
            );
        }
        count += 1;
    }
    assert_eq!(count, 2936);
}
