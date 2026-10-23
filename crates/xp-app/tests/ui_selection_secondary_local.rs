#[path = "support/ui_selection_secondary_replay.rs"]
mod replay;
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_SELECTION_SECONDARY_VECTORS"]
fn whole_secondary_with_actual_primary_membership_getters_equality_and_gate() {
    use std::io::BufRead;
    let f =
        std::fs::File::open(std::env::var("OPENXPLANE_UI_SELECTION_SECONDARY_VECTORS").unwrap())
            .unwrap();
    let mut counts = [0; 4];
    for (index, line) in std::io::BufReader::new(f).lines().enumerate() {
        let row = line.unwrap();
        if !row.starts_with("V ") {
            continue;
        }
        let result = std::panic::catch_unwind(|| replay::run_row(&row));
        assert!(result.is_ok(), "original corpus line {}", index + 1);
        let i = match row.split_whitespace().nth(1).unwrap() {
            "S" => 0,
            "P" => 1,
            "E" => 2,
            "G" => 3,
            _ => panic!("mode"),
        };
        counts[i] += 1;
    }
    assert_eq!(counts, [4968, 72, 72, 2016]);
}
