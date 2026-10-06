//! OBJ8 geometry and static animation evaluation. Not an X-Plane runtime.
use glam::{Mat4, Vec3};
use std::{collections::BTreeMap, ops::Range};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Clone, Debug)]
pub struct Draw {
    pub indices: Range<usize>,
    pub transform: Mat4,
    pub visible: bool,
    pub cull: bool,
}

#[derive(Debug, Default)]
pub struct Object {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub draws: Vec<Draw>,
    pub texture: Option<String>,
    pub glass: bool,
    pub unsupported: BTreeMap<String, usize>,
    pub datarefs: BTreeMap<String, usize>,
    pub compatibility_notes: Vec<String>,
}

#[derive(Clone)]
struct Pose {
    matrix: Mat4,
    visible: bool,
}

enum Keys {
    Rotate { axis: Vec3, keys: Vec<(f32, f32)> },
    Translate { keys: Vec<(f32, Vec3)> },
}

fn numeric(tokens: &[&str], count: usize) -> Result<Vec<f32>, String> {
    if tokens.len() < count {
        return Err("missing numeric arguments".into());
    }
    tokens[..count].iter().map(|v| obj_decimal(v)).collect()
}

/// Observed decimal scanner at VA 0x140873240: each dot sets the fractional flag;
/// digits accumulate in f64 and the reader then converts to f32. See ACF_LOADING.md.
fn obj_decimal(token: &str) -> Result<f32, String> {
    let (mut value, mut divisor, mut sign) = (0_f64, 1_f64, 1_f64);
    let mut fractional = false;
    let mut digits = false;
    for byte in token.bytes() {
        match byte {
            b'-' => sign = -1.0,
            b'+' => sign = 1.0,
            b'.' => fractional = true,
            b'0'..=b'9' => {
                value = value * 10.0 + (byte - b'0') as f64;
                if fractional {
                    divisor *= 10.0;
                }
                digits = true;
            }
            _ => return Err(format!("unsupported OBJ decimal syntax: {token}")),
        }
    }
    if !digits {
        return Err(format!("invalid number: {token}"));
    }
    let value = (value / divisor * sign) as f32;
    if !value.is_finite() {
        return Err("non-finite value".into());
    }
    Ok(value)
}

fn numeric_defaults(tokens: &[&str], minimum: usize, full: usize) -> Result<Vec<f32>, String> {
    if tokens.len() < minimum {
        return Err("missing numeric arguments".into());
    }
    let mut values = numeric(tokens, tokens.len().min(full))?;
    values.resize(full, 0.0);
    Ok(values)
}

/// Lower-bound insertion and replacement at equal key, observed at VA 0x1408751b0.
fn insert_key<T>(keys: &mut Vec<(f32, T)>, key: f32, value: T) {
    let index = keys.partition_point(|p| p.0 < key);
    if index < keys.len() && keys[index].0 == key {
        keys[index].1 = value;
    } else {
        keys.insert(index, (key, value));
    }
}

fn vec3(v: &[f32]) -> Vec3 {
    Vec3::new(v[0], v[1], v[2])
}

fn rotation(axis: Vec3, angle: f32) -> Result<Mat4, String> {
    if axis.length_squared() < 1e-12 {
        return Err("zero animation rotation axis".into());
    }
    Ok(Mat4::from_axis_angle(axis.normalize(), angle.to_radians()))
}

fn fraction(value: f32, a: f32, b: f32) -> f32 {
    if a == b { 0.0 } else { (value - a) / (b - a) }
}

/// Extrapolate using nearest pair as specified by OBJ8. Input keys are unique and sorted.
fn interpolate<
    T: Copy + std::ops::Sub<Output = T> + std::ops::Add<Output = T> + std::ops::Mul<f32, Output = T>,
>(
    keys: &[(f32, T)],
    value: f32,
) -> Result<T, String> {
    if keys.is_empty() {
        return Err("empty animation keys".into());
    }
    if keys.windows(2).any(|p| p[0].0 > p[1].0) {
        return Err("animation keys must be sorted".into());
    }
    if keys.len() == 1 {
        return Ok(keys[0].1);
    }
    let i = keys
        .windows(2)
        .position(|p| value <= p[1].0)
        .unwrap_or(keys.len() - 2);
    let (a, b) = (keys[i], keys[i + 1]);
    Ok(a.1 + (b.1 - a.1) * fraction(value, a.0, b.0))
}

