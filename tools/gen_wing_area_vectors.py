#!/usr/bin/env python3
"""Generates test vectors for element_area by running the ORIGINAL function 0x1411a0080.

Line: sweep_field span elements chord0 chord1 | area        (float32 bit patterns in hex)

    python3 tools/gen_wing_area_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/wing_area.txt
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
    rng = random.Random(20261008)
    print('# element area vectors from the original 0x1411a0080 (see tools/gen_wing_area_vectors.py)')
    for _ in range(400):
        w = emu.alloc(0x200)
        n = rng.randrange(1, 9)
        sweep, span = rng.uniform(-80, 80), rng.uniform(0.5, 25)
        c0, c1 = rng.uniform(0.1, 4), rng.uniform(0.1, 4)
        emu.write_f32(w + 0x30, sweep)
        emu.write_f32(w + 0x10, span)
        emu.write_u32(w + 4, n)
        emu.write_f32(w + 0x70, c0)
        emu.write_f32(w + 0x74, c1)
        emu.call(0x1411a0080, ints=[w, 0])
        print(hx(sweep), hx(span), n, hx(c0), hx(c1), '|', hx(emu.xmm0_f32()))


if __name__ == '__main__':
    main()
