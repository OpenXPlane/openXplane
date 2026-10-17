use openxplane::{
    ui_input_construct::Block,
    ui_payload_move::{self, Services, State},
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
fn state(s: &str) -> State {
    State {
        blocks: s
            .split('/')
            .map(|r| {
                let (p, b) = r.split_once(':').unwrap();
                (num(p), hex(b))
            })
            .collect(),
    }
}
fn snapshot(s: &State) -> String {
    s.blocks
        .iter()
        .map(|(p, b)| {
            format!(
                "{p}:{}",
                b.iter().map(|v| format!("{v:02x}")).collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("/")
}
struct Replay {
    events: Vec<String>,
    index: usize,
}
impl Replay {
    fn event(&mut self) -> Vec<String> {
        let e = self.events[self.index]
            .split('~')
            .map(str::to_string)
            .collect();
        self.index += 1;
        e
    }
}
impl Services for Replay {
    fn allocate_vector(&mut self, s: &mut State, bytes: usize) -> Result<Block, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "A");
        let (n, p) = e[1].split_once(',').unwrap();
        assert_eq!(num(n) as usize, bytes);
        assert_eq!(snapshot(s), e[2]);
        *s = state(&e[3]);
        Ok(Block {
            address: num(p),
            bytes: hex(&e[4]),
        })
    }
    fn free(&mut self, s: &mut State, p: u64, n: Option<usize>) -> Result<(), &'static str> {
        let e = self.event();
        assert_eq!(e[0], "F");
        assert_eq!(
            e[1],
            format!("{p},{}", n.map_or("-".to_string(), |v| v.to_string()))
        );
        assert_eq!(snapshot(s), e[2]);
        *s = state(&e[3]);
        Ok(())
    }
}
#[test]
#[ignore = "requires local corpus via OPENXPLANE_UI_PAYLOAD_MOVE_VECTORS"]
fn whole_native_payload_optional_string_vector_ownership_growth_and_self_move() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_PAYLOAD_MOVE_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        let v: Vec<_> = row.split_whitespace().collect();
        assert_eq!(v.len(), 7);
        assert_eq!(v[4], "->");
        let mut s = state(v[3]);
        let mut replay = Replay {
            events: if v[6] == "-" {
                vec![]
            } else {
                v[6].split('|').map(str::to_string).collect()
            },
            index: 0,
        };
        ui_payload_move::assign(&mut s, num(v[1]), num(v[2]), &mut replay).unwrap();
        assert_eq!(snapshot(&s), v[5], "case {cases}");
        assert_eq!(replay.index, replay.events.len());
        cases += 1;
    }
    assert_eq!(cases, 720);
}
