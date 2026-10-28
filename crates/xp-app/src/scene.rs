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

pub struct BorderMaterial {
    pub texture: image::RgbaImage,
    pub uv: Vec<[f32; 2]>,
    pub repeat: bool,
    pub composite: bool,
    pub no_alpha: bool,
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub border: Option<BorderMaterial>,
    pub texture: image::RgbaImage,
    /// World translation and Y-axis angle in radians; identity for baked geometry.
    pub instances: Vec<[f32; 4]>,
    pub glass: bool,
    pub repeat_texture: bool,
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

/// Source-topology inspection before terrain material/physics integration.
pub fn terrain_mesh(root: &Path, icao: &str) -> Result<Scene, Box<dyn std::error::Error>> {
    Ok(terrain_scene(root, icao)?.0)
}

fn terrain_scene(
    root: &Path,
    icao: &str,
) -> Result<(Scene, openxplane::ground::Ground), Box<dyn std::error::Error>> {
    let install = openxplane::install::Install::open(root)?;
    let (airport, _) = install.find_airport(icao)?.ok_or("airport not found")?;
    let (frame, _) = openxplane::world::airport_layers(&airport, 8000.0)?;
    let origin = frame.origin();
    let path = install
        .terrain_tile(origin[0], origin[1])
        .ok_or("base DSF tile not found")?;
    let bytes = openxplane::dsf::read_bytes(&path)?;
    let raster = match openxplane::dsf::HeightRaster::parse(&bytes) {
        Ok(raster) => raster,
        Err(error) => {
            let global = install
                .global_terrain_tile(origin[0], origin[1])
                .ok_or(error)?;
            eprintln!("Using global elevation raster for {}", path.display());
            openxplane::dsf::HeightRaster::read(&global)?
        }
    };
    let features = openxplane::dsf::features_with_terrain(
        &bytes,
        Some([
            origin[1] - 0.15,
            origin[0] - 0.1,
            origin[1] + 0.15,
            origin[0] + 0.1,
        ]),
    )?;
    let season = std::env::var("OPENXPLANE_SCENERY_SEASON").unwrap_or_else(|_| "sum".into());
    println!(
        "Terrain library season={season}; explicit preview policy, not simulator calendar/climate selection"
    );
    let library = install.preview_library_for(Some(origin), Some(&season))?;
    let mut materials = BTreeMap::new();
    for triangle in &features.terrain {
        let name = &triangle.resource;
        if name == "terrain_Water" || materials.contains_key(name) {
            continue;
        }
        let parsed = (|| -> Result<_, Box<dyn std::error::Error>> {
            let package = path.ancestors().nth(3).ok_or("DSF package path missing")?;
            let local = package.join(name.replace('\\', "/"));
            let resource = if local.is_file() {
                &local
            } else {
                library.get(name).ok_or("terrain library path unresolved")?
            };
            let material =
                openxplane::scenery_terrain::Material::parse(&fs::read_to_string(resource)?)?;
            let texture = image::imageops::flip_vertical(
                &open_texture(
                    terrain_texture_path(resource, &material.base).ok_or("terrain base texture missing")?,
                )?
                .to_rgba8(),
            );
            println!(
                "TER {name}: projected={:?}, unsupported={:?}",
                material.projected, material.unsupported
            );
            let border = material
                .border
                .as_ref()
                .and_then(|name| terrain_texture_path(resource, name))
                .and_then(|path| open_texture(path).ok())
                .map(|image| image::imageops::flip_vertical(&image.to_rgba8()));
            if material.border.is_some() && border.is_none() {
                eprintln!(
                    "TER border unavailable {name}: {:?}; preview retains unmasked base",
                    material.border
                );
            }
            Ok((material, texture, border))
        })();
        match parsed {
            Ok(m) => {
                materials.insert(name.clone(), Some(m));
            }
            Err(e) => {
                eprintln!("TER material fallback {name}: {e}");
                materials.insert(name.clone(), None);
            }
        }
    }
    let mut grouped = BTreeMap::<(bool, String), Vec<Vertex>>::new();
    let mut border_failures = std::collections::BTreeSet::new();
    let mut border_uvs = BTreeMap::<(bool, String), Vec<[f32; 2]>>::new();
    let mut uv_failures = BTreeMap::<String, String>::new();
    let mut ground_triangles = Vec::new();
    let mut physical = 0;
    let mut overlay = 0;
    for triangle in &features.terrain {
        if triangle.flags & 1 != 0 {
            physical += 1;
        }
        if triangle.flags & 2 != 0 {
            overlay += 1;
        }
        let water = triangle.resource == "terrain_Water";
        let positions = triangle.points.each_ref().map(|p| {
            let [x, z] = frame.to_local(p[1], p[0]);
            let height = if p[2] == -32768.0 || water {
                raster.sample(p[1], p[0])
            } else {
                p[2] as f32
            };
            Vec3::new(x, height - airport.elevation_ft * 0.3048, z)
        });
        if triangle.flags & 1 != 0 {
            ground_triangles.push(positions.map(|p| p.to_array()));
        }
        let face_normal = (positions[1] - positions[0])
            .cross(positions[2] - positions[0])
            .normalize_or_zero();
        for (p, position) in triangle.points.iter().zip(positions) {
            let normal = if p[3] == 0.0 && p[4] == 0.0 {
                face_normal
            } else {
                Vec3::new(
                    p[3] as f32,
                    (1.0 - p[3] * p[3] - p[4] * p[4]).max(0.0).sqrt() as f32,
                    p[4] as f32,
                )
            };
            let border_uv = materials
                .get(&triangle.resource)
                .and_then(Option::as_ref)
                .and_then(|(m, _, _)| match m.border_uv(p) {
                    Ok(uv) => Some(uv),
                    Err(_) => {
                        if triangle.flags & 2 != 0 && m.border.is_some() {
                            border_failures.insert(triangle.resource.clone());
                        }
                        None
                    }
                })
                .unwrap_or([1.0, 0.0]);
            border_uvs
                .entry((triangle.flags & 2 != 0, triangle.resource.clone()))
                .or_default()
                .push(border_uv);
            grouped
                .entry((triangle.flags & 2 != 0, triangle.resource.clone()))
                .or_default()
                .push(Vertex {
                    position: (position
                        + if triangle.flags & 2 != 0 {
                            Vec3::Y * 0.005
                        } else {
                            Vec3::ZERO
                        })
                    .to_array(),
                    normal: normal.to_array(),
                    uv: materials
                        .get(&triangle.resource)
                        .and_then(Option::as_ref)
                        .and_then(|(m, _, _)| match m.uv([position.x, position.z], p) {
                            Ok(uv) => Some(uv),
                            Err(e) => {
                                uv_failures.insert(triangle.resource.clone(), e);
                                None
                            }
                        })
                        .unwrap_or([0.5, 0.5]),
                });
        }
    }
    println!(
        "DSF source mesh {}: {} intersecting triangles, {physical} physical, {overlay} overlay, {} terrain definitions; source topology with elevation raster, base-material preview, all LODs, no collision integration",
        path.display(),
        features.terrain.len(),
        grouped
            .keys()
            .map(|(_, name)| name)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    let mut textured = 0;
    let mut masked = 0;
    let meshes = grouped
        .into_iter()
        .map(|((overlay, name), vertices)| {
            let mut border = None;
            let mut glass = false;
            let (texture, repeat_texture) =
                if let Some(Some((material, base, mask))) = materials.get(&name) {
                    let mut texture = base.clone();
                    if overlay && let Some(mask) = mask {
                        masked += 1;
                        border = Some(BorderMaterial {
                            texture: mask.clone(),
                            uv: border_uvs.remove(&(overlay, name.clone())).unwrap(),
                            repeat: material.border_repeat,
                            composite: material.composite_borders,
                            no_alpha: material.no_alpha,
                        });
                        glass = !material.composite_borders;
                    } else if material.no_alpha {
                        for pixel in texture.pixels_mut() {
                            pixel.0[3] = 255;
                        }
                    }
                    textured += 1;
                    (texture, material.repeat)
                } else {
                    (
                        solid_texture(if name == "terrain_Water" {
                            [35, 80, 125, 255]
                        } else {
                            [85, 105, 65, 255]
                        }),
                        false,
                    )
                };
            Mesh {
                vertices,
                texture,
                repeat_texture,
                glass,
                center: Vec3::ZERO,
                instances: vec![[0.0; 4]],
                border,
                animation: None,
            }
        })
        .collect();
    println!(
        "TER preview: {textured} textured batches, {masked} masked overlay batches; grayscale borders, original masks/coords with approximate depth ordering; missing border coordinates: {border_failures:?}; no composite color/normal/decal/water shaders; UV failures: {uv_failures:?}"
    );
    Ok((
        Scene {
            meshes,
            center: Vec3::ZERO,
            radius: 30000.0,
            background: [0.30, 0.52, 0.80],
        },
        openxplane::ground::Ground::new(ground_triangles),
    ))
}

/// Inspect one actual type-2 facade segment in its unchanged file coordinates.
pub fn facade_segment(
    path: &Path,
    floor_index: usize,
    segment_index: usize,
) -> Result<Scene, Box<dyn std::error::Error>> {
    let facade = openxplane::facade::parse(&fs::read_to_string(path)?)?;
    let floor = facade
        .floors
        .get(floor_index)
        .ok_or("FAC floor index out of range")?;
    let segment = floor
        .segments
        .iter()
        .filter(|s| !s.curved)
        .nth(segment_index)
        .ok_or("FAC segment index out of range")?;
    let texture = if let Some(name) = &facade.wall_texture {
        let texture = texture_path(path, name).ok_or("missing FAC wall texture")?;
        image::imageops::flip_vertical(&open_texture(texture)?.to_rgba8())
    } else {
        solid_texture([180, 180, 180, 255])
    };
    let mut meshes = Vec::new();
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    let mut ordered = segment.meshes.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|m| m.group);
    for mesh in ordered {
        let vertices = mesh
            .indices
            .iter()
            .map(|&i| {
                let v = mesh.vertices[i];
                let p = Vec3::new(v[0], v[1], v[2]);
                low = low.min(p);
                high = high.max(p);
                Vertex {
                    position: [v[0], v[1], v[2]],
                    normal: [v[3], v[4], v[5]],
                    uv: [v[6], v[7]],
                }
            })
            .collect::<Vec<_>>();
        if !vertices.is_empty() {
            meshes.push(Mesh {
                vertices,
                texture: texture.clone(),
                instances: vec![[0.0; 4]],
                repeat_texture: false,
                glass: false,
                center: Vec3::ZERO,
                border: None,
                animation: None,
            });
        }
    }
    let center = (low + high) * 0.5;
    let radius = (high - low).length() * 0.5;
    if !radius.is_finite() || radius <= 0.0 {
        return Err("empty FAC segment bounds".into());
    }
    println!(
        "FAC floor {floor_index} '{}', segment {segment_index}: {} meshes, bounds {low:?} .. {high:?}; file-space inspection only, all LOD groups, no wall fitting/roof/attachments. Unimplemented directives: {:?}",
        floor.name,
        meshes.len(),
        facade.unsupported
    );
    Ok(Scene {
        meshes,
        center,
        radius,
        background: PREVIEW_BACKGROUND,
    })
}

/// Decodes a texture file chosen by `texture_path`; KTX2 is handled here because
/// the `image` crate does not read it.
pub fn open_texture(path: impl AsRef<Path>) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
    let path = path.as_ref();
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("ktx2"))
    {
        return Ok(image::DynamicImage::ImageRgba8(decode_ktx2(&fs::read(path)?)?));
    }
    Ok(image::open(path)?)
}

