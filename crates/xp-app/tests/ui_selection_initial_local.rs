#[path = "support/ui_selection_finish_replay.rs"]
mod finish_replay;
#[path = "support/ui_selection_initial_replay.rs"]
mod replay;
use openxplane::{ui_payload_move, ui_selection_initial};
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_SELECTION_INITIAL_VECTORS"]
fn whole_original_initial_callback_float_alias_type_transition_and_recursive_reset_requests() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_SELECTION_INITIAL_VECTORS").unwrap())
            .unwrap();
    let (mut cases, mut pairs) = (0, 0);
    for row in corpus.lines() {
        if row.starts_with("C ") {
            replay::run_row(row, None, &Default::default());
            cases += 1;
        }
        if row.starts_with("Q ") {
            let v = row.split_whitespace().collect::<Vec<_>>();
            assert_eq!(v.len(), 7);
            assert_eq!(v[4], "->");
            let receiver = v[1].parse::<u64>().unwrap();
            let mut image = vec![v[2].parse::<u8>().unwrap(); 0x90];
            image[0x68] = v[3].parse().unwrap();
            let mut mem = ui_payload_move::State::default();
            mem.blocks.insert(receiver, image.clone());
            assert_eq!(
                u8::from(ui_selection_initial::tag_is_three(&mem, receiver).unwrap()),
                v[5].parse::<u8>().unwrap()
            );
            assert_eq!(
                ui_selection_initial::payload_if_three(&mem, receiver).unwrap(),
                v[6].parse::<u64>().unwrap()
            );
            assert_eq!(mem.read(receiver, image.len()).unwrap(), image);
            pairs += 1;
        }
    }
    assert_eq!((cases, pairs), (712, 768));
}

#[test]
#[ignore = "requires composed comparison corpus via OPENXPLANE_UI_SELECTION_INITIAL_COMPARE_VECTORS"]
fn whole_initial_with_actual_record_comparison_before_and_after_finish() {
    use std::io::BufRead;
    let file = std::fs::File::open(
        std::env::var("OPENXPLANE_UI_SELECTION_INITIAL_COMPARE_VECTORS").unwrap(),
    )
    .unwrap();
    let (mut count, mut predicates) = (0, 0);
    for row in std::io::BufReader::new(file).lines() {
        let row = row.unwrap();
        if !row.starts_with("C ") {
            continue;
        }
        predicates += row
            .split_whitespace()
            .nth(8)
            .unwrap()
            .split('|')
            .filter(|e| e.starts_with("P~"))
            .count();
        replay::run_row(&row, None, &Default::default());
        count += 1;
    }
    assert_eq!((count, predicates), (712, 1143));
}
