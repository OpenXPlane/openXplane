//! Common OBJ attachment reads, recovered from reference function 0x141aaa610.
//! This is the read-mode loader, not a constructor or a renderer transform.
use crate::{Aircraft, aircraft::ACF_FT_TO_M};

const FIELDS: [&str; 11] = [
    "_v10_steers_with_gear",
    "_v10_is_internal",
    "_v10_att_wing",
    "_v10_att_body",
    "_v10_att_gear",
    "_v10_att_x_acf_prt_ref",
    "_v10_att_psi_ref",
    "_v10_att_y_acf_prt_ref",
    "_v10_att_the_ref",
    "_v10_att_z_acf_prt_ref",
    "_v10_att_phi_ref",
];

/// Inputs to the preliminary draw gate at 0x141a7b6a0. These are runtime
/// values, not inferred camera state or attachment constructor defaults.
pub struct DrawContext {
    pub pass: u32,
    pub mode: i32,
    pub loaded: bool,
    pub selected_cockpit_loaded: bool,
    pub cockpit_enabled: bool,
    pub main_view_enabled: bool,
}

/// The original gate's accept/skip boundary. Acceptance does not establish
/// final visibility: later pass classification, kill datarefs and draw state remain.
pub fn preliminary_draw_gate(flags: u32, context: &DrawContext) -> bool {
    if !context.loaded
        || flags & 0x200 != 0
        || (flags & 0x100 != 0 && context.pass & 0x100 == 0)
        || (flags & 2 != 0 && context.pass & 0x18 == 0)
    {
        return false;
    }
    if flags & 0x1800 != 0 {
        context.selected_cockpit_loaded
            && match context.mode {
                0 => context.cockpit_enabled,
                1 | 4 => true,
                _ => false,
            }
    } else {
        match context.mode {
            0 => context.main_view_enabled,
            1 | 4 => true,
            _ => false,
        }
    }
}

/// Caller-supplied object state. Missing properties preserve earlier words before
/// the loader's coordinate conversion. Constructor defaults are not inferred.
#[derive(Clone, Debug)]
pub struct Attachment {
    pub file: String,
    /// Integer payloads; their runtime effects have not been established here.
    pub steers_with_gear: i32,
    pub internal: i32,
    /// Wing, body and gear indices, in that order. All -1 selects the CG-relative branch.
    pub targets: [i32; 3],
    /// X/Y/Z in metres after loading; before loading these are caller-supplied words.
    pub position: [f32; 3],
    /// Psi/theta/phi as stored; the loader does not scale angles or build a rotation.
    pub angles: [f32; 3],
}

fn optional<'a>(acf: &'a Aircraft, prefix: &str, field: &str) -> Result<Option<&'a str>, String> {
    let key = format!("{prefix}{field}");
    if !acf.properties.iter().any(|p| p.key == key) {
        return Ok(None);
    }
    Ok(Some(&acf.unique_property(&key)?.value))
}

impl Attachment {
    /// Load a modern record whose filename and all common numeric fields are explicit.
    /// This avoids choosing unknown constructor defaults for an incomplete record.
    pub fn from_explicit(acf: &Aircraft, prefix: &str, cg_yz: [f32; 2]) -> Result<Self, String> {
        for field in FIELDS.into_iter().chain(["_v10_att_file_stl"]) {
            acf.unique_property(&format!("{prefix}{field}"))?;
        }
        let mut state = Self {
            file: String::new(),
            steers_with_gear: 0,
            internal: 0,
            targets: [0; 3],
            position: [0.0; 3],
            angles: [0.0; 3],
        };
        state.read_into(acf, prefix, cg_yz)?;
        Ok(state)
    }

