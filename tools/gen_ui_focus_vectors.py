#!/usr/bin/env python3
"""Replay complete native focus transition with logged notification endpoints."""
import random
import sys
from emulate_xp import Emulator, RET_ADDR
from extract_commands import REFERENCE_SHA256, detailed_records
from unicorn.x86_const import UC_X86_REG_RCX, UC_X86_REG_RDX

SELECTED, MODE, FOCUSED = 0x145a0def0, 0x145a0def8, 0x145a0dfc8


def cases():
    modes = [0, 1, 2, 0xffffffff]
    for selected in range(4):
        for focused in range(4):
            for target in range(4):
                for old in modes:
                    for new in modes:
                        yield selected, focused, old, target, new, -1, -1
    rng = random.Random(2026101302)
    for _ in range(1000):
        yield rng.randrange(4), rng.randrange(4), rng.choice([*modes, rng.getrandbits(32)]), rng.randrange(4), rng.choice(modes), rng.choice([-1, 0, 1, 2, 3]), rng.choice([-1, 0, 1, 2, 3])


def main(path):
    detailed_records(path)
    e = Emulator(path)
    vt = e.alloc(0x48)
    e.write_u64(vt + 0x40, RET_ADDR + 0x300)
    tokens = [0]
    ids = {0: 0}
    leaves = {}
    for root_id in range(1, 4):
        nodes = [e.alloc(0x20) for _ in range(root_id)]
        tokens.append(nodes[0])
        for index, node in enumerate(nodes):
            ids[node] = root_id if index == 0 else root_id * 10 + index
            e.write_u64(node, vt)
            e.write_u64(node + 0x10, nodes[index + 1] if index + 1 < len(nodes) else 0)
        leaves[root_id] = ids[nodes[-1]]
    log = []
    updates = [-1, -1]

    def state():
        return ids[e.read_u64(SELECTED)], ids[e.read_u64(FOCUSED)], e.read_u32(MODE)

    def callback(kind, mutation):
        def run(emu):
            obj = ids[emu.reg(UC_X86_REG_RCX)]
            if kind != 'B':
                assert emu.reg(UC_X86_REG_RDX) & 255 == 1
            log.append((kind, obj, *state()))
            if mutation is not None and updates[mutation] >= 0:
                emu.write_u64(FOCUSED, tokens[updates[mutation]])
                emu.write_u64(SELECTED, tokens[3])
                emu.write_u32(MODE, 0x76543210)
        return run

    e.stubs[0x1412e4680] = callback('L', 0)
    e.stubs[0x1412e47a0] = callback('E', None)
    e.stubs[RET_ADDR + 0x300] = callback('B', 1)
    print('# reference_sha256', REFERENCE_SHA256)
    print('# native 0x1412f7fe0 unchanged; leave/enter helpers and leaf slot40 notifications logged, not executed')
    print('# F selected focused old_mode target new_mode leave_focus_update blur_focus_update -> selected focused mode n [kind object observed_selected observed_focused observed_mode]*')
    count = 0
    for selected, focused, old, target, new, leave_update, blur_update in cases():
        e.write_u64(SELECTED, tokens[selected])
        e.write_u64(FOCUSED, tokens[focused])
        e.write_u32(MODE, old)
        updates[:] = [leave_update, blur_update]
        log.clear()
        e.call(0x1412f7fe0, ints=[tokens[target], new], max_instructions=1000)
        print('F', selected, focused, old, target, new, leave_update, blur_update, '->', *state(), len(log), *(v for row in log for v in row))
        count += 1
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
