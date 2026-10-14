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
use openxplane::{ui_menu_direct as direct, ui_menu_direct_graph};
impl direct::Services for Replay {
    fn prepare(&mut self, _state: &mut direct::State) {
        self.trace.push(format!("P:{}", _state.guard_c9));
    }
    fn controller(&mut self, _state: &mut direct::State) -> u64 {
        self.trace.push("M".into());
        7
    }
    fn close(
        &mut self,
        _state: &mut direct::State,
        _controller: u64,
        _kind: i32,
        _flag: u8,
    ) -> Result<(), &'static str> {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn notify(&mut self, _state: &mut direct::State) {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn query_a(&mut self, _state: &mut direct::State) -> u8 {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn document(&mut self, _state: &mut direct::State) -> Vec<u8> {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn validate(&mut self, _state: &mut direct::State, _document: &[u8]) -> u8 {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn load(&mut self, _state: &mut direct::State, _document: &[u8], _validation: u8) -> u8 {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn load_failed(&mut self, _state: &mut direct::State) {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn drop_document(&mut self, _state: &mut direct::State, _document: Vec<u8>) {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn allocate(&mut self, _state: &mut direct::State, _bytes: u64) -> u64 {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn callback(&mut self, _state: &mut direct::State, _id: i32) -> u64 {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn construct(
        &mut self,
        _state: &mut direct::State,
        _allocation: u64,
        _source: u64,
        _second: u64,
        _first: u64,
    ) -> u64 {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn drop_callback(&mut self, _state: &mut direct::State, _callback: u64) {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn field_c8_action(&mut self, _state: &mut direct::State, _object: u64) {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn component_260_action(&mut self, _state: &mut direct::State, _object: u64) {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn field_d8_action(&mut self, _state: &mut direct::State, _object: u64) {
        panic!("unexpected direct effect in mode0/2 profile")
    }
    fn destroy(&mut self, _state: &mut direct::State, _object: u64, _flag: u32) {
        panic!("unexpected direct effect in mode0/2 profile")
    }
}

#[test]
#[ignore = "requires combined original direct close graph corpus via OPENXPLANE_UI_MENU_DIRECT_GRAPH_VECTORS"]
fn native_complete_direct_close_graph_mode0_2() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_MENU_DIRECT_GRAPH_VECTORS").expect("corpus path"),
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
        let mut replay = Replay {
            trace: vec![],
            config,
            updates: 0,
        };
        let mut queries = vec![];
        let mut panel = direct::State {
            guard_c9: if count % 4 == 0 { 255 } else { 0 },
            flag_c8: if count % 3 == 0 { 255 } else { 0 },
            ui_mode: ((count % 2) * 2) as i32,
            ..Default::default()
        };
        for _ in 0..2 {
            ui_menu_direct_graph::dispatch(&mut panel, &mut s, 7, 0, &mut replay).unwrap();
            queries.push(u8::from(s.query(11).unwrap()));
        }
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
            [panel.guard_c9 as i32, panel.flag_c8 as i32, panel.ui_mode].to_vec(),
            csv::<i32>(v[24]),
            "{line}"
        );
        assert_eq!(replay.trace, &v[25..], "{line}");
        count += 1;
    }
    assert_eq!(count, 728);
}
