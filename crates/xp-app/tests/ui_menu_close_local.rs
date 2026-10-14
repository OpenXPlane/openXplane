use openxplane::{
    ui_menu_close::{self, Services, State},
    ui_state::RegisteredWindow,
};
const PATHS: &[&[f32]] = &[
    &[1.0],
    &[0.0],
    &[-1.0],
    &[-1.0, -1.0],
    &[1.0, 0.0],
    &[0.5, 0.5],
    &[-0.0],
    &[1e-30, 0.5],
];
struct Replay {
    v: Vec<i64>,
    updates: usize,
    trace: Vec<String>,
}
impl Services for Replay {
    fn update(&mut self, s: &mut State, kind: i32, flag: u8) {
        self.trace
            .push(format!("U:{kind}:{flag}:{}", s.windows.by_kind[&kind]));
        if self.updates == 0 && self.v[13] != 0 {
            s.windows.by_kind.insert(kind, 2);
        }
        self.updates += 1;
    }
    fn reset(&mut self, s: &mut State, token: u64, mode: i32, extra: i32) {
        self.trace.push(format!("R:{token}:{mode}:{extra}"));
        if self.v[9] >= 0 {
            s.modes[token as usize - 1] = self.v[9] as i32;
        }
    }
    fn visibility(&mut self, s: &mut State, token: u64, show: bool, mode: i32, time: f32) {
        assert_eq!(time.to_bits(), 0);
        self.trace
            .push(format!("V:{token}:{}:{mode}:0", u8::from(show)));
        if self.v[10] >= 0 {
            *s.windows.registered[token as usize - 1]
                .as_mut()
                .unwrap()
                .opacity_root_to_leaf
                .last_mut()
                .unwrap() = self.v[10] as f32;
        }
    }
    fn delete(&mut self, _: &mut State, token: u64, flag: u8) {
        self.trace.push(format!("D:{token}:{flag}"));
    }
    fn environment(&mut self, s: &mut State) -> u8 {
        self.trace.push("Q".into());
        if self.v[11] >= 0 {
            s.flag_2a8 = self.v[11] as u8;
        }
        if self.v[12] >= 0 {
            s.flag_11f48 = self.v[12] as i32;
        }
        self.v[8] as u8
    }
    fn command_once(&mut self, _: &mut State) {
        self.trace.push("C".into());
    }
    fn refresh(&mut self, _: &mut State) {
        self.trace.push("P".into());
    }
    fn final_update(&mut self, s: &mut State, kind: i32) {
        self.trace
            .push(format!("F:{kind}:{}", s.windows.by_kind[&kind]));
    }
}
fn state(v: &[i64]) -> State {
    let kind = v[0] as i32;
    let mut s = State {
        modes: vec![v[3] as i32, 6],
        delete_kinds: if v[4] != 0 { vec![kind] } else { vec![] },
        flag_11f48: v[6] as i32,
        flag_2a8: v[7] as u8,
        ..Default::default()
    };
    s.windows.by_kind.insert(kind, v[1] as u64);
    s.windows.registered = vec![
        Some(RegisteredWindow {
            token: 1,
            classification: 0,
            opacity_root_to_leaf: PATHS[v[2] as usize].to_vec(),
        }),
        Some(RegisteredWindow {
            token: 2,
            classification: 0,
            opacity_root_to_leaf: vec![v[14] as f32],
        }),
    ];
    s
}
#[test]
#[ignore = "requires native registered-window close replay; set OPENXPLANE_UI_MENU_CLOSE_VECTORS"]
fn native_close_map_store_modes_toggle_and_post_query_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_UI_MENU_CLOSE_VECTORS")
            .expect("set OPENXPLANE_UI_MENU_CLOSE_VECTORS"),
    )
    .unwrap();
    assert!(text.lines().any(|l| l
        == "# reference_sha256 2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"));
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("C ")) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let v: Vec<i64> = a
            .split_whitespace()
            .skip(1)
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(v.len(), 15);
        let mut state = state(&v);
        let kind = v[0] as i32;
        let flag = v[5] as u8;
        let mut replay = Replay {
            v,
            updates: 0,
            trace: Vec::new(),
        };
        ui_menu_close::dispatch(&mut state, kind, flag, &mut replay).unwrap();
        let mut output = b.split_whitespace();
        let expected: Vec<u64> = (0..8)
            .map(|_| output.next().unwrap().parse().unwrap())
            .collect();
        let leaf = |index: usize| {
            state.windows.registered[index]
                .as_ref()
                .unwrap()
                .opacity_root_to_leaf
                .last()
                .unwrap()
                .to_bits() as u64
        };
        let actual = [
            state.windows.by_kind[&kind],
            state.modes[0] as u32 as u64,
            state.modes[1] as u32 as u64,
            leaf(0),
            leaf(1),
            state.flag_11f48 as u32 as u64,
            state.flag_2a8 as u64,
            u64::from(state.windows.query(kind).unwrap()),
        ];
        assert_eq!(actual.as_slice(), expected, "state/query case {count}");
        assert_eq!(
            replay.trace,
            output.map(str::to_string).collect::<Vec<_>>(),
            "trace case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 574);
}
#[test]
fn clearing_mapping_changes_query_and_modes_are_reread_after_reset() {
    let v = vec![11, 1, 0, 4, 1, 255, 1, 255, 0, 6, -1, 0, -1, 0, 0];
    let mut s = state(&v);
    let mut replay = Replay {
        v,
        updates: 0,
        trace: Vec::new(),
    };
    ui_menu_close::dispatch(&mut s, 11, 255, &mut replay).unwrap();
    assert_eq!(
        replay.trace,
        [
            "U:11:0:1",
            "R:1:0:-1",
            "R:1:0:-1",
            "V:1:0:0:0",
            "D:1:1",
            "U:11:255:0",
            "Q",
            "F:11:0"
        ]
    );
    assert_eq!(s.windows.by_kind[&11], 0);
    assert!(!s.windows.query(11).unwrap());
    assert_eq!(
        s.windows.registered[0]
            .as_ref()
            .unwrap()
            .opacity_root_to_leaf,
        [1.0]
    );
    replay.trace.clear();
    ui_menu_close::dispatch(&mut s, 11, 255, &mut replay).unwrap();
    assert_eq!(replay.trace, ["F:11:0"]);
}
