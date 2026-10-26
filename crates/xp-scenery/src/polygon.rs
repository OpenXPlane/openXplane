//! POL base-texture subset and bounded straight-contour draping. Native layer
//! ordering, Bezier/holes, material extensions and surface physics are pending.
pub struct Material {
    pub texture: String,
    pub scale: Option<[f32; 2]>,
    pub repeat: bool,
    pub no_alpha: bool,
    pub layer: String,
    pub offset: i32,
    pub unsupported: std::collections::BTreeSet<String>,
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
        let triangles = crate::world::triangulate(points);
        if triangles.len() != points.len() - 2 {
            return Err("POL triangulation incomplete".into());
        }
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
