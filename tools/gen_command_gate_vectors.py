#!/usr/bin/env python3
"""Execute entire command mask query and existing-key map accessor without stubs."""
import random
import struct
import sys
from emulate_xp import Emulator
from extract_commands import detailed_records, REFERENCE_SHA256


def cases():
    # Every ID in the validated 3138-record native command catalogue.
    for command in range(3138):
        yield command, 0, 0xffffffff, []
    ids = [*range(-2, 67), *range(0xb40, 0xb50), *range(0xb87, 0xc14), -2147483648, 2147483647]
    for command in ids:
        for default, source in [(0, 0), (1, 0), (1, 1), (3, 1), (0xffffffff, 0xffffffff)]:
            yield command, source, default, []
            yield command, source, default, [(command, default ^ 0xffffffff)]
    rng = random.Random(2026101306)
    for _ in range(1000):
        command = rng.choice(ids)
        keys = sorted(set([command, *rng.choices(ids, k=rng.randrange(0, 16))]))
        yield command, rng.getrandbits(32), rng.getrandbits(32), [(k, rng.getrandbits(32)) for k in keys]


def main(path):
    detailed_records(path)
    e = Emulator(path)
    persistent = e.heap_top
    print('# reference_sha256', REFERENCE_SHA256)
    print('# full 0x140c63f00 and found-key accessor 0x14071d590 unchanged, no stubs')
    print('# G command source default n [key mask]* -> blocked')
    count = 0
    for command, source, default, mapping in cases():
        e.heap_top = persistent
        head = e.alloc(40)
        e.write(head, bytes(40)); e.write(head + 25, b'\1')
        def tree(rows, parent):
            if not rows: return head
            i = len(rows) // 2
            key, mask = rows[i]
            node = e.alloc(40)
            e.write(node, bytes(40))
            e.write_u64(node + 8, parent)
            e.write_u64(node, tree(rows[:i], node))
            e.write_u64(node + 16, tree(rows[i+1:], node))
            e.write(node + 28, struct.pack('<iI', key, mask))
            return node
        e.write_u64(head + 8, tree(mapping, head))
        e.write_u64(0x145777510, head)
        e.write_u32(0x145777508, default)
        result = e.call(0x140c63f00, ints=[command, source], max_instructions=10000) & 255
        print('G', command, source, default, len(mapping), *(v for row in mapping for v in row), '->', result)
        count += 1
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
