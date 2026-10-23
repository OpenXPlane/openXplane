//! Strict non-stack replay; all secondary/primary/member/equality defaults execute.
#![allow(dead_code)]
use openxplane::{
    ui_payload_move, ui_selection_reset_gate as gate,
    ui_selection_secondary::{self, Services},
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
struct Replay {
    events: Vec<String>,
    position: usize,
}
impl Services for Replay {
    fn invoke(
        &mut self,
        m: &mut ui_payload_move::State,
        f: u64,
        a: &[u64],
    ) -> Result<u64, &'static str> {
        let fields = self
            .events
            .get(self.position)
            .expect("unexpected secondary provider")
            .split('~')
            .collect::<Vec<_>>();
        self.position += 1;
        assert_eq!(fields.len(), 5);
        assert_eq!(fields[0], "I");
        let mut call = fields[1].split(',');
        assert_eq!(call.next().unwrap().parse::<u64>().unwrap(), f);
        assert_eq!(
            call.map(|v| v.parse::<u64>().unwrap()).collect::<Vec<_>>(),
            a,
            "provider {f:x}"
        );
        assert_eq!(m.blocks, memory(fields[2]), "before {f:x}");
        m.blocks = memory(fields[3]);
        Ok(fields[4].parse().unwrap())
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
    fn secondary(&mut self, s: &mut gate::State, receiver: u64) -> Result<u64, &'static str> {
        ui_selection_secondary::secondary(&mut s.memory, self, receiver)
    }
}
pub fn run_row(row: &str) {
    let v = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(v.len(), 8);
    assert_eq!(v[0], "V");
    assert_eq!(v[4], "->");
    let args = v[2]
        .split(',')
        .map(|a| a.parse::<u64>().unwrap())
        .collect::<Vec<_>>();
    let mut m = ui_payload_move::State {
        blocks: memory(v[3]),
    };
    let mut h = Replay {
        events: if v[7] == "-" {
            vec![]
        } else {
            v[7].split('|').map(str::to_owned).collect()
        },
        position: 0,
    };
    let result = match v[1] {
        "S" => {
            assert_eq!(args.len(), 1);
            ui_selection_secondary::secondary(&mut m, &mut h, args[0]).unwrap()
        }
        "P" => {
            assert_eq!(args.len(), 1);
            ui_selection_secondary::primary(&mut m, &mut h, args[0]).unwrap()
        }
        "E" => {
            assert_eq!(args.len(), 2);
            ui_selection_secondary::equal(&mut m, &mut h, args[0], args[1]).unwrap()
        }
        "G" => {
            assert_eq!(args.len(), 1);
            let mut s = gate::State {
                identity: args[0],
                memory: m,
            };
            let value = gate::skip_reset(&mut s, &mut h).unwrap();
            m = s.memory;
            u64::from(value)
        }
        _ => panic!("unknown mode"),
    };
    assert_eq!(result, v[6].parse::<u64>().unwrap(), "result mode {}", v[1]);
    assert_eq!(m.blocks, memory(v[5]), "final mode {}", v[1]);
    assert_eq!(h.position, h.events.len());
}
