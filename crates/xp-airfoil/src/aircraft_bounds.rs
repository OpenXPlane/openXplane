//! Original aircraft reference-size block, with an explicit input point vector.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AircraftBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub planar_halfspan: f32,
    /// Original Aircraft+0x64f4/0x64f8/0x64fc: 1.01 times each largest absolute extent.
    pub size: [f32; 3],
}

fn sse_min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}
fn sse_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// `0x141a89096..0x141a8923e`. Includes the origin, even for an empty vector.
/// The upstream producer of the original point list must be recovered separately.
pub fn aircraft_bounds(points: &[[f32; 3]]) -> Result<AircraftBounds, String> {
    let mut min = [0.0; 3];
    let mut max = [0.0; 3];
    for point in points {
        for axis in 0..3 {
            if !point[axis].is_finite() {
                return Err("aircraft bounds require finite coordinates".into());
            }
            min[axis] = sse_min(min[axis], point[axis]);
            max[axis] = sse_max(max[axis], point[axis]);
        }
    }
    let half_x = (f64::from(max[0] - min[0]) * 0.5) as f32;
    let half_z = (f64::from(max[2] - min[2]) * 0.5) as f32;
    let planar_halfspan = sse_max(half_x, half_z);
    let size = std::array::from_fn(|axis| {
        (f64::from(sse_max(min[axis].abs(), max[axis].abs())) * 1.01) as f32
    });
    Ok(AircraftBounds {
        min,
        max,
        planar_halfspan,
        size,
    })
}

/// `0x141a88000..0x141a88318`: emit leading/trailing edge points of an enabled wing.
/// Allocation and outer enabled-wing selection remain the caller's responsibility.
pub fn wing_bounds_points(
    boundary: &crate::wing_element::Boundary,
) -> Result<Vec<[f32; 3]>, String> {
    let n = boundary
        .chord
        .len()
        .checked_sub(1)
        .ok_or("empty wing boundary")?;
    if !(1..=10).contains(&n)
        || [boundary.x.len(), boundary.y.len(), boundary.z.len()]
            .iter()
            .any(|v| *v != n + 1)
        || !boundary
            .chord
            .iter()
            .chain(boundary.x)
            .chain(boundary.y)
            .chain(boundary.z)
            .all(|v| v.is_finite())
    {
        return Err("invalid wing boundary point input".into());
    }
    let mut result = Vec::with_capacity((n + 1) * 2);
    for i in 0..=n {
        let lower = i.min(n - 1);
        let sample = |values: &[f32]| {
            crate::wing_element::interpolate_clamped(
                lower as f32,
                values[lower],
                (lower + 1) as f32,
                values[lower + 1],
                i as f32,
            )
        };
        let x = sample(boundary.x);
        let y = sample(boundary.y);
        let z = sample(boundary.z);
        let chord = f64::from(boundary.chord[i]);
        result.push([x, y, (f64::from(z) - chord * 0.25) as f32]);
        result.push([x, y, (chord * 0.75 + f64::from(z)) as f32]);
    }
    Ok(result)
}

/// Input to the original body point-list pass. Grid values are the already
/// transformed runtime coordinates, with the original fixed eighteen-point stride.
pub struct BodyBoundary<'a> {
    pub enabled: bool,
    pub origin: [f32; 3],
    pub rings: usize,
    pub stations: &'a [[[f32; 3]; 18]],
}

/// Original 0x141a88340..0x141a88735 body emission (one body).
/// Each quad contributes all four vertices, including duplicates and ring wrap.
/// Raw ACF grid conversion and runtime body transforms remain caller work.
pub fn body_bounds_points(body: &BodyBoundary<'_>) -> Result<Vec<[f32; 3]>, String> {
    if !body.enabled || body.stations.len() < 2 || body.rings == 0 {
        return Ok(Vec::new());
    }
    if body.rings > 18
        || body.stations.len() > 20
        || !body.origin.iter().all(|v| v.is_finite())
        || !body
            .stations
            .iter()
            .flat_map(|s| s[..body.rings].iter())
            .flatten()
            .all(|v| v.is_finite())
    {
        return Err(
            "body bounds require finite coordinates, <=20 stations and <=18 ring points".into(),
        );
    }
    let mut points = Vec::with_capacity((body.stations.len() - 1) * body.rings * 4);
    for station in 0..body.stations.len() - 1 {
        for ring in 0..body.rings {
            let next = (ring + 1) % body.rings;
            for (s, r) in [
                (station, ring),
                (station + 1, ring),
                (station + 1, next),
                (station, next),
            ] {
                points.push(std::array::from_fn(|axis| {
                    body.stations[s][r][axis] + body.origin[axis]
                }));
            }
        }
    }
    Ok(points)
}

