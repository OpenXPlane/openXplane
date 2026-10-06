#!/usr/bin/env python3
"""Generates test vectors for two small wing helpers by running the ORIGINAL functions:
0x1411b1cf0 (angle_shape) and 0x1406ea0b0 (interpolate_clamped).

Lines: S x | result        I a0 v0 a1 v1 x | result      (float32 bit patterns in hex)

    python3 tools/gen_wing_helper_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/wing_helpers.txt
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
    rng = random.Random(20261010)
    print('# wing helper vectors from the originals (see tools/gen_wing_helper_vectors.py)')
    obj = emu.alloc(0x100)
    for _ in range(500):
        x = rng.choice([rng.uniform(-90, 90), rng.uniform(-25, 25), rng.choice([0.0, 20.0, 45.0, -20.0, -45.0, 19.999, 44.999])])
        emu.call(0x1411b1cf0, ints=[obj], floats=[0.0, x])
        print('S', hx(x), '|', hx(emu.xmm0_f32()))
    for _ in range(500):
        a0 = rng.uniform(-5, 5)
        a1 = a0 if rng.random() < 0.1 else rng.uniform(-5, 5)
        v0, v1 = rng.uniform(-3, 3), rng.uniform(-3, 3)
        x = rng.uniform(-8, 8)
        emu.call(0x1406ea0b0, floats=[a0, v0, a1, v1], stack=[x])
        print('I', hx(a0), hx(v0), hx(a1), hx(v1), hx(x), '|', hx(emu.xmm0_f32()))


if __name__ == '__main__':
    main()
