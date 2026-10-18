#[path = "support/ui_selection_alternate_replay.rs"]
mod replay;
#[test]
#[ignore = "requires original alternate corpus via OPENXPLANE_UI_SELECTION_ALTERNATE_VECTORS"]
fn native_whole_selection_alternate_tags_vectors_fallbacks_and_live_inputs() {
    let corpus = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_SELECTION_ALTERNATE_VECTORS").unwrap(),
    )
    .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        replay::run_row(row, None, None, &Default::default());
        cases += 1;
    }
    assert_eq!(cases, 528);
}
