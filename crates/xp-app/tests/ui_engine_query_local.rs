use openxplane::{ui_engine_query, ui_payload_move};
#[test]
#[ignore = "requires local original corpus via OPENXPLANE_UI_ENGINE_QUERY_VECTORS"]
fn native_read_only_query_all_byte_values_and_full_return_register() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_ENGINE_QUERY_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        let v = row.split_whitespace().collect::<Vec<_>>();
        assert_eq!(v.len(), 6);
        assert_eq!(v[4], "->");
        let receiver = v[1].parse::<u64>().unwrap();
        let mut image = vec![v[2].parse::<u8>().unwrap(); 0x600];
        image[0x525] = v[3].parse().unwrap();
        let mut memory = ui_payload_move::State::default();
        memory.blocks.insert(receiver, image.clone());
        assert_eq!(
            ui_engine_query::query_at(&memory, receiver).unwrap(),
            v[5].parse::<u64>().unwrap()
        );
        assert_eq!(memory.read(receiver, image.len()).unwrap(), image);
        cases += 1;
    }
    assert_eq!(cases, 768);
}
