use xp_airfoil::wing_dimensions::{
    GroupingWing, WingGroupMember, group_wings, group_wings_with_areas,
};
use xp_airfoil::wing_element::Boundary;

#[test]
fn ordered_group_scan_matches_original_with_outputstream_only_stubs() {
    let mut count = 0;
    let mut duplicate_cases = 0;
    for (line_index, line) in include_str!("data/wing_group.txt").lines().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let mut t = line.split_whitespace().filter(|s| *s != "|");
        assert_eq!(t.next(), Some("G"));
        let root_id: usize = t.next().unwrap().parse().unwrap();
        let n: usize = t.next().unwrap().parse().unwrap();
        let f = |v: &str| f32::from_bits(u32::from_str_radix(v, 16).unwrap());
        let owned: Vec<_> = (0..n)
            .map(|_| {
                let id: usize = t.next().unwrap().parse().unwrap();
                let enabled = t.next().unwrap() == "1";
                let elements: usize = t.next().unwrap().parse().unwrap();
                let fields: [f32; 5] = std::array::from_fn(|_| f(t.next().unwrap()));
                let arrays: [Vec<f32>; 4] =
                    std::array::from_fn(|_| (0..=elements).map(|_| f(t.next().unwrap())).collect());
                let areas: Vec<f32> = (0..elements).map(|_| f(t.next().unwrap())).collect();
                (id, enabled, fields, arrays, areas)
            })
            .collect();
        let wings: Vec<_> = owned
            .iter()
            .map(|(id, en, fields, a, _)| GroupingWing {
                id: *id,
                enabled: *en,
                root_chord: fields[2],
                tip_chord: fields[3],
                dihedral_deg: fields[4],
                member: WingGroupMember {
                    span: fields[0],
                    sweep_deg: fields[1],
                    boundary: Boundary {
                        chord: &a[0],
                        x: &a[1],
                        y: &a[2],
                        z: &a[3],
                    },
                },
            })
            .collect();
        let areas: Vec<_> = owned.iter().map(|(_, _, _, _, a)| a.clone()).collect();
        let nmembers: usize = t.next().unwrap().parse().unwrap();
        let want_ids: Vec<usize> = (0..nmembers)
            .map(|_| t.next().unwrap().parse().unwrap())
            .collect();
        let want: [f32; 11] = std::array::from_fn(|_| f(t.next().unwrap()));
        assert!(t.next().is_none());
        let actual = group_wings_with_areas(&wings, root_id, &areas).unwrap();
        assert_eq!(
            actual.members,
            want_ids,
            "membership original line {}",
            line_index + 1
        );
        let have: Vec<u32> = actual
            .endpoints
            .iter()
            .flatten()
            .copied()
            .chain([actual.area])
            .chain(actual.reference_chords)
            .chain(actual.reference_dihedrals)
            .map(f32::to_bits)
            .collect();
        assert_eq!(
            have,
            want.map(f32::to_bits),
            "group state original line {}",
            line_index + 1
        );
        let platform = group_wings(&wings, root_id).unwrap();
        assert_eq!(platform.members, want_ids);
        assert!((platform.area - actual.area).abs() <= actual.area.abs().max(1.0) * 2e-6);
        if actual
            .members
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len()
            < actual.members.len()
        {
            duplicate_cases += 1;
        }
        count += 1;
    }
    assert_eq!(count, 125);
    assert!(
        duplicate_cases > 0,
        "same candidate must exercise both joins in one scan iteration"
    );
}
