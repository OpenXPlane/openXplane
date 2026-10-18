use openxplane::{ui_payload_move, ui_selection_reset_gate as gate};
use std::collections::VecDeque;

fn memory(encoded: &str) -> ui_payload_move::State {
    let mut out = ui_payload_move::State::default();
    for block in encoded.split('/') {
        let (p, hex) = block.split_once(':').unwrap();
        let bytes = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert!(out.blocks.insert(p.parse().unwrap(), bytes).is_none());
    }
    out
}
fn assert_memory(s: &gate::State, expected: &str) {
    assert_eq!(s.memory.blocks, memory(expected).blocks);
}
struct Replay {
    events: VecDeque<String>,
    mutation: u8,
    decoy: u64,
}
impl Replay {
    fn effect(&self, s: &mut gate::State) {
        if self.mutation != 0 {
            s.memory
                .write(
                    s.identity + 0xc0,
                    &[if self.mutation == 1 { 0 } else { 255 }],
                )
                .unwrap();
            s.memory
                .write(s.identity + 0xc8, &self.decoy.to_le_bytes())
                .unwrap();
            s.memory
                .write(gate::RESOURCE_INDEX, &1u32.to_le_bytes())
                .unwrap();
        }
    }
}
impl gate::Services for Replay {
    fn compare_bytes(
        &mut self,
        s: &mut gate::State,
        left: u64,
        right: u64,
        bytes: u64,
    ) -> Result<i32, &'static str> {
        let event = self
            .events
            .pop_front()
            .expect("missing original memcmp request");
        let v = event.split('~').collect::<Vec<_>>();
        assert_eq!(v.len(), 7);
        assert_eq!(v[0], "M");
        assert_eq!(
            [left, right, bytes],
            [
                v[1].parse().unwrap(),
                v[2].parse().unwrap(),
                v[3].parse().unwrap()
            ]
        );
        assert_memory(s, v[4]);
        let result = gate::compare_bytes(s, left, right, bytes)?;
        self.effect(s);
        assert_memory(s, v[5]);
        assert_eq!(result, v[6].parse::<i32>().unwrap());
        Ok(result)
    }
    fn secondary(&mut self, s: &mut gate::State, receiver: u64) -> Result<u64, &'static str> {
        let event = self
            .events
            .pop_front()
            .expect("missing original secondary request");
        let v = event.split('~').collect::<Vec<_>>();
        assert_eq!(v.len(), 5);
        assert_eq!(v[0], "S");
        assert_eq!(receiver, v[1].parse::<u64>().unwrap());
        assert_memory(s, v[2]);
        self.effect(s);
        assert_memory(s, v[3]);
        Ok(v[4].parse().unwrap())
    }
}
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_SELECTION_RESET_GATE_VECTORS"]
fn original_reset_gate_primary_comparison_and_variant_tags() {
    let corpus = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_SELECTION_RESET_GATE_VECTORS").unwrap(),
    )
    .unwrap();
    let (mut gates, mut tags) = (0, 0);
    for row in corpus
        .lines()
        .filter(|r| r.starts_with("C ") || r.starts_with("T "))
    {
        let v = row.split_whitespace().collect::<Vec<_>>();
        if v[0] == "T" {
            assert_eq!(v.len(), 6);
            assert_eq!(v[4], "->");
            let receiver = v[1].parse::<u64>().unwrap();
            let mut image = vec![v[2].parse::<u8>().unwrap(); 0x90];
            image[0x68] = v[3].parse().unwrap();
            let mut mem = ui_payload_move::State::default();
            mem.blocks.insert(receiver, image.clone());
            assert_eq!(
                gate::variant_is_two_or_three(&mem, receiver).unwrap(),
                v[5].parse::<u64>().unwrap()
            );
            assert_eq!(mem.read(receiver, image.len()).unwrap(), image);
            tags += 1;
        } else {
            assert_eq!(v.len(), 9);
            assert_eq!(v[5], "->");
            let mut s = gate::State {
                identity: v[1].parse().unwrap(),
                memory: memory(v[4]),
            };
            let mut h = Replay {
                mutation: v[2].parse().unwrap(),
                decoy: v[3].parse().unwrap(),
                events: if v[8] == "-" {
                    VecDeque::new()
                } else {
                    v[8].split('|').map(str::to_owned).collect()
                },
            };
            assert_eq!(
                u8::from(gate::skip_reset(&mut s, &mut h).unwrap()),
                v[6].parse::<u8>().unwrap(),
                "case {}",
                gates
            );
            assert_memory(&s, v[7]);
            assert!(h.events.is_empty());
            gates += 1;
        }
    }
    assert_eq!((gates, tags), (1536, 768));
}
