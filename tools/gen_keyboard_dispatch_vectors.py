#!/usr/bin/env python3
"""Replay complete keyboard entry, native focused filter, and binding loop together."""
import itertools
import random
import struct
import sys
from emulate_xp import Emulator, RET_ADDR
from extract_commands import detailed_records, REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9


def cases():
    for capture, first, second, own, mode, block, char, key, mask in itertools.product(
            [0, 1], [0, 1], [0, 1], [0, 1, 255], [0, 1000, 1001], [0, 255], [-1, 128], [0, 27], [0, 0xffffffff]):
        yield capture, first, second, own, mode, block, char, key, mask, 8, 1, 0, -1, -1
    rng = random.Random(2026101303)
    for _ in range(1000):
        yield rng.randrange(2), rng.choice([-1, 0, 1]), rng.choice([-1, 0, 1]), rng.choice([0, 1, 255]), rng.choice([0, 1000, 1001]), rng.choice([0, 1, 255]), rng.choice([-2147483648, -1, 0, 3, 27, 65, 127, 128, 255, 2147483647]), rng.choice([0, 27, 67, 283, 0xffffffff]), rng.choice([0, 8, 0x400, 0xffffffff]), rng.getrandbits(32), rng.randrange(2), rng.choice([0, 1, -1]), rng.choice([-1, 0, 1000, 1001]), rng.choice([-1, 0, 255])


def main(path):
    detailed_records(path)
    e = Emulator(path)
    owner, vector, root, vt, mode_ptr = e.alloc(64), e.alloc(48), e.alloc(64), e.alloc(96), e.alloc(4)
    e.write_u64(root, vt)
    e.write_u64(vt + 0x50, 0x1412ee8a0)
    e.write_u64(vt + 0x38, RET_ADDR + 0x100)
    e.write_u64(0x142f01eb0, mode_ptr)
    e.write_u64(owner + 8, vector)
    e.write_u64(owner + 16, vector + 48)
    log = []
    values = []

    def mutation():
        if values[12] >= 0:
            e.write_u32(mode_ptr, values[12])
        if values[13] >= 0:
            e.write(0x145a0df02, bytes([values[13]]))

    def capture(emu):
        log.append('Q')
        emu.uc.reg_write(UC_X86_REG_RAX, values[0])

    def intercept(emu):
        char, flags, key, phase = [emu.reg(r) & 0xffffffff for r in [UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9]]
        assert char <= 255 and key <= 255 and phase in [0, 2]
        log.append(f'P:{char}:{key}:{flags}:{phase}')
        mutation()
        emu.uc.reg_write(UC_X86_REG_RAX, values[1 if phase == 0 else 2] & 0xffffffff)

    def ui_trace(uc, addr, size, user):
        if addr == 0x1412c6300:
            args = [e.reg(r) & 0xffffffff for r in [UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8]]
            log.append('U:' + ':'.join(map(str, args)))

    def own(emu):
        assert emu.reg(UC_X86_REG_RCX) == root
        log.append('O')
        mutation()
        emu.uc.reg_write(UC_X86_REG_RAX, values[3])

    def command(kind):
        def run(emu):
            assert emu.reg(UC_X86_REG_R8) == 0
            log.append(f'{kind}:{emu.reg(UC_X86_REG_RDX) & 0xffffffff}')
        return run

    e.uc.hook_add(UC_HOOK_CODE, ui_trace, begin=0x1412c6300, end=0x1412c6300)
    e.stubs[0x1412e3470] = capture
    e.stubs[0x141913800] = intercept
    e.stubs[RET_ADDR + 0x100] = own
    e.stubs[0x140c7fac0] = command('B')
    e.stubs[0x140c7fc40] = command('E')
    print('# reference_sha256', REFERENCE_SHA256)
    print('# whole 0x140c627a0, focused wrapper/base/filter/CRT, binding loop unchanged; five service endpoints logged')
    print('# K capture first second own mode block char key mask flags focus suppression update_mode update_block -> suppression active* mode block trace*')
    count = 0
    for case in cases():
        # Out-of-domain negative CRT inputs are only replayed without a focus
        # or with the named Escape shortcut, which bypasses classification.
        if case[6] < -1 and case[7] != 27:
            case = (*case[:10], 0, *case[11:])
        values[:] = case
        capture_value, first, second, own_value, mode, block, char, key, mask, flags, focus, suppression, _, _ = case
        entries = [(key, flags & 7, 3, 0), (key, flags & 7, 29, 1), (key, flags & 7, 30, -1)]
        e.write(vector, b''.join(struct.pack('<IIii', *v) for v in entries))
        e.write_u32(owner + 32, suppression)
        e.write_u32(mode_ptr, mode)
        e.write(0x145a0df02, bytes([block]))
        e.write_u64(0x145a0dfc8, root if focus else 0)
        e.write_u32(root + 0x18, mask)
        log.clear()
        e.call(0x140c627a0, ints=[owner, char, key, flags], max_instructions=5000)
        active = [struct.unpack('<i', e.read(vector + i * 16 + 12, 4))[0] for i in range(3)]
        after = struct.unpack('<i', e.read(owner + 32, 4))[0]
        print('K', *case, '->', after, *active, e.read_u32(mode_ptr), e.read(0x145a0df02, 1)[0], *log)
        count += 1
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
