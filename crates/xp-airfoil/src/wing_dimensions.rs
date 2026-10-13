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

#[derive(Clone, Copy, Debug)]
pub enum CandidateEnd {
    Root,
    Tip,
}

pub struct WingJoinReference {
    pub point: [f32; 3],
    pub chord: f32,
    pub dihedral_deg: f32,
    /// Original final grouping pass permits an angle mismatch away from the centerline.
    pub late_pass: bool,
}

/// Candidate-root predicate `0x141a6beb0`, or candidate-tip `0x141a6bbf0`.
/// Endpoint extraction executes the same clamped interpolation as the original.
pub fn can_join_wing(
    candidate: &Boundary,
    dihedral_deg: f32,
    reference: &WingJoinReference,
    end: CandidateEnd,
) -> Result<bool, String> {
    let n = candidate
        .x
        .len()
        .checked_sub(1)
        .ok_or("empty join candidate")?;
    if !(1..=10).contains(&n)
        || candidate.y.len() != n + 1
        || candidate.z.len() != n + 1
        || !candidate
            .x
            .iter()
            .chain(candidate.y)
            .chain(candidate.z)
            .all(|v| v.is_finite())
        || !reference.point.iter().all(|v| v.is_finite())
        || !reference.chord.is_finite()
        || !reference.dihedral_deg.is_finite()
        || !dihedral_deg.is_finite()
    {
        return Err("invalid wing join operand".into());
    }
    let sample = |values: &[f32], index: usize| {
        let lower = index.min(n - 1);
        interpolate_clamped(
            lower as f32,
            values[lower],
            (lower + 1) as f32,
            values[lower + 1],
            index as f32,
        )
    };
    let root_x = f64::from(sample(candidate.x, 0));
    let tip_x = f64::from(sample(candidate.x, n));
    if (root_x < -0.01 && tip_x > 0.01) || (root_x > 0.01 && tip_x < -0.01) {
        return Ok(false);
    }
    let index = match end {
        CandidateEnd::Root => 0,
        CandidateEnd::Tip => n,
    };
    for (axis, values) in [candidate.x, candidate.y, candidate.z].iter().enumerate() {
        let delta = (reference.point[axis] - sample(values, index)).abs();
        let threshold = f64::from(reference.chord) * if axis == 2 { 0.2 } else { 0.01 };
        if threshold <= f64::from(delta) {
            return Ok(false);
        }
    }
    let mut angle = dihedral_deg - reference.dihedral_deg;
    if !angle.is_finite() || angle.abs() > 1_000_000.0 {
        return Err("wing join angle outside supported domain".into());
    }
    while -180.0 > angle {
        angle += 360.0;
    }
    while angle > 180.0 {
        angle += -360.0;
    }
    Ok(angle.abs() <= 45.0 || (f64::from(reference.point[0].abs()) >= 0.01 && reference.late_pass))
}

/// One explicitly enabled/disabled slot of the original 48-wing geometry array.
pub struct GroupingWing<'a> {
    pub id: usize,
    pub enabled: bool,
    pub root_chord: f32,
    pub tip_chord: f32,
    pub dihedral_deg: f32,
    pub member: WingGroupMember<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WingGroup {
    /// Discovery order; the original may append a candidate twice in one iteration.
    pub members: Vec<usize>,
    pub endpoints: [[f32; 3]; 2],
    pub area: f32,
    pub reference_chords: [f32; 2],
    pub reference_dihedrals: [f32; 2],
}

fn group_endpoint(boundary: &Boundary, tip: bool) -> [f32; 3] {
    let n = boundary.x.len() - 1;
    let index = if tip { n } else { 0 };
    let lower = index.min(n - 1);
    std::array::from_fn(|axis| {
        let values = [boundary.x, boundary.y, boundary.z][axis];
        interpolate_clamped(
            lower as f32,
            values[lower],
            (lower + 1) as f32,
            values[lower + 1],
            index as f32,
        )
    })
}

