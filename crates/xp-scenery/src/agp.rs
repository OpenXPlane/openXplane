//! Public-format AG_POINT object annotations. This is a preview subset, not
//! the original autogen evaluator: first tile, full density, flat ground.

#[derive(Debug, Clone)]
pub struct ObjectPlacement {
    pub resource: String,
    /// Tile-relative pixel coordinates converted to meters around its anchor.
    pub offset: [f32; 3],
    pub heading: f32,
}

#[derive(Debug)]
pub struct PointPreview {
    pub objects: Vec<ObjectPlacement>,
    pub rotation: f32,
    pub tiles: usize,
    pub omitted_features: usize,
}

pub fn parse_preview(text: &str) -> Result<PointPreview, String> {
    let lines = text
        .lines()
        .map(|s| s.split('#').next().unwrap_or("").trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if lines.len() < 3
        || !["A", "I"].contains(&lines[0])
        || lines[1] != "1000"
        || lines[2] != "AG_POINT"
    {
        return Err("unsupported AG_POINT header".into());
    }
    let mut definitions = Vec::new();
    let mut scale = None;
    let mut width = None;
    let mut bounds = None;
    let mut anchor = None;
    let mut objects = Vec::new();
    let mut out = PointPreview {
        objects: Vec::new(),
        rotation: 0.0,
        tiles: 0,
        omitted_features: 0,
    };
    for line in &lines[3..] {
        let mut fields = line.split_whitespace();
        let command = fields.next().unwrap();
        let values = fields.collect::<Vec<_>>();
        if command == "OBJECT" {
            if values.is_empty() {
                return Err("empty AGP object definition".into());
            }
            definitions.push(values.join(" "));
            continue;
        }
        if command == "TILE" {
            out.tiles += 1;
        }
        if out.tiles > 1 {
            continue;
        }
        let number = |i: usize| -> Result<f32, String> {
            let n = values
                .get(i)
                .ok_or("missing AGP field")?
                .parse::<f32>()
                .map_err(|_| "invalid AGP number")?;
            if !n.is_finite() {
                return Err("non-finite AGP number".into());
            }
            Ok(n)
        };
        match command {
            "TEXTURE_SCALE" => scale = Some([number(0)?, number(1)?]),
            "TEXTURE_WIDTH" => width = Some(number(0)?),
            "TILE" => bounds = Some([number(0)?, number(1)?, number(2)?, number(3)?]),
            "ANCHOR_PT" => anchor = Some([number(0)?, number(1)?]),
            "ROTATION" => {
                let n = number(0)?;
                if n.fract() != 0.0 || !(0.0..=3.0).contains(&n) {
                    return Err("invalid AGP rotation".into());
                }
                out.rotation = n * 90.0;
            }
            "OBJ_DRAPED" | "OBJ_GRADED" | "OBJ_DELTA" | "OBJ_SCRAPER" => {
                if out.tiles == 0 {
                    return Err("AGP annotation before tile".into());
                }
                let delta = command == "OBJ_DELTA";
                let index = values
                    .get(if delta { 4 } else { 3 })
                    .ok_or("missing AGP object index")?
                    .parse::<usize>()
                    .map_err(|_| "invalid AGP object index")?;
                objects.push((
                    index,
                    [number(0)?, if delta { number(3)? } else { 0.0 }, number(1)?],
                    number(2)?,
                ));
            }
            "FAC" | "TREE" | "TREE_LINE" => out.omitted_features += 1,
            _ => {}
        }
    }
    let b = bounds.ok_or("missing AGP tile")?;
    let s = scale.ok_or("missing AGP texture scale")?;
    let w = width.ok_or("missing AGP texture width")?;
    if s[0] <= 0.0 || s[1] <= 0.0 || w <= 0.0 || b[2] <= b[0] || b[3] <= b[1] {
        return Err("invalid AGP dimensions".into());
    }
    let a = anchor.unwrap_or([(b[0] + b[2]) * 0.5, (b[1] + b[3]) * 0.5]);
    for (index, p, heading) in objects {
        out.objects.push(ObjectPlacement {
            resource: definitions
                .get(index)
                .ok_or("AGP object index out of range")?
                .clone(),
            offset: [(p[0] - a[0]) * w / s[0], p[1], (p[2] - a[1]) * w / s[0]],
            heading,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn annotations_use_anchor_square_pixels_and_delta() {
        let text = "A\n1000\nAG_POINT\nTEXTURE_SCALE 100 200\nTEXTURE_WIDTH 50\nOBJECT models/test object.obj\nTILE 0 0 100 200\nOBJ_DELTA 30 60 45 7 0\nANCHOR_PT 10 20\nROTATION 2\n";
        let p = parse_preview(text).unwrap();
        assert_eq!(p.objects[0].offset, [10.0, 7.0, 20.0]);
        assert_eq!(p.objects[0].heading, 45.0);
        assert_eq!(p.objects[0].resource, "models/test object.obj");
        assert_eq!(p.rotation, 180.0);
        assert!(parse_preview(&text.replace("7 0", "7 1")).is_err());
        assert!(parse_preview(&text.replace("50", "NaN")).is_err());
    }
    #[test]
    fn first_tile_is_explicit_preview_policy() {
        let p = parse_preview("I\n1000\nAG_POINT\nTEXTURE_SCALE 10 10\nTEXTURE_WIDTH 10\nOBJECT a.obj\nTILE 0 0 10 10\nOBJ_DRAPED 5 5 0 0\nTILE 0 0 20 20\nOBJ_DRAPED 2 2 0 0\n").unwrap();
        assert_eq!(p.tiles, 2);
        assert_eq!(p.objects.len(), 1);
        assert_eq!(p.objects[0].offset, [0.0; 3]);
    }
}