/// Reads level 0 of a 2D KTX2 file without supercompression: BC1, BC3 or RGBA8.
fn decode_ktx2(data: &[u8]) -> Result<image::RgbaImage, Box<dyn std::error::Error>> {
    const MAGIC: &[u8; 12] = b"\xabKTX 20\xbb\r\n\x1a\n";
    let u32_at = |o: usize| -> Result<u32, Box<dyn std::error::Error>> {
        Ok(u32::from_le_bytes(data.get(o..o + 4).ok_or("truncated KTX2")?.try_into()?))
    };
    if data.get(..12) != Some(MAGIC) {
        return Err("not a KTX2 file".into());
    }
    let (format, width, height) = (u32_at(12)?, u32_at(20)?, u32_at(24)?);
    if u32_at(44)? != 0 {
        return Err("KTX2 supercompression is unsupported".into());
    }
    if u32_at(32)? > 1 || u32_at(36)? > 1 || u32_at(40)? == 0 {
        return Err("only 2D single-layer KTX2 textures are supported".into());
    }
    let offset = u64::from_le_bytes(data.get(80..88).ok_or("truncated KTX2")?.try_into()?) as usize;
    let length = u64::from_le_bytes(data.get(88..96).ok_or("truncated KTX2")?.try_into()?) as usize;
    let level = data.get(offset..offset.checked_add(length).ok_or("bad KTX2 level")?).ok_or("truncated KTX2 level")?;
    let mut out = image::RgbaImage::new(width, height);
    let rgb565 = |v: u16| {
        let (r, g, b) = ((v >> 11) & 31, (v >> 5) & 63, v & 31);
        [
            ((r * 255 + 15) / 31) as u8,
            ((g * 255 + 31) / 63) as u8,
            ((b * 255 + 15) / 31) as u8,
        ]
    };
    let bc1_colors = |block: &[u8], opaque_only: bool| -> [[u8; 4]; 4] {
        let (c0, c1) = (
            u16::from_le_bytes([block[0], block[1]]),
            u16::from_le_bytes([block[2], block[3]]),
        );
        let (a, b) = (rgb565(c0), rgb565(c1));
        let mix = |x: u16, y: u16, i: usize| ((x * (3 - i as u16) + y * i as u16) / 3) as u8;
        let mut table = [[a[0], a[1], a[2], 255], [b[0], b[1], b[2], 255], [0; 4], [0; 4]];
        if c0 > c1 || opaque_only {
            for i in 0..3 {
                table[2][i] = mix(a[i] as u16, b[i] as u16, 1);
                table[3][i] = mix(a[i] as u16, b[i] as u16, 2);
            }
            table[2][3] = 255;
            table[3][3] = 255;
        } else {
            for i in 0..3 {
                table[2][i] = ((a[i] as u16 + b[i] as u16) / 2) as u8;
            }
            table[2][3] = 255;
        }
        table
    };
    match format {
        37 | 43 => {
            let need = width as usize * height as usize * 4;
            let src = level.get(..need).ok_or("truncated KTX2 level")?;
            out.copy_from_slice(src);
        }
        131 | 132 | 137 | 138 => {
            let bc3 = format >= 137;
            let size = if bc3 { 16 } else { 8 };
            let blocks_x = width.div_ceil(4) as usize;
            for by in 0..height.div_ceil(4) as usize {
                for bx in 0..blocks_x {
                    let at = (by * blocks_x + bx) * size;
                    let block = level.get(at..at + size).ok_or("truncated KTX2 level")?;
                    let (alpha, color) = if bc3 { (Some(&block[..8]), &block[8..]) } else { (None, block) };
                    let table = bc1_colors(color, bc3);
                    let bits = u32::from_le_bytes([color[4], color[5], color[6], color[7]]);
                    let alpha_values = alpha.map(|a| {
                        let (a0, a1) = (a[0] as u16, a[1] as u16);
                        let mut v = [0u8; 8];
                        v[0] = a0 as u8;
                        v[1] = a1 as u8;
                        for i in 2..8u16 {
                            v[i as usize] = if a0 > a1 {
                                (((8 - i) * a0 + (i - 1) * a1) / 7) as u8
                            } else if i < 6 {
                                (((6 - i) * a0 + (i - 1) * a1) / 5) as u8
                            } else if i == 6 {
                                0
                            } else {
                                255
                            };
                        }
                        let mut idx = 0u64;
                        for (i, b) in a[2..8].iter().enumerate() {
                            idx |= (*b as u64) << (8 * i);
                        }
                        (v, idx)
                    });
                    for py in 0..4usize {
                        for px in 0..4usize {
                            let (x, y) = ((bx * 4 + px) as u32, (by * 4 + py) as u32);
                            if x >= width || y >= height {
                                continue;
                            }
                            let n = py * 4 + px;
                            let mut c = table[((bits >> (2 * n)) & 3) as usize];
                            if let Some((v, idx)) = &alpha_values {
                                c[3] = v[((idx >> (3 * n)) & 7) as usize];
                            }
                            out.put_pixel(x, y, image::Rgba(c));
                        }
                    }
                }
            }
        }
        _ => return Err(format!("unsupported KTX2 vkFormat {format}").into()),
    }
    Ok(out)
}

