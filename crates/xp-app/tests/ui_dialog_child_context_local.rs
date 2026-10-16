use openxplane::{
    ui_callback_links::{self, Kind, Node, State},
    ui_callback_list::{self, List},
    ui_dialog_child_context::{self, Context, Services},
    ui_input_construct::{Block, Services as Allocator},
};
use std::collections::{BTreeMap, BTreeSet};
fn num(s: &str) -> u64 {
    s.parse().unwrap()
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn parse_graph(s: &str) -> State {
    let mut state = State::default();
    if s != "-" {
        for r in s.split(';') {
            let f: Vec<_> = r.split(':').collect();
            state.nodes.insert(
                num(f[0]),
                Node {
                    kind: if f[1] == "B" {
                        Kind::Container
                    } else {
                        Kind::Wrapper {
                            first: num(f[2]),
                            second: num(f[3]),
                        }
                    },
                    peers: if f[4] == "-" {
                        BTreeSet::new()
                    } else {
                        f[4].split(',').map(num).collect()
                    },
                },
            );
        }
    }
    state
}
fn graph(s: &State) -> String {
    let out = s
        .nodes
        .iter()
        .map(|(p, n)| {
            let (kind, a, b) = match n.kind {
                Kind::Container => ("B", 0, 0),
                Kind::Wrapper { first, second } => ("W", first, second),
            };
            let peers = n.peers.iter().map(u64::to_string).collect::<Vec<_>>();
            format!(
                "{p}:{kind}:{a}:{b}:{}",
                if peers.is_empty() {
                    "-".to_string()
                } else {
                    peers.join(",")
                }
            )
        })
        .collect::<Vec<_>>();
    if out.is_empty() {
        "-".to_string()
    } else {
        out.join(";")
    }
}
fn links(list: &List) -> String {
    list.links()
        .into_iter()
        .map(|(p, n, b)| format!("{p}:{n}:{b}"))
        .collect::<Vec<_>>()
        .join(";")
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
}
impl Allocator for Replay {
    fn allocate(&mut self, image: &[u8], bytes: usize) -> Result<Block, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "H");
        assert_eq!(num(&e[1]) as usize, bytes);
        assert_eq!(image, hex(&e[3]));
        Ok(Block {
            address: num(&e[2]),
            bytes: vec![self.poison; bytes],
        })
    }
}
impl ui_callback_links::Services for Replay {
    fn allocate(&mut self, bytes: usize) {
        let e = self.event();
        assert_eq!(e, vec!["A".to_string(), bytes.to_string()]);
    }
    fn free(&mut self, bytes: usize) {
        let e = self.event();
        assert_eq!(e, vec!["F".to_string(), bytes.to_string()]);
    }
}
impl ui_callback_list::Services for Replay {
    fn allocate_entry(&mut self, bytes: usize) -> Result<u64, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "L");
        assert_eq!(num(&e[1]) as usize, bytes);
        Ok(num(&e[2]))
    }
}
impl Services for Replay {
    fn refresh_context(&mut self, image: &mut [u8; 96]) -> Result<(), &'static str> {
        let e = self.event();
        assert_eq!(e[0], "R");
        assert_eq!(image.to_vec(), hex(&e[1]));
        image[0x38..0x58].copy_from_slice(&hex(&e[2]));
        image[0x58..0x60].copy_from_slice(&num(&e[3]).to_le_bytes());
        Ok(())
    }
}
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_DIALOG_CHILD_CONTEXT_VECTORS"]
fn native_child_constructor_with_real_retained_callback_lifetime() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_DIALOG_CHILD_CONTEXT_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut callbacks = State::default();
    let mut parents = BTreeMap::new();
    let mut objects = vec![];
    let mut parent_ids = vec![];
    let mut seed = 0;
    let mut initial = 0;
    let mut cases = 0;
    let mut steps = 0;
    for line in data.lines().filter(|l| !l.starts_with('#')) {
        let v: Vec<_> = line.split_whitespace().collect();
        if v[0] == "C" {
            seed = num(v[1]) as u8;
            initial = num(v[3]) as usize;
            objects = v[6].split(',').map(num).collect();
            parent_ids = v[7].split(',').map(num).collect();
            let heads: Vec<_> = v[8].split(',').map(num).collect();
            parents.clear();
            for i in 0..2 {
                parents.insert(
                    parent_ids[i],
                    List {
                        sentinel: heads[i],
                        entries: if v[9 + i] == "-" {
                            vec![]
                        } else {
                            v[9 + i].split(',').map(num).collect()
                        },
                    },
                );
            }
            callbacks = parse_graph(v[11]);
            cases += 1;
            continue;
        }
        assert_eq!(v[0], "T");
        assert_eq!(v[3], "->");
        let index = num(v[1]) as usize;
        let temporary = num(v[2]);
        let mut image = [seed ^ ((index as u8) * 0x11); 96];
        let mut replay = Replay {
            events: v[10].split('|').map(str::to_string).collect(),
            index: 0,
            poison: seed ^ 0x5a,
        };
        let result = ui_dialog_child_context::construct(
            &mut image,
            parent_ids[initial ^ index],
            Context {
                identity: objects[index],
                temporary,
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut replay,
        )
        .unwrap();
        assert_eq!(image.to_vec(), hex(v[4]), "case {cases}, child {index}");
        assert_eq!(result.base_sentinel.address, num(v[5]));
        assert_eq!(
            parents[&objects[index]].sentinel,
            result.base_sentinel.address
        );
        assert!(parents[&objects[index]].entries.is_empty());
        assert_eq!(result.base_sentinel.bytes, hex(v[6]));
        assert_eq!(
            result.callback_head,
            u64::from_le_bytes(image[0x28..0x30].try_into().unwrap())
        );
        assert_eq!(links(&parents[&parent_ids[0]]), v[7]);
        assert_eq!(links(&parents[&parent_ids[1]]), v[8]);
        assert_eq!(graph(&callbacks), v[9]);
        assert!(!callbacks.nodes.contains_key(&temporary));
        assert_eq!(replay.index, replay.events.len());
        steps += 1;
    }
    assert_eq!((cases, steps), (108, 216));
}
