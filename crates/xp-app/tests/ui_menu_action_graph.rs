include!("ui_menu_direct_graph.rs");
use openxplane::{ui_menu_action as action, ui_menu_action_graph};
impl action::Services for Replay {
    fn query_a(&mut self, _state: &mut action::State) -> u8 {
        self.trace.push("fallback-a".into());
        1
    }
    fn query_b(&mut self, _state: &mut action::State) -> u8 {
        self.trace.push("fallback-b".into());
        1
    }
    fn direct(&mut self, _state: &mut action::State, _argument: i32) -> Result<(), &'static str> {
        panic!("unexpected fallback effect in bridge test")
    }
    fn allocate(&mut self, _state: &mut action::State, _bytes: u64) -> u64 {
        panic!("unexpected fallback effect in bridge test")
    }
    fn callback(&mut self, _state: &mut action::State, _index: u8) -> u64 {
        panic!("unexpected fallback effect in bridge test")
    }
    fn construct(
        &mut self,
        _state: &mut action::State,
        _allocation: u64,
        _second: u64,
        _first: u64,
    ) -> u64 {
        panic!("unexpected fallback effect in bridge test")
    }
    fn drop_callback(&mut self, _state: &mut action::State, _callback: u64) {
        panic!("unexpected fallback effect in bridge test")
    }
    fn manager(&mut self, _state: &mut action::State) -> u64 {
        panic!("unexpected fallback effect in bridge test")
    }
    fn registry_input(&mut self, _state: &mut action::State, _object: u64) -> u64 {
        panic!("unexpected fallback effect in bridge test")
    }
    fn resolve(&mut self, _state: &mut action::State, _manager: u64, _input: u64) -> u64 {
        panic!("unexpected fallback effect in bridge test")
    }
    fn destroy(&mut self, _state: &mut action::State, _object: u64, _flag: u32) {
        panic!("unexpected fallback effect in bridge test")
    }
    fn submit(&mut self, _state: &mut action::State, _word: i32, _result: u64) {
        panic!("unexpected fallback effect in bridge test")
    }
}
#[test]
fn fallback_direct_branches_use_actual_graph_close() {
    for engine_byte in [0, 1] {
        let mut action = action::State {
            engine_byte,
            ui_mode: 0,
        };
        let mut panel = State::default();
        let mut c = controller();
        let mut effects = Replay {
            v: vec![0, 0, 0, 0, 0, 0, 0, -1, -1, 7, 0, 0, 0],
            trace: vec![],
        };
        ui_menu_action_graph::dispatch(&mut action, &mut panel, &mut c, 7, &mut effects).unwrap();
        assert_eq!(panel.guard_c9, 0);
        assert_eq!(c.by_kind[&11], 0);
        assert!(!c.graph.visible(11).unwrap());
        assert!(effects.trace.iter().any(|s| s == "graph-bind"));
        assert_eq!(
            effects
                .trace
                .iter()
                .filter(|s| s.as_str() == "fallback-a")
                .count(),
            usize::from(engine_byte == 0)
        );
    }
}
#[test]
fn concrete_property_gate_keeps_graph_unchanged() {
    let mut action = action::State {
        engine_byte: 0,
        ui_mode: 2,
    };
    let mut panel = State::default();
    let mut c = controller();
    let mut effects = Replay {
        v: vec![],
        trace: vec![],
    };
    ui_menu_action_graph::dispatch(&mut action, &mut panel, &mut c, 7, &mut effects).unwrap();
    assert!(effects.trace.is_empty());
    assert_eq!(c.by_kind[&11], 11);
    assert!(c.graph.visible(11).unwrap());
}
