//! An approximate rigid-body flight model built from the ACF and the airfoil tables.
//!
//! This is NOT the reference build's flight model. What comes from the original: the geometry and
//! mass data of the ACF, the airfoil lookup/stall/blending of `profile::evaluate` (verified bit for
//! bit, research/VERIFICATION.md), the three-airfoil span weights and the finite-wing factor
//! structure read from `0x1411b6630` (research/WING_ELEMENT.md). What is openXplane's own
//! simplification: no wind, a flat ground at height 0, a momentum-style propeller, the control
//! surface effectiveness, the fuselage drag area, published Cessna 172 inertias scaled by mass,
//! and a synthetic stall-buffet noise table. Treat results as plausible, not as agreement with
//! X-Plane.
use glam::{Mat3, Quat, Vec3};
use std::{collections::BTreeMap, path::Path};
use xp_acf::{
    Aircraft,
    aircraft::{ACF_FT_TO_M, AircraftParameters},
    wing::{Element, Wing, aspect_ratios},
};
use xp_airfoil::{
    airfoil::Airfoil,
    buffet::{NoiseTable, TABLE_LEN},
    profile::{self, OuterInputs},
    runtime::RunningTime,
    wing_element::{
        self, Aircraft as ElementAircraft, ElementInputs, ElementState, Flow, FoilCall, FoilResult,
        WingFields,
    },
};
use xp_scenery::install::Install;

const G: f32 = 9.80665;
const LB_TO_N: f32 = 4.448_222;
const HP_TO_W: f32 = 745.7;
const SEA_LEVEL_DENSITY: f32 = 1.225;
/// Published Cessna 172 inertias (kg m^2) at 1043 kg: about the lateral (pitch), longitudinal (roll)
/// and vertical (yaw) axes. Scaled with mass.
const INERTIA_REF: (f32, f32, f32, f32) = (1043.0, 1825.0, 1285.0, 2667.0);
/// Assumed equivalent flat-plate drag area of fuselage, struts' fittings, landing gear and cowling.
const PARASITE_AREA: f32 = 0.30;
/// Assumed static thrust of the fixed-pitch propeller at sea level and full power, newtons.
const STATIC_THRUST: f32 = 2400.0;
const PROP_EFFICIENCY: f32 = 0.78;
/// Speed offset in the thrust law T = min(static, eta P / (V + V_C)), m/s.
const PROP_SPEED_OFFSET: f32 = 22.0;
/// Assumed effectiveness of a plain control surface relative to thin-airfoil theory.
const CONTROL_EFFECTIVENESS: f32 = 0.65;
/// Scale of the textbook downwash angle 2 CL / (pi AR) at the tail (typically dEps/dAlpha of 0.4-0.5).
const DOWNWASH_FACTOR: f32 = 1.2;
/// Assumed destabilising pitching-moment slope of the fuselage per radian of angle of attack,
/// referenced to the main wing area and mean chord (the lifting surfaces alone are too stable).
const FUSELAGE_CM_ALPHA: f32 = 0.5;

#[derive(Clone, Copy, Debug, Default)]
pub struct Controls {
    /// Stick back positive (nose up): -1..1.
    pub elevator: f32,
    /// Stick right positive (roll right): -1..1.
    pub aileron: f32,
    /// Right pedal positive: -1..1.
    pub rudder: f32,
    /// 0..1.
    pub throttle: f32,
    /// 0..1 wheel brakes.
    pub brake: f32,
    /// Flap handle 0..1 (up to the largest ACF detent).
    pub flaps: f32,
    /// Trim added to the matching stick axis, -1..1.
    pub elevator_trim: f32,
    pub aileron_trim: f32,
    pub rudder_trim: f32,
}

#[derive(Clone, Debug)]
pub struct Gear {
    /// Contact point with the strut fully extended, body frame.
    pub tip: Vec3,
    pub stiffness: f32,
    pub damping: f32,
    pub brakes: bool,
    pub steer_max_rad: f32,
    pub compression: f32,
    pub on_ground: bool,
}

/// Boundary points and per-element flags of one wing in the layout the wing element function reads.
struct WingGeom {
    x: Vec<f32>,
    y: Vec<f32>,
    z: Vec<f32>,
    chord: Vec<f32>,
    flap_flags: Vec<i32>,
    slat_flags: Vec<i32>,
    names: [String; 3],
    ratios: [f32; 4],
    elements: i32,
    ar: f32,
    side: f32,
    /// A swept wing whose sweep passes 40 degrees would select the unported delta-wing block.
    swept: bool,
}

