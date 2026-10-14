include!("ui_menu_key_graph.rs");
use openxplane::{ui_focus_binding as binding, ui_menu_focus_graph as focus};
impl binding::Services for Replay {
    fn blur(&mut self, s: &mut binding::State, leaf: usize) {
        self.trace.push(format!("live-blur:{leaf}"));
        s.focused = None;
    }
    fn stop(&mut self, _: &mut binding::State, i: usize) -> u8 {
        self.trace.push(format!("live-stop:{i}"));
        0
    }
}
impl focus::Services for Replay {
    fn child(&mut self, _: &mut binding::State, i: usize, _: KeyInput) -> u8 {
        self.trace.push(format!("live-child:{i}"));
        self.v[4] as u8
    }
}
fn live_state() -> focus::State {
    let mut s = focus::State {
        menu: keys::State {
            controller: controller(),
            controller_identity: 7,
            ..Default::default()
        },
        widget_bindings: vec![0, 1],
        entry_bindings: vec![2],
        panel_binding: 3,
        ..Default::default()
    };
    s.menu.action.engine_byte = 1;
    s.targets.insert(1, 0);
    s.targets.insert(2, 1);
    s.binding.nodes = vec![
        binding::Node {
            owner: Some(3),
            selected: Some(2),
        },
        binding::Node {
            owner: Some(0),
            selected: None,
        },
        binding::Node {
            owner: Some(0),
            selected: None,
        },
        binding::Node {
            owner: None,
            selected: Some(0),
        },
    ];
    s.binding.focused = Some(3);
    s
}
#[test]
fn closing_clears_selected_links_and_next_key_observes_lost_focus() {
    let mut s = live_state();
    let mut effects = Replay {
        v: vec![0, 0, 0, 0, 0, 0, 0, -1, -1, 7, 0, 0, 0],
        trace: vec![],
    };
    let input = KeyInput {
        character: 27,
        virtual_key: 27,
        flags: 8,
    };
    assert_eq!(
        focus::dispatch_focused(&mut s, 0xfff, input, &mut effects).unwrap(),
        1
    );
    assert_eq!(s.binding.focused, None);
    assert_eq!(s.binding.nodes[0].selected, None);
    assert_eq!(s.binding.nodes[3].selected, None);
    assert_eq!(s.binding.nodes[1].owner, None);
    assert_eq!(s.binding.nodes[2].owner, None);
    assert_eq!(s.menu.controller.by_kind[&11], 0);
    assert!(effects.trace.iter().any(|s| s == "live-blur:2"));
    let n = effects.trace.len();
    assert_eq!(
        focus::dispatch_focused(&mut s, 0xfff, input, &mut effects).unwrap(),
        0
    );
    assert_eq!(effects.trace.len(), n);
}
#[test]
fn child_consumption_precedes_filter_and_window_action() {
    let mut s = live_state();
    let mut effects = Replay {
        v: vec![0, 0, 0, 0, 255, 0, 0, -1, -1, 7, 0, 0, 0],
        trace: vec![],
    };
    assert_eq!(
        focus::dispatch_focused(
            &mut s,
            0,
            KeyInput {
                character: 27,
                virtual_key: 27,
                flags: 8
            },
            &mut effects
        )
        .unwrap(),
        1
    );
    assert_eq!(effects.trace, ["live-child:0"]);
    assert_eq!(s.menu.controller.by_kind[&11], 11);
}
