use openxplane::{
    ui_callback_links::{self, Kind, Node, State as Callbacks},
    ui_callback_list::{self, List},
    ui_context_refresh::{self, State as RefreshState},
    ui_dialog_child_context::{self, Context, Services},
    ui_input_construct::{Block, Services as Allocator},
};
use std::collections::{BTreeMap, BTreeSet};
#[path = "support/context_refresh_replay.rs"]
mod replay;
use replay::{Replay, hex, num, parse_links};
fn parse_graph(s: &str) -> Callbacks {
    let mut state = Callbacks::default();
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
fn graph(s: &Callbacks) -> String {
    s.nodes
        .iter()
        .map(|(p, n)| {
            let (kind, a, b) = match n.kind {
                Kind::Container => ("B", 0, 0),
                Kind::Wrapper { first, second } => ("W", first, second),
            };
            let ps = n.peers.iter().map(u64::to_string).collect::<Vec<_>>();
            format!(
                "{p}:{kind}:{a}:{b}:{}",
                if ps.is_empty() {
                    "-".to_string()
                } else {
                    ps.join(",")
                }
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}
fn links(l: &List) -> String {
    l.links()
        .into_iter()
        .map(|(p, n, b)| format!("{p}:{n}:{b}"))
        .collect::<Vec<_>>()
        .join(";")
}
struct Combined {
    events: Vec<String>,
    position: usize,
    poison: u8,
    refresh: Replay,
    global: u64,
    next: BTreeMap<u64, u64>,
}
impl Combined {
    fn event(&mut self) -> Vec<String> {
        let e = self.events[self.position]
            .split(':')
            .map(str::to_string)
            .collect();
        self.position += 1;
        e
    }
}
impl Allocator for Combined {
    fn allocate(&mut self, image: &[u8], bytes: usize) -> Result<Block, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "H");
        assert_eq!(num(&e[1]) as usize, bytes);
        assert_eq!(hex(&e[3]), image);
        Ok(Block {
            address: num(&e[2]),
            bytes: vec![self.poison; bytes],
        })
    }
}
impl ui_callback_links::Services for Combined {
    fn allocate(&mut self, bytes: usize) {
        assert_eq!(self.event(), vec!["A".to_string(), bytes.to_string()]);
    }
    fn free(&mut self, bytes: usize) {
        assert_eq!(self.event(), vec!["F".to_string(), bytes.to_string()]);
    }
}
impl ui_callback_list::Services for Combined {
    fn allocate_entry(&mut self, bytes: usize) -> Result<u64, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "L");
        assert_eq!(num(&e[1]) as usize, bytes);
        Ok(num(&e[2]))
    }
}
impl Services for Combined {
    fn refresh_context(&mut self, image: &mut [u8; 96]) -> Result<(), &'static str> {
        let e = self.event();
        assert_eq!(e[0], "R");
        assert_eq!(hex(&e[1]), image.to_vec());
        let head = u64::from_le_bytes(image[8..16].try_into().unwrap());
        self.next.insert(head, head);
        ui_context_refresh::refresh_image(
            image,
            &mut self.global,
            &mut self.next,
            &mut self.refresh,
        )?;
        let e = self.event();
        assert_eq!(e[0], "S");
        assert_eq!(
            e[1],
            replay::state(&RefreshState {
                image: *image,
                active_global: self.global,
                next: self.next.clone()
            })
        );
        Ok(())
    }
}
#[test]
#[ignore = "requires combined original corpus via OPENXPLANE_UI_CHILD_REFRESH_COMBINED_VECTORS"]
fn native_child_refresh_and_retained_callback_compose_in_one_execution() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_CHILD_REFRESH_COMBINED_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        assert_eq!(v[20], "->");
        let ids: Vec<_> = v[15].split(',').map(num).collect();
        let heads: Vec<_> = v[16].split(',').map(num).collect();
        let mut parents = BTreeMap::new();
        for i in 0..2 {
            parents.insert(
                ids[i],
                List {
                    sentinel: heads[i],
                    entries: if v[17 + i] == "-" {
                        vec![]
                    } else {
                        v[17 + i].split(',').map(num).collect()
                    },
                },
            );
        }
        let mut callbacks = parse_graph(v[19]);
        let mut image: [u8; 96] = hex(v[7]).try_into().unwrap();
        let next = parse_links(v[8]);
        let refresh = Replay::from_fields(&v, v[28], &next);
        let global = refresh.v[1];
        let mut combined = Combined {
            events: v[27].split('|').map(str::to_string).collect(),
            position: 0,
            poison: num(v[1]) as u8 ^ 0x5a,
            refresh,
            global,
            next,
        };
        let temporary = num(v[31]);
        let identity = num(v[32]);
        let init = ui_dialog_child_context::construct(
            &mut image,
            num(v[14]),
            Context {
                identity,
                temporary,
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut combined,
        )
        .unwrap();
        assert_eq!(image.to_vec(), hex(v[21]), "case {count}");
        assert_eq!(init.base_sentinel.address, num(v[22]));
        assert_eq!(init.base_sentinel.bytes, hex(v[23]));
        assert_eq!(
            init.callback_head,
            u64::from_le_bytes(image[0x28..0x30].try_into().unwrap())
        );
        assert_eq!(links(&parents[&ids[0]]), v[24]);
        assert_eq!(links(&parents[&ids[1]]), v[25]);
        assert_eq!(graph(&callbacks), v[26]);
        assert_eq!(combined.position, combined.events.len());
        assert_eq!(combined.refresh.position, combined.refresh.event_count());
        assert_eq!(combined.global, num(v[29]));
        assert_eq!(combined.next, parse_links(v[30]));
        assert!(!callbacks.nodes.contains_key(&temporary));
        count += 1;
    }
    assert_eq!(count, 972);
}
