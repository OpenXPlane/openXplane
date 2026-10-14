include!("ui_menu_action_graph.rs");
use openxplane::{ui_input::KeyInput, ui_menu_key_graph as keys};
#[test]
fn escape_key_runs_complete_graph_chain_and_releases_decline() {
    let mut s = keys::State {
        controller: controller(),
        controller_identity: 7,
        ..Default::default()
    };
    s.action.engine_byte = 1;
    let mut effects = Replay {
        v: vec![0, 0, 0, 0, 0, 0, 0, -1, -1, 7, 0, 0, 0],
        trace: vec![],
    };
    let input = KeyInput {
        character: 27,
        virtual_key: 27,
        flags: 16,
    };
    assert_eq!(keys::dispatch(&mut s, input, &mut effects).unwrap(), 0);
    assert!(effects.trace.is_empty());
    assert_eq!(
        keys::dispatch_focused(
            &mut s,
            true,
            8,
            KeyInput { flags: 8, ..input },
            &mut effects
        )
        .unwrap(),
        1
    );
    assert_eq!(s.controller.by_kind[&11], 0);
    assert!(!s.controller.graph.visible(11).unwrap());
}
#[test]
fn escape_noop_flag_and_focus_filter_leave_window_present() {
    let mut s = keys::State {
        controller: controller(),
        controller_identity: 7,
        ..Default::default()
    };
    s.action.engine_byte = 1;
    s.key.flag169 = 255;
    let mut effects = Replay {
        v: vec![],
        trace: vec![],
    };
    let input = KeyInput {
        character: 27,
        virtual_key: 27,
        flags: 8,
    };
    assert_eq!(keys::dispatch(&mut s, input, &mut effects).unwrap(), 1);
    assert!(effects.trace.is_empty());
    assert_eq!(
        keys::dispatch_focused(&mut s, true, 0, input, &mut effects).unwrap(),
        0
    );
    assert_eq!(
        keys::dispatch_focused(&mut s, false, 8, input, &mut effects).unwrap(),
        0
    );
    assert!(s.controller.query(11).unwrap());
}