impl Object {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut records = text.lines().enumerate().filter_map(|(n, l)| {
            let l = l.split('#').next().unwrap().trim();
            (!l.is_empty()).then_some((n + 1, l))
        });
        let marker = records
            .next()
            .ok_or("empty OBJ8")?
            .1
            .trim_start_matches('\u{feff}');
        if !matches!(marker, "A" | "I")
            || records.next().ok_or("missing OBJ version")?.1 != "800"
            || records.next().ok_or("missing OBJ type")?.1 != "OBJ"
        {
            return Err("unsupported OBJ8 header".into());
        }
        let mut obj = Self::default();
        let mut pose = Pose {
            matrix: Mat4::IDENTITY,
            visible: true,
        };
        let mut stack = Vec::new();
        let mut keys: Option<Keys> = None;
        let mut cull = true;
        let mut enabled = true;
        let mut counts = None;
        let mut lod_count = 0;
        for (line, record) in records {
            let tokens: Vec<_> = record.split_whitespace().collect();
            let command = tokens[0];
            let args = &tokens[1..];
            for arg in args {
                if arg.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '+')
                    && arg.matches('.').count() > 1
                {
                    obj.compatibility_notes.push(format!(
                        "line {line}: reference decimal scanner accepts repeated dots in {arg}"
                    ));
                }
            }
            let result = (|| -> Result<(), String> {
                match command {
                    "POINT_COUNTS" => {
                        if args.len() != 4 {
                            return Err("POINT_COUNTS expects four counts".into());
                        }
                        let values: Vec<usize> = args
                            .iter()
                            .map(|s| s.parse().map_err(|_| "invalid count"))
                            .collect::<Result<_, _>>()?;
                        counts = Some(values);
                    }
                    "VT" => {
                        let v = numeric(args, 8)?;
                        obj.vertices.push(Vertex {
                            position: vec3(&v).to_array(),
                            normal: vec3(&v[3..]).to_array(),
                            uv: [v[6], v[7]],
                        });
                    }
                    "IDX" | "IDX10" => {
                        let count = if command == "IDX" { 1 } else { 10 };
                        if args.len() != count {
                            return Err("incorrect index count".into());
                        }
                        for arg in args {
                            obj.indices.push(arg.parse().map_err(|_| "invalid index")?);
                        }
                    }
                    "TEXTURE" => {
                        let path = record.strip_prefix(command).unwrap().trim();
                        if !path.is_empty() {
                            obj.texture = Some(path.into());
                        }
                    }
                    "BLEND_GLASS" => {
                        obj.glass = true;
                    }
                    "ANIM_begin" => stack.push(pose.clone()),
                    "ANIM_end" => {
                        if keys.is_some() {
                            return Err("unclosed animation key block".into());
                        }
                        pose = stack.pop().ok_or("unbalanced ANIM_end")?;
                    }
                    "ANIM_trans" => {
                        let v = numeric_defaults(args, 6, 8)?;
                        if args.len() < 8 {
                            obj.compatibility_notes.push(format!(
                                "line {line}: omitted translation calibration defaults to zero"
                            ));
                        }
                        let t = fraction(0.0, v[6], v[7]);
                        pose.matrix *= Mat4::from_translation(vec3(&v).lerp(vec3(&v[3..]), t));
                    }
                    "ANIM_rotate" => {
                        let v = numeric_defaults(args, 5, 7)?;
                        if args.len() < 7 {
                            obj.compatibility_notes.push(format!(
                                "line {line}: omitted rotation calibration defaults to zero"
                            ));
                        }
                        let angle = v[3] + (v[4] - v[3]) * fraction(0.0, v[5], v[6]);
                        pose.matrix *= rotation(vec3(&v), angle)?;
                    }
                    "ANIM_rotate_begin" => {
                        if keys.is_some() {
                            return Err("nested animation keys".into());
                        }
                        keys = Some(Keys::Rotate {
                            axis: vec3(&numeric(args, 3)?),
                            keys: vec![],
                        });
                    }
                    "ANIM_rotate_key" => {
                        let v = numeric(args, 2)?;
                        if let Some(Keys::Rotate { keys, .. }) = &mut keys {
                            insert_key(keys, v[0], v[1]);
                        } else {
                            return Err("rotate key outside block".into());
                        }
                    }
                    "ANIM_rotate_end" => {
                        if let Some(Keys::Rotate { axis, keys: values }) = keys.take() {
                            pose.matrix *= rotation(axis, interpolate(&values, 0.0)?)?;
                        } else {
                            return Err("rotate end outside block".into());
                        }
                    }
                    "ANIM_trans_begin" => {
                        if keys.is_some() {
                            return Err("nested animation keys".into());
                        }
                        keys = Some(Keys::Translate { keys: vec![] });
                    }
                    "ANIM_trans_key" => {
                        let v = numeric(args, 4)?;
                        if let Some(Keys::Translate { keys }) = &mut keys {
                            insert_key(keys, v[0], vec3(&v[1..]));
                        } else {
                            return Err("translation key outside block".into());
                        }
                    }
                    "ANIM_trans_end" => {
                        if let Some(Keys::Translate { keys: values }) = keys.take() {
                            pose.matrix *= Mat4::from_translation(interpolate(&values, 0.0)?);
                        } else {
                            return Err("translation end outside block".into());
                        }
                    }
                    "ANIM_hide" | "ANIM_show" => {
                        let v = numeric(args, 2)?;
                        let in_range = v[0] <= 0.0 && 0.0 <= v[1];
                        if in_range {
                            pose.visible = command == "ANIM_show";
                        }
                    }
                    "ATTR_cull" => cull = true,
                    "ATTR_no_cull" => cull = false,
                    "ATTR_draw_enable" => enabled = true,
                    "ATTR_draw_disable" => enabled = false,
                    "ATTR_LOD" => {
                        lod_count += 1;
                        *obj.unsupported.entry(command.into()).or_default() += 1;
                    }
                    "TRIS" => {
                        if args.len() != 2 {
                            return Err("TRIS expects offset and count".into());
                        }
                        let start: usize = args[0].parse().map_err(|_| "invalid TRIS offset")?;
                        let count: usize = args[1].parse().map_err(|_| "invalid TRIS count")?;
                        if !count.is_multiple_of(3) {
                            return Err("TRIS count is not divisible by three".into());
                        }
                        let end = start.checked_add(count).ok_or("TRIS range overflow")?;
                        obj.draws.push(Draw {
                            indices: start..end,
                            transform: pose.matrix,
                            visible: pose.visible && enabled && lod_count <= 1,
                            cull,
                        });
                    }
                    _ => {
                        *obj.unsupported.entry(command.into()).or_default() += 1;
                    }
                }
                if command.starts_with("ANIM_") {
                    for arg in args {
                        if arg.contains('/') && *arg != "none" {
                            *obj.datarefs.entry((*arg).into()).or_default() += 1;
                        }
                    }
                }
                Ok(())
            })();
            result.map_err(|e| format!("OBJ8 line {line}: {e}"))?;
        }
        if !stack.is_empty() || keys.is_some() {
            return Err("unclosed animation block".into());
        }
        let counts = counts.ok_or("missing POINT_COUNTS")?;
        if counts[0] != obj.vertices.len() || counts[3] != obj.indices.len() {
            return Err("POINT_COUNTS does not match geometry tables".into());
        }
        for draw in &obj.draws {
            if draw.indices.end > obj.indices.len() {
                return Err("TRIS outside index table".into());
            }
            if obj.indices[draw.indices.clone()]
                .iter()
                .any(|i| *i as usize >= obj.vertices.len())
            {
                return Err("TRIS vertex index outside vertex table".into());
            }
        }
        Ok(obj)
    }

    pub fn baked_vertices(&self) -> Vec<Vertex> {
        let mut result = Vec::new();
        for draw in self.draws.iter().filter(|d| d.visible) {
            for i in &self.indices[draw.indices.clone()] {
                let mut vertex = self.vertices[*i as usize];
                vertex.position = draw
                    .transform
                    .transform_point3(Vec3::from_array(vertex.position))
                    .to_array();
                vertex.normal = draw
                    .transform
                    .transform_vector3(Vec3::from_array(vertex.normal))
                    .normalize_or_zero()
                    .to_array();
                result.push(vertex);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const TRI: &str = "A\n800\nOBJ\nPOINT_COUNTS 3 0 0 3\nVT 0 0 0 0 1 0 0 0\nVT 1 0 0 0 1 0 1 0\nVT 0 0 1 0 1 0 0 1\nIDX 0\nIDX 1\nIDX 2\n";
    #[test]
    fn applies_nested_transforms_and_restores_parent() {
        let o = Object::parse(&format!(
            "{TRI}ANIM_begin\nANIM_trans 2 0 0 2 0 0 0 0 none\nTRIS 0 3\nANIM_end\nTRIS 0 3\n"
        ))
        .unwrap();
        let v = o.baked_vertices();
        assert_eq!(v[0].position, [2.0, 0.0, 0.0]);
        assert_eq!(v[3].position, [0.0, 0.0, 0.0]);
    }
    #[test]
    fn rejects_invalid_ranges_and_indices() {
        assert!(Object::parse(&format!("{TRI}TRIS 0 6")).is_err());
        assert!(Object::parse(&format!("{}TRIS 0 3", TRI.replace("IDX 2", "IDX 9"))).is_err());
        assert!(Object::parse(&format!("{TRI}ANIM_begin\nTRIS 0 3")).is_err());
    }
    #[test]
    fn evaluates_keys_and_visibility_at_zero() {
        let o = Object::parse(&format!("{TRI}ANIM_begin\nANIM_trans_begin sim/test\nANIM_trans_key -1 -1 0 0\nANIM_trans_key 1 1 0 0\nANIM_trans_end\nTRIS 0 3\nANIM_end\nANIM_begin\nANIM_hide 0 1 sim/test\nTRIS 0 3\nANIM_end")).unwrap();
        assert_eq!(o.baked_vertices().len(), 3);
        assert_eq!(o.baked_vertices()[0].position, [0.0, 0.0, 0.0]);
        assert_eq!(interpolate(&[(0.0, 0.0), (1.0, 2.0)], 2.0).unwrap(), 4.0);
    }

    #[test]
    fn show_resumes_drawing_after_multiple_hides() {
        let o = Object::parse(&format!("{TRI}ANIM_begin\nANIM_hide 0 1 sim/a\nANIM_hide 0 1 sim/b\nANIM_show 0 1 sim/c\nTRIS 0 3\nANIM_end\nANIM_begin\nANIM_show 2 3 sim/d\nTRIS 0 3\nANIM_end")).unwrap();
        assert_eq!(o.baked_vertices().len(), 6);
    }

    #[test]
    fn reference_decimal_scanner_accepts_stock_vor_typo() {
        assert_eq!(obj_decimal("-2.5.000000").unwrap(), -2.5);
        assert_eq!(obj_decimal(".25").unwrap(), 0.25);
        assert!(obj_decimal("NaN").is_err());
        assert!(obj_decimal("1e3").is_err());
    }

    #[test]
    fn static_shorthand_and_unsorted_duplicate_keys_follow_reference() {
        let o = Object::parse(&format!("{TRI}ANIM_begin\nANIM_trans 2 0 0 2 0 0\nANIM_rotate 0 1 0 0 0\nANIM_rotate_begin 0 1 0 sim/test\nANIM_rotate_key 0 99\nANIM_rotate_key -20 -20\nANIM_rotate_key 20 20\nANIM_rotate_key 0 0\nANIM_rotate_end\nTRIS 0 3\nANIM_end")).unwrap();
        assert_eq!(o.baked_vertices()[0].position, [2.0, 0.0, 0.0]);
        assert_eq!(o.compatibility_notes.len(), 2);
        let mut keys = vec![];
        insert_key(&mut keys, 1.0, 10.0);
        insert_key(&mut keys, 0.0, 0.0);
        insert_key(&mut keys, 1.0, 20.0);
        assert_eq!(keys, vec![(0.0, 0.0), (1.0, 20.0)]);
    }
}
