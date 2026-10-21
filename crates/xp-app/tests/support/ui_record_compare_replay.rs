//! Strict comparison replay; leaf effects remain controlled.
#![allow(dead_code)]
#[path = "ui_record_class_string_replay.rs"]
mod class_string_replay;
use openxplane::{
    ui_payload_move,
    ui_record_compare::{self, Services, State},
};
use std::collections::BTreeMap;
fn memory(encoded: &str) -> BTreeMap<u64, Vec<u8>> {
    encoded
        .split('/')
        .map(|r| {
            let (p, b) = r.split_once(':').unwrap();
            (
                p.parse().unwrap(),
                (0..b.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&b[i..i + 2], 16).unwrap())
                    .collect(),
            )
        })
        .collect()
}
fn compare(s: &State, encoded: &str, label: &str) {
    let target = memory(encoded);
    assert_eq!(s.memory.blocks.len(), target.len(), "{label} maps");
    for (p, b) in target {
        assert_eq!(s.memory.read(p, b.len()).unwrap(), b, "{label} at{p:x}");
    }
}
struct Replay {
    events: Vec<String>,
    position: usize,
}
impl Services for Replay {
    fn derived(&mut self, s: &mut State, record: u64, out: u64) -> Result<u64, &'static str> {
        if let Some(encoded) = self.events[self.position].strip_prefix("D~") {
            let result =
                class_string_replay::from_caller(encoded, record, out, s.stack, &mut s.memory);
            self.position += 1;
            Ok(result)
        } else {
            self.invoke(s, 0x141a3df30, &[record, out])
        }
    }

    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str> {
        let event = self
            .events
            .get(self.position)
            .expect("unexpected comparison provider");
        self.position += 1;
        let f = event.split('~').collect::<Vec<_>>();
        assert_eq!(f.len(), 5);
        assert_eq!(f[0], "I");
        let (name, a) = f[1].split_once(',').unwrap();
        assert_eq!(function, name.parse::<u64>().unwrap());
        let expected = a
            .split(',')
            .map(|v| v.parse::<u64>().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(args, expected, "provider{function:x}");
        compare(s, f[2], &format!("before{function:x}"));
        s.memory.blocks = memory(f[3]);
        Ok(f[4].parse().unwrap())
    }
}
pub fn run_row(row: &str, shared: &BTreeMap<u64, Vec<u8>>) -> State {
    let v = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(v.len(), 10);
    assert_eq!(v[6], "->");
    let left = v[1].parse().unwrap();
    let right = v[2].parse().unwrap();
    let inverted = v[4] == "1";
    assert!(inverted || v[4] == "0");
    let mut s = State {
        memory: ui_payload_move::State {
            blocks: memory(v[5]),
        },
        stack: v[3].parse().unwrap(),
    };
    for (p, b) in shared {
        assert_eq!(
            s.memory.read(*p, b.len()).unwrap(),
            *b,
            "live comparison input at{p:x}"
        );
    }
    let mut h = Replay {
        events: if v[9] == "-" {
            vec![]
        } else {
            v[9].split('|').map(str::to_owned).collect()
        },
        position: 0,
    };
    let result = if inverted {
        ui_record_compare::different(&mut s, &mut h, left, right)
    } else {
        ui_record_compare::equal(&mut s, &mut h, left, right)
    }
    .unwrap();
    assert_eq!(u64::from(result), v[8].parse::<u64>().unwrap());
    compare(&s, v[7], "final comparison");
    assert_eq!(h.position, h.events.len());
    s
}

pub fn from_caller(
    encoded: &str,
    left: u64,
    right: u64,
    stack: u64,
    memory: &mut ui_payload_move::State,
) -> bool {
    let bytes = (0..encoded.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&encoded[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let row = String::from_utf8(bytes).unwrap();
    let fields = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(fields[1].parse::<u64>().unwrap(), left);
    assert_eq!(fields[2].parse::<u64>().unwrap(), right);
    assert_eq!(fields[3].parse::<u64>().unwrap(), stack - 0xa0);
    assert_eq!(fields[4], "1");
    let runtime = run_row(&row, &memory.blocks);
    for (p, b) in &mut memory.blocks {
        *b = runtime.memory.read(*p, b.len()).unwrap();
    }
    fields[8] == "1"
}

pub fn actual_classes_in_encoded_row(encoded: &str) -> usize {
    let bytes = (0..encoded.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&encoded[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let row = String::from_utf8(bytes).unwrap();
    let fields = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(fields.len(), 10);
    fields[9]
        .split('|')
        .filter(|event| event.starts_with("D~"))
        .count()
}

pub fn actual_strings_in_row(row: &str) -> usize {
    let fields = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(fields.len(), 10);
    assert_eq!(fields[0], "C");
    fields[9]
        .split('|')
        .filter_map(|event| event.strip_prefix("D~"))
        .map(class_string_replay::actual_strings_in_encoded_row)
        .sum()
}
