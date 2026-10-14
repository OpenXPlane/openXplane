use openxplane::ui_widget_tree::{self, Node, Services, Tree};
struct Replay {
    effect: u8,
    redirect: usize,
    mutflags: u32,
    trace: Vec<String>,
}
impl Services for Replay {
    fn bind(&mut self, t: &mut Tree, child: usize, target: u64) {
        assert_eq!(child, 4);
        self.trace.push(format!("B:{target}"));
        if self.effect == 3 {
            t.nodes[child].parent = Some(self.redirect);
        }
    }
    fn removed(&mut self, t: &mut Tree, parent: usize, child: usize) {
        self.trace.push(format!("D:{parent}:{child}"));
        if self.effect == 1 {
            t.nodes[child].parent = Some(self.redirect);
        }
    }
    fn added(&mut self, t: &mut Tree, parent: usize, child: usize) {
        self.trace.push(format!("A:{parent}:{child}"));
        if self.effect == 2 {
            t.nodes[child].parent = Some(self.redirect);
            t.nodes[child].flags = self.mutflags;
        }
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
fn optional(v: i32) -> Option<usize> {
    (v >= 0).then_some(v as usize)
}
#[test]
fn dirty_only_propagates_new_bits_and_transforms_them() {
    let mut t = Tree {
        nodes: vec![
            Node::default(),
            Node {
                parent: Some(0),
                ..Default::default()
            },
            Node {
                parent: Some(1),
                ..Default::default()
            },
        ],
    };
    t.dirty(2, 1).unwrap();
    assert_eq!(
        t.nodes.iter().map(|n| n.flags).collect::<Vec<_>>(),
        [2, 2, 1]
    );
    t.nodes[0].flags = 0;
    t.dirty(2, 1).unwrap();
    assert_eq!(t.nodes[0].flags, 0);
    t.dirty(2, 4).unwrap();
    assert_eq!(
        t.nodes.iter().map(|n| n.flags).collect::<Vec<_>>(),
        [7, 7, 5]
    );
}
#[test]
#[ignore = "requires local original reparent corpus via OPENXPLANE_UI_WIDGET_TREE_VECTORS"]
fn native_reparent_callback_mutations_and_dirty_bits() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_WIDGET_TREE_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let old = v[1].parse::<i32>().unwrap();
        let new = v[2].parse::<i32>().unwrap();
        let flags: Vec<u32> = csv(v[5]);
        let parents = [None, None, Some(0), Some(1), optional(old), None];
        let children = [
            vec![2],
            vec![3],
            vec![5, 4, 4, 5],
            vec![4, 5, 4],
            vec![],
            vec![],
        ];
        let mut tree = Tree {
            nodes: (0..6)
                .map(|i| Node {
                    parent: parents[i],
                    flags: flags[i],
                    children: children[i].clone(),
                    binding_override: if i == 4 { v[3].parse().unwrap() } else { 0 },
                })
                .collect(),
        };
        let mut replay = Replay {
            effect: v[4].parse().unwrap(),
            mutflags: v[6].parse().unwrap(),
            redirect: v[7].parse().unwrap(),
            trace: vec![],
        };
        ui_widget_tree::reparent(&mut tree, 4, optional(new), &mut replay).unwrap();
        assert_eq!(v[8], "->");
        assert_eq!(
            tree.nodes
                .iter()
                .map(|n| n.parent.map_or(-1, |i| i as i32))
                .collect::<Vec<_>>(),
            csv::<i32>(v[9]),
            "{line}"
        );
        assert_eq!(
            tree.nodes.iter().map(|n| n.flags).collect::<Vec<_>>(),
            csv::<u32>(v[10]),
            "{line}"
        );
        assert_eq!(
            tree.nodes
                .iter()
                .map(|n| n.children.clone())
                .collect::<Vec<_>>(),
            v[11].split(';').map(csv::<usize>).collect::<Vec<_>>(),
            "{line}"
        );
        assert_eq!(replay.trace, &v[12..], "{line}");
        count += 1;
    }
    assert_eq!(count, 744);
}

#[test]
#[ignore = "requires local original dirty corpus via OPENXPLANE_UI_WIDGET_TREE_VECTORS"]
fn native_standalone_dirty_all_bits() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_WIDGET_TREE_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("D ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let parents: Vec<i32> = csv(v[1]);
        let flags: Vec<u32> = csv(v[2]);
        let mut t = Tree {
            nodes: parents
                .iter()
                .zip(&flags)
                .map(|(&p, &flags)| Node {
                    parent: optional(p),
                    flags,
                    ..Default::default()
                })
                .collect(),
        };
        t.dirty(4, v[3].parse().unwrap()).unwrap();
        assert_eq!(v[4], "->");
        assert_eq!(
            t.nodes.iter().map(|n| n.flags).collect::<Vec<_>>(),
            csv::<u32>(v[5]),
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 744);
}