/// Evaluate the original element-area helper before performing ordered grouping.
pub fn group_wings(wings: &[GroupingWing], root_id: usize) -> Result<WingGroup, String> {
    let areas: Vec<Vec<f32>> = wings
        .iter()
        .map(|wing| {
            let b = &wing.member.boundary;
            let n = b.chord.len().saturating_sub(1);
            (0..n)
                .map(|i| {
                    crate::wing_element::element_area(
                        wing.member.sweep_deg,
                        wing.member.span,
                        b.chord,
                        i,
                        n as i32,
                    )
                })
                .collect()
        })
        .collect();
    group_wings_with_areas(wings, root_id, &areas)
}

/// Ordered 49-pass group scan of `0x141170d40`, with explicit per-element areas.
/// Numerical predicates are native-code verified; full scan evidence is tracked separately.
pub fn group_wings_with_areas(
    wings: &[GroupingWing],
    root_id: usize,
    element_areas: &[Vec<f32>],
) -> Result<WingGroup, String> {
    if wings.len() != element_areas.len() || root_id >= 48 {
        return Err("invalid wing grouping input".into());
    }
    let mut slots: [Option<usize>; 48] = [None; 48];
    for (index, wing) in wings.iter().enumerate() {
        if wing.id >= 48 || slots[wing.id].replace(index).is_some() {
            return Err("duplicate or invalid wing slot".into());
        }
        if !wing.enabled {
            continue;
        }
        let b = &wing.member.boundary;
        let n = b.chord.len().saturating_sub(1);
        if !(1..=10).contains(&n)
            || [b.x.len(), b.y.len(), b.z.len()]
                .iter()
                .any(|v| *v != n + 1)
            || element_areas[index].len() != n
            || !element_areas[index].iter().all(|v| v.is_finite())
            || !b
                .x
                .iter()
                .chain(b.y)
                .chain(b.z)
                .chain(b.chord)
                .all(|v| v.is_finite())
            || ![wing.root_chord, wing.tip_chord, wing.dihedral_deg]
                .iter()
                .all(|v| v.is_finite())
        {
            return Err("invalid enabled wing grouping geometry".into());
        }
    }
    let root_index = slots[root_id].ok_or("missing grouping root")?;
    let root = &wings[root_index];
    if !root.enabled {
        return Err("disabled grouping root".into());
    }
    let mut result = WingGroup {
        members: vec![root_id],
        endpoints: [
            group_endpoint(&root.member.boundary, false),
            group_endpoint(&root.member.boundary, true),
        ],
        area: element_areas[root_index].iter().fold(0.0, |sum, v| sum + v),
        reference_chords: [root.root_chord, root.tip_chord],
        reference_dihedrals: [root.dihedral_deg; 2],
    };
    let mut visited = [false; 48];
    for pass in 0..49 {
        for (id, slot) in slots.iter().enumerate() {
            let Some(index) = slot else {
                continue;
            };
            let wing = &wings[*index];
            if id == root_id || !wing.enabled || visited[id] {
                continue;
            }
            // No visited recheck between these two tests: they share one candidate iteration.
            for side in 0..2 {
                let reference = WingJoinReference {
                    point: result.endpoints[side],
                    chord: result.reference_chords[side],
                    dihedral_deg: result.reference_dihedrals[side],
                    late_pass: pass == 48,
                };
                let selected = if side == 0 {
                    CandidateEnd::Tip
                } else {
                    CandidateEnd::Root
                };
                if !can_join_wing(
                    &wing.member.boundary,
                    wing.dihedral_deg,
                    &reference,
                    selected,
                )? {
                    continue;
                }
                let subtotal = element_areas[*index]
                    .iter()
                    .fold(0.0, |sum, area| sum + area);
                result.area += subtotal;
                result.endpoints[side] = group_endpoint(&wing.member.boundary, side == 1);
                result.reference_chords[side] = if side == 0 {
                    wing.root_chord
                } else {
                    wing.tip_chord
                };
                result.reference_dihedrals[side] = wing.dihedral_deg;
                visited[id] = true;
                result.members.push(id);
            }
        }
    }
    Ok(result)
}
