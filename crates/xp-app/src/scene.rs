use glam::Vec3;
use openxplane::{
    Aircraft,
    obj8::{Object, Vertex},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub texture: image::RgbaImage,
    pub glass: bool,
    pub center: Vec3,
}

pub struct Scene {
    pub meshes: Vec<Mesh>,
    pub center: Vec3,
    pub radius: f32,
    /// Clear colour (linear RGB).
    pub background: [f64; 3],
}

/// Clear colour of the aircraft-only preview.
pub const PREVIEW_BACKGROUND: [f64; 3] = [0.028, 0.045, 0.07];

pub fn texture_path(object: &Path, name: &str) -> Option<PathBuf> {
    let path = object.parent()?.join(name.replace('\\', "/"));
    // X-Plane assets commonly declare PNG while shipping a DDS with the same stem.
    let dds = path.with_extension("dds");
    if dds.is_file() {
        Some(dds)
    } else if path.is_file() {
        Some(path)
    } else {
        None
    }
}

pub fn audit(aircraft_path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let acf = Aircraft::parse(&fs::read_to_string(aircraft_path)?)?;
    let dir = aircraft_path.parent().ok_or("ACF needs parent directory")?;
    let mut unsupported = BTreeMap::<String, usize>::new();
    let mut objects = 0;
    let mut triangles = 0;
    let mut missing = Vec::new();
    let mut errors = Vec::new();
    for p in acf
        .properties
        .iter()
        .filter(|p| p.key.ends_with("/_v10_att_file_stl") && !p.value.is_empty())
    {
        let path = dir.join("objects").join(p.value.replace('\\', "/"));
        let object = match fs::read_to_string(&path) {
            Ok(text) => match Object::parse(&text) {
                Ok(object) => object,
                Err(e) => {
                    errors.push(format!("{}: {e}", path.display()));
                    continue;
                }
            },
            Err(e) => {
                errors.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        objects += 1;
        triangles += object
            .draws
            .iter()
            .map(|d| d.indices.len() / 3)
            .sum::<usize>();
        if let Some(name) = &object.texture
            && texture_path(&path, name).is_none()
        {
            missing.push(format!("{}: {name}", path.display()));
        }
        for (key, count) in object.unsupported {
            *unsupported.entry(key).or_default() += count;
        }
        for note in object.compatibility_notes {
            println!("  COMPAT {}: {note}", path.display());
        }
    }
    println!("OBJ8 audit: {objects} objects, {triangles} declared triangles");
    println!("Missing declared base textures: {}", missing.len());
    let complete = missing.is_empty() && errors.is_empty();
    for item in missing {
        println!("  MISSING: {item}");
    }
    println!("Unimplemented commands (occurrences):");
    for (key, count) in unsupported {
        println!("  {key}: {count}");
    }
    println!("Objects with read/parse errors: {}", errors.len());
    for error in errors {
        println!("  ERROR: {error}");
    }
    Ok(complete)
}

pub fn load(aircraft_path: &Path) -> Result<Scene, Box<dyn std::error::Error>> {
    let acf = Aircraft::parse(&fs::read_to_string(aircraft_path)?)?;
    let dir = aircraft_path.parent().ok_or("ACF needs parent directory")?;
    // Static preview policy: the union of the normal surface/transparent passes,
    // mode4 and zero animation/kill datarefs. This uses the recovered preliminary
    // gate; it is not the original complete render pipeline.
    let mut attachments = acf
        .properties
        .iter()
        .filter_map(|p| {
            let index = p
                .key
                .strip_prefix("_obja/")?
                .strip_suffix("/_v10_att_file_stl")?
                .parse::<u32>()
                .ok()?;
            (!p.value.is_empty()).then_some((index, p))
        })
        .collect::<Vec<_>>();
    attachments.sort_by_key(|(index, _)| *index);
    let flags_for = |p: &openxplane::Property| -> Result<u32, Box<dyn std::error::Error>> {
        acf.unique_property(&p.key)?;
        let prefix = p.key.trim_end_matches("_v10_att_file_stl");
        let value = acf
            .unique_property(&format!("{prefix}_obj_flags"))?
            .value
            .parse::<i32>()?;
        Ok(value as u32)
    };
    let mut selected_cockpit_loaded = false;
    for (_, p) in &attachments {
        if flags_for(p)? & 0x1800 != 0 {
            selected_cockpit_loaded = dir
                .join("objects")
                .join(p.value.replace('\\', "/"))
                .is_file();
            break;
        }
    }
    let mut meshes = Vec::new();
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    for (_, p) in attachments {
        let name = p.value.as_str();
        let flags = flags_for(p)?;
        let context = openxplane::attachment::DrawContext {
            pass: 0x19,
            mode: 4,
            loaded: true,
            selected_cockpit_loaded,
            cockpit_enabled: true,
            main_view_enabled: true,
        };
        if !openxplane::attachment::preliminary_draw_gate(flags, &context) {
            println!("Skipped ACF object {name}: preliminary draw gate, flags={flags:#x}");
            continue;
        }
        let prefix = p.key.trim_end_matches("_v10_att_file_stl");
        // Vertices stay in the ACF frame here. FlightModel::to_world subtracts
        // the CG later, so use a zero CG for this loader-stage translation.
        let attachment = openxplane::attachment::Attachment::from_explicit(&acf, prefix, [0.0; 2])?;
        if attachment.targets != [-1; 3] {
            return Err(format!(
                "Parent-part attachment transform not yet supported: {}",
                p.key
            )
            .into());
        }
        if attachment.angles != [0.0; 3] {
            return Err(format!("Attachment rotation not yet supported: {}", p.key).into());
        }
        let path = dir.join("objects").join(name.replace('\\', "/"));
        let object = Object::parse(&fs::read_to_string(&path)?)?;
        let mut vertices = object.baked_vertices();
        for vertex in &mut vertices {
            vertex.position = (Vec3::from_array(vertex.position)
                + Vec3::from_array(attachment.position))
            .to_array();
        }
        if vertices.is_empty() {
            println!(
                "Skipped ACF object {name}: no triangles at zero datarefs (lights and other commands may remain unsupported)"
            );
            continue;
        }
        let texture = match object.texture.as_ref() {
            Some(texture_name) => {
                let texture_path = texture_path(&path, texture_name)
                    .ok_or_else(|| format!("Missing texture for {name}: {texture_name}"))?;
                image::imageops::flip_vertical(&image::open(&texture_path)?.to_rgba8())
            }
            None => {
                println!("ACF object {name}: no declared base texture; preview uses white");
                solid_texture([255, 255, 255, 255])
            }
        };
        let mut mesh_low = Vec3::splat(f32::INFINITY);
        let mut mesh_high = Vec3::splat(f32::NEG_INFINITY);
        for vertex in &vertices {
            let v = Vec3::from_array(vertex.position);
            mesh_low = mesh_low.min(v);
            mesh_high = mesh_high.max(v);
            low = low.min(v);
            high = high.max(v);
        }
        println!(
            "Loaded {name}: {} triangles, {}x{} texture, {} unsupported command types",
            vertices.len() / 3,
            texture.width(),
            texture.height(),
            object.unsupported.len()
        );
        meshes.push(Mesh {
            vertices,
            texture,
            glass: object.glass || flags & 2 != 0,
            center: (mesh_low + mesh_high) * 0.5,
        });
    }
    let center = (low + high) * 0.5;
    let radius = (high - low).length() * 0.5;
    if !radius.is_finite() || radius <= 0.0 {
        return Err("Invalid scene bounds".into());
    }
    println!(
        "Static ACF preview: preliminary object gate, mode4/pass0x19; all animation/kill datarefs = 0; final visibility and materials incomplete. Bounds: {low:?} .. {high:?}"
    );
    Ok(Scene {
        meshes,
        center,
        radius,
        background: PREVIEW_BACKGROUND,
    })
}

fn solid_texture(color: [u8; 4]) -> image::RgbaImage {
    image::RgbaImage::from_pixel(2, 2, image::Rgba(color))
}

/// The aircraft's vertices in the body frame, kept so the moving aircraft can be re-posed each frame.
pub struct AircraftBody {
    /// Index of the first aircraft mesh in `Scene::meshes` (the ground layers come first).
    pub first_mesh: usize,
    base: Vec<Vec<Vertex>>,
    base_center: Vec<Vec3>,
}

impl AircraftBody {
    /// Poses the aircraft meshes of `scene` for the model's current state and centres the scene
    /// (and so the camera target) on the aircraft.
    pub fn apply(&self, scene: &mut Scene, model: &openxplane::flight::FlightModel) {
        let rot = glam::Mat3::from_quat(model.state.orientation);
        for ((mesh, base), center) in scene.meshes[self.first_mesh..]
            .iter_mut()
            .zip(&self.base)
            .zip(&self.base_center)
        {
            for (v, b) in mesh.vertices.iter_mut().zip(base) {
                v.position = model.to_world(Vec3::from_array(b.position)).to_array();
                v.normal = (rot * Vec3::from_array(b.normal)).to_array();
            }
            mesh.center = model.to_world(*center);
        }
        scene.center = model.state.position;
    }
}

/// An airport scene with the aircraft, its body vertices and a flight model resting on the first runway.
pub struct FlightSession {
    pub scene: Scene,
    pub body: AircraftBody,
    pub model: openxplane::flight::FlightModel,
}

/// The aircraft placed on the first runway of an airport found in an installation, with the airport's
/// ground, pavements, runways and markings. The aircraft stands 150 m from the first runway end, nose
/// along the runway, wheels on the runway surface, with an approximate flight model ready to fly.
pub fn airport_flight(
    root: &Path,
    icao: &str,
    aircraft_path: &Path,
) -> Result<FlightSession, Box<dyn std::error::Error>> {
    let install = openxplane::install::Install::open(root)?;
    let (airport, source) = install
        .find_airport(icao)?
        .ok_or_else(|| format!("airport {icao} not found"))?;
    let (frame, layers) = openxplane::world::airport_layers(&airport, 8000.0)?;
    let runway = airport
        .runways
        .first()
        .ok_or("airport has no land runway")?;
    let [a, b] = openxplane::world::runway_ends(runway, &frame);
    let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dz * dz).sqrt();
    if len < 200.0 {
        return Err("first runway is shorter than 200 m".into());
    }
    let (ux, uz) = (dx / len, dz / len);
    println!(
        "Airport {} {} from {}: runway {}/{}, {:.0} m, {} pavements",
        airport.id,
        airport.name,
        source.display(),
        runway.ends[0].name,
        runway.ends[1].name,
        len,
        airport.pavements.len()
    );

    let aircraft = load(aircraft_path)?;
    let low_y = aircraft
        .meshes
        .iter()
        .flat_map(|m| m.vertices.iter().map(|v| v.position[1]))
        .fold(f32::INFINITY, f32::min);
    let mut model = openxplane::flight::FlightModel::load(aircraft_path, &install, 80.0, 90.0)?;
    model.elevation_m = airport.elevation_ft * 0.3048;
    let direction = Vec3::new(ux, 0.0, uz);
    model.rest_on_ground(low_y, [a[0] + ux * 150.0, a[1] + uz * 150.0], direction);

    let mut meshes = Vec::new();
    for layer in layers {
        let vertices = layer
            .triangles
            .iter()
            .map(|p| Vertex {
                position: *p,
                normal: [0.0, 1.0, 0.0],
                uv: [0.5, 0.5],
            })
            .collect::<Vec<_>>();
        if vertices.is_empty() {
            continue;
        }
        let mut low = Vec3::splat(f32::INFINITY);
        let mut high = Vec3::splat(f32::NEG_INFINITY);
        for v in &vertices {
            low = low.min(Vec3::from_array(v.position));
            high = high.max(Vec3::from_array(v.position));
        }
        meshes.push(Mesh {
            vertices,
            texture: solid_texture(layer.color),
            glass: false,
            center: (low + high) * 0.5,
        });
    }
    let first_mesh = meshes.len();
    let base: Vec<Vec<Vertex>> = aircraft.meshes.iter().map(|m| m.vertices.clone()).collect();
    let base_center: Vec<Vec3> = aircraft.meshes.iter().map(|m| m.center).collect();
    meshes.extend(aircraft.meshes);
    let mut session = FlightSession {
        scene: Scene {
            meshes,
            center: Vec3::ZERO,
            radius: 30.0,
            background: [0.30, 0.52, 0.80],
        },
        body: AircraftBody {
            first_mesh,
            base,
            base_center,
        },
        model,
    };
    session.body.apply(&mut session.scene, &session.model);
    Ok(session)
}

/// The static airport scene (aircraft at rest on the first runway).
pub fn airport(
    root: &Path,
    icao: &str,
    aircraft_path: &Path,
) -> Result<Scene, Box<dyn std::error::Error>> {
    Ok(airport_flight(root, icao, aircraft_path)?.scene)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_arbitrary_acf_objects_with_offsets_and_skips_gated_missing_files() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "openxplane-attachment-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&dir).unwrap();
        fs::create_dir(dir.join("objects")).unwrap();
        fs::write(dir.join("objects/custom-shape.obj"), "A\n800\nOBJ\nPOINT_COUNTS 3 0 0 3\nVT 0 0 0 0 1 0 0 0\nVT 1 0 0 0 1 0 1 0\nVT 0 0 1 0 1 0 0 1\nIDX 0\nIDX 1\nIDX 2\nTRIS 0 3\n").unwrap();
        let mut props = String::from(
            "P _obja/2/_v10_att_file_stl missing-broken.obj\nP _obja/2/_obj_flags 512\n",
        );
        for (field, value) in [
            ("_v10_att_file_stl", "custom-shape.obj"),
            ("_obj_flags", "0"),
            ("_v10_steers_with_gear", "0"),
            ("_v10_is_internal", "0"),
            ("_v10_att_wing", "-1"),
            ("_v10_att_body", "-1"),
            ("_v10_att_gear", "-1"),
            ("_v10_att_x_acf_prt_ref", "2"),
            ("_v10_att_y_acf_prt_ref", "-3"),
            ("_v10_att_z_acf_prt_ref", "4"),
            ("_v10_att_psi_ref", "0"),
            ("_v10_att_the_ref", "0"),
            ("_v10_att_phi_ref", "0"),
        ] {
            props.push_str(&format!("P _obja/7/{field} {value}\n"));
        }
        let acf = dir.join("synthetic.acf");
        fs::write(
            &acf,
            format!("I\n1200 Version\nACF\nPROPERTIES_BEGIN\n{props}PROPERTIES_END\n"),
        )
        .unwrap();
        let scene = load(&acf).unwrap();
        assert_eq!(scene.meshes.len(), 1);
        assert_eq!(scene.meshes[0].vertices.len(), 3);
        let scale = openxplane::aircraft::ACF_FT_TO_M;
        assert_eq!(
            scene.meshes[0].vertices[0].position,
            [2.0 * scale, -3.0 * scale, 4.0 * scale]
        );
        assert_eq!(scene.meshes[0].vertices[1].position[0], 1.0 + 2.0 * scale);
        fs::remove_dir_all(dir).unwrap();
    }
}
