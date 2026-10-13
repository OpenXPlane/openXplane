use xp_dataref::key_event::{Action, Entry, route};

#[test]
#[ignore = "requires local original-code corpus; set OPENXPLANE_KEY_EVENT_VECTORS"]
fn native_key_vector_transitions_match() {
    let source = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_KEY_EVENT_VECTORS").expect("set OPENXPLANE_KEY_EVENT_VECTORS"),
    )
    .unwrap();
    let mut cases = 0;
    for line in source.lines().filter(|s| s.starts_with("E ")) {
        let (input, output) = line.split_once(" -> ").unwrap();
        let mut input = input.split_whitespace().skip(1);
        let key = input.next().unwrap().parse().unwrap();
        let flags = input.next().unwrap().parse().unwrap();
        let mut suppression = input.next().unwrap().parse().unwrap();
        let count: usize = input.next().unwrap().parse().unwrap();
        let mut entries: Vec<_> = (0..count)
            .map(|_| Entry {
                key: input.next().unwrap().parse().unwrap(),
                modifiers: input.next().unwrap().parse().unwrap(),
                command: input.next().unwrap().parse().unwrap(),
                active: input.next().unwrap().parse().unwrap(),
            })
            .collect();
        assert!(input.next().is_none());
        let calls = route(&mut entries, key, flags, &mut suppression);
        let mut output = output.split_whitespace();
        let expected_suppression: i32 = output.next().unwrap().parse().unwrap();
        assert_eq!(suppression, expected_suppression, "case {cases}");
        for entry in entries {
            let expected: i32 = output.next().unwrap().parse().unwrap();
            assert_eq!(entry.active, expected, "case {cases}");
        }
        let count: usize = output.next().unwrap().parse().unwrap();
        let expected: Vec<_> = (0..count)
            .map(|_| {
                let kind = output.next().unwrap();
                let command = output.next().unwrap().parse().unwrap();
                match kind {
                    "0" => Action::Begin(command),
                    "1" => Action::End(command),
                    _ => panic!("bad action"),
                }
            })
            .collect();
        assert_eq!(calls, expected, "case {cases}");
        assert!(output.next().is_none());
        cases += 1;
    }
    assert_eq!(cases, 2728, "the complete local corpus is required");
}
