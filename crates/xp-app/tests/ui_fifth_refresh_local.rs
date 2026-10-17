#[path = "support/ui_fifth_replay.rs"]
mod replay;
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_FIFTH_REFRESH_VECTORS"]
fn native_whole_fifth_refresh_hash_clear_iterators_cleanup_and_live_listeners() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_FIFTH_REFRESH_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        replay::run_row(row, None, None);
        cases += 1;
    }
    assert_eq!(cases, 528);
}
