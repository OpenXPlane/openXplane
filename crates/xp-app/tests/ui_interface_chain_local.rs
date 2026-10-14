use openxplane::ui_interface_chain::{Node, Services, State};
struct Replay(Vec<String>);
impl Services for Replay {
    fn free(&mut self, s: &mut State, i: usize, n: usize) {
        self.0
            .push(format!("F:{i}:{n}:{}", s.head.map_or(-1, |i| i as i32)));
    }
}
#[test]
fn middle_unlink_keeps_detached_links_and_updates_neighbors() {
    let mut s = State {
        nodes: vec![Node::default(); 3],
        ..Default::default()
    };
    for i in 0..3 {
        s.construct(i).unwrap();
    }
    let mut calls = Replay(vec![]);
    s.destroy(1, 257, &mut calls).unwrap();
    assert_eq!(s.head, Some(2));
    assert_eq!(s.nodes[2].older, Some(0));
    assert_eq!(s.nodes[0].newer, Some(2));
    assert_eq!(s.nodes[1].newer, Some(2));
    assert_eq!(s.nodes[1].older, Some(0));
    assert_eq!(calls.0, ["F:1:24:2"]);
}
#[test]
#[ignore = "requires original interface chain corpus via OPENXPLANE_UI_INTERFACE_CHAIN_VECTORS"]
fn native_interface_ctor_and_destructor() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_INTERFACE_CHAIN_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let size = v[1].parse::<usize>().unwrap();
        let added = v[2].parse::<usize>().unwrap();
        let pos = v[3].parse::<usize>().unwrap();
        let flags = v[4].parse::<u32>().unwrap();
        let n = size + added;
        let mut s = State {
            nodes: vec![Node::default(); 6],
            ..Default::default()
        };
        for i in (0..size).rev().chain(size..n) {
            assert_eq!(s.construct(i).unwrap(), i);
            s.nodes[i].table = 0x1426067c8;
        }
        let active = (size..n).rev().chain(0..size).collect::<Vec<_>>();
        let mut calls = Replay(vec![]);
        let target = active[pos];
        assert_eq!(s.destroy(target, flags, &mut calls).unwrap(), target);
        assert_eq!(v[5], "->");
        assert_eq!(
            s.head.map_or(-1, |i| i as i32),
            v[6].parse::<i32>().unwrap(),
            "{line}"
        );
        let csv = |s: &str| {
            s.split(',')
                .map(|v| v.parse::<u64>().unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            s.nodes[..n].iter().map(|n| n.table).collect::<Vec<_>>(),
            csv(v[7]),
            "{line}"
        );
        let links = |s: &str| {
            s.split(',')
                .map(|v| v.parse::<i32>().unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            s.nodes[..n]
                .iter()
                .map(|n| n.newer.map_or(-1, |i| i as i32))
                .collect::<Vec<_>>(),
            links(v[8]),
            "{line}"
        );
        assert_eq!(
            s.nodes[..n]
                .iter()
                .map(|n| n.older.map_or(-1, |i| i as i32))
                .collect::<Vec<_>>(),
            links(v[9]),
            "{line}"
        );
        assert_eq!(calls.0, &v[10..], "{line}");
        assert_eq!(s.nodes[target].free_requested, flags & 1 != 0);
        count += 1;
    }
    assert_eq!(count, 210);
}
