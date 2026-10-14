use openxplane::xplm_context::{Services, State};
struct Replay {
    load_mutation: i32,
    unload_mutation: bool,
    trace: Vec<String>,
}
impl Services for Replay {
    fn validate_enabled(&mut self, _: &mut State) {
        self.trace.push("V".into());
    }
    fn ensure_loaded(&mut self, s: &mut State, p: i32) {
        self.trace.push(format!("L:{p}"));
        if self.load_mutation == 1
            && let Ok(i) = usize::try_from(p)
            && let Some(byte) = s.enabled.get_mut(i)
        {
            *byte = 255;
        }
        if self.load_mutation == 2 {
            *s.stack.last_mut().unwrap() = 0;
        }
    }
    fn unload(&mut self, s: &mut State, p: i32) {
        self.trace.push(format!("U:{p}"));
        if self.unload_mutation {
            s.stack.push(9);
        }
    }
    fn notify_unload(&mut self, _: &mut State) {
        self.trace.push("N".into());
    }
}
#[test]
#[ignore = "requires native initialized-TLS XPLM replay; set OPENXPLANE_XPLM_CONTEXT_VECTORS"]
fn native_plugin_context_stack_activation_and_pending_unload_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_XPLM_CONTEXT_VECTORS")
            .expect("set OPENXPLANE_XPLM_CONTEXT_VECTORS"),
    )
    .unwrap();
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("T ")) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let mut v = a.split_whitespace().skip(1);
        let mut next = || v.next().unwrap().parse::<i64>().unwrap();
        let plugin = next() as i32;
        let pending_unload = next() as i32;
        let n = next();
        let stack = (0..n).map(|_| next() as i32).collect();
        let n = next();
        let enabled = (0..n).map(|_| next() as u8).collect();
        let mut r = Replay {
            load_mutation: next() as i32,
            unload_mutation: next() != 0,
            trace: Vec::new(),
        };
        assert!(v.next().is_none());
        let mut s = State {
            stack,
            enabled,
            pending_unload,
        };
        s.enter(plugin);
        let active = s.active(&mut r).unwrap();
        s.leave(&mut r).unwrap();
        let mut expected = b.split_whitespace();
        assert_eq!(
            active,
            expected.next().unwrap() == "1",
            "active case {count}"
        );
        let n: usize = expected.next().unwrap().parse().unwrap();
        let stack: Vec<i32> = (0..n)
            .map(|_| expected.next().unwrap().parse().unwrap())
            .collect();
        assert_eq!(s.stack, stack, "stack case {count}");
        let n: usize = expected.next().unwrap().parse().unwrap();
        let enabled: Vec<u8> = (0..n)
            .map(|_| expected.next().unwrap().parse().unwrap())
            .collect();
        assert_eq!(s.enabled, enabled, "enabled case {count}");
        assert_eq!(
            s.pending_unload as u32,
            expected.next().unwrap().parse::<u32>().unwrap(),
            "pending case {count}"
        );
        assert_eq!(
            r.trace,
            expected.map(str::to_string).collect::<Vec<_>>(),
            "trace case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 1108);
}

#[test]
#[ignore = "requires native initialized-TLS XPLM replay; set OPENXPLANE_XPLM_CONTEXT_VECTORS"]
fn native_owner_query_matches_and_preserves_all_context_frames() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_XPLM_CONTEXT_VECTORS")
            .expect("set OPENXPLANE_XPLM_CONTEXT_VECTORS"),
    )
    .unwrap();
    assert!(text.lines().any(|line| line
        == "# dll_sha256 706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f"));
    let mut count = 0;
    for line in text.lines().filter(|line| line.starts_with("O ")) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let mut input = a.split_whitespace().skip(1);
        let n: usize = input.next().unwrap().parse().unwrap();
        let stack: Vec<i32> = (0..n)
            .map(|_| input.next().unwrap().parse().unwrap())
            .collect();
        assert!(input.next().is_none());
        let state = State {
            stack,
            ..State::default()
        };
        let mut output = b.split_whitespace();
        let owner: i32 = output.next().unwrap().parse().unwrap();
        assert_eq!(state.current_id(), owner, "owner case {count}");
        let n: usize = output.next().unwrap().parse().unwrap();
        let final_stack: Vec<i32> = (0..n)
            .map(|_| output.next().unwrap().parse().unwrap())
            .collect();
        assert_eq!(state.stack, final_stack, "stack case {count}");
        assert!(output.next().is_none());
        count += 1;
    }
    assert_eq!(count, 1657);
}
