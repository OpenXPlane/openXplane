//! Whole original append/growth replay. Numeric input remains local.
#![allow(dead_code)]
use openxplane::{
    ui_payload_move,
    ui_string_append::{self, Services, State},
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
    let expected = memory(encoded);
    assert_eq!(s.memory.blocks.len(), expected.len(), "{label} maps");
    for (p, b) in expected {
        assert_eq!(s.memory.read(p, b.len()).unwrap(), b, "{label} at {p:x}");
    }
}
struct Replay {
    events: Vec<String>,
    position: usize,
}
impl Services for Replay {
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str> {
        let event = self
            .events
            .get(self.position)
            .expect("unexpected string provider");
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
        assert_eq!(args, expected, "provider {function:x}");
        compare(s, f[2], &format!("before {function:x}"));
        s.memory.blocks = memory(f[3]);
        Ok(f[4].parse().unwrap())
    }
}
pub fn run_row(row: &str) -> State {
    let v = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(v.len(), 11);
    assert_eq!(v[0], "S");
    assert_eq!(v[7], "->");
    let args = v[4]
        .split(',')
        .map(|a| a.parse::<u64>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(args[0], v[2].parse::<u64>().unwrap());
    assert_eq!(v[5].split(',').count(), 24);
    let mut s = State {
        memory: ui_payload_move::State {
            blocks: memory(v[6]),
        },
        stack: v[3].parse().unwrap(),
    };
    let mut h = Replay {
        events: v[10].split('|').map(str::to_owned).collect(),
        position: 0,
    };
    let result = match v[1] {
        "A" => {
            assert_eq!(args.len(), 3);
            ui_string_append::append(&mut s, &mut h, args[0], args[1], args[2])
        }
        "G" => {
            assert_eq!(args.len(), 5);
            ui_string_append::grow(&mut s, &mut h, args[0], args[1], args[2], args[3], args[4])
        }
        _ => panic!("unknown operation"),
    }
    .unwrap();
    assert_eq!(result, v[9].parse::<u64>().unwrap());
    compare(&s, v[8], "final string");
    assert_eq!(h.position, h.events.len());
    s
}

pub fn from_caller(
    encoded: &str,
    mode: &str,
    args: &[u64],
    stack: u64,
    memory_state: &mut ui_payload_move::State,
) -> u64 {
    let bytes = (0..encoded.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&encoded[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let row = String::from_utf8(bytes).unwrap();
    let fields = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(fields[1], mode);
    assert_eq!(fields[3].parse::<u64>().unwrap(), stack);
    assert_eq!(
        fields[4]
            .split(',')
            .map(|a| a.parse::<u64>().unwrap())
            .collect::<Vec<_>>(),
        args
    );
    let initial = ui_payload_move::State {
        blocks: memory(fields[6]),
    };
    for (p, b) in &memory_state.blocks {
        assert_eq!(
            initial.read(*p, b.len()).unwrap(),
            *b,
            "live string input at {p:x}"
        );
    }
    let runtime = run_row(&row);
    let pools = fields[5]
        .split(',')
        .map(|p| p.parse::<u64>().unwrap())
        .collect::<Vec<_>>();
    for (p, b) in &mut memory_state.blocks {
        *b = if pools.contains(p) {
            runtime.memory.blocks.get(p).unwrap().clone()
        } else {
            runtime.memory.read(*p, b.len()).unwrap()
        };
    }
    fields[9].parse().unwrap()
}

/// Only these declared allocation pools may grow across a callee boundary.
pub fn pools_in_encoded_row(encoded: &str) -> Vec<u64> {
    let bytes = (0..encoded.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&encoded[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let row = String::from_utf8(bytes).unwrap();
    let fields = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(fields.len(), 11);
    assert_eq!(fields[0], "S");
    fields[5].split(',').map(|p| p.parse().unwrap()).collect()
}
