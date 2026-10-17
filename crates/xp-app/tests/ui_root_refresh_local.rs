use openxplane::{
    ui_callback_links::{self, Kind, Node},
    ui_callback_list::{self, List},
    ui_payload_move,
    ui_root_refresh::{self, Context, Services, State},
};
use std::collections::{BTreeMap, BTreeSet};
fn num(s: &str) -> u64 {
    s.parse().unwrap()
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn text(b: &[u8]) -> String {
    b.iter().map(|v| format!("{v:02x}")).collect()
}
fn parse_graph(s: &str) -> ui_callback_links::State {
    let mut state = ui_callback_links::State::default();
    if s != "-" {
        for r in s.split(';') {
            let f: Vec<_> = r.split(':').collect();
            state.nodes.insert(
                num(f[0]),
                Node {
                    kind: if f[1] == "B" {
                        Kind::Container
                    } else {
                        Kind::Wrapper {
                            first: num(f[2]),
                            second: num(f[3]),
                        }
                    },
                    peers: if f[4] == "-" {
                        BTreeSet::new()
                    } else {
                        f[4].split(',').map(num).collect()
                    },
                },
            );
        }
    }
    state
}
fn graph(s: &ui_callback_links::State) -> String {
    s.nodes
        .iter()
        .map(|(p, n)| {
            let (k, a, b) = match n.kind {
                Kind::Container => ("B", 0, 0),
                Kind::Wrapper { first, second } => ("W", first, second),
            };
            let peers = n.peers.iter().map(u64::to_string).collect::<Vec<_>>();
            format!(
                "{p}:{k}:{a}:{b}:{}",
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
fn list(l: &List) -> String {
    l.links()
        .into_iter()
        .map(|(p, n, b)| format!("{p}:{n}:{b}"))
        .collect::<Vec<_>>()
        .join(";")
}

fn parse_memory(s: &str) -> BTreeMap<u64, Vec<u8>> {
    s.split('/')
        .map(|r| {
            let (p, b) = r.split_once(':').unwrap();
            (num(p), hex(b))
        })
        .collect()
}
fn parse_snapshot(s: &str, identity: u64, stack: u64) -> State {
    let v: Vec<_> = s.split(';').collect();
    State {
        memory: ui_payload_move::State {
            blocks: parse_memory(v[0]),
        },
        identity,
        stack,
        active_global: num(v[1]),
        resource_index: num(v[2]) as u32,
    }
}
fn snapshot(s: &State) -> String {
    format!(
        "{};{};{}",
        s.memory
            .blocks
            .iter()
            .filter(|(p, _)| **p != s.stack)
            .map(|(p, b)| format!("{p}:{}", text(b)))
            .collect::<Vec<_>>()
            .join("/"),
        s.active_global,
        s.resource_index
    )
}
fn compare(actual: &str, expected: &str, where_: &str) {
    if actual != expected {
        let p = actual
            .bytes()
            .zip(expected.bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(actual.len().min(expected.len()));
        panic!(
            "{where_}: mismatch at byte {p}; actual/expected lengths {}/{}",
            actual.len(),
            expected.len()
        )
    }
}
struct Replay {
    events: Vec<String>,
    position: usize,
}
impl Replay {
    fn event(&mut self) -> Vec<String> {
        let e = self.events[self.position]
            .split('~')
            .map(str::to_string)
            .collect();
        self.position += 1;
        e
    }
}
impl ui_callback_links::Services for Replay {
    fn allocate(&mut self, n: usize) {
        assert_eq!(self.event(), vec!["A".to_string(), n.to_string()]);
    }
    fn free(&mut self, n: usize) {
        assert_eq!(self.event(), vec!["D".to_string(), n.to_string()]);
    }
}
impl ui_callback_list::Services for Replay {
    fn allocate_entry(&mut self, n: usize) -> Result<u64, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "L");
        let a: Vec<_> = e[1].split(',').collect();
        assert_eq!(num(a[0]) as usize, n);
        Ok(num(a[1]))
    }
}
impl Services for Replay {
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "I", "event {}", self.position);
        let v: Vec<_> = e[1].split(',').collect();
        assert_eq!(num(v[0]), function, "event {}", self.position);
        let expected = if v[1] == "-" {
            vec![]
        } else {
            v[1..].iter().map(|x| num(x)).collect::<Vec<_>>()
        };
        assert_eq!(args, expected);
        compare(
            &snapshot(s),
            &e[2],
            &format!("before {function:x} event{}", self.position),
        );
        if e[5] != "-" {
            for (p, b) in parse_memory(&e[5]) {
                assert_eq!(
                    s.memory.read(p, b.len()).unwrap(),
                    b,
                    "read {function:x} at {p:x}"
                );
            }
        }
        let stack = s.memory.blocks.remove(&s.stack).unwrap();
        *s = parse_snapshot(&e[3], s.identity, s.stack);
        s.memory.blocks.insert(s.stack, stack);
        if e[6] != "-" {
            for r in e[6].split('/') {
                let (p, b) = r.split_once(':').unwrap();
                s.memory.write(num(p), &hex(b)).unwrap();
            }
        }
        Ok(num(&e[4]))
    }
}
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_ROOT_REFRESH_VECTORS"]
fn whole_native_root_refresh_resource_cli_fallbacks_vectors_and_retained_callbacks() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_ROOT_REFRESH_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        let v: Vec<_> = row.split_whitespace().collect();
        assert_eq!(v.len(), 13);
        assert_eq!(v[8], "->");
        let id = num(v[1]);
        let stack = num(v[2]);
        let mut state = parse_snapshot(v[3], id, stack);
        state.memory.blocks.insert(stack, hex(v[4]));
        let mut callbacks = parse_graph(v[7]);
        let mut parents = BTreeMap::from([(
            id + 0x1e0,
            List {
                sentinel: num(v[5]),
                entries: if v[6] == "-" {
                    vec![]
                } else {
                    v[6].split(',').map(num).collect()
                },
            },
        )]);
        let mut replay = Replay {
            events: v[12].split('|').map(str::to_string).collect(),
            position: 0,
        };
        let mut image = state.memory.read(id, 0x3e0).unwrap();
        ui_root_refresh::refresh_image(
            &mut image,
            &mut state,
            Context {
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut replay,
        )
        .unwrap();
        assert_eq!(image, state.memory.read(id, 0x3e0).unwrap());
        compare(&snapshot(&state), v[9], &format!("final case{cases}"));
        assert_eq!(graph(&callbacks), v[10]);
        assert_eq!(list(&parents[&(id + 0x1e0)]), v[11]);
        assert_eq!(replay.position, replay.events.len());
        cases += 1;
    }
    assert_eq!(cases, 118);
}
