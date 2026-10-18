use openxplane::{
    ui_callback_links::{self, Kind, Node},
    ui_callback_list::{self, List},
    ui_dialog_child_selection::{self, Context, Services, State},
    ui_input_construct::{Block, Services as Allocator},
    ui_interface_chain::{self, Node as Interface},
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
fn text(b: &[u8]) -> String {
    b.iter().map(|v| format!("{v:02x}")).collect()
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
fn chain(s: &State, addresses: &[u64]) -> String {
    s.interfaces
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            format!(
                "{}:{}:{}:{}",
                addresses[i],
                n.table,
                n.newer.map_or(0, |j| addresses[j]),
                n.older.map_or(0, |j| addresses[j])
            )
        })
        .collect::<Vec<_>>()
        .join("/")
}
struct Replay {
    events: Vec<String>,
    position: usize,
    poison: u8,
    addresses: Vec<u64>,
    old_head: u64,
    resource_present: bool,
}
impl Replay {
    fn event(&mut self) -> Vec<String> {
        let v = self.events[self.position]
            .split('~')
            .map(str::to_string)
            .collect();
        self.position += 1;
        v
    }
    fn snapshot(&self, s: &State) -> String {
        format!(
            "{};{};{};{};{};{}",
            text(&s.image),
            s.active_global,
            s.catalog_base,
            s.catalog_values
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(","),
            s.interfaces.head.map_or(0, |i| self.addresses[i]),
            chain(s, &self.addresses)
        )
    }
    fn effect(&mut self, s: &mut State, kind: &str, args: &str) -> i64 {
        let e = self.event();
        assert_eq!(e[0], kind, "event {}", self.position);
        assert_eq!(e[1], args);
        assert_eq!(
            e[2],
            self.snapshot(s),
            "{kind} before event {}",
            self.position
        );
        let after: Vec<_> = e[3].split(';').collect();
        s.image = hex(after[0]);
        s.active_global = num(after[1]);
        s.catalog_base = num(after[2]);
        s.catalog_values = after[3].split(',').map(num).collect();
        assert_eq!(
            e[3],
            self.snapshot(s),
            "{kind} after event {}",
            self.position
        );
        e[4].parse().unwrap()
    }
}
impl Allocator for Replay {
    fn allocate(&mut self, image: &[u8], bytes: usize) -> Result<Block, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "H");
        let a: Vec<_> = e[1].split(',').collect();
        assert_eq!(num(a[0]) as usize, bytes);
        let before: Vec<_> = e[2].split(';').collect();
        assert_eq!(image, hex(before[0]));
        let table = u64::from_le_bytes(image[0x20..0x28].try_into().unwrap());
        assert_eq!(
            num(before[1]),
            if table == ui_interface_chain::BASE_TABLE || table == 0x1426430c8 {
                *self.addresses.last().unwrap()
            } else {
                self.old_head
            }
        );
        Ok(Block {
            address: num(a[1]),
            bytes: vec![self.poison; bytes],
        })
    }
}
impl ui_callback_links::Services for Replay {
    fn allocate(&mut self, bytes: usize) {
        assert_eq!(self.event(), vec!["A".to_string(), bytes.to_string()]);
    }
    fn free(&mut self, bytes: usize) {
        assert_eq!(self.event(), vec!["D".to_string(), bytes.to_string()]);
    }
}
impl ui_callback_list::Services for Replay {
    fn allocate_entry(&mut self, bytes: usize) -> Result<u64, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "L");
        let a: Vec<_> = e[1].split(',').collect();
        assert_eq!(num(a[0]) as usize, bytes);
        Ok(num(a[1]))
    }
}
impl Services for Replay {
    fn bind_property(&mut self, s: &mut State, field: u64, enabled: u32, key: &str) {
        self.effect(s, "P", &format!("{field},{enabled},{key}"));
    }
    fn engine(&mut self, s: &mut State) -> u8 {
        self.effect(s, "E", "-") as u8
    }
    fn resource_present(&mut self, s: &mut State, index: u32) -> Result<bool, &'static str> {
        self.effect(s, "R", &index.to_string());
        Ok(self.resource_present)
    }
    fn alternate(&mut self, s: &mut State) {
        self.effect(s, "V", "-");
    }
    fn selection_value(&mut self, s: &mut State) -> u64 {
        self.effect(s, "T", "-") as u64
    }
    fn selected_index(&mut self, s: &mut State, token: u64) -> i32 {
        self.effect(s, "I", &token.to_string()) as i32
    }
    fn changed(&mut self, s: &mut State) {
        self.effect(s, "C", "-");
    }
    fn reset(&mut self, s: &mut State, arg: u32) {
        self.effect(s, "Z", &arg.to_string());
    }
    fn finish(&mut self, s: &mut State) {
        self.effect(s, "F", "-");
    }
}
#[test]
#[ignore = "requires original local corpus via OPENXPLANE_UI_DIALOG_CHILD_SELECTION_VECTORS"]
fn native_whole_selection_child_data_global_interface_clamping_variants_and_callbacks() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_DIALOG_CHILD_SELECTION_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut cases = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        assert_eq!(v[15], "->");
        let seed = num(v[1]) as u8;
        let inputs: Vec<i64> = v[2].split(',').map(|s| s.parse().unwrap()).collect();
        let identity = num(v[3]);
        let root_ids: Vec<_> = v[5].split(',').map(num).collect();
        let root_heads: Vec<_> = v[6].split(',').map(num).collect();
        let old = num(v[7]) as usize;
        let addresses: Vec<_> = v[8].split(',').map(num).collect();
        let bases: Vec<_> = v[9].split(',').map(num).collect();
        let mut nodes = (0..old)
            .map(|i| Interface {
                table: ui_interface_chain::BASE_TABLE,
                newer: i.checked_sub(1),
                older: if i + 1 < old { Some(i + 1) } else { None },
                free_requested: false,
            })
            .collect::<Vec<_>>();
        nodes.push(Interface::default());
        let mut s = State {
            image: vec![seed; 0x2d0],
            active_global: inputs[6] as u64,
            resource_index: 0,
            catalog_base: bases[0],
            catalog_values: v[10].split(',').map(num).collect(),
            interfaces: ui_interface_chain::State {
                head: if old == 0 { None } else { Some(0) },
                nodes,
            },
        };
        let mut callbacks = parse_graph(v[14]);
        let mut parents = BTreeMap::new();
        for i in 0..2 {
            parents.insert(
                root_ids[i],
                List {
                    sentinel: root_heads[i],
                    entries: if v[12 + i] == "-" {
                        vec![]
                    } else {
                        v[12 + i].split(',').map(num).collect()
                    },
                },
            );
        }
        let temps: [u64; 2] = v[27]
            .split(',')
            .map(num)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let mut replay = Replay {
            events: v[28].split('|').map(str::to_string).collect(),
            position: 0,
            poison: seed ^ 0x5a,
            addresses: addresses.clone(),
            old_head: if old == 0 { 0 } else { addresses[0] },
            resource_present: inputs[8] != 0,
        };
        let init = ui_dialog_child_selection::construct(
            &mut s,
            num(v[4]),
            Context {
                identity,
                interface: old,
                addresses: &addresses,
                temporaries: temps,
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut replay,
        )
        .unwrap();
        assert_eq!(s.image, hex(v[16]), "case {cases}");
        assert_eq!(init.base_sentinel.address, num(v[17]));
        assert_eq!(init.base_sentinel.bytes, hex(v[18]));
        assert_eq!(init.record_sentinel.address, num(v[19]));
        assert_eq!(init.record_sentinel.bytes, hex(v[20]));
        assert_eq!(s.interfaces.head.map(|i| addresses[i]), Some(num(v[21])));
        assert_eq!(chain(&s, &addresses), v[22]);
        assert_eq!(list(&parents[&root_ids[0]]), v[23]);
        assert_eq!(list(&parents[&root_ids[1]]), v[24]);
        assert_eq!(list(&parents[&identity]), v[25]);
        assert_eq!(graph(&callbacks), v[26]);
        assert_eq!(replay.position, replay.events.len());
        assert_eq!(s.active_global, num(v[29]));
        assert_eq!(s.catalog_base, num(v[30]));
        assert_eq!(
            s.catalog_values,
            v[31].split(',').map(num).collect::<Vec<_>>()
        );
        assert_eq!(
            init.callback_head,
            u64::from_le_bytes(s.image[0x40..0x48].try_into().unwrap())
        );
        cases += 1;
    }
    assert_eq!(cases, 920);
}
