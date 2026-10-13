use xp_airfoil::wing_boundary::{
    Curvature, Transform, Trig, boundary_point, boundary_point_with_trig,
};
#[test]
fn original_loop_and_complete_transform_match_with_recorded_crt() {
    let mut cases = 0;
    let mut points = 0;
    let mut platform_exact = 0;
    let mut worst = 0.0f32;
    for line in include_str!("data/wing_boundary.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        let n: usize = fields[1].parse().unwrap();
        let words: Vec<u32> = fields[3..]
            .iter()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        let floats: Vec<f32> = words.iter().map(|b| f32::from_bits(*b)).collect();
        let t = Transform {
            span: floats[0],
            design_dihedral: floats[1],
            design_sweep: floats[2],
            rotation: floats[3],
            dihedral_delta: floats[4],
            side: floats[5],
            variable_sweep: floats[6] as i32,
            rotation_offset: floats[7],
            retract: floats[8],
            origin: floats[9..12].try_into().unwrap(),
        };
        let c = Curvature {
            angle: floats[12],
            amplitude: floats[13],
            scale: floats[14],
        };
        assert_eq!(words.len(), 15 + 2 * (n + 1) + 17 * (n + 1));
        for rib in 0..=n {
            let at = 15 + 2 * (n + 1) + rib * 17;
            let tr = &floats[at + 3..at + 17];
            let trig = Trig([
                tr[0], tr[1], tr[2], tr[3], tr[7], tr[8], tr[9], tr[10], tr[12], tr[13],
            ]);
            let got = boundary_point_with_trig(
                &t,
                c,
                n,
                rib,
                floats[15 + rib],
                floats[15 + n + 1 + rib],
                trig,
            );
            let platform =
                boundary_point(&t, c, n, rib, floats[15 + rib], floats[15 + n + 1 + rib]);
            for axis in 0..3 {
                assert_eq!(
                    got[axis].to_bits(),
                    words[at + axis],
                    "case {cases}, rib {rib}, axis {axis}"
                );
                if platform[axis].to_bits() == words[at + axis] {
                    platform_exact += 1;
                }
                let error =
                    (platform[axis] - floats[at + axis]).abs() / floats[at + axis].abs().max(1.0);
                worst = worst.max(error);
                assert!(
                    error <= 0.000002,
                    "platform case {cases} rib {rib} axis {axis}: {error}"
                );
            }
            points += 1;
        }
        cases += 1;
    }
    assert_eq!(cases, 240);
    eprintln!(
        "{cases} cases, {points} points, {} exact recorded words; {platform_exact} exact platform words, worst scaled error {worst}",
        points * 3
    );
}
