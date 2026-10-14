use openxplane::{
    ui_input_construct::Block,
    ui_menu_member_construct::{self, Registry, Services},
};
fn hex(s: &str) -> Vec<u8> {
    if s == "-" {
        vec![]
    } else {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }
}
struct Replay {
    blocks: Vec<Block>,
    trace: Vec<String>,
    index: usize,
}
impl Services for Replay {
    fn allocate(&mut self, n: usize) -> Result<Block, &'static str> {
        let b = self.blocks[self.index].clone();
        assert_eq!(b.bytes.len(), n);
        self.index += 1;
        self.trace.push(format!("A:{n}"));
        Ok(b)
    }
    fn copied(&mut self, n: usize) {
        self.trace.push(format!("M:{n}"));
    }
    fn free(&mut self, b: Block) {
        self.trace.push(format!("F:{}", b.bytes.len()));
    }
}
#[test]
#[ignore = "requires member constructor corpus via OPENXPLANE_UI_MENU_MEMBER_CONSTRUCT_VECTORS"]
fn native_member_ctor_and_registration_growth() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_MENU_MEMBER_CONSTRUCT_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let mapped = v[2] == "1";
        let cap = v[3].parse::<usize>().unwrap();
        let len = v[4].parse::<usize>().unwrap();
        let seed = v[5].parse::<u8>().unwrap();
        let identity = v[6].parse().unwrap();
        let parent = v[7].parse().unwrap();
        let old = v[8].parse::<u64>().unwrap();
        assert_eq!(v[9], "->");
        let entries = (0..len).map(|i| 100 + i as u64).collect::<Vec<_>>();
        let storage = if old == 0 {
            None
        } else {
            let mut bytes = vec![seed ^ 0x5a; cap * 8];
            for (i, &value) in entries.iter().enumerate() {
                bytes[i * 8..i * 8 + 8].copy_from_slice(&value.to_le_bytes());
            }
            Some(Block {
                address: old,
                bytes,
            })
        };
        let mut registry = Registry {
            entries,
            capacity: cap,
            storage,
        };
        let mut image = vec![seed; 0x58];
        let mut replay = Replay {
            blocks: vec![],
            trace: vec![],
            index: 0,
        };
        if v[15] != "-" {
            for block in v[15].split(',') {
                let f: Vec<_> = block.split(':').collect();
                replay.blocks.push(Block {
                    address: f[0].parse().unwrap(),
                    bytes: vec![seed ^ 0x5a; f[1].parse().unwrap()],
                });
            }
        }
        ui_menu_member_construct::construct(
            &mut image,
            identity,
            if mapped { parent } else { 0 },
            v[1] == "1",
            &mut registry,
            &mut replay,
        )
        .unwrap();
        assert_eq!(image, hex(v[10]), "{line}");
        assert_eq!(registry.capacity, v[11].parse::<usize>().unwrap(), "{line}");
        assert_eq!(
            registry.storage.as_ref().map_or(0, |b| b.address),
            v[12].parse::<u64>().unwrap(),
            "{line}"
        );
        let entries = if v[13] == "-" {
            vec![]
        } else {
            v[13]
                .split(',')
                .map(|v| v.parse::<u64>().unwrap())
                .collect()
        };
        assert_eq!(registry.entries, entries, "{line}");
        assert_eq!(
            registry
                .storage
                .as_ref()
                .map_or(vec![], |b| b.bytes.clone()),
            hex(v[14]),
            "{line}"
        );
        assert_eq!(replay.trace, &v[16..], "{line}");
        count += 1;
    }
    assert_eq!(count, 180);
}
