use openxplane::ui_state::{RegisteredWindow, WindowState};

#[test]
#[ignore = "requires local native collection/query replay; set OPENXPLANE_UI_CAPTURE_VECTORS"]
fn native_classification_collection_and_visibility_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_UI_CAPTURE_VECTORS")
            .expect("set OPENXPLANE_UI_CAPTURE_VECTORS"),
    )
    .unwrap();
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("C ")) {
        let (input, output) = line.split_once(" -> ").unwrap();
        let mut fields = input.split_whitespace().skip(1);
        let n: usize = fields.next().unwrap().parse().unwrap();
        let mut state = WindowState::default();
        for _ in 0..n {
            if fields.next().unwrap() == "0" {
                state.registered.push(None);
                continue;
            }
            let token = fields.next().unwrap().parse().unwrap();
            let classification = fields.next().unwrap().parse().unwrap();
            let depth: usize = fields.next().unwrap().parse().unwrap();
            let opacity_root_to_leaf = (0..depth)
                .map(|_| f32::from_bits(u32::from_str_radix(fields.next().unwrap(), 16).unwrap()))
                .collect();
            state.registered.push(Some(RegisteredWindow {
                token,
                classification,
                opacity_root_to_leaf,
            }));
        }
        assert!(fields.next().is_none());
        let mut fields = output.split_whitespace();
        assert_eq!(
            state.visible_classification(2).unwrap(),
            fields.next().unwrap() == "1",
            "case {count}"
        );
        let n: usize = fields.next().unwrap().parse().unwrap();
        let expected: Vec<usize> = (0..n)
            .map(|_| fields.next().unwrap().parse().unwrap())
            .collect();
        let selected = state.classified_indices(2);
        assert_eq!(selected, expected, "case {count}");
        assert!(fields.next().is_none());
        count += 1;
    }
    assert_eq!(count, 1054);
}
