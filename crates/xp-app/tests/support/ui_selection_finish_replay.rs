//! Raw Unicorn caller replay; NaN propagation is explicitly oracle-specific.
//! Production scalar policy is checked independently against actual SSE instructions.
#![allow(dead_code)]
use openxplane::{
    ui_payload_move,
    ui_selection_finish::{self, Services, State},
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
    assert_eq!(s.memory.blocks.len(), target.len(), "{label} map count");
    for (p, b) in target {
        let actual = s.memory.read(p, b.len()).unwrap();
        if actual != b {
            let i = actual.iter().zip(&b).position(|(a, b)| a != b).unwrap();
            panic!(
                "{label} at{p:x}+{i:x}: {:02x?} != {:02x?}",
                &actual[i..(i + 16).min(actual.len())],
                &b[i..(i + 16).min(b.len())]
            );
        }
    }
}
struct Replay {
    events: Vec<String>,
    position: usize,
}
impl Services for Replay {
    fn scalar(&mut self, a: u32, b: u32, op: ui_selection_finish::ScalarOp) -> u32 {
        let is_nan = |v: u32| v & 0x7f800000 == 0x7f800000 && v & 0x7fffff != 0;
        if is_nan(a) && is_nan(b) {
            // Unicorn2.1.4 x86 softfloat uses x87 larger-significand selection.
            // This compatibility override belongs to this oracle, not the engine.
            let aq = a & 0x400000 != 0;
            let bq = b & 0x400000 != 0;
            let chosen = if aq != bq {
                if aq { a } else { b }
            } else if a << 1 > b << 1 {
                a
            } else if a << 1 < b << 1 {
                b
            } else {
                a.min(b)
            };
            return chosen | 0x400000;
        }
        ui_selection_finish::scalar_sse(a, b, op)
    }
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str> {
        let event = self
            .events
            .get(self.position)
            .expect("unexpected finish provider");
        self.position += 1;
        let v = event.split('~').collect::<Vec<_>>();
        assert_eq!(v.len(), 5);
        assert_eq!(v[0], "I");
        let (f, a) = v[1].split_once(',').unwrap();
        assert_eq!(
            function,
            f.parse::<u64>().unwrap(),
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
        assert_eq!(args, expected, "args {function:x} event{}", self.position);
        compare(
            s,
            v[2],
            &format!("before{function:x} event{}", self.position),
        );
        s.memory.blocks = memory(v[3]);
        Ok(v[4].parse().unwrap())
    }
}
pub fn run_row(row: &str, image: Option<&[u8]>, shared: &BTreeMap<u64, Vec<u8>>) -> State {
    let v = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(v.len(), 8);
    assert_eq!(v[3], "U214", "raw Unicorn2.1.4 arithmetic scope required");
    assert_eq!(v[5], "->");
    let mut s = State {
        identity: v[1].parse().unwrap(),
        stack: v[2].parse().unwrap(),
        memory: ui_payload_move::State {
            blocks: memory(v[4]),
        },
    };
    if let Some(image) = image {
        assert_eq!(s.memory.read(s.identity, image.len()).unwrap(), image);
    }
    for (p, b) in shared {
        assert_eq!(
            s.memory.read(*p, b.len()).unwrap(),
            *b,
            "shared finish input at{p:x}"
        );
    }
    let mut h = Replay {
        events: v[7].split('|').map(str::to_owned).collect(),
        position: 0,
    };
    ui_selection_finish::finish(&mut s, &mut h).unwrap();
    compare(&s, v[6], "final finish");
    assert_eq!(h.position, h.events.len());
    s
}
