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

#[test]
#[ignore = "requires composed local corpus via OPENXPLANE_UI_RECORD_COMPARE_CLASS_VECTORS"]
fn whole_equality_with_actual_class_tree_label_and_resize_callers() {
    use std::io::BufRead;
    let file =
        std::fs::File::open(std::env::var("OPENXPLANE_UI_RECORD_COMPARE_CLASS_VECTORS").unwrap())
            .unwrap();
    let (mut count, mut derived) = (0, 0);
    for row in std::io::BufReader::new(file).lines() {
        let row = row.unwrap();
        if !row.starts_with("C ") {
            continue;
        }
        derived += row
            .split_whitespace()
            .nth(9)
            .unwrap()
            .split('|')
            .filter(|e| e.starts_with("D~"))
            .count();
        replay::run_row(&row, &Default::default());
        count += 1;
    }
    assert_eq!((count, derived), (1500, 1458));
}

#[test]
#[ignore = "requires local actual-string corpus via OPENXPLANE_UI_RECORD_COMPARE_NATIVE_STRINGS_VECTORS"]
fn whole_equality_with_actual_classes_append_growth_and_live_first_output() {
    use std::io::BufRead;
    let file = std::fs::File::open(
        std::env::var("OPENXPLANE_UI_RECORD_COMPARE_NATIVE_STRINGS_VECTORS").unwrap(),
    )
    .unwrap();
    let (mut count, mut derived, mut strings) = (0, 0, 0);
    for (index, row) in std::io::BufReader::new(file).lines().enumerate() {
        let row = row.unwrap();
        if !row.starts_with("C ") {
            continue;
        }
        derived += row
            .split_whitespace()
            .nth(9)
            .unwrap()
            .split('|')
            .filter(|event| event.starts_with("D~"))
            .count();
        strings += replay::actual_strings_in_row(&row);
        let result = std::panic::catch_unwind(|| replay::run_row(&row, &Default::default()));
        assert!(result.is_ok(), "original corpus line {}", index + 1);
        count += 1;
    }
    assert_eq!((count, derived, strings), (1500, 1458, 8932));
}
