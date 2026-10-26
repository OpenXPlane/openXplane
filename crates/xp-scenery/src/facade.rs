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
    }
}
