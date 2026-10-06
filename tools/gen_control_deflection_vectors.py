#!/usr/bin/env python3
"""Generates test vectors for control_deflection by running the ORIGINAL function 0x141192870.

Line: code index first last a b chord0..chord19 | result     (float32 bit patterns in hex; code, index decimal)

    python3 tools/gen_control_deflection_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/control_deflection.txt
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402

TABLE = {
    0xb: (0x324, 0x328, 0x1dfc, 0x1e00), 0xc: (0x354, 0x358, 0x1e0c, 0x1e10),
    0xd: (0x444, 0x448, 0x1e18, 0x1e1c), 0xe: (0x474, 0x478, 0x1e28, 0x1e2c),
    0xf: (0x4a4, 0x4a8, 0x1e3c, 0x1e40), 0x10: (0x384, 0x388, 0x1e4c, 0x1e50),
    0x11: (0x3b4, 0x3b8, 0x1e60, 0x1e64), 0x12: (0x3e4, 0x3e8, 0x1e74, 0x1e78),
    0x13: (0x414, 0x418, 0x1e88, 0x1e8c), 0x14: (0x534, 0x538, 0x1e9c, 0x1ea0),
    0x15: (0x564, 0x568, 0x1ec4, 0x1ec8), 0x16: (0x4d4, 0x4d8, 0x1ee0, 0x1ee4),
    0x17: (0x504, 0x508, 0x1ef0, 0x1ef4),
}


def hx(x):
    return f'{struct.unpack("<I", struct.pack("<f", float(x)))[0]:08x}'


def main():
    emu = Emulator(sys.argv[1])
    rng = random.Random(20261009)
    print('# control deflection vectors from the original 0x141192870 (see tools/gen_control_deflection_vectors.py)')
    for n in range(650):
        code = list(TABLE)[n % 13]
        fa, la, aa, ba = TABLE[code]
        wing, ctrl = emu.alloc(0x800), emu.alloc(0x2000)
        first = rng.randrange(0, 18)
        last = first if rng.random() < 0.2 else rng.randrange(first, 20)
        index = rng.randrange(0, 20)
        a, b = rng.uniform(-1, 1), rng.uniform(-1, 1)
        chords = [rng.uniform(0.1, 3) for _ in range(20)]
        emu.write_f32(wing + fa, first)
        emu.write_f32(wing + la, last)
        emu.write_f32(ctrl + aa, a)
        emu.write_f32(ctrl + ba, b)
        for k, c in enumerate(chords):
            emu.write_f32(wing + 0x70 + 4 * k, c)
        emu.call(0x141192870, ints=[ctrl, wing, index, code])
        print(code, index, hx(first), hx(last), hx(a), hx(b), *map(hx, chords), '|', hx(emu.xmm0_f32()))


if __name__ == '__main__':
    main()
