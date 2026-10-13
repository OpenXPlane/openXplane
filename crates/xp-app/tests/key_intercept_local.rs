use openxplane::{
    key_intercept::{self, Services, State},
    keyboard_dispatch::InterceptInput,
};
struct Replay {
    times: [f64; 2],
    next: usize,
    token: i32,
    result: i32,
    flip: bool,
    elapsed: bool,
    context: bool,
    trace: Vec<String>,
}
impl Services for Replay {
    fn clock(&mut self, _: &mut State) -> f64 {
        self.trace.push(format!("T:{}", self.next));
        let t = self.times[self.next];
        self.next += 1;
        t
    }
    fn enter_scope(&mut self, _: &mut State, c: u64) -> u64 {
        self.trace.push(format!("S:{c}"));
        123
    }
    fn leave_scope(&mut self, _: &mut State, c: u64, t: u64) {
        self.trace.push(format!("R:{c}:{t}"));
    }
    fn enter_hook(&mut self, s: &mut State) -> i32 {
        self.trace.push("A".into());
        if self.flip {
            s.export_present = !s.export_present;
        }
        self.token
    }
    fn leave_hook(&mut self, _: &mut State, t: i32) {
        self.trace.push(format!("Z:{}", t as u32));
    }
    fn call_hook(&mut self, s: &mut State, i: InterceptInput, r: u64) -> i32 {
        self.trace.push(format!(
            "H:{}:{}:{}:{}:{r}",
            i.character, i.flags, i.virtual_key, i.phase
        ));
        if self.elapsed {
            s.elapsed = -3.5;
        }
        if self.context {
            s.scope_context = 99;
        }
        self.result
    }
}
#[test]
#[ignore = "requires local native interceptor replay; set OPENXPLANE_KEY_INTERCEPT_VECTORS"]
fn native_interceptor_gates_adapter_timing_and_order_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_KEY_INTERCEPT_VECTORS")
            .expect("set OPENXPLANE_KEY_INTERCEPT_VECTORS"),
    )
    .unwrap();
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("J ")) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let v: Vec<_> = a.split_whitespace().skip(1).collect();
        let u = |i: usize| v[i].parse::<u64>().unwrap();
        let int = |i: usize| v[i].parse::<i32>().unwrap();
        let f = |i: usize| f64::from_bits(u64::from_str_radix(v[i], 16).unwrap());
        let mut s = State {
            registered: u(0) != 0,
            export_present: u(1) != 0,
            mode: int(2),
            scope_mask: u(3) as u32,
            enabled_mask: u(4) as u32,
            scope_context: u(5),
            elapsed: f(6),
        };
        let mut r = Replay {
            times: [f(7), f(8)],
            next: 0,
            token: int(9),
            result: int(10),
            flip: u(15) != 0,
            elapsed: u(16) != 0,
            context: u(17) != 0,
            trace: Vec::new(),
        };
        let input = InterceptInput {
            character: u(11) as u8,
            virtual_key: u(12) as u8,
            flags: u(13) as u32,
            phase: u(18) as u32,
        };
        let result = key_intercept::intercept(&mut s, input, u(14), &mut r);
        let mut expected = b.split_whitespace();
        assert_eq!(
            result as u32,
            expected.next().unwrap().parse::<u32>().unwrap(),
            "case {count}"
        );
        assert_eq!(
            s.elapsed.to_bits(),
            u64::from_str_radix(expected.next().unwrap(), 16).unwrap(),
            "time case {count}"
        );
        assert_eq!(
            s.scope_context,
            expected.next().unwrap().parse::<u64>().unwrap(),
            "context case {count}"
        );
        assert_eq!(
            s.export_present,
            expected.next().unwrap() == "1",
            "export case {count}"
        );
        assert_eq!(
            r.trace,
            expected.map(str::to_string).collect::<Vec<_>>(),
            "trace case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 1864);
}
