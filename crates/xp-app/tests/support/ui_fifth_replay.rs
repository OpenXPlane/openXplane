//! Strict local original-code replay support, reused by whole-factory tests.
#![allow(dead_code)]
use openxplane::{
    ui_fifth_refresh::{self, Services, State},
    ui_payload_move,
};
use std::collections::BTreeMap;
fn num(s: &str) -> u64 {
    s.parse().unwrap()
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn text(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn memory(s: &str) -> BTreeMap<u64, Vec<u8>> {
    s.split('/')
        .map(|r| {
            let (p, b) = r.split_once(':').unwrap();
            (num(p), hex(b))
        })
        .collect()
}
fn parse(s: &str, id: u64, stack: u64) -> State {
    let f = s.split(';').collect::<Vec<_>>();
    assert_eq!(f.len(), 5);
    State {
        memory: ui_payload_move::State {
            blocks: memory(f[0]),
        },
        identity: id,
        stack,
        active_global: num(f[1]),
        resource_index: num(f[2]) as u32,
        override_enabled: num(f[3]) as u32,
        override_bits: num(f[4]) as u32,
    }
}
fn snapshot(s: &State) -> String {
    let raw = s
        .memory
        .blocks
        .iter()
        .filter(|(p, b)| !(**p <= s.stack && s.stack < **p + b.len() as u64))
        .map(|(p, b)| format!("{p}:{}", text(b)))
        .collect::<Vec<_>>()
        .join("/");
    format!(
        "{raw};{};{};{};{}",
        s.active_global, s.resource_index, s.override_enabled, s.override_bits
    )
}
fn compare(a: &str, b: &str, label: &str) {
    if a != b {
        let p = a
            .bytes()
            .zip(b.bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(a.len().min(b.len()));
        panic!("{label}: mismatch at{p}; lengths {}/{}", a.len(), b.len());
    }
}
struct Replay {
    events: Vec<String>,
    position: usize,
}
impl Services for Replay {
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str> {
        let f = self.events[self.position].split('~').collect::<Vec<_>>();
        self.position += 1;
        assert_eq!(f.len(), 7);
        assert_eq!(f[0], "I");
        let (name, a) = f[1].split_once(',').unwrap();
        assert_eq!(num(name), function, "event{}", self.position);
        let expected = if a == "-" {
            vec![]
        } else {
            a.split(',').map(num).collect::<Vec<_>>()
        };
        assert_eq!(args, expected);
        compare(
            &snapshot(s),
            f[2],
            &format!("before {function:x} event{}", self.position),
        );
        if f[5] != "-" {
            for (p, b) in memory(f[5]) {
                assert_eq!(
                    s.memory.read(p, b.len()).unwrap(),
                    b,
                    "read{function:x} at{p:x}"
                );
            }
        }
        let scratch = s.memory.blocks.remove(&s.stack).unwrap();
        *s = parse(f[3], s.identity, s.stack);
        s.memory.blocks.insert(s.stack, scratch);
        if f[6] != "-" {
            for (p, b) in memory(f[6]) {
                s.memory.write(p, &b)?;
            }
        }
        Ok(num(f[4]))
    }
}
pub fn run_row(row: &str, initial_image: Option<&[u8]>, inherited: Option<(u64, u32)>) -> State {
    let f = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(f.len(), 8);
    assert_eq!(f[5], "->");
    let id = num(f[1]);
    let stack = num(f[2]);
    let mut s = parse(f[3], id, stack);
    let mut image = s.memory.read(id, 0xb0).unwrap();
    if let Some(expected) = initial_image {
        assert_eq!(image, expected);
    }
    if let Some((active, index)) = inherited {
        assert_eq!(s.active_global, active);
        assert_eq!(s.resource_index, index);
    }
    s.memory.blocks.insert(stack, hex(f[4]));
    let mut replay = Replay {
        events: f[7].split('|').map(str::to_string).collect(),
        position: 0,
    };
    ui_fifth_refresh::refresh_image(&mut image, &mut s, &mut replay).unwrap();
    assert_eq!(image, s.memory.read(id, 0xb0).unwrap());
    compare(&snapshot(&s), f[6], "final fifth refresh");
    assert_eq!(replay.position, replay.events.len());
    s
}
