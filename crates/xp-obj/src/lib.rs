//! OBJ8 geometry and cached dataref-driven animation evaluation. Not an X-Plane runtime.
pub mod lights;
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
    animation_ops: Vec<usize>,
    enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightKind {
    /// `LIGHT_NAMED name x y z`: a light from the named-light table.
    Named,
    /// `LIGHT_PARAM name x y z params...`: a named light with parameters.
    Param,
    /// `LIGHT_CUSTOM x y z r g b a size u1 v1 u2 v2 dataref`.
    Custom,
    /// `LIGHT_SPILL_CUSTOM x y z r g b a size dx dy dz width dataref`.
    Spill,
    /// `VLIGHT` table entry drawn by `LIGHTS first count`.
    Legacy,
}

/// A light declared by an OBJ. Parsed only: nothing draws it yet.
#[derive(Clone, Debug)]
pub struct Light {
    pub kind: LightKind,
    pub name: Option<String>,
    /// Position in object space, transformed by the animation pose active at the declaration.
    pub position: Vec3,
    pub color: Option<[f32; 4]>,
    /// Remaining numeric arguments in file order.
    pub params: Vec<f32>,
    pub dataref: Option<String>,
}

#[derive(Debug, Default)]
pub struct Object {
    pub lights: Vec<Light>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub draws: Vec<Draw>,
    pub texture: Option<String>,
    pub glass: bool,
    pub unsupported: BTreeMap<String, usize>,
    pub datarefs: BTreeMap<String, usize>,
    pub compatibility_notes: Vec<String>,
    animations: Vec<Animation>,
}

#[derive(Clone)]
struct Pose {
    matrix: Mat4,
    visible: bool,
    operations: Vec<usize>,
}

enum Keys {
    Rotate {
        axis: Vec3,
        keys: Vec<(f32, f32)>,
        dataref: String,
    },
    Translate {
        keys: Vec<(f32, Vec3)>,
        dataref: String,
    },
}

#[derive(Debug)]
enum Animation {
    Rotate {
        axis: Vec3,
        keys: Vec<(f32, f32)>,
        dataref: String,
        linear: bool,
        looping: Option<f32>,
    },
    Translate {
        keys: Vec<(f32, Vec3)>,
        dataref: String,
        linear: bool,
        looping: Option<f32>,
    },
    Visibility {
        low: f32,
        high: f32,
        show: bool,
        dataref: String,
        looping: Option<f32>,
    },
}

