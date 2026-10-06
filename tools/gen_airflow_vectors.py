#!/usr/bin/env python3
"""Vectors for airflow::airflow from the original 0x14121b580.

The wind sampler 0x141ba80a0, the finite check 0x141176330 and the engine flag 0x1417f12c0 are stubs: the
sampler returns random wind and records the double position it was asked for.

Line: A x z y r12 flag | s0 c0 s1 c1 s2 c2 | o378 o380 o388 (double) | r368 r36c r370 | rx ry rz |
      wind1 wind2 wind3 | seen_x seen_y seen_z (double) | out1 out2 out3

    python3 tools/gen_airflow_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/airflow.txt
"""
import math
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from unicorn.x86_const import (  # noqa: E402
    UC_X86_REG_RAX, UC_X86_REG_RDX, UC_X86_REG_RSP, UC_X86_REG_R12, UC_X86_REG_XMM1, UC_X86_REG_XMM2,
    UC_X86_REG_XMM3)


def hx(x):
    return f'{struct.unpack("<I", struct.pack("<f", float(x)))[0]:08x}'


def hd(x):
    return f'{struct.unpack("<Q", struct.pack("<d", float(x)))[0]:016x}'


def main():
    emu = Emulator(sys.argv[1])
    rng = random.Random(20261026)
    st = {'flag': 0, 'wind': [0.0] * 3, 'seen': None}

    def sanitize(e):
        ptr = e.uc.reg_read(UC_X86_REG_RDX)
        v = e.read_f32(ptr)
        if not math.isfinite(v):
            e.write_f32(ptr, 0.0)

    def sampler(e):
        rsp = e.uc.reg_read(UC_X86_REG_RSP)
        pointers = [e.read_u64(rsp + 0x28 + 8 * i) for i in range(3)]
        reg = [e.uc.reg_read(r) & 0xffffffffffffffff for r in (UC_X86_REG_XMM1, UC_X86_REG_XMM2, UC_X86_REG_XMM3)]
        st['seen'] = [struct.unpack('<d', struct.pack('<Q', r))[0] for r in reg]
        for p, v in zip(pointers, st['wind']):
            e.write_f32(p, v)

    emu.stubs[0x141176330] = sanitize
    emu.stubs[0x141ba80a0] = sampler
    emu.stubs[0x1417f12c0] = lambda e: e.uc.reg_write(UC_X86_REG_RAX, st['flag'])
    F = emu.alloc(0x10000)
    outs = [emu.alloc(16) for _ in range(3)]
    print('# airflow vectors from 0x14121b580 (tools/gen_airflow_vectors.py)')
    for _ in range(500):
        s = [rng.uniform(-1, 1) for _ in range(6)]
        origin = [rng.choice([rng.uniform(-5000, 5000), 0.0]) for _ in range(3)]
        ref = [rng.uniform(-20, 20) for _ in range(3)]
        if rng.random() < 0.1:
            ref[rng.randrange(3)] = rng.choice([float('nan'), float('inf')])
        rates = [rng.uniform(-1, 1) for _ in range(3)]
        for off, v in zip((0x430, 0x434, 0x440, 0x444, 0x450, 0x454), s):
            emu.write_f32(F + off, v)
        for off, v in zip((0x378, 0x380, 0x388), origin):
            emu.write(F + off, struct.pack('<d', v))
        for off, v in zip((0x368, 0x36c, 0x370, 0x3cc, 0x3d0, 0x3d4), ref + rates):
            emu.write_f32(F + off, v)
        x, z, y = (rng.uniform(-100, 100) for _ in range(3))
        # a non-finite world position reaches the logging code inside the function, which is not emulated
        r12 = rng.choice([1, 1, 0, 0x12345678])
        st['flag'] = rng.randrange(2)
        st['wind'] = [rng.uniform(-60, 60) for _ in range(3)]
        emu.uc.reg_write(UC_X86_REG_R12, r12)
        emu.call(0x14121b580, ints=[F, 0, outs[0]], floats=[0, x, 0, z],
                 stack=[outs[1], y, outs[2], 0, 0xffffffff, 0xffffffff, 0])
        print('A', *map(hx, (x, z, y)), r12, st['flag'], '|', *map(hx, s), '|', *map(hd, origin), '|',
              *map(hx, ref), '|', *map(hx, rates), '|', *map(hx, st['wind']), '|', *map(hd, st['seen']), '|',
              *(hx(emu.read_f32(o)) for o in outs))


if __name__ == '__main__':
    main()
