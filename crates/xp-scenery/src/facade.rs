//! Type-2 FAC input. Geometry is retained in file coordinates; wall fitting and
//! native selection are separate work, not implied by successful parsing.

#[derive(Debug)]
pub struct Mesh {
    pub group: u32,
    pub far_lod: f32,
    pub cuts: u32,
    pub vertices: Vec<[f32; 8]>,
    pub indices: Vec<usize>,
    expected_vertices: usize,
    expected_indices: usize,
}

#[derive(Debug, Default)]
pub struct Segment {
    pub curved: bool,
    pub meshes: Vec<Mesh>,
}

#[derive(Debug, Default)]
pub struct Wall {
    pub rules: Vec<[f32; 4]>,
    pub spellings: Vec<Vec<usize>>,
}

#[derive(Debug, Default)]
pub struct Floor {
    pub name: String,
    pub roofs: Vec<f32>,
    pub segments: Vec<Segment>,
    pub walls: Vec<Wall>,
}

#[derive(Debug)]
pub struct Facade {
    pub floors: Vec<Floor>,
    pub wall_texture: Option<String>,
    pub roof_texture: Option<String>,
    pub roof_scale: Option<[f32; 2]>,
    pub ring: bool,
    pub graded: bool,
    pub no_wall_mesh: bool,
    pub no_roof_mesh: bool,
    pub unsupported: std::collections::BTreeSet<String>,
}

impl Facade {
    /// Straight-wall preview in x-east/z-south coordinates. Selection is
    /// deterministic; original random fits, corner miters and curves are pending.
    pub fn straight_walls(
        &self,
        points: &[[f32; 2]],
        choices: &[usize],
        height: f32,
    ) -> Result<Vec<[f32; 8]>, String> {
        if points.len() < 2
            || points.len() != choices.len()
            || !height.is_finite()
            || points.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid FAC footprint".into());
        }
        let floor = self
            .floors
            .iter()
            .filter(|f| !f.roofs.is_empty())
            .min_by(|a, b| {
                let delta = |f: &Floor| {
                    (f.roofs.iter().copied().fold(f32::NEG_INFINITY, f32::max) - height).abs()
                };
                delta(a).total_cmp(&delta(b))
            })
            .ok_or("FAC has no roof-height floor")?;
        let segments = floor
            .segments
            .iter()
            .filter(|s| !s.curved)
            .collect::<Vec<_>>();
        let length = |i: usize| -> Result<f32, String> {
            let segment = segments.get(i).ok_or("FAC spelling index out of range")?;
            let length = segment
                .meshes
                .iter()
                .flat_map(|m| &m.vertices)
                .map(|v| -v[2])
                .fold(0.0f32, f32::max);
            if length <= 0.0 || !length.is_finite() {
                return Err("FAC segment has no negative-Z length".into());
            }
            Ok(length)
        };
        let mut out = Vec::new();
        if self.no_wall_mesh {
            return Ok(out);
        }
        let walls = if self.ring {
            points.len()
        } else {
            points.len() - 1
        };
        for n in 0..walls {
            let a = points[n];
            let b = points[(n + 1) % points.len()];
            let dx = b[0] - a[0];
            let dz = b[1] - a[1];
            let distance = dx.hypot(dz);
            if distance < 0.001 {
                continue;
            }
            let wall = floor
                .walls
                .get(choices[n])
                .ok_or("FAC wall choice out of range")?;
            let mut best = None;
            for spelling in &wall.spellings {
                let total = spelling
                    .iter()
                    .map(|&i| length(i))
                    .collect::<Result<Vec<_>, _>>()?
                    .iter()
                    .sum::<f32>();
                if total <= 0.0 {
                    continue;
                }
                let repeats = (distance / total).round().clamp(1.0, 4096.0) as usize;
                let error = (distance / (total * repeats as f32) - 1.0).abs();
                if best.as_ref().is_none_or(|(_, _, _, e)| error < *e) {
                    best = Some((spelling, total, repeats, error));
                }
            }
            let (spelling, total, repeats, _) = best.ok_or("FAC wall has no spelling")?;
            let scale = distance / (total * repeats as f32);
            let (ux, uz) = (dx / distance, dz / distance);
            let mut offset = 0.0;
            for _ in 0..repeats {
                for &index in spelling {
                    for mesh in &segments[index].meshes {
                        for &i in &mesh.indices {
                            let v = mesh.vertices[i];
                            // local -Z follows the edge, +X points to its right.
                            let along = offset - v[2] * scale;
                            let mut nx = -uz * v[3] - ux * v[5] / scale;
                            let mut ny = v[4];
                            let mut nz = ux * v[3] - uz * v[5] / scale;
                            let magnitude = (nx * nx + ny * ny + nz * nz).sqrt();
                            if magnitude > 0.0 {
                                nx /= magnitude;
                                ny /= magnitude;
                                nz /= magnitude;
                            }
                            out.push([
                                a[0] + ux * along - uz * v[0],
                                v[1],
                                a[1] + uz * along + ux * v[0],
                                nx,
                                ny,
                                nz,
                                v[6],
                                v[7],
                            ]);
                            if out.len() > 6_000_000 {
                                return Err("FAC preview vertex limit".into());
                            }
                        }
                    }
                    offset += length(index)? * scale;
                }
            }
        }
        Ok(out)
    }
}

