//! Actual original functions, controlled settings/catalog/CRT/allocator leaves.
use openxplane::{
    ui_payload_move, ui_selection_dependencies as deps, ui_selection_reset_gate as gate,
    ui_selection_secondary,
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
fn check(m: &ui_payload_move::State, expected: &str) {
    for (p, b) in memory(expected) {
        assert_eq!(m.read(p, b.len()).unwrap(), b, "memory {p:x}");
    }
}
struct Replay {
    events: Vec<String>,
    position: usize,
    stack: u64,
}
impl Replay {
    fn invoke(
        &mut self,
        m: &mut ui_payload_move::State,
        f: u64,
        a: &[u64],
    ) -> Result<u64, &'static str> {
        let event = self
            .events
            .get(self.position)
            .expect("unexpected dependency service");
        self.position += 1;
        let v = event.split('~').collect::<Vec<_>>();
        assert_eq!(v.len(), 5);
        assert_eq!(v[0], "I");
        let call = v[1]
            .split(',')
            .map(|n| n.parse::<u64>().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(call[0], f, "function");
        assert_eq!(&call[1..], a, "args {f:x}");
        check(m, v[2]);
        for (p, b) in memory(v[3]) {
            m.write(p, &b)?;
        }
        Ok(v[4].parse().unwrap())
    }
}
impl deps::Services for Replay {
    fn invoke(&mut self, s: &mut deps::State, f: u64, a: &[u64]) -> Result<u64, &'static str> {
        self.invoke(&mut s.memory, f, a)
    }
}
impl gate::Services for Replay {
    fn compare_bytes(
        &mut self,
        s: &mut gate::State,
        l: u64,
        r: u64,
        n: u64,
    ) -> Result<i32, &'static str> {
        Ok(self.invoke(&mut s.memory, 0x1423c0180, &[l, r, n])? as i32)
    }
    fn secondary(&mut self, s: &mut gate::State, p: u64) -> Result<u64, &'static str> {
        ui_selection_secondary::with_dependencies(&mut s.memory, self.stack - 0x30, p, |m, f, a| {
            self.invoke(m, f, a)
        })
    }
}
#[test]
#[ignore = "requires original corpus via OPENXPLANE_UI_SELECTION_DEPENDENCIES_VECTORS"]
fn actual_mode_header_singleton_and_secondary_composition() {
    use std::io::BufRead;
    let f =
        std::fs::File::open(std::env::var("OPENXPLANE_UI_SELECTION_DEPENDENCIES_VECTORS").unwrap())
            .unwrap();
    let mut counts = [0; 5];
    for (line, row) in std::io::BufReader::new(f).lines().enumerate() {
        let row = row.unwrap();
        if !row.starts_with("V ") {
            continue;
        }
        let result = std::panic::catch_unwind(|| {
            let v = row.split_whitespace().collect::<Vec<_>>();
            assert_eq!(v.len(), 8);
            assert_eq!(v[4], "->");
            let args = v[2]
                .split(',')
                .map(|n| n.parse::<u64>().unwrap())
                .collect::<Vec<_>>();
            let mut s = deps::State {
                memory: ui_payload_move::State {
                    blocks: memory(v[3]),
                },
                stack: args[1],
            };
            let mut h = Replay {
                events: if v[7] == "-" {
                    vec![]
                } else {
                    v[7].split('|').map(str::to_owned).collect()
                },
                position: 0,
                stack: args[1],
            };
            let result = match v[1] {
                "L" => deps::singleton(&mut s, &mut h).unwrap(),
                "M" => deps::mode(&mut s, &mut h).unwrap(),
                "H" => deps::header(&mut s, &mut h, args[0]).unwrap(),
                "S" => {
                    s.memory.blocks.insert(s.stack - 0x70, vec![0; 96]);
                    ui_selection_secondary::with_dependencies(
                        &mut s.memory,
                        s.stack,
                        args[0],
                        |m, f, a| h.invoke(m, f, a),
                    )
                    .unwrap()
                }
                "G" => {
                    let mut g = gate::State {
                        identity: args[0],
                        memory: std::mem::take(&mut s.memory),
                    };
                    let result = gate::skip_reset(&mut g, &mut h).unwrap();
                    s.memory = g.memory;
                    u64::from(result)
                }
                _ => panic!("mode"),
            };
            assert_eq!(result, v[6].parse::<u64>().unwrap(), "full RAX");
            check(&s.memory, v[5]);
            assert_eq!(h.position, h.events.len(), "service count");
        });
        assert!(result.is_ok(), "original corpus line {}", line + 1);
        counts[match row.split_whitespace().nth(1).unwrap() {
            "L" => 0,
            "M" => 1,
            "H" => 2,
            "S" => 3,
            "G" => 4,
            _ => unreachable!(),
        }] += 1;
    }
    assert_eq!(counts, [48, 116, 160, 1536, 128]);
}
