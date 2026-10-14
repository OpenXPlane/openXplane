use openxplane::{
    ui_input::{self, KeyInput},
    ui_menu_key::{self, Services, State},
};
struct Cancel {
    update: [u8; 2],
    calls: usize,
}
impl Services for Cancel {
    fn fallback_action(&mut self, state: &mut State) {
        self.calls += 1;
        state.flag168 = self.update[0];
        state.flag169 = self.update[1];
    }
}
#[test]
#[ignore = "requires native concrete menu key replay; set OPENXPLANE_UI_MENU_KEY_VECTORS"]
fn native_concrete_menu_own_and_focused_key_paths_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_UI_MENU_KEY_VECTORS")
            .expect("set OPENXPLANE_UI_MENU_KEY_VECTORS"),
    )
    .unwrap();
    assert!(text.lines().any(|l| l
        == "# reference_sha256 2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"));
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("M ")) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let v: Vec<i64> = a
            .split_whitespace()
            .skip(1)
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(v.len(), 8);
        let input = KeyInput {
            character: v[2] as i32,
            virtual_key: v[3] as u32,
            flags: v[4] as u32,
        };
        let mask = v[5] as u32;
        for (focused, expected) in [false, true].into_iter().zip(b.split(" | ")) {
            let mut state = State {
                flag168: v[0] as u8,
                flag169: v[1] as u8,
            };
            let mut services = Cancel {
                update: [v[6] as u8, v[7] as u8],
                calls: 0,
            };
            let result = if focused {
                ui_input::dispatch_focused(
                    true,
                    || mask,
                    input,
                    None,
                    |event| ui_menu_key::dispatch(&mut state, event, &mut services),
                )
                .unwrap()
            } else {
                ui_menu_key::dispatch(&mut state, input, &mut services)
            };
            let mut fields = expected.split_whitespace();
            assert_eq!(
                [result, state.flag168, state.flag169],
                [
                    fields.next().unwrap().parse().unwrap(),
                    fields.next().unwrap().parse().unwrap(),
                    fields.next().unwrap().parse().unwrap()
                ],
                "case {count} focused {focused}"
            );
            let calls: Vec<_> = fields.collect();
            assert_eq!(
                services.calls,
                calls.iter().filter(|s| **s == "C").count(),
                "cancel case {count}"
            );
            let ran = !focused || calls.contains(&"K");
            let expected_calls = if !ran {
                vec![]
            } else if input.flags & 8 == 0 {
                vec!["K"]
            } else {
                match input.virtual_key {
                    27 => vec!["K", "F", if v[1] == 0 { "C" } else { "N" }],
                    13 | 0xbb => {
                        if v[0] != 0 {
                            vec!["K", "A", "V", "N"]
                        } else {
                            vec!["K", "A", "F", if v[1] == 0 { "C" } else { "N" }]
                        }
                    }
                    _ => vec!["K"],
                }
            };
            assert_eq!(
                calls, expected_calls,
                "real getters/no-op trace case {count}"
            );
        }
        count += 1;
    }
    assert_eq!(count, 1940);
}
