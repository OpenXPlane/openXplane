#!/usr/bin/env python3
"""Vectors for engine::engine_held_back from the original 0x1411d9f60.

Line: H kind lever low high idx mode | b179 b1f9 b2f6 b2f7 b2f8 b2f9 | fallback | result

    python3 tools/gen_engine_held_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/engine_held.txt
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_R8  # noqa: E402

IDS = [0x179, 0x1f9, 0x2f6, 0x2f7, 0x2f8, 0x2f9]


def hx(x):
    return f'{struct.unpack("<I", struct.pack("<f", float(x)))[0]:08x}'


def main():
    emu = Emulator(sys.argv[1])
    rng = random.Random(20261023)
    answers, fb = {}, {'v': 0}
    emu.stubs[0x1407ace10] = lambda e: e.uc.reg_write(
        UC_X86_REG_RAX, answers[e.uc.reg_read(UC_X86_REG_R8)])
    emu.stubs[0x140822620] = lambda e: e.uc.reg_write(UC_X86_REG_RAX, fb['v'])
    F = emu.alloc(0x10000)
    B = emu.alloc(0x10000)
    R = emu.alloc(0x3770 * 3)
    emu.write(F + 0x20, struct.pack('<Q', B))
    emu.write(B + 0x6010, struct.pack('<Q', R))
    print('# engine_held_back vectors from 0x1411d9f60 (tools/gen_engine_held_vectors.py)')
    for _ in range(600):
        idx = rng.randrange(3)
        kind = rng.choice([6, 6, 0, 1, 3])
        lever = rng.choice([0.0, 0.0, rng.uniform(-1, 1)])
        low, high = (rng.choice([0.0, rng.uniform(-5, 5)]) for _ in range(2))
        mode = rng.choice([0, 1, 2, 3])
        emu.write_u32(R + idx * 0x3770, kind)
        emu.write_f32(F + 0x64b4, lever)
        emu.write_f32(R + idx * 0x3770 + 0x790, low)
        emu.write_f32(R + idx * 0x3770 + 0x798, high)
        for i in IDS:
            answers[i] = int(rng.random() < 0.35)
        fb['v'] = rng.randrange(-3, 4)
        result = emu.call(0x1411d9f60, ints=[F, idx, mode]) & 0xffffffff
        result -= 1 << 32 if result >> 31 else 0
        print('H', kind, hx(lever), hx(low), hx(high), idx, mode, '|',
              *(answers[i] for i in IDS), '|', fb['v'], '|', result)


if __name__ == '__main__':
    main()
