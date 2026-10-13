#!/usr/bin/env python3
"""Replay interceptor wrapper and registered adapter; only external service endpoints stubbed."""
import itertools
import random
import struct
import sys
from emulate_xp import Emulator, RET_ADDR
from extract_commands import REFERENCE_SHA256, detailed_records
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_RSP, UC_X86_REG_XMM0


def dbits(v):
    return struct.unpack('<Q', struct.pack('<d', v))[0]


def cases():
    for registered, exported, mode, enabled, token, result, char, phase in itertools.product([0, 1], [0, 1], [0, 1, 1000], [0, 1], [-1, 0, 7], [-1, 0, 1], [0, 27], [0, 2]):
        yield registered, exported, mode, 1, enabled, 17, 2., 1., 1.25, token, result, char, 27, 8, 0, 0, 0, 0, phase
    rng = random.Random(2026101305)
    for _ in range(1000):
        yield rng.randrange(2), rng.randrange(2), rng.choice([0, 0, -1, 1000]), rng.getrandbits(32), rng.getrandbits(32), rng.getrandbits(64), rng.uniform(-1000., 1000.), rng.uniform(-100., 100.), rng.uniform(-100., 100.), rng.choice([-2147483648, -1, 0, 1, 2147483647]), rng.choice([-2147483648, -1, 0, 1, 255, 2147483647]), rng.choice([0, 27, 255]), rng.choice([0, 27, 255]), rng.choice([0, 8, 12, 0xffffffff]), rng.getrandbits(64), rng.randrange(2), rng.randrange(2), rng.randrange(2), rng.choice([0, 2, 0xffffffff])


def main(path):
    detailed_records(path)
    e = Emulator(path)
    mode_ptr = e.alloc(4)
    e.write_u64(0x142f01eb0, mode_ptr)
    e.write_u64(0x146105010, RET_ADDR + 0x200)
    e.write_u64(0x146105018, RET_ADDR + 0x300)
    values, log, clock_index = [], [], [0]

    def clock(emu):
        i = clock_index[0]
        assert i < 2
        clock_index[0] += 1
        log.append(f'T:{i}')
        emu.uc.reg_write(UC_X86_REG_XMM0, dbits(values[7 + i]))

    def enter_scope(emu):
        log.append(f'S:{emu.reg(UC_X86_REG_RCX)}')
        emu.uc.reg_write(UC_X86_REG_RAX, 123)

    def leave_scope(emu):
        log.append(f'R:{emu.reg(UC_X86_REG_RCX)}:{emu.reg(UC_X86_REG_RDX)}')

    def enter_hook(emu):
        log.append('A')
        if values[15]:
            emu.write_u64(0x146106038, 0 if values[1] else RET_ADDR + 0x100)
        emu.uc.reg_write(UC_X86_REG_RAX, values[9] & 0xffffffff)

    def leave_hook(emu):
        log.append(f'Z:{emu.reg(UC_X86_REG_RCX) & 0xffffffff}')

    def hook(emu):
        args = [emu.reg(r) & 0xffffffff for r in [UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9]]
        refcon = emu.read_u64(emu.reg(UC_X86_REG_RSP) + 40)
        log.append('H:' + ':'.join(map(str, [*args, refcon])))
        if values[16]:
            emu.write(0x142f018f8, struct.pack('<d', -3.5))
        if values[17]:
            emu.write_u64(0x146105520, 99)
        emu.uc.reg_write(UC_X86_REG_RAX, values[10] & 0xffffffff)

    e.stubs.update({0x1413fa080: clock, 0x1415ad510: enter_scope, 0x1415b1920: leave_scope,
                    RET_ADDR + 0x100: hook, RET_ADDR + 0x200: enter_hook, RET_ADDR + 0x300: leave_hook})
    print('# reference_sha256', REFERENCE_SHA256)
    print('# complete wrapper 0x141913800 and adapter 0x141917f20 unchanged; six clock/scope/context/export endpoints logged')
    print('# J registered exported mode masks context initial start end token result char key flags refcon flip_export change_elapsed change_context phase -> result elapsed_bits context export trace*')
    count = 0
    for case in cases():
        values[:] = case
        registered, exported, mode, mask, enabled, context, elapsed, start, end, token, result, char, key, flags, refcon, _, _, _, phase = case
        e.write_u64(0x146104f88, 0x141917f20 if registered else 0)
        e.write_u64(0x146106038, RET_ADDR + 0x100 if exported else 0)
        e.write_u32(mode_ptr, mode)
        e.write_u32(0x14610551c, mask)
        e.write_u32(0x145a16bd8, enabled)
        e.write_u64(0x146105520, context)
        e.write(0x142f018f8, struct.pack('<d', elapsed))
        log.clear(); clock_index[0] = 0
        actual = e.call(0x141913800, ints=[char, flags, key, phase], stack=[refcon], max_instructions=10000) & 0xffffffff
        fields = list(case)
        for i in [6, 7, 8]:
            fields[i] = f'{dbits(fields[i]):016x}'
        print('J', *fields, '->', actual, f'{e.read_u64(0x142f018f8):016x}', e.read_u64(0x146105520), int(e.read_u64(0x146106038) != 0), *log)
        count += 1
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
