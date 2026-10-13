use openxplane::{
    keyboard_dispatch::{self, InterceptInput, Services},
    ui_input::{self, KeyInput},
};
use xp_dataref::key_event::{Action, Entry};

struct Replay {
    v: Vec<i64>,
    mode: i32,
    block: u8,
    trace: Vec<String>,
}
impl Replay {
    fn mutate(&mut self) {
        if self.v[12] >= 0 {
            self.mode = self.v[12] as i32;
        }
        if self.v[13] >= 0 {
            self.block = self.v[13] as u8;
        }
    }
}
impl Services for Replay {
    fn has_visible_capture(&mut self) -> bool {
        self.trace.push("Q".into());
        self.v[0] != 0
    }
    fn intercept(&mut self, i: InterceptInput) -> i32 {
        self.trace.push(format!(
            "P:{}:{}:{}:{}",
            i.character, i.virtual_key, i.flags, i.phase
        ));
        self.mutate();
        self.v[if i.phase == 0 { 1 } else { 2 }] as i32
    }
    fn focused(&mut self, i: KeyInput) -> u8 {
        self.trace.push(format!(
            "U:{}:{}:{}",
            i.character as u32, i.virtual_key, i.flags
        ));
        let mask = self.v[8] as u32;
        ui_input::dispatch_focused(
            self.v[10] != 0,
            || mask,
            i,
            None,
            |_| {
                self.trace.push("O".into());
                self.mutate();
                self.v[3] as u8
            },
        )
        .unwrap()
    }
    fn command_gate(&mut self) -> (i32, u8) {
        (self.mode, self.block)
    }
}

#[test]
#[ignore = "requires local full keyboard replay; set OPENXPLANE_KEYBOARD_DISPATCH_VECTORS"]
fn native_keyboard_gates_ui_and_binding_transitions_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_KEYBOARD_DISPATCH_VECTORS")
            .expect("set OPENXPLANE_KEYBOARD_DISPATCH_VECTORS"),
    )
    .unwrap();
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("K ")) {
        let (input, output) = line.split_once(" -> ").unwrap();
        let v: Vec<i64> = input
            .split_whitespace()
            .skip(1)
            .map(|s| s.parse().unwrap())
            .collect();
        let event = KeyInput {
            character: v[6] as i32,
            virtual_key: v[7] as u32,
            flags: v[9] as u32,
        };
        let mut suppression = v[11] as i32;
        let mut entries = [0, 1, -1]
            .into_iter()
            .zip([3, 29, 30])
            .map(|(active, command)| Entry {
                key: event.virtual_key,
                modifiers: event.flags & 7,
                active,
                command,
            })
            .collect::<Vec<_>>();
        let mut r = Replay {
            mode: v[4] as i32,
            block: v[5] as u8,
            v,
            trace: Vec::new(),
        };
        let actions = keyboard_dispatch::dispatch(&mut entries, &mut suppression, event, &mut r);
        for action in actions {
            r.trace.push(match action {
                Action::Begin(c) => format!("B:{}", c as u32),
                Action::End(c) => format!("E:{}", c as u32),
            });
        }
        let mut fields = output.split_whitespace();
        let actual = [
            suppression as i64,
            entries[0].active as i64,
            entries[1].active as i64,
            entries[2].active as i64,
            r.mode as u32 as i64,
            r.block as i64,
        ];
        let expected: Vec<i64> = (0..6)
            .map(|_| fields.next().unwrap().parse().unwrap())
            .collect();
        assert_eq!(actual.as_slice(), expected, "case {count}");
        assert_eq!(
            r.trace,
            fields.map(str::to_string).collect::<Vec<_>>(),
            "case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 2152);
}
