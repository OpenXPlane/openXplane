//! Strict whole initial callback replay with explicit helper scope.
#![allow(dead_code)]
use openxplane::{
    ui_payload_move,
    ui_selection_initial::{self, Services, State},
};
use std::collections::BTreeMap;
fn memory(encoded: &str) -> BTreeMap<u64, Vec<u8>> {
    encoded
        .split('/')
        .map(|r| {
            let (p, b) = r.split_once(':').unwrap();
            let bytes = (0..b.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&b[i..i + 2], 16).unwrap())
                .collect();
            (p.parse().unwrap(), bytes)
        })
        .collect()
}
fn compare(s: &State, expected: &str, label: &str) {
    let target = memory(expected);
    assert_eq!(s.memory.blocks.len(), target.len());
    for (p, bytes) in target {
        assert_eq!(
            s.memory.read(p, bytes.len()).unwrap(),
            bytes,
            "{label} at{p:x}"
        );
    }
}
pub type RecursiveReset = fn(&str, &mut State);
struct Replay {
    events: Vec<String>,
    position: usize,
    controlled: bool,
    recursive: Option<RecursiveReset>,
}
impl Services for Replay {
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str> {
        let event = self
            .events
            .get(self.position)
            .expect("unexpected callback dependency");
        self.position += 1;
        if let Some(encoded) = event.strip_prefix("R~") {
            assert_eq!(function, 0x14113a460);
            assert_eq!(args, [s.identity, 1]);
            let bytes = (0..encoded.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&encoded[i..i + 2], 16).unwrap())
                .collect::<Vec<_>>();
            let row = String::from_utf8(bytes).unwrap();
            self.recursive
                .expect("actual recursive reset handler required")(&row, s);
            return Ok(0); // Callback ignores reset's unspecified RAX.
        }
        let f = event.split('~').collect::<Vec<_>>();
        assert_eq!(f.len(), 5);
        assert_eq!(f[0], "I");
        let (name, a) = f[1].split_once(',').unwrap();
        assert_eq!(
            function,
            name.parse::<u64>().unwrap(),
            "event{}",
            self.position
        );
        let expected = a
            .split(',')
            .map(|x| x.parse::<u64>().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(args, expected);
        compare(
            s,
            f[2],
            &format!("before{function:x} event{}", self.position),
        );
        s.memory.blocks = memory(f[3]);
        Ok(f[4].parse().unwrap())
    }
    fn tag_is_three(&mut self, s: &mut State, receiver: u64) -> Result<bool, &'static str> {
        if self.controlled {
            Ok(self.invoke(s, 0x1417fb370, &[receiver])? as u8 != 0)
        } else {
            ui_selection_initial::tag_is_three(&s.memory, receiver)
        }
    }
    fn payload(&mut self, s: &mut State, receiver: u64) -> Result<u64, &'static str> {
        if self.controlled {
            self.invoke(s, 0x1417f8940, &[receiver])
        } else {
            ui_selection_initial::payload_if_three(&s.memory, receiver)
        }
    }
}
pub fn run_row(row: &str, image: Option<&[u8]>, shared: &BTreeMap<u64, Vec<u8>>) -> State {
    run_row_with_reset(row, image, shared, None)
}
pub fn run_row_with_reset(
    row: &str,
    image: Option<&[u8]>,
    shared: &BTreeMap<u64, Vec<u8>>,
    recursive: Option<RecursiveReset>,
) -> State {
    let v = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(v.len(), 9);
    assert_eq!(v[6], "->");
    let mut s = State {
        identity: v[1].parse().unwrap(),
        stack: v[2].parse().unwrap(),
        memory: ui_payload_move::State {
            blocks: memory(v[5]),
        },
    };
    if let Some(bytes) = image {
        assert_eq!(s.memory.read(s.identity, bytes.len()).unwrap(), bytes);
    }
    for (p, b) in shared {
        assert_eq!(
            s.memory.read(*p, b.len()).unwrap(),
            *b,
            "shared callback input at{p:x}"
        );
    }
    let mut h = Replay {
        events: if v[8] == "-" {
            vec![]
        } else {
            v[8].split('|').map(str::to_owned).collect()
        },
        position: 0,
        controlled: v[4] == "1",
        recursive,
    };
    ui_selection_initial::callback(&mut s, &mut h, v[3].parse().unwrap()).unwrap();
    compare(&s, v[7], "final callback");
    assert_eq!(h.position, h.events.len());
    s
}
