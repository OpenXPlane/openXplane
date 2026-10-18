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

#[test]
#[ignore = "requires actual predicate corpus via OPENXPLANE_UI_FIFTH_GATE_VECTORS"]
fn native_fifth_refresh_with_actual_payload_availability_gate() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_FIFTH_GATE_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        replay::run_native_row(row, None, None, &Default::default());
        cases += 1;
    }
    assert_eq!(cases, 528);
}

#[test]
#[ignore = "requires real table-accessor corpus via OPENXPLANE_UI_FIFTH_TABLES_VECTORS"]
fn native_fifth_refresh_with_actual_signed_record_tables() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_FIFTH_TABLES_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        replay::run_tables_row(row, None, None, &Default::default());
        cases += 1;
    }
    assert_eq!(cases, 528);
}

#[test]
#[ignore = "requires actual query/table corpus via OPENXPLANE_UI_FIFTH_ENGINE_VECTORS"]
fn native_fifth_refresh_with_actual_read_only_query_and_tables() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_FIFTH_ENGINE_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        replay::run_engine_row(row, None, None, &Default::default());
        cases += 1;
    }
    assert_eq!(cases, 528);
}