/// Stored gear fields used by the original reference point-list producer.
pub struct GearBoundary {
    pub kind: i32,
    pub leg_length: f32,
    pub tire_radius: f32,
    /// Gear+0x64/+0x70/+0x7c, already in the CG-relative frame.
    pub center: [f32; 3],
}

/// `0x141a87e30..0x141a87fb2`: ten gear slots, nonzero kind enabled.
/// Emits two opposite corners with extent leg_length+tire_radius.
pub fn gear_bounds_points(gears: &[GearBoundary]) -> Result<Vec<[f32; 3]>, String> {
    if gears.len() > 10 {
        return Err("original gear bounds have ten slots".into());
    }
    let mut points = Vec::with_capacity(gears.len() * 2);
    for gear in gears.iter().filter(|g| g.kind != 0) {
        if ![gear.leg_length, gear.tire_radius]
            .iter()
            .chain(&gear.center)
            .all(|v| v.is_finite())
        {
            return Err("gear bounds require finite active fields".into());
        }
        let extent = gear.tire_radius + gear.leg_length;
        points.push(gear.center.map(|v| v - extent));
        points.push(gear.center.map(|v| extent + v));
    }
    Ok(points)
}

/// Stored part/propeller fields for the reference point producer.
pub struct PropellerBoundary {
    pub kind: i32,
    pub elements: usize,
    /// Part+0x644 float array, including the extra upper sample at index 11.
    pub radii: [f32; 12],
    /// Part+0x778/+0x77c/+0x780 in the stored CG-relative frame.
    pub center: [f32; 3],
}

/// Original propeller point pass `0x141a8879b..0x141a88dc6`.
/// Kind 9 is skipped. The query is always 10, using the original value-clamped
/// interpolation at lower index min(elements-1, 10), not a radius maximum.
pub fn propeller_bounds_points(parts: &[PropellerBoundary]) -> Result<Vec<[f32; 3]>, String> {
    if parts.len() > 8 {
        return Err("original part point pass supports eight propeller slots".into());
    }
    let mut points = Vec::with_capacity(parts.len() * 8);
    for part in parts.iter().filter(|p| p.kind != 9) {
        if part.elements == 0
            || part.elements > 100
            || !part.center.iter().chain(&part.radii).all(|v| v.is_finite())
        {
            return Err("propeller bounds require finite fields and element count 1..100".into());
        }
        let lower = (part.elements - 1).min(10);
        let radius = crate::wing_element::interpolate_clamped(
            lower as f32,
            part.radii[lower],
            (lower + 1) as f32,
            part.radii[lower + 1],
            10.0,
        );
        for x in [false, true] {
            for y in [false, true] {
                for z in [false, true] {
                    points.push(std::array::from_fn(|axis| {
                        if [x, y, z][axis] {
                            radius + part.center[axis]
                        } else {
                            part.center[axis] - radius
                        }
                    }));
                }
            }
        }
    }
    Ok(points)
}

/// Assemble the original main reference vector in source order, then reduce it.
/// Every input must already contain the stored runtime geometry. This API does
/// not populate grids, update articulated geometry or substitute renderer bounds.
pub fn reference_geometry_bounds(
    gears: &[GearBoundary],
    wings: &[Option<crate::wing_element::Boundary<'_>>],
    bodies: &[BodyBoundary<'_>],
    propellers: &[PropellerBoundary],
) -> Result<(Vec<[f32; 3]>, AircraftBounds), String> {
    if wings.len() > 48 || bodies.len() > 39 {
        return Err("original reference geometry has 48 wing and 39 body slots".into());
    }
    let mut points = gear_bounds_points(gears)?;
    for wing in wings.iter().flatten() {
        points.extend(wing_bounds_points(wing)?);
    }
    for body in bodies {
        points.extend(body_bounds_points(body)?);
    }
    points.extend(propeller_bounds_points(propellers)?);
    let bounds = aircraft_bounds(&points)?;
    Ok((points, bounds))
}