pub fn parse(text: &str) -> Result<Facade, String> {
    let lines = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>();
    if lines.len() < 3
        || !["A", "I"].contains(&lines[0])
        || lines[1] != "1000"
        || lines[2] != "FACADE"
    {
        return Err("expected type-2 FAC version 1000".into());
    }
    let mut out = Facade {
        floors: Vec::new(),
        wall_texture: None,
        roof_texture: None,
        roof_scale: None,
        ring: true,
        graded: false,
        no_wall_mesh: false,
        no_roof_mesh: false,
        unsupported: Default::default(),
    };
    let mut roof_shader = false;
    for line in &lines[3..] {
        let mut fields = line.split_whitespace();
        let command = fields.next().unwrap();
        let values = fields.collect::<Vec<_>>();
        let number = |i: usize| -> Result<f32, String> {
            let n = values
                .get(i)
                .ok_or("missing FAC field")?
                .parse::<f32>()
                .map_err(|_| "invalid FAC number")?;
            if !n.is_finite() {
                return Err("nonfinite FAC value".into());
            }
            Ok(n)
        };
        let integer = |i: usize| -> Result<usize, String> {
            values
                .get(i)
                .ok_or("missing FAC integer")?
                .parse()
                .map_err(|_| "invalid FAC integer".into())
        };
        match command {
            "SHADER_WALL" => roof_shader = false,
            "SHADER_ROOF" => roof_shader = true,
            "TEXTURE" => {
                let path = values.join(" ");
                if path.is_empty() {
                    return Err("empty FAC texture path".into());
                }
                if roof_shader {
                    out.roof_texture = Some(path);
                } else {
                    out.wall_texture = Some(path);
                }
            }
            "ROOF_SCALE" => {
                let x = number(0)?;
                let y = if values.len() > 1 { number(1)? } else { x };
                if x <= 0.0 || y <= 0.0 {
                    return Err("invalid FAC roof scale".into());
                }
                out.roof_scale = Some([x, y]);
            }
            "GRADED" => out.graded = true,
            "RING" => {
                let n = integer(0)?;
                if n > 1 {
                    return Err("invalid FAC ring".into());
                }
                out.ring = n == 1;
            }
            "NO_WALL_MESH" => out.no_wall_mesh = true,
            "NO_ROOF_MESH" => out.no_roof_mesh = true,
            "FLOOR" => out.floors.push(Floor {
                name: values.join(" "),
                ..Default::default()
            }),
            "ROOF_HEIGHT" => out
                .floors
                .last_mut()
                .ok_or("FAC roof before floor")?
                .roofs
                .push(number(0)?),
            "SEGMENT" | "SEGMENT_CURVED" => out
                .floors
                .last_mut()
                .ok_or("FAC segment before floor")?
                .segments
                .push(Segment {
                    curved: command == "SEGMENT_CURVED",
                    ..Default::default()
                }),
            "MESH" => {
                let vertex_count = integer(3)?;
                let index_count = integer(4)?;
                if vertex_count > 2_000_000
                    || index_count > 6_000_000
                    || !index_count.is_multiple_of(3)
                {
                    return Err("invalid FAC mesh size".into());
                }
                let segment = out
                    .floors
                    .last_mut()
                    .and_then(|f| f.segments.last_mut())
                    .ok_or("FAC mesh before segment")?;
                segment.meshes.push(Mesh {
                    group: integer(0)?.try_into().map_err(|_| "FAC group overflow")?,
                    far_lod: number(1)?,
                    cuts: integer(2)?.try_into().map_err(|_| "FAC cuts overflow")?,
                    vertices: Vec::new(),
                    indices: Vec::new(),
                    expected_vertices: vertex_count,
                    expected_indices: index_count,
                });
            }
            "VERTEX" | "IDX" => {
                let mesh = out
                    .floors
                    .last_mut()
                    .and_then(|f| f.segments.last_mut())
                    .and_then(|s| s.meshes.last_mut())
                    .ok_or("FAC vertex/index before mesh")?;
                if command == "VERTEX" {
                    let mut v = [0.0; 8];
                    for (i, n) in v.iter_mut().enumerate() {
                        *n = number(i)?;
                    }
                    mesh.vertices.push(v);
                    if mesh.vertices.len() > mesh.expected_vertices {
                        return Err("too many FAC vertices".into());
                    }
                } else {
                    for i in 0..values.len() {
                        mesh.indices.push(integer(i)?);
                    }
                    if mesh.indices.len() > mesh.expected_indices {
                        return Err("too many FAC indices".into());
                    }
                }
            }
            "WALL" | "WALL_RULE" => {
                let floor = out.floors.last_mut().ok_or("FAC wall before floor")?;
                let rule = [number(0)?, number(1)?, number(2)?, number(3)?];
                if command == "WALL" {
                    floor.walls.push(Wall::default());
                }
                floor
                    .walls
                    .last_mut()
                    .ok_or("FAC rule before wall")?
                    .rules
                    .push(rule);
            }
            "SPELLING" => {
                let indices = (0..values.len())
                    .map(integer)
                    .collect::<Result<Vec<_>, _>>()?;
                out.floors
                    .last_mut()
                    .and_then(|f| f.walls.last_mut())
                    .ok_or("FAC spelling before wall")?
                    .spellings
                    .push(indices);
            }
            _ => {
                out.unsupported.insert(command.into());
            }
        }
    }
    if out.floors.is_empty() {
        return Err("FAC has no floors".into());
    }
    for floor in &out.floors {
        let count = floor.segments.iter().filter(|s| !s.curved).count();
        for wall in &floor.walls {
            if wall
                .spellings
                .iter()
                .any(|s| s.is_empty() || s.iter().any(|i| *i >= count))
            {
                return Err("invalid FAC spelling segment".into());
            }
        }
        for segment in &floor.segments {
            for mesh in &segment.meshes {
                if mesh.vertices.len() != mesh.expected_vertices
                    || mesh.indices.len() != mesh.expected_indices
                    || mesh.indices.iter().any(|i| *i >= mesh.vertices.len())
                {
                    return Err("incomplete FAC mesh or invalid index".into());
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_meshes_materials_and_spelling_with_validation() {
        let text = "A\n1000\nFACADE\nTEXTURE wall.png\nSHADER_ROOF\nTEXTURE roof.png\nROOF_SCALE 10\nGRADED\nFLOOR one\nROOF_HEIGHT 3\nSEGMENT 0\nMESH 0 20000 1 3 3\nVERTEX 0 0 0 1 0 0 0 0\nVERTEX 0 3 0 1 0 0 0 1\nVERTEX 0 3 -2 1 0 0 1 1\nIDX 0 1 2\nWALL 0 100 0 0\nSPELLING 0\n";
        let f = parse(text).unwrap();
        assert_eq!(f.wall_texture.as_deref(), Some("wall.png"));
        assert_eq!(f.roof_texture.as_deref(), Some("roof.png"));
        assert_eq!(f.roof_scale, Some([10.0, 10.0]));
        assert_eq!(f.floors[0].segments[0].meshes[0].vertices[2][2], -2.0);
        assert_eq!(f.floors[0].walls[0].spellings, vec![vec![0]]);
        assert!(parse(&text.replace("IDX 0 1 2", "IDX 0 1 3")).is_err());
        assert!(parse(&text.replace("SPELLING 0", "SPELLING 1")).is_err());
        assert!(parse(&text.replace("3 3\nVERTEX", "4 3\nVERTEX")).is_err());
        assert!(parse(&text.replace("ROOF_HEIGHT 3", "ROOF_HEIGHT NaN")).is_err());
        let mut f = parse(text).unwrap();
        f.ring = false;
        let v = f
            .straight_walls(&[[10.0, 20.0], [14.0, 20.0]], &[0, 0], 3.0)
            .unwrap();
        assert_eq!(v.len(), 6); // two instances of the 2m source segment
        assert_eq!(&v[0][..3], &[10.0, 0.0, 20.0]);
        assert_eq!(&v[2][..3], &[12.0, 3.0, 20.0]);
        assert_eq!(&v[5][..3], &[14.0, 3.0, 20.0]);
        assert_eq!(&v[0][3..6], &[0.0, 0.0, 1.0]);
        assert!(
            f.straight_walls(&[[0.0, 0.0], [1.0, 0.0]], &[1, 0], 3.0)
                .is_err()
        );
        let stretched = f
            .straight_walls(&[[0.0, 0.0], [0.0, 3.0]], &[0, 0], 3.0)
            .unwrap();
        assert_eq!(&stretched[5][..3], &[0.0, 3.0, 3.0]);
        assert_eq!(&stretched[0][3..6], &[-1.0, 0.0, 0.0]);
        f.no_wall_mesh = true;
        assert!(
            f.straight_walls(&[[0.0, 0.0], [2.0, 0.0]], &[0, 0], 3.0)
                .unwrap()
                .is_empty()
        );
    }
}
