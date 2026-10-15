use openxplane::{
    ui_callback_links::{self, Kind, State},
    ui_callback_list::{self, List},
};
fn num(s: &str) -> u64 {
    s.parse().unwrap()
}
#[derive(Default)]
struct Audit {
    next: u64,
    events: Vec<String>,
}
impl ui_callback_links::Services for Audit {
    fn allocate(&mut self, bytes: usize) {
        assert_eq!(bytes, 40);
        self.events.push("A:40".to_string());
    }
    fn free(&mut self, bytes: usize) {
        assert_eq!(bytes, 40);
        self.events.push("F:40".to_string());
    }
}
impl ui_callback_list::Services for Audit {
    fn allocate_entry(&mut self, bytes: usize) -> Result<u64, &'static str> {
        assert_eq!(bytes, 120);
        self.events.push(format!("A:120:{}", self.next));
        Ok(self.next)
    }
}
fn graph(s: &State) -> String {
    s.nodes
        .iter()
        .map(|(p, n)| {
            let (kind, a, b) = match n.kind {
                Kind::Container => ("B", 0, 0),
                Kind::Wrapper { first, second } => ("W", first, second),
            };
            let peers = n.peers.iter().map(u64::to_string).collect::<Vec<_>>();
            format!(
                "{p}:{kind}:{a}:{b}:{}",
                if peers.is_empty() {
                    "-".to_string()
                } else {
                    peers.join(",")
                }
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}
#[test]
#[ignore = "requires native local corpus via OPENXPLANE_UI_CALLBACK_LIST_VECTORS"]
fn native_list_append_copies_parameters_and_peer_registrations() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_CALLBACK_LIST_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut state = State::default();
    let mut audit = Audit::default();
    let mut list = List::new(1).unwrap();
    let mut sources = vec![];
    let mut cases = 0;
    let mut steps = 0;
    for line in data.lines().filter(|l| !l.starts_with('#')) {
        let v: Vec<_> = line.split_whitespace().collect();
        if v[0] == "C" {
            state = State::default();
            audit = Audit::default();
            let profile = num(v[2]);
            let containers: Vec<_> = v[3].split(',').map(num).collect();
            sources = v[4].split(',').map(num).collect();
            list = List::new(num(v[5])).unwrap();
            for p in &containers {
                state.container(*p, &mut audit).unwrap();
            }
            for (i, p) in sources.iter().enumerate() {
                let parent = if profile == 0 || (profile == 2 && i == 2) {
                    None
                } else {
                    Some(containers[usize::from(profile == 2 && i == 1)])
                };
                state
                    .construct(*p, parent, 3 + i as u64, u64::MAX - i as u64, &mut audit)
                    .unwrap();
                if profile == 2 && i == 0 {
                    state.bind(*p, containers[1], &mut audit).unwrap();
                }
            }
            cases += 1;
            continue;
        }
        if v[0] == "D" {
            assert_eq!(v[2], "->");
            audit.events.clear();
            state.destroy(num(v[1]), &mut audit).unwrap();
            let links = list
                .links()
                .into_iter()
                .map(|(p, next, prev)| format!("{p}:{next}:{prev}"))
                .collect::<Vec<_>>()
                .join(";");
            assert_eq!(links, v[3]);
            assert_eq!(graph(&state), v[4]);
            assert_eq!(audit.events.join("|"), v[5]);
            steps += 1;
            continue;
        }
        assert_eq!(v[0], "T");
        assert_eq!(v[3], "->");
        audit.events.clear();
        audit.next = num(v[2]);
        let entry = list
            .append(sources[num(v[1]) as usize], &mut state, &mut audit)
            .unwrap();
        assert_eq!(entry, num(v[2]));
        let links = list
            .links()
            .into_iter()
            .map(|(p, next, prev)| format!("{p}:{next}:{prev}"))
            .collect::<Vec<_>>()
            .join(";");
        assert_eq!(links, v[4], "case {cases}, step {steps}");
        assert_eq!(graph(&state), v[5], "case {cases}, step {steps}");
        assert_eq!(audit.events.join("|"), v[6]);
        steps += 1;
    }
    assert_eq!((cases, steps), (72, 936));
}
