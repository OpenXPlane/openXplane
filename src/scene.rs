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
    // A deliberate exterior subset; broken variants and full cockpit devices need runtime state. The
    // landing gear object has an alternative in the seaplane variant (floats).
    let wanted: [&[&str]; 7] = [
        &["fuselage.obj"],
        &["fuselage_gear.obj", "fuselage_floats.obj"],
        &["fuselage_prop.obj"],
        &["wings.obj"],
        &["cockpit_seats_front.obj"],
        &["cockpit_seats_rear.obj"],
        &["glass_out.obj"],
    ];
    let mut names = Vec::new();
    for alternatives in wanted {
        let found = alternatives.iter().find(|n| {
            acf.properties
                .iter()
                .any(|p| p.key.ends_with("/_v10_att_file_stl") && p.value == **n)
        });
        names.push(*found.ok_or_else(|| {
            format!(
                "Preview subset requires one of the ACF objects: {}",
                alternatives.join(", ")
            )
        })?);
    }
    let mut meshes = Vec::new();
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    for name in names {
        let p = acf
            .properties
            .iter()
            .find(|p| p.key.ends_with("/_v10_att_file_stl") && p.value == name)
            .ok_or_else(|| format!("Preview subset requires ACF object: {name}"))?;
        let prefix = p.key.trim_end_matches("_v10_att_file_stl");
        for field in [
            "_v10_att_x_acf_prt_ref",
            "_v10_att_y_acf_prt_ref",
            "_v10_att_z_acf_prt_ref",
            "_v10_att_phi_ref",
            "_v10_att_psi_ref",
            "_v10_att_the_ref",
        ] {
            let key = format!("{prefix}{field}");
            if let Some(p) = acf.properties.iter().find(|p| p.key == key)
                && p.value.parse::<f32>()? != 0.0
            {
                return Err(
                    format!("Non-zero attachment pose not yet supported: {}", p.key).into(),
                );
            }
        }
        let path = dir.join("objects").join(name);
        let object = Object::parse(&fs::read_to_string(&path)?)?;
        let texture_name = object
            .texture
            .as_ref()
            .ok_or("Preview object has no base texture")?;
        let texture_path = texture_path(&path, texture_name)
            .ok_or_else(|| format!("Missing texture for {name}: {texture_name}"))?;
        let texture = image::open(&texture_path)?.to_rgba8();
        let texture = image::imageops::flip_vertical(&texture);
        let vertices = object.baked_vertices();
        if vertices.is_empty() {
            return Err(format!("No visible preview geometry: {name}").into());
        }
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
            glass: object.glass || name.starts_with("glass"),
            center: (mesh_low + mesh_high) * 0.5,
        });
    }
    let center = (low + high) * 0.5;
    let radius = (high - low).length() * 0.5;
    if !radius.is_finite() || radius <= 0.0 {
        return Err("Invalid scene bounds".into());
    }
    println!(
        "Static exterior preview: all animation datarefs = 0; no flight simulation. Bounds: {low:?} .. {high:?}"
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