    /// Apply the native read-mode algorithm to explicit prior state. Duplicate,
    /// invalid and non-finite inputs are rejected rather than assigning unverified semantics.
    pub fn read_into(
        &mut self,
        acf: &Aircraft,
        prefix: &str,
        cg_yz: [f32; 2],
    ) -> Result<(), String> {
        if acf.version != 1200 {
            return Err("attachment reads currently require ACF 1200".into());
        }
        let mut next = self.clone();
        if let Some(value) = optional(acf, prefix, "_v10_att_file_stl")? {
            next.file = value.to_owned();
        }
        let int = |field: &str| -> Result<Option<i32>, String> {
            optional(acf, prefix, field)?
                .map(|v| {
                    v.parse()
                        .map_err(|_| format!("invalid attachment integer: {prefix}{field}"))
                })
                .transpose()
        };
        if let Some(v) = int(FIELDS[0])? {
            next.steers_with_gear = v;
        }
        if let Some(v) = int(FIELDS[1])? {
            next.internal = v;
        }
        let legacy = int("_v10_att_part")?;
        let mut present = false;
        for (i, target) in next.targets.iter_mut().enumerate() {
            if let Some(v) = int(FIELDS[i + 2])? {
                *target = v;
                present = true;
            }
        }
        if let Some(part) = legacy.filter(|_| !present) {
            next.targets = [-1; 3];
            match part {
                8..56 => next.targets[0] = part - 8,
                56..85 => next.targets[1] = part - 56,
                85..95 => next.targets[2] = part - 85,
                _ => {}
            }
        }
        for i in 0..3 {
            let key = format!("{prefix}{}", FIELDS[5 + i * 2]);
            if optional(acf, prefix, FIELDS[5 + i * 2])?.is_some() {
                next.position[i] = acf.float_property(&key)?;
            }
            next.position[i] *= ACF_FT_TO_M;
            if next.targets == [-1; 3] && i > 0 {
                next.position[i] -= cg_yz[i - 1];
            }
            let field = FIELDS[6 + i * 2];
            if optional(acf, prefix, field)?.is_some() {
                next.angles[i] = acf.float_property(&format!("{prefix}{field}"))?;
            }
        }
        if next
            .position
            .iter()
            .chain(next.angles.iter())
            .any(|v| !v.is_finite())
        {
            return Err("non-finite attachment coordinate".into());
        }
        *self = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_original_preliminary_draw_gate() {
        for line in include_str!("../data/attachment_gate_vectors.tsv")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let c: Vec<i32> = line.split('\t').map(|v| v.parse().unwrap()).collect();
            let context = DrawContext {
                pass: c[1] as u32,
                mode: c[2],
                loaded: c[3] != 0,
                selected_cockpit_loaded: c[4] != 0,
                cockpit_enabled: c[5] != 0,
                main_view_enabled: c[6] != 0,
            };
            assert_eq!(
                preliminary_draw_gate(c[0] as u32, &context),
                c[7] != 0,
                "{line}"
            );
        }
    }

    #[test]
    fn matches_original_loader_words() {
        let words = |s: &str| -> Vec<u32> {
            s.split(',')
                .map(|v| u32::from_str_radix(v, 16).unwrap())
                .collect()
        };
        let local = std::env::var_os("OPENXPLANE_ATTACHMENT_VECTORS")
            .map(|p| std::fs::read_to_string(p).expect("read local attachment vectors"));
        let fixture = local
            .as_deref()
            .unwrap_or(include_str!("../data/attachment_vectors.tsv"));
        for line in fixture.lines().filter(|l| !l.starts_with('#')) {
            let c: Vec<_> = line.split('\t').collect();
            let props = c[1]
                .split(';')
                .filter(|p| !p.is_empty())
                .map(|p| {
                    let (k, v) = p.split_once('=').unwrap();
                    format!("P _obja/3/{k} {v}\n")
                })
                .collect::<String>();
            let acf = Aircraft::parse(&format!(
                "I\n1200 Version\nACF\nPROPERTIES_BEGIN\n{props}PROPERTIES_END\n"
            ))
            .unwrap();
            let w = words(c[2]);
            let mut a = Attachment {
                file: "before.obj".into(),
                steers_with_gear: w[0] as i32,
                internal: w[1] as i32,
                targets: [w[2] as i32, w[3] as i32, w[4] as i32],
                position: [
                    f32::from_bits(w[5]),
                    f32::from_bits(w[7]),
                    f32::from_bits(w[9]),
                ],
                angles: [
                    f32::from_bits(w[6]),
                    f32::from_bits(w[8]),
                    f32::from_bits(w[10]),
                ],
            };
            let cg = words(c[3]);
            a.read_into(
                &acf,
                "_obja/3/",
                [f32::from_bits(cg[0]), f32::from_bits(cg[1])],
            )
            .unwrap();
            let actual = [
                a.steers_with_gear as u32,
                a.internal as u32,
                a.targets[0] as u32,
                a.targets[1] as u32,
                a.targets[2] as u32,
                a.position[0].to_bits(),
                a.angles[0].to_bits(),
                a.position[1].to_bits(),
                a.angles[1].to_bits(),
                a.position[2].to_bits(),
                a.angles[2].to_bits(),
            ];
            assert_eq!(actual.as_slice(), words(c[4]), "{}", c[0]);
            assert_eq!(a.file, c[5], "{}", c[0]);
            if c[0].starts_with("installed-") {
                let explicit = Attachment::from_explicit(
                    &acf,
                    "_obja/3/",
                    [f32::from_bits(cg[0]), f32::from_bits(cg[1])],
                )
                .unwrap();
                assert_eq!(
                    explicit.position.map(f32::to_bits),
                    a.position.map(f32::to_bits)
                );
                assert_eq!(
                    explicit.angles.map(f32::to_bits),
                    a.angles.map(f32::to_bits)
                );
                assert_eq!(explicit.targets, a.targets);
                assert_eq!(explicit.file, a.file);
            }
        }
    }
}
