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
    settings: Vec<i64>,
    mode: i32,
    block: u8,
    windows: openxplane::ui_state::WindowState,
    sniffers: Option<State>,
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
        self.mutate();
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

impl Replay {
    fn mutate(&mut self) {
        if self.settings[7] >= 0 {
            self.mode = self.settings[7] as i32;
        }
        if self.settings[8] >= 0 {
            self.block = self.settings[8] as u8;
        }
    }
}
impl openxplane::keyboard_dispatch::Services for Replay {
    fn has_visible_capture(&mut self) -> bool {
        self.trace.push("UIQ".into());
        self.windows.visible_classification(2).unwrap()
    }
    fn intercept(&mut self, input: InterceptInput) -> i32 {
        self.trace.push(format!(
            "IP:{}:{}:{}:{}",
            input.character, input.virtual_key, input.flags, input.phase
        ));
        let mut state = openxplane::key_intercept::State {
            registered: true,
            export_present: true,
            mode: self.mode,
            ..Default::default()
        };
        openxplane::key_intercept::intercept(&mut state, input, 0, &mut HookReplay(self))
    }
    fn focused(&mut self, input: openxplane::ui_input::KeyInput) -> u8 {
        self.trace.push(format!(
            "UI:{}:{}:{}",
            input.character as u32, input.virtual_key, input.flags
        ));
        let mask = self.settings[3] as u32;
        openxplane::ui_input::dispatch_focused(
            self.settings[1] != 0,
            || mask,
            input,
            None,
            |_| {
                self.trace.push("UIO".into());
                self.mutate();
                self.settings[2] as u8
            },
        )
        .unwrap()
    }
    fn command_gate(&mut self) -> (i32, u8) {
        (self.mode, self.block)
    }
}
struct HookReplay<'a>(&'a mut Replay);
impl openxplane::key_intercept::Services for HookReplay<'_> {
    fn clock(&mut self, _: &mut openxplane::key_intercept::State) -> f64 {
        0.0
    }
    fn enter_scope(&mut self, _: &mut openxplane::key_intercept::State, _: u64) -> u64 {
        panic!("scope disabled")
    }
    fn leave_scope(&mut self, _: &mut openxplane::key_intercept::State, _: u64, _: u64) {
        panic!("scope disabled")
    }
    fn enter_hook(&mut self, _: &mut openxplane::key_intercept::State) -> i32 {
        0
    }
    fn leave_hook(&mut self, _: &mut openxplane::key_intercept::State, _: i32) {}
    fn call_hook(
        &mut self,
        _: &mut openxplane::key_intercept::State,
        input: InterceptInput,
        refcon: u64,
    ) -> i32 {
        assert_eq!(refcon, 0);
        self.0.trace.push(format!("XD:{}", input.phase));
        let mut sdk = self.0.sniffers.take().unwrap();
        let result = xplm_sniffer::dispatch(&mut sdk, input, self.0).unwrap();
        assert_eq!(sdk.list.context.stack, [0]);
        self.0.sniffers = Some(sdk);
        result as i32
    }
}
#[test]
#[ignore = "requires registered paired EXE/DLL replay; set OPENXPLANE_KEYBOARD_REGISTERED_VECTORS"]
fn native_exe_routing_and_mutable_registered_dll_callbacks_match_together() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_KEYBOARD_REGISTERED_VECTORS")
            .expect("set OPENXPLANE_KEYBOARD_REGISTERED_VECTORS"),
    )
    .unwrap();
    assert!(
        text.lines().any(|l| l
            == "# dll_sha256 706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f")
    );
    assert!(text.lines().any(|l| l
        == "# reference_sha256 2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"));
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("K ")) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let mut values = a.split_whitespace().skip(1);
        let mut next = || values.next().unwrap().parse::<i128>().unwrap();
        let settings: Vec<_> = (0..9).map(|_| next() as i64).collect();
        let character = next() as i32;
        let virtual_key = next() as u32;
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
        let event_n = next() as usize;
        for _ in 0..event_n {
            next();
        }
        assert!(values.next().is_none());
        let window = |token, classification, opacity| {
            Some(openxplane::ui_state::RegisteredWindow {
                token,
                classification,
                opacity_root_to_leaf: vec![opacity],
            })
        };
        let windows = openxplane::ui_state::WindowState {
            registered: if settings[0] != 0 {
                vec![None, window(9, 2, 1.0)]
            } else {
                vec![window(9, 1, 1.0), window(10, 2, 0.0)]
            },
            ..Default::default()
        };
        let mut replay = Replay {
            mode: settings[4] as i32,
            block: settings[5] as u8,
            settings,
            specs,
            time,
            new_before,
            trace: Vec::new(),
            windows,
            sniffers: None,
        };
        let mut state = State::default();
        state.list.context.stack = vec![0];
        state.list.context.enabled = (0..4).map(|i| u8::from(enabled & (1 << i) != 0)).collect();
        register_initial(&mut state, &mut replay);
        state.iteration.depth = depth;
        replay.sniffers = Some(state);
        let mut suppression = replay.settings[6] as i32;
        let mut entries: Vec<_> = [0, 1, -1]
            .into_iter()
            .zip([3, 29, 30])
            .map(|(active, command)| xp_dataref::key_event::Entry {
                key: virtual_key,
                modifiers: flags & 7,
                active,
                command,
            })
            .collect();
        for _ in 0..event_n {
            let actions = openxplane::keyboard_dispatch::dispatch(
                &mut entries,
                &mut suppression,
                openxplane::ui_input::KeyInput {
                    character,
                    virtual_key,
                    flags,
                },
                &mut replay,
            );
            for action in actions {
                replay.trace.push(match action {
                    xp_dataref::key_event::Action::Begin(command) => format!("BEGIN:{command}"),
                    xp_dataref::key_event::Action::End(command) => format!("END:{command}"),
                });
            }
        }
        let state = replay.sniffers.as_ref().unwrap();
        assert_eq!(state.list.context.stack, [0], "stack case {count}");
        let mut output = b.split_whitespace();
        let mut next = || output.next().unwrap().parse::<u64>().unwrap();
        // EXE return registers are not compared; zero placeholders are not native return evidence.
        for _ in 0..event_n {
            assert_eq!(next(), 0);
        }
        assert_eq!(
            [
                state.list.head,
                state.list.deferred,
                state.iteration.depth as u32 as u64,
                state.iteration.pending as u64,
                state.list.nodes.len() as u64
            ],
            [next(), next(), next(), next(), next()],
            "SDK state case {count}"
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
            [
                suppression as u32 as u64,
                entries[0].active as u32 as u64,
                entries[1].active as u32 as u64,
                entries[2].active as u32 as u64,
                replay.mode as u32 as u64,
                replay.block as u64
            ],
            [next(), next(), next(), next(), next(), next()],
            "EXE state case {count}"
        );
        assert_eq!(
            replay.trace,
            output.map(str::to_string).collect::<Vec<_>>(),
            "trace case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 528);
}

