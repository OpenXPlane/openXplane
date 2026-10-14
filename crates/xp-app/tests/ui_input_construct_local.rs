use openxplane::ui_input_construct::{self, Block, Services};
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
struct Replay {
    blocks: Vec<Block>,
    snapshots: Vec<Vec<u8>>,
    index: usize,
}
impl Services for Replay {
    fn allocate(&mut self, image: &[u8], bytes: usize) -> Result<Block, &'static str> {
        assert_eq!(image, self.snapshots[self.index]);
        let block = self.blocks[self.index].clone();
        assert_eq!(block.bytes.len(), bytes);
        self.index += 1;
        Ok(block)
    }
}
#[test]
#[ignore = "requires original constructor corpus via OPENXPLANE_UI_INPUT_CONSTRUCT_VECTORS"]
fn native_input_constructor_exact_stores_and_allocation_order() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_INPUT_CONSTRUCT_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let seed = v[2].parse::<u8>().unwrap();
        let poison = v[3].parse::<u8>().unwrap();
        let mut image = vec![seed; 0x128];
        let mut name: [u8; 32] = hex(v[4]).try_into().unwrap();
        assert_eq!(v[5], "->");
        let mut replay = Replay {
            blocks: vec![],
            snapshots: vec![],
            index: 0,
        };
        let mut expected = vec![];
        for record in &v[8..] {
            let f: Vec<_> = record.split(':').collect();
            let size = f[0].parse::<usize>().unwrap();
            let address = f[1].parse::<u64>().unwrap();
            replay.blocks.push(Block {
                address,
                bytes: vec![poison; size],
            });
            replay.snapshots.push(hex(f[3]));
            expected.push(hex(f[2]));
        }
        let blocks = ui_input_construct::construct(&mut image, &mut name, &mut replay).unwrap();
        assert_eq!(image, hex(v[6]), "{line}");
        assert_eq!(name.to_vec(), hex(v[7]), "{line}");
        assert_eq!(
            blocks.iter().map(|b| b.bytes.clone()).collect::<Vec<_>>(),
            expected,
            "{line}"
        );
        assert_eq!(replay.index, 4);
        count += 1;
    }
    assert_eq!(count, 64);
}
