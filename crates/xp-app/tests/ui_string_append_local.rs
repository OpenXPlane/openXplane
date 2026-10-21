#[path = "support/ui_string_append_replay.rs"]
mod replay;
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_STRING_APPEND_VECTORS"]
fn whole_append_growth_aliasing_metadata_reload_and_pointer_publication() {
    use std::io::BufRead;
    let file =
        std::fs::File::open(std::env::var("OPENXPLANE_UI_STRING_APPEND_VECTORS").unwrap()).unwrap();
    let (mut append, mut grow) = (0, 0);
    for (index, row) in std::io::BufReader::new(file).lines().enumerate() {
        let row = row.unwrap();
        if !row.starts_with("S ") {
            continue;
        }
        let result = std::panic::catch_unwind(|| replay::run_row(&row));
        assert!(result.is_ok(), "original corpus line {}", index + 1);
        if row.starts_with("S A ") {
            append += 1;
        } else {
            grow += 1;
        }
    }
    assert_eq!((append, grow), (1023, 2913));
}
