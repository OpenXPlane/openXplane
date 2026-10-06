//! Ground geometry of an airport built from apt.dat rows: local projection, bezier flattening,
//! polygon triangulation and the layered triangle lists the viewer draws.
//!
//! Projection: local frame x = east, y = up, z = south (the X-Plane OpenGL-style frame), with a flat
//! equirectangular approximation around the airport datum on a sphere of radius 6,371,000 m; no
//! datum or elevation model. The surfaces lie just below the flight model's ground plane at height 0
//! (ground -0.20 m, pavement -0.12, holes -0.09, runway -0.03, markings -0.01) so wheels rest on them. Pavement holes (loops after the first), the bezier handle rule
//! (incoming handle mirrored through the node) and the surface colour table are the documented
//! apt.dat conventions and openXplane policy, not behaviour confirmed in the reference build.
use crate::apt::{Airport, Node, Pavement, Runway};

const EARTH_RADIUS_M: f64 = 6_371_000.0;
const BEZIER_STEPS: usize = 8;

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    lat0: f64,
    lon0: f64,
}

impl Frame {
    pub fn new(lat0: f64, lon0: f64) -> Self {
        Self { lat0, lon0 }
    }

    /// (x east, z south) in metres.
    pub fn to_local(&self, lat: f64, lon: f64) -> [f32; 2] {
        let x = (lon - self.lon0).to_radians() * self.lat0.to_radians().cos() * EARTH_RADIUS_M;
        let z = -(lat - self.lat0).to_radians() * EARTH_RADIUS_M;
        [x as f32, z as f32]
    }
}

/// Flattens a closed loop of nodes into a polygon, sampling a cubic bezier on segments where
/// either end has a control point.
pub fn flatten_loop(nodes: &[Node], frame: &Frame) -> Vec<[f32; 2]> {
    let n = nodes.len();
    let mut out = Vec::new();
    for i in 0..n {
        let a = nodes[i];
        let b = nodes[(i + 1) % n];
        let p0 = (a.lat, a.lon);
        let p3 = (b.lat, b.lon);
        let c0 = a.ctrl.unwrap_or(p0);
        // the incoming handle of the next node is its control point mirrored through the node
        let c1 = b.ctrl.map_or(p3, |c| (2.0 * p3.0 - c.0, 2.0 * p3.1 - c.1));
        if a.ctrl.is_none() && b.ctrl.is_none() {
            out.push(frame.to_local(p0.0, p0.1));
            continue;
        }
        for s in 0..BEZIER_STEPS {
            let t = s as f64 / BEZIER_STEPS as f64;
            let u = 1.0 - t;
            let lat = u * u * u * p0.0
                + 3.0 * u * u * t * c0.0
                + 3.0 * u * t * t * c1.0
                + t * t * t * p3.0;
            let lon = u * u * u * p0.1
                + 3.0 * u * u * t * c0.1
                + 3.0 * u * t * t * c1.1
                + t * t * t * p3.1;
            out.push(frame.to_local(lat, lon));
        }
    }
    out
}

fn signed_area(poly: &[[f32; 2]]) -> f32 {
    let mut a = 0.0;
    for i in 0..poly.len() {
        let p = poly[i];
        let q = poly[(i + 1) % poly.len()];
        a += p[0] * q[1] - q[0] * p[1];
    }
    a * 0.5
}

