use openxplane::{
    ui_callback_links,
    ui_dialog_child_root::{self, Context, Services, State},
    ui_input_construct::Block,
};
use std::collections::BTreeMap;
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn blocks(s: &str) -> Vec<Block> {
    if s == "-" {
        return vec![];
    }
    s.split('/')
        .map(|r| {
            let (p, b) = r.split_once(':').unwrap();
            Block {
                address: p.parse().unwrap(),
                bytes: hex(b),
            }
        })
        .collect()
}
fn check(image: &[u8], heap: &[Block], snapshot: &str) {
    let (a, b) = snapshot.split_once(';').unwrap();
    assert_eq!(image, hex(a));
    let expected = blocks(b);
    assert_eq!(heap.len(), expected.len());
    for (a, b) in heap.iter().zip(expected) {
        assert_eq!(a.address, b.address);
        assert_eq!(a.bytes, b.bytes);
    }
}
struct Replay {
    events: Vec<Vec<String>>,
    index: usize,
    poison: u8,
    native_nested: bool,
}
impl Replay {
    fn next(&mut self) -> Vec<String> {
        let e = self.events[self.index].clone();
        self.index += 1;
        e
    }
    fn refresh(&mut self, kind: &str, image: &mut [u8], heap: &mut [Block], id: u64) {
        let e = self.next();
        assert_eq!(e[0], kind);
        assert_eq!(e[1].parse::<u64>().unwrap(), id);
        check(image, heap, &e[2]);
        if kind == "N" && self.native_nested {
            let raw = &mut image[0x268..0x3c0];
            let head = u64::from_le_bytes(raw[8..16].try_into().unwrap());
            let mut links = BTreeMap::from([(head, head)]);
            openxplane::ui_nested_refresh::refresh_image(
                raw, &mut 0, &mut 0, &mut links, &mut Fresh,
            )
            .unwrap();
            assert_eq!(links, BTreeMap::from([(head, head)]));
            check(image, heap, &e[3]);
            return;
        }
        let (a, b) = e[3].split_once(';').unwrap();
        image.copy_from_slice(&hex(a));
        let after = blocks(b);
        assert_eq!(heap.len(), after.len());
        for (a, b) in heap.iter_mut().zip(after) {
            assert_eq!(a.address, b.address);
            assert_eq!(a.bytes.len(), b.bytes.len());
            a.bytes = b.bytes;
        }
    }
}
impl Services for Replay {
    fn allocate(
        &mut self,
        image: &[u8],
        heap: &[Block],
        size: usize,
    ) -> Result<Block, &'static str> {
        let e = self.next();
        assert_eq!(e[0], "A");
        let (n, p) = e[1].split_once(',').unwrap();
        assert_eq!(n.parse::<usize>().unwrap(), size);
        check(image, heap, &e[2]);
        Ok(Block {
            address: p.parse().unwrap(),
            bytes: vec![self.poison; size],
        })
    }
    fn refresh_nested(
        &mut self,
        image: &mut [u8],
        heap: &mut Vec<Block>,
        id: u64,
    ) -> Result<(), &'static str> {
        self.refresh("N", image, heap, id);
        Ok(())
    }
    fn refresh_root(
        &mut self,
        image: &mut [u8],
        heap: &mut Vec<Block>,
        id: u64,
    ) -> Result<(), &'static str> {
        self.refresh("R", image, heap, id);
        Ok(())
    }
}
#[test]
#[ignore = "requires local original-generated corpus; two refresh bodies controlled"]
fn native_whole_root_constructor_snapshots_and_all_seven_blocks() {
    replay_corpus("OPENXPLANE_UI_DIALOG_CHILD_ROOT_VECTORS", false);
}
#[test]
#[ignore = "requires local combined corpus; original nested refresh, controlled engine0/root refresh"]
fn native_whole_root_with_original_nested_refresh() {
    replay_corpus("OPENXPLANE_UI_ROOT_NESTED_REFRESH_VECTORS", true);
}
fn replay_corpus(env: &str, native_nested: bool) {
    let corpus = std::fs::read_to_string(std::env::var(env).unwrap()).unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        let v: Vec<_> = row.split_whitespace().collect();
        assert_eq!(v.len(), 10);
        let seed: u8 = v[1].parse().unwrap();
        let id: u64 = v[4].parse().unwrap();
        let mut state = State::new(vec![seed; 0x3e0]);
        let mut callbacks = ui_callback_links::State::default();
        let mut parents = BTreeMap::new();
        let mut replay = Replay {
            events: v[9]
                .split('|')
                .map(|s| s.split('~').map(str::to_string).collect())
                .collect(),
            index: 0,
            poison: seed ^ 0x5a,
            native_nested,
        };
        ui_dialog_child_root::construct(
            &mut state,
            v[2].parse().unwrap(),
            Context {
                identity: id,
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut replay,
        )
        .unwrap();
        check(&state.image, &state.blocks, &format!("{};{}", v[6], v[7]));
        assert_eq!(
            state
                .blocks
                .iter()
                .map(|b| b.bytes.len())
                .collect::<Vec<_>>(),
            vec![120, 32, 120, 72, 72, 120, 40]
        );
        assert_eq!(replay.index, replay.events.len());
        assert_eq!(parents.len(), 3);
        for r in v[8].split('/') {
            let f: Vec<u64> = r.split(':').map(|n| n.parse().unwrap()).collect();
            let p = &parents[&f[0]];
            assert_eq!(p.sentinel, f[1]);
            assert_eq!(p.entries.len() as u64, f[2]);
        }
        assert_eq!(callbacks.nodes.len(), 1);
        let node = &callbacks.nodes[&(id + 0x288)];
        assert!(matches!(node.kind, ui_callback_links::Kind::Container));
        assert!(node.peers.is_empty());
        cases += 1;
    }
    assert_eq!(cases, 192);
}

