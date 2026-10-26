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
    /// World translation and Y-axis angle in radians; identity for baked geometry.
    pub instances: Vec<[f32; 4]>,
    pub glass: bool,
    pub center: Vec3,
    animation: Option<AnimatedObject>,
}

struct AnimatedObject {
    object: Object,
    rotation: glam::Mat3,
    translation: Vec3,
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
    load_aircraft(aircraft_path, false)
}

fn load_aircraft(aircraft_path: &Path, animate: bool) -> Result<Scene, Box<dyn std::error::Error>> {
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
        let m = openxplane::attachment::affine_matrix(attachment.position, attachment.angles);
        let rotation = glam::Mat3::from_cols_array(&[
            m[0][0], m[1][0], m[2][0], m[0][1], m[1][1], m[2][1], m[0][2], m[1][2], m[2][2],
        ]);
        let path = dir.join("objects").join(name.replace('\\', "/"));
        let object = Object::parse(&fs::read_to_string(&path)?)?;
        let mut vertices = if animate {
            object.animated_vertices(|_| None)?
        } else {
            object.baked_vertices()
        };
        for vertex in &mut vertices {
            vertex.position = (rotation * Vec3::from_array(vertex.position)
                + Vec3::from_array(attachment.position))
            .to_array();
            vertex.normal = (rotation * Vec3::from_array(vertex.normal)).to_array();
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
            instances: vec![[0.0; 4]],
            animation: animate.then_some(AnimatedObject {
                object,
                rotation,
                translation: Vec3::from_array(attachment.position),
            }),
        });
    }
    let center = (low + high) * 0.5;
    let radius = (high - low).length() * 0.5;
    if !radius.is_finite() || radius <= 0.0 {
        return Err("Invalid scene bounds".into());
    }
    println!(
        "ACF preview: preliminary object gate, mode4/pass0x19; initial animation/kill datarefs = 0; runtime animation cache={animate}; final visibility and materials incomplete. Bounds: {low:?} .. {high:?}"
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
    animations: Vec<Option<AnimatedObject>>,
}

impl AircraftBody {
    /// Poses the aircraft meshes of `scene` for the model's current state and centres the scene
    /// (and so the camera target) on the aircraft.
    pub fn apply(
        &self,
        scene: &mut Scene,
        model: &openxplane::flight::FlightModel,
        controls: &openxplane::flight::Controls,
    ) -> Result<(), String> {
        let rot = glam::Mat3::from_quat(model.state.orientation);
        for (index, mesh) in scene.meshes[self.first_mesh..].iter_mut().enumerate() {
            if let Some(source) = &self.animations[index] {
                let vertices = source
                    .object
                    .animated_vertices(|name| model.animation_dataref(name, controls))?;
                if vertices.len() != mesh.vertices.len() {
                    return Err("animated vertex capacity changed".into());
                }
                let mut low = Vec3::splat(f32::INFINITY);
                let mut high = Vec3::splat(f32::NEG_INFINITY);
                for (v, b) in mesh.vertices.iter_mut().zip(vertices) {
                    let point = source.rotation * Vec3::from_array(b.position) + source.translation;
                    v.position = model.to_world(point).to_array();
                    v.normal = (rot * source.rotation * Vec3::from_array(b.normal)).to_array();
                    low = low.min(Vec3::from_array(v.position));
                    high = high.max(Vec3::from_array(v.position));
                }
                mesh.center = (low + high) * 0.5;
            } else {
                for (v, b) in mesh.vertices.iter_mut().zip(&self.base[index]) {
                    v.position = model.to_world(Vec3::from_array(b.position)).to_array();
                    v.normal = (rot * Vec3::from_array(b.normal)).to_array();
                }
                mesh.center = model.to_world(self.base_center[index]);
            }
        }
        scene.center = model.state.position;
        Ok(())
    }
}

