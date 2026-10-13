use openxplane::{
    menu_commands::{Command, Request},
    ui_state::{RegisteredWindow, WindowState, cumulative_opacity},
};

fn request(name: &str) -> Request {
    match name {
        "QueryMainMenu" => Request::QueryMainMenu,
        "QueryState" => Request::QueryState,
        "CloseMainMenu" => Request::CloseMainMenu,
        "EnsureMainMenu" => Request::EnsureMainMenu,
        "Refresh" => Request::Refresh,
        "ToggleSettingsWindow" => Request::ToggleSettingsWindow,
        _ => panic!("unknown native request"),
    }
}

#[test]
#[ignore = "requires local original UI-state replay; set OPENXPLANE_UI_STATE_VECTORS"]
fn native_queries_opacity_and_menu_routing_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_UI_STATE_VECTORS").expect("set OPENXPLANE_UI_STATE_VECTORS"),
    )
    .unwrap();
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("W ")) {
        let (input, output) = line.split_once(" -> ").unwrap();
        let mut input = input.split_whitespace().skip(1);
        let kind: i32 = input.next().unwrap().parse().unwrap();
        let byte = input.next().unwrap().parse().unwrap();
        let mut state = WindowState {
            state_byte_525: byte,
            ..Default::default()
        };
        let mapping_count: usize = input.next().unwrap().parse().unwrap();
        for _ in 0..mapping_count {
            let key = input.next().unwrap().parse().unwrap();
            let token = input.next().unwrap().parse().unwrap();
            assert!(state.by_kind.insert(key, token).is_none());
        }
        let registry_count: usize = input.next().unwrap().parse().unwrap();
        for _ in 0..registry_count {
            let token: u64 = input.next().unwrap().parse().unwrap();
            let depth: usize = input.next().unwrap().parse().unwrap();
            if token == 0 {
                assert_eq!(depth, 0);
                state.registered.push(None);
            } else {
                let path = (0..depth)
                    .map(|_| {
                        f32::from_bits(u32::from_str_radix(input.next().unwrap(), 16).unwrap())
                    })
                    .collect();
                state.registered.push(Some(RegisteredWindow {
                    token,
                    opacity_root_to_leaf: path,
                }));
            }
        }
        assert!(input.next().is_none());
        let mut output = output.split_whitespace();
        let expected_query: u8 = output.next().unwrap().parse().unwrap();
        assert_eq!(
            state.query(kind).unwrap(),
            expected_query != 0,
            "context {count}"
        );
        for entry in &state.registered {
            let expected = u32::from_str_radix(output.next().unwrap(), 16).unwrap();
            if let Some(window) = entry {
                assert_eq!(
                    cumulative_opacity(&window.opacity_root_to_leaf)
                        .unwrap()
                        .to_bits(),
                    expected,
                    "opacity in context {count}"
                );
            } else {
                assert_eq!(expected, 0);
            }
        }
        for command in [Command::ToggleMainMenu, Command::ToggleSettings] {
            let trace_count: usize = output.next().unwrap().parse().unwrap();
            let expected: Vec<_> = (0..trace_count)
                .map(|_| request(output.next().unwrap()))
                .collect();
            assert_eq!(
                state.menu_requests(command).unwrap(),
                expected,
                "menu routing in context {count}"
            );
        }
        assert!(output.next().is_none());
        count += 1;
    }
    assert_eq!(count, 1033, "the complete replay corpus is required");
}