#[test]
fn callback_mode_change_blocks_commands_after_native_list_removal() {
    let mut state = State::default();
    state.list.context.stack = vec![0];
    let mut replay = Replay {
        settings: vec![0, 0, 0, 0xffffffff, 0, 0, 0, 1000, -1],
        mode: 0,
        block: 0,
        windows: Default::default(),
        sniffers: None,
        time: 0,
        specs: vec![Spec {
            plugin: 0,
            before: 1,
            action: 1,
            result: 1,
        }],
        new_before: 1,
        trace: Vec::new(),
    };
    register_initial(&mut state, &mut replay);
    replay.sniffers = Some(state);
    replay.trace.clear();
    let mut entries: Vec<_> = [0, 1, -1]
        .into_iter()
        .zip([3, 29, 30])
        .map(|(active, command)| xp_dataref::key_event::Entry {
            key: 27,
            modifiers: 0,
            active,
            command,
        })
        .collect();
    let mut suppression = 0;
    for _ in 0..2 {
        let actions = openxplane::keyboard_dispatch::dispatch(
            &mut entries,
            &mut suppression,
            openxplane::ui_input::KeyInput {
                character: 27,
                virtual_key: 27,
                flags: 8,
            },
            &mut replay,
        );
        assert!(actions.is_empty());
    }
    assert_eq!(replay.mode, 1000);
    assert_eq!(
        entries.iter().map(|e| e.active).collect::<Vec<_>>(),
        [0, 1, -1]
    );
    let state = replay.sniffers.unwrap();
    assert_eq!(
        (
            state.list.head,
            state.list.deferred,
            state.iteration.pending
        ),
        (0, 0, 0)
    );
    assert!(state.list.nodes[&1].freed);
    assert_eq!(state.list.context.stack, [0]);
    assert_eq!(
        replay
            .trace
            .iter()
            .filter(|s| s.starts_with("XD:"))
            .cloned()
            .collect::<Vec<_>>(),
        ["XD:0"]
    );
}