impl Animation {
    fn apply(&self, pose: &mut Pose, resolve: &impl Fn(&str) -> Option<f32>) -> Result<(), String> {
        let read = |name: &str| resolve(name).filter(|v| v.is_finite()).unwrap_or(0.0);
        match self {
            Self::Rotate {
                axis,
                keys,
                dataref,
                linear,
                looping,
            } => {
                let value = looping.map_or_else(|| read(dataref), |period| read(dataref) % period);
                let angle = if *linear {
                    keys[0].1 + (keys[1].1 - keys[0].1) * fraction(value, keys[0].0, keys[1].0)
                } else {
                    interpolate(keys, value)?
                };
                pose.matrix *= rotation(*axis, angle)?;
            }
            Self::Translate {
                keys,
                dataref,
                linear,
                looping,
            } => {
                let value = looping.map_or_else(|| read(dataref), |period| read(dataref) % period);
                let offset = if *linear {
                    keys[0]
                        .1
                        .lerp(keys[1].1, fraction(value, keys[0].0, keys[1].0))
                } else {
                    interpolate(keys, value)?
                };
                pose.matrix *= Mat4::from_translation(offset);
            }
            Self::Visibility {
                low,
                high,
                show,
                dataref,
                looping,
            } => {
                let value = looping.map_or_else(|| read(dataref), |period| read(dataref) % period);
                if *low <= value && value <= *high {
                    pose.visible = *show;
                }
            }
        }
        Ok(())
    }
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
    /// Decode meaningful OBJ records as UTF-8 after dropping byte-level comments.
    /// This permits legacy comment encodings without altering paths or datarefs.
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut text = String::new();
        for (line, bytes) in bytes.split(|b| *b == b'\n').enumerate() {
            let end = bytes.iter().position(|b| *b == b'#').unwrap_or(bytes.len());
            let record = std::str::from_utf8(&bytes[..end])
                .map_err(|_| format!("OBJ8 line {}: non-UTF-8 data", line + 1))?;
            text.push_str(record);
            text.push('\n');
        }
        Self::parse(&text)
    }

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
            operations: Vec::new(),
        };
        let mut stack = Vec::new();
        let mut keys: Option<Keys> = None;
        let mut loop_closed_key_block: Option<bool> = None;
        let mut cull = true;
        let mut enabled = true;
        let mut counts = None;
        let mut lod_count = 0;
        let mut vlights: Vec<([f32; 3], [f32; 3])> = Vec::new();
        for (line, record) in records {
            let tokens: Vec<_> = record.split_whitespace().collect();
            let command = tokens[0];
            if command != "ANIM_trans_end" && command != "ANIM_rotate_end" {
                loop_closed_key_block = None;
            }
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
                        obj.animate(
                            &mut pose,
                            Animation::Translate {
                                keys: vec![(v[6], vec3(&v)), (v[7], vec3(&v[3..]))],
                                dataref: args.get(8).unwrap_or(&"none").to_string(),
                                linear: true,
                                looping: None,
                            },
                        )?;
                    }
                    "ANIM_rotate" => {
                        let v = numeric_defaults(args, 5, 7)?;
                        if args.len() < 7 {
                            obj.compatibility_notes.push(format!(
                                "line {line}: omitted rotation calibration defaults to zero"
                            ));
                        }
                        obj.animate(
                            &mut pose,
                            Animation::Rotate {
                                axis: vec3(&v),
                                keys: vec![(v[5], v[3]), (v[6], v[4])],
                                dataref: args.get(7).unwrap_or(&"none").to_string(),
                                linear: true,
                                looping: None,
                            },
                        )?;
                    }
                    "ANIM_rotate_begin" => {
                        if keys.is_some() {
                            return Err("nested animation keys".into());
                        }
                        keys = Some(Keys::Rotate {
                            axis: vec3(&numeric(args, 3)?),
                            keys: vec![],
                            dataref: args.get(3).unwrap_or(&"none").to_string(),
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
                        if keys.is_none() && loop_closed_key_block.take() == Some(true) {
                            return Ok(());
                        }
                        if let Some(Keys::Rotate {
                            axis,
                            keys: values,
                            dataref,
                        }) = keys.take()
                        {
                            obj.animate(
                                &mut pose,
                                Animation::Rotate {
                                    axis,
                                    keys: values,
                                    dataref,
                                    linear: false,
                                    looping: None,
                                },
                            )?;
                        } else {
                            return Err("rotate end outside block".into());
                        }
                    }
                    "ANIM_trans_begin" => {
                        if keys.is_some() {
                            return Err("nested animation keys".into());
                        }
                        keys = Some(Keys::Translate {
                            keys: vec![],
                            dataref: args.first().unwrap_or(&"none").to_string(),
                        });
                    }
                    "ANIM_trans_key" => {
                        let v = numeric(args, 4)?;
                        if let Some(Keys::Translate { keys, .. }) = &mut keys {
                            insert_key(keys, v[0], vec3(&v[1..]));
                        } else {
                            return Err("translation key outside block".into());
                        }
                    }
                    "ANIM_trans_end" => {
                        if keys.is_none() && loop_closed_key_block.take() == Some(false) {
                            return Ok(());
                        }
                        if let Some(Keys::Translate {
                            keys: values,
                            dataref,
                        }) = keys.take()
                        {
                            obj.animate(
                                &mut pose,
                                Animation::Translate {
                                    keys: values,
                                    dataref,
                                    linear: false,
                                    looping: None,
                                },
                            )?;
                        } else {
                            return Err("translation end outside block".into());
                        }
                    }
                    "ANIM_hide" | "ANIM_show" => {
                        let v = numeric(args, 2)?;
                        obj.animate(
                            &mut pose,
                            Animation::Visibility {
                                low: v[0],
                                high: v[1],
                                show: command == "ANIM_show",
                                dataref: args.get(2).unwrap_or(&"none").to_string(),
                                looping: None,
                            },
                        )?;
                    }
                    "ANIM_keyframe_loop" => {
                        let period = numeric(args, 1)?[0];
                        if let Some(pending) = keys.take() {
                            loop_closed_key_block = Some(matches!(&pending, Keys::Rotate { .. }));
                            let animation = match pending {
                                Keys::Rotate {
                                    axis,
                                    keys,
                                    dataref,
                                } => Animation::Rotate {
                                    axis,
                                    keys,
                                    dataref,
                                    linear: false,
                                    looping: None,
                                },
                                Keys::Translate { keys, dataref } => Animation::Translate {
                                    keys,
                                    dataref,
                                    linear: false,
                                    looping: None,
                                },
                            };
                            obj.animate(&mut pose, animation)?;
                            obj.compatibility_notes.push(format!(
                                "line {line}: loop closes pending animation keys in preview"
                            ));
                        }
                        let index = *pose
                            .operations
                            .last()
                            .ok_or("animation loop has no preceding animation")?;
                        match &mut obj.animations[index] {
                            Animation::Rotate { looping, .. }
                            | Animation::Translate { looping, .. } => {
                                *looping = (period != 0.0).then_some(period)
                            }
                            Animation::Visibility { looping, .. } => {
                                *looping = (period != 0.0).then_some(period)
                            }
                        }
                    }
                    "LIGHT_NAMED" | "LIGHT_PARAM" => {
                        let name = args.first().ok_or("light needs a name")?;
                        let v = numeric(&args[1.min(args.len())..], 3)?;
                        let params = args[4.min(args.len())..]
                            .iter()
                            .filter_map(|a| obj_decimal(a).ok())
                            .collect();
                        obj.lights.push(Light {
                            kind: if command == "LIGHT_NAMED" {
                                LightKind::Named
                            } else {
                                LightKind::Param
                            },
                            name: Some((*name).into()),
                            position: pose.matrix.transform_point3(vec3(&v)),
                            color: None,
                            params,
                            dataref: None,
                        });
                    }
                    "LIGHT_CUSTOM" | "LIGHT_SPILL_CUSTOM" => {
                        let v = numeric(args, 7)?;
                        let params = args[7..]
                            .iter()
                            .filter_map(|a| obj_decimal(a).ok())
                            .collect();
                        let dataref = args
                            .last()
                            .filter(|a| a.contains('/') || **a == "none")
                            .map(|a| (*a).to_string());
                        obj.lights.push(Light {
                            kind: if command == "LIGHT_CUSTOM" {
                                LightKind::Custom
                            } else {
                                LightKind::Spill
                            },
                            name: None,
                            position: pose.matrix.transform_point3(vec3(&v)),
                            color: Some([v[3], v[4], v[5], v[6]]),
                            params,
                            dataref,
                        });
                    }
                    "VLIGHT" => {
                        let v = numeric(args, 6)?;
                        vlights.push(([v[0], v[1], v[2]], [v[3], v[4], v[5]]));
                    }
                    "LIGHTS" => {
                        let v: Vec<usize> = args
                            .iter()
                            .map(|a| a.parse().map_err(|_| "invalid LIGHTS range"))
                            .collect::<Result<_, _>>()?;
                        if v.len() != 2 {
                            return Err("LIGHTS expects first and count".into());
                        }
                        let end = v[0].checked_add(v[1]).ok_or("LIGHTS range overflow")?;
                        let table = vlights
                            .get(v[0]..end)
                            .ok_or("LIGHTS outside VLIGHT table")?;
                        for (at, rgb) in table {
                            obj.lights.push(Light {
                                kind: LightKind::Legacy,
                                name: None,
                                position: pose.matrix.transform_point3(Vec3::from_array(*at)),
                                color: Some([rgb[0], rgb[1], rgb[2], 1.0]),
                                params: vec![],
                                dataref: None,
                            });
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
                            animation_ops: pose.operations.clone(),
                            enabled: enabled && lod_count <= 1,
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

    fn animate(&mut self, pose: &mut Pose, animation: Animation) -> Result<(), String> {
        animation.apply(pose, &|_| None)?;
        pose.operations.push(self.animations.len());
        self.animations.push(animation);
        Ok(())
    }

    /// Evaluate cached animation operations without reparsing the OBJ. Unknown or
    /// non-finite datarefs use zero (viewer fallback, not original registry behavior).
    /// Hidden triangles retain degenerate slots so GPU vertex counts stay constant.
    pub fn animated_vertices(
        &self,
        resolve: impl Fn(&str) -> Option<f32>,
    ) -> Result<Vec<Vertex>, String> {
        let mut result = Vec::with_capacity(self.draws.iter().map(|d| d.indices.len()).sum());
        for draw in &self.draws {
            let mut pose = Pose {
                matrix: Mat4::IDENTITY,
                visible: true,
                operations: Vec::new(),
            };
            for index in &draw.animation_ops {
                self.animations[*index].apply(&mut pose, &resolve)?;
            }
            for index in &self.indices[draw.indices.clone()] {
                let mut vertex = self.vertices[*index as usize];
                if pose.visible && draw.enabled {
                    vertex.position = pose
                        .matrix
                        .transform_point3(Vec3::from_array(vertex.position))
                        .to_array();
                    vertex.normal = pose
                        .matrix
                        .transform_vector3(Vec3::from_array(vertex.normal))
                        .normalize_or_zero()
                        .to_array();
                } else {
                    vertex.position = [0.0; 3];
                    vertex.normal = [0.0; 3];
                }
                result.push(vertex);
            }
        }
        Ok(result)
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

    #[test]
    fn light_commands_are_parsed_not_dropped() {
        let o = Object::parse(
            "A\n800\nOBJ\nPOINT_COUNTS 0 0 0 0\nVLIGHT 1 2 3 0.5 0.25 1\nVLIGHT 4 5 6 1 1 1\nLIGHTS 1 1\nLIGHT_NAMED taxi_blue 7 8 9\nLIGHT_PARAM pad 1 2 3 4 5\nLIGHT_CUSTOM 0 1 0 1 0 0 1 2 0 0 1 1 none\nLIGHT_SPILL_CUSTOM 0 0 0 1 1 1 1 5 0 -1 0 0.5 sim/x/y\n",
        )
        .unwrap();
        assert!(o.unsupported.is_empty(), "{:?}", o.unsupported);
        let kinds: Vec<_> = o.lights.iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            [
                LightKind::Legacy,
                LightKind::Named,
                LightKind::Param,
                LightKind::Custom,
                LightKind::Spill
            ]
        );
        assert_eq!(o.lights[0].position, Vec3::new(4.0, 5.0, 6.0));
        assert_eq!(o.lights[1].name.as_deref(), Some("taxi_blue"));
        assert_eq!(o.lights[2].params, vec![4.0, 5.0]);
        assert_eq!(o.lights[3].dataref.as_deref(), Some("none"));
        assert_eq!(o.lights[4].dataref.as_deref(), Some("sim/x/y"));
    }
    #[test]
    fn non_utf8_comments_do_not_hide_invalid_resource_names() {
        let mut bytes = TRI.as_bytes().to_vec();
        bytes.extend(b"# legacy \xa9 comment\nTRIS 0 3\n");
        assert!(Object::parse_bytes(&bytes).is_ok());
        bytes.extend(b"TEXTURE bad\xff.png\n");
        assert!(
            Object::parse_bytes(&bytes)
                .unwrap_err()
                .contains("non-UTF-8 data")
        );
    }

    #[test]
    #[ignore = "requires read-only installed ruscenery via OPENXPLANE_REFERENCE_ROOT"]
    fn installed_legacy_comment_object_loads() {
        let root = std::env::var_os("OPENXPLANE_REFERENCE_ROOT").expect("reference root required");
        let path =
            std::path::Path::new(&root).join("Custom Scenery/ruscenery/buildings/fuel-tank3.obj");
        let object = Object::parse_bytes(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(object.texture.as_deref(), Some("fuel-tank.png"));
        assert!(!object.baked_vertices().is_empty());
    }
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
    fn reevaluates_nested_rotation_and_show_without_changing_vertex_capacity() {
        let o = Object::parse(&format!("{TRI}ANIM_begin\nANIM_trans 2 0 0 2 0 0 0 0 none\nANIM_hide 0 1 sim/hide\nANIM_show 1 1 sim/show\nANIM_begin\nANIM_rotate_begin 0 0 1 sim/angle[3]\nANIM_rotate_key 1 90\nANIM_rotate_key 0 0\nANIM_rotate_end\nTRIS 0 3\nANIM_end\nANIM_end\nTRIS 0 3\n")).unwrap();
        assert_eq!(o.baked_vertices().len(), 3);
        let hidden = o.animated_vertices(|_| None).unwrap();
        assert_eq!(hidden.len(), 6);
        assert!(hidden[..3].iter().all(|v| v.position == [0.0; 3]));
        for angle in [-1.0_f32, 0.0, 1.0, 2.0] {
            let v = o
                .animated_vertices(|name| match name {
                    "sim/show" => Some(1.0),
                    "sim/angle[3]" => Some(angle),
                    _ => None,
                })
                .unwrap();
            assert_eq!(v.len(), 6);
            let radians = angle * std::f32::consts::FRAC_PI_2;
            let expected = Vec3::new(2.0 + radians.cos(), radians.sin(), 0.0);
            assert!((Vec3::from_array(v[1].position) - expected).length() < 1e-5);
            assert_eq!(v[4].position, [1.0, 0.0, 0.0]);
        }
        let v = o
            .animated_vertices(|name| match name {
                "sim/show" | "sim/angle[3]" => Some(1.0),
                _ => None,
            })
            .unwrap();
        assert!((Vec3::from_array(v[1].normal) + Vec3::X).length() < 1e-5);
    }

    #[test]
    fn evaluates_translation_keys_and_keeps_static_zero_result() {
        let o = Object::parse(&format!("{TRI}ANIM_begin\nANIM_trans_begin sim/travel[1]\nANIM_trans_key 1 2 0 0\nANIM_trans_key -1 -2 0 0\nANIM_trans_key 1 3 0 0\nANIM_trans_end\nTRIS 0 3\nANIM_end\n")).unwrap();
        let zero = o.animated_vertices(|_| None).unwrap();
        assert_eq!(zero[0].position, o.baked_vertices()[0].position);
        let v = o
            .animated_vertices(|name| (name == "sim/travel[1]").then_some(2.0))
            .unwrap();
        assert_eq!(v[0].position, [5.5, 0.0, 0.0]);
    }
    #[test]
    fn keyframe_loop_wraps_dataref_before_interpolation() {
        let o = Object::parse(&format!("{TRI}ANIM_begin\nANIM_trans_begin sim/time\nANIM_trans_key 0 0 0 0\nANIM_trans_key 8 8 0 0\nANIM_trans_end\nANIM_keyframe_loop 8\nTRIS 0 3\nANIM_end\n")).unwrap();
        let at_two = o.animated_vertices(|_| Some(2.0)).unwrap();
        let at_ten = o.animated_vertices(|_| Some(10.0)).unwrap();
        assert_eq!(at_two[0].position, at_ten[0].position);
        assert!(!o.unsupported.contains_key("ANIM_keyframe_loop"));
    }
    #[test]
    fn loop_inside_unclosed_key_table_allows_nested_animation() {
        let o = Object::parse(&format!("{TRI}ANIM_begin\nANIM_trans_begin sim/time\nANIM_trans_key 0 0 0 0\nANIM_trans_key 8 8 0 0\nANIM_keyframe_loop 8\nANIM_begin\nANIM_rotate_begin 0 1 0 sim/time\nANIM_rotate_key 0 0\nANIM_rotate_key 8 90\nANIM_keyframe_loop 8\nTRIS 0 3\nANIM_end\nANIM_end\n")).unwrap();
        assert_eq!(
            o.animated_vertices(|_| Some(2.0)).unwrap()[0].position,
            o.animated_vertices(|_| Some(10.0)).unwrap()[0].position
        );
        assert_eq!(o.compatibility_notes.len(), 2);
    }
    #[test]
    fn visibility_loop_wraps_before_hide_check() {
        let o = Object::parse(&format!(
            "{TRI}ANIM_begin\nANIM_hide 1 3 sim/time\nANIM_keyframe_loop 8\nTRIS 0 3\nANIM_end\n"
        ))
        .unwrap();
        assert_eq!(
            o.animated_vertices(|_| Some(2.0)).unwrap()[0].position,
            o.animated_vertices(|_| Some(10.0)).unwrap()[0].position
        );
        assert_ne!(
            o.animated_vertices(|_| Some(2.0)).unwrap()[1].position,
            o.animated_vertices(|_| Some(4.0)).unwrap()[1].position
        );
    }
    #[test]
    fn zero_loop_period_keeps_unwrapped_preview() {
        let o = Object::parse(&format!("{TRI}ANIM_begin\nANIM_trans 0 0 0 8 0 0 0 8 sim/time\nANIM_keyframe_loop 0\nTRIS 0 3\nANIM_end\n")).unwrap();
        assert_eq!(
            o.animated_vertices(|_| Some(10.0)).unwrap()[0].position,
            [10.0, 0.0, 0.0]
        );
    }
    #[test]
    fn keyframe_loop_before_explicit_end_is_accepted() {
        let o = Object::parse(&format!("{TRI}ANIM_begin\nANIM_rotate_begin 0 1 0 sim/time\nANIM_rotate_key 0 0\nANIM_rotate_key 5 90\nANIM_keyframe_loop 5\nANIM_rotate_end\nTRIS 0 3\nANIM_end\n")).unwrap();
        assert_eq!(
            o.animated_vertices(|_| Some(2.0)).unwrap()[1].position,
            o.animated_vertices(|_| Some(7.0)).unwrap()[1].position
        );
    }

    #[test]
    #[ignore = "requires read-only installed UHWW scenery via OPENXPLANE_REFERENCE_ROOT"]
    fn installed_uhww_animated_object_loads() {
        let root = std::env::var_os("OPENXPLANE_REFERENCE_ROOT").expect("reference root required");
        for name in [
            "Custom Scenery/1_UHWW/objects/animated/Rigolan_CargaCinta_Animated.obj",
            "Custom Scenery/1_UHWW/objects/SecurityCameras.obj",
            "Custom Scenery/1_UHWW/objects/Vehicles/Catering_Trucks/Medium/LSG_737_animated.obj",
            "Resources/default scenery/airport scenery/Modern_Airports/Control_Towers/radar_4m_1.obj",
        ] {
            let path = std::path::Path::new(&root).join(name);
            let object = Object::parse(&std::fs::read_to_string(&path).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert!(!object.draws.is_empty());
            assert!(!object.animated_vertices(|_| Some(1.0)).unwrap().is_empty());
        }
    }

    #[test]
    #[ignore = "requires read-only installed UHWW scenery via OPENXPLANE_REFERENCE_ROOT"]
    fn installed_uhww_lights_parse_without_new_failures() {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("obj"))
                {
                    out.push(path);
                }
            }
        }
        let root = std::env::var_os("OPENXPLANE_REFERENCE_ROOT").expect("reference root required");
        let mut files = Vec::new();
        walk(
            &std::path::Path::new(&root).join("Custom Scenery/1_UHWW"),
            &mut files,
        );
        let (mut parsed, mut lights, mut light_errors) = (0, 0, Vec::new());
        for path in files {
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            match Object::parse_bytes(&bytes) {
                Ok(object) => {
                    parsed += 1;
                    lights += object.lights.len();
                    for key in [
                        "LIGHT_NAMED",
                        "LIGHT_PARAM",
                        "LIGHT_CUSTOM",
                        "LIGHT_SPILL_CUSTOM",
                        "VLIGHT",
                        "LIGHTS",
                    ] {
                        assert!(
                            !object.unsupported.contains_key(key),
                            "{}: {key}",
                            path.display()
                        );
                    }
                }
                Err(e) if e.contains("LIGHT") => {
                    light_errors.push(format!("{}: {e}", path.display()))
                }
                Err(_) => {}
            }
        }
        println!("{parsed} objects parsed, {lights} lights");
        assert!(light_errors.is_empty(), "{light_errors:#?}");
        assert!(lights > 0);
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