/// Reference resolver arguments (`FUN_1419eb8c0`): mode 1 with EDX == 2 is a
/// terrain texture, any other EDX an object texture.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TextureKind {
    Object,
    Terrain,
}

pub fn texture_path(resource: &Path, name: &str) -> Option<PathBuf> {
    resolve_texture(resource, name, TextureKind::Object)
}

pub fn terrain_texture_path(resource: &Path, name: &str) -> Option<PathBuf> {
    resolve_texture(resource, name, TextureKind::Terrain)
}

fn resolve_texture(resource: &Path, name: &str, kind: TextureKind) -> Option<PathBuf> {
    let name = name.replace('\\', "/");
    // OBJ, TER BASE_TEX/TEXTURE and POL all call the reference resolver with
    // mode 1, which replaces the last four bytes for DDS/PNG/BMP, but
    // removes the last dotted suffix for KTX2. Stop on the first file even
    // if its decoder subsequently fails; a decode error is not a search miss.
    let replacement = |extension| {
        if name.len() > 4 {
            Some(format!("{}{extension}", name.get(..name.len() - 4)?))
        } else {
            Some(name.clone())
        }
    };
    let ktx_stem = name
        .rsplit_once('.')
        .map_or(name.as_str(), |(stem, _)| stem);
    let parent = resource.parent()?;
    let [dds, png, bmp] = [replacement(".dds")?, replacement(".png")?, replacement(".bmp")?];
    let mut candidates = vec![
        parent.join(&dds),
        parent.join(&png),
        parent.join(&bmp),
        parent.join(format!("{ktx_stem}.ktx2")),
    ];
    // After the package-relative candidates the resolver tries the package
    // root's `custom object textures/` (or `custom terrain textures/`) folder
    // with the DDS, PNG and BMP names only.
    if let Some(root) = scenery_pack_root(resource) {
        let folder = match kind {
            TextureKind::Object => "custom object textures",
            TextureKind::Terrain => "custom terrain textures",
        };
        candidates.extend([dds, png, bmp].map(|n| root.join(folder).join(n)));
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

/// `<...>/Custom Scenery/<pack>` or `<...>/Global Scenery/<pack>` for a path inside it.
fn scenery_pack_root(path: &Path) -> Option<PathBuf> {
    let parts: Vec<_> = path.components().collect();
    let at = parts.iter().position(|c| {
        let c = c.as_os_str().to_string_lossy();
        c.eq_ignore_ascii_case("Custom Scenery") || c.eq_ignore_ascii_case("Global Scenery")
    })?;
    (parts.len() > at + 2).then(|| parts[..=at + 1].iter().collect())
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
        let object = match fs::read(&path) {
            Ok(bytes) => match Object::parse_bytes(&bytes) {
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
        let object = Object::parse_bytes(&fs::read(&path)?)?;
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
                image::imageops::flip_vertical(&open_texture(&texture_path)?.to_rgba8())
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
            repeat_texture: false,
            glass: object.glass || flags & 2 != 0,
            center: (mesh_low + mesh_high) * 0.5,
            instances: vec![[0.0; 4]],
            border: None,
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
    pack: &Path,
    bytes: &[u8],
    library: &BTreeMap<String, PathBuf>,
    frame: &openxplane::world::Frame,
    ground: Option<&openxplane::ground::Ground>,
    exclusions: &[openxplane::dsf::ObjectExclusion],
) -> Result<Vec<Mesh>, String> {
    let features = openxplane::dsf::features(bytes)?;
    let placements = features.objects;
    let mut grouped = BTreeMap::<PathBuf, Vec<(Vec3, f32)>>::new();
    let mut unresolved = 0;
    let mut excluded = 0;
    let mut unresolved_names = std::collections::BTreeSet::new();
    for p in &placements {
        if exclusions.iter().any(|zone| zone.contains(p.lon, p.lat)) {
            excluded += 1;
            continue;
        }
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
            unresolved_names.insert(p.resource.clone());
        }
    }
    if std::env::var_os("OPENXPLANE_LOG_UNRESOLVED").is_some() {
        for name in &unresolved_names {
            eprintln!("unresolved object: {name}");
        }
    }
    let mut meshes = airport_facades(&features.polygons, pack, library, frame, ground);
    meshes.extend(airport_polygons(
        &features.polygons,
        pack,
        library,
        frame,
        ground,
    ));
    let facade_batches = meshes.len();
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
                    "AGP preview {}: first of {} tiles; {} facade/vegetation annotations omitted; ground-point alignment approximate, full density, no base tile",
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
            let object = Object::parse_bytes(&fs::read(&path)?)?;
            let base = object.baked_vertices();
            let texture = if let Some(name) = &object.texture {
                let texture = texture_path(&path, name).ok_or("missing scenery texture")?;
                image::imageops::flip_vertical(&open_texture(texture)?.to_rgba8())
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
                let position = *position
                    + Vec3::Y
                        * ground
                            .and_then(|g| g.sample(position.x, position.z))
                            .map_or(0.0, |(y, _)| y);
                instances.push([position.x, position.y, position.z, angle]);
                let rotation = glam::Mat3::from_rotation_y(angle);
                for x in [base_low.x, base_high.x] {
                    for y in [base_low.y, base_high.y] {
                        for z in [base_low.z, base_high.z] {
                            let p = rotation * Vec3::new(x, y, z) + position;
                            low = low.min(p);
                            high = high.max(p);
                        }
                    }
                }
            }
            Ok(Mesh {
                vertices: base,
                texture,
                repeat_texture: false,
                glass: object.glass,
                center: (low + high) * 0.5,
                instances,
                border: None,
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
        "DSF objects: {} tile placements; {instances} nearby instances, {} resource batches; {excluded} excluded, {unresolved} unresolved placements; {failed} failed resources. Preview libraries/terrain height with flat fallback; partial straight FAC walls, no networks.",
        placements.len(),
        meshes.len() - facade_batches
    );
    Ok(meshes)
}

fn airport_polygons(
    polygons: &[openxplane::dsf::PolygonPlacement],
    pack: &Path,
    library: &BTreeMap<String, PathBuf>,
    frame: &openxplane::world::Frame,
    ground: Option<&openxplane::ground::Ground>,
) -> Vec<Mesh> {
    let mut grouped = BTreeMap::<PathBuf, Vec<&openxplane::dsf::PolygonPlacement>>::new();
    let mut skipped = 0;
    let mut placed = 0;
    for polygon in polygons {
        if !polygon.resource.ends_with(".pol") {
            continue;
        }
        let Some(p) = polygon.windings.first().and_then(|w| w.first()) else {
            continue;
        };
        let [x, z] = frame.to_local(p[1], p[0]);
        if x.abs() > 6000.0 || z.abs() > 6000.0 {
            continue;
        }
        let direct = pack.join(&polygon.resource);
        let resource = if direct.is_file() {
            Some(direct)
        } else {
            library.get(&polygon.resource).cloned()
        };
        if let Some(resource) = resource {
            grouped.entry(resource).or_default().push(polygon);
        } else {
            skipped += 1;
        }
    }
    let mut meshes = Vec::new();
    for (path, polygons) in grouped {
        let loaded = (|| -> Result<Mesh, Box<dyn std::error::Error>> {
            let material = openxplane::polygon::Material::parse(&fs::read_to_string(&path)?)?;
            let mut texture = image::imageops::flip_vertical(
                &open_texture(
                    texture_path(&path, &material.texture).ok_or("POL texture missing")?,
                )?
                .to_rgba8(),
            );
            if material.no_alpha {
                for p in texture.pixels_mut() {
                    p.0[3] = 255;
                }
            }
            let mut vertices = Vec::new();
            for polygon in polygons {
                let footprint = match openxplane::polygon::footprint(
                    &polygon.windings,
                    frame,
                    polygon.parameter == 65535,
                ) {
                    Ok(f) => f,
                    Err(e) => {
                        skipped += 1;
                        eprintln!("POL contour skipped {}: {e}", path.display());
                        continue;
                    }
                };
                match material.vertices_with_holes(
                    &footprint.points,
                    &footprint.holes,
                    footprint.st.as_deref(),
                    polygon.parameter,
                    ground,
                ) {
                    Ok(v) => {
                        placed += 1;
                        vertices.extend(v.into_iter().map(|v| Vertex {
                            position: [v[0], v[1], v[2]],
                            normal: [v[3], v[4], v[5]],
                            uv: [v[6], v[7]],
                        }));
                    }
                    Err(e) => {
                        skipped += 1;
                        eprintln!("POL footprint skipped {}: {e}", path.display());
                    }
                }
            }
            println!(
                "POL {}: layer={} offset={}, unsupported={:?}",
                path.display(),
                material.layer,
                material.offset,
                material.unsupported
            );
            Ok(Mesh {
                vertices,
                texture,
                instances: vec![[0.0; 4]],
                glass: !material.no_alpha,
                repeat_texture: material.repeat,
                center: Vec3::ZERO,
                border: None,
                animation: None,
            })
        })();
        match loaded {
            Ok(mesh) if !mesh.vertices.is_empty() => meshes.push(mesh),
            Ok(_) => {}
            Err(e) => {
                skipped += 1;
                eprintln!("POL resource skipped {}: {e}", path.display());
            }
        }
    }
    println!(
        "POL preview: {placed} polygons, {} resource batches, {skipped} skipped footprints/resources; holes/Bezier preview, approximate layer bias/draping, no normal/decal/night/surface physics",
        meshes.len()
    );
    meshes
}

fn airport_facades(
    polygons: &[openxplane::dsf::PolygonPlacement],
    pack: &Path,
    library: &BTreeMap<String, PathBuf>,
    frame: &openxplane::world::Frame,
    ground: Option<&openxplane::ground::Ground>,
) -> Vec<Mesh> {
    let mut grouped = BTreeMap::<PathBuf, Vec<&openxplane::dsf::PolygonPlacement>>::new();
    let mut skipped = 0;
    for polygon in polygons {
        if !polygon.resource.ends_with(".fac") {
            continue;
        }
        let Some(first) = polygon.windings.first().and_then(|w| w.first()) else {
            continue;
        };
        let [x, z] = frame.to_local(first[1], first[0]);
        if x.abs() > 6000.0 || z.abs() > 6000.0 {
            continue;
        }
        let direct = pack.join(&polygon.resource);
        if direct.is_file() {
            grouped.entry(direct).or_default().push(polygon);
        } else if let Some(path) = library.get(&polygon.resource) {
            grouped.entry(path.clone()).or_default().push(polygon);
        } else {
            skipped += 1;
        }
    }
    let mut meshes = Vec::new();
    let mut placed = 0;
    let mut roofed = 0;
    let mut roof_skipped = 0;
    for (path, polygons) in grouped {
        let result = (|| -> Result<Vec<Mesh>, Box<dyn std::error::Error>> {
            let facade = openxplane::facade::parse(&fs::read_to_string(&path)?)?;
            let texture = if let Some(name) = &facade.wall_texture {
                image::imageops::flip_vertical(
                    &open_texture(texture_path(&path, name).ok_or("missing facade wall texture")?)?
                        .to_rgba8(),
                )
            } else {
                solid_texture([180, 180, 180, 255])
            };
            let roof_texture = if let Some(name) = facade
                .roof_texture
                .as_ref()
                .or(facade.wall_texture.as_ref())
            {
                texture_path(&path, name)
                    .and_then(|p| open_texture(p).ok())
                    .map(|im| image::imageops::flip_vertical(&im.to_rgba8()))
            } else {
                Some(solid_texture([180, 180, 180, 255]))
            };
            let mut roof_vertices = Vec::new();
            let mut vertices = Vec::new();
            for polygon in polygons {
                if polygon.windings.len() != 1
                    || polygon.windings[0]
                        .iter()
                        .any(|p| ![2, 3].contains(&p.len()))
                {
                    skipped += 1;
                    continue;
                }
                let winding = &polygon.windings[0];
                let mut points = winding
                    .iter()
                    .map(|p| frame.to_local(p[1], p[0]))
                    .collect::<Vec<_>>();
                let mut choices = winding
                    .iter()
                    .map(|p| p.get(2).copied().unwrap_or(0.0) as usize)
                    .collect::<Vec<_>>();
                if points.len() > 2 && points.first() == points.last() {
                    points.pop();
                    choices.pop();
                }
                let center = points
                    .first()
                    .zip(points.get(1))
                    .map(|(a, b)| [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5])
                    .unwrap_or([0.0; 2]);
                let graded_height = ground
                    .and_then(|g| g.sample(center[0], center[1]))
                    .map_or(0.0, |(y, _)| y);
                let height_at = |x: f32, z: f32| {
                    if facade.graded {
                        graded_height
                    } else {
                        ground.and_then(|g| g.sample(x, z)).map_or(0.0, |(y, _)| y)
                    }
                };
                match facade.straight_walls(&points, &choices, polygon.parameter as f32) {
                    Ok(wall) => {
                        placed += 1;
                        if roof_texture.is_some() {
                            match facade.roofs(&points, polygon.parameter as f32) {
                                Ok(roof) if !roof.is_empty() => {
                                    roofed += 1;
                                    roof_vertices.extend(roof.into_iter().map(|v| Vertex {
                                        position: [v[0], v[1] + height_at(v[0], v[2]), v[2]],
                                        normal: [v[3], v[4], v[5]],
                                        uv: [v[6], v[7]],
                                    }));
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    roof_skipped += 1;
                                    eprintln!("Facade roof skipped {}: {e}", path.display());
                                }
                            }
                        } else {
                            roof_skipped += 1;
                        }

                        vertices.extend(wall.into_iter().map(|v| Vertex {
                            position: [v[0], v[1] + height_at(v[0], v[2]), v[2]],
                            normal: [v[3], v[4], v[5]],
                            uv: [v[6], v[7]],
                        }));
                    }
                    Err(e) => {
                        skipped += 1;
                        eprintln!("Facade footprint skipped {}: {e}", path.display());
                    }
                }
            }
            let walls = Mesh {
                vertices,
                texture,
                instances: vec![[0.0; 4]],
                repeat_texture: false,
                glass: false,
                center: Vec3::ZERO,
                border: None,
                animation: None,
            };
            let mut batches = vec![walls];
            if !roof_vertices.is_empty() {
                batches.push(Mesh {
                    vertices: roof_vertices,
                    texture: roof_texture.unwrap(),
                    instances: vec![[0.0; 4]],
                    repeat_texture: true,
                    glass: false,
                    center: Vec3::ZERO,
                    border: None,
                    animation: None,
                });
            }
            Ok(batches)
        })();
        match result {
            Ok(batches) => meshes.extend(batches.into_iter().filter(|m| !m.vertices.is_empty())),
            Err(e) => {
                skipped += 1;
                eprintln!("Facade resource skipped {}: {e}", path.display());
            }
        }
    }
    println!(
        "FAC wall/roof preview: {placed} footprints, {} batches, {skipped} skipped footprints/resources; deterministic fits, approximate graded/draped height, {roofed} roofed footprints, {roof_skipped} roof failures; no curved walls/miters/attachments.",
        meshes.len()
    );
    meshes
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
        repeat_texture: false,
        glass: false,
        center: Vec3::ZERO,
        instances: vec![[0.0; 4]],
        border: None,
        animation: None,
    })
}

/// Subdivide long airport triangles before projecting vertices onto physical
/// terrain. This bounded visual approximation is not native mesh draping.
fn drape_triangles(input: &[[f32; 3]], ground: &openxplane::ground::Ground) -> Vec<[f32; 3]> {
    let mut pending = input
        .as_chunks::<3>()
        .0
        .iter()
        .copied()
        .map(|t| (t, 0))
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    while let Some((t, depth)) = pending.pop() {
        let distance = |a: [f32; 3], b: [f32; 3]| (a[0] - b[0]).hypot(a[2] - b[2]);
        let edge = (0..3)
            .max_by(|&a, &b| {
                distance(t[a], t[(a + 1) % 3]).total_cmp(&distance(t[b], t[(b + 1) % 3]))
            })
            .unwrap();
        let next = (edge + 1) % 3;
        let other = (edge + 2) % 3;
        if depth < 16 && distance(t[edge], t[next]) > 50.0 {
            let mid = std::array::from_fn(|i| (t[edge][i] + t[next][i]) * 0.5);
            pending.push(([t[edge], mid, t[other]], depth + 1));
            pending.push(([mid, t[next], t[other]], depth + 1));
        } else {
            out.extend(t.map(|p| {
                [
                    p[0],
                    p[1] + 0.21 + ground.sample(p[0], p[2]).map_or(0.0, |(y, _)| y),
                    p[2],
                ]
            }));
        }
    }
    out
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
    let mut ground = match terrain_scene(root, icao) {
        Ok((terrain, ground)) => {
            println!(
                "Live airport source terrain enabled; physical height queries active, airport surface draping remains preview interpolation"
            );
            meshes.extend(terrain.meshes);
            Some(std::sync::Arc::new(ground))
        }
        Err(e) => {
            eprintln!("Source terrain unavailable; flat airport fallback: {e}");
            let origin = frame.origin();
            if let Some(path) = install.terrain_tile(origin[0], origin[1])
                && let Ok(mesh) = xp_scenery_raster(&path, &frame, model.elevation_m)
            {
                meshes.push(mesh);
            }
            None
        }
    };
    let mut airport_surfaces = Vec::new();
    for layer in layers {
        if ground.is_some() && layer.kind == openxplane::world::Kind::Ground {
            continue;
        }
        let triangles = if let Some(ground) = &ground {
            drape_triangles(&layer.triangles, ground)
        } else {
            layer.triangles
        };
        if ground.is_some()
            && matches!(
                layer.kind,
                openxplane::world::Kind::Runway | openxplane::world::Kind::Pavement
            )
        {
            airport_surfaces.extend_from_slice(triangles.as_chunks::<3>().0);
        }
        let vertices = triangles
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
            repeat_texture: false,
            glass: false,
            center: (low + high) * 0.5,
            instances: vec![[0.0; 4]],
            border: None,
            animation: None,
        });
    }
    if let Some(base) = ground.take() {
        ground = Some(std::sync::Arc::new(
            openxplane::ground::Ground::with_surfaces(base, airport_surfaces),
        ));
    }
    if let Some(ground) = &ground {
        model.set_ground(ground.clone());
    }
    let season = std::env::var("OPENXPLANE_SCENERY_SEASON").unwrap_or_else(|_| "sum".into());
    let overlay_library = install.preview_library_for(Some(frame.origin()), Some(&season))?;
    let mut exclusions = Vec::new();
    for (pack, path) in install.overlay_tiles(frame.origin()[0], frame.origin()[1])? {
        println!("Scenery overlay: {}", path.display());
        let bytes = match openxplane::dsf::read_bytes(&path) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("DSF overlay unavailable {}: {e}", path.display());
                continue;
            }
        };
        match airport_objects(
            &pack,
            &bytes,
            &overlay_library,
            &frame,
            ground.as_deref(),
            &exclusions,
        ) {
            Ok(objects) => meshes.extend(objects),
            Err(e) => eprintln!("DSF overlay unavailable {}: {e}", path.display()),
        }
        match openxplane::dsf::object_exclusions(&bytes) {
            Ok(zones) => exclusions.extend(zones),
            Err(e) => eprintln!("DSF object exclusions unavailable {}: {e}", path.display()),
        }
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
    fn polygon_texture_follows_native_candidate_order() {
        let dir = std::env::temp_dir().join(format!("xp_pol_tex_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let pol = dir.join("a.pol");
        assert_eq!(texture_path(&pol, "tile.dds"), None);
        fs::write(dir.join("tile.ktx2"), b"").unwrap();
        assert_eq!(texture_path(&pol, "tile.dds"), Some(dir.join("tile.ktx2")));
        fs::write(dir.join("tile.bmp"), b"").unwrap();
        assert_eq!(texture_path(&pol, "tile.dds"), Some(dir.join("tile.bmp")));
        fs::write(dir.join("tile.png"), b"").unwrap();
        assert_eq!(texture_path(&pol, "tile.dds"), Some(dir.join("tile.png")));
        fs::write(dir.join("tile.dds"), b"").unwrap();
        assert_eq!(texture_path(&pol, "tile.dds"), Some(dir.join("tile.dds")));
        let _ = fs::remove_dir_all(&dir);
    }

    fn ktx2(format: u32, w: u32, h: u32, payload: &[u8]) -> Vec<u8> {
        let mut d = b"\xabKTX 20\xbb\r\n\x1a\n".to_vec();
        for v in [format, 1, w, h, 0, 0, 1, 1, 0] {
            d.extend(v.to_le_bytes());
        }
        d.extend([0u8; 16]); // dfd and kvd ranges
        d.extend([0u8; 16]); // sgd range
        d.extend(104u64.to_le_bytes());
        d.extend((payload.len() as u64).to_le_bytes());
        d.extend((payload.len() as u64).to_le_bytes());
        d.extend(payload);
        d
    }

    #[test]
    fn ktx2_decodes_bc1_bc3_and_rgba8() {
        // BC1: red (0xf800) and blue (0x001f), every texel index 0 -> red.
        let bc1 = [0x00, 0xf8, 0x1f, 0x00, 0, 0, 0, 0];
        let img = decode_ktx2(&ktx2(131, 4, 4, &bc1)).unwrap();
        assert!(img.pixels().all(|p| p.0 == [255, 0, 0, 255]));
        // Index 1 selects the second colour.
        let bc1 = [0x00, 0xf8, 0x1f, 0x00, 0x55, 0x55, 0x55, 0x55];
        let img = decode_ktx2(&ktx2(131, 4, 4, &bc1)).unwrap();
        assert!(img.pixels().all(|p| p.0 == [0, 0, 255, 255]));
        // BC3: alpha endpoints 255/0, all alpha indices 1 -> alpha 0.
        let mut bc3 = vec![255, 0, 0x49, 0x92, 0x24, 0x49, 0x92, 0x24];
        bc3.extend([0x00, 0xf8, 0x1f, 0x00, 0, 0, 0, 0]);
        let img = decode_ktx2(&ktx2(137, 4, 4, &bc3)).unwrap();
        assert!(img.pixels().all(|p| p.0 == [255, 0, 0, 0]));
        let img = decode_ktx2(&ktx2(37, 1, 1, &[1, 2, 3, 4])).unwrap();
        assert_eq!(img.get_pixel(0, 0).0, [1, 2, 3, 4]);
        assert!(decode_ktx2(&ktx2(999, 1, 1, &[0; 4])).is_err());
    }

    #[test]
    #[ignore = "needs the installed X-Plane; set OPENXPLANE_KTX2_SAMPLE and optionally OPENXPLANE_KTX2_OUT"]
    fn ktx2_decodes_installed_sample() {
        let path = std::env::var("OPENXPLANE_KTX2_SAMPLE").unwrap();
        let img = open_texture(&path).unwrap().to_rgba8();
        assert!(img.width() > 0 && img.height() > 0);
        if let Ok(out) = std::env::var("OPENXPLANE_KTX2_OUT") {
            img.save(out).unwrap();
        }
    }

    #[test]
    fn custom_texture_folders_follow_package_relative_candidates() {
        let base = std::env::temp_dir().join(format!("xp_custom_tex_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let pack = base.join("Custom Scenery").join("Pack");
        let obj = pack.join("objects").join("a.obj");
        fs::create_dir_all(obj.parent().unwrap()).unwrap();
        fs::create_dir_all(pack.join("custom object textures")).unwrap();
        fs::create_dir_all(pack.join("custom terrain textures")).unwrap();
        fs::write(pack.join("custom object textures/t.png"), b"").unwrap();
        fs::write(pack.join("custom terrain textures/t.bmp"), b"").unwrap();
        assert_eq!(texture_path(&obj, "t.dds"), Some(pack.join("custom object textures/t.png")));
        assert_eq!(
            terrain_texture_path(&obj, "t.dds"),
            Some(pack.join("custom terrain textures/t.bmp"))
        );
        // A package-relative file wins over the custom folder, KTX2 is never tried there.
        fs::write(pack.join("objects/t.ktx2"), b"").unwrap();
        assert_eq!(texture_path(&obj, "t.dds"), Some(pack.join("objects/t.ktx2")));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn texture_path_edge_names_follow_native_replacement() {
        let dir = std::env::temp_dir().join(format!("xp_tex_edge_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let pol = dir.join("a.pol");
        // Names of four bytes or fewer are not rewritten by the DDS/PNG/BMP blocks.
        fs::write(dir.join("abcd"), b"").unwrap();
        assert_eq!(texture_path(&pol, "abcd"), Some(dir.join("abcd")));
        // Undotted names get ".ktx2" appended; backslashes are separators.
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub").join("noext.ktx2"), b"").unwrap();
        assert_eq!(texture_path(&pol, r"sub\noext"), Some(dir.join("sub/noext.ktx2")));
        // The original extension is replaced, not preserved, so a PNG declaration finds DDS.
        fs::write(dir.join("sub").join("x.dds"), b"").unwrap();
        assert_eq!(texture_path(&pol, "sub/x.png"), Some(dir.join("sub/x.dds")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn draped_airport_triangles_follow_physical_plane_without_changing_area() {
        let ground = openxplane::ground::Ground::new(vec![[
            [0.0, 10.0, 0.0],
            [200.0, 30.0, 0.0],
            [0.0, 10.0, 200.0],
        ]]);
        let out = drape_triangles(
            &[[0.0, -0.03, 0.0], [200.0, -0.03, 0.0], [0.0, -0.03, 200.0]],
            &ground,
        );
        assert!(out.len() > 3);
        let area: f32 = out
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                for p in t {
                    assert!((p[1] - (10.18 + p[0] * 0.1)).abs() < 1e-4);
                }
                ((t[1][0] - t[0][0]) * (t[2][2] - t[0][2])
                    - (t[1][2] - t[0][2]) * (t[2][0] - t[0][0]))
                    .abs()
                    * 0.5
            })
            .sum();
        assert!((area - 20000.0).abs() < 0.01);
    }

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
