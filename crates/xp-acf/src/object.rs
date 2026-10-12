//! The aircraft object filled from the ACF properties, following the schema recovered from the reference loader
//! (`0x141a972a0` and the loaders it calls; `research/ACF_OBJECT.md`). The schema (`data/object_schema.tsv`) was
//! derived by running the original loader on the reference aircraft and on randomised copies of them
//! (`tools/derive_acf_schema.py`): each row names a key template (integer path components are `{}`), the region
//! it lands in, the destination as a base offset plus a stride per index, and the transform from the parsed value
//! to the stored 32-bit word.
//!
//! Only properties present in the file are stored; a property that is absent keeps the word the object had before
//! loading (zero here: the original's constructor defaults are not applied by this function).
use crate::Aircraft;
use std::collections::{BTreeMap, HashMap};

const SCHEMA: &str = include_str!("../data/object_schema.tsv");
const DEFAULTS: &str = include_str!("../data/object_defaults.tsv");

/// Where a word lands: the aircraft object itself or one of the arrays it points to (the name is the offset of the
/// pointer in the aircraft object: `R5ff8` engines, `R6010` parts, `R6028` wings, `R6080` gear, ...).
pub type Region = String;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Float,
    FloatScaled,
    Int,
    Text,
}

/// How the parsed value becomes the stored word.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Transform {
    /// The float32 (times the reader's own coefficient for a scaled read) as read.
    Raw,
    /// The value times a float32 constant.
    Scale(f32),
    /// The value times a float32 constant minus the stored centre-of-gravity word at the given aircraft offset.
    ScaleMinus(f32, u32),
    /// A 32-bit integer.
    Int,
    /// The value (times `scale` when given) replaced by `default` when `limit` is above it (double precision
    /// when `double`; an unordered comparison keeps the value).
    Floor {
        limit: f64,
        double: bool,
        default: u32,
        scale: Option<f32>,
    },
    /// Not derived by the tool: the entry is skipped.
    Unknown,
}

/// Which index of a template selects the transform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexPosition {
    First,
    Last,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub template: String,
    pub kind: Kind,
    pub coefficient: f32,
    pub region: Region,
    pub base: u32,
    pub strides: Vec<u32>,
    pub transform: Transform,
    /// Transforms that depend on an index: `(index position, value, transform)` where the position is `Last` or
    /// `First`; the entry's own transform is the default.
    pub by_index: Vec<(IndexPosition, u32, Transform)>,
}

pub struct Schema {
    entries: Vec<Entry>,
    by_template: HashMap<String, Vec<usize>>,
}

fn kind_of(name: &str) -> Kind {
    match name {
        "float" => Kind::Float,
        "coeff" => Kind::FloatScaled,
        "int" => Kind::Int,
        _ => Kind::Text,
    }
}

impl Schema {
    pub fn load() -> Schema {
        let mut entries = Vec::new();
        for line in SCHEMA.lines().filter(|l| !l.is_empty()) {
            let f: Vec<&str> = line.split('\t').collect();
            let hex = |s: &str| u32::from_str_radix(s, 16).unwrap_or(0);
            let parse = |t: &str| match t.split(':').collect::<Vec<_>>().as_slice() {
                ["raw"] => Transform::Raw,
                ["int"] => Transform::Int,
                ["scale", c] => Transform::Scale(f32::from_bits(hex(c))),
                ["floor", w, limit, default, scale] => {
                    let double = *w == "d";
                    let bits = u64::from_str_radix(limit, 16).unwrap_or(0);
                    Transform::Floor {
                        limit: if double {
                            f64::from_bits(bits)
                        } else {
                            f64::from(f32::from_bits(bits as u32))
                        },
                        double,
                        default: hex(default),
                        scale: (*scale != "raw").then(|| f32::from_bits(hex(scale))),
                    }
                }
                ["scale_minus_cgy", c] => Transform::ScaleMinus(f32::from_bits(hex(c)), 0x2850),
                ["scale_minus_cgz", c] => Transform::ScaleMinus(f32::from_bits(hex(c)), 0x2854),
                _ => Transform::Unknown,
            };
            let mut by_index = Vec::new();
            let mut transform = Transform::Unknown;
            if let Some(rest) = f[5].strip_prefix("last:") {
                for part in rest.split('|') {
                    if let Some((i, t)) = part.split_once('=') {
                        by_index.push((IndexPosition::Last, i.parse().unwrap_or(0), parse(t)));
                    }
                }
            } else if let Some(rest) = f[5].strip_prefix("first:") {
                for (n, part) in rest.split('|').enumerate() {
                    match (n, part.split_once('=')) {
                        (0, _) => transform = parse(part),
                        (_, Some((i, t))) => {
                            by_index.push((IndexPosition::First, i.parse().unwrap_or(0), parse(t)));
                        }
                        _ => {}
                    }
                }
            } else {
                transform = parse(f[5]);
            }
            entries.push(Entry {
                template: f[0].to_string(),
                kind: kind_of(f[1]),
                coefficient: f.get(6).map_or(1.0, |c| f32::from_bits(hex(c))),
                region: f[2].to_string(),
                base: hex(f[3]),
                strides: f[4]
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.parse().unwrap_or(0))
                    .collect(),
                transform,
                by_index,
            });
        }
        let mut by_template: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, e) in entries.iter().enumerate() {
            by_template.entry(e.template.clone()).or_default().push(i);
        }
        Schema {
            entries,
            by_template,
        }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}

