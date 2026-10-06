//! Lifting surfaces read from the ACF: typed wing parameters and the geometry of their span elements.
//!
//! Units and fields follow the ACF loader schema (research/ACF_SCHEMA.md): lengths in feet are
//! converted with the reference build's float32 constant, angles stay in degrees. The body frame is
//! the OBJ frame: x right, y up, z aft (the nose points to -z). `_part_x/y/z` is the quarter-chord
//! point of the root (checked against the Cessna's `wings.obj`: the root chord spans z 0.56..2.17 m and
//! `_part_z` is 0.975 m, 26% of the chord). Treating `_semilen_SEG` as the length along the span line,
//! sweep as the quarter-chord sweep and the chord as linear root-to-tip are openXplane simplifications.
use crate::{Aircraft, aircraft::ACF_FT_TO_M};
use glam::Vec3;

/// A control surface deflection shared by the elements that carry it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Aileron,
    Elevator,
    Rudder,
    Flap,
}

#[derive(Clone, Debug)]
pub struct Wing {
    pub index: usize,
    /// Quarter-chord point of the root, body frame, metres.
    pub root: Vec3,
    pub root_chord: f32,
    pub tip_chord: f32,
    pub semilen: f32,
    pub dihedral_deg: f32,
    pub sweep_deg: f32,
    /// `_part_the`: pitch of the whole surface (incidence), degrees.
    pub incidence_deg: f32,
    /// `_is_right_mult`: +1 for the right half, -1 for the left half.
    pub side: f32,
    pub elements: usize,
    pub airfoils: [String; 3],
    pub foil_ratios: [f32; 4],
    /// Per element: which controls it carries (1.0 = full span of the element).
    pub controls: Vec<[f32; 4]>,
}

/// One span element of a wing in the body frame.
#[derive(Clone, Copy, Debug)]
pub struct Element {
    pub wing: usize,
    pub index: usize,
    /// Quarter-chord point of the element centre.
    pub center: Vec3,
    pub chord: f32,
    pub area: f32,
    /// Unit vector from the leading to the trailing edge.
    pub trailing: Vec3,
    /// Unit surface normal (upper side).
    pub normal: Vec3,
    /// Unit vector along the span to the right-hand side of the aircraft.
    pub span_right: Vec3,
    /// Fraction of the span from the root, 0..1, at the element centre.
    pub span_fraction: f32,
}

fn float(acf: &Aircraft, key: &str) -> Result<f32, String> {
    acf.float_property(key)
}

fn int(acf: &Aircraft, key: &str) -> Result<i32, String> {
    let p = acf.unique_property(key)?;
    p.value
        .parse()
        .map_err(|_| format!("invalid integer {key} at line {}", p.line))
}

