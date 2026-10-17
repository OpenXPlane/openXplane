//! Shared local replay support for independent and factory-composed root calls.
#![allow(dead_code)]
use openxplane::{
    ui_callback_links::{self, Kind, Node},
    ui_callback_list::{self, List},
    ui_payload_move,
    ui_root_refresh::{self, Context, Services, State},
};
use std::collections::{BTreeMap, BTreeSet};
pub(crate) fn num(s: &str) -> u64 {
    s.parse().unwrap()
}
pub(crate) fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
pub(crate) fn text(b: &[u8]) -> String {
    b.iter().map(|v| format!("{v:02x}")).collect()
}
pub(crate) fn parse_graph(s: &str) -> ui_callback_links::State {
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
pub(crate) fn graph(s: &ui_callback_links::State) -> String {
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
pub(crate) fn list(l: &List) -> String {
    l.links()
        .into_iter()
        .map(|(p, n, b)| format!("{p}:{n}:{b}"))
        .collect::<Vec<_>>()
        .join(";")
}

pub(crate) fn parse_memory(s: &str) -> BTreeMap<u64, Vec<u8>> {
    s.split('/')
        .map(|r| {
            let (p, b) = r.split_once(':').unwrap();
            (num(p), hex(b))
        })
        .collect()
}
pub(crate) fn parse_snapshot(s: &str, identity: u64, stack: u64) -> State {
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
pub(crate) fn snapshot(s: &State) -> String {
    format!(
        "{};{};{}",
        s.memory
            .blocks
            .iter()
            .filter(|(p, b)| !(**p <= s.stack && s.stack < **p + b.len() as u64))
            .map(|(p, b)| format!("{p}:{}", text(b)))
            .collect::<Vec<_>>()
            .join("/"),
        s.active_global,
        s.resource_index
    )
}
pub(crate) fn compare(actual: &str, expected: &str, where_: &str) {
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
pub(crate) struct Replay {
    pub(crate) events: Vec<String>,
    pub(crate) position: usize,
    pub(crate) native_nested: bool,
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
        if self.native_nested && function == 0x14113a300 {
            assert_eq!(args, [s.identity + 0x268]);
            openxplane::ui_refresh_chain::refresh_nested(s, self, s.stack - 0x100)?;
            return Ok(0);
        }
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
        let scratch = if self.native_nested {
            s.stack - 0x200
        } else {
            s.stack
        };
        let stack = s.memory.blocks.remove(&scratch).unwrap();
        *s = parse_snapshot(&e[3], s.identity, s.stack);
        s.memory.blocks.insert(scratch, stack);
        if e[6] != "-" {
            for r in e[6].split('/') {
                let (p, b) = r.split_once(':').unwrap();
                s.memory.write(num(p), &hex(b)).unwrap();
            }
        }
        Ok(num(&e[4]))
    }
}
/// Whole fresh constructor, including both nested refresh invocations and the
/// final root refresh. Callback tree allocation bytes remain deliberately opaque.
struct ConstructorHost {
    runtime: State,
    replay: Replay,
}
impl ConstructorHost {
    fn publish(&mut self, image: &[u8], blocks: &[openxplane::ui_input_construct::Block]) {
        self.runtime
            .memory
            .blocks
            .insert(self.runtime.identity, image.to_vec());
        for block in blocks.iter().filter(|b| b.bytes.len() != 40) {
            self.runtime
                .memory
                .blocks
                .insert(block.address, block.bytes.clone());
        }
    }
    fn capture(&mut self, image: &mut [u8], blocks: &mut [openxplane::ui_input_construct::Block]) {
        image.copy_from_slice(
            &self
                .runtime
                .memory
                .read(self.runtime.identity, image.len())
                .unwrap(),
        );
        for block in blocks.iter_mut().filter(|b| b.bytes.len() != 40) {
            block.bytes = self
                .runtime
                .memory
                .read(block.address, block.bytes.len())
                .unwrap();
        }
    }
}
impl openxplane::ui_dialog_child_root::Services for ConstructorHost {
    fn allocate(
        &mut self,
        image: &[u8],
        blocks: &[openxplane::ui_input_construct::Block],
        bytes: usize,
    ) -> Result<openxplane::ui_input_construct::Block, &'static str> {
        self.publish(image, blocks);
        let e = self.replay.event();
        assert_eq!(e[0], "C");
        let (n, p) = e[1].split_once(',').unwrap();
        assert_eq!(num(n) as usize, bytes);
        compare(
            &snapshot(&self.runtime),
            &e[2],
            &format!("constructor allocation {}", self.replay.position),
        );
        let scratch = self.runtime.stack - 0x200;
        let stack = self.runtime.memory.blocks.remove(&scratch).unwrap();
        self.runtime = parse_snapshot(&e[3], self.runtime.identity, self.runtime.stack);
        self.runtime.memory.blocks.insert(scratch, stack);
        Ok(openxplane::ui_input_construct::Block {
            address: num(p),
            bytes: hex(&e[4]),
        })
    }
    fn refresh_nested(
        &mut self,
        image: &mut [u8],
        blocks: &mut Vec<openxplane::ui_input_construct::Block>,
        identity: u64,
    ) -> Result<(), &'static str> {
        assert_eq!(identity, self.runtime.identity + 0x268);
        self.publish(image, blocks);
        let temporary = self.runtime.stack + 0x1e0;
        openxplane::ui_refresh_chain::refresh_nested(
            &mut self.runtime,
            &mut self.replay,
            temporary,
        )?;
        self.capture(image, blocks);
        Ok(())
    }
    fn refresh_root(
        &mut self,
        _: &mut [u8],
        _: &mut Vec<openxplane::ui_input_construct::Block>,
        _: u64,
    ) -> Result<(), &'static str> {
        Err("final refresh requires freshly constructed callback context")
    }
    fn refresh_root_with_context(
        &mut self,
        image: &mut [u8],
        blocks: &mut Vec<openxplane::ui_input_construct::Block>,
        identity: u64,
        callbacks: &mut ui_callback_links::State,
        parents: &mut BTreeMap<u64, List>,
    ) -> Result<(), &'static str> {
        assert_eq!(identity, self.runtime.identity);
        self.publish(image, blocks);
        ui_root_refresh::refresh_image(
            image,
            &mut self.runtime,
            Context { callbacks, parents },
            &mut self.replay,
        )?;
        self.capture(image, blocks);
        Ok(())
    }
}
pub(crate) fn run_constructor_row(
    row: &str,
    callbacks: &mut ui_callback_links::State,
    parents: &mut BTreeMap<u64, List>,
) -> (openxplane::ui_dialog_child_root::State, State) {
    let v: Vec<_> = row.split_whitespace().collect();
    assert_eq!(v.len(), 14);
    assert_eq!(&v[6..9], ["-", "-", "->"]);
    let (id, stack) = (num(v[1]), num(v[2]));
    let mut runtime = parse_snapshot(v[3], id, stack);
    runtime.memory.blocks.insert(stack - 0x200, hex(v[4]));
    let mut state =
        openxplane::ui_dialog_child_root::State::new(runtime.memory.read(id, 0x3e0).unwrap());
    let mut host = ConstructorHost {
        runtime,
        replay: Replay {
            events: v[12].split('|').map(str::to_string).collect(),
            position: 0,
            native_nested: true,
        },
    };
    openxplane::ui_dialog_child_root::construct(
        &mut state,
        num(v[13]),
        openxplane::ui_dialog_child_root::Context {
            identity: id,
            callbacks,
            parents,
        },
        &mut host,
    )
    .unwrap();
    compare(&snapshot(&host.runtime), v[9], "final whole constructor");
    assert_eq!(state.image, host.runtime.memory.read(id, 0x3e0).unwrap());
    assert_eq!(graph(&callbacks), v[10]);
    assert_eq!(list(&parents[&(id + 0x1e0)]), v[11]);
    assert_eq!(parents[&(id + 0x1e0)].sentinel, num(v[5]));
    assert_eq!(parents.len(), 3);
    assert_eq!(host.replay.position, host.replay.events.len());
    (state, host.runtime)
}
