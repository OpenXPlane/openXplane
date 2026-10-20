#[path = "support/ui_record_class_string_replay.rs"]
mod replay;
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_RECORD_CLASS_STRING_VECTORS"]
fn whole_derived_class_string_tree_reload_labels_growth_cleanup_and_final_shrink() {
    use std::io::BufRead;
    let file =
        std::fs::File::open(std::env::var("OPENXPLANE_UI_RECORD_CLASS_STRING_VECTORS").unwrap())
            .unwrap();
    let (mut derived, mut labels) = (0, 0);
    for row in std::io::BufReader::new(file).lines() {
        let row = row.unwrap();
        if !row.starts_with("D ") && !row.starts_with("L ") {
            continue;
        }
        replay::run_row(&row, &Default::default());
        if row.starts_with("D ") {
            derived += 1;
        } else {
            labels += 1;
        }
    }
    assert_eq!((derived, labels), (960, 60));
}
