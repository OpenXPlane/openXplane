//! Production equality/class/string chain with actual bytes and owned CRT storage.
use openxplane::{ui_payload_move, ui_record_compare};
#[test]
fn nonempty_comparison_defaults_keep_both_strings_live_and_release_all_temporaries() {
    for different_class in [false, true] {
        let (left, right, head, first, second, other, stack) = (
            0x10000u64, 0x20000, 0x30000, 0x31000, 0x32000, 0x33000, 0x100000,
        );
        let mut m = ui_payload_move::State::default();
        for (p, n) in [
            (left, 0x198),
            (right, 0x198),
            (head, 40),
            (first, 40),
            (second, 40),
            (other, 40),
        ] {
            m.blocks.insert(p, vec![0; n]);
        }
        m.blocks.insert(0x142505110, vec![b'/']);
        for p in [head, other] {
            m.write(p + 0x19, &[1]).unwrap();
        }
        m.write(head + 8, &first.to_le_bytes()).unwrap();
        m.write(other + 8, &first.to_le_bytes()).unwrap();
        // Separate right sentinel can stop key8 lookup without changing key1.
        let one_node = 0x34000u64;
        m.blocks.insert(one_node, vec![0; 40]);
        for (node, nil, key, next) in [
            (first, head, 1i32, second),
            (second, head, 8, head),
            (one_node, other, 1, other),
        ] {
            m.write(node, &nil.to_le_bytes()).unwrap();
            m.write(node + 0x10, &next.to_le_bytes()).unwrap();
            m.write(node + 0x1c, &key.to_le_bytes()).unwrap();
        }
        m.write(other + 8, &one_node.to_le_bytes()).unwrap();
        for (p, tree) in [
            (left, head),
            (right, if different_class { other } else { head }),
        ] {
            for offset in [0, 0x20, 0x40, 0x60, 0x80, 0xa8, 0xc8, 0xe8, 0x108] {
                m.write(p + offset, b"x\0y").unwrap();
                m.write(p + offset + 16, &3u64.to_le_bytes()).unwrap();
                m.write(p + offset + 24, &15u64.to_le_bytes()).unwrap();
            }
            m.write(p + 0x180, &tree.to_le_bytes()).unwrap();
        }
        let original = m.blocks.keys().copied().collect::<Vec<_>>();
        let (mut allocations, mut frees, mut comparisons) = (0u64, 0u64, 0);
        let mut compared_distinct_heaps = false;
        let different =
            ui_record_compare::different_with(&mut m, stack, left, right, |m, f, a| match f {
                0x1422e7bf0 => {
                    allocations += 1;
                    let p = 0x50000 + allocations * 0x1000;
                    assert!(m.blocks.insert(p, vec![0xa5; a[0] as usize]).is_none());
                    Ok(p)
                }
                0x1423c0280 => {
                    if a[2] != 0 {
                        let b = m.read(a[1], a[2] as usize)?;
                        m.write(a[0], &b)?;
                    }
                    Ok(a[0])
                }
                0x1422e7c2c => {
                    let b = m.blocks.remove(&a[0]).expect("free of non-owned storage");
                    assert_eq!(b.len() as u64, a[1]);
                    frees += 1;
                    Ok(u64::MAX)
                }
                0x1423c0180 => {
                    comparisons += 1;
                    if a[2] > 15 {
                        assert_ne!(a[0], a[1]);
                        assert!(m.blocks.contains_key(&a[0]) && m.blocks.contains_key(&a[1]));
                        compared_distinct_heaps = true;
                    }
                    let different = m.read(a[0], a[2] as usize)? != m.read(a[1], a[2] as usize)?;
                    m.write(left + 0xa4, &[0x7e])?;
                    Ok(u64::from(different))
                }
                _ => Err("opaque record/class/string endpoint must not be requested"),
            })
            .unwrap();
        assert_eq!(different, different_class);
        assert_eq!(
            (allocations, frees),
            if different_class { (6, 6) } else { (8, 8) }
        );
        assert_eq!(comparisons, if different_class { 3 } else { 7 });
        assert_eq!(compared_distinct_heaps, !different_class);
        assert_eq!(m.blocks.keys().copied().collect::<Vec<_>>(), original);
        assert_eq!(m.read(left + 0xa4, 1).unwrap(), [0x7e]);
    }
}
