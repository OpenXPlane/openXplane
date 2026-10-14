use openxplane::xplm_deferred::{self, Allocation, Kind, Node, Rejection, Services, State, Text};
use std::collections::BTreeMap;
struct Replay {
    time: u32,
    trace: Vec<String>,
}
impl Services for Replay {
    fn diagnostic(&mut self, s: &mut State, reason: Rejection, node: u64) {
        let reason = match reason {
            Rejection::MissingBefore => 0,
            Rejection::OwnerBefore => 1,
            Rejection::MissingAfter => 2,
            Rejection::OwnerAfter => 3,
        };
        self.trace.push(format!("D:{reason}:{node}:{}", s.deferred));
    }
    fn notify(&mut self, s: &mut State) {
        self.trace.push(format!("L:{}", s.deferred));
    }
    fn bad_free(&mut self, s: &mut State, node: u64) {
        self.trace.push(format!("B:{node}:{}", s.deferred));
    }
    fn elapsed(&mut self, s: &mut State) -> f32 {
        self.trace.push(format!("C:{}", s.deferred));
        f32::from_bits(self.time)
    }
    fn tracking_entry(&mut self, s: &mut State, _: Kind, node: u64) {
        self.trace.push(format!("T:{node}:{}", s.deferred));
    }
    fn free(&mut self, s: &mut State, allocation: Allocation, bytes: u64) {
        match allocation {
            Allocation::Node(node) => {
                if let Some(text) = s.nodes[&node].text.as_ref() {
                    assert_eq!((text.length, text.capacity, text.first), (0, 15, 0));
                }
                self.trace
                    .push(format!("F:N:{node}:{bytes}:{}", s.deferred));
            }
            Allocation::Text { node, pointer } => {
                let text = s.nodes[&node].text.as_ref().unwrap();
                let base = u8::from(pointer != text.data);
                assert_eq!(
                    pointer,
                    if base == 0 {
                        text.data
                    } else {
                        text.allocation_base.unwrap()
                    }
                );
                self.trace
                    .push(format!("F:S:{node}:{base}:{bytes}:{}", s.deferred));
            }
        }
    }
}
#[test]
#[ignore = "requires native Windows SDK deletion replay; set OPENXPLANE_XPLM_DEFERRED_VECTORS"]
fn native_deferred_deletion_topology_ownership_strings_and_requests_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_XPLM_DEFERRED_VECTORS")
            .expect("set OPENXPLANE_XPLM_DEFERRED_VECTORS"),
    )
    .unwrap();
    assert!(
        text.lines().any(|l| l
            == "# dll_sha256 706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f")
    );
    let mut count = 0;
    let mut kinds = [0, 0];
    for line in text.lines().filter(|l| l.starts_with("D ")) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let mut input = a.split_whitespace().skip(1);
        let mut next = || input.next().unwrap().parse::<i128>().unwrap();
        let kind_index = next() as usize;
        let kind = [Kind::Sniffer, Kind::Hotkey][kind_index];
        kinds[kind_index] += 1;
        let stack_n = next() as usize;
        let stack: Vec<i32> = (0..stack_n).map(|_| next() as i32).collect();
        let mut replay = Replay {
            time: next() as u32,
            trace: Vec::new(),
        };
        let n = next() as u64;
        let mut nodes = BTreeMap::new();
        for token in 1..=n {
            let owner = next() as i32;
            let capacity = next() as u64;
            let length = next() as u64;
            let first = next() as u8;
            let alignment = next() as u64;
            let data = token * 0x1000 + 0x100;
            nodes.insert(
                token,
                Node {
                    next: 0,
                    previous: 0,
                    deferred_next: 0,
                    owner,
                    text: (kind == Kind::Hotkey).then_some(Text {
                        length,
                        capacity,
                        first,
                        data,
                        allocation_base: Some(data - alignment),
                    }),
                    tracking: None,
                    freed: false,
                },
            );
        }
        let live_n = next() as usize;
        let live: Vec<u64> = (0..live_n).map(|_| next() as u64).collect();
        let queue_n = next() as usize;
        let queue: Vec<u64> = (0..queue_n).map(|_| next() as u64).collect();
        assert!(input.next().is_none());
        for (i, token) in live.iter().enumerate() {
            nodes.get_mut(token).unwrap().next = live.get(i + 1).copied().unwrap_or(0);
            nodes.get_mut(token).unwrap().previous = i
                .checked_sub(1)
                .and_then(|i| live.get(i))
                .copied()
                .unwrap_or(0);
        }
        for (i, token) in queue.iter().enumerate() {
            nodes.get_mut(token).unwrap().deferred_next = queue.get(i + 1).copied().unwrap_or(0);
        }
        let mut state = State {
            head: live.first().copied().unwrap_or(0),
            deferred: queue.first().copied().unwrap_or(0),
            nodes,
            context: openxplane::xplm_context::State {
                stack: stack.clone(),
                ..Default::default()
            },
        };
        xplm_deferred::drain(kind, &mut state, &mut replay).unwrap();
        assert_eq!(state.context.stack, stack, "context case {count}");
        let mut output = b.split_whitespace();
        let mut next = || output.next().unwrap().parse::<u64>().unwrap();
        assert_eq!(state.head, next(), "head case {count}");
        assert_eq!(state.deferred, next(), "queue case {count}");
        for node in state.nodes.values() {
            assert_eq!(
                [node.next, node.previous, node.deferred_next],
                [next(), next(), next()],
                "links case {count}"
            );
            let expected_text = [next(), next(), next()];
            let actual_text = node
                .text
                .as_ref()
                .map(|t| [t.length, t.capacity, u64::from(t.first)])
                .unwrap_or([0, 0, 0]);
            assert_eq!(actual_text, expected_text, "text case {count}");
            assert_eq!(node.freed, next() != 0, "free case {count}");
            let tracked = next() != 0;
            let plugin = next() as u32;
            let flag = next() as u8;
            let time = next() as u32;
            assert_eq!(node.tracking.is_some(), tracked, "tracking case {count}");
            if let Some(t) = node.tracking {
                assert_eq!(
                    (t.plugin as u32, t.flag, t.elapsed.to_bits()),
                    (plugin, flag, time),
                    "tracking fields case {count}"
                );
            }
        }
        assert_eq!(
            replay.trace,
            output.map(str::to_string).collect::<Vec<_>>(),
            "trace case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 1704);
    assert_eq!(kinds, [852, 852]);
}
#[test]
fn rejected_owner_pops_queue_without_unlinking_and_allowed_delete_relinks() {
    let make = |owner, next, previous, deferred_next| Node {
        owner,
        next,
        previous,
        deferred_next,
        text: None,
        tracking: None,
        freed: false,
    };
    let mut state = State {
        head: 1,
        deferred: 1,
        nodes: BTreeMap::from([(1, make(8, 2, 0, 2)), (2, make(7, 0, 1, 0))]),
        context: openxplane::xplm_context::State {
            stack: vec![7, 0],
            ..Default::default()
        },
    };
    let mut replay = Replay {
        time: 0x80000000,
        trace: Vec::new(),
    };
    xplm_deferred::drain(Kind::Sniffer, &mut state, &mut replay).unwrap();
    assert_eq!(
        (state.head, state.deferred, state.nodes[&1].next),
        (1, 0, 0)
    );
    assert!(!state.nodes[&1].freed && state.nodes[&2].freed);
    assert_eq!(
        replay.trace,
        ["D:1:1:2", "L:2", "B:1:2", "C:0", "T:2:0", "F:N:2:64:0"]
    );
}
