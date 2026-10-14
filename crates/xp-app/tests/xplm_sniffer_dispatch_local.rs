use openxplane::{
    keyboard_dispatch::InterceptInput,
    xplm_context,
    xplm_deferred::{Allocation, Kind, Rejection, Services as DeferredServices},
    xplm_sniffer::{self, Binding, DispatchServices, Record, Services, State},
};
#[derive(Clone, Copy)]
struct Spec {
    plugin: i32,
    before: i32,
    action: u8,
    result: i32,
}
struct Replay {
    time: u32,
    specs: Vec<Spec>,
    new_before: i32,
    trace: Vec<String>,
}
fn top(s: &openxplane::xplm_deferred::State) -> i32 {
    *s.context.stack.last().unwrap()
}
impl DeferredServices for Replay {
    fn diagnostic(
        &mut self,
        s: &mut openxplane::xplm_deferred::State,
        reason: Rejection,
        token: u64,
    ) {
        let reason = match reason {
            Rejection::MissingBefore => 0,
            Rejection::OwnerBefore => 1,
            Rejection::MissingAfter => 2,
            Rejection::OwnerAfter => 3,
        };
        self.trace
            .push(format!("D:{reason}:{token}:{}:{}", s.deferred, top(s)));
    }
    fn notify(&mut self, s: &mut openxplane::xplm_deferred::State) {
        self.trace.push(format!("L:{}:{}", s.deferred, top(s)));
    }
    fn bad_free(&mut self, s: &mut openxplane::xplm_deferred::State, token: u64) {
        self.trace
            .push(format!("X:{token}:{}:{}", s.deferred, top(s)));
    }
    fn elapsed(&mut self, s: &mut openxplane::xplm_deferred::State) -> f32 {
        self.trace.push(format!("C:{}:{}", s.deferred, top(s)));
        f32::from_bits(self.time)
    }
    fn tracking_entry(&mut self, s: &mut openxplane::xplm_deferred::State, kind: Kind, token: u64) {
        assert_eq!(kind, Kind::Sniffer);
        self.trace
            .push(format!("T:{token}:{}:{}", s.deferred, top(s)));
    }
    fn free(
        &mut self,
        s: &mut openxplane::xplm_deferred::State,
        allocation: Allocation,
        bytes: u64,
    ) {
        let Allocation::Node(token) = allocation else {
            panic!("unexpected text free")
        };
        assert_eq!(bytes, 64);
        self.trace
            .push(format!("F:N:{token}:64:{}:{}", s.deferred, top(s)));
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
        assert_eq!(bytes, 64);
        let token = s.nodes.len() as u64 + 1;
        self.trace
            .push(format!("A:{token}:64:{}:{}", s.deferred, top(s)));
        token
    }
}
impl xplm_context::Services for Replay {
    fn validate_enabled(&mut self, _: &mut xplm_context::State) {
        self.trace.push("B".into());
    }
    fn ensure_loaded(&mut self, _: &mut xplm_context::State, plugin: i32) {
        self.trace.push(format!("Z:{}", plugin as u32));
    }
    fn unload(&mut self, _: &mut xplm_context::State, _: i32) {
        panic!("unexpected unload")
    }
    fn notify_unload(&mut self, _: &mut xplm_context::State) {
        panic!("unexpected unload notification")
    }
}
impl DispatchServices for Replay {
    fn validate_key_caller(&mut self, _: &mut openxplane::xplm_deferred::State) {
        self.trace.push("Q".into());
    }
    fn callback(
        &mut self,
        state: &mut State,
        token: u64,
        record: Record,
        input: InterceptInput,
    ) -> i32 {
        let cb = record.binding.callback;
        self.trace.push(format!(
            "K:{token}:{cb}:{}:{}",
            input.phase,
            top(&state.list)
        ));
        if cb == 999 {
            return 1;
        }
        let spec = self.specs[cb as usize - 1];
        if spec.action == 1 || spec.action == 4 {
            xplm_sniffer::unregister(state, record.binding, self).unwrap();
        }
        if spec.action == 2 {
            let target = if cb > 1 {
                cb - 1
            } else {
                self.specs.len() as u64
            };
            let binding = Binding {
                callback: target,
                before: self.specs[target as usize - 1].before,
                refcon: 100 + target,
            };
            xplm_sniffer::unregister(state, binding, self).unwrap();
        }
        if spec.action == 3 {
            xplm_sniffer::register(
                state,
                Binding {
                    callback: 999,
                    before: self.new_before,
                    refcon: 1099,
                },
                self,
            )
            .unwrap();
        }
        if spec.action == 4 {
            xplm_sniffer::register(state, record.binding, self).unwrap();
        }
        spec.result
    }
}
fn register_initial(state: &mut State, replay: &mut Replay) {
    for (i, spec) in replay.specs.clone().into_iter().enumerate() {
        state.list.context.enter(spec.plugin);
        assert_eq!(
            xplm_sniffer::register(
                state,
                Binding {
                    callback: i as u64 + 1,
                    before: spec.before,
                    refcon: 101 + i as u64
                },
                replay
            )
            .unwrap(),
            1
        );
        state.list.context.leave(replay).unwrap();
    }
}
#[test]
#[ignore = "requires native mutable sniffer event replay; set OPENXPLANE_XPLM_SNIFFER_DISPATCH_VECTORS"]
fn native_registered_callbacks_mutate_lists_with_original_context_order() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_XPLM_SNIFFER_DISPATCH_VECTORS")
            .expect("set OPENXPLANE_XPLM_SNIFFER_DISPATCH_VECTORS"),
    )
    .unwrap();
    assert!(
        text.lines().any(|l| l
            == "# dll_sha256 706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f")
    );
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("P ")) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let mut values = a.split_whitespace().skip(1);
        let mut next = || values.next().unwrap().parse::<i128>().unwrap();
        let character = next() as u8;
        let virtual_key = next() as u8;
        let flags = next() as u32;
        let enabled = next() as u32;
        let depth = next() as i32;
        let new_before = next() as i32;
        let time = next() as u32;
        let n = next() as usize;
        let specs = (0..n)
            .map(|_| Spec {
                plugin: next() as i32,
                before: next() as i32,
                action: next() as u8,
                result: next() as i32,
            })
            .collect();
        let mut replay = Replay {
            time,
            specs,
            new_before,
            trace: Vec::new(),
        };
        let event_n = next() as usize;
        let phases: Vec<_> = (0..event_n).map(|_| next() as u32).collect();
        assert!(values.next().is_none());
        let mut state = State::default();
        state.list.context.stack = vec![0];
        state.list.context.enabled = (0..4).map(|i| u8::from(enabled & (1 << i) != 0)).collect();
        register_initial(&mut state, &mut replay);
        state.iteration.depth = depth;
        let results: Vec<_> = phases
            .into_iter()
            .map(|phase| {
                xplm_sniffer::dispatch(
                    &mut state,
                    InterceptInput {
                        character,
                        virtual_key,
                        flags,
                        phase,
                    },
                    &mut replay,
                )
                .unwrap() as u64
            })
            .collect();
        assert_eq!(state.list.context.stack, [0], "stack case {count}");
        let mut output = b.split_whitespace();
        let mut next = || output.next().unwrap().parse::<u64>().unwrap();
        let expected: Vec<_> = (0..event_n).map(|_| next()).collect();
        assert_eq!(results, expected, "results case {count}");
        assert_eq!(
            [
                state.list.head,
                state.list.deferred,
                state.iteration.depth as u32 as u64,
                state.iteration.pending as u64,
                state.list.nodes.len() as u64
            ],
            [next(), next(), next(), next(), next()],
            "state case {count}"
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
    assert_eq!(count, 524);
}
#[test]
fn after_phase_visits_next_deleted_callback_and_new_head_waits_for_next_event() {
    let mut state = State::default();
    state.list.context.stack = vec![0];
    let mut replay = Replay {
        time: 0,
        specs: vec![
            Spec {
                plugin: 0,
                before: 0,
                action: 0,
                result: 1,
            },
            Spec {
                plugin: 0,
                before: 0,
                action: 2,
                result: 1,
            },
            Spec {
                plugin: 0,
                before: 0,
                action: 3,
                result: 1,
            },
        ],
        new_before: 0,
        trace: Vec::new(),
    };
    register_initial(&mut state, &mut replay);
    replay.trace.clear();
    let input = InterceptInput {
        character: 27,
        virtual_key: 27,
        flags: 8,
        phase: 2,
    };
    assert_eq!(
        xplm_sniffer::dispatch(&mut state, input, &mut replay).unwrap(),
        1
    );
    let callbacks: Vec<_> = replay
        .trace
        .iter()
        .filter(|l| l.starts_with("K:"))
        .cloned()
        .collect();
    assert_eq!(callbacks, ["K:3:3:2:0", "K:2:2:2:0", "K:1:1:2:0"]);
    assert!(state.list.nodes[&1].freed);
    assert_eq!(state.list.head, 4);
    replay.trace.clear();
    xplm_sniffer::dispatch(&mut state, input, &mut replay).unwrap();
    assert_eq!(
        replay.trace.iter().find(|l| l.starts_with("K:")).unwrap(),
        "K:4:999:2:0"
    );
}