fn cross(o: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

fn in_triangle(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    cross(a, b, p) >= 0.0 && cross(b, c, p) >= 0.0 && cross(c, a, p) >= 0.0
}

/// Ear-clipping triangulation of a simple polygon (either winding). Returns triangles as index
/// triples into `poly`. A polygon that is not simple (self-intersecting) yields the triangles
/// that could be clipped; the rest is dropped rather than guessed.
pub fn triangulate(poly: &[[f32; 2]]) -> Vec<[usize; 3]> {
    let n = poly.len();
    if n < 3 {
        return Vec::new();
    }
    let ccw = signed_area(poly) > 0.0;
    let mut idx: Vec<usize> = if ccw {
        (0..n).collect()
    } else {
        (0..n).rev().collect()
    };
    let mut tris = Vec::with_capacity(n - 2);
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for k in 0..m {
            let (ia, ib, ic) = (idx[(k + m - 1) % m], idx[k], idx[(k + 1) % m]);
            let (a, b, c) = (poly[ia], poly[ib], poly[ic]);
            if cross(a, b, c) <= 1e-9 {
                continue; // reflex or collinear corner
            }
            let blocked = idx
                .iter()
                .filter(|&&j| j != ia && j != ib && j != ic)
                .any(|&j| in_triangle(poly[j], a, b, c));
            if blocked {
                continue;
            }
            tris.push([ia, ib, ic]);
            idx.remove(k);
            clipped = true;
            break;
        }
        if !clipped {
            // drop a degenerate vertex so the loop always terminates
            let k = (0..idx.len())
                .min_by(|&x, &y| {
                    let f = |k: usize| {
                        let m = idx.len();
                        cross(
                            poly[idx[(k + m - 1) % m]],
                            poly[idx[k]],
                            poly[idx[(k + 1) % m]],
                        )
                        .abs()
                    };
                    f(x).total_cmp(&f(y))
                })
                .unwrap();
            idx.remove(k);
        }
    }
    if idx.len() == 3 && cross(poly[idx[0]], poly[idx[1]], poly[idx[2]]) > 1e-9 {
        tris.push([idx[0], idx[1], idx[2]]);
    }
    tris
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Ground,
    Pavement,
    /// A pavement hole, drawn in the ground colour above the pavement.
    Hole,
    Runway,
    Marking,
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub kind: Kind,
    pub color: [u8; 4],
    /// Triangle vertices (x, y, z), three per triangle, all facing up.
    pub triangles: Vec<[f32; 3]>,
}

const GROUND_COLOR: [u8; 4] = [70, 108, 58, 255];
const MARKING_COLOR: [u8; 4] = [235, 235, 230, 255];

/// Surface code colours (apt.dat convention; unknown codes use the asphalt grey).
pub fn surface_color(surface: u32) -> [u8; 4] {
    match surface {
        2 => [150, 150, 146, 255],
        3 => [78, 122, 60, 255],
        4 => [122, 98, 72, 255],
        5 => [134, 128, 116, 255],
        12 => [196, 180, 140, 255],
        13 => [52, 92, 140, 255],
        14 => [236, 240, 244, 255],
        _ => [84, 84, 88, 255],
    }
}

fn quad(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2], y: f32, out: &mut Vec<[f32; 3]>) {
    for p in [a, b, c, a, c, d] {
        out.push([p[0], y, p[1]]);
    }
}

/// A strip of `width` metres centred on the segment `a`..`b` (x, z).
fn strip(a: [f32; 2], b: [f32; 2], width: f32, y: f32, out: &mut Vec<[f32; 3]>) {
    let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dz * dz).sqrt();
    if len < 1e-3 {
        return;
    }
    let (nx, nz) = (-dz / len * width * 0.5, dx / len * width * 0.5);
    quad(
        [a[0] + nx, a[1] + nz],
        [b[0] + nx, b[1] + nz],
        [b[0] - nx, b[1] - nz],
        [a[0] - nx, a[1] - nz],
        y,
        out,
    );
}

fn pavement_layers(p: &Pavement, frame: &Frame, layers: &mut Vec<Layer>) {
    let color = surface_color(p.surface);
    let mut tri = Vec::new();
    let mut holes = Vec::new();
    for (i, nodes) in p.loops.iter().enumerate() {
        let poly = flatten_loop(nodes, frame);
        let target = if i == 0 { &mut tri } else { &mut holes };
        let y = if i == 0 { -0.12 } else { -0.09 };
        for t in triangulate(&poly) {
            for k in t {
                target.push([poly[k][0], y, poly[k][1]]);
            }
        }
    }
    if !tri.is_empty() {
        layers.push(Layer {
            kind: Kind::Pavement,
            color,
            triangles: tri,
        });
    }
    if !holes.is_empty() {
        layers.push(Layer {
            kind: Kind::Hole,
            color: GROUND_COLOR,
            triangles: holes,
        });
    }
}

/// Local (x, z) coordinates of a runway's two ends.
pub fn runway_ends(r: &Runway, frame: &Frame) -> [[f32; 2]; 2] {
    [
        frame.to_local(r.ends[0].lat, r.ends[0].lon),
        frame.to_local(r.ends[1].lat, r.ends[1].lon),
    ]
}

