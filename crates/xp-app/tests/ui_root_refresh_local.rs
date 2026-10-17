use openxplane::{
    ui_callback_links,
    ui_callback_list::List,
    ui_root_refresh::{self, Context},
};
use std::collections::BTreeMap;
#[path = "support/ui_root_replay.rs"]
mod replay_support;
use replay_support::*;
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_ROOT_REFRESH_VECTORS"]
fn whole_native_root_refresh_resource_cli_fallbacks_vectors_and_retained_callbacks() {
    replay_corpus("OPENXPLANE_UI_ROOT_REFRESH_VECTORS", false);
}
#[test]
#[ignore = "requires local combined corpus via OPENXPLANE_UI_REFRESH_CHAIN_VECTORS"]
fn native_root_nested_refresh_and_payload_chain() {
    replay_corpus("OPENXPLANE_UI_REFRESH_CHAIN_VECTORS", true);
}
fn replay_corpus(env: &str, native_nested: bool) {
    let corpus = std::fs::read_to_string(std::env::var(env).unwrap()).unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        let v: Vec<_> = row.split_whitespace().collect();
        assert_eq!(v.len(), 13);
        assert_eq!(v[8], "->");
        let id = num(v[1]);
        let stack = num(v[2]);
        let mut state = parse_snapshot(v[3], id, stack);
        state
            .memory
            .blocks
            .insert(if native_nested { stack - 0x200 } else { stack }, hex(v[4]));
        let mut callbacks = parse_graph(v[7]);
        let mut parents = BTreeMap::from([(
            id + 0x1e0,
            List {
                sentinel: num(v[5]),
                entries: if v[6] == "-" {
                    vec![]
                } else {
                    v[6].split(',').map(num).collect()
                },
            },
        )]);
        let mut replay = Replay {
            events: v[12].split('|').map(str::to_string).collect(),
            position: 0,
            native_nested,
        };
        let mut image = state.memory.read(id, 0x3e0).unwrap();
        ui_root_refresh::refresh_image(
            &mut image,
            &mut state,
            Context {
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut replay,
        )
        .unwrap();
        assert_eq!(image, state.memory.read(id, 0x3e0).unwrap());
        compare(&snapshot(&state), v[9], &format!("final case{cases}"));
        assert_eq!(graph(&callbacks), v[10]);
        assert_eq!(list(&parents[&(id + 0x1e0)]), v[11]);
        assert_eq!(replay.position, replay.events.len());
        cases += 1;
    }
    assert_eq!(cases, 118);
}

#[test]
#[ignore = "requires local whole constructor corpus via OPENXPLANE_UI_ROOT_CONSTRUCT_CHAIN_VECTORS"]
fn native_whole_root_constructor_both_refreshes_and_payload_chain() {
    let corpus = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_ROOT_CONSTRUCT_CHAIN_VECTORS").unwrap(),
    )
    .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        run_constructor_row(
            row,
            &mut ui_callback_links::State::default(),
            &mut BTreeMap::new(),
        );
        cases += 1;
    }
    assert_eq!(cases, 118);
}