#[test]
fn consumed_event_drains_before_leaving_callback_context() {
    let mut state = State::default();
    state.list.context.stack = vec![0];
    state.list.context.enabled = vec![0, 1];
    let mut replay = Replay {
        time: 0,
        specs: vec![
            Spec {
                plugin: 0,
                before: 1,
                action: 0,
                result: 1,
            },
            Spec {
                plugin: 1,
                before: 1,
                action: 1,
                result: 0,
            },
            Spec {
                plugin: 0,
                before: 1,
                action: 1,
                result: 1,
            },
        ],
        new_before: 1,
        trace: Vec::new(),
    };
    register_initial(&mut state, &mut replay);
    replay.trace.clear();
    let input = InterceptInput {
        character: 27,
        virtual_key: 27,
        flags: 8,
        phase: 0,
    };
    assert_eq!(
        xplm_sniffer::dispatch(&mut state, input, &mut replay).unwrap(),
        0
    );
    assert!(state.list.nodes[&2].freed);
    assert!(!state.list.nodes[&3].freed);
    assert_eq!(state.list.nodes[&3].deleted, 1);
    assert_eq!(state.list.context.stack, [0]);
    assert_eq!((state.list.deferred, state.iteration.pending), (0, 0));
    assert!(replay.trace.iter().any(|l| l == "D:1:3:0:1"));
}
