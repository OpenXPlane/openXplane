#[path = "support/ui_selection_finish_replay.rs"]
mod finish_replay;
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

#[test]
#[ignore = "requires composed local corpus via OPENXPLANE_UI_SELECTION_RESET_INITIAL_VECTORS"]
fn whole_reset_with_actual_initial_callback_and_read_only_helpers() {
    let corpus = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_SELECTION_RESET_INITIAL_VECTORS").unwrap(),
    )
    .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        assert!(row.split_whitespace().nth(7).unwrap().starts_with("K~C@"));
        replay::run_row(row, None, None, &Default::default());
        cases += 1;
    }
    assert_eq!(cases, 528);
}

#[test]
#[ignore = "requires recursive local corpus via OPENXPLANE_UI_SELECTION_RESET_RECURSIVE_VECTORS"]
fn whole_reset_callback_and_actual_nested_reset_restore_caller_frames() {
    let corpus = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_SELECTION_RESET_RECURSIVE_VECTORS").unwrap(),
    )
    .unwrap();
    let (mut cases, mut recursive) = (0, 0);
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        assert!(row.split_whitespace().nth(7).unwrap().starts_with("K~C@"));
        recursive += usize::from(row.contains("%R~"));
        replay::run_row(row, None, None, &Default::default());
        cases += 1;
    }
    assert_eq!((cases, recursive), (528, 55));
}

#[test]
#[ignore = "requires catalog arithmetic corpus via OPENXPLANE_UI_SELECTION_RESET_CATALOG_VECTORS"]
fn original_catalog_signed_division_low32_last_index_and_fallback() {
    let corpus = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_SELECTION_RESET_CATALOG_VECTORS").unwrap(),
    )
    .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        replay::run_row(row, None, None, &Default::default());
        cases += 1;
    }
    assert_eq!(cases, 504);
}

#[test]
#[ignore = "requires composed local corpus via OPENXPLANE_UI_SELECTION_RESET_FINISH_VECTORS"]
fn whole_recursive_reset_callback_and_real_finish_publish_inherited_state() {
    use std::io::BufRead;
    let file =
        std::fs::File::open(std::env::var("OPENXPLANE_UI_SELECTION_RESET_FINISH_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in std::io::BufReader::new(file).lines() {
        let row = row.unwrap();
        if !row.starts_with("C ") {
            continue;
        }
        let events = row.split_whitespace().nth(7).unwrap();
        assert!(events.starts_with("K~C@"));
        assert!(events.split('|').next_back().unwrap().starts_with("F~"));
        replay::run_row(&row, None, None, &Default::default());
        cases += 1;
    }
    assert_eq!(cases, 528);
}
