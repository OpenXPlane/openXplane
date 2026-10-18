use openxplane::{
    ui_indexed_records::{self, Kind},
    ui_payload_move,
};
#[test]
#[ignore = "requires original arithmetic corpus via OPENXPLANE_UI_INDEXED_RECORDS_VECTORS"]
fn native_record_accessors_signed_low_word_and_wrapping_addresses() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_INDEXED_RECORDS_VECTORS").unwrap())
            .unwrap();
    let mut cases = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        let v = row.split_whitespace().collect::<Vec<_>>();
        assert_eq!(v.len(), 6);
        assert_eq!(v[4], "->");
        let kind = if v[1] == "0" {
            Kind::Resource
        } else {
            Kind::Settings
        };
        let base = v[2].parse::<u64>().unwrap();
        let index = v[3].parse::<u64>().unwrap();
        let expected = v[5].parse::<u64>().unwrap();
        let mut memory = ui_payload_move::State::default();
        memory
            .blocks
            .insert(kind.table(), base.to_le_bytes().to_vec());
        assert_eq!(
            ui_indexed_records::lookup(&memory, kind, index).unwrap(),
            expected
        );
        assert_eq!(memory.read(kind.table(), 8).unwrap(), base.to_le_bytes());
        cases += 1;
    }
    assert_eq!(cases, 872);
}
