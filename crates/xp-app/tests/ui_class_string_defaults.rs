//! Exercise production class/label/append/growth defaults with only CRT primitives.
use openxplane::{ui_payload_move, ui_record_class_string};
#[test]
fn nonempty_category_tree_uses_recovered_string_bodies_and_releases_temporary_storage() {
    let (record, out, head, first, second, stack) =
        (0x10000u64, 0x20000, 0x30000, 0x31000, 0x32000, 0x100000);
    let mut m = ui_payload_move::State::default();
    for (p, n) in [
        (record, 0x198),
        (out, 32),
        (head, 40),
        (first, 40),
        (second, 40),
    ] {
        m.blocks.insert(p, vec![0; n]);
    }
    m.blocks.insert(0x142505110, vec![b'/']);
    m.write(record + 0x180, &head.to_le_bytes()).unwrap();
    m.write(head + 8, &first.to_le_bytes()).unwrap();
    m.write(head + 0x19, &[1]).unwrap();
    for (node, key, right) in [(first, 1i32, second), (second, 8, head)] {
        m.write(node, &head.to_le_bytes()).unwrap();
        m.write(node + 0x10, &right.to_le_bytes()).unwrap();
        m.write(node + 0x1c, &key.to_le_bytes()).unwrap();
    }
    let original = m.blocks.keys().copied().collect::<Vec<_>>();
    let (mut allocations, mut frees, mut moves) = (0u64, 0, 0);
    assert_eq!(
        ui_record_class_string::build_with(&mut m, stack, record, out, |m, f, a| {
            match f {
                0x1422e7bf0 => {
                    allocations += 1;
                    let p = 0x50000 + allocations * 0x1000;
                    assert!(m.blocks.insert(p, vec![0xa5; a[0] as usize]).is_none());
                    Ok(p)
                }
                0x1423c0280 => {
                    moves += 1;
                    if a[2] != 0 {
                        let bytes = m.read(a[1], a[2] as usize)?;
                        m.write(a[0], &bytes)?;
                    }
                    Ok(a[0])
                }
                0x1422e7c2c => {
                    frees += 1;
                    let b = m
                        .blocks
                        .remove(&a[0])
                        .expect("free must own live allocation");
                    assert_eq!(b.len() as u64, a[1]);
                    Ok(u64::MAX)
                }
                _ => Err("opaque string service must not be requested"),
            }
        })
        .unwrap(),
        out
    );
    let q = |p| u64::from_le_bytes(m.read(p, 8).unwrap().try_into().unwrap());
    assert_eq!(q(out + 16), 32);
    assert_eq!(q(out + 24), 47);
    assert_eq!(
        m.read(q(out), 33).unwrap(),
        b"General Aviation/Science Fiction\0"
    );
    assert_eq!((allocations, frees, moves), (4, 3, 6));
    assert_eq!(m.blocks.len(), original.len() + 1);
    for p in original {
        assert!(m.blocks.contains_key(&p));
    }
    assert!(
        !m.blocks
            .keys()
            .any(|p| *p >= stack - 0x100 && *p <= stack + 0x100)
    );
}