struct Fresh;
impl openxplane::ui_nested_refresh::Services for Fresh {
    fn engine(&mut self, _: &mut openxplane::ui_nested_refresh::State) -> u8 {
        0
    }
    fn release_item(&mut self, _: &mut openxplane::ui_nested_refresh::State, _: u64) {
        panic!("fresh vector")
    }
    fn resource(&mut self, _: &mut openxplane::ui_nested_refresh::State, _: u32) -> u64 {
        panic!("engine0")
    }
    fn available(&self, _: u64) -> bool {
        panic!("engine0")
    }
    fn assign_resource(&mut self, _: &mut openxplane::ui_nested_refresh::State, _: u64) {
        panic!("engine0")
    }
    fn settings(&mut self, _: &mut openxplane::ui_nested_refresh::State, _: u32) -> u64 {
        panic!("engine0")
    }
    fn setting_mode(&self, _: u64) -> i32 {
        panic!("engine0")
    }
    fn temporary(
        &mut self,
        _: &mut openxplane::ui_nested_refresh::State,
        _: u64,
    ) -> openxplane::ui_nested_refresh::Temporary {
        panic!("engine0")
    }
    fn assign_payload(
        &mut self,
        _: &mut openxplane::ui_nested_refresh::State,
        _: &mut openxplane::ui_nested_refresh::Temporary,
    ) {
        panic!("engine0")
    }
    fn free_temporary_vector(&mut self, _: &mut openxplane::ui_nested_refresh::State, _: u64) {
        panic!("engine0")
    }
    fn destroy_optional(
        &mut self,
        _: &mut openxplane::ui_nested_refresh::State,
        _: &mut openxplane::ui_nested_refresh::Temporary,
    ) {
        panic!("engine0")
    }
    fn notify(&mut self, _: &mut openxplane::ui_nested_refresh::State, _: u64) {
        panic!("fresh listeners")
    }
}
