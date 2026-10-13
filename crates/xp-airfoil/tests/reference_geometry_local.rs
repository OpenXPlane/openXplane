use xp_airfoil::{aircraft_bounds::*, wing_element::Boundary};
struct Cursor {
    words: Vec<u32>,
    at: usize,
}
impl Cursor {
    fn word(&mut self) -> u32 {
        let v = self.words[self.at];
        self.at += 1;
        v
    }
    fn count(&mut self) -> usize {
        self.word() as usize
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn xyz(&mut self) -> [f32; 3] {
        [self.float(), self.float(), self.float()]
    }
    fn floats(&mut self, n: usize) -> Vec<f32> {
        (0..n).map(|_| self.float()).collect()
    }
}
#[test]
#[ignore = "requires local original-code corpus; set OPENXPLANE_REFERENCE_GEOMETRY_VECTORS"]
fn combined_original_reference_points_and_bounds_match_exactly() {
    let path = std::env::var("OPENXPLANE_REFERENCE_GEOMETRY_VECTORS")
        .expect("set OPENXPLANE_REFERENCE_GEOMETRY_VECTORS");
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains(
        "reference_sha256 2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"
    ));
    let mut cases = 0;
    let mut coordinates = 0;
    for line in text.lines().filter(|l| l.starts_with("C ")) {
        let sections: Vec<_> = line.split(" -> ").collect();
        assert_eq!(sections.len(), 3);
        let mut c = Cursor {
            words: sections[0]
                .split_whitespace()
                .skip(1)
                .map(|s| u32::from_str_radix(s, 16).unwrap())
                .collect(),
            at: 0,
        };
        let gears: Vec<_> = (0..10)
            .map(|_| GearBoundary {
                kind: c.word() as i32,
                leg_length: c.float(),
                tire_radius: c.float(),
                center: c.xyz(),
            })
            .collect();
        let mut owned_wings = Vec::new();
        let wc = c.count();
        for _ in 0..wc {
            let slot = c.count();
            let n = c.count();
            let arrays: Vec<_> = (0..4).map(|_| c.floats(n + 1)).collect();
            owned_wings.push((slot, arrays));
        }
        let mut wings: Vec<Option<Boundary<'_>>> = (0..48).map(|_| None).collect();
        for (slot, a) in &owned_wings {
            wings[*slot] = Some(Boundary {
                chord: &a[0],
                x: &a[1],
                y: &a[2],
                z: &a[3],
            });
        }
        let mut owned_bodies = Vec::new();
        let bc = c.count();
        for _ in 0..bc {
            let slot = c.count();
            let ns = c.count();
            let rings = c.count();
            let origin = c.xyz();
            let mut grid = vec![[[0.; 3]; 18]; ns];
            for row in &mut grid {
                for point in row {
                    *point = c.xyz();
                }
            }
            owned_bodies.push((slot, rings, origin, grid));
        }
        let bodies: Vec<_> = owned_bodies
            .iter()
            .map(|(_, rings, origin, grid)| BodyBoundary {
                enabled: true,
                origin: *origin,
                rings: *rings,
                stations: grid,
            })
            .collect();
        let pc = c.count();
        let parts: Vec<_> = (0..pc)
            .map(|_| PropellerBoundary {
                kind: c.word() as i32,
                elements: c.count(),
                radii: c.floats(12).try_into().unwrap(),
                center: c.xyz(),
            })
            .collect();
        assert_eq!(c.at, c.words.len());
        let (points, bounds) = reference_geometry_bounds(&gears, &wings, &bodies, &parts).unwrap();
        let output: Vec<_> = sections[1].split_whitespace().collect();
        let count: usize = output[0].parse().unwrap();
        assert_eq!(points.len(), count, "case {cases}");
        assert_eq!(output.len(), count * 3 + 1);
        for (i, v) in points.iter().flatten().enumerate() {
            assert_eq!(
                v.to_bits(),
                u32::from_str_radix(output[1 + i], 16).unwrap(),
                "case {cases} coordinate {i}"
            );
            coordinates += 1;
        }
        let expected: Vec<u32> = sections[2]
            .split_whitespace()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        assert_eq!(expected.len(), 10);
        let got: Vec<_> = bounds
            .min
            .into_iter()
            .chain(bounds.max)
            .chain([bounds.planar_halfspan])
            .chain(bounds.size)
            .map(f32::to_bits)
            .collect();
        assert_eq!(got, expected, "bounds case {cases}");
        cases += 1;
    }
    assert_eq!(cases, 80);
    eprintln!(
        "{cases} combined cases; {coordinates} exact point words and {} exact bounds words",
        cases * 10
    );
}
#[test]
fn empty_reference_pipeline_includes_origin() {
    let (points, bounds) = reference_geometry_bounds(&[], &[], &[], &[]).unwrap();
    assert!(points.is_empty());
    assert_eq!(bounds.min, [0.; 3]);
    assert_eq!(bounds.max, [0.; 3]);
    assert_eq!(bounds.planar_halfspan, 0.);
    assert_eq!(bounds.size, [0.; 3]);
}