impl Wing {
    /// The wings that exist in the ACF: those with a positive span and root chord.
    pub fn all_from_acf(acf: &Aircraft) -> Result<Vec<Wing>, String> {
        let mut wings = Vec::new();
        for index in 0..32 {
            let key = |k: &str| format!("_wing/{index}/{k}");
            let Ok(semilen) = float(acf, &key("_semilen_SEG")) else {
                continue;
            };
            let root_chord = float(acf, &key("_Croot"))?;
            if semilen <= 0.0 || root_chord <= 0.0 {
                continue;
            }
            let elements = int(acf, &key("_els"))?;
            if !(1..=100).contains(&elements) {
                return Err(format!(
                    "wing {index}: unsupported element count {elements}"
                ));
            }
            let elements = elements as usize;
            let ft = |v: f32| v * ACF_FT_TO_M;
            let mut controls = vec![[0.0f32; 4]; elements];
            for (slot, name) in ["_ailn1", "_elev1", "_rudd1", "_flap1"].iter().enumerate() {
                for (i, c) in controls.iter_mut().enumerate() {
                    if let Ok(p) = acf.unique_property(&key(&format!("{name}/{i}"))) {
                        c[slot] = p.value.parse().unwrap_or(0.0);
                    }
                }
            }
            let airfoil = |n: u32| -> Result<String, String> {
                Ok(acf
                    .unique_property(&key(&format!("_afl_file_{n}")))?
                    .value
                    .clone())
            };
            wings.push(Wing {
                index,
                root: Vec3::new(
                    ft(float(acf, &key("_part_x"))?),
                    ft(float(acf, &key("_part_y"))?),
                    ft(float(acf, &key("_part_z"))?),
                ),
                root_chord: ft(root_chord),
                tip_chord: ft(float(acf, &key("_Ctip"))?),
                semilen: ft(semilen),
                dihedral_deg: float(acf, &key("_dihed_design"))?,
                sweep_deg: float(acf, &key("_sweep_design"))?,
                incidence_deg: float(acf, &key("_part_the")).unwrap_or(0.0),
                side: if float(acf, &key("_is_right_mult"))? < 0.0 {
                    -1.0
                } else {
                    1.0
                },
                elements,
                airfoils: [airfoil(1)?, airfoil(2)?, airfoil(3)?],
                foil_ratios: [
                    float(acf, &key("_foil_rat_rot"))?,
                    float(acf, &key("_foil_rat_mid_inner"))?,
                    float(acf, &key("_foil_rat_mid_outer"))?,
                    float(acf, &key("_foil_rat_tip"))?,
                ],
                controls,
            });
        }
        Ok(wings)
    }

    /// Planform area of the surface (trapezoid), m².
    pub fn area(&self) -> f32 {
        0.5 * (self.root_chord + self.tip_chord) * self.semilen
    }

    /// Unit vector along the span from the root to the tip, body frame.
    pub fn span_direction(&self) -> Vec3 {
        let d = self.dihedral_deg.to_radians();
        Vec3::new(
            self.side * d.cos(),
            d.sin(),
            self.sweep_deg.to_radians().tan(),
        )
        .normalize()
    }

    pub fn element(&self, index: usize) -> Element {
        let n = self.elements as f32;
        let s0 = index as f32 / n;
        let s1 = (index + 1) as f32 / n;
        let sc = 0.5 * (s0 + s1);
        let c0 = self.root_chord + (self.tip_chord - self.root_chord) * s0;
        let c1 = self.root_chord + (self.tip_chord - self.root_chord) * s1;
        let chord = 0.5 * (c0 + c1);
        let u = self.span_direction();
        // direction to the right-hand side along the span (flips for left halves)
        let span_right = if self.side < 0.0 { -u } else { u };
        let pitch = self.incidence_deg.to_radians();
        // leading-to-trailing direction, pitched up by the incidence (leading edge up)
        let trailing = Vec3::new(0.0, -pitch.sin(), pitch.cos());
        let normal = span_right.cross(-trailing).normalize();
        Element {
            wing: self.index,
            index,
            center: self.root + u * (self.semilen * sc),
            chord,
            area: chord * self.semilen / n,
            trailing,
            normal,
            span_right,
            span_fraction: sc,
        }
    }

    pub fn elements(&self) -> impl Iterator<Item = Element> + '_ {
        (0..self.elements).map(|i| self.element(i))
    }

    /// Tip point of the span line, body frame.
    pub fn tip(&self) -> Vec3 {
        self.root + self.span_direction() * self.semilen
    }
}