fn airport_objects(
    install: &openxplane::install::Install,
    frame: &openxplane::world::Frame,
) -> Result<Vec<Mesh>, String> {
    let origin = frame.origin();
    let lat = origin[0].floor() as i32;
    let lon = origin[1].floor() as i32;
    let relative = format!(
        "Earth nav data/{:+03}{:+04}/{lat:+03}{lon:+04}.dsf",
        lat.div_euclid(10) * 10,
        lon.div_euclid(10) * 10
    );
    let pack = install.global_airports_dir();
    let path = pack.join(relative);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let placements =
        openxplane::dsf::object_placements(&fs::read(&path).map_err(|e| e.to_string())?)?;
    let library = install.preview_library_at(Some(origin))?;
    let mut grouped = BTreeMap::<PathBuf, Vec<(Vec3, f32)>>::new();
    let mut unresolved = 0;
    for p in &placements {
        let [x, z] = frame.to_local(p.lat, p.lon);
        if x.abs() > 6000.0 || z.abs() > 6000.0 {
            continue;
        }
        let direct = pack.join(&p.resource);
        let resource = if direct.is_file() {
            Some(direct)
        } else {
            library.get(&p.resource).cloned()
        };
        if let Some(resource) = resource {
            grouped
                .entry(resource)
                .or_default()
                .push((Vec3::new(x, 0.0, z), p.heading));
        } else {
            unresolved += 1;
        }
    }
    let mut meshes = Vec::new();
    let mut instances = 0;
    let mut failed = 0;
    let mut expanded = BTreeMap::<PathBuf, Vec<(Vec3, f32)>>::new();
    let mut agp_count = 0;
    for (path, poses) in grouped {
        if path.extension().and_then(|s| s.to_str()) != Some("agp") {
            expanded.entry(path).or_default().extend(poses);
            continue;
        }
        let point = fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| openxplane::agp::parse_preview(&s));
        match point {
            Ok(point) => {
                agp_count += poses.len();
                eprintln!(
                    "AGP preview {}: first of {} tiles; {} facade/vegetation annotations omitted; flat ground, full density, no base tile",
                    path.display(),
                    point.tiles,
                    point.omitted_features
                );
                for child in point.objects {
                    let direct = path.parent().unwrap().join(&child.resource);
                    let resource = if direct.is_file() {
                        Some(direct)
                    } else {
                        library.get(&child.resource).cloned()
                    };
                    let Some(resource) = resource else {
                        eprintln!(
                            "AGP resource unresolved in {}: {}",
                            path.display(),
                            child.resource
                        );
                        unresolved += poses.len();
                        continue;
                    };
                    for (position, heading) in &poses {
                        // Tile rotation is CCW; object heading offsets are CW.
                        let rotation =
                            glam::Mat3::from_rotation_y((point.rotation - heading).to_radians());
                        expanded.entry(resource.clone()).or_default().push((
                            *position + rotation * Vec3::from_array(child.offset),
                            heading - point.rotation + child.heading,
                        ));
                    }
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("Scenery AGP skipped {}: {e}", path.display());
            }
        }
    }
    println!(
        "AGP preview: {agp_count} grouped placements expanded; tile surfaces/facades/vegetation not drawn."
    );
    for (path, poses) in expanded {
        let loaded = (|| -> Result<Mesh, Box<dyn std::error::Error>> {
            let object = Object::parse(&fs::read_to_string(&path)?)?;
            let base = object.baked_vertices();
            let texture = if let Some(name) = &object.texture {
                let texture = texture_path(&path, name).ok_or("missing scenery texture")?;
                image::imageops::flip_vertical(&image::open(texture)?.to_rgba8())
            } else {
                solid_texture([255; 4])
            };
            let mut low = Vec3::splat(f32::INFINITY);
            let mut high = Vec3::splat(f32::NEG_INFINITY);
            let mut base_low = Vec3::splat(f32::INFINITY);
            let mut base_high = Vec3::splat(f32::NEG_INFINITY);
            for v in &base {
                base_low = base_low.min(Vec3::from_array(v.position));
                base_high = base_high.max(Vec3::from_array(v.position));
            }
            let mut instances = Vec::with_capacity(poses.len());
            for (position, heading) in &poses {
                let angle = -heading.to_radians();
                instances.push([position.x, position.y, position.z, angle]);
                let rotation = glam::Mat3::from_rotation_y(angle);
                for x in [base_low.x, base_high.x] {
                    for y in [base_low.y, base_high.y] {
                        for z in [base_low.z, base_high.z] {
                            let p = rotation * Vec3::new(x, y, z) + *position;
                            low = low.min(p);
                            high = high.max(p);
                        }
                    }
                }
            }
            Ok(Mesh {
                vertices: base,
                texture,
                glass: object.glass,
                center: (low + high) * 0.5,
                instances,
                animation: None,
            })
        })();
        match loaded {
            Ok(mesh) if !mesh.vertices.is_empty() => {
                instances += poses.len();
                meshes.push(mesh);
            }
            Ok(_) => {}
            Err(e) => {
                failed += 1;
                eprintln!("Scenery object skipped {}: {e}", path.display());
            }
        }
    }
    println!(
        "DSF objects: {} tile placements; {instances} nearby instances, {} resource batches; {unresolved} unresolved placements; {failed} failed resources. Preview libraries/flat placement; no facade/network support.",
        placements.len(),
        meshes.len()
    );
    Ok(meshes)
}

