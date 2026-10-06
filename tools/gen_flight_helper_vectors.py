#!/usr/bin/env python3
"""Vectors for engine::{cosine_blend, root_ratio, record_flag_6028} from 0x14121b4d0, 0x1411b5ee0, 0x1411da150.

Lines (float32 hex unless noted):
  C a10 a14 a18 r18 r20 r70 | result (double hex)
  N v10 record f664 idx | result
  W flag bound | result

    python3 tools/gen_flight_helper_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/flight_helpers.txt
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_XMM0  # noqa: E402


def hx(x):
    return f'{struct.unpack("<I", struct.pack("<f", float(x)))[0]:08x}'


def main():
    emu = Emulator(sys.argv[1])
    rng = random.Random(20261025)
    bound = {'v': 0}
    emu.stubs[0x1407ace10] = lambda e: e.uc.reg_write(UC_X86_REG_RAX, bound['v'])
    this = emu.alloc(0x1000)
    other = emu.alloc(0x1000)
    emu.write(this, struct.pack('<Q', other))
    print('# flight helper vectors (tools/gen_flight_helper_vectors.py)')
    for _ in range(300):
        a = [rng.uniform(-1, 1), rng.uniform(-180, 180), rng.uniform(-180, 180)]
        r = [rng.uniform(-5, 5), rng.uniform(-5, 5), rng.uniform(-5, 5)]
        for o, v in zip((0x10, 0x14, 0x18), a):
            emu.write_f32(this + o, v)
        for o, v in zip((0x18, 0x20, 0x70), r):
            emu.write_f32(other + o, v)
        emu.call(0x14121b4d0, ints=[this])
        d = emu.uc.reg_read(UC_X86_REG_XMM0) & 0xffffffffffffffff
        print('C', *map(hx, a), *map(hx, r), '|', f'{d:016x}')
    for _ in range(400):
        v10 = rng.choice([rng.uniform(0, 30), 0.0])
        idx = rng.choice([0, 2, 3, 4])  # index 1 would put the record on +0x664
        rec = rng.uniform(-2, 30)
        f664 = rng.uniform(-2, 30)
        emu.write_f32(this + 0x10, v10)
        emu.write_u32(this + 0x654, idx)
        emu.write_f32(this + 0x58c + idx * 0xd8, rec)
        emu.write_f32(this + 0x664, f664)
        emu.call(0x1411b5ee0, ints=[this])
        print('N', hx(v10), hx(rec), hx(f664), idx, '|', hx(emu.xmm0_f32()))
    B = emu.alloc(0x10000)
    R = emu.alloc(0x36c8 * 3)
    emu.write(this + 0x20, struct.pack('<Q', B))
    emu.write(B + 0x6028, struct.pack('<Q', R))
    for _ in range(200):
        idx = rng.randrange(3)
        flag = rng.choice([0, 1, 9, 255])
        emu.write(R + idx * 0x36c8 + 0x678, bytes([flag]))
        bound['v'] = rng.randrange(2)
        result = emu.call(0x1411da150, ints=[this, idx]) & 0xff
        print('W', flag, bound['v'], '|', result)


if __name__ == '__main__':
    main()
