#!/usr/bin/env python3
"""Vectors for engine::record_flag_6040 from the original 0x1411d9ec0.

Line: G mode blocked flag bind | result      (bind = answer of the 0x179 query, which is given `mode`)

    python3 tools/gen_record_flag_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/record_flag.txt
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_R9  # noqa: E402


def main():
    emu = Emulator(sys.argv[1])
    rng = random.Random(20261024)
    state = {'bind': 0, 'seen': None}

    def bind(e):
        state['seen'] = e.uc.reg_read(UC_X86_REG_R9) & 0xffffffff
        e.uc.reg_write(UC_X86_REG_RAX, state['bind'])

    emu.stubs[0x1407ace10] = bind
    F = emu.alloc(0x1000)
    B = emu.alloc(0x10000)
    R = emu.alloc(0x34c8 * 3)
    emu.write(F + 0x20, struct.pack('<Q', B))
    emu.write(B + 0x6040, struct.pack('<Q', R))
    print('# record_flag_6040 vectors from 0x1411d9ec0 (tools/gen_record_flag_vectors.py)')
    for _ in range(400):
        idx = rng.randrange(3)
        base = R + idx * 0x34c8
        mode = rng.choice([rng.randrange(0, 0x30), rng.randrange(0, 0x100)])
        blocked = rng.choice([0, 0, 1, 7])
        flag = rng.choice([0, 1, 5, 255])
        emu.write_u32(base + 0x5f0, mode)
        emu.write_u32(base + 0x54, blocked)
        emu.write(base + 0x588, bytes([flag]))
        state['bind'] = int(rng.random() < 0.5)
        state['seen'] = None
        result = emu.call(0x1411d9ec0, ints=[F, idx]) & 0xff
        if state['seen'] is not None:
            assert state['seen'] == mode
        print('G', mode, blocked, flag, state['bind'], '|', result)


if __name__ == '__main__':
    main()
