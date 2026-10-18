//! Strict whole reset caller replay. Provider bodies remain controlled.
#![allow(dead_code)]
use openxplane::{
    ui_payload_move,
    ui_selection_reset::{self, Services, State},
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
    assert_eq!(s.memory.blocks.len(), target.len(), "{label} map length");
    for (p, bytes) in target {
        assert_eq!(
            s.memory.read(p, bytes.len()).unwrap(),
            bytes,
            "{label} at{p:x}"
        );
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
            .expect("unexpected provider request");
        self.position += 1;
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
        let expected = if a == "-" {
            vec![]
        } else {
            a.split(',')
                .map(|x| x.parse::<u64>().unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(args, expected);
        compare(
            s,
            f[2],
            &format!("before{function:x} event{}", self.position),
        );
        s.memory.blocks = memory(f[3]);
        Ok(f[4].parse().unwrap())
    }
}
pub fn run_row(
    row: &str,
    image: Option<&[u8]>,
    base: Option<&openxplane::ui_input_construct::Block>,
    shared: &BTreeMap<u64, Vec<u8>>,
) -> State {
    let v = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(v.len(), 8);
    assert_eq!(v[5], "->");
    let mut s = State {
        identity: v[1].parse().unwrap(),
        stack: v[2].parse().unwrap(),
        memory: ui_payload_move::State {
            blocks: memory(v[4]),
        },
    };
    if let Some(bytes) = image {
        assert_eq!(s.memory.read(s.identity, bytes.len()).unwrap(), bytes);
    }
    if let Some(b) = base {
        assert_eq!(s.memory.read(b.address, b.bytes.len()).unwrap(), b.bytes);
    }
    for (p, b) in shared {
        assert_eq!(
            s.memory.read(*p, b.len()).unwrap(),
            *b,
            "shared reset input at{p:x}"
        );
    }
    let mut h = Replay {
        events: v[7].split('|').map(str::to_owned).collect(),
        position: 0,
    };
    ui_selection_reset::reset(&mut s, &mut h, v[3].parse::<u64>().unwrap() as u32).unwrap();
    compare(&s, v[6], "final reset");
    assert_eq!(h.position, h.events.len());
    s
}