struct ElementData {
    element: Element,
    /// Index into `FlightModel::geoms`.
    geom: usize,
    foil_ids: [usize; 3],
    /// `parameters[4]` of the middle airfoil's first table: the angle limit the flap factor scales.
    g10: f32,
    /// aileron, elevator, rudder, flap: fraction of the element carrying the surface.
    controls: [f32; 4],
    control_chord: [f32; 4],
    side: f32,
    stalled: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    /// Centre of gravity, world frame (x east, y up, z south), metres.
    pub position: Vec3,
    pub velocity: Vec3,
    /// Body to world.
    pub orientation: Quat,
    /// Body frame, rad/s.
    pub omega: Vec3,
    pub time: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Telemetry {
    pub airspeed_kt: f32,
    pub altitude_ft: f32,
    pub vertical_speed_fpm: f32,
    pub pitch_deg: f32,
    pub roll_deg: f32,
    pub heading_deg: f32,
    pub alpha_deg: f32,
    pub throttle: f32,
    pub on_ground: bool,
    pub stalled_elements: usize,
}

pub struct FlightModel {
    pub mass: f32,
    pub inertia: Vec3,
    pub cg: Vec3,
    pub state: State,
    pub gears: Vec<Gear>,
    pub elevation_m: f32,
    wings: Vec<Wing>,
    elements: Vec<ElementData>,
    geoms: Vec<WingGeom>,
    airfoils: Vec<Airfoil>,
    noise: NoiseTable,
    max_power_w: f32,
    controls_range: [(f32, f32); 4],
    flap_detents: Vec<f32>,
    rest_cg_height: f32,
    /// Per wing segment: (index, force, moment about the CG), body frame, from the last force evaluation.
    pub wing_report: Vec<(usize, Vec3, Vec3)>,
    /// Area-weighted lift coefficient of the surfaces ahead of the tail at the last evaluation.
    downwash_cl: f32,
    main_wing_ar: f32,
    reference_area: f32,
    reference_chord: f32,
    rest: Option<(State, Vec<Gear>)>,
}

/// A simple scripted pilot for tests and demonstrations: full power after two seconds, rotation at 55
/// knots to 11 degrees of pitch below 30 feet and 8 degrees above, with pitch-rate damping.
pub fn takeoff_pilot(model: &FlightModel, c: &mut Controls) {
    let t = model.telemetry(c);
    if model.state.time > 2.0 {
        c.throttle = 1.0;
    }
    if t.airspeed_kt > 55.0 {
        let target = if t.altitude_ft < 30.0 { 11.0 } else { 8.0 };
        let rate = model.state.omega.x.to_degrees();
        c.elevator = (0.12 + 0.10 * (target - t.pitch_deg) - 0.06 * rate).clamp(-0.5, 0.9);
    }
}

pub fn isa(height_m: f32) -> (f32, f32) {
    let h = height_m.clamp(-500.0, 11000.0);
    let t = 288.15 - 0.0065 * h;
    let density = SEA_LEVEL_DENSITY * (t / 288.15).powf(4.2559);
    let mu = 1.458e-6 * t.powf(1.5) / (t + 110.4);
    (density, mu)
}

fn synthetic_noise() -> NoiseTable {
    let values = (0..TABLE_LEN as u64)
        .map(|i| {
            let h = ((i * 2_654_435_761) & 0xffff_ffff) >> 8;
            (h as f32 / 16_777_216.0) * 2.0 - 1.0
        })
        .collect();
    NoiseTable::new(values).expect("fixed length")
}

fn control_deflection(
    control: usize,
    side: f32,
    c: &Controls,
    range: (f32, f32),
    flap_deg: f32,
) -> f32 {
    // range = (up, down) limits in degrees; the result is positive for the trailing edge down.
    let (up, down) = range;
    let signed = |s: f32| if s >= 0.0 { -s * up } else { -s * down };
    let sum = |stick: f32, trim: f32| (stick + trim).clamp(-1.0, 1.0);
    match control {
        0 => signed(sum(c.aileron, c.aileron_trim) * side),
        1 => signed(sum(c.elevator, c.elevator_trim)),
        2 => signed(-sum(c.rudder, c.rudder_trim)),
        _ => flap_deg,
    }
}

impl FlightModel {
    pub fn load(
        acf_path: &Path,
        install: &Install,
        fuel_kg: f32,
        payload_kg: f32,
    ) -> Result<Self, String> {
        let text = std::fs::read_to_string(acf_path)
            .map_err(|e| format!("{}: {e}", acf_path.display()))?;
        let acf = Aircraft::parse(&text)?;
        let dir = acf_path.parent().ok_or("ACF has no parent directory")?;
        let params = AircraftParameters::from_acf(&acf)?;
        let wings = Wing::all_from_acf(&acf)?;
        if wings.is_empty() {
            return Err("the ACF has no lifting surfaces".into());
        }
        let ars = aspect_ratios(&wings);

        let mut airfoils: Vec<Airfoil> = Vec::new();
        let mut index: BTreeMap<String, usize> = BTreeMap::new();
        let mut load_foil = |airfoils: &mut Vec<Airfoil>, name: &str| -> Result<usize, String> {
            if let Some(&i) = index.get(name) {
                return Ok(i);
            }
            let path = install
                .airfoil_candidates(dir, name)
                .into_iter()
                .find(|p| p.is_file())
                .ok_or_else(|| format!("airfoil not found: {name}"))?;
            let foil = Airfoil::parse(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            airfoils.push(foil);
            index.insert(name.to_string(), airfoils.len() - 1);
            Ok(airfoils.len() - 1)
        };

        let scalar = |key: &str| acf.float_property(key).unwrap_or(0.0);
        let controls_range = [
            (scalar("acf/_ailn1_up"), scalar("acf/_ailn1_dn")),
            (scalar("acf/_elev1_up"), scalar("acf/_elev1_dn")),
            (
                scalar("acf/_rudd1_up").max(25.0),
                scalar("acf/_rudd1_dn").max(25.0),
            ),
            (0.0, 0.0),
        ];
        let cratio = |name: &str| {
            (
                scalar(&format!("acf/_{name}_cratR")),
                scalar(&format!("acf/_{name}_cratT")),
            )
        };
        let crats = [
            cratio("ailn1"),
            cratio("elev1"),
            cratio("rudd1"),
            cratio("flap1"),
        ];
        let flap_detents: Vec<f32> = (0..12)
            .map(|i| scalar(&format!("acf/_flap1_dn/{i}")))
            .filter(|d| *d > 0.0)
            .collect();

        let mut elements = Vec::new();
        let mut geoms = Vec::new();
        for (w, ar) in wings.iter().zip(&ars) {
            let names = [
                w.airfoils[0].as_str(),
                w.airfoils[1].as_str(),
                w.airfoils[2].as_str(),
            ];
            let foil_ids = [
                load_foil(&mut airfoils, names[0])?,
                load_foil(&mut airfoils, names[1])?,
                load_foil(&mut airfoils, names[2])?,
            ];
            // `parameters[4]` of the middle airfoil's first table (the original reads it at +0x10 of
            // the first table of the middle airfoil)
            let g10 = airfoils[foil_ids[1]]
                .polars
                .first()
                .map_or(1.0, |p| p.parameters[4]);
            // boundary points of the span elements; z carries a quarter chord so that z - chord/4 is the
            // quarter-chord line the sweep is measured on
            let n = w.elements;
            let u = w.span_direction();
            let (mut x, mut y, mut z, mut chord) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
            for i in 0..=n {
                let t = i as f32 / n as f32;
                let c = w.root_chord + (w.tip_chord - w.root_chord) * t;
                let p = w.root + u * (w.semilen * t);
                x.push(p.x);
                y.push(p.y);
                z.push(p.z + 0.25 * c);
                chord.push(c);
            }
            let mut flap_flags = vec![0i32; n + 1];
            for (i, c) in w.controls.iter().enumerate() {
                flap_flags[i] = i32::from(c[3] > 0.0);
            }
            geoms.push(WingGeom {
                x,
                y,
                z,
                chord,
                flap_flags,
                slat_flags: vec![0; n + 1],
                names: [
                    names[0].to_string(),
                    names[1].to_string(),
                    names[2].to_string(),
                ],
                ratios: w.foil_ratios,
                elements: n as i32,
                ar: (*ar).max(0.5),
                side: w.side,
                swept: w.sweep_deg.abs() > 35.0,
            });
            let geom = geoms.len() - 1;
            for e in w.elements() {
                let mut ctrl_chord = [0.0; 4];
                for (c, (r, t)) in ctrl_chord.iter_mut().zip(crats) {
                    *c = r + (t - r) * e.span_fraction;
                }
                elements.push(ElementData {
                    element: e,
                    geom,
                    foil_ids,
                    g10,
                    controls: w.controls[e.index],
                    control_chord: ctrl_chord,
                    side: w.side,
                    stalled: false,
                });
            }
        }

        let mut gears = Vec::new();
        for i in 0..10 {
            let key = |k: &str| format!("_gear/{i}/{k}");
            let Ok(leg) = acf.float_property(&key("_leg_len")) else {
                continue;
            };
            let x = acf.float_property(&key("_gear_x"))? * ACF_FT_TO_M;
            let y = acf.float_property(&key("_gear_y"))? * ACF_FT_TO_M;
            let z = acf.float_property(&key("_gear_z"))? * ACF_FT_TO_M;
            let def = acf.float_property(&key("_strut_max_wgt_def"))?.max(0.01) * ACF_FT_TO_M;
            let force = acf.float_property(&key("_strut_max_wgt_frc"))? * LB_TO_N;
            let damp = acf.float_property(&key("_damp"))? * LB_TO_N / ACF_FT_TO_M;
            if leg <= 0.0 || force <= 0.0 {
                continue;
            }
            gears.push(Gear {
                tip: Vec3::new(x, y - leg * ACF_FT_TO_M, z),
                stiffness: force / def,
                damping: damp,
                brakes: acf.float_property(&key("_gear_brakes")).unwrap_or(0.0) > 0.0,
                steer_max_rad: acf
                    .float_property(&key("_steerdeg_lospeed"))
                    .unwrap_or(0.0)
                    .to_radians()
                    * if x.abs() < 0.01 { 1.0 } else { 0.0 },
                compression: 0.0,
                on_ground: false,
            });
        }
        if gears.is_empty() {
            return Err("the ACF has no landing gear".into());
        }

        let main_ar = wings
            .iter()
            .zip(&ars)
            .max_by(|a, b| a.0.area().total_cmp(&b.0.area()))
            .map_or(1.0, |(_, ar)| (*ar).max(1.0));
        // reference area and mean chord of the main wing: horizontal surfaces ahead of the tail
        let main: Vec<&ElementData> = elements
            .iter()
            .filter(|e| {
                e.element.normal.y.abs() > 0.7
                    && e.element.chord > 0.5
                    && e.element.center.z < params.cg_z_acf_m + 2.0
            })
            .collect();
        let ref_area: f32 = main.iter().map(|e| e.element.area).sum();
        let ref_chord = if ref_area > 0.0 {
            main.iter()
                .map(|e| e.element.chord * e.element.area)
                .sum::<f32>()
                / ref_area
        } else {
            1.0
        };
        let mass =
            params.empty_mass_kg + fuel_kg.clamp(0.0, params.maximum_fuel_mass_kg) + payload_kg;
        let scale = mass / INERTIA_REF.0;
        let inertia = Vec3::new(INERTIA_REF.1, INERTIA_REF.3, INERTIA_REF.2) * scale;
        let power_hp = scalar("_engn/0/_power_max_limit");
        Ok(Self {
            mass,
            inertia, // (x: pitch axis, y: yaw axis, z: roll axis)
            cg: Vec3::new(0.0, params.cg_y_acf_m, params.cg_z_acf_m),
            state: State {
                position: Vec3::new(0.0, 2.0, 0.0),
                velocity: Vec3::ZERO,
                orientation: Quat::IDENTITY,
                omega: Vec3::ZERO,
                time: 0.0,
            },
            gears,
            elevation_m: 0.0,
            wings,
            elements,
            geoms,
            airfoils,
            noise: synthetic_noise(),
            max_power_w: power_hp * HP_TO_W,
            controls_range,
            flap_detents,
            rest_cg_height: 0.0,
            wing_report: Vec::new(),
            downwash_cl: 0.0,
            main_wing_ar: main_ar,
            reference_area: ref_area,
            reference_chord: ref_chord,
            rest: None,
        })
    }

    pub fn wings(&self) -> &[Wing] {
        &self.wings
    }

    /// World position of a body-frame point.
    pub fn to_world(&self, p: Vec3) -> Vec3 {
        self.state.position + self.state.orientation * (p - self.cg)
    }

    /// Height of the CG above the ground when the aircraft rests level on its gear.
    pub fn ground_cg_height(&self) -> f32 {
        self.rest_cg_height
    }

    /// Sets the gear so the aircraft rests level on flat ground at height 0 with its wheel bottoms at
    /// `wheel_bottom_y` in the body frame (for example the lowest vertex of the aircraft model), and
    /// places it there with the body origin above `origin_xz`, nose along `direction`. Each strut's
    /// free length follows from its static load: the loads solve the weight and moment balance for a
    /// tricycle gear (three struts); with another count the weight is shared by stiffness.
    pub fn rest_on_ground(&mut self, wheel_bottom_y: f32, origin_xz: [f32; 2], direction: Vec3) {
        let weight = self.mass * G;
        let loads = static_loads(&self.gears, self.cg, weight);
        for (g, f) in self.gears.iter_mut().zip(&loads) {
            g.tip.y = wheel_bottom_y - f / g.stiffness;
            g.compression = f / g.stiffness;
            g.on_ground = true;
        }
        let nose = Vec3::new(direction.x, 0.0, direction.z).normalize();
        let q = Quat::from_rotation_y((-nose.x).atan2(-nose.z));
        self.rest_cg_height = self.cg.y - wheel_bottom_y;
        let origin = Vec3::new(origin_xz[0], self.rest_cg_height - self.cg.y, origin_xz[1]);
        self.state = State {
            position: origin + q * self.cg,
            velocity: Vec3::ZERO,
            orientation: q,
            omega: Vec3::ZERO,
            time: 0.0,
        };
        self.rest = Some((self.state, self.gears.clone()));
    }

    /// Puts the aircraft back where `rest_on_ground` placed it, at rest.
    pub fn reset(&mut self) {
        if let Some((state, gears)) = &self.rest {
            self.state = *state;
            self.gears = gears.clone();
            for e in &mut self.elements {
                e.stalled = false;
            }
            self.downwash_cl = 0.0;
        }
    }

    /// Gives the aircraft an airspeed along its nose (for starting in the air).
    pub fn set_airspeed(&mut self, speed: f32) {
        self.state.velocity = self.state.orientation * Vec3::NEG_Z * speed;
    }

    fn flap_degrees(&self, handle: f32) -> f32 {
        if self.flap_detents.is_empty() {
            return 0.0;
        }
        // evenly spaced detents: 0, d1, d2, ... with the handle spanning them
        let n = self.flap_detents.len() as f32;
        let x = handle.clamp(0.0, 1.0) * n;
        let i = (x.floor() as usize).min(self.flap_detents.len() - 1);
        let lo = if i == 0 {
            0.0
        } else {
            self.flap_detents[i - 1]
        };
        let hi = self.flap_detents[i];
        lo + (hi - lo) * (x - i as f32).clamp(0.0, 1.0)
    }

    /// Forces on the CG (world frame) and moments about it (body frame).
    fn forces(&mut self, c: &Controls) -> (Vec3, Vec3) {
        let st = self.state;
        let rot = Mat3::from_quat(st.orientation);
        let rot_t = rot.transpose();
        let v_body = rot_t * st.velocity;
        let height = self.elevation_m + st.position.y - self.rest_cg_height;
        let (rho, _) = isa(height);
        let temperature_k = 288.15 - 0.0065 * height.clamp(-500.0, 11000.0);
        let temperature_c = temperature_k - 273.15;
        let sound_speed = 20.046_8 * temperature_k.sqrt();
        let mut force_b = Vec3::ZERO;
        let mut moment_b = Vec3::ZERO;
        let flap_deg = self.flap_degrees(c.flaps);
        self.wing_report.clear();
        let (mut cl_weighted, mut area_sum) = (0.0f32, 0.0f32);
        let time = RunningTime::from_snapshot(st.time)
            .unwrap_or_else(|_| RunningTime::from_snapshot(0.0).unwrap());

        for ed in &mut self.elements {
            let e = ed.element;
            let r = e.center - self.cg;
            let w = -(v_body + st.omega.cross(r));
            let (wt, wn) = (w.dot(e.trailing), w.dot(e.normal));
            let vp = wt.hypot(wn);
            if vp < 0.5 {
                continue;
            }
            // control surfaces change the effective angle of attack
            let mut shift = 0.0;
            for k in 0..4 {
                if ed.controls[k] > 0.0 && ed.control_chord[k] > 0.0 {
                    let cf = ed.control_chord[k].clamp(0.05, 0.6);
                    let theta = (1.0 - 2.0 * cf).acos();
                    let tau = 1.0 - (theta - theta.sin()) / std::f32::consts::PI;
                    let delta = control_deflection(k, ed.side, c, self.controls_range[k], flap_deg);
                    shift += CONTROL_EFFECTIVENESS * tau * delta * ed.controls[k];
                }
            }
            // tail surfaces sit in the downwash of the wing, estimated from its lift coefficient one step ago
            let behind = e.center.z > self.cg.z + 2.0 && e.normal.y.abs() > 0.7;
            let downwash = if behind {
                (2.0 * self.downwash_cl / (std::f32::consts::PI * self.main_wing_ar)).to_degrees()
                    * DOWNWASH_FACTOR
            } else {
                0.0
            };
            let alpha_deg = wn.atan2(wt).to_degrees() + shift - downwash;
            let geom = &self.geoms[ed.geom];
            let mach = vp / sound_speed;
            let inputs = ElementInputs {
                index: e.index,
                retain_request: true,
                arg6: mach,
                ice: 0.0,
                alpha_in: alpha_deg,
                extra: [0.0; 3],
                flow: Flow {
                    f5c: temperature_c,
                    f6c: rho,
                    f1a0: 0.0,
                    f1a4: 0.0,
                    f408: 0.0,
                    flag_dac: false,
                    diagnostics: false,
                },
                aircraft: ElementAircraft {
                    f1f00: 0.0,
                    f1f3c: 0.0,
                    f64f4: 10.0,
                    f64f8: 10.0,
                    f64fc: 10.0,
                },
                g10: ed.g10,
                wing: WingFields {
                    is_right: geom.side,
                    elements: geom.elements,
                    f14: geom.ar,
                    f18: 0.0,
                    f1c: 0.0,
                    ratios: geom.ratios,
                    boundary: wing_element::Boundary {
                        x: &geom.x,
                        y: &geom.y,
                        z: &geom.z,
                        chord: &geom.chord,
                    },
                    flap_flags: &geom.flap_flags,
                    slat_flags: &geom.slat_flags,
                    names: [&geom.names[0], &geom.names[1], &geom.names[2]],
                },
                state: ElementState {
                    v: 1.0,
                    r11: 0.0,
                    r21: vp,
                    r163: 1.0,
                    stall_flag: ed.stalled,
                },
                skip_delta_block: geom.swept,
            };
            let airfoils = &self.airfoils;
            let noise = &self.noise;
            let foil_ids = ed.foil_ids;
            let Ok(out) = wing_element::evaluate(&inputs, |c: &FoilCall| {
                let o = profile::outer(
                    &airfoils[foil_ids[c.slot]],
                    OuterInputs {
                        alpha_deg: c.alpha,
                        multiplier: c.multiplier,
                        divisor: c.divisor,
                        regime: c.re_meg,
                        mach: c.arg6,
                        time,
                        noise_coordinates: [c.x_norm, c.y_norm, c.z_norm],
                        retain_stall: c.retain,
                    },
                    c.stalled,
                    Some(noise),
                )?;
                Ok(FoilResult {
                    ret: o.ret,
                    cl: o.cl,
                    cd: o.cd,
                    cm: o.cm,
                    ratio: o.normalized_alpha,
                    stalled: o.stalled,
                })
            }) else {
                continue;
            };
            ed.stalled = out.stall_flag;
            let (cl, mut cd, cm) = (out.cl, out.cd, out.cm);
            if !behind && e.normal.y.abs() > 0.7 && e.chord > 0.5 {
                cl_weighted += cl * e.area;
                area_sum += e.area;
            }
            if ed.controls[3] > 0.0 {
                cd += 0.012 * (flap_deg / 10.0).powi(2) * ed.controls[3];
            }
            let q = 0.5 * rho * vp * vp * e.area;
            let lift_dir = (e.normal * wt - e.trailing * wn) / vp;
            let drag_dir = (e.trailing * wt + e.normal * wn) / vp;
            let f = q * (cl * lift_dir + cd * drag_dir);
            force_b += f;
            let m = r.cross(f) + e.span_right * (q * e.chord * cm);
            moment_b += m;
            match self.wing_report.iter_mut().find(|w| w.0 == e.wing) {
                Some(w) => {
                    w.1 += f;
                    w.2 += m;
                }
                None => self.wing_report.push((e.wing, f, m)),
            }
        }

        if area_sum > 0.0 {
            self.downwash_cl = cl_weighted / area_sum;
        }

        // fuselage and other parasite drag
        let speed = v_body.length();
        if speed > 0.1 {
            let f = -0.5 * rho * PARASITE_AREA * speed * v_body;
            force_b += f;
            let alpha_body = (-v_body.y).atan2(-v_body.z).clamp(-0.35, 0.35);
            let q_inf = 0.5 * rho * speed * speed;
            moment_b += Vec3::X
                * (FUSELAGE_CM_ALPHA
                    * q_inf
                    * self.reference_area
                    * self.reference_chord
                    * alpha_body);
        }

        // propeller: thrust is the smaller of the static thrust (scaled with power) and eta P / (V + Vc)
        let forward = (-v_body.z).max(0.0);
        let lapse = ((rho / SEA_LEVEL_DENSITY) - 0.12) / 0.88;
        let power = c.throttle.clamp(0.0, 1.0) * self.max_power_w * lapse.max(0.0);
        let static_cap = STATIC_THRUST * (power / self.max_power_w).powf(2.0 / 3.0);
        let thrust = (PROP_EFFICIENCY * power / (forward + PROP_SPEED_OFFSET)).min(static_cap);
        force_b += Vec3::NEG_Z * thrust;
        moment_b += (Vec3::new(0.0, 0.0, -1.6) - self.cg).cross(Vec3::NEG_Z * thrust);

        // landing gear
        let mut force_w = rot * force_b;
        for g in &mut self.gears {
            let arm = g.tip - self.cg;
            let p = st.position + st.orientation * arm;
            let penetration = -p.y;
            if penetration <= 0.0 {
                g.compression = 0.0;
                g.on_ground = false;
                continue;
            }
            let v = st.velocity + st.orientation * st.omega.cross(arm);
            g.compression = penetration;
            g.on_ground = true;
            let normal = (g.stiffness * penetration - g.damping * v.y).max(0.0)
                + if penetration > 0.35 {
                    g.stiffness * 8.0 * (penetration - 0.35)
                } else {
                    0.0
                };
            // wheel heading: the nose direction in the ground plane, steered for the nose wheel
            let steer = -(c.rudder + c.rudder_trim).clamp(-1.0, 1.0) * g.steer_max_rad;
            let heading = st.orientation * (Quat::from_rotation_y(steer) * Vec3::NEG_Z);
            let long_dir = Vec3::new(heading.x, 0.0, heading.z).normalize_or_zero();
            let lat_dir = Vec3::new(-long_dir.z, 0.0, long_dir.x);
            let (v_long, v_lat) = (v.dot(long_dir), v.dot(lat_dir));
            let rolling = 0.02;
            let brake = if g.brakes {
                0.7 * c.brake.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut f_long = -normal * (rolling + brake) * (v_long / 0.4).tanh();
            let mut f_lat = -normal * 0.9 * (v_lat / 0.6).tanh();
            // friction circle
            let cap = 0.9 * normal;
            let mag = f_long.hypot(f_lat);
            if mag > cap {
                f_long *= cap / mag;
                f_lat *= cap / mag;
            }
            let f_w = Vec3::Y * normal + long_dir * f_long + lat_dir * f_lat;
            force_w += f_w;
            moment_b += arm.cross(rot_t * f_w);
        }
        (force_w, moment_b)
    }

    /// Advances the simulation by `dt` seconds (semi-implicit Euler; use steps of 1/200 s or less).
    pub fn step(&mut self, dt: f32, c: &Controls) {
        let (force_w, moment_b) = self.forces(c);
        let st = &mut self.state;
        let iw = self.inertia * st.omega;
        let domega = (moment_b - st.omega.cross(iw)) / self.inertia;
        st.omega += domega * dt;
        st.velocity += (force_w / self.mass + Vec3::new(0.0, -G, 0.0)) * dt;
        st.position += st.velocity * dt;
        st.orientation = (st.orientation * Quat::from_scaled_axis(st.omega * dt)).normalize();
        st.time += f64::from(dt);
    }

    pub fn telemetry(&self, controls: &Controls) -> Telemetry {
        let st = &self.state;
        let rot = Mat3::from_quat(st.orientation);
        let v_body = rot.transpose() * st.velocity;
        let forward = rot * Vec3::NEG_Z;
        let right = rot * Vec3::X;
        let heading = forward.x.atan2(-forward.z).to_degrees().rem_euclid(360.0);
        Telemetry {
            airspeed_kt: v_body.length() * 1.943_844,
            altitude_ft: (self.elevation_m + st.position.y - self.rest_cg_height) * 3.280_84,
            vertical_speed_fpm: st.velocity.y * 196.85,
            pitch_deg: forward.y.clamp(-1.0, 1.0).asin().to_degrees(),
            roll_deg: (-right.y).clamp(-1.0, 1.0).asin().to_degrees(),
            heading_deg: heading,
            alpha_deg: (-v_body.y).atan2(-v_body.z).to_degrees(),
            throttle: controls.throttle,
            on_ground: self.gears.iter().any(|g| g.on_ground),
            stalled_elements: self.elements.iter().filter(|e| e.stalled).count(),
        }
    }
}

/// Static loads on the gear for the weight `weight` with the CG at `cg`. Three struts give the exact
/// solution of the force and moment balance; otherwise the weight is shared in proportion to stiffness.
pub fn static_loads(gears: &[Gear], cg: Vec3, weight: f32) -> Vec<f32> {
    if gears.len() == 3 {
        // unknown loads f0..f2: sum f = W, sum f x = W cg.x, sum f z = W cg.z
        let m = Mat3::from_cols(
            Vec3::new(1.0, gears[0].tip.x, gears[0].tip.z),
            Vec3::new(1.0, gears[1].tip.x, gears[1].tip.z),
            Vec3::new(1.0, gears[2].tip.x, gears[2].tip.z),
        );
        if m.determinant().abs() > 1e-6 {
            let f = m.inverse() * Vec3::new(weight, weight * cg.x, weight * cg.z);
            return vec![f.x.max(0.0), f.y.max(0.0), f.z.max(0.0)];
        }
    }
    let k_total: f32 = gears.iter().map(|g| g.stiffness).sum();
    gears
        .iter()
        .map(|g| weight * g.stiffness / k_total)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_atmosphere_matches_the_tables() {
        let (rho0, mu0) = isa(0.0);
        assert!(
            (rho0 - 1.225).abs() < 1e-4 && (mu0 - 1.789e-5).abs() < 2e-8,
            "{rho0} {mu0}"
        );
        let (rho5, _) = isa(5000.0);
        assert!((rho5 - 0.7364).abs() < 0.003, "{rho5}");
    }

    #[test]
    fn three_strut_loads_balance_weight_and_moments() {
        let g = |x: f32, z: f32| Gear {
            tip: Vec3::new(x, -1.0, z),
            stiffness: 50_000.0,
            damping: 0.0,
            brakes: false,
            steer_max_rad: 0.0,
            compression: 0.0,
            on_ground: false,
        };
        let gears = [g(0.0, -0.6), g(-1.2, 1.4), g(1.2, 1.4)];
        let cg = Vec3::new(0.0, 0.0, 0.9);
        let w = 9000.0;
        let f = static_loads(&gears, cg, w);
        assert!((f.iter().sum::<f32>() - w).abs() < 0.5);
        let mx: f32 = f.iter().zip(&gears).map(|(f, g)| f * g.tip.x).sum();
        let mz: f32 = f.iter().zip(&gears).map(|(f, g)| f * g.tip.z).sum();
        assert!(mx.abs() < 1.0 && (mz - w * 0.9).abs() < 1.0);
        assert!(
            f[0] > 0.0 && f[0] < f[1],
            "the nose carries less than a main: {f:?}"
        );
        assert_eq!(f[1], f[2]);
    }
}
