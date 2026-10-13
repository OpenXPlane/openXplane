use openxplane::ui_input::{KeyInput, dispatch_focused};

#[test]
#[ignore = "requires local native focused-input replay; set OPENXPLANE_UI_INPUT_VECTORS"]
fn native_focused_input_matches_filter_return_and_call_order() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_UI_INPUT_VECTORS").expect("set OPENXPLANE_UI_INPUT_VECTORS"),
    )
    .unwrap();
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("I ")) {
        let (input, output) = line.split_once(" -> ").unwrap();
        let fields: Vec<_> = input.split_whitespace().collect();
        let mask = fields[2].parse().unwrap();
        let focus = fields[1] == "1";
        let mask = std::cell::Cell::new(mask);
        let event = KeyInput {
            character: fields[3].parse().unwrap(),
            virtual_key: fields[4].parse().unwrap(),
            flags: fields[5].parse().unwrap(),
        };
        let child_result: i32 = fields[6].parse().unwrap();
        let own_result: u8 = fields[7].parse().unwrap();
        let update: i64 = fields[8].parse().unwrap();
        let log = std::cell::RefCell::new(Vec::new());
        let mut child = |observed| {
            assert_eq!(observed, event);
            log.borrow_mut().push("child");
            if update >= 0 {
                mask.set(update as u32);
            }
            child_result as u8
        };
        let own = |observed| {
            assert_eq!(observed, event);
            log.borrow_mut().push("own");
            own_result
        };
        let child: Option<&mut dyn FnMut(KeyInput) -> u8> = if child_result < 0 {
            None
        } else {
            Some(&mut child)
        };
        let actual = dispatch_focused(focus, || mask.get(), event, child, own).unwrap();
        let mut output = output.split_whitespace();
        let expected: u8 = output.next().unwrap().parse().unwrap();
        assert_eq!(actual, expected, "case {count}");
        let expected_mask: u32 = output.next().unwrap().parse().unwrap();
        assert_eq!(mask.get(), expected_mask, "mask in case {count}");
        let n: usize = output.next().unwrap().parse().unwrap();
        let expected: Vec<_> = (0..n).map(|_| output.next().unwrap()).collect();
        assert_eq!(*log.borrow(), expected, "case {count}");
        assert!(output.next().is_none());
        count += 1;
    }
    assert_eq!(count, 7120);
}
