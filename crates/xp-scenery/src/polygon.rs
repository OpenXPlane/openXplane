//! POL base-texture subset with holes and bounded Bezier/drape tessellation.
//! Native tessellation/layer ordering, material extensions and physics are pending.
pub struct Material {
    pub texture: String,
    pub scale: Option<[f32; 2]>,
    pub repeat: bool,
    pub no_alpha: bool,
    pub layer: String,
    pub offset: i32,
    pub unsupported: std::collections::BTreeSet<String>,
}

pub struct Footprint {
    pub points: Vec<[f32; 2]>,
    pub holes: Vec<usize>,
    pub st: Option<Vec<[f32; 2]>>,
}

/// Interpret DSF straight/Bezier coordinate layouts. Fixed eight subdivisions
/// per curved side follow the existing port preview, not native tessellation.
pub fn footprint(
    windings: &[Vec<Vec<f64>>],
    frame: &crate::world::Frame,
    explicit: bool,
) -> Result<Footprint, String> {
    let mut out = Footprint {
        points: Vec::new(),
        holes: Vec::new(),
        st: explicit.then(Vec::new),
    };
    for (ring_index, winding) in windings.iter().enumerate() {
        let mut n = winding.len();
        if n > 1 && winding[0].get(..2) == winding[n - 1].get(..2) {
            n -= 1;
        }
        if n < 3 {
            return Err("POL ring too short".into());
        }
        let dimensions = winding[0].len();
        let (flat, curved) = if explicit { (4, 8) } else { (2, 4) };
        if ![flat, curved].contains(&dimensions)
            || winding
                .iter()
                .any(|p| p.len() != dimensions || p.iter().any(|v| !v.is_finite()))
        {
            return Err("unsupported POL coordinate layout".into());
        }
        if ring_index > 0 {
            out.holes.push(out.points.len());
        }
        for i in 0..n {
            let a = &winding[i];
            let b = &winding[(i + 1) % n];
            let steps = if dimensions == curved { 8 } else { 1 };
            for step in 0..steps {
                let t = step as f64 / steps as f64;
                let u = 1.0 - t;
                let cubic = |start: f64, c0: f64, c1: f64, end: f64| {
                    u * u * u * start
                        + 3.0 * u * u * t * c0
                        + 3.0 * u * t * t * c1
                        + t * t * t * end
                };
                let coordinate = |axis: usize| {
                    if dimensions == curved {
                        cubic(a[axis], a[axis + 2], 2.0 * b[axis] - b[axis + 2], b[axis])
                    } else {
                        a[axis]
                    }
                };
                out.points
                    .push(frame.to_local(coordinate(1), coordinate(0)));
                if let Some(st) = &mut out.st {
                    st.push(if dimensions == curved {
                        [
                            cubic(a[4], a[6], 2.0 * b[4] - b[6], b[4]) as f32,
                            cubic(a[5], a[7], 2.0 * b[5] - b[7], b[5]) as f32,
                        ]
                    } else {
                        [a[2] as f32, a[3] as f32]
                    });
                }
            }
        }
    }
    if out.points.is_empty() {
        return Err("POL has no contours".into());
    }
    Ok(out)
}

