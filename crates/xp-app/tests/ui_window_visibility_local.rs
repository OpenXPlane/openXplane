use openxplane::{
    ui_state::RegisteredWindow,
    ui_window_order::Registry,
    ui_window_visibility::{self, Services, State},
};
const PATHS: &[&[f32]] = &[
    &[1.0],
    &[0.0],
    &[-1.0],
    &[-1.0, -1.0],
    &[0.0, 1.0],
    &[1.0, 0.0],
    &[0.5, 0.5],
    &[1e-30, 0.5],
];
#[derive(Default)]
struct Replay(Vec<String>);
impl Services for Replay {
    fn owner(&mut self, _: &mut State, i: usize, attached: bool) {
        self.0.push(format!("O:{i}:{}", u8::from(attached)));
    }
    fn prepare_hide(&mut self, _: &mut State, i: usize) {
        self.0.push(format!("H:{i}"));
    }
    fn activate(&mut self, _: &mut State, i: usize, show: bool) {
        self.0.push(format!("B:{i}:{}", u8::from(show)));
    }
    fn reparent(&mut self, s: &mut State, i: usize, show: bool) {
        let bits = s.registry.entries[i]
            .opacity_root_to_leaf
            .last()
            .unwrap()
            .to_bits();
        self.0.push(format!("P:{i}:{}:{bits}", u8::from(show)));
    }
    fn refresh(&mut self, _: &mut State) {
        self.0.push("R".into());
    }
    fn event(&mut self, _: &mut State, token: u64) {
        self.0.push(format!("E:4:1:{token}"));
    }
}
fn csv<T: std::str::FromStr>(s: &str) -> Vec<T>
where
    T::Err: std::fmt::Debug,
{
    if s == "-" {
        vec![]
    } else {
        s.split(',').map(|v| v.parse().unwrap()).collect()
    }
}
#[test]
fn repeated_show_keeps_zero_ancestor_and_can_repeat_membership() {
    let mut s = State {
        registry: Registry {
            entries: vec![RegisteredWindow {
                token: 11,
                classification: 2,
                opacity_root_to_leaf: vec![0.0, 0.0],
            }],
            order: vec![0],
        },
        attached_manager: vec![false],
        ..Default::default()
    };
    let mut replay = Replay::default();
    ui_window_visibility::dispatch(&mut s, 11, 256, 100, -5.0, &mut replay).unwrap();
    assert!(!s.visible(11).unwrap());
    assert_eq!(s.registry.entries[0].opacity_root_to_leaf, [0.0, 1.0]);
    ui_window_visibility::toggle(&mut s, 11, &mut replay).unwrap();
    assert_eq!(s.membership, [0, 0]);
    let n = replay.0.len();
    ui_window_visibility::dispatch(&mut s, 11, 0, 0, 0.0, &mut replay).unwrap();
    assert_eq!(replay.0.len(), n);
}
#[test]
#[ignore = "requires local original show/hide replay via OPENXPLANE_UI_WINDOW_VISIBILITY_VECTORS"]
fn native_show_hide_membership_order_alpha_and_trace() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_WINDOW_VISIBILITY_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let classes: Vec<i32> = csv(v[7]);
        let mut s = State {
            registry: Registry {
                entries: (0..3)
                    .map(|i| RegisteredWindow {
                        token: [11, 12, 11][i],
                        classification: classes[i],
                        opacity_root_to_leaf: PATHS[v[4 + i].parse::<usize>().unwrap()].to_vec(),
                    })
                    .collect(),
                order: csv(v[8]),
            },
            membership: csv(v[9]),
            attached_manager: csv::<u8>(v[10]).into_iter().map(|v| v != 0).collect(),
            root_manager: v[1] == "1",
        };
        let mut replay = Replay::default();
        for word in &v[2..4] {
            ui_window_visibility::dispatch(
                &mut s,
                11,
                word.parse().unwrap(),
                123,
                -5.0,
                &mut replay,
            )
            .unwrap();
        }
        assert_eq!(v[11], "->");
        assert_eq!(s.registry.order, csv::<usize>(v[12]), "{line}");
        assert_eq!(s.membership, csv::<usize>(v[13]), "{line}");
        assert_eq!(
            s.attached_manager,
            csv::<u8>(v[14])
                .into_iter()
                .map(|v| v != 0)
                .collect::<Vec<_>>(),
            "{line}"
        );
        assert_eq!(
            s.registry
                .entries
                .iter()
                .map(|e| e.opacity_root_to_leaf.last().unwrap().to_bits())
                .collect::<Vec<_>>(),
            csv::<u32>(v[15]),
            "{line}"
        );
        assert_eq!(replay.0, &v[16..], "{line}");
        count += 1;
    }
    assert_eq!(count, 728);
}
