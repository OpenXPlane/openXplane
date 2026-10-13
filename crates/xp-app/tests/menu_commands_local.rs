use openxplane::menu_commands::{Command, Request, requests};

#[test]
#[ignore = "requires local original-handler trace; set OPENXPLANE_MENU_COMMAND_VECTORS"]
fn original_menu_routing_requests_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_MENU_COMMAND_VECTORS")
            .expect("set OPENXPLANE_MENU_COMMAND_VECTORS"),
    )
    .unwrap();
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("M ")) {
        let (input, output) = line.split_once(" -> ").unwrap();
        let fields: Vec<_> = input.split_whitespace().collect();
        let command = match fields[1] {
            "29" => Command::ToggleMainMenu,
            "30" => Command::ToggleSettings,
            _ => panic!("unknown command"),
        };
        let expected: Vec<_> = output
            .split_whitespace()
            .map(|name| match name {
                "QueryMainMenu" => Request::QueryMainMenu,
                "QueryState" => Request::QueryState,
                "CloseMainMenu" => Request::CloseMainMenu,
                "EnsureMainMenu" => Request::EnsureMainMenu,
                "Refresh" => Request::Refresh,
                "ToggleSettingsWindow" => Request::ToggleSettingsWindow,
                _ => panic!("unknown request"),
            })
            .collect();
        assert_eq!(
            requests(command, fields[2] == "1", fields[3] == "1"),
            expected
        );
        count += 1;
    }
    assert_eq!(count, 8);
}
