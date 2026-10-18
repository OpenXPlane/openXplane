use openxplane::ui_context_refresh::{self, State};
#[path = "support/context_refresh_replay.rs"]
mod replay;
use replay::{Replay, hex, num, parse_links};
#[test]
#[ignore = "requires original local corpus via OPENXPLANE_UI_CONTEXT_REFRESH_VECTORS"]
fn native_whole_refresh_context_cli_numeric_edges_and_mutable_listeners() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_CONTEXT_REFRESH_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut cases = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        assert_eq!(v[14], "->");
        let next = parse_links(v[8]);
        let mut replay = Replay::from_fields(&v, v[18], &next);
        let mut s = State {
            image: hex(v[7]).try_into().unwrap(),
            active_global: replay.v[1],
            resource_index: 0,
            next,
        };
        ui_context_refresh::refresh(&mut s, &mut replay).unwrap();
        assert_eq!(s.image.to_vec(), hex(v[15]), "case {cases}");
        assert_eq!(s.active_global, num(v[16]));
        assert_eq!(replay::state(&s).split(';').next_back().unwrap(), v[17]);
        assert_eq!(replay.position, replay.event_count());
        cases += 1;
    }
    assert_eq!(cases, 972);
}