/// The words of the aircraft object and of its sub-object regions: region name to offset to word.
#[derive(Debug, Default)]
pub struct ObjectWords {
    pub regions: BTreeMap<String, BTreeMap<u32, u32>>,
}

impl ObjectWords {
    pub fn word(&self, region: &str, offset: u32) -> u32 {
        self.regions
            .get(region)
            .and_then(|r| r.get(&offset))
            .copied()
            .unwrap_or(0)
    }
}

fn template_and_indices(key: &str) -> (String, Vec<u32>) {
    let mut indices = Vec::new();
    let template: Vec<String> = key
        .split('/')
        .map(|p| {
            let numbers: Vec<&str> = p.split(',').collect();
            if numbers
                .iter()
                .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            {
                for n in &numbers {
                    indices.push(n.parse().unwrap_or(0));
                }
                vec!["{}"; numbers.len()].join(",")
            } else {
                p.to_string()
            }
        })
        .collect();
    (template.join("/"), indices)
}

/// Fills the object from the properties of a parsed ACF file.
pub fn load_object(acf: &Aircraft, schema: &Schema) -> ObjectWords {
    let mut words = ObjectWords::default();
    // the words the loader leaves for absent properties (its constants and zero fills)
    for line in DEFAULTS.lines().filter(|l| !l.is_empty()) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() == 3 {
            words.regions.entry(f[0].to_string()).or_default().insert(
                u32::from_str_radix(f[1], 16).unwrap_or(0),
                u32::from_str_radix(f[2], 16).unwrap_or(0),
            );
        }
    }
    // the centre-of-gravity words come first: other entries subtract them
    let mut pending: Vec<(&Entry, u32, &str, Transform)> = Vec::new();
    for p in &acf.properties {
        let (template, indices) = template_and_indices(&p.key);
        let Some(candidates) = schema.by_template.get(&template) else {
            continue;
        };
        for &c in candidates {
            let e = &schema.entries[c];
            if e.strides.len() != indices.len() {
                continue;
            }
            let offset = e.base
                + e.strides
                    .iter()
                    .zip(&indices)
                    .map(|(s, i)| s * i)
                    .sum::<u32>();
            let transform = e
                .by_index
                .iter()
                .find(|(position, value, _)| {
                    let at = match position {
                        IndexPosition::First => indices.first(),
                        IndexPosition::Last => indices.last(),
                    };
                    at == Some(value)
                })
                .map_or(e.transform, |(_, _, t)| *t);
            pending.push((e, offset, p.value.as_str(), transform));
        }
    }
    let store = |words: &mut ObjectWords, region: &str, offset: u32, word: u32| {
        words
            .regions
            .entry(region.to_string())
            .or_default()
            .insert(offset, word);
    };
    for second_pass in [false, true] {
        for (e, offset, text, transform) in &pending {
            let needs_cg = matches!(transform, Transform::ScaleMinus(..));
            if needs_cg != second_pass {
                continue;
            }
            if e.kind == Kind::Text {
                // a std::string object: up to 15 characters inline, then the size and the capacity (a longer
                // string is on the heap; its pointer is not modelled)
                let bytes = text.as_bytes();
                let r = words.regions.entry(e.region.clone()).or_default();
                let inline = bytes.len().min(15);
                for w in 0..4u32 {
                    let mut chunk = [0u8; 4];
                    for (k, c) in chunk.iter_mut().enumerate() {
                        let at = (w * 4) as usize + k;
                        if bytes.len() <= 15 && at < bytes.len() {
                            *c = bytes[at];
                        }
                    }
                    if bytes.len() <= 15 {
                        r.insert(*offset + 4 * w, u32::from_le_bytes(chunk));
                    }
                }
                let _ = inline;
                r.insert(*offset + 0x10, bytes.len() as u32);
                r.insert(*offset + 0x14, 0);
                let capacity = if bytes.len() <= 15 {
                    15
                } else {
                    (bytes.len() | 15) as u32
                };
                r.insert(*offset + 0x18, capacity);
                r.insert(*offset + 0x1c, 0);
                continue;
            }
            let word = match (e.kind, *transform) {
                (_, Transform::Unknown) => continue,
                (Kind::Int, _) => {
                    let Ok(v) = text.trim().parse::<f64>() else {
                        continue;
                    };
                    v as i32 as u32
                }
                (kind, transform) => {
                    let Ok(v) = text.trim().parse::<f32>() else {
                        continue;
                    };
                    let v = if kind == Kind::FloatScaled {
                        v * e.coefficient
                    } else {
                        v
                    };
                    match transform {
                        Transform::Raw => v.to_bits(),
                        Transform::Scale(c) => (v * c).to_bits(),
                        Transform::Floor {
                            limit,
                            double,
                            default,
                            scale,
                        } => {
                            let stored = scale.map_or(v, |c| v * c);
                            let below = if double {
                                limit > f64::from(stored)
                            } else {
                                (limit as f32) > stored
                            };
                            if below { default } else { stored.to_bits() }
                        }
                        Transform::ScaleMinus(c, reference) => {
                            let cg = f32::from_bits(words.word("B", reference));
                            ((v * c) - cg).to_bits()
                        }
                        _ => continue,
                    }
                }
            };
            store(&mut words, &e.region, *offset, word);
        }
    }
    words
}
