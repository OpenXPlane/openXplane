#!/usr/bin/env python3
"""Vectors for element_force::boundary_ratio from the original 0x14121b290.

Line: Q n flag base(double) f430 f434 f450 f454 f42f5c | y614[0..11] y5e8[0..11] y5bc[0..11] | r

    python3 tools/gen_boundary_ratio_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/boundary_ratio.txt
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX  # noqa: E402


def hx(x):
    return f'{struct.unpack("<I", struct.pack("<f", float(x)))[0]:08x}'


def hd(x):
    return f'{struct.unpack("<Q", struct.pack("<d", float(x)))[0]:016x}'


def main():
    emu = Emulator(sys.argv[1])
    rng = random.Random(20261022)
    flag = {'v': 0}
    emu.stubs[0x1417f12c0] = lambda e: e.uc.reg_write(UC_X86_REG_RAX, flag['v'])
    F = emu.alloc(0x50000)
    P = emu.alloc(0x10000)
    A = emu.alloc(0x36c8 * 3)
    emu.write(F + 0x20, struct.pack('<Q', P))
    emu.write(P + 0x6028, struct.pack('<Q', A))
    print('# boundary_ratio vectors from 0x14121b290 (tools/gen_boundary_ratio_vectors.py)')
    for _ in range(300):
        idx = rng.randrange(3)
        W = A + idx * 0x36c8
        n = rng.randrange(1, 11)
        emu.write_u32(W + 4, n)
        arrays = [[rng.uniform(-8, 8) for _ in range(11)] for _ in range(3)]
        for base, vals in zip((0x614, 0x5e8, 0x5bc), arrays):
            for i, v in enumerate(vals):
                emu.write_f32(W + base + 4 * i, v)
        small = rng.random() < 0.3
        f = [rng.uniform(-1, 1) * (0.005 if small else 1) for _ in range(4)]
        for o, v in zip((0x430, 0x434, 0x450, 0x454), f):
            emu.write_f32(F + o, v)
        base = rng.uniform(-500, 500)
        emu.write(F + 0x380, struct.pack('<d', base))
        ref = rng.uniform(-50, 50)
        emu.write_f32(F + 0x42f5c, ref)
        flag['v'] = rng.randrange(2)
        emu.call(0x14121b290, ints=[F, idx])
        print('Q', n, flag['v'], hd(base), *map(hx, f), hx(ref), '|',
              *(hx(v) for a in arrays for v in a), '|', hx(emu.xmm0_f32()))


if __name__ == '__main__':
    main()
