#!/usr/bin/env python3
"""Replay native classification collection and visible-layer query; only alloc/free replaced."""
import random
import struct
import sys
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256, detailed_records
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX


def bits(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def install_allocator(emu):
    def alloc(e):
        size = e.reg(UC_X86_REG_RCX)
        assert 0 < size < 0x10000
        e.uc.reg_write(UC_X86_REG_RAX, e.alloc(size, align=32))
    emu.stubs[0x1422e7bf0] = alloc
    emu.stubs[0x1422e7c2c] = lambda e: None


def build_registry(emu, manager, implementation, registered):
    emu.write_u64(manager + 0x128, implementation)
    head = emu.alloc(24)
    nodes = [emu.alloc(24) for _ in registered]
    emu.write_u64(head, nodes[0] if nodes else head)
    emu.write_u64(implementation + 0x150, head)
    items = []
    for index, entry in enumerate(registered):
        node = nodes[index]
        emu.write(node, bytes(24))
        emu.write_u64(node, nodes[index + 1] if index + 1 < len(nodes) else head)
        if entry is None:
            items.append(0)
            continue
        token, classification, path = entry
        item = emu.alloc(0x178)
        emu.write(item, bytes(0x178))
        emu.write_u64(node + 16, item)
        emu.write_u64(item + 0x138, token)
        emu.write_u32(item + 0x170, classification)
        parent = 0
        for alpha in path:
            widget = emu.alloc(0x70)
            emu.write(widget, bytes(0x70))
            emu.write_f32(widget + 0x58, alpha)
            emu.write_u64(widget + 0x60, parent)
            parent = widget
        emu.write_u64(item + 0x140, parent)
        items.append(item)
    return items


def cases():
    yield []
    for classification in [0, 1, 2, 3, -1, -2147483648, 2147483647]:
        for path in [[0.], [-0.], [1.], [-1.], [-1., -1.], [1., 0.], [.5, .5, .5]]:
            yield [None, (1, classification, path)]
    yield [(1, 0, [0.]), (1, 2, [1.])]
    yield [(1, 0, [1.]), (1, 2, [0.])]
    yield [(0, 2, [1.])]
    yield [(1, 2, [0.]), (2, 2, [1.])]
    rng = random.Random(2026101304)
    for _ in range(1000):
        registered = []
        for _ in range(rng.randrange(0, 17)):
            if rng.randrange(5) == 0:
                registered.append(None)
            else:
                registered.append((rng.randrange(0, 7), rng.choice([0, 1, 2, 2, 3, -1]),
                    [rng.choice([0., -0., 1., -1., .5, 1e-30, rng.uniform(-1., 1.)]) for _ in range(rng.randrange(1, 9))]))
        yield registered


def main(path):
    detailed_records(path)
    e = Emulator(path)
    manager, implementation, out = e.alloc(0x180), e.alloc(0x180), e.alloc(24)
    install_allocator(e)
    persistent_top = e.heap_top
    print('# reference_sha256', REFERENCE_SHA256)
    print('# unchanged collection 0x1412dc3c0, query 0x1412e3470, opacity 0x1412d9430; only allocator/free stubs')
    print('# C count [present (token classification depth alpha_bits*)?]* -> bool selected_count selected_indices*')
    count = 0
    for registered in cases():
        e.heap_top = persistent_top
        items = build_registry(e, manager, implementation, registered)
        e.call(0x1412dc3c0, ints=[implementation, out, 2], max_instructions=100000)
        begin, end = e.read_u64(out), e.read_u64(out + 8)
        selected = [items.index(e.read_u64(address)) for address in range(begin, end, 8)]
        result = e.call(0x1412e3470, ints=[manager], max_instructions=100000) & 255
        flat = []
        for entry in registered:
            if entry is None:
                flat.append(0)
            else:
                token, classification, alphas = entry
                flat.extend([1, token, classification, len(alphas), *(f'{bits(a):08x}' for a in alphas)])
        print('C', len(registered), *flat, '->', result, len(selected), *selected)
        count += 1
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
