#!/usr/bin/env python3
"""Generates test vectors for wing_element::signed_sqrt (0x14122d4b0) and element_dihedral (0x1411a02f0).

Lines: Q x | r        D x0 x1 y0 y1 z0 z1 | r      (float32 hex)

    python3 tools/gen_wing_misc_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/wing_misc.txt
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402


def hx(x):
    return f'{struct.unpack("<I", struct.pack("<f", float(x)))[0]:08x}'


def main():
    emu = Emulator(sys.argv[1])
    rng = random.Random(20261020)
    W = emu.alloc(0x1000)
    print('# wing helper vectors (see tools/gen_wing_misc_vectors.py)')
    for _ in range(300):
        x = rng.choice([rng.uniform(-9, 9), 0.0, -0.0, rng.uniform(0, 100)])
        emu.call(0x14122d4b0, floats=[x])
        print('Q', hx(x), '|', hx(emu.xmm0_f32()))
    for _ in range(400):
        i = rng.randrange(0, 8)
        v = [rng.uniform(-6, 6) for _ in range(6)]
        for base, (a, b) in zip((0x5bc, 0x5e8, 0x614), ((v[0], v[1]), (v[2], v[3]), (v[4], v[5]))):
            emu.write_f32(W + base + 4 * i, a)
            emu.write_f32(W + base + 4 * (i + 1), b)
        emu.call(0x1411a02f0, ints=[W, i])
        print('D', *map(hx, (v[0], v[1], v[2], v[3], v[4], v[5])), '|', hx(emu.xmm0_f32()))


if __name__ == '__main__':
    main()
