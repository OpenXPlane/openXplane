use openxplane::{
    keyboard_dispatch::InterceptInput,
    xplm_key_hook::{self, Hotkey, Iteration, List, Services, Sniffer, State, Window},
};
struct Replay {
    context: openxplane::xplm_context::State,
    outcomes: Vec<i32>,
    trace: Vec<String>,
}
impl Services for Replay {
    fn validate(&mut self, s: u8) {
        self.trace.push(format!("V:{s}"));
    }
    fn enter(&mut self, _: &mut State, p: i32) -> bool {
        self.trace.push(format!("G:{p}"));
        self.trace.push(format!("A:{p}"));
        self.context.enter(p);
        self.context
            .active(&mut ContextTrace(&mut self.trace))
            .unwrap()
    }
    fn leave(&mut self, _: &mut State, p: i32) {
        self.trace.push(format!("E:{p}"));
        self.context
            .leave(&mut ContextTrace(&mut self.trace))
            .unwrap();
    }
    fn sniff(&mut self, _: &mut State, i: usize, _: InterceptInput, r: u64) -> i32 {
        self.trace.push(format!("S:{i}:{r}"));
        self.outcomes[i]
    }
    fn hotkey(&mut self, _: &mut State, i: usize, r: u64) {
        self.trace.push(format!("H:{i}:{r}"));
    }
    fn window(&mut self, _: &mut State, t: u64, _: InterceptInput, r: u64) {
        self.trace.push(format!("W:{t}:{r}"));
    }
    fn flush(&mut self, s: &mut State, l: List, step: u8) {
        let (i, it) = match l {
            List::Sniffers => (0, s.sniffer_iteration),
            List::Hotkeys => (1, s.hotkey_iteration),
        };
        self.trace
            .push(format!("F:{i}:{step}:{}:{}", it.depth, it.pending));
    }
}
struct ContextTrace<'a>(&'a mut Vec<String>);
impl openxplane::xplm_context::Services for ContextTrace<'_> {
    fn validate_enabled(&mut self, _: &mut openxplane::xplm_context::State) {
        self.0.push("C".into());
    }
    fn ensure_loaded(&mut self, _: &mut openxplane::xplm_context::State, p: i32) {
        self.0.push(format!("L:{p}"));
    }
    fn unload(&mut self, _: &mut openxplane::xplm_context::State, _: i32) {
        panic!("no pending unload in key replay")
    }
    fn notify_unload(&mut self, _: &mut openxplane::xplm_context::State) {
        panic!("no pending unload in key replay")
    }
}

#[test]
#[ignore = "requires local Windows XPLM DLL replay; set OPENXPLANE_XPLM_KEY_HOOK_VECTORS"]
fn native_xplm_key_phases_callbacks_and_iteration_state_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_XPLM_KEY_HOOK_VECTORS")
            .expect("set OPENXPLANE_XPLM_KEY_HOOK_VECTORS"),
    )
    .unwrap();
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("P ")) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let mut v = a.split_whitespace().skip(1);
        let mut next = || v.next().unwrap().parse::<i64>().unwrap();
        let phase = next() as u32;
        let character = next() as u8;
        let virtual_key = next() as u8;
        let flags = next() as u32;
        let active = next() as u32;
        let target = next() as u64;
        let focus = next() != 0;
        let sniffer_iteration = Iteration {
            depth: next() as i32,
            pending: next() as u8,
        };
        let hotkey_iteration = Iteration {
            depth: next() as i32,
            pending: next() as u8,
        };
        let n = next();
        let mut sniffers = Vec::new();
        let mut outcomes = Vec::new();
        for i in 0..n {
            sniffers.push(Sniffer {
                deleted: next() as u8,
                before: next() as i32,
                plugin: next() as i32,
                refcon: 100 + i as u64,
            });
            outcomes.push(next() as i32);
        }
        let n = next();
        let mut hotkeys = Vec::new();
        for i in 0..n {
            hotkeys.push(Hotkey {
                deleted: next() as u8,
                key: next() as u8,
                flags: next() as u32,
                plugin: next() as i32,
                refcon: 200 + i as u64,
            });
        }
        assert!(v.next().is_none());
        let mut s = State {
            sniffers,
            hotkeys,
            sniffer_iteration,
            hotkey_iteration,
            focused: focus.then_some(Window {
                token: 1,
                plugin: 3,
                refcon: 300,
            }),
        };
        let mut r = Replay {
            context: openxplane::xplm_context::State {
                stack: vec![-1],
                enabled: (0..4).map(|i| u8::from(active & (1 << i) != 0)).collect(),
                ..openxplane::xplm_context::State::default()
            },
            outcomes,
            trace: Vec::new(),
        };
        let input = InterceptInput {
            character,
            virtual_key,
            flags,
            phase,
        };
        let result = xplm_key_hook::dispatch(&mut s, input, target, &mut r);
        assert_eq!(r.context.stack, [-1]);
        let mut out = b.split_whitespace();
        let expected: Vec<u32> = (0..5)
            .map(|_| out.next().unwrap().parse().unwrap())
            .collect();
        assert_eq!(
            [
                result,
                s.sniffer_iteration.depth as u32,
                s.sniffer_iteration.pending as u32,
                s.hotkey_iteration.depth as u32,
                s.hotkey_iteration.pending as u32
            ]
            .as_slice(),
            expected,
            "state case {count}"
        );
        assert_eq!(
            r.trace,
            out.map(str::to_string).collect::<Vec<_>>(),
            "trace case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 1090);
}