impl Material {
    pub fn parse(text: &str) -> Result<Self, String> {
        let lines = text
            .lines()
            .map(|l| l.split('#').next().unwrap_or("").trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>();
        if lines.len() < 3
            || !["A", "I"].contains(&lines[0])
            || lines[1] != "850"
            || lines[2] != "DRAPED_POLYGON"
        {
            return Err("expected POL850 header".into());
        }
        let mut out = Self {
            texture: String::new(),
            scale: None,
            repeat: true,
            no_alpha: false,
            layer: "taxiways".into(),
            offset: 0,
            unsupported: Default::default(),
        };
        for line in &lines[3..] {
            let mut fields = line.split_whitespace();
            let command = fields.next().unwrap();
            let values = fields.collect::<Vec<_>>();
            match command {
                "TEXTURE" | "TEXTURE_NOWRAP" | "TEXTURE_NOWRWAP" => {
                    out.texture = values.join(" ").replace('\\', "/");
                    out.repeat = command == "TEXTURE";
                }
                "SCALE" => {
                    let value = |i: usize| {
                        values
                            .get(i)
                            .ok_or("missing POL scale")?
                            .parse::<f32>()
                            .map_err(|_| "invalid POL scale")
                    };
                    let scale = [value(0)?, value(1)?];
                    if scale.iter().any(|v| !v.is_finite() || *v <= 0.0) {
                        return Err("invalid POL scale".into());
                    }
                    out.scale = Some(scale);
                }
                "NO_ALPHA" => out.no_alpha = true,
                "LAYER_GROUP" => {
                    out.layer = values.first().ok_or("missing POL layer")?.to_string();
                    out.offset = values
                        .get(1)
                        .ok_or("missing POL layer offset")?
                        .parse()
                        .map_err(|_| "invalid POL offset")?;
                }
                _ => {
                    out.unsupported.insert(command.into());
                }
            }
        }
        if out.texture.is_empty() {
            return Err("POL texture missing".into());
        }
        Ok(out)
    }

    pub fn vertices(
        &self,
        points: &[[f32; 2]],
        st: Option<&[[f32; 2]]>,
        parameter: u16,
        ground: Option<&crate::ground::Ground>,
    ) -> Result<Vec<[f32; 8]>, String> {
        self.vertices_with_holes(points, &[], st, parameter, ground)
    }

    pub fn vertices_with_holes(
        &self,
        points: &[[f32; 2]],
        holes: &[usize],
        st: Option<&[[f32; 2]]>,
        parameter: u16,
        ground: Option<&crate::ground::Ground>,
    ) -> Result<Vec<[f32; 8]>, String> {
        if points.len() < 3 || points.iter().flatten().any(|v| !v.is_finite()) {
            return Err("invalid POL footprint".into());
        }
        let uv = if parameter == 65535 {
            let st = st.ok_or("POL explicit ST missing")?;
            if st.len() != points.len() || st.iter().flatten().any(|v| !v.is_finite()) {
                return Err("invalid POL ST".into());
            }
            st.to_vec()
        } else {
            let scale = self.scale.ok_or("POL projected scale missing")?;
            let angle = ((parameter % 360) as f32 + (parameter / 360) as f32 / 128.0).to_radians();
            let (s, c) = angle.sin_cos();
            points
                .iter()
                .map(|p| {
                    [
                        (p[0] * c + p[1] * s) / scale[0],
                        (p[0] * s - p[1] * c) / scale[1],
                    ]
                })
                .collect()
        };
        if holes.windows(2).any(|w| w[0] >= w[1])
            || holes.iter().any(|&i| i < 3 || i >= points.len())
        {
            return Err("invalid POL hole indices".into());
        }
        let mut first = 0;
        for end in holes.iter().copied().chain(std::iter::once(points.len())) {
            if end - first < 3 {
                return Err("POL ring needs three points".into());
            }
            first = end;
        }
        let coordinates = points
            .iter()
            .flat_map(|p| p.map(f64::from))
            .collect::<Vec<_>>();
        let indices = earcutr::earcut(&coordinates, holes, 2)
            .map_err(|e| format!("POL triangulation: {e}"))?;
        let deviation = earcutr::deviation(&coordinates, holes, 2, &indices);
        if indices.is_empty() || !deviation.is_finite() || deviation > 1e-5 {
            return Err(format!("POL triangulation area mismatch: {deviation}"));
        }
        let triangles = indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let mut t = *t;
                let [a, b, c] = t.map(|i| points[i]);
                if (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) < 0.0 {
                    t.swap(1, 2);
                }
                t
            })
            .collect::<Vec<_>>();
        let mut pending = triangles
            .into_iter()
            .map(|t| {
                (
                    t.map(|i| [points[i][0], points[i][1], uv[i][0], uv[i][1]]),
                    0,
                )
            })
            .collect::<Vec<_>>();
        let bias = match self.layer.as_str() {
            "terrain" => 0.02,
            "taxiways" => 0.13,
            _ => 0.23,
        } + self.offset.clamp(-30, 30) as f32 * 0.001;
        let mut out = Vec::new();
        while let Some((t, depth)) = pending.pop() {
            let distance =
                |i: usize| (t[i][0] - t[(i + 1) % 3][0]).hypot(t[i][1] - t[(i + 1) % 3][1]);
            let edge = (0..3)
                .max_by(|&a, &b| distance(a).total_cmp(&distance(b)))
                .unwrap();
            if depth < 16 && distance(edge) > 50.0 {
                let next = (edge + 1) % 3;
                let other = (edge + 2) % 3;
                let mid = std::array::from_fn(|i| (t[edge][i] + t[next][i]) * 0.5);
                pending.push(([t[edge], mid, t[other]], depth + 1));
                pending.push(([mid, t[next], t[other]], depth + 1));
            } else {
                for i in [0, 2, 1] {
                    let p = t[i];
                    let height = ground
                        .and_then(|g| g.terrain_sample(p[0], p[1]))
                        .map_or(0.0, |(y, _)| y);
                    out.push([p[0], height + bias, p[1], 0.0, 1.0, 0.0, p[2], p[3]]);
                }
                if out.len() > 6_000_000 {
                    return Err("POL vertex limit".into());
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holes_keep_area_and_empty_centers_with_explicit_st() {
        let m = Material::parse("A\n850\nDRAPED_POLYGON\nTEXTURE ground.png\nSCALE 1 1\n").unwrap();
        let points = [
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
            [3.0, 3.0],
            [3.0, 7.0],
            [7.0, 7.0],
            [7.0, 3.0],
        ];
        let st = points.map(|p| [p[0] / 10.0, p[1] / 10.0]);
        let vertices = m
            .vertices_with_holes(&points, &[4], Some(&st), 65535, None)
            .unwrap();
        let area: f32 = vertices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let x = (t[0][0] + t[1][0] + t[2][0]) / 3.0;
                let z = (t[0][2] + t[1][2] + t[2][2]) / 3.0;
                assert!(!(x > 3.0 && x < 7.0 && z > 3.0 && z < 7.0));
                let area = ((t[1][2] - t[0][2]) * (t[2][0] - t[0][0])
                    - (t[1][0] - t[0][0]) * (t[2][2] - t[0][2]))
                    * 0.5;
                assert!(area > 0.0);
                area
            })
            .sum();
        assert!((area - 84.0).abs() < 1e-4);
        assert!(
            vertices
                .iter()
                .all(|v| (v[6] - v[0] / 10.0).abs() < 1e-5 && (v[7] - v[2] / 10.0).abs() < 1e-5)
        );
        assert!(m.vertices_with_holes(&points, &[2], None, 0, None).is_err());
        assert!(
            m.vertices_with_holes(&points, &[4, 4], None, 0, None)
                .is_err()
        );
    }

    #[test]
    fn bezier_geometry_and_uv_follow_the_same_parameterization() {
        let frame = crate::world::Frame::new(0.0, 0.0);
        let winding = vec![
            vec![0.0, 0.0, 0.00003, 0.0, 0.0, 0.0, 0.3, 0.0],
            vec![0.0001, 0.0, 0.0001, 0.00003, 1.0, 0.0, 1.0, 0.3],
            vec![0.0, 0.0001, -0.00003, 0.0001, 0.0, 1.0, -0.3, 1.0],
        ];
        let f = footprint(&[winding], &frame, true).unwrap();
        assert_eq!(f.points.len(), 24);
        assert!(f.holes.is_empty());
        let meters = frame.to_local(0.0, 0.0001)[0];
        for (p, st) in f.points.iter().zip(f.st.unwrap()) {
            assert!((p[0] / meters - st[0]).abs() < 1e-5);
            assert!((-p[1] / meters - st[1]).abs() < 1e-5);
        }
        assert!(footprint(&[vec![vec![0.0; 7]; 3]], &frame, true).is_err());
    }
    #[test]
    fn explicit_uv_scale_heading_and_terrain_draping() {
        let text = "A\n850\nDRAPED_POLYGON\nTEXTURE_NOWRAP pavement.png\nSCALE 10 20\nNO_ALPHA\nLAYER_GROUP taxiways 1\nSURFACE concrete\n";
        let m = Material::parse(text).unwrap();
        assert!(!m.repeat && m.no_alpha);
        assert!(m.unsupported.contains("SURFACE"));
        let points = [[0.0, 0.0], [100.0, 0.0], [0.0, 100.0]];
        let ground = crate::ground::Ground::new(vec![[
            [0.0, 2.0, 0.0],
            [100.0, 12.0, 0.0],
            [0.0, 2.0, 100.0],
        ]]);
        let v = m.vertices(&points, None, 0, Some(&ground)).unwrap();
        assert!(v.len() > 3);
        for p in &v {
            assert!((p[1] - (2.131 + p[0] * 0.1)).abs() < 1e-4);
            assert!((p[6] - p[0] / 10.0).abs() < 1e-4);
        }
        let st = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        let v = m.vertices(&points, Some(&st), 65535, None).unwrap();
        assert!(
            v.iter()
                .all(|p| (p[6] - p[0] / 100.0).abs() < 1e-5 && (p[7] - p[2] / 100.0).abs() < 1e-5)
        );
        assert!(m.vertices(&points, None, 65535, None).is_err());
        assert!(Material::parse(&text.replace("10 20", "0 20")).is_err());
        let rotated = m.vertices(&points, None, 90, None).unwrap();
        for p in &rotated {
            assert!((p[6] - p[2] / 10.0).abs() < 1e-4);
            assert!((p[7] - p[0] / 20.0).abs() < 1e-4);
        }
    }
}
