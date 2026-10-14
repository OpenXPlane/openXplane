use openxplane::{
    xplm_deferred::{Allocation, Kind, Node, Rejection, Services as DeferredServices},
    xplm_sniffer::{self, Binding, Record, Services, State},
};
struct Replay {
    time: u32,
    trace: Vec<String>,
}
impl DeferredServices for Replay {
    fn diagnostic(
        &mut self,
        s: &mut openxplane::xplm_deferred::State,
        reason: Rejection,
        node: u64,
    ) {
        let reason = match reason {
            Rejection::MissingBefore => 0,
            Rejection::OwnerBefore => 1,
            Rejection::MissingAfter => 2,
            Rejection::OwnerAfter => 3,
        };
        self.trace.push(format!("D:{reason}:{node}:{}", s.deferred));
    }
    fn notify(&mut self, s: &mut openxplane::xplm_deferred::State) {
        self.trace.push(format!("L:{}", s.deferred));
    }
    fn bad_free(&mut self, s: &mut openxplane::xplm_deferred::State, node: u64) {
        self.trace.push(format!("B:{node}:{}", s.deferred));
    }
    fn elapsed(&mut self, s: &mut openxplane::xplm_deferred::State) -> f32 {
        self.trace.push(format!("C:{}", s.deferred));
        f32::from_bits(self.time)
    }
    fn tracking_entry(&mut self, s: &mut openxplane::xplm_deferred::State, kind: Kind, node: u64) {
        assert_eq!(kind, Kind::Sniffer);
        self.trace.push(format!("T:{node}:{}", s.deferred));
    }
    fn free(
        &mut self,
        s: &mut openxplane::xplm_deferred::State,
        allocation: Allocation,
        bytes: u64,
    ) {
        let Allocation::Node(node) = allocation else {
            panic!("unexpected sniffer text free")
        };
        assert_eq!(bytes, 64);
        self.trace
            .push(format!("F:N:{node}:{bytes}:{}", s.deferred));
    }
}
impl Services for Replay {
    fn validate_thread(&mut self, _: &mut openxplane::xplm_deferred::State) {
        self.trace.push("V".into());
    }
    fn ensure_loaded(&mut self, _: &mut openxplane::xplm_deferred::State, plugin: i32) {
        self.trace.push(format!("Z:{}", plugin as u32));
    }
    fn allocate(&mut self, s: &mut openxplane::xplm_deferred::State, bytes: u64) -> u64 {
        let token = s.nodes.len() as u64 + 1;
        self.trace.push(format!("A:{token}:{bytes}:{}", s.deferred));
        token
    }
}
#[test]
#[ignore = "requires native Windows SDK sniffer lifecycle replay; set OPENXPLANE_XPLM_SNIFFER_VECTORS"]
fn native_sniffer_registration_unregistration_and_end_call_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_XPLM_SNIFFER_VECTORS")
            .expect("set OPENXPLANE_XPLM_SNIFFER_VECTORS"),
    )
    .unwrap();
    assert!(
        text.lines().any(|l| l
            == "# dll_sha256 706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f")
    );
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("S ")) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let mut input = a.split_whitespace().skip(1);
        let mut next = || input.next().unwrap().parse::<i128>().unwrap();
        let mut state = State::default();
        let stack_n = next() as usize;
        state.list.context.stack = (0..stack_n).map(|_| next() as i32).collect();
        let initial_stack = state.list.context.stack.clone();
        state.iteration.depth = next() as i32;
        state.iteration.pending = next() as u8;
        let mut replay = Replay {
            time: next() as u32,
            trace: Vec::new(),
        };
        let n = next() as u64;
        for token in 1..=n {
            let owner = next() as i32;
            let deleted = next() as u8;
            let callback = next() as u64;
            let plugin = next() as i32;
            let before = next() as i32;
            let refcon = next() as u64;
            state.list.nodes.insert(
                token,
                Node {
                    next: if token == n { 0 } else { token + 1 },
                    previous: token - 1,
                    deferred_next: 0,
                    owner,
                    deleted,
                    text: None,
                    tracking: None,
                    freed: false,
                },
            );
            state.records.insert(
                token,
                Record {
                    binding: Binding {
                        callback,
                        before,
                        refcon,
                    },
                    plugin,
                },
            );
        }
        state.list.head = if n == 0 { 0 } else { 1 };
        let queue_n = next() as usize;
        let queue: Vec<u64> = (0..queue_n).map(|_| next() as u64).collect();
        state.list.deferred = queue.first().copied().unwrap_or(0);
        for (i, token) in queue.iter().enumerate() {
            state.list.nodes.get_mut(token).unwrap().deferred_next =
                queue.get(i + 1).copied().unwrap_or(0);
        }
        let op_n = next() as usize;
        let operations: Vec<_> = (0..op_n)
            .map(|_| {
                (
                    next() as u8,
                    Binding {
                        callback: next() as u64,
                        before: next() as i32,
                        refcon: next() as u64,
                    },
                )
            })
            .collect();
        assert!(input.next().is_none());
        let results: Vec<_> = operations
            .into_iter()
            .map(|(op, binding)| match op {
                0 => xplm_sniffer::register(&mut state, binding, &mut replay).unwrap() as u64,
                1 => xplm_sniffer::unregister(&mut state, binding, &mut replay).unwrap() as u64,
                2 => {
                    state.end_iteration(&mut replay).unwrap();
                    0
                }
                _ => panic!("unknown operation"),
            })
            .collect();
        assert_eq!(
            state.list.context.stack, initial_stack,
            "stack case {count}"
        );
        let mut output = b.split_whitespace();
        let mut next = || output.next().unwrap().parse::<u64>().unwrap();
        let expected_results: Vec<_> = (0..op_n).map(|_| next()).collect();
        assert_eq!(results, expected_results, "results case {count}");
        assert_eq!(
            [
                state.list.head,
                state.list.deferred,
                state.iteration.depth as u32 as u64,
                state.iteration.pending as u64,
                state.list.nodes.len() as u64
            ],
            [next(), next(), next(), next(), next()],
            "list case {count}"
        );
        for (token, node) in &state.list.nodes {
            let record = state.records[token];
            let actual = [
                node.next,
                node.previous,
                node.deferred_next,
                node.owner as u32 as u64,
                node.deleted as u64,
                record.binding.callback,
                record.plugin as u32 as u64,
                record.binding.before as u32 as u64,
                record.binding.refcon,
                u64::from(node.freed),
                u64::from(node.tracking.is_some()),
            ];
            let expected: Vec<_> = (0..11).map(|_| next()).collect();
            assert_eq!(actual.as_slice(), expected, "node {token} case {count}");
            let expected = [next(), next(), next()];
            let actual = node
                .tracking
                .map(|t| {
                    [
                        t.plugin as u32 as u64,
                        t.flag as u64,
                        t.elapsed.to_bits() as u64,
                    ]
                })
                .unwrap_or([0, 0, 0]);
            assert_eq!(actual, expected, "tracking {token} case {count}");
        }
        assert_eq!(
            replay.trace,
            output.map(str::to_string).collect::<Vec<_>>(),
            "trace case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 1126);
}
#[test]
fn duplicate_keeps_new_allocation_and_deleted_entry_blocks_reregistration_until_end() {
    let mut state = State::default();
    state.list.context.stack = vec![7, 0];
    state.iteration.depth = 1;
    let mut replay = Replay {
        time: 0x80000000,
        trace: Vec::new(),
    };
    let binding = Binding {
        callback: 42,
        before: -1,
        refcon: 99,
    };
    assert_eq!(
        xplm_sniffer::register(&mut state, binding, &mut replay).unwrap(),
        1
    );
    assert_eq!(
        xplm_sniffer::unregister(&mut state, binding, &mut replay).unwrap(),
        1
    );
    assert_eq!(
        (
            state.list.head,
            state.list.deferred,
            state.iteration.depth,
            state.iteration.pending
        ),
        (1, 1, 1, 1)
    );
    assert_eq!(
        xplm_sniffer::register(&mut state, binding, &mut replay).unwrap(),
        0
    );
    assert!(!state.list.nodes[&2].freed);
    assert_eq!(state.list.nodes[&2].owner, -1);
    state.end_iteration(&mut replay).unwrap();
    assert_eq!(
        (
            state.list.head,
            state.list.deferred,
            state.iteration.pending
        ),
        (0, 0, 0)
    );
    assert!(state.list.nodes[&1].freed);
    assert_eq!(
        xplm_sniffer::register(&mut state, binding, &mut replay).unwrap(),
        1
    );
}
