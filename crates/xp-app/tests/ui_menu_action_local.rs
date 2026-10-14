use openxplane::ui_menu_action::{self, Services, State};
struct Replay {
    pending: [u8; 2],
    index: usize,
    cache: u8,
    update: i32,
    result: u64,
    trace: Vec<String>,
}
impl Services for Replay {
    fn query_a(&mut self, s: &mut State) -> u8 {
        let index = self.index;
        self.index += 1;
        self.trace.push(format!("P:{index}"));
        if index == 0 && self.update >= 0 {
            s.engine_byte = self.update as u8;
        }
        self.pending[index]
    }
    fn query_b(&mut self, _: &mut State) -> u8 {
        self.trace.push("Q".into());
        self.cache
    }
    fn direct(&mut self, _: &mut State, argument: i32) -> Result<(), &'static str> {
        self.trace.push(format!("V:{argument}"));
        Ok(())
    }
    fn allocate(&mut self, _: &mut State, bytes: u64) -> u64 {
        self.trace.push(format!("A:{bytes}"));
        1
    }
    fn callback(&mut self, _: &mut State, index: u8) -> u64 {
        self.trace.push(format!("H:{index}"));
        200 + u64::from(index)
    }
    fn construct(&mut self, _: &mut State, allocation: u64, second: u64, first: u64) -> u64 {
        assert_eq!((allocation, second, first), (1, 201, 200));
        self.trace.push("C:1:0".into());
        1
    }
    fn drop_callback(&mut self, _: &mut State, callback: u64) {
        self.trace.push(format!("D:{}", callback - 200));
    }
    fn manager(&mut self, _: &mut State) -> u64 {
        self.trace.push("M".into());
        18
    }
    fn registry_input(&mut self, _: &mut State, object: u64) -> u64 {
        assert_eq!(object, 1);
        self.trace.push("I:17".into());
        19
    }
    fn resolve(&mut self, _: &mut State, manager: u64, input: u64) -> u64 {
        self.trace.push(format!("R:{manager}:{input}"));
        self.result
    }
    fn destroy(&mut self, _: &mut State, object: u64, flag: u32) {
        assert_eq!(object, 1);
        self.trace.push(format!("X:{flag}"));
    }
    fn submit(&mut self, _: &mut State, word: i32, result: u64) {
        self.trace.push(format!("S:{word}:{result}"));
    }
}
#[test]
#[ignore = "requires native concrete menu fallback replay; set OPENXPLANE_UI_MENU_ACTION_VECTORS"]
fn native_menu_fallback_request_flow_matches_with_actual_property_gate() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_UI_MENU_ACTION_VECTORS")
            .expect("set OPENXPLANE_UI_MENU_ACTION_VECTORS"),
    )
    .unwrap();
    assert!(text.lines().any(|l| l
        == "# reference_sha256 2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"));
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("F ")) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let v: Vec<i128> = a
            .split_whitespace()
            .skip(1)
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(v.len(), 7);
        for (path, expected) in b.split(" | ").enumerate() {
            let mut state = State {
                engine_byte: v[0] as u8,
                ui_mode: v[1] as i32,
            };
            let mut replay = Replay {
                pending: [v[2] as u8, v[3] as u8],
                cache: v[4] as u8,
                update: v[5] as i32,
                result: v[6] as u64,
                index: 0,
                trace: Vec::new(),
            };
            struct Bridge<'a>(&'a mut State, &'a mut Replay);
            impl openxplane::ui_menu_key::Services for Bridge<'_> {
                fn fallback_action(&mut self, _: &mut openxplane::ui_menu_key::State) {
                    ui_menu_action::dispatch(self.0, self.1).unwrap();
                }
            }
            let input = openxplane::ui_input::KeyInput {
                character: 27,
                virtual_key: 27,
                flags: 8,
            };
            let result = if path == 0 {
                ui_menu_action::dispatch(&mut state, &mut replay).unwrap();
                0
            } else {
                let mut key_state = openxplane::ui_menu_key::State::default();
                let mut bridge = Bridge(&mut state, &mut replay);
                if path == 1 {
                    openxplane::ui_menu_key::dispatch(&mut key_state, input, &mut bridge)
                } else {
                    openxplane::ui_input::dispatch_focused(
                        true,
                        || 0xfff,
                        input,
                        None,
                        |event| {
                            openxplane::ui_menu_key::dispatch(&mut key_state, event, &mut bridge)
                        },
                    )
                    .unwrap()
                }
            };
            let mut out = expected.split_whitespace();
            assert_eq!(
                result,
                out.next().unwrap().parse::<u8>().unwrap(),
                "return path {path} case {count}"
            );
            assert_eq!(
                state.engine_byte,
                out.next().unwrap().parse::<u8>().unwrap(),
                "state case {count}"
            );
            assert_eq!(
                replay.trace,
                out.map(str::to_string).collect::<Vec<_>>(),
                "trace case {count}"
            );
        }
        assert_eq!(b.split(" | ").count(), 3);
        count += 1;
    }
    assert_eq!(count, 746);
}
#[test]
fn repeat_query_is_not_cached_and_engine_change_does_not_repeat_initial_gate() {
    let mut state = State {
        engine_byte: 0,
        ui_mode: 0,
    };
    let mut replay = Replay {
        pending: [0, 1],
        index: 0,
        cache: 0,
        update: 255,
        result: 42,
        trace: Vec::new(),
    };
    ui_menu_action::dispatch(&mut state, &mut replay).unwrap();
    assert_eq!(state.engine_byte, 255);
    assert_eq!(replay.trace, ["P:0", "P:1", "S:0:0"]);
    state.engine_byte = 0;
    state.ui_mode = 2;
    replay.trace.clear();
    ui_menu_action::dispatch(&mut state, &mut replay).unwrap();
    assert!(replay.trace.is_empty());
}
