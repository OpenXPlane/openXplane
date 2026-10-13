#!/usr/bin/env python3
"""Replay native key-vector transitions locally; only Begin/End callbacks are replaced by loggers."""
import random
import struct
import sys
from emulate_xp import Emulator, STACK_TOP
from extract_commands import REFERENCE_SHA256, detailed_records
from unicorn.x86_const import (UC_X86_REG_RBP, UC_X86_REG_RSP, UC_X86_REG_RSI,
                              UC_X86_REG_RDI, UC_X86_REG_R15, UC_X86_REG_RDX, UC_X86_REG_R8)

BEGIN, END = 0x140c62868, 0x140c628ea


def cases(defaults):
    # All real default key/mask chords in the native catalogue, including duplicates.
    for key, modifiers, _, _ in defaults:
        for flags in [modifiers | 8, modifiers | 16, 16, modifiers | 24]:
            yield key, flags, 0, [(k, m, c, int(bool(flags & 16))) for k, m, c, _ in defaults]
    rng = random.Random(20261013)
    for _ in range(2000):
        key = rng.choice([0, 27, 70, 183, 0xffffffff])
        flags = rng.randrange(0, 256)
        suppression = rng.choice([0, 1, -1])
        entries = [(rng.choice([key, key, 27, 70]), rng.randrange(0, 10),
                    rng.randrange(-10, 4000), rng.choice([0, 0, 1, -1, 2]))
                   for _ in range(rng.randrange(0, 17))]
        yield key, flags, suppression, entries


def main(path):
    records = detailed_records(path)  # SHA256 and native table-bound validation
    defaults = [(key, mask, index, 0) for index, (_, _, key, _, mask) in enumerate(records) if key]
    emu = Emulator(path)
    owner, vector = emu.alloc(64), emu.alloc(4096)
    log = []

    def logger(kind):
        def call(e):
            command = e.uc.reg_read(UC_X86_REG_RDX) & 0xffffffff
            if command >= 0x80000000:
                command -= 0x100000000
            assert e.uc.reg_read(UC_X86_REG_R8) == 0
            log.append((kind, command))
        return call

    emu.stubs[0x140c7fac0] = logger(0)
    emu.stubs[0x140c7fc40] = logger(1)
    print('# reference_sha256', REFERENCE_SHA256)
    print('# range', hex(BEGIN), hex(END), 'exclusive')
    print('# local original key loop; BEGIN/END callbacks logged, callback bodies not executed')
    print('# E key flags suppress count (key mask command active)* -> suppress active* event_count (kind command)*')
    total = 0
    for key, flags, suppression, entries in cases(defaults):
        log.clear()
        raw = b''.join(struct.pack('<IIii', *entry) for entry in entries)
        emu.write(vector, raw)
        emu.write_u64(owner + 8, vector)
        emu.write_u64(owner + 16, vector + len(raw))
        emu.write(owner + 32, struct.pack('<i', suppression))
        for reg, value in [(UC_X86_REG_RBP, owner), (UC_X86_REG_RSP, STACK_TOP - 0x1000),
                           (UC_X86_REG_RSI, key), (UC_X86_REG_RDI, flags), (UC_X86_REG_R15, 0)]:
            emu.uc.reg_write(reg, value)
        emu.run_range(BEGIN, END)
        after = struct.unpack('<i', emu.read(owner + 32, 4))[0]
        active = [struct.unpack('<i', emu.read(vector + i * 16 + 12, 4))[0] for i in range(len(entries))]
        print('E', key, flags, suppression, len(entries), *(v for entry in entries for v in entry),
              '->', after, *active, len(log), *(v for call in log for v in call))
        total += 1
    print('# cases', total)


if __name__ == '__main__':
    main(sys.argv[1])
