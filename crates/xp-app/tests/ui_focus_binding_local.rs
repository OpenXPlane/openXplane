use openxplane::ui_focus_binding::{self, Node, Services, State};
struct Replay {
    v: Vec<i32>,
    trace: Vec<String>,
}
fn opt(i: i32) -> Option<usize> {
    (i >= 0).then_some(i as usize)
}
impl Services for Replay {
    fn blur(&mut self, s: &mut State, i: usize) {
        self.trace.push(format!("B:{i}"));
        if self.v[7] != -2 {
            s.nodes[3].owner = opt(self.v[7]);
        }
        if self.v[11] != -2 {
            s.focused = opt(self.v[11]);
        }
    }
    fn stop(&mut self, s: &mut State, i: usize) -> u8 {
        assert_eq!(s.nodes[i].selected, None);
        self.trace.push(format!("S:{i}"));
        if i as i32 == self.v[8] && self.v[9] != -2 {
            s.nodes[i].owner = opt(self.v[9]);
        }
        if self.v[10] != 0 {
            s.nodes[i].selected = Some(5);
        }
        if i as i32 == self.v[5] {
            self.v[6] as u8
        } else {
            0
        }
    }
}
fn state(v: &[i32]) -> State {
    let owners = [-1, 0, 1, if v[0] == -1 { -1 } else { 2 }, -1, -1];
    let selected = [v[0], v[1], v[2], 4, -1, -1];
    State {
        nodes: (0..6)
            .map(|i| Node {
                owner: opt(owners[i]),
                selected: opt(selected[i]),
            })
            .collect(),
        focused: opt(v[3]),
    }
}
#[test]
fn blur_reloads_owner_and_callback_changes_survive_final_store() {
    let v = vec![1, 2, 3, 0, 4, -1, 255, 4, 4, -1, 1, 4];
    let mut s = state(&v);
    let mut calls = Replay { v, trace: vec![] };
    ui_focus_binding::bind(&mut s, 3, Some(4), &mut calls).unwrap();
    assert_eq!(calls.trace, ["B:4", "S:4"]);
    assert_eq!(s.nodes[3].owner, Some(4));
    assert_eq!(s.nodes[4].selected, Some(5));
    assert_eq!(s.focused, Some(4));
}
#[test]
#[ignore = "requires original binding corpus via OPENXPLANE_UI_FOCUS_BINDING_VECTORS"]
fn native_binding_owner_selected_mutations() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_FOCUS_BINDING_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("B ")) {
        let fields: Vec<_> = line.split_whitespace().collect();
        let v: Vec<i32> = fields[1..13].iter().map(|v| v.parse().unwrap()).collect();
        let mut s = state(&v);
        let requested = opt(v[4]);
        let mut calls = Replay { v, trace: vec![] };
        ui_focus_binding::bind(&mut s, 3, requested, &mut calls).unwrap();
        assert_eq!(fields[13], "->");
        let csv = |v: &str| {
            v.split(',')
                .map(|v| v.parse::<i32>().unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            s.nodes
                .iter()
                .map(|n| n.owner.map_or(-1, |i| i as i32))
                .collect::<Vec<_>>(),
            csv(fields[14]),
            "{line}"
        );
        assert_eq!(
            s.nodes
                .iter()
                .map(|n| n.selected.map_or(-1, |i| i as i32))
                .collect::<Vec<_>>(),
            csv(fields[15]),
            "{line}"
        );
        assert_eq!(
            s.focused.map_or(-1, |i| i as i32),
            fields[16].parse::<i32>().unwrap(),
            "{line}"
        );
        assert_eq!(calls.trace, &fields[17..], "{line}");
        count += 1;
    }
    assert_eq!(count, 1045);
}
