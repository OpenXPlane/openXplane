use openxplane::{
    ui_callback_links::{self, Kind, Node},
    ui_callback_list::{self, List},
    ui_dialog_child_weather::{self, Context, Services},
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
fn parse_graph(s: &str) -> ui_callback_links::State {
    let mut state = ui_callback_links::State::default();
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
fn graph(s: &ui_callback_links::State) -> String {
    s.nodes
        .iter()
        .map(|(p, n)| {
            let (k, a, b) = match n.kind {
                Kind::Container => ("B", 0, 0),
                Kind::Wrapper { first, second } => ("W", first, second),
            };
            let peers = n.peers.iter().map(u64::to_string).collect::<Vec<_>>();
            format!(
                "{p}:{k}:{a}:{b}:{}",
                if peers.is_empty() {
                    "-".to_string()
                } else {
                    peers.join(",")
                }
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}
fn list(l: &List) -> String {
    l.links()
        .into_iter()
        .map(|(p, n, b)| format!("{p}:{n}:{b}"))
        .collect::<Vec<_>>()
        .join(";")
}

struct Replay {
    events: Vec<String>,
    position: usize,
    poison: u8,
}
impl Replay {
    fn event(&mut self) -> Vec<String> {
        let e = self.events[self.position]
            .split('~')
            .map(str::to_string)
            .collect();
        self.position += 1;
        e
    }
    fn effect(&mut self, image: &mut [u8], kind: &str, args: &str) -> String {
        let e = self.event();
        assert_eq!(e[0], kind, "event {}", self.position);
        assert_eq!(e[1], args);
        assert_eq!(image, hex(&e[2]), "before {kind}");
        image.copy_from_slice(&hex(&e[3]));
        e[4].clone()
    }
}
impl Allocator for Replay {
    fn allocate(&mut self, image: &[u8], n: usize) -> Result<Block, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "H");
        let a: Vec<_> = e[1].split(',').collect();
        assert_eq!(num(a[0]) as usize, n);
        assert_eq!(image, hex(&e[2]));
        Ok(Block {
            address: num(a[1]),
            bytes: vec![self.poison; n],
        })
    }
}
impl ui_callback_links::Services for Replay {
    fn allocate(&mut self, n: usize) {
        assert_eq!(self.event(), vec!["A".to_string(), n.to_string()]);
    }
    fn free(&mut self, n: usize) {
        assert_eq!(self.event(), vec!["D".to_string(), n.to_string()]);
    }
}
impl ui_callback_list::Services for Replay {
    fn allocate_entry(&mut self, n: usize) -> Result<u64, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "L");
        let a: Vec<_> = e[1].split(',').collect();
        assert_eq!(num(a[0]) as usize, n);
        Ok(num(a[1]))
    }
}
impl Services for Replay {
    fn weather(&mut self, image: &mut [u8], offset: usize, index: i32, bits: u32) {
        self.effect(image, "W", &format!("{offset},{index},{bits}"));
    }
    fn altitude(&mut self, image: &mut [u8], parent: u64) -> u32 {
        self.effect(image, "A0", &parent.to_string())
            .parse()
            .unwrap()
    }
    fn selection(&mut self, image: &mut [u8], parent: u64) -> [u8; 32] {
        hex(&self.effect(image, "S", &parent.to_string()))
            .try_into()
            .unwrap()
    }
    fn refresh(&mut self, image: &mut [u8]) {
        self.effect(image, "R", "-");
    }
    fn special(&mut self, image: &mut [u8]) {
        self.effect(image, "F", "-");
    }
}
#[test]
#[ignore = "requires local corpus via OPENXPLANE_UI_DIALOG_CHILD_WEATHER_VECTORS"]
fn whole_native_weather_child_initialization_copying_rebind_and_callback_lifetimes() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_DIALOG_CHILD_WEATHER_VECTORS").unwrap(),
    )
    .unwrap();
    let mut cases = 0;
    for row in data.lines().filter(|r| r.starts_with("C ")) {
        let v: Vec<_> = row.split_whitespace().collect();
        assert_eq!(v.len(), 20);
        assert_eq!(v[10], "->");
        let seed = num(v[1]) as u8;
        let id = num(v[2]);
        let root_ids: Vec<_> = v[5].split(',').map(num).collect();
        let heads: Vec<_> = v[6].split(',').map(num).collect();
        let mut parents = BTreeMap::new();
        for i in 0..2 {
            parents.insert(
                root_ids[i],
                List {
                    sentinel: heads[i],
                    entries: if v[7 + i] == "-" {
                        vec![]
                    } else {
                        v[7 + i].split(',').map(num).collect()
                    },
                },
            );
        }
        let mut callbacks = parse_graph(v[9]);
        let temps = v[18]
            .split(',')
            .map(num)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let mut replay = Replay {
            events: v[19].split('|').map(str::to_string).collect(),
            position: 0,
            poison: seed ^ 0x5a,
        };
        let mut image = vec![seed; 0x3e8];
        let init = ui_dialog_child_weather::construct(
            &mut image,
            num(v[3]),
            num(v[4]),
            Context {
                identity: id,
                temporaries: temps,
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut replay,
        )
        .unwrap();
        assert_eq!(image, hex(v[11]), "case {cases}");
        assert_eq!(init.base_sentinel.address, num(v[12]));
        assert_eq!(init.base_sentinel.bytes, hex(v[13]));
        assert_eq!(list(&parents[&root_ids[0]]), v[14]);
        assert_eq!(list(&parents[&root_ids[1]]), v[15]);
        assert_eq!(list(&parents[&id]), v[16]);
        assert_eq!(graph(&callbacks), v[17]);
        assert_eq!(replay.position, replay.events.len());
        assert_eq!(
            init.callback_head,
            u64::from_le_bytes(image[0x28..0x30].try_into().unwrap())
        );
        cases += 1;
    }
    assert_eq!(cases, 720);
}
