#[path = "support/ui_selection_changed_replay.rs"]
mod replay;
#[test]
#[ignore = "requires original notification corpus via OPENXPLANE_UI_SELECTION_CHANGED_VECTORS"]
fn native_selection_change_dirty_and_live_callback_links() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_SELECTION_CHANGED_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        replay::run_row(row, None, None);
        cases += 1;
    }
    assert_eq!(cases, 348);
}
