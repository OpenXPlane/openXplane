use openxplane::{
    ui_payload_move,
    ui_record_class_membership::{self, Services},
};
use std::collections::BTreeMap;
fn memory(encoded: &str) -> BTreeMap<u64, Vec<u8>> {
    encoded
        .split('/')
        .map(|r| {
            let (p, b) = r.split_once(':').unwrap();
            (
                p.parse().unwrap(),
                (0..b.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&b[i..i + 2], 16).unwrap())
                    .collect(),
            )
        })
        .collect()
}
struct Replay {
    events: Vec<String>,
    position: usize,
}
impl Services for Replay {
    fn invoke(
        &mut self,
        m: &mut ui_payload_move::State,
        f: u64,
        args: &[u64],
    ) -> Result<u64, &'static str> {
        let fields = self.events[self.position].split('~').collect::<Vec<_>>();
        self.position += 1;
        assert_eq!(fields.len(), 5);
        assert_eq!(fields[0], "I");
        let mut call = fields[1].split(',');
        assert_eq!(call.next().unwrap().parse::<u64>().unwrap(), f);
        assert_eq!(
            call.map(|v| v.parse::<u64>().unwrap()).collect::<Vec<_>>(),
            args
        );
        assert_eq!(m.blocks, memory(fields[2]), "provider pre-state");
        m.blocks = memory(fields[3]);
        Ok(fields[4].parse().unwrap())
    }
}
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_RECORD_CLASS_MEMBERSHIP_VECTORS"]
fn whole_membership_signed_tree_nil_markers_favorites_tail_and_full_rax() {
    use std::io::BufRead;
    let file = std::fs::File::open(
        std::env::var("OPENXPLANE_UI_RECORD_CLASS_MEMBERSHIP_VECTORS").unwrap(),
    )
    .unwrap();
    let (mut cases, mut favorites) = (0, 0);
    for (index, row) in std::io::BufReader::new(file).lines().enumerate() {
        let row = row.unwrap();
        if !row.starts_with("M ") {
            continue;
        }
        let v = row.split_whitespace().collect::<Vec<_>>();
        assert_eq!(v.len(), 8);
        assert_eq!(v[4], "->");
        let mut m = ui_payload_move::State {
            blocks: memory(v[3]),
        };
        let mut h = Replay {
            events: if v[7] == "-" {
                vec![]
            } else {
                v[7].split('|').map(str::to_owned).collect()
            },
            position: 0,
        };
        let result = ui_record_class_membership::contains(
            &mut m,
            &mut h,
            v[1].parse().unwrap(),
            v[2].parse::<i128>().unwrap() as u64,
        )
        .unwrap();
        assert_eq!(
            result,
            v[6].parse::<u64>().unwrap(),
            "original line {}",
            index + 1
        );
        assert_eq!(m.blocks, memory(v[5]), "final bytes line {}", index + 1);
        assert_eq!(h.position, h.events.len());
        if h.position != 0 {
            assert_eq!(h.position, 2);
            favorites += 1;
        }
        cases += 1;
    }
    assert_eq!((cases, favorites), (3312, 288));
}
#[test]
fn favorites_delegates_without_tree_reads_and_preserves_full_result_and_provider_effects() {
    struct Host {
        calls: usize,
    }
    impl Services for Host {
        fn invoke(
            &mut self,
            m: &mut ui_payload_move::State,
            f: u64,
            a: &[u64],
        ) -> Result<u64, &'static str> {
            self.calls += 1;
            match f {
                0x1414cc7f0 => {
                    assert!(a.is_empty());
                    m.blocks.insert(0x20000, vec![0x7e]);
                    Ok(0x123456789abcdef0)
                }
                0x1414cc820 => {
                    assert_eq!(a, [0x123456789abcdef0, 0x10000]);
                    assert_eq!(m.read(0x20000, 1)?, [0x7e]);
                    m.write(0x20000, &[0xa5])?;
                    Ok(0xfedcba9876543200)
                }
                _ => Err("unexpected membership dependency"),
            }
        }
    }
    let mut m = ui_payload_move::State::default();
    let mut h = Host { calls: 0 };
    assert_eq!(
        ui_record_class_membership::contains(&mut m, &mut h, 0x10000, 0x10000000b).unwrap(),
        0xfedcba9876543200
    );
    assert_eq!(h.calls, 2);
    assert_eq!(m.read(0x20000, 1).unwrap(), [0xa5]);
}
