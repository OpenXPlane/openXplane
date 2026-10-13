use xp_dataref::command_gate::CommandGate;

#[test]
#[ignore = "requires local command gate replay; set OPENXPLANE_COMMAND_GATE_VECTORS"]
fn native_command_exemptions_and_mask_query_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_COMMAND_GATE_VECTORS")
            .expect("set OPENXPLANE_COMMAND_GATE_VECTORS"),
    )
    .unwrap();
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("G ")) {
        let (input, output) = line.split_once(" -> ").unwrap();
        let mut v = input.split_whitespace().skip(1);
        let command = v.next().unwrap().parse().unwrap();
        let source = v.next().unwrap().parse().unwrap();
        let default_mask = v.next().unwrap().parse().unwrap();
        let n: usize = v.next().unwrap().parse().unwrap();
        let mut gate = CommandGate {
            default_mask,
            ..CommandGate::default()
        };
        for _ in 0..n {
            gate.by_command.insert(
                v.next().unwrap().parse().unwrap(),
                v.next().unwrap().parse().unwrap(),
            );
        }
        assert!(v.next().is_none());
        assert_eq!(
            gate.is_blocked(command, source),
            output == "1",
            "case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 6418);
}
