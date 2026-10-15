use openxplane::{
    ui_input_construct::{Block, Services as Allocator},
    ui_interface_chain::{BASE_TABLE, Node, State},
    ui_main_menu_construct::{self, Context, Services},
    ui_menu_member_construct::Registry,
};
use std::collections::BTreeMap;
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn num(s: &str) -> u64 {
    s.parse().unwrap()
}
fn q(v: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(v[off..off + 8].try_into().unwrap())
}
struct Allocation {
    size: usize,
    address: u64,
    snapshot: Vec<u8>,
    head: u64,
    expected: Vec<u8>,
}
struct Replay {
    records: Vec<Allocation>,
    index: usize,
    poison: u8,
    events: Vec<String>,
    freed: Vec<Block>,
    old_head: u64,
    identity: u64,
}
impl Allocator for Replay {
    fn allocate(&mut self, image: &[u8], size: usize) -> Result<Block, &'static str> {
        let r = &self.records[self.index];
        assert_eq!(size, r.size);
        assert_eq!(image, r.snapshot, "allocation {}", self.index);
        assert_eq!(
            r.head,
            if q(image, 0x188) == 0x1426067c8 {
                self.identity + 0x188
            } else {
                self.old_head
            }
        );
        self.events.push(format!("A:{size}:{}", r.address));
        self.index += 1;
        Ok(Block {
            address: r.address,
            bytes: if r.address == 0 {
                vec![]
            } else {
                vec![self.poison; size]
            },
        })
    }
}
impl Services for Replay {
    fn copied(&mut self, bytes: usize) {
        self.events.push(format!("M:{bytes}"));
    }
    fn free(&mut self, block: Block) {
        self.events
            .push(format!("F:{}:{}", block.address, block.bytes.len()));
        self.freed.push(block);
    }
}
#[test]
#[ignore = "requires local native prefix corpus via OPENXPLANE_UI_MAIN_MENU_CONSTRUCT_VECTORS"]
fn native_concrete_prefix_snapshots_chain_registration_and_null_callback() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_MAIN_MENU_CONSTRUCT_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let seed = num(v[1]) as u8;
        let flag = num(v[2]) as u8;
        let mapped = num(v[3]) != 0;
        let cap = num(v[4]) as usize;
        let len = num(v[5]) as usize;
        let heads = num(v[6]) as usize;
        let identity = num(v[8]);
        let parent = if mapped { num(v[9]) } else { 0 };
        let old = num(v[10]);
        let mut addresses: Vec<u64> = v[11].split(',').take(heads).map(num).collect();
        addresses.push(identity + 0x188);
        let localized = hex(v[12]).try_into().unwrap();
        assert_eq!(v[13], "->");
        let mut nodes: Vec<Node> = (0..heads)
            .map(|i| Node {
                table: BASE_TABLE,
                newer: i.checked_sub(1),
                older: if i + 1 < heads { Some(i + 1) } else { None },
                free_requested: false,
            })
            .collect();
        nodes.push(Node::default());
        let mut chain = State {
            head: if heads > 0 { Some(0) } else { None },
            nodes,
        };
        let mut storage = vec![seed ^ 0x5a; cap * 8];
        for i in 0..len {
            storage[i * 8..i * 8 + 8].copy_from_slice(&(100 + i as u64).to_le_bytes());
        }
        let mut registry = Registry {
            entries: (0..len).map(|i| 100 + i as u64).collect(),
            capacity: cap,
            storage: if old == 0 {
                None
            } else {
                Some(Block {
                    address: old,
                    bytes: storage,
                })
            },
        };
        let mut replay = Replay {
            records: v[21]
                .split('|')
                .map(|r| {
                    let f: Vec<_> = r.split(':').collect();
                    Allocation {
                        size: num(f[0]) as usize,
                        address: num(f[1]),
                        snapshot: hex(f[2]),
                        head: num(f[3]),
                        expected: if f[4] == "-" { vec![] } else { hex(f[4]) },
                    }
                })
                .collect(),
            index: 0,
            poison: seed ^ 0x5a,
            events: vec![],
            freed: vec![],
            old_head: if heads > 0 { addresses[0] } else { 0 },
            identity,
        };
        let mut image = vec![seed; 0x300];
        let blocks = ui_main_menu_construct::construct_prefix(
            &mut image,
            localized,
            flag,
            Context {
                identity,
                parent,
                registry: &mut registry,
                chain: &mut chain,
                addresses: &addresses,
                interface: heads,
            },
            &mut replay,
        )
        .unwrap();
        assert_eq!(image, hex(v[14]), "case {count}");
        assert_eq!(addresses[chain.head.unwrap()], num(v[15]));
        if heads > 0 {
            for (i, r) in v[16].split(',').enumerate() {
                let f: Vec<_> = r.split(':').collect();
                assert_eq!(addresses[i], num(f[0]));
                let n = &chain.nodes[i];
                let actual = [
                    n.table,
                    n.newer.map_or(0, |j| addresses[j]),
                    n.older.map_or(0, |j| addresses[j]),
                ]
                .into_iter()
                .flat_map(u64::to_le_bytes)
                .collect::<Vec<_>>();
                assert_eq!(actual, hex(f[1]));
            }
        }
        assert_eq!(registry.capacity, num(v[18]) as usize);
        assert_eq!(registry.entries.len(), num(v[19]) as usize);
        assert_eq!(
            registry.storage.as_ref().map_or(0, |b| b.address),
            num(v[17])
        );
        if let Some(b) = &registry.storage {
            assert_eq!(b.bytes, hex(v[20]));
            for (i, value) in registry.entries.iter().enumerate() {
                assert_eq!(*value, q(&b.bytes, i * 8));
            }
        }
        assert_eq!(replay.index, replay.records.len());
        assert_eq!(replay.events.join(","), v[22]);
        let actual: BTreeMap<_, _> = blocks
            .into_iter()
            .chain(registry.storage.into_iter())
            .chain(replay.freed)
            .map(|b| (b.address, b.bytes))
            .collect();
        for r in replay.records.iter().filter(|r| r.address != 0) {
            assert_eq!(
                actual.get(&r.address),
                Some(&r.expected),
                "case {count}, block {}",
                r.address
            );
        }
        count += 1;
    }
    assert_eq!(count, 480);
}
