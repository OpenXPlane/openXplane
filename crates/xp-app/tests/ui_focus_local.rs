use openxplane::ui_focus::{FocusState, Notification};

#[test]
#[ignore = "requires local native focus replay; set OPENXPLANE_UI_FOCUS_VECTORS"]
fn native_focus_state_and_callback_order_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_UI_FOCUS_VECTORS").expect("set OPENXPLANE_UI_FOCUS_VECTORS"),
    )
    .unwrap();
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("F ")) {
        let (input, output) = line.split_once(" -> ").unwrap();
        let v: Vec<i64> = input
            .split_whitespace()
            .skip(1)
            .map(|s| s.parse().unwrap())
            .collect();
        let mut state = FocusState {
            selected: v[0] as u64,
            focused: v[1] as u64,
            mode: v[2] as u32,
        };
        let mut log = Vec::new();
        state.select(
            v[3] as u64,
            v[4] as u32,
            |root| match root {
                1 => 1,
                2 => 21,
                3 => 32,
                _ => panic!("unknown root"),
            },
            |s, n| {
                let (kind, object, update) = match n {
                    Notification::Leave(t) => ("L", t, v[5]),
                    Notification::Blur(t) => ("B", t, v[6]),
                    Notification::Enter(t) => ("E", t, -1),
                };
                log.extend([
                    kind.to_string(),
                    object.to_string(),
                    s.selected.to_string(),
                    s.focused.to_string(),
                    s.mode.to_string(),
                ]);
                if update >= 0 {
                    s.focused = update as u64;
                    s.selected = 3;
                    s.mode = 0x76543210;
                }
            },
        );
        let mut fields = output.split_whitespace();
        assert_eq!(
            state.selected,
            fields.next().unwrap().parse::<u64>().unwrap(),
            "case {count}"
        );
        assert_eq!(
            state.focused,
            fields.next().unwrap().parse::<u64>().unwrap(),
            "case {count}"
        );
        assert_eq!(
            state.mode,
            fields.next().unwrap().parse::<u32>().unwrap(),
            "case {count}"
        );
        let n: usize = fields.next().unwrap().parse().unwrap();
        let expected: Vec<_> = fields.map(str::to_string).collect();
        assert_eq!(expected.len(), n * 5);
        assert_eq!(log, expected, "case {count}");
        count += 1;
    }
    assert_eq!(count, 2024);
}
