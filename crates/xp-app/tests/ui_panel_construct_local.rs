use openxplane::{
    ui_input_construct::{Block, Services},
    ui_panel_construct,
};
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
        let b = self.blocks[self.index].clone();
        assert_eq!(b.bytes.len(), bytes);
        self.index += 1;
        Ok(b)
    }
}
#[test]
#[ignore = "requires panel ctor corpus via OPENXPLANE_UI_PANEL_CONSTRUCT_VECTORS"]
fn native_common_panel_strings_flags_and_base_snapshots() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_PANEL_CONSTRUCT_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let mut image = vec![v[2].parse::<u8>().unwrap(); 0x170];
        let poison = v[3].parse::<u8>().unwrap();
        let flags: [u8; 3] = v[4]
            .split(',')
            .map(|v| v.parse().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let mut strings: [[u8; 32]; 3] = [
            hex(v[5]).try_into().unwrap(),
            hex(v[6]).try_into().unwrap(),
            hex(v[7]).try_into().unwrap(),
        ];
        assert_eq!(v[8], "->");
        let mut replay = Replay {
            blocks: vec![],
            snapshots: vec![],
            index: 0,
        };
        let mut expected = vec![];
        for record in &v[13..] {
            let f: Vec<_> = record.split(':').collect();
            let size = f[0].parse::<usize>().unwrap();
            replay.blocks.push(Block {
                address: f[1].parse().unwrap(),
                bytes: vec![poison; size],
            });
            replay.snapshots.push(hex(f[3]));
            expected.push(hex(f[2]));
        }
        let blocks =
            ui_panel_construct::construct(&mut image, &mut strings, flags, &mut replay).unwrap();
        assert_eq!(image, hex(v[9]), "{line}");
        for i in 0..3 {
            assert_eq!(strings[i].to_vec(), hex(v[10 + i]), "{line}");
        }
        assert_eq!(
            blocks.iter().map(|b| b.bytes.clone()).collect::<Vec<_>>(),
            expected,
            "{line}"
        );
        assert_eq!(replay.index, 4);
        let key = ui_panel_construct::key_state(&image).unwrap();
        assert_eq!((key.flag168, key.flag169), (flags[0], flags[1]));
        count += 1;
    }
    assert_eq!(count, 256);
}
