#!/usr/bin/env python3
"""Vectors for the scalar helpers of xp-airfoil::scalar from the originals 0x14081dfa0 (clamp), 0x140910be0
(sign), 0x1411b4a20 (max3), 0x1411b03f0 (snap), 0x140819240 (lerp) and 0x1411e26e0 (kind_is_3_or_7).

Lines: C x lo hi | r      S x | r      M a b c | r      N x lo hi | r      L a b t | r      K kind | r

    python3 tools/gen_scalar_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/scalar.txt
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
    rng = random.Random(20261027)

    def value():
        r = rng.random()
        if r < 0.05:
            return float('nan')
        if r < 0.15:
            return rng.choice([0.0, 1.0, -1.0, 0.5])
        return rng.uniform(-3, 3)

    F = emu.alloc(64)
    print('# scalar helper vectors (tools/gen_scalar_vectors.py)')
    for _ in range(300):
        x, lo, hi = value(), value(), value()
        print('C', *map(hx, (x, lo, hi)), '|', hx(emu.call_float(0x14081dfa0, floats=[x, lo, hi])))
        print('S', hx(x), '|', hx(emu.call_float(0x140910be0, floats=[x])))
        print('M', *map(hx, (x, lo, hi)), '|', hx(emu.call_float(0x1411b4a20, floats=[x, lo, hi])))
        print('N', *map(hx, (x, lo, hi)), '|', hx(emu.call_float(0x1411b03f0, floats=[x, lo, hi])))
        print('L', *map(hx, (x, lo, hi)), '|', hx(emu.call_float(0x140819240, floats=[x, lo, hi])))
    for kind in list(range(-3, 20)) + [0x7fffffff, -0x80000000]:
        emu.write_u32(F, kind & 0xffffffff)
        print('K', kind, '|', emu.call(0x1411e26e0, ints=[F]) & 0xff)


if __name__ == '__main__':
    main()
