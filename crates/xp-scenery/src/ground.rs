//! Local physical-triangle height queries. Port-side spatial index and vertical
//! interpolation, not the original terrain collision implementation.
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Ground {
    triangles: Vec<[[f32; 3]; 3]>,
    cells: BTreeMap<(i32, i32), Vec<usize>>,
    base: Option<std::sync::Arc<Ground>>,
    wide: Vec<usize>,
}

impl Ground {
    /// Source terrain beneath added airport surfaces, for visual layer draping.
    pub fn terrain_sample(&self, x: f32, z: f32) -> Option<(f32, [f32; 3])> {
        self.base
            .as_ref()
            .map_or_else(|| self.sample(x, z), |g| g.terrain_sample(x, z))
    }
    const CELL: f32 = 256.0;
    pub fn new(triangles: Vec<[[f32; 3]; 3]>) -> Self {
        let mut out = Self {
            triangles,
            cells: BTreeMap::new(),
            base: None,
            wide: Vec::new(),
        };
        for (i, t) in out.triangles.iter().enumerate() {
            let minx = t.iter().map(|v| v[0]).fold(f32::INFINITY, f32::min);
            let maxx = t.iter().map(|v| v[0]).fold(f32::NEG_INFINITY, f32::max);
            let minz = t.iter().map(|v| v[2]).fold(f32::INFINITY, f32::min);
            let maxz = t.iter().map(|v| v[2]).fold(f32::NEG_INFINITY, f32::max);
            if t.iter().flatten().any(|v| !v.is_finite()) {
                continue;
            }
            let cell = |v: f32| (v / Self::CELL).floor() as i32;
            let count_x = i64::from(cell(maxx)) - i64::from(cell(minx)) + 1;
            let count_z = i64::from(cell(maxz)) - i64::from(cell(minz)) + 1;
            // Keep giant triangles queryable without expanding an unbounded
            // grid from malformed or unusually broad input coordinates.
            if count_x.saturating_mul(count_z) > 65536 {
                out.wide.push(i);
                continue;
            }
            for x in cell(minx)..=cell(maxx) {
                for z in cell(minz)..=cell(maxz) {
                    out.cells.entry((x, z)).or_default().push(i);
                }
            }
        }
        out
    }

    pub fn with_surfaces(base: std::sync::Arc<Ground>, triangles: Vec<[[f32; 3]; 3]>) -> Self {
        let mut out = Self::new(triangles);
        out.base = Some(base);
        out
    }

    /// Highest physical surface at x/z, with an upward normalized geometric
    /// normal. Missing coverage is None rather than extrapolated terrain.
    pub fn sample(&self, x: f32, z: f32) -> Option<(f32, [f32; 3])> {
        if !x.is_finite() || !z.is_finite() {
            return None;
        }
        let cell = (
            (x / Self::CELL).floor() as i32,
            (z / Self::CELL).floor() as i32,
        );
        let mut best = self.base.as_ref().and_then(|g| g.sample(x, z));
        for &i in self
            .cells
            .get(&cell)
            .into_iter()
            .flatten()
            .chain(&self.wide)
        {
            let [a, b, c] = self.triangles[i];
            let (ux, uz, vx, vz) = (b[0] - a[0], b[2] - a[2], c[0] - a[0], c[2] - a[2]);
            let det = ux * vz - uz * vx;
            if det.abs() < 1e-6 || !det.is_finite() {
                continue;
            }
            let u = ((x - a[0]) * vz - (z - a[2]) * vx) / det;
            let v = (ux * (z - a[2]) - uz * (x - a[0])) / det;
            if u < -1e-5 || v < -1e-5 || u + v > 1.00001 {
                continue;
            }
            let y = a[1] + u * (b[1] - a[1]) + v * (c[1] - a[1]);
            let uy = b[1] - a[1];
            let vy = c[1] - a[1];
            let mut n = [uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx];
            if n[1] < 0.0 {
                for v in &mut n {
                    *v = -*v;
                }
            }
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            for v in &mut n {
                *v /= len;
            }
            if best.as_ref().is_none_or(|(h, _)| y > *h) {
                best = Some((y, n));
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_cells_edges_slopes_and_overlapping_surfaces() {
        let tri = [
            [-300.0, 0.0, -300.0],
            [-100.0, 20.0, -300.0],
            [-300.0, 0.0, -100.0],
        ];
        let g = Ground::new(vec![tri]);
        let (h, n) = g.sample(-250.0, -250.0).unwrap();
        assert!((h - 5.0).abs() < 1e-5);
        assert!(n[1] > 0.99 && n[0] < 0.0);
        assert_eq!(g.sample(-100.0, -300.0).unwrap().0, 20.0);
        assert!(g.sample(0.0, 0.0).is_none());
        let top = tri.map(|mut p| {
            p[1] += 10.0;
            p
        });
        assert_eq!(
            Ground::new(vec![tri, top])
                .sample(-250.0, -250.0)
                .unwrap()
                .0,
            15.0
        );
        let base = std::sync::Arc::new(Ground::new(vec![tri]));
        let surfaces = Ground::with_surfaces(
            base,
            vec![[
                [-300.0, 50.0, -300.0],
                [-250.0, 50.0, -300.0],
                [-300.0, 50.0, -250.0],
            ]],
        );
        assert_eq!(surfaces.sample(-290.0, -290.0).unwrap().0, 50.0);
        assert!((surfaces.sample(-200.0, -280.0).unwrap().0 - 10.0).abs() < 1e-5);
    }

    #[test]
    fn broad_triangles_remain_queryable_without_grid_expansion() {
        let g = Ground::new(vec![[
            [0.0, 4.0, 0.0],
            [100000.0, 4.0, 0.0],
            [0.0, 4.0, 100000.0],
        ]]);
        assert!(g.cells.is_empty());
        assert_eq!(g.sample(100.0, 100.0).unwrap().0, 4.0);
        assert!(g.sample(-10.0, -10.0).is_none());
    }
}