/// Aspect ratio of the lifting surface each wing segment belongs to. Segments are grouped when one
/// starts where another ends (root to tip, within 5 cm) or when they are mirror images of each other
/// across the aircraft's centre plane. A horizontal group has span (its lateral extent)^2 / area; a
/// vertical one (dihedral over 45 degrees) is treated as half of a mirrored surface:
/// 2 * height^2 / area. This is the aspect ratio of the whole surface a segment is part of, which is
/// what the finite-wing lift slope needs, not the ratio of an inner or outer segment alone.
pub fn aspect_ratios(wings: &[Wing]) -> Vec<f32> {
    let n = wings.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut Vec<usize>, i: usize) -> usize {
        if p[i] != i {
            let r = find(p, p[i]);
            p[i] = r;
        }
        p[i]
    }
    let near = |a: Vec3, b: Vec3| (a - b).length() < 0.05;
    for a in 0..n {
        for b in (a + 1)..n {
            let (wa, wb) = (&wings[a], &wings[b]);
            let chained = near(wa.tip(), wb.root) || near(wb.tip(), wa.root);
            let mirrored = wa.side != wb.side
                && near(Vec3::new(-wa.root.x, wa.root.y, wa.root.z), wb.root)
                && (wa.dihedral_deg - wb.dihedral_deg).abs() < 1e-3
                && wa.airfoils == wb.airfoils;
            if chained || mirrored {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                parent[ra] = rb;
            }
        }
    }
    let mut out = vec![0.0; n];
    for (a, slot) in out.iter_mut().enumerate() {
        let root = find(&mut parent, a);
        let group: Vec<&Wing> = (0..n)
            .filter(|&b| find(&mut parent, b) == root)
            .map(|b| &wings[b])
            .collect();
        let area: f32 = group.iter().map(|w| w.area()).sum();
        let vertical = group.iter().all(|w| w.dihedral_deg.abs() > 45.0);
        let points = group.iter().flat_map(|w| [w.root, w.tip()]);
        let ar = if vertical {
            let (lo, hi) = points.fold((f32::INFINITY, f32::NEG_INFINITY), |(l, h), p| {
                (l.min(p.y), h.max(p.y))
            });
            2.0 * (hi - lo) * (hi - lo) / area
        } else {
            let (lo, hi) = points.fold((f32::INFINITY, f32::NEG_INFINITY), |(l, h), p| {
                (l.min(p.x), h.max(p.x))
            });
            (hi - lo) * (hi - lo) / area
        };
        *slot = if ar.is_finite() { ar } else { 0.0 };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acf(text: &str) -> Aircraft {
        Aircraft::parse(&format!(
            "I\n1200 Version\nACF\nPROPERTIES_BEGIN\n{text}PROPERTIES_END\n"
        ))
        .unwrap()
    }

    fn wing_text(n: usize, x: f32, side: f32, dihed: f32, sweep: f32) -> String {
        let mut s = String::new();
        for (k, v) in [
            ("_semilen_SEG", "10"),
            ("_Croot", "5"),
            ("_Ctip", "3"),
            ("_els", "4"),
            ("_dihed_design", &dihed.to_string()),
            ("_sweep_design", &sweep.to_string()),
            ("_part_y", "2"),
            ("_part_z", "3"),
            ("_foil_rat_rot", "0.1"),
            ("_foil_rat_mid_inner", "0.2"),
            ("_foil_rat_mid_outer", "0.8"),
            ("_foil_rat_tip", "0.9"),
            ("_afl_file_1", "A.afl"),
            ("_afl_file_2", "B.afl"),
            ("_afl_file_3", "C.afl"),
            ("_ailn1/1", "1"),
            ("_ailn1/2", "1"),
        ] {
            s += &format!("P _wing/{n}/{k} {v}\n");
        }
        s += &format!("P _wing/{n}/_part_x {x}\nP _wing/{n}/_is_right_mult {side}\n");
        s
    }

    #[test]
    fn reads_surfaces_in_metres_and_skips_missing_ones() {
        let text = wing_text(0, -2.0, -1.0, 0.0, 0.0) + &wing_text(3, 2.0, 1.0, 0.0, 0.0);
        let wings = Wing::all_from_acf(&acf(&text)).unwrap();
        assert_eq!(wings.len(), 2);
        assert_eq!(wings[1].index, 3);
        assert!((wings[1].semilen - 10.0 * ACF_FT_TO_M).abs() < 1e-5);
        assert!((wings[0].root.x + 2.0 * ACF_FT_TO_M).abs() < 1e-5);
        assert_eq!(wings[0].side, -1.0);
        assert_eq!(wings[1].airfoils[2], "C.afl");
        assert_eq!(wings[1].controls[1][0], 1.0);
        assert_eq!(wings[1].controls[0][0], 0.0);
    }

    #[test]
    fn element_normals_point_up_on_both_halves_and_chord_tapers() {
        let text = wing_text(0, -2.0, -1.0, 0.0, 0.0) + &wing_text(1, 2.0, 1.0, 0.0, 0.0);
        for w in Wing::all_from_acf(&acf(&text)).unwrap() {
            for e in w.elements() {
                assert!((e.normal - Vec3::Y).length() < 1e-6, "{:?}", e.normal);
                assert!((e.trailing - Vec3::Z).length() < 1e-6);
            }
            let (a, b) = (w.element(0), w.element(3));
            assert!(a.chord > b.chord);
            let total: f32 = w.elements().map(|e| e.area).sum();
            assert!((total - w.area()).abs() < 1e-4);
            // elements run outwards from the root on the correct side
            assert_eq!((b.center.x - w.root.x).signum(), w.side);
        }
    }

    #[test]
    fn dihedral_lifts_the_tip_sweep_moves_it_aft_and_vertical_surfaces_stand_up() {
        let flat = &Wing::all_from_acf(&acf(&wing_text(1, 2.0, 1.0, 0.0, 0.0))).unwrap()[0];
        let dihed = &Wing::all_from_acf(&acf(&wing_text(1, 2.0, 1.0, 10.0, 0.0))).unwrap()[0];
        let swept = &Wing::all_from_acf(&acf(&wing_text(1, 2.0, 1.0, 0.0, 20.0))).unwrap()[0];
        assert!(dihed.element(3).center.y > flat.element(3).center.y);
        assert!(swept.element(3).center.z > flat.element(3).center.z);
        let fin = &Wing::all_from_acf(&acf(&wing_text(1, 0.0, 1.0, 90.0, 0.0))).unwrap()[0];
        let top = fin.element(3);
        assert!(top.center.y > fin.root.y + 1.0 && top.center.x.abs() < 1e-4);
        assert!(top.normal.x.abs() > 0.99, "a fin's normal is lateral");
    }

    #[test]
    fn incidence_tilts_the_chord_leading_edge_up() {
        let mut w = Wing::all_from_acf(&acf(&wing_text(1, 2.0, 1.0, 0.0, 0.0)))
            .unwrap()
            .remove(0);
        w.incidence_deg = 5.0;
        let e = w.element(0);
        assert!(e.trailing.y < 0.0 && e.normal.z > 0.0);
    }

    #[test]
    fn aspect_ratio_covers_the_whole_surface_not_one_segment() {
        // inner segments at x = +-2 ft, outer ones continue where the inner ones end
        let ft = ACF_FT_TO_M;
        let inner = 10.0 * ft;
        let mut text = wing_text(0, -2.0, -1.0, 0.0, 0.0) + &wing_text(1, 2.0, 1.0, 0.0, 0.0);
        text += &wing_text(2, -(2.0 + 10.0), -1.0, 0.0, 0.0);
        text += &wing_text(3, 2.0 + 10.0, 1.0, 0.0, 0.0);
        let wings = Wing::all_from_acf(&acf(&text)).unwrap();
        let ar = aspect_ratios(&wings);
        let area: f32 = wings.iter().map(|w| w.area()).sum();
        let span = 2.0 * (2.0 * ft + 2.0 * inner);
        assert!((ar[0] - span * span / area).abs() < 0.05, "{ar:?}");
        assert!(ar.iter().all(|a| (a - ar[0]).abs() < 1e-4));
        // a fin: vertical, single, mirrored by the ground plane
        let fin = Wing::all_from_acf(&acf(&wing_text(5, 0.0, 1.0, 90.0, 0.0))).unwrap();
        let h = fin[0].semilen;
        assert!((aspect_ratios(&fin)[0] - 2.0 * h * h / fin[0].area()).abs() < 0.05);
    }

    #[test]
    fn rejects_unsupported_element_counts() {
        let text = wing_text(0, 1.0, 1.0, 0.0, 0.0).replace("_els 4", "_els 0");
        assert!(Wing::all_from_acf(&acf(&text)).is_err());
    }
}
