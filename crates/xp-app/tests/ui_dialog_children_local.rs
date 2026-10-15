use openxplane::{
    ui_dialog_children::{Services, State},
    ui_input_construct::Block,
};
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn num(s: &str) -> u64 {
    s.parse().unwrap()
}
struct Replay {
    events: Vec<String>,
    index: usize,
    poison: u8,
}
impl Replay {
    fn event(&mut self) -> Vec<String> {
        let result = self.events[self.index]
            .split(':')
            .map(str::to_string)
            .collect();
        self.index += 1;
        result
    }
    fn snapshot(&self, state: &State, expected: &str) {
        let v: Vec<_> = expected.split('~').collect();
        assert_eq!(
            state.bundle.to_vec(),
            hex(v[0]),
            "bundle event {}",
            self.index
        );
        assert_eq!(state.blocks.len(), v.len() - 1);
        for r in &v[1..] {
            let (p, b) = r.split_once('=').unwrap();
            assert_eq!(state.blocks[&num(p)], hex(b), "block event {}", self.index);
        }
    }
}
fn q(state: &mut State, p: u64, off: usize, value: u64) {
    state.blocks.get_mut(&p).unwrap()[off..off + 8].copy_from_slice(&value.to_le_bytes());
}
impl Services for Replay {
    fn allocate(&mut self, state: &mut State, bytes: usize) -> Result<Option<Block>, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "A");
        assert_eq!(num(&e[1]) as usize, bytes);
        self.snapshot(state, &e[3]);
        let p = num(&e[2]);
        Ok(if p == 0 {
            None
        } else {
            Some(Block {
                address: p,
                bytes: vec![self.poison; bytes],
            })
        })
    }
    fn construct(
        &mut self,
        state: &mut State,
        kind: u8,
        p: u64,
        args: &[u64],
    ) -> Result<u64, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "N");
        assert_eq!(num(&e[1]) as u8, kind);
        assert_eq!(num(&e[2]), p);
        assert_eq!(args, e[3].split(',').map(num).collect::<Vec<_>>());
        self.snapshot(state, &e[5]);
        q(state, p, 0, 0x11110000 + kind as u64);
        q(state, p, 8, args[0]);
        q(state, p, 16, *args.get(1).unwrap_or(&0));
        Ok(num(&e[4]))
    }
    fn resource(&mut self, state: &mut State, index: u32) -> Result<u64, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "R");
        assert_eq!(num(&e[1]) as u32, index);
        self.snapshot(state, &e[3]);
        Ok(num(&e[2]))
    }
    fn refresh(&mut self, state: &mut State, p: u64) -> Result<(), &'static str> {
        let e = self.event();
        assert_eq!(e[0], "Z");
        assert_eq!(num(&e[1]), p);
        self.snapshot(state, &e[3]);
        let value = num(&e[2]);
        q(state, p, 0x98, value);
        state.blocks.get_mut(&p).unwrap()[0x28] = value as u8;
        Ok(())
    }
}
#[test]
#[ignore = "requires native local corpus via OPENXPLANE_UI_DIALOG_CHILDREN_VECTORS"]
fn native_whole_outer_factory_with_controlled_child_bodies() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_DIALOG_CHILDREN_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let seed = num(v[1]) as u8;
        assert_eq!(v[5], "->");
        let mut state = State::new([seed; 40]);
        let mut replay = Replay {
            events: v[7].split('|').map(str::to_string).collect(),
            index: 0,
            poison: seed ^ 0x5a,
        };
        state.construct(&mut replay).unwrap();
        assert_eq!(replay.index, replay.events.len());
        replay.snapshot(&state, v[6]);
        count += 1;
    }
    assert_eq!(count, 576);
}
