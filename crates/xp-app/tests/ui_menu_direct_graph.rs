include!("ui_menu_direct_local.rs");
use openxplane::{
    ui_menu_close_graph as close, ui_menu_direct_graph, ui_state::RegisteredWindow,
    ui_widget_tree as tree, ui_window_graph as graph,
};
impl tree::Services for Replay {
    fn bind(&mut self, _: &mut tree::Tree, _: usize, _: u64) {
        self.trace.push("graph-bind".into());
    }
    fn removed(&mut self, _: &mut tree::Tree, _: usize, _: usize) {}
    fn added(&mut self, _: &mut tree::Tree, _: usize, _: usize) {}
}
impl graph::Services for Replay {
    fn owner(&mut self, _: usize, _: bool) {}
    fn prepare_hide(&mut self, _: usize) {}
    fn activate(&mut self, _: usize, _: bool) {}
    fn refresh(&mut self) {
        self.trace.push("graph-refresh".into());
    }
    fn event(&mut self, _: u64) {}
}
impl close::Services for Replay {
    fn update(&mut self, _: &mut close::State, _: i32, _: u8) {}
    fn reset(&mut self, _: &mut close::State, _: u64, _: i32, _: i32) {}
    fn delete(&mut self, _: &mut close::State, _: u64, _: u8) {
        self.trace.push("graph-delete-request".into());
    }
    fn environment(&mut self, _: &mut close::State) -> u8 {
        0
    }
    fn command_once(&mut self, _: &mut close::State) {}
    fn controller_refresh(&mut self, _: &mut close::State) {}
    fn final_update(&mut self, _: &mut close::State, _: i32) {
        self.trace.push("graph-final".into());
    }
}
fn controller() -> close::State {
    let mut c = close::State {
        modes: vec![0],
        delete_kinds: vec![11],
        ..Default::default()
    };
    c.by_kind.insert(11, 11);
    c.graph.tree.nodes = vec![tree::Node::default(), tree::Node::default()];
    c.graph.alpha = vec![1.0, 1.0];
    c.graph.widgets = vec![1];
    c.graph.implementation_node = 0;
    c.graph.windows.registry.entries = vec![RegisteredWindow {
        token: 11,
        classification: 2,
        opacity_root_to_leaf: vec![],
    }];
    c.graph.windows.registry.order = vec![0];
    c.graph.windows.membership = vec![0];
    c.graph.windows.attached_manager = vec![true];
    c
}
#[test]
fn direct_action_uses_real_graph_close_and_keeps_guard() {
    let mut panel = State::default();
    let mut c = controller();
    let mut effects = Replay {
        v: vec![0, 0, 0, 0, 0, 0, 0, -1, -1, 7, 0, 1, 0],
        trace: vec![],
    };
    ui_menu_direct_graph::dispatch(&mut panel, &mut c, 7, 0, &mut effects).unwrap();
    assert_eq!(panel.guard_c9, 0);
    assert_eq!(c.by_kind[&11], 0);
    assert!(!c.graph.visible(11).unwrap());
    assert_eq!(c.graph.alpha[1], 0.0);
    assert!(c.graph.windows.membership.is_empty());
    assert_eq!(
        effects.trace,
        [
            "P:1",
            "M",
            "graph-bind",
            "graph-refresh",
            "graph-delete-request",
            "graph-final"
        ]
    );
}
#[test]
fn wrong_controller_propagates_error_before_mutating_graph() {
    let mut panel = State::default();
    let mut c = controller();
    let mut effects = Replay {
        v: vec![0, 0, 0, 0, 0, 0, 0, -1, -1, 7, 0, 0, 0],
        trace: vec![],
    };
    assert!(ui_menu_direct_graph::dispatch(&mut panel, &mut c, 8, 0, &mut effects).is_err());
    assert_eq!(c.by_kind[&11], 11);
    assert!(c.graph.visible(11).unwrap());
    assert_eq!(effects.trace, ["P:1", "M"]);
}
