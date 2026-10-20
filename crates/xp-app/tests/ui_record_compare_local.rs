#[path = "support/ui_record_compare_replay.rs"]
mod replay;
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_RECORD_COMPARE_VECTORS"]
fn whole_record_equality_wrapper_string_order_aliases_and_reverse_cleanup() {
    use std::io::BufRead;
    let file = std::fs::File::open(std::env::var("OPENXPLANE_UI_RECORD_COMPARE_VECTORS").unwrap())
        .unwrap();
    let mut count = 0;
    for row in std::io::BufReader::new(file).lines() {
        let row = row.unwrap();
        if !row.starts_with("C ") {
            continue;
        }
        replay::run_row(&row, &Default::default());
        count += 1;
    }
    assert_eq!(count, 1500);
}
