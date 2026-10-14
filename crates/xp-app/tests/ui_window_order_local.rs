use openxplane::{ui_state::RegisteredWindow, ui_window_order::Registry};
fn entries(classes: &[i32], tokens: &[u64]) -> Vec<RegisteredWindow> {
    classes
        .iter()
        .zip(tokens)
        .map(|(&classification, &token)| RegisteredWindow {
            classification,
            token,
            opacity_root_to_leaf: vec![1.0],
        })
        .collect()
}
#[test]
fn ties_and_duplicate_tokens_keep_native_order() {
    let mut r = Registry {
        entries: entries(&[2, 2, 0], &[1, 1, 2]),
        order: vec![0, 1, 2],
    };
    let index = r.extract(1).unwrap();
    assert_eq!(index, 0);
    r.insert(index, 0).unwrap();
    assert_eq!(r.order, [1, 0, 2]);
    let index = r.extract(1).unwrap();
    assert_eq!(index, 1);
    r.insert(index, 255).unwrap();
    assert_eq!(r.order, [1, 0, 2]);
    assert!(r.extract(99).is_err());
}
#[test]
#[ignore = "requires local synthetic original-code corpus via OPENXPLANE_UI_WINDOW_ORDER_VECTORS"]
fn native_registry_order() {
    let path = std::env::var("OPENXPLANE_UI_WINDOW_ORDER_VECTORS").expect("local corpus path");
    let data = std::fs::read_to_string(path).unwrap();
    let mut count = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        let parse_indices = |s: &str| {
            s.split(',')
                .map(|v| v.parse::<usize>().unwrap())
                .collect::<Vec<_>>()
        };
        let classes: Vec<i32> = v[3].split(',').map(|v| v.parse().unwrap()).collect();
        let tokens: Vec<u64> = v[4].split(',').map(|v| v.parse().unwrap()).collect();
        let mut r = Registry {
            entries: entries(&classes, &tokens),
            order: parse_indices(v[5]),
        };
        assert_eq!(v[6], "->");
        let index = r.extract(v[2].parse().unwrap()).unwrap();
        assert_eq!(index, v[7].parse::<usize>().unwrap(), "{line}");
        assert_eq!(r.order.len(), v[8].parse::<usize>().unwrap(), "{line}");
        r.insert(index, v[1].parse::<u32>().unwrap() as u8).unwrap();
        assert_eq!(r.order.len(), v[9].parse::<usize>().unwrap(), "{line}");
        assert_eq!(r.order, parse_indices(v[10]), "{line}");
        assert_eq!(v[11], "FA");
        count += 1;
    }
    assert_eq!(count, 700);
}
