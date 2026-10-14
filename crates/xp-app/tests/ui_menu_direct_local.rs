use openxplane::ui_menu_direct::{self, Services, State};
struct Replay {
    v: Vec<i128>,
    trace: Vec<String>,
}
impl Services for Replay {
    fn prepare(&mut self, s: &mut State) {
        self.trace.push(format!("P:{}", s.guard_c9));
        if self.v[7] >= 0 {
            s.flag_c8 = self.v[7] as u8;
        }
        if self.v[11] != 0 {
            ui_menu_direct::dispatch(s, -5, self).unwrap();
        }
    }
    fn controller(&mut self, _: &mut State) -> u64 {
        self.trace.push("M".into());
        7
    }
    fn close(
        &mut self,
        s: &mut State,
        controller: u64,
        kind: i32,
        flag: u8,
    ) -> Result<(), &'static str> {
        assert_eq!(controller, 7);
        self.trace.push(format!("C:{kind}:{flag}:{}", s.guard_c9));
        if self.v[8] >= 0 {
            s.ui_mode = self.v[8] as i32;
        }
        Ok(())
    }
    fn notify(&mut self, _: &mut State) {
        self.trace.push("N".into());
    }
    fn query_a(&mut self, _: &mut State) -> u8 {
        self.trace.push("Q".into());
        self.v[4] as u8
    }
    fn document(&mut self, _: &mut State) -> Vec<u8> {
        self.trace.push("F".into());
        [b"".as_slice(), b"a", b"test.sit", b"xxxxxxxxxxxxxxx"][self.v[12] as usize].to_vec()
    }
    fn validate(&mut self, _: &mut State, document: &[u8]) -> u8 {
        let hex = if document.is_empty() {
            "-".into()
        } else {
            document
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        self.trace.push(format!("V:{hex}"));
        self.v[5] as u8
    }
    fn load(&mut self, _: &mut State, _: &[u8], validation: u8) -> u8 {
        self.trace.push(format!("L:{validation}"));
        self.v[6] as u8
    }
    fn load_failed(&mut self, _: &mut State) {
        self.trace.push("FAIL".into());
    }
    fn drop_document(&mut self, _: &mut State, _: Vec<u8>) {
        self.trace.push("DROP".into());
    }
    fn allocate(&mut self, _: &mut State, bytes: u64) -> u64 {
        self.trace.push(format!("A:{bytes}"));
        100
    }
    fn callback(&mut self, _: &mut State, id: i32) -> u64 {
        self.trace.push(format!("H:{id}"));
        self.trace.push(format!("B:{id}"));
        200 + id as u64
    }
    fn construct(
        &mut self,
        _: &mut State,
        allocation: u64,
        source: u64,
        second: u64,
        first: u64,
    ) -> u64 {
        assert_eq!((allocation, second, first), (100, 203, 200));
        self.trace.push(format!("T:{source}:3:0"));
        100
    }
    fn drop_callback(&mut self, _: &mut State, callback: u64) {
        self.trace.push(format!("D:{}", callback - 200));
    }
    fn field_c8_action(&mut self, s: &mut State, object: u64) {
        assert_eq!(object, 100);
        assert_eq!(s.flag_7770c6, 1);
        self.trace.push("E:c8:1".into());
    }
    fn component_260_action(&mut self, _: &mut State, object: u64) {
        assert_eq!(object, 100);
        self.trace.push("E:260".into());
    }
    fn field_d8_action(&mut self, _: &mut State, object: u64) {
        assert_eq!(object, 100);
        self.trace.push("E:d8".into());
    }
    fn destroy(&mut self, _: &mut State, object: u64, flag: u32) {
        assert_eq!(object, 100);
        self.trace.push(format!("X:{flag}"));
    }
}
#[test]
#[ignore = "requires native direct-menu replay; set OPENXPLANE_UI_MENU_DIRECT_VECTORS"]
fn native_full_direct_action_guard_close_document_and_dialog_requests_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_UI_MENU_DIRECT_VECTORS")
            .expect("set OPENXPLANE_UI_MENU_DIRECT_VECTORS"),
    )
    .unwrap();
    assert!(text.lines().any(|l| l
        == "# reference_sha256 2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"));
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("D ")) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let v: Vec<i128> = a
            .split_whitespace()
            .skip(1)
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(v.len(), 13);
        let argument = v[0] as i32;
        let mut state = State {
            guard_c9: v[1] as u8,
            flag_c8: v[2] as u8,
            ui_mode: v[3] as i32,
            source_token: v[9] as u64,
            flag_7770c6: v[10] as u8,
        };
        let mut replay = Replay {
            v,
            trace: Vec::new(),
        };
        ui_menu_direct::dispatch(&mut state, argument, &mut replay).unwrap();
        let mut output = b.split_whitespace();
        let expected: Vec<u64> = (0..4)
            .map(|_| output.next().unwrap().parse().unwrap())
            .collect();
        assert_eq!(
            [
                state.guard_c9 as u64,
                state.flag_c8 as u64,
                state.ui_mode as u32 as u64,
                state.flag_7770c6 as u64
            ]
            .as_slice(),
            expected,
            "state case {count}"
        );
        assert_eq!(
            replay.trace,
            output.map(str::to_string).collect::<Vec<_>>(),
            "trace case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 694);
}
#[test]
fn recursive_prepare_is_guarded_and_close_mode_is_read_after_close() {
    let mut state = State {
        ui_mode: 1,
        flag_7770c6: 255,
        ..Default::default()
    };
    let mut replay = Replay {
        v: vec![0, 0, 0, 1, 0, 0, 0, 255, 2, 7, 255, 1, 1],
        trace: Vec::new(),
    };
    ui_menu_direct::dispatch(&mut state, 0, &mut replay).unwrap();
    assert_eq!(replay.trace, ["P:1", "M", "C:11:0:1"]);
    assert_eq!(
        (
            state.guard_c9,
            state.flag_c8,
            state.ui_mode,
            state.flag_7770c6
        ),
        (0, 255, 2, 255)
    );
    state.guard_c9 = 255;
    let before = state.clone();
    replay.trace.clear();
    ui_menu_direct::dispatch(&mut state, 0, &mut replay).unwrap();
    assert_eq!(state, before);
    assert!(replay.trace.is_empty());
}
