use openxplane::{
    ui_menu_close_graph::{self, State as CloseState},
    ui_state::RegisteredWindow,
    ui_widget_tree::{self, Node, Tree},
    ui_window_graph::{Services, State},
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
struct Replay {
    focus_case: usize,
    pending: [u8; 2],
    index: usize,
    cache: u8,
    update: i32,
    result: u64,
    directv: Vec<i128>,
    trace: Vec<String>,
    config: Vec<i64>,
    updates: usize,
}
impl ui_widget_tree::Services for Replay {
    fn bind(&mut self, _: &mut Tree, child: usize, target: u64) {
        self.trace.push(format!("B:{child}:{target}"));
    }
    fn removed(&mut self, _: &mut Tree, parent: usize, child: usize) {
        self.trace.push(format!("D:{parent}:{child}"));
    }
    fn added(&mut self, _: &mut Tree, parent: usize, child: usize) {
        self.trace.push(format!("A:{parent}:{child}"));
    }
}
impl Services for Replay {
    fn owner(&mut self, i: usize, attached: bool) {
        self.trace.push(format!("O:{i}:{}", u8::from(attached)));
    }
    fn prepare_hide(&mut self, i: usize) {
        self.trace.push(format!("H:{i}"));
    }
    fn activate(&mut self, i: usize, show: bool) {
        self.trace.push(format!("B:{i}:{}", u8::from(show)));
    }
    fn refresh(&mut self) {
        self.trace.push("R".into());
    }
    fn event(&mut self, token: u64) {
        self.trace.push(format!("E:4:1:{token}"));
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
fn build(paths: &[usize], classes: &[i32], impl_alpha: f32) -> State {
    let mut s = State {
        tree: Tree {
            nodes: vec![Node::default()],
        },
        alpha: vec![impl_alpha],
        implementation_node: 0,
        ..Default::default()
    };
    for (i, &pid) in paths.iter().enumerate() {
        let mut parent = None;
        for &alpha in PATHS[pid] {
            let index = s.tree.nodes.len();
            s.tree.nodes.push(Node {
                parent,
                flags: (index % 4) as u32,
                ..Default::default()
            });
            s.alpha.push(alpha);
            if let Some(parent) = parent {
                s.tree.nodes[parent].children.push(index);
            }
            parent = Some(index);
        }
        s.widgets.push(parent.unwrap());
        s.windows.registry.entries.push(RegisteredWindow {
            token: [11, 12, 11][i],
            classification: classes[i],
            opacity_root_to_leaf: vec![],
        });
    }
    s
}

impl ui_menu_close_graph::Services for Replay {
    fn update(&mut self, s: &mut CloseState, kind: i32, flag: u8) {
        assert_eq!(kind, 11);
        self.trace.push(format!("U:{flag}:{}", s.by_kind[&kind]));
        if self.updates == 0 && self.config[1] != 0 {
            s.by_kind.insert(kind, 12);
        }
        self.updates += 1;
    }
    fn reset(&mut self, s: &mut CloseState, token: u64, mode: i32, extra: i32) {
        self.trace.push(format!("S:{token}:{mode}:{extra}"));
        let i = s.index(token).unwrap();
        if self.config[6] >= 0 {
            s.modes[i] = self.config[6] as i32;
        }
        if self.config[7] >= 0 {
            s.graph.alpha[s.graph.widgets[i]] = self.config[7] as f32;
        }
    }
    fn delete(&mut self, _: &mut CloseState, token: u64, flag: u8) {
        self.trace.push(format!("X:{token}:{flag}"));
    }
    fn environment(&mut self, _: &mut CloseState) -> u8 {
        self.trace.push("Q".into());
        self.config[5] as u8
    }
    fn command_once(&mut self, _: &mut CloseState) {
        self.trace.push("C".into());
    }
    fn controller_refresh(&mut self, _: &mut CloseState) {
        self.trace.push("P".into());
    }
    fn final_update(&mut self, s: &mut CloseState, kind: i32) {
        self.trace.push(format!("F:{}", s.by_kind[&kind]));
    }
}
use openxplane::ui_menu_direct as direct;
impl direct::Services for Replay {
    fn prepare(&mut self, s: &mut direct::State) {
        self.trace.push(format!("P:{}", s.guard_c9));
        if self.directv[7] >= 0 {
            s.flag_c8 = self.directv[7] as u8;
        }
        if self.directv[11] != 0 {
            direct::dispatch(s, -5, self).unwrap();
        }
    }
    fn controller(&mut self, _: &mut direct::State) -> u64 {
        self.trace.push("M".into());
        7
    }
    fn close(
        &mut self,
        s: &mut direct::State,
        controller: u64,
        kind: i32,
        flag: u8,
    ) -> Result<(), &'static str> {
        assert_eq!(controller, 7);
        self.trace.push(format!("C:{kind}:{flag}:{}", s.guard_c9));
        if self.directv[8] >= 0 {
            s.ui_mode = self.directv[8] as i32;
        }
        Ok(())
    }
    fn notify(&mut self, _: &mut direct::State) {
        self.trace.push("N".into());
    }
    fn query_a(&mut self, _: &mut direct::State) -> u8 {
        self.trace.push("Q".into());
        self.directv[4] as u8
    }
    fn document(&mut self, _: &mut direct::State) -> Vec<u8> {
        self.trace.push("F".into());
        [b"".as_slice(), b"a", b"test.sit", b"xxxxxxxxxxxxxxx"][self.directv[12] as usize].to_vec()
    }
    fn validate(&mut self, _: &mut direct::State, document: &[u8]) -> u8 {
        let hex = if document.is_empty() {
            "-".into()
        } else {
            document
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        self.trace.push(format!("V:{hex}"));
        self.directv[5] as u8
    }
    fn load(&mut self, _: &mut direct::State, _: &[u8], validation: u8) -> u8 {
        self.trace.push(format!("L:{validation}"));
        self.directv[6] as u8
    }
    fn load_failed(&mut self, _: &mut direct::State) {
        self.trace.push("FAIL".into());
    }
    fn drop_document(&mut self, _: &mut direct::State, _: Vec<u8>) {
        self.trace.push("DROP".into());
    }
    fn allocate(&mut self, _: &mut direct::State, bytes: u64) -> u64 {
        self.trace.push(format!("A:{bytes}"));
        100
    }
    fn callback(&mut self, _: &mut direct::State, id: i32) -> u64 {
        self.trace.push(format!("H:{id}"));
        self.trace.push(format!("B:{id}"));
        200 + id as u64
    }
    fn construct(
        &mut self,
        _: &mut direct::State,
        allocation: u64,
        source: u64,
        second: u64,
        first: u64,
    ) -> u64 {
        assert_eq!((allocation, second, first), (100, 203, 200));
        self.trace.push(format!("T:{source}:3:0"));
        100
    }
    fn drop_callback(&mut self, _: &mut direct::State, callback: u64) {
        self.trace.push(format!("D:{}", callback - 200));
    }
    fn field_c8_action(&mut self, s: &mut direct::State, object: u64) {
        assert_eq!(object, 100);
        assert_eq!(s.flag_7770c6, 1);
        self.trace.push("E:c8:1".into());
    }
    fn component_260_action(&mut self, _: &mut direct::State, object: u64) {
        assert_eq!(object, 100);
        self.trace.push("E:260".into());
    }
    fn field_d8_action(&mut self, _: &mut direct::State, object: u64) {
        assert_eq!(object, 100);
        self.trace.push("E:d8".into());
    }
    fn destroy(&mut self, _: &mut direct::State, object: u64, flag: u32) {
        assert_eq!(object, 100);
        self.trace.push(format!("X:{flag}"));
    }
}

use openxplane::ui_menu_action as action;
use openxplane::{ui_input::KeyInput, ui_menu_key_graph as keys};
impl action::Services for Replay {
    fn query_a(&mut self, s: &mut action::State) -> u8 {
        let index = self.index;
        self.index += 1;
        self.trace.push(format!("P:{index}"));
        if index == 0 && self.update >= 0 {
            s.engine_byte = self.update as u8;
        }
        self.pending[index]
    }
    fn query_b(&mut self, _: &mut action::State) -> u8 {
        self.trace.push("Q".into());
        self.cache
    }
    fn direct(&mut self, _: &mut action::State, argument: i32) -> Result<(), &'static str> {
        self.trace.push(format!("V:{argument}"));
        Ok(())
    }
    fn allocate(&mut self, _: &mut action::State, bytes: u64) -> u64 {
        self.trace.push(format!("A:{bytes}"));
        1
    }
    fn callback(&mut self, _: &mut action::State, index: u8) -> u64 {
        self.trace.push(format!("H:{index}"));
        200 + u64::from(index)
    }
    fn construct(
        &mut self,
        _: &mut action::State,
        allocation: u64,
        second: u64,
        first: u64,
    ) -> u64 {
        assert_eq!((allocation, second, first), (1, 201, 200));
        self.trace.push("C:1:0".into());
        1
    }
    fn drop_callback(&mut self, _: &mut action::State, callback: u64) {
        self.trace.push(format!("D:{}", callback - 200));
    }
    fn manager(&mut self, _: &mut action::State) -> u64 {
        self.trace.push("M".into());
        18
    }
    fn registry_input(&mut self, _: &mut action::State, object: u64) -> u64 {
        assert_eq!(object, 1);
        self.trace.push("I:17".into());
        19
    }
    fn resolve(&mut self, _: &mut action::State, manager: u64, input: u64) -> u64 {
        self.trace.push(format!("R:{manager}:{input}"));
        self.result
    }
    fn destroy(&mut self, _: &mut action::State, object: u64, flag: u32) {
        assert_eq!(object, 1);
        self.trace.push(format!("X:{flag}"));
    }
    fn submit(&mut self, _: &mut action::State, word: i32, result: u64) {
        self.trace.push(format!("S:{word}:{result}"));
    }
}

#[test]
#[ignore = "requires combined original direct close graph corpus via OPENXPLANE_UI_MENU_FOCUS_GRAPH_VECTORS"]
fn native_mutable_focus_owner_key_graph() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_MENU_FOCUS_GRAPH_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let classes: Vec<i32> = csv(v[7]);
        let config: Vec<i64> = csv(v[12]);
        let mut graph = build(
            &v[4..7]
                .iter()
                .map(|p| p.parse().unwrap())
                .collect::<Vec<usize>>(),
            &classes,
            [1.0, 0.0, -1.0, 0.5][v[11].parse::<usize>().unwrap()],
        );
        graph.windows.root_manager = v[1] == "1";
        graph.windows.registry.order = csv(v[8]);
        graph.windows.membership = csv(v[9]);
        graph.windows.attached_manager = csv::<u8>(v[10]).iter().map(|&v| v != 0).collect();
        let mut s = CloseState {
            graph,
            modes: (0..3).map(|i| [0, 4, 6][(count + i) % 3]).collect(),
            delete_kinds: if config[2] != 0 { vec![11] } else { vec![] },
            flag_11f48: config[3] as i32,
            flag_2a8: config[4] as u8,
            ..Default::default()
        };
        s.by_kind.insert(11, config[0] as u64);
        let initial_engine = [0, 1, 255, 0][count % 4];
        let action = action::State {
            engine_byte: initial_engine,
            ui_mode: (count % 3) as i32,
        };
        let mut replay = Replay {
            focus_case: count,
            pending: [
                ((count % 2) * 255) as u8,
                if count % 7 == 0 { 0 } else { 255 },
            ],
            index: 0,
            cache: if count % 5 == 0 { 0 } else { 255 },
            update: if count % 11 == 0 { 255 } else { -1 },
            result: 42,
            directv: vec![
                0,
                0,
                0,
                (count % 3) as i128,
                ((count % 2) * 255) as i128,
                (count % 256) as i128,
                if count % 5 == 0 { 0 } else { 255 },
                -1,
                -1,
                (count + 7) as i128,
                0,
                0,
                2,
            ],
            trace: vec![],
            config,
            updates: 0,
        };
        let mut queries = vec![];
        let panel = direct::State {
            guard_c9: if count % 4 == 0 { 255 } else { 0 },
            flag_c8: if count % 3 == 0 { 255 } else { 0 },
            ui_mode: (count % 3) as i32,
            source_token: (count + 7) as u64,
            ..Default::default()
        };
        let input = KeyInput {
            character: 27,
            virtual_key: [27, 13, 0xbb, 0xbc, 65][count % 5],
            flags: [8, 16, 8, 0, 8, 8, 8][count % 7],
        };
        let combined = keys::State {
            key: openxplane::ui_menu_key::State {
                flag168: if count % 13 == 0 { 255 } else { 0 },
                flag169: if count % 11 == 0 { 255 } else { 0 },
            },
            action,
            panel,
            controller: s,
            controller_identity: 7,
        };
        let n = combined.controller.graph.tree.nodes.len();
        let panel_binding = n + 3;
        let mut owners = vec![Some(0); n + 4];
        owners[0] = Some(panel_binding);
        owners[panel_binding] = None;
        let mut selected = vec![None; n + 4];
        selected[0] = Some(n + count % 3);
        selected[panel_binding] = Some(0);
        let mut combined = focus::State {
            menu: combined,
            binding: binding::State {
                nodes: (0..n + 4)
                    .map(|i| binding::Node {
                        owner: owners[i],
                        selected: selected[i],
                    })
                    .collect(),
                focused: if count % 23 == 0 {
                    None
                } else {
                    Some(panel_binding)
                },
            },
            entry_bindings: (n..n + 3).collect(),
            widget_bindings: (0..n).collect(),
            targets: (0..n).map(|i| (i as u64 + 1, i)).collect(),
            panel_binding,
        };
        let mut results = vec![];
        for route in 0..2 {
            replay.index = 0;
            results.push(if route == 0 {
                focus::dispatch(&mut combined, input, &mut replay).unwrap()
            } else {
                focus::dispatch_focused(
                    &mut combined,
                    if count % 17 == 0 { 0 } else { 0xfff },
                    input,
                    &mut replay,
                )
                .unwrap()
            });
            queries.push(u8::from(combined.menu.controller.query(11).unwrap()));
        }
        let binding = combined.binding;
        let combined = combined.menu;
        let s = combined.controller;
        let panel = combined.panel;
        let action = combined.action;
        assert_eq!(v[13], "->");
        assert_eq!(
            s.graph.windows.registry.order,
            csv::<usize>(v[14]),
            "{line}"
        );
        assert_eq!(s.graph.windows.membership, csv::<usize>(v[15]), "{line}");
        assert_eq!(
            s.graph.windows.attached_manager,
            csv::<u8>(v[16]).iter().map(|&v| v != 0).collect::<Vec<_>>(),
            "{line}"
        );
        assert_eq!(
            s.graph
                .widgets
                .iter()
                .map(|&i| s.graph.alpha[i].to_bits())
                .collect::<Vec<_>>(),
            csv::<u32>(v[17]),
            "{line}"
        );
        assert_eq!(
            s.graph
                .tree
                .nodes
                .iter()
                .map(|n| n.parent.map_or(-1, |i| i as i32))
                .collect::<Vec<_>>(),
            csv::<i32>(v[18]),
            "{line}"
        );
        assert_eq!(
            s.graph
                .tree
                .nodes
                .iter()
                .map(|n| n.flags)
                .collect::<Vec<_>>(),
            csv::<u32>(v[19]),
            "{line}"
        );
        assert_eq!(
            s.graph
                .tree
                .nodes
                .iter()
                .map(|n| n.children.clone())
                .collect::<Vec<_>>(),
            v[20].split(';').map(csv::<usize>).collect::<Vec<_>>(),
            "{line}"
        );
        assert_eq!(queries, csv::<u8>(v[21]), "{line}");
        assert_eq!(s.modes, csv::<i32>(v[22]), "{line}");
        assert_eq!(s.by_kind[&11], v[23].parse::<u64>().unwrap(), "{line}");
        assert_eq!(
            [
                panel.guard_c9 as i32,
                panel.flag_c8 as i32,
                panel.ui_mode,
                panel.flag_7770c6 as i32,
                action.engine_byte as i32
            ]
            .to_vec(),
            csv::<i32>(v[24]),
            "{line}"
        );
        assert_eq!(results, csv::<u8>(v[25]), "{line}");
        assert_eq!(
            binding
                .nodes
                .iter()
                .map(|n| n.owner.map_or(-1, |i| i as i32))
                .collect::<Vec<_>>(),
            csv::<i32>(v[26]),
            "{line}"
        );
        assert_eq!(
            binding
                .nodes
                .iter()
                .map(|n| n.selected.map_or(-1, |i| i as i32))
                .collect::<Vec<_>>(),
            csv::<i32>(v[27]),
            "{line}"
        );
        assert_eq!(
            binding.focused.map_or(-1, |i| i as i32),
            v[28].parse::<i32>().unwrap(),
            "{line}"
        );
        assert_eq!(replay.trace, &v[29..], "{line}");
        count += 1;
    }
    assert_eq!(count, 728);
}

use openxplane::{ui_focus_binding as binding, ui_menu_focus_graph as focus};
impl binding::Services for Replay {
    fn blur(&mut self, s: &mut binding::State, i: usize) {
        self.trace.push(format!("BL:{i}"));
        if self.focus_case.is_multiple_of(2) {
            s.focused = None;
        }
        if self.focus_case.is_multiple_of(29) {
            s.nodes[i].owner = None;
        }
    }
    fn stop(&mut self, s: &mut binding::State, i: usize) -> u8 {
        assert_eq!(s.nodes[i].selected, None);
        self.trace.push(format!("ST:{i}"));
        if i == 0 && self.focus_case.is_multiple_of(31) {
            s.nodes[i].owner = None;
        }
        if i == 0 && self.focus_case.is_multiple_of(37) {
            255
        } else {
            0
        }
    }
}
impl focus::Services for Replay {
    fn child(&mut self, _: &mut binding::State, i: usize, _: KeyInput) -> u8 {
        self.trace.push(format!("CH:{i}"));
        if self.focus_case.is_multiple_of(19) {
            255
        } else {
            0
        }
    }
}
