//! Recovered group-wise wing aspect and radial-centroid shape parameters.
use crate::wing_element::{Boundary, interpolate_clamped};

pub struct WingGroupMember<'a> {
    pub span: f32,
    pub sweep_deg: f32,
    pub boundary: Boundary<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WingDimensions {
    /// Original Wing+0x18.
    pub aspect_parameter: f32,
    /// Original Wing+0x1c; not the tip/root chord ratio.
    pub shape_parameter: f32,
    pub area_sum: f32,
    pub weighted_radius_sum: f32,
    pub mean_radius: f32,
    pub centroid_fraction: f32,
    pub length: f32,
}

const RAD: f32 = f32::from_bits(0x3c8e_fa36);
fn sse_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}
fn sse_min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

fn length(endpoints: [[f32; 3]; 2]) -> f32 {
    let dx = endpoints[0][0] - endpoints[1][0];
    let dy = endpoints[0][1] - endpoints[1][1];
    let dz = endpoints[0][2] - endpoints[1][2];
    ((dx * dx + dy * dy) + dz * dz).sqrt()
}

pub fn wing_dimensions(
    members: &[WingGroupMember],
    endpoints: [[f32; 3]; 2],
    group_area: f32,
) -> Result<WingDimensions, String> {
    let cosine: Vec<f32> = members.iter().map(|m| (m.sweep_deg * RAD).cos()).collect();
    let ratio = (endpoints[1][2] - endpoints[0][2]) / length(endpoints);
    let angle_deg = (f64::from(ratio).asin() as f32) * 57.295_776;
    let projection_cosine = (angle_deg * RAD).cos();
    wing_dimensions_with_trig(members, endpoints, group_area, &cosine, projection_cosine)
}

/// `0x14117285b..0x141172bc6`, with original CRT cosines supplied for deterministic replay.
/// Group membership, endpoints and previously accumulated group area remain explicit.
pub fn wing_dimensions_with_trig(
    members: &[WingGroupMember],
    endpoints: [[f32; 3]; 2],
    group_area: f32,
    cosine: &[f32],
    projection_cosine: f32,
) -> Result<WingDimensions, String> {
    if members.is_empty()
        || cosine.len() != members.len()
        || !group_area.is_finite()
        || group_area <= 0.0
        || !projection_cosine.is_finite()
        || !endpoints.iter().flatten().all(|v| v.is_finite())
    {
        return Err("invalid wing group operand".into());
    }
    let length = length(endpoints);
    if !length.is_finite() || length <= 0.0 {
        return Err("invalid wing group endpoints".into());
    }
    let mut area_sum = 0.0_f32;
    let mut weighted_radius_sum = 0.0_f32;
    for (member, cos) in members.iter().zip(cosine) {
        let b = &member.boundary;
        let n = b.chord.len().checked_sub(1).ok_or("empty wing boundary")?;
        if !(1..=10).contains(&n)
            || [b.x.len(), b.y.len(), b.z.len()]
                .iter()
                .any(|v| *v != n + 1)
            || !member.span.is_finite()
            || member.span <= 0.0
            || !cos.is_finite()
            || !b
                .x
                .iter()
                .chain(b.y)
                .chain(b.z)
                .chain(b.chord)
                .all(|v| v.is_finite())
        {
            return Err("invalid wing group member geometry".into());
        }
        for index in 0..n {
            let mean = f64::from(b.chord[index] + b.chord[index + 1]) * 0.5;
            let area =
                ((mean * f64::from(member.span) * f64::from(*cos)) / f64::from(n as f32)) as f32;
            area_sum += area;
            let midpoint = (f64::from(index as f32) + 0.5) as f32;
            let cx = interpolate_clamped(
                index as f32,
                b.x[index],
                (index + 1) as f32,
                b.x[index + 1],
                midpoint,
            );
            let cy = interpolate_clamped(
                index as f32,
                b.y[index],
                (index + 1) as f32,
                b.y[index + 1],
                midpoint,
            );
            let dx = cx - endpoints[0][0];
            let dy = cy - endpoints[0][1];
            let radius = (dy * dy + dx * dx).sqrt();
            weighted_radius_sum += radius * area;
        }
    }
    let mean_radius = weighted_radius_sum / sse_max(area_sum, 0.01);
    let dx = endpoints[1][0] - endpoints[0][0];
    let dy = endpoints[1][1] - endpoints[0][1];
    let projected_distance = (dx * dx + dy * dy).sqrt();
    let centroid_fraction = mean_radius / sse_max(projected_distance, 0.01);
    let ramp =
        (centroid_fraction - f32::from_bits(0x3ea8_f5c3)) * f32::from_bits(0x40bc_3c3d) + 0.0;
    let shape_parameter = if 0.0 > ramp { 0.0 } else { sse_min(1.0, ramp) };
    let projection = projection_cosine * length;
    let full_span = (f64::from(projection) * 2.0) as f32;
    let aspect_parameter = sse_max(
        (f64::from(full_span * full_span) / (f64::from(group_area) * 2.0)) as f32,
        0.5,
    );
    Ok(WingDimensions {
        aspect_parameter,
        shape_parameter,
        area_sum,
        weighted_radius_sum,
        mean_radius,
        centroid_fraction,
        length,
    })
}
