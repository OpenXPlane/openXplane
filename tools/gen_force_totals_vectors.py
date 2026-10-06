#!/usr/bin/env python3
"""Generates test vectors for forces::force_totals by running the block 0x14126f37f..0x14126f7f3 of the original
flight model step 0x1412656b0 on a random flight object. The debug range checks inside the block are replaced by
a stub that returns.

Line: before (words 0x2b0..0x340 then 0x6750..0x67d0, hex) | after (same words)

    python3 tools/gen_force_totals_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/force_totals.txt
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator, STACK_TOP  # noqa: E402
from unicorn.x86_const import UC_X86_REG_R15, UC_X86_REG_RSP  # noqa: E402

BEGIN, END = 0x14126f37f, 0x14126f7f3
RANGES = [(0x2b0, 0x340), (0x6750, 0x67d0)]


def bits(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


def main():
    emu = Emulator(sys.argv[1])
    emu.stubs[0x1408e25a0] = lambda e: None
    F = emu.alloc(0x10000)
    rng = random.Random(20261019)
    print('# force totals vectors from the original block 0x14126f37f (see tools/gen_force_totals_vectors.py)')
    for _ in range(300):
        words = {}
        for lo, hi in RANGES:
            for off in range(lo, hi, 4):
                words[off] = bits(rng.uniform(-5, 5))
        for flag in (0x675c, 0x6768, 0x676c, 0x6770):
            words[flag] = rng.choice([0, 1, 1]) if flag != 0x675c else rng.choice([0, 0, 0, 1])
        for off, v in words.items():
            emu.write_u32(F + off, v)
        uc = emu.uc
        uc.reg_write(UC_X86_REG_R15, F)
        uc.reg_write(UC_X86_REG_RSP, STACK_TOP - 0x2000)
        uc.emu_start(BEGIN, END)
        offs = [o for lo, hi in RANGES for o in range(lo, hi, 4)]
        after = [emu.read_u32(F + o) for o in offs]
        print(*[f'{words[o]:08x}' for o in offs], '|', *[f'{v:08x}' for v in after])


if __name__ == '__main__':
    main()
