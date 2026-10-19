//! Strict whole reset caller replay. Provider bodies remain controlled.
#![allow(dead_code)]
#[path = "ui_selection_initial_replay.rs"]
mod initial_replay;
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
    fn initial_callback(&mut self, s: &mut State) -> Result<(), &'static str> {
        if !self.events[self.position].starts_with("K~") {
            self.invoke(s, 0x1410fe0f0, &[s.identity + 0x38, 0x1e1b9, 0])?;
            return Ok(());
        }
        let row = self.events[self.position][2..]
            .replace('@', " ")
            .replace('%', "|");
        self.position += 1;
        let fields = row.split_whitespace().collect::<Vec<_>>();
        assert_eq!(fields.len(), 9);
        assert_eq!(fields[4], "0", "composed callback must use actual helpers");
        assert_eq!(fields[1].parse::<u64>().unwrap(), s.identity);
        assert_eq!(fields[2].parse::<u64>().unwrap(), s.stack - 0xa0);
        assert_eq!(fields[3].parse::<u64>().unwrap(), 0x1e1b9);
        let image = s.memory.read(s.identity, 0x2d0)?;
        let runtime = initial_replay::run_row_with_reset(
            &row,
            Some(&image),
            &s.memory.blocks,
            Some(reset_from_callback),
        );
        for (p, b) in &mut s.memory.blocks {
            *b = runtime.memory.read(*p, b.len())?;
        }
        Ok(())
    }
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

fn reset_from_callback(row: &str, s: &mut openxplane::ui_selection_initial::State) {
    let fields = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(fields.len(), 8);
    assert_eq!(fields[1].parse::<u64>().unwrap(), s.identity);
    assert_eq!(fields[2].parse::<u64>().unwrap(), s.stack - 0xe0);
    assert_eq!(fields[3], "1");
    assert_eq!(s.memory.read(s.identity + 0xc0, 1).unwrap(), [0]);
    assert!(
        fields[7].starts_with("K~C@"),
        "nested reset must execute real callback"
    );
    let image = s.memory.read(s.identity, 0x2d0).unwrap();
    let runtime = run_row(row, Some(&image), None, &s.memory.blocks);
    for (p, b) in &mut s.memory.blocks {
        *b = runtime.memory.read(*p, b.len()).unwrap();
    }
}
