use openxplane::{
    ui_input_construct::{Block, Services as Allocator},
    ui_native_string::{NativeString, Services},
    ui_panel_construct,
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
struct Replay {
    events: Vec<String>,
    index: usize,
    poison: u8,
    expected: BTreeMap<u64, Vec<u8>>,
    freed: BTreeMap<u64, Vec<u8>>,
}
impl Allocator for Replay {
    fn allocate(&mut self, image: &[u8], bytes: usize) -> Result<Block, &'static str> {
        let f: Vec<_> = self.events[self.index].split(':').collect();
        assert_eq!(f[0], "A");
        assert_eq!(num(f[1]) as usize, bytes);
        assert_eq!(image, hex(f[3]), "allocation event {}", self.index);
        let address = num(f[2]);
        self.index += 1;
        Ok(Block {
            address,
            bytes: vec![self.poison; bytes],
        })
    }
}
impl Services for Replay {
    fn release(&mut self, image: &[u8], block: Block) -> Result<(), &'static str> {
        let f: Vec<_> = self.events[self.index].split(':').collect();
        assert_eq!(f[0], "F");
        assert_eq!(num(f[1]), block.address);
        assert_eq!(num(f[2]) as usize, block.bytes.len());
        assert_eq!(image, hex(f[3]), "free event {}", self.index);
        assert_eq!(Some(&block.bytes), self.expected.get(&block.address));
        assert!(self.freed.insert(block.address, block.bytes).is_none());
        self.index += 1;
        Ok(())
    }
}
#[test]
#[ignore = "requires local native corpus via OPENXPLANE_UI_PANEL_HEAP_VECTORS"]
fn native_mixed_inline_heap_and_aligned_panel_string_ownership() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_PANEL_HEAP_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let seed = num(v[1]) as u8;
        let flags: [u8; 3] = v[2]
            .split(',')
            .map(|s| s.parse().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let mut backing = BTreeMap::new();
        if v[6] != "-" {
            for r in v[6].split(';') {
                let (p, b) = r.split_once(':').unwrap();
                backing.insert(num(p), hex(b));
            }
        }
        let mut strings: [NativeString; 3] = std::array::from_fn(|i| {
            let raw: [u8; 32] = hex(v[3 + i]).try_into().unwrap();
            let capacity = u64::from_le_bytes(raw[24..].try_into().unwrap());
            let pointer = u64::from_le_bytes(raw[..8].try_into().unwrap());
            let storage = if capacity <= 15 {
                None
            } else {
                let (address, bytes) = backing.range(..=pointer).next_back().unwrap();
                assert!(pointer < address + bytes.len() as u64);
                Some(Block {
                    address: *address,
                    bytes: bytes.clone(),
                })
            };
            NativeString { raw, storage }
        });
        assert_eq!(v[7], "->");
        let mut expected = backing;
        let mut allocated = vec![];
        for r in v[12].split(';') {
            let f: Vec<_> = r.split(':').collect();
            let p = num(f[0]);
            let bytes = hex(f[2]);
            assert_eq!(bytes.len(), num(f[1]) as usize);
            expected.insert(p, bytes);
            allocated.push(p);
        }
        let mut replay = Replay {
            events: v[13].split('|').map(str::to_string).collect(),
            index: 0,
            poison: seed ^ 0x5a,
            expected,
            freed: BTreeMap::new(),
        };
        let mut image = vec![seed; 0x170];
        let blocks =
            ui_panel_construct::construct_owned(&mut image, &mut strings, flags, &mut replay)
                .unwrap();
        assert_eq!(image, hex(v[8]), "case {count}");
        for i in 0..3 {
            assert_eq!(strings[i].raw.to_vec(), hex(v[9 + i]));
            assert!(strings[i].storage.is_none());
        }
        assert_eq!(replay.index, replay.events.len());
        let mut actual = replay.freed;
        for b in blocks {
            assert!(actual.insert(b.address, b.bytes).is_none());
        }
        for p in allocated {
            assert_eq!(
                actual.get(&p),
                replay.expected.get(&p),
                "case {count} allocation {p}"
            );
        }
        assert_eq!(actual.len(), replay.expected.len());
        count += 1;
    }
    assert_eq!(count, 324);
}