/// A raster-based terrain preview. The original DSF triangle topology, terrain
/// textures and physical ground are not substituted by this visual approximation.
fn xp_scenery_raster(
    path: &Path,
    frame: &openxplane::world::Frame,
    elevation: f32,
) -> Result<Mesh, String> {
    let raster = openxplane::dsf::HeightRaster::read(path)?;
    let origin = frame.origin();
    let step = 100.0_f64;
    let half: i32 = 150;
    let point = |x: i32, z: i32| {
        let lat = origin[0] - (z as f64 * step / 6_371_000.0).to_degrees();
        let lon = origin[1]
            + (x as f64 * step / (6_371_000.0 * origin[0].to_radians().cos())).to_degrees();
        Vec3::new(
            (x as f64 * step) as f32,
            raster.sample(lat, lon) - elevation - 0.3,
            (z as f64 * step) as f32,
        )
    };
    let mut vertices = Vec::new();
    for z in -half..half {
        for x in -half..half {
            // Keep the existing flat airport slab until airport draping/collision is implemented.
            if x.abs() < 80 && z.abs() < 80 {
                continue;
            }
            let a = point(x, z);
            let b = point(x + 1, z);
            let c = point(x + 1, z + 1);
            let d = point(x, z + 1);
            for triangle in [[a, c, b], [a, d, c]] {
                let normal = (triangle[1] - triangle[0])
                    .cross(triangle[2] - triangle[0])
                    .normalize_or_zero();
                for p in triangle {
                    vertices.push(Vertex {
                        position: p.to_array(),
                        normal: normal.to_array(),
                        uv: [0.5, 0.5],
                    });
                }
            }
        }
    }
    Ok(Mesh {
        vertices,
        texture: solid_texture([85, 105, 65, 255]),
        glass: false,
        center: Vec3::ZERO,
        instances: vec![[0.0; 4]],
        animation: None,
    })
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

    let mut aircraft = load_aircraft(aircraft_path, true)?;
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
    let origin = frame.origin();
    if let Some(path) = install.terrain_tile(origin[0], origin[1]) {
        match xp_scenery_raster(&path, &frame, model.elevation_m) {
            Ok(mesh) => {
                println!(
                    "Loaded DSF elevation terrain: {} ({} triangles; raster preview, no landclass textures)",
                    path.display(),
                    mesh.vertices.len() / 3
                );
                meshes.push(mesh);
            }
            Err(error) => eprintln!("DSF terrain unavailable: {}: {error}", path.display()),
        }
    }
    match airport_objects(&install, &frame) {
        Ok(objects) => meshes.extend(objects),
        Err(e) => eprintln!("DSF objects unavailable: {e}"),
    }
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
            instances: vec![[0.0; 4]],
            animation: None,
        });
    }
    let first_mesh = meshes.len();
    let base: Vec<Vec<Vertex>> = aircraft.meshes.iter().map(|m| m.vertices.clone()).collect();
    let base_center: Vec<Vec3> = aircraft.meshes.iter().map(|m| m.center).collect();
    let animations = aircraft
        .meshes
        .iter_mut()
        .map(|m| m.animation.take())
        .collect();
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
            animations,
        },
        model,
    };
    session.body.apply(
        &mut session.scene,
        &session.model,
        &openxplane::flight::Controls::default(),
    )?;
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
        let rotated = props.replace("_v10_att_phi_ref 0", "_v10_att_phi_ref 90");
        fs::write(
            &acf,
            format!("I\n1200 Version\nACF\nPROPERTIES_BEGIN\n{rotated}PROPERTIES_END\n"),
        )
        .unwrap();
        let rotated_scene = load(&acf).unwrap();
        let vertices = &rotated_scene.meshes[0].vertices;
        let expected = Vec3::new(2.0 * scale, -3.0 * scale - 1.0, 4.0 * scale);
        assert!((Vec3::from_array(vertices[1].position) - expected).length() < 1e-6);
        assert!((Vec3::from_array(vertices[1].normal) - Vec3::X).length() < 1e-6);
        fs::remove_dir_all(dir).unwrap();
    }
}
