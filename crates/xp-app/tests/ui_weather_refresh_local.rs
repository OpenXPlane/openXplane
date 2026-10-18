#[path = "support/ui_weather_replay.rs"]
mod replay;
#[test]
#[ignore = "requires complete original weather corpus via OPENXPLANE_UI_WEATHER_REFRESH_VECTORS"]
fn native_whole_weather_refresh_records_live_settings_aliases_and_listeners() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_WEATHER_REFRESH_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        replay::run_row(row, None, None, &Default::default());
        cases += 1;
    }
    assert_eq!(cases, 384);
}