fn runway_layers(r: &Runway, frame: &Frame, layers: &mut Vec<Layer>) {
    let [a, b] = runway_ends(r, frame);
    let mut body = Vec::new();
    strip(a, b, r.width_m, -0.03, &mut body);
    layers.push(Layer {
        kind: Kind::Runway,
        color: surface_color(r.surface),
        triangles: body,
    });
    let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dz * dz).sqrt();
    if len < 120.0 {
        return;
    }
    let (ux, uz) = (dx / len, dz / len);
    let mut marks = Vec::new();
    // centerline dashes: 30 m dashes every 50 m, leaving room at both thresholds
    let mut s = 60.0;
    while s + 30.0 < len - 60.0 {
        let p = [a[0] + ux * s, a[1] + uz * s];
        let q = [a[0] + ux * (s + 30.0), a[1] + uz * (s + 30.0)];
        strip(p, q, 0.9, -0.01, &mut marks);
        s += 50.0;
    }
    // edge lines and threshold bars
    let half = r.width_m * 0.5 - 1.0;
    let (nx, nz) = (-uz, ux);
    for side in [-1.0f32, 1.0] {
        let off = [nx * half * side, nz * half * side];
        strip(
            [a[0] + off[0] + ux * 6.0, a[1] + off[1] + uz * 6.0],
            [b[0] + off[0] - ux * 6.0, b[1] + off[1] - uz * 6.0],
            0.4,
            -0.01,
            &mut marks,
        );
    }
    for (end, dir) in [(a, 1.0f32), (b, -1.0)] {
        let p = [end[0] + ux * dir * 8.0, end[1] + uz * dir * 8.0];
        let q = [end[0] + ux * dir * 20.0, end[1] + uz * dir * 20.0];
        // a row of short bars across the runway width
        let bars = ((r.width_m - 4.0) / 3.0).max(2.0) as i32;
        for i in 0..bars {
            let t = (i as f32 + 0.5) / bars as f32 - 0.5;
            let c = [nx * t * (r.width_m - 4.0), nz * t * (r.width_m - 4.0)];
            strip(
                [p[0] + c[0], p[1] + c[1]],
                [q[0] + c[0], q[1] + c[1]],
                1.1,
                -0.01,
                &mut marks,
            );
        }
    }
    layers.push(Layer {
        kind: Kind::Marking,
        color: MARKING_COLOR,
        triangles: marks,
    });
}

