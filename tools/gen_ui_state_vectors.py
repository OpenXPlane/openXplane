#!/usr/bin/env python3
"""Replay full native window queries and menu routing; only four UI action bodies are loggers."""
import random
import struct
import sys
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256, detailed_records
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8


def bits(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def cases():
    # Missing controller mapping, null mapping, unregistered, duplicate registrations and parent opacity.
    yield 11, 0, [], []
    yield 11, 255, [(11, 0)], []
    yield 11, 1, [(11, 1)], []
    for byte in [0, 1, 2, 255]:
        for path in [[1.], [0.], [-0.], [-1.], [1., .5, 0.], [-1., -1.], [.5, .5, .5]]:
            yield 11, byte, [(11, 1)], [(1, path)]
    yield 11, 1, [(11, 1)], [None, (1, [0.]), (1, [1.])]
    yield 11, 1, [(11, 1)], [None, (1, [1.]), (1, [0.])]
    rng = random.Random(20261013)
    for _ in range(1000):
        mapping = [(kind, rng.randrange(0, 7)) for kind in sorted(set(rng.choices([-2147483648, -10, 0, 11, 12, 27, 2147483647], k=rng.randrange(0, 8))))]
        registered = []
        for _ in range(rng.randrange(0, 9)):
            if rng.randrange(0, 5) == 0:
                registered.append(None)
            else:
                path = [rng.choice([0., -0., .5, 1., -1., 1e-30, rng.uniform(-1., 1.)])
                        for _ in range(rng.randrange(1, 9))]
                registered.append((rng.randrange(1, 7), path))
        yield rng.choice([-2147483648, -10, 0, 11, 12, 27, 2147483647]), rng.randrange(0, 256), mapping, registered


def main(path):
    detailed_records(path)  # validates baseline SHA256
    emu = Emulator(path)
    owner, manager, implementation = emu.alloc(0x2b0), emu.alloc(0x180), emu.alloc(0x180)
    emu.write_u64(0x145a0e308, manager)
    emu.write_u64(manager + 0x128, implementation)
    persistent_top = emu.heap_top
    log = []

    def effect(name, kind=None, flag=None):
        def call(e):
            assert e.reg(UC_X86_REG_RCX) == owner
            if kind is not None:
                assert e.reg(UC_X86_REG_RDX) == kind
            if flag is not None:
                assert e.reg(UC_X86_REG_R8) & 255 == flag
            log.append(name)
        return call

    emu.stubs.update({0x14150ce20: effect('CloseMainMenu', 11, 1),
                      0x14150e900: effect('EnsureMainMenu', 11, 0),
                      0x14150d370: effect('Refresh'),
                      0x14150fe90: effect('ToggleSettingsWindow', 12)})
    # Observe real query entry points without replacing them or altering their results.
    for addr, name in [(0x141512eb0, 'QueryMainMenu'), (0x1417fb320, 'QueryState')]:
        emu.uc.hook_add(UC_HOOK_CODE, lambda uc, a, n, user, label=name: log.append(label), begin=addr, end=addr)
    print('# reference_sha256', REFERENCE_SHA256)
    print('# native 0x141512eb0, 0x1412e35c0, 0x1412fd160, 0x1412d9430, 0x1417fb320 execute unchanged')
    print('# native handler 0x14150bfe0; four UI action bodies logged; finite opacity paths')
    print('# W kind byte map_count (kind token)* registered_count (token depth alpha_bits*)* -> query opacity_bits* main_trace_count names* settings_trace_count names*')
    count = 0
    for kind, byte, mapping, registered in cases():
        emu.heap_top = persistent_top
        handle = {i: emu.alloc(16) for i in range(1, 7)}
        sentinel = emu.alloc(48)
        emu.write(sentinel, bytes(48))
        emu.write(sentinel + 0x19, b'\1')

        def tree(rows):
            if not rows:
                return sentinel
            middle = len(rows) // 2
            key, token = rows[middle]
            node = emu.alloc(48)
            emu.write(node, bytes(48))
            emu.write_u64(node, tree(rows[:middle]))
            emu.write_u64(node + 16, tree(rows[middle + 1:]))
            emu.write(node + 32, struct.pack('<i', key))
            emu.write_u64(node + 40, handle[token] if token else 0)
            return node

        emu.write_u64(sentinel + 8, tree(mapping))
        emu.write_u64(owner + 0x148, sentinel)
        head = emu.alloc(24)
        nodes = [emu.alloc(24) for _ in registered]
        emu.write_u64(head, nodes[0] if nodes else head)
        emu.write_u64(implementation + 0x150, head)
        expected_opacities = []
        for index, entry in enumerate(registered):
            node = nodes[index]
            emu.write(node, bytes(24))
            emu.write_u64(node, nodes[index + 1] if index + 1 < len(nodes) else head)
            if entry is not None:
                token, path_values = entry
                item = emu.alloc(0x148)
                emu.write_u64(node + 16, item)
                emu.write_u64(item + 0x138, handle[token])
                parent = 0
                for alpha in path_values:
                    widget = emu.alloc(0x70)
                    emu.write_f32(widget + 0x58, alpha)
                    emu.write_u64(widget + 0x60, parent)
                    parent = widget
                emu.write_u64(item + 0x140, parent)
                emu.call(0x1412d9430, ints=[parent])
                expected_opacities.append(bits(emu.xmm0_f32()))
            else:
                expected_opacities.append(0)
        emu.write(0x142f2d770 + 0x525, bytes([byte]))
        query = emu.call(0x141512eb0, ints=[owner, kind & 0xffffffff]) & 255
        traces = []
        for command in [29, 30]:
            log.clear()
            emu.call(0x14150bfe0, ints=[owner, command], max_instructions=5000)
            traces.extend([len(log), *log])
        flat = []
        for entry in registered:
            if entry is None:
                flat.extend([0, 0])
            else:
                token, alphas = entry
                flat.extend([token, len(alphas), *(f'{bits(a):08x}' for a in alphas)])
        print('W', kind, byte, len(mapping), *(v for pair in mapping for v in pair), len(registered), *flat,
              '->', query, *(f'{b:08x}' for b in expected_opacities), *traces)
        count += 1
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
