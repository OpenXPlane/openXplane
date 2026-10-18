//! Whole original change notification comparison, including callback effects.
#![allow(dead_code)]
use openxplane::{
    ui_input_construct::Block,
    ui_payload_move,
    ui_selection_changed::{self, Services, State},
};
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
    b.iter().map(|v| format!("{v:02x}")).collect()
}
fn parse(s: &str, id: u64) -> State {
    State {
        identity: id,
        memory: ui_payload_move::State {
            blocks: s
                .split('/')
                .map(|r| {
                    let (p, b) = r.split_once(':').unwrap();
                    (num(p), hex(b))
                })
                .collect(),
        },
    }
}
fn snapshot(s: &State) -> String {
    s.memory
        .blocks
        .iter()
        .map(|(p, b)| format!("{p}:{}", text(b)))
        .collect::<Vec<_>>()
        .join("/")
}
fn compare(a: &str, b: &str, label: &str) {
    if a != b {
        let p = a
            .bytes()
            .zip(b.bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(a.len().min(b.len()));
        panic!("{label}: mismatch at{p}; lengths{}/{}", a.len(), b.len());
    }
}
struct Replay {
    events: Vec<String>,
    position: usize,
}
impl Services for Replay {
    fn notify(&mut self, s: &mut State, wrapper: u64) -> Result<(), &'static str> {
        let f = self.events[self.position].split('~').collect::<Vec<_>>();
        self.position += 1;
        assert_eq!(f.len(), 4);
        assert_eq!(f[0], "I");
        assert_eq!(num(f[1]), wrapper);
        compare(&snapshot(s), f[2], "before notify");
        *s = parse(f[3], s.identity);
        Ok(())
    }
}
pub fn run_row(row: &str, image: Option<&[u8]>, base: Option<&Block>) -> State {
    let f = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(f.len(), 6);
    assert_eq!(f[3], "->");
    let id = num(f[1]);
    let mut s = parse(f[2], id);
    let mut raw = s.memory.read(id, 0x2d0).unwrap();
    if let Some(expected) = image {
        assert_eq!(raw, expected);
    }
    if let Some(base) = base {
        assert_eq!(
            u64::from_le_bytes(raw[8..16].try_into().unwrap()),
            base.address
        );
        assert_eq!(
            s.memory.read(base.address, base.bytes.len()).unwrap(),
            base.bytes
        );
    }
    let mut replay = Replay {
        events: if f[5] == "-" {
            vec![]
        } else {
            f[5].split('|').map(str::to_string).collect()
        },
        position: 0,
    };
    ui_selection_changed::changed_image(&mut raw, &mut s, &mut replay).unwrap();
    compare(&snapshot(&s), f[4], "final selection change");
    assert_eq!(raw, s.memory.read(id, 0x2d0).unwrap());
    assert_eq!(replay.position, replay.events.len());
    s
}
