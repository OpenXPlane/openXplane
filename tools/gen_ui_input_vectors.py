#!/usr/bin/env python3
"""Replay focused UI dispatch with the native acceptance filter/CRT and logged child/own handlers."""
import random
import sys
from emulate_xp import Emulator, RET_ADDR
from extract_commands import REFERENCE_SHA256, detailed_records
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9


def cases():
    masks = [0, 0xffffffff, *(1 << bit for bit in range(14))]
    keys = [8, 9, 13, 27, *range(33, 41), 46, 67, 86, 88, 0xbb, 0xbc, 0, 255]
    chars = [0, 3, 22, 24, 27, 32, 43, 45, 48, 65, 97, 63, 127, 128, 255, -1]
    for key in keys:
        for char in chars:
            for mask in masks:
                yield True, mask, char, key, 12, None, 1, None
    rng = random.Random(20261013)
    for _ in range(2000):
        yield bool(rng.randrange(2)), rng.getrandbits(32), rng.choice(chars), rng.choice(keys), rng.getrandbits(32), rng.choice([None, 0, 1, 2, 255]), rng.choice([0, 1, 2, 255]), rng.choice([None, 0, 8, 0xffffffff])


def main(path):
    detailed_records(path)
    emu = Emulator(path)
    root, child, vt, child_vt = emu.alloc(64), emu.alloc(64), emu.alloc(96), emu.alloc(96)
    emu.write_u64(root, vt)
    emu.write_u64(child, child_vt)
    emu.write_u64(vt + 0x50, 0x1412ee8a0)
    emu.write_u64(vt + 0x38, RET_ADDR + 0x100)
    emu.write_u64(child_vt + 0x50, RET_ADDR + 0x200)
    log = []
    supplied = [0, 0]
    incoming = [0, 0, 0]
    update = [None]

    def callback(index, object_pointer, name):
        def call(e):
            assert e.reg(UC_X86_REG_RCX) == object_pointer
            assert [e.reg(r) & 0xffffffff for r in [UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9]] == incoming
            log.append(name)
            if index == 0 and update[0] is not None:
                e.write_u32(root + 0x18, update[0])
            e.uc.reg_write(UC_X86_REG_RAX, supplied[index])
        return call

    emu.stubs[RET_ADDR + 0x100] = callback(1, root, 'own')
    emu.stubs[RET_ADDR + 0x200] = callback(0, child, 'child')
    print('# reference_sha256', REFERENCE_SHA256)
    print('# native wrapper 0x1412c6300, dispatch 0x1412ee8a0, filter 0x1412e5a90, initial mapped CRT execute unchanged')
    print('# only two virtual handler endpoints stubbed; optional child mask mutation; no graph replacement')
    print('# I focus mask char key flags child_result(-1=absent) own_result child_mask_update(-1=none) -> raw_return final_mask call_count names*')
    count = 0
    for focus, mask, char, key, flags, child_result, own_result, child_update in cases():
        log.clear()
        supplied[:] = [child_result or 0, own_result]
        update[:] = [child_update]
        incoming[:] = [char & 0xffffffff, key, flags]
        emu.write_u64(0x145a0dfc8, root if focus else 0)
        emu.write_u64(root + 0x10, child if child_result is not None else 0)
        emu.write_u32(root + 0x18, mask)
        result = emu.call(0x1412c6300, ints=[char, key, flags], max_instructions=5000) & 255
        print('I', int(focus), mask, char, key, flags, -1 if child_result is None else child_result, own_result, -1 if child_update is None else child_update,
              '->', result, emu.read_u32(root + 0x18), len(log), *log)
        count += 1
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
