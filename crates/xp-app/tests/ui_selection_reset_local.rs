#[path = "support/ui_selection_reset_replay.rs"]
mod replay;
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_SELECTION_RESET_VECTORS"]
fn whole_original_reset_catalog_strings_variants_live_listeners_and_tail_finish() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_SELECTION_RESET_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        replay::run_row(row, None, None, &Default::default());
        cases += 1;
    }
    assert_eq!(cases, 528);
}
