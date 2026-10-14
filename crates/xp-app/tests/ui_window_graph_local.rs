use openxplane::{
    ui_state::RegisteredWindow,
    ui_widget_tree::{self, Node, Tree},
    ui_window_graph::{self, Services, State},
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
impl ui_widget_tree::Services for Replay {
    fn bind(&mut self, _: &mut Tree, child: usize, target: u64) {
        self.0.push(format!("B:{child}:{target}"));
    }
    fn removed(&mut self, _: &mut Tree, parent: usize, child: usize) {
        self.0.push(format!("D:{parent}:{child}"));
    }
    fn added(&mut self, _: &mut Tree, parent: usize, child: usize) {
        self.0.push(format!("A:{parent}:{child}"));
    }
}
impl Services for Replay {
    fn owner(&mut self, i: usize, attached: bool) {
        self.0.push(format!("O:{i}:{}", u8::from(attached)));
    }
    fn prepare_hide(&mut self, i: usize) {
        self.0.push(format!("H:{i}"));
    }
    fn activate(&mut self, i: usize, show: bool) {
        self.0.push(format!("B:{i}:{}", u8::from(show)));
    }
    fn refresh(&mut self) {
        self.0.push("R".into());
    }
    fn event(&mut self, token: u64) {
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
#[test]
fn show_replaces_zero_ancestor_and_next_hide_detaches() {
    let mut s = build(&[4], &[2], 1.0);
    s.windows.registry.order = vec![0];
    s.windows.attached_manager = vec![false];
    let mut replay = Replay::default();
    assert!(!s.visible(11).unwrap());
    ui_window_graph::dispatch(&mut s, 11, 256, &mut replay).unwrap();
    assert!(s.visible(11).unwrap());
    assert_eq!(s.tree.nodes[s.widgets[0]].parent, Some(0));
    ui_window_graph::toggle(&mut s, 11, &mut replay).unwrap();
    assert!(!s.visible(11).unwrap());
    assert_eq!(s.tree.nodes[s.widgets[0]].parent, None);
    assert!(s.windows.membership.is_empty());
    assert_eq!(s.alpha[1], 0.0);
}
#[test]
#[ignore = "requires combined original graph corpus via OPENXPLANE_UI_WINDOW_GRAPH_VECTORS"]
fn native_shared_window_widget_graph_action_sequence() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_WINDOW_GRAPH_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let classes: Vec<i32> = csv(v[7]);
        let mut s = build(
            &v[4..7]
                .iter()
                .map(|p| p.parse().unwrap())
                .collect::<Vec<usize>>(),
            &classes,
            [1.0, 0.0, -1.0, 0.5][v[11].parse::<usize>().unwrap()],
        );
        s.windows.root_manager = v[1] == "1";
        s.windows.registry.order = csv(v[8]);
        s.windows.membership = csv(v[9]);
        s.windows.attached_manager = csv::<u8>(v[10]).iter().map(|&v| v != 0).collect();
        let mut replay = Replay::default();
        let mut queries = vec![];
        for word in &v[2..4] {
            ui_window_graph::dispatch(&mut s, 11, word.parse().unwrap(), &mut replay).unwrap();
            queries.push(u8::from(s.visible(11).unwrap()));
        }
        ui_window_graph::toggle(&mut s, 11, &mut replay).unwrap();
        queries.push(u8::from(s.visible(11).unwrap()));
        assert_eq!(v[12], "->");
        assert_eq!(s.windows.registry.order, csv::<usize>(v[13]), "{line}");
        assert_eq!(s.windows.membership, csv::<usize>(v[14]), "{line}");
        assert_eq!(
            s.windows.attached_manager,
            csv::<u8>(v[15]).iter().map(|&v| v != 0).collect::<Vec<_>>(),
            "{line}"
        );
        assert_eq!(
            s.widgets
                .iter()
                .map(|&i| s.alpha[i].to_bits())
                .collect::<Vec<_>>(),
            csv::<u32>(v[16]),
            "{line}"
        );
        assert_eq!(
            s.tree
                .nodes
                .iter()
                .map(|n| n.parent.map_or(-1, |i| i as i32))
                .collect::<Vec<_>>(),
            csv::<i32>(v[17]),
            "{line}"
        );
        assert_eq!(
            s.tree.nodes.iter().map(|n| n.flags).collect::<Vec<_>>(),
            csv::<u32>(v[18]),
            "{line}"
        );
        assert_eq!(
            s.tree
                .nodes
                .iter()
                .map(|n| n.children.clone())
                .collect::<Vec<_>>(),
            v[19].split(';').map(csv::<usize>).collect::<Vec<_>>(),
            "{line}"
        );
        assert_eq!(queries, csv::<u8>(v[20]), "{line}");
        assert_eq!(replay.0, &v[21..], "{line}");
        count += 1;
    }
    assert_eq!(count, 728);
}
