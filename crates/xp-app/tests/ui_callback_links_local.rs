use openxplane::ui_callback_links::{Kind, Services, State};
#[derive(Default)]
struct Audit(Vec<&'static str>);
impl Services for Audit {
    fn allocate(&mut self, bytes: usize) {
        assert_eq!(bytes, 40);
        self.0.push("A");
    }
    fn free(&mut self, bytes: usize) {
        assert_eq!(bytes, 40);
        self.0.push("F");
    }
}
fn snapshot(s: &State) -> String {
    let out = s
        .nodes
        .iter()
        .map(|(id, node)| {
            let (kind, first, second) = match node.kind {
                Kind::Container => ("B", 0, 0),
                Kind::Wrapper { first, second } => ("W", first, second),
            };
            let peers = if node.peers.is_empty() {
                "-".to_string()
            } else {
                node.peers
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            };
            format!("{id}:{kind}:{first}:{second}:{peers}")
        })
        .collect::<Vec<_>>();
    if out.is_empty() {
        "-".to_string()
    } else {
        out.join(";")
    }
}
#[test]
#[ignore = "requires local native corpus via OPENXPLANE_UI_CALLBACK_LINKS_VECTORS"]
fn native_callback_wrapper_copy_and_connection_lifecycle() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_CALLBACK_LINKS_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut state = State::default();
    let mut audit = Audit::default();
    let mut ids = vec![];
    let mut cases = 0;
    let mut steps = 0;
    for line in data.lines().filter(|l| !l.starts_with('#')) {
        let v: Vec<_> = line.split_whitespace().collect();
        if v[0] == "C" {
            assert!(state.nodes.is_empty());
            state = State::default();
            ids = v[2].split(',').map(|s| s.parse::<u64>().unwrap()).collect();
            cases += 1;
            continue;
        }
        assert_eq!(v[0], "T");
        assert_eq!(v[2], "->");
        audit.0.clear();
        let op: Vec<_> = v[1].split(':').collect();
        let i = op[1].parse::<usize>().unwrap();
        let id = ids[i];
        match op[0] {
            "B" => state.container(id, &mut audit).unwrap(),
            "C" => {
                let parent = op[2].parse::<i32>().unwrap();
                state
                    .construct(
                        id,
                        if parent < 0 {
                            None
                        } else {
                            Some(ids[parent as usize])
                        },
                        op[3].parse().unwrap(),
                        op[4].parse().unwrap(),
                        &mut audit,
                    )
                    .unwrap();
            }
            "P" => state
                .copy(id, ids[op[2].parse::<usize>().unwrap()], &mut audit)
                .unwrap(),
            "L" => state
                .bind(id, ids[op[2].parse::<usize>().unwrap()], &mut audit)
                .unwrap(),
            "D" => state.destroy(id, &mut audit).unwrap(),
            other => panic!("unknown operation {other}"),
        }
        assert_eq!(
            snapshot(&state),
            v[3],
            "case {cases}, step {steps}, {}",
            v[1]
        );
        assert_eq!(
            if audit.0.is_empty() {
                "-".to_string()
            } else {
                audit.0.join(",")
            },
            v[4],
            "case {cases}, step {steps}"
        );
        steps += 1;
    }
    assert!(state.nodes.is_empty());
    assert_eq!((cases, steps), (324, 5832));
}