/// All layers of an airport in draw order, plus the frame used. The frame is centred on the
/// `datum_lat`/`datum_lon` metadata when present, else on the first runway's midpoint.
pub fn airport_layers(
    airport: &Airport,
    ground_half_size: f32,
) -> Result<(Frame, Vec<Layer>), String> {
    let datum = airport
        .metadata_value("datum_lat")
        .zip(airport.metadata_value("datum_lon"))
        .and_then(|(a, b)| Some((a.parse::<f64>().ok()?, b.parse::<f64>().ok()?)));
    let (lat0, lon0) = match (datum, airport.runways.first()) {
        (Some(d), _) => d,
        (None, Some(r)) => (
            (r.ends[0].lat + r.ends[1].lat) * 0.5,
            (r.ends[0].lon + r.ends[1].lon) * 0.5,
        ),
        (None, None) => {
            return Err(format!(
                "airport {} has no datum and no runways",
                airport.id
            ));
        }
    };
    let frame = Frame::new(lat0, lon0);
    let h = ground_half_size;
    let mut ground = Vec::new();
    quad([-h, -h], [h, -h], [h, h], [-h, h], -0.20, &mut ground);
    let mut layers = vec![Layer {
        kind: Kind::Ground,
        color: GROUND_COLOR,
        triangles: ground,
    }];
    for p in &airport.pavements {
        pavement_layers(p, &frame, &mut layers);
    }
    for r in &airport.runways {
        runway_layers(r, &frame, &mut layers);
    }
    Ok((frame, layers))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area_of(poly: &[[f32; 2]], tris: &[[usize; 3]]) -> f32 {
        tris.iter()
            .map(|t| signed_area(&[poly[t[0]], poly[t[1]], poly[t[2]]]).abs())
            .sum()
    }

    #[test]
    fn projection_is_metric_and_oriented() {
        let f = Frame::new(47.0, -122.0);
        let origin = f.to_local(47.0, -122.0);
        assert_eq!(origin, [0.0, 0.0]);
        // one degree north is ~111.2 km, towards -z; east is +x and shrinks with cos(lat)
        let n = f.to_local(48.0, -122.0);
        assert!((n[1] + 111_194.9).abs() < 5.0 && n[0].abs() < 1e-3);
        let e = f.to_local(47.0, -121.0);
        assert!((e[0] - 111_194.9 * 47f32.to_radians().cos()).abs() < 5.0 && e[1].abs() < 1e-3);
    }

    #[test]
    fn triangulates_convex_and_concave_polygons_in_either_winding() {
        let square = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        assert_eq!(area_of(&square, &triangulate(&square)), 100.0);
        let rev: Vec<_> = square.iter().rev().copied().collect();
        assert_eq!(area_of(&rev, &triangulate(&rev)), 100.0);
        // L shape, area 100 - 25
        let l = [
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 5.0],
            [5.0, 5.0],
            [5.0, 10.0],
            [0.0, 10.0],
        ];
        let tris = triangulate(&l);
        assert_eq!(tris.len(), 4);
        assert!((area_of(&l, &tris) - 75.0).abs() < 1e-3);
        assert!(triangulate(&square[..2]).is_empty());
    }

    #[test]
    fn bezier_nodes_add_curve_samples_and_plain_loops_stay_plain() {
        let f = Frame::new(0.0, 0.0);
        let plain = [
            Node {
                lat: 0.0,
                lon: 0.0,
                ctrl: None,
            },
            Node {
                lat: 0.0,
                lon: 0.001,
                ctrl: None,
            },
            Node {
                lat: 0.001,
                lon: 0.001,
                ctrl: None,
            },
        ];
        assert_eq!(flatten_loop(&plain, &f).len(), 3);
        let curved = [
            Node {
                lat: 0.0,
                lon: 0.0,
                ctrl: Some((0.0005, 0.0)),
            },
            Node {
                lat: 0.0,
                lon: 0.001,
                ctrl: None,
            },
            Node {
                lat: 0.001,
                lon: 0.001,
                ctrl: None,
            },
        ];
        let poly = flatten_loop(&curved, &f);
        // both segments touching the curved node are sampled, the plain one stays a single point
        assert_eq!(poly.len(), 2 * BEZIER_STEPS + 1);
        assert_eq!(poly[0], f.to_local(0.0, 0.0));
    }

    fn sample_airport() -> Airport {
        let text = [
            "I",
            "1200 t",
            "",
            "1 0 0 0 TEST Test Field",
            "1302 datum_lat 47.0",
            "1302 datum_lon -122.0",
            "100 30.00 1 0 0.25 0 1 0 09 47.0 -122.01 0 0 3 0 0 0 27 47.0 -121.99 0 0 3 0 0 0",
            "110 2 0.25 0 apron",
            "111 47.001 -122.0",
            "111 47.001 -121.999",
            "111 47.002 -121.999",
            "113 47.002 -122.0",
            "99",
        ]
        .join("\n");
        crate::apt::find_airport(text.as_bytes(), "TEST")
            .unwrap()
            .unwrap()
    }

    #[test]
    fn airport_layers_have_ground_pavement_runway_and_markings() {
        let (frame, layers) = airport_layers(&sample_airport(), 500.0).unwrap();
        let kinds: Vec<Kind> = layers.iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            [Kind::Ground, Kind::Pavement, Kind::Runway, Kind::Marking]
        );
        assert_eq!(frame.to_local(47.0, -122.0), [0.0, 0.0]);
        let ground = &layers[0];
        assert_eq!(ground.triangles.len(), 6);
        // runway body is one 30 m x ~1.5 km strip: two triangles
        assert_eq!(layers[2].triangles.len(), 6);
        assert_eq!(layers[1].color, surface_color(2));
        assert!(layers[3].triangles.len() > 30);
        assert!(layers.iter().all(|l| l.triangles.len() % 3 == 0));
    }

    #[test]
    fn airport_without_datum_or_runways_is_an_error() {
        let text = "I\n1200 t\n1 0 0 0 NONE None\n99\n";
        let a = crate::apt::find_airport(text.as_bytes(), "NONE")
            .unwrap()
            .unwrap();
        assert!(airport_layers(&a, 100.0).is_err());
    }
}
