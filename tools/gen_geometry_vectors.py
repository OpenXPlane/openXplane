#!/usr/bin/env python3
"""Generates test vectors for the small geometry helpers of the wing force function by running the originals:
0x1408be280 (hypot2), 0x14090e310 (hypot3) and 0x141291580 (rotate by three Euler angles).

Lines: H2 x y | r      H3 x y z | r      R a0 a1 a2 | a b c | o1 o2 o3     (float32 hex)
a0, a1, a2 are the object floats at +0x9c, +0xa0, +0xa4 (degrees).

    python3 tools/gen_geometry_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/geometry.txt
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
    rng = random.Random(20261013)
    obj = emu.alloc(0x200)
    outs = [emu.alloc(16) for _ in range(3)]
    print('# geometry helper vectors from the originals (see tools/gen_geometry_vectors.py)')
    for _ in range(300):
        x, y, z = (rng.uniform(-50, 50) for _ in range(3))
        emu.call(0x1408be280, floats=[x, y])
        print('H2', hx(x), hx(y), '|', hx(emu.xmm0_f32()))
        emu.call(0x14090e310, floats=[x, y, z])
        print('H3', hx(x), hx(y), hx(z), '|', hx(emu.xmm0_f32()))
    for _ in range(400):
        ang = [rng.choice([rng.uniform(-180, 180), rng.uniform(-30, 30), 0.0]) for _ in range(3)]
        for off, v in zip((0x9c, 0xa0, 0xa4), ang):
            emu.write_f32(obj + off, v)
        a, b, c = (rng.uniform(-20, 20) for _ in range(3))
        emu.call(0x141291580, ints=[obj, 0, outs[0]], floats=[0.0, a, 0.0, b], stack=[outs[1], c, outs[2]])
        print('R', *map(hx, ang), '|', hx(a), hx(b), hx(c), '|', *[hx(emu.read_f32(o)) for o in outs])


if __name__ == '__main__':
    main()
