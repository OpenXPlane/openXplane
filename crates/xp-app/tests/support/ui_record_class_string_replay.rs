//! Whole class-string/label replay, with explicit controlled leaf boundaries.
#![allow(dead_code)]
use openxplane::{
    ui_payload_move,
    ui_record_class_string::{self, Services, State},
};
use std::collections::BTreeMap;
#[path = "ui_string_append_replay.rs"]
mod string_replay;
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
        assert_eq!(s.memory.read(p, b.len()).unwrap(), b, "{label} at{p:x}");
    }
}
struct Replay {
    events: Vec<String>,
    position: usize,
}
impl Services for Replay {
    fn append(
        &mut self,
        s: &mut State,
        out: u64,
        data: u64,
        length: u64,
    ) -> Result<u64, &'static str> {
        if let Some(encoded) = self.events[self.position].strip_prefix("S~") {
            let result = string_replay::from_caller(
                encoded,
                "A",
                &[out, data, length],
                s.stack - 0x40,
                &mut s.memory,
            );
            self.position += 1;
            Ok(result)
        } else {
            self.invoke(s, 0x140624040, &[out, data, length])
        }
    }
    fn grow_separator(
        &mut self,
        s: &mut State,
        out: u64,
        opaque: u64,
    ) -> Result<u64, &'static str> {
        if let Some(encoded) = self.events[self.position].strip_prefix("S~") {
            let result = string_replay::from_caller(
                encoded,
                "G",
                &[out, 1, opaque, 0x142505110, 1],
                s.stack - 0x50,
                &mut s.memory,
            );
            self.position += 1;
            Ok(result)
        } else {
            self.invoke(s, 0x1405eca90, &[out, 1, opaque, 0x142505110, 1])
        }
    }

    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str> {
        let event = self
            .events
            .get(self.position)
            .expect("unexpected class-string provider");
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
    let derived = v[0] == "D";
    assert!(derived || v[0] == "L");
    assert_eq!(v.len(), if derived { 9 } else { 8 });
    let offset = usize::from(derived);
    assert_eq!(v[4 + offset], "->");
    let mut s = State {
        memory: ui_payload_move::State {
            blocks: memory(v[3 + offset]),
        },
        stack: if derived { v[3].parse().unwrap() } else { 0 },
    };
    for (p, b) in shared {
        assert_eq!(
            s.memory.read(*p, b.len()).unwrap(),
            *b,
            "live class-string input at{p:x}"
        );
    }
    let mut h = Replay {
        events: if v[7 + offset] == "-" {
            vec![]
        } else {
            v[7 + offset].split('|').map(str::to_owned).collect()
        },
        position: 0,
    };
    let result = if derived {
        ui_record_class_string::build(&mut s, &mut h, v[1].parse().unwrap(), v[2].parse().unwrap())
    } else {
        ui_record_class_string::label(
            &mut s,
            &mut h,
            v[1].parse().unwrap(),
            v[2].parse::<i128>().unwrap() as u64,
        )
    }
    .unwrap();
    assert_eq!(result, v[6 + offset].parse::<u64>().unwrap());
    compare(&s, v[5 + offset], "final class-string");
    assert_eq!(h.position, h.events.len());
    s
}
pub fn from_caller(
    encoded: &str,
    record: u64,
    out: u64,
    stack: u64,
    memory: &mut ui_payload_move::State,
) -> u64 {
    let bytes = (0..encoded.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&encoded[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let row = String::from_utf8(bytes).unwrap();
    let fields = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(fields[0], "D");
    assert_eq!(fields[1].parse::<u64>().unwrap(), record);
    assert_eq!(fields[2].parse::<u64>().unwrap(), out);
    assert_eq!(fields[3].parse::<u64>().unwrap(), stack - 0xa0);
    let runtime = run_row(&row, &memory.blocks);
    for (p, b) in &mut memory.blocks {
        *b = runtime.memory.read(*p, b.len()).unwrap();
    }
    out
}
