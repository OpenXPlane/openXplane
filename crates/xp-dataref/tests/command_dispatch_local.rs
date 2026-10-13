use xp_dataref::command_dispatch::{self, CommandState, ContinueServices, Phase, Services, State};
struct Replay {
    mutation: u32,
    primary_mutation: bool,
    before: i32,
    trace: Vec<String>,
}
impl Services for Replay {
    fn callback(&mut self, s: &mut State, object: u64, command: i32, phase: Phase) {
        self.trace.push(format!(
            "C:{object}:{}:{}",
            if phase == Phase::Begin { 0 } else { 1 },
            s.commands[command as usize].count
        ));
        if object == 1 {
            match self.mutation {
                1 => {
                    s.observers.remove(&2);
                }
                2 => {
                    s.observers.remove(&2);
                    s.observers.remove(&3);
                }
                3 => {
                    s.observers.insert(4);
                    s.dt = -0.375;
                }
                _ => {}
            }
        }
    }
    fn intercept(&mut self, s: &mut State, command: i32, _: Phase, before: bool) -> i32 {
        self.trace.push(format!("P:{}", u8::from(before)));
        if before && self.primary_mutation {
            s.commands[command as usize].primary = 4;
            s.dt = 0.25;
        }
        self.before
    }
}
impl ContinueServices for Replay {
    fn continued(&mut self, s: &mut State, object: u64, command: i32, elapsed: f32, dt: f32) {
        self.callback(s, object, command, Phase::Continue);
        *self.trace.last_mut().unwrap() = format!(
            "F:{object}:{:08x}:{:08x}:{}",
            elapsed.to_bits(),
            dt.to_bits(),
            s.commands[command as usize].count
        );
    }
}

#[test]
#[ignore = "requires local full Begin/Continue/End replay; set OPENXPLANE_COMMAND_DISPATCH_VECTORS"]
fn native_command_phases_counters_snapshots_and_callbacks_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_COMMAND_DISPATCH_VECTORS")
            .expect("set OPENXPLANE_COMMAND_DISPATCH_VECTORS"),
    )
    .unwrap();
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("D ")) {
        let (input, output) = line.split_once(" -> ").unwrap();
        let v: Vec<_> = input.split_whitespace().skip(1).collect();
        let signed = |i: usize| v[i].parse::<i32>().unwrap();
        let unsigned = |i: usize| v[i].parse::<u32>().unwrap();
        let command = signed(1);
        let phase = match unsigned(0) {
            0 => Phase::Begin,
            1 => Phase::End,
            _ => Phase::Continue,
        };
        let mut state = State {
            commands: vec![CommandState::default(); 3138],
            observers: [1, 2, 3].into_iter().collect(),
            block_byte: unsigned(3) as u8,
            dt: v[12].parse().unwrap(),
            rate_delta: 0.75,
            now: f64::from_bits(u64::from_str_radix(v[9], 16).unwrap()),
            ..State::default()
        };
        state.commands[command as usize] = CommandState {
            count: signed(2),
            started: 0.75,
            primary: unsigned(7) as u64,
        };
        state.gate.default_mask = unsigned(4);
        if unsigned(6) != 0 {
            state.overrides.insert(command, 1);
        }
        let mut r = Replay {
            mutation: unsigned(10),
            primary_mutation: unsigned(11) != 0,
            before: signed(8),
            trace: Vec::new(),
        };
        if phase == Phase::Continue {
            command_dispatch::continue_command(&mut state, command, unsigned(5), &mut r).unwrap();
        } else {
            command_dispatch::dispatch(&mut state, command, unsigned(5), phase, &mut r).unwrap();
        }
        let mut expected = output.split_whitespace();
        let slot = state.commands[command as usize];
        assert_eq!(
            slot.count,
            expected.next().unwrap().parse::<i32>().unwrap(),
            "count case {count}"
        );
        assert_eq!(
            slot.started.to_bits(),
            u32::from_str_radix(expected.next().unwrap(), 16).unwrap(),
            "timestamp case {count}"
        );
        assert_eq!(
            slot.primary,
            expected.next().unwrap().parse::<u64>().unwrap(),
            "primary case {count}"
        );
        let observers: Vec<u64> = expected
            .next()
            .unwrap()
            .split(',')
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(
            state.observers.iter().copied().collect::<Vec<_>>(),
            observers,
            "observers case {count}"
        );
        assert_eq!(
            state.rate_delta.to_bits(),
            u32::from_str_radix(expected.next().unwrap(), 16).unwrap(),
            "rate case {count}"
        );
        assert_eq!(
            state.dt.to_bits(),
            u32::from_str_radix(expected.next().unwrap(), 16).unwrap(),
            "dt case {count}"
        );
        assert_eq!(
            r.trace,
            expected.map(str::to_string).collect::<Vec<_>>(),
            "trace case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 2152);
}
