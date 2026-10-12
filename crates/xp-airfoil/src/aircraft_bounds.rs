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
