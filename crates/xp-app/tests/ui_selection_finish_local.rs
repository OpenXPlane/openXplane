#[path = "support/ui_selection_finish_replay.rs"]
mod replay;
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_SELECTION_FINISH_VECTORS"]
fn whole_original_finish_vector_insert_live_fields_scalar_bits_and_cleanup() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_SELECTION_FINISH_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        replay::run_row(row, None, &Default::default());
        cases += 1;
    }
    assert_eq!(cases, 560);
}

#[test]
#[ignore = "requires instruction corpus via OPENXPLANE_UI_SELECTION_SSE_VECTORS; local macOS run uses Rosetta"]
fn default_scalar_values_match_independent_x86_instruction_oracle() {
    use openxplane::ui_selection_finish::{ScalarOp, scalar_sse};
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_SELECTION_SSE_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("Q ")) {
        let v = row.split_whitespace().collect::<Vec<_>>();
        assert_eq!(v.len(), 6);
        assert_eq!(v[4], "->");
        let op = match v[1] {
            "0" => ScalarOp::Multiply,
            "1" => ScalarOp::Divide,
            "2" => ScalarOp::Add,
            _ => panic!("unknown op"),
        };
        assert_eq!(
            scalar_sse(v[2].parse().unwrap(), v[3].parse().unwrap(), op),
            v[5].parse::<u32>().unwrap(),
            "{row}"
        );
        cases += 1;
    }
    assert_eq!(cases, 1452);
}
