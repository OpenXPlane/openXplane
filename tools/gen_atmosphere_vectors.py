#!/usr/bin/env python3
"""Generates test vectors for the atmosphere accessors by running the ORIGINALS: 0x141ba6750 (temperature,
table path), 0x141baf820 (pressure) and 0x141ba64e0 (density ratio), with a synthetic table at 0x14612bd90.

First line: TABLE p0 t0 p1 t1 ...   (0x803 entries, float32 hex)
Then:  T alt obj64 | result     P alt obj98 | result     D alt celsius obj98 obj9c | result

    python3 tools/gen_atmosphere_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/atmosphere.txt
"""
import math
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402

TABLE = 0x14612bd90


def hx(x):
    return f'{struct.unpack("<I", struct.pack("<f", float(x)))[0]:08x}'


def main():
    emu = Emulator(sys.argv[1])
    rng = random.Random(20261018)
    entries = []
    for i in range(0x803):
        h = i * 100 - 5000
        p = 101325.0 * math.exp(-h / 8434.0) * rng.uniform(0.98, 1.02)
        t = max(-56.5, 15.0 - 0.0065 * h) + rng.uniform(-2, 2)
        entries.append((p, t))
    emu.write(TABLE, b''.join(struct.pack('<ff', p, t) for p, t in entries))
    print('TABLE', *[f'{hx(p)} {hx(t)}' for p, t in entries])
    obj = emu.alloc(0x200)
    for _ in range(400):
        alt = rng.choice([rng.uniform(-6000, 12000), rng.uniform(0, 4000), rng.uniform(-100000, 300000)])
        o64 = rng.choice([-300.0, -273.15, -400.0])
        emu.write_f32(obj + 0x64, o64)
        emu.write_f32(obj + 0x98, rng.uniform(90000, 105000))
        emu.write_f32(obj + 0x9c, rng.uniform(-100, 3000))
        emu.call(0x141ba6750, ints=[obj], floats=[0, alt])
        print('T', hx(alt), hx(o64), '|', hx(emu.xmm0_f32()))
        # the C runtime's power function fails (domain error) for some inputs; those cases are dropped
        try:
            emu.call(0x141baf820, ints=[obj], floats=[0, alt])
            print('P', hx(alt), hx(emu.read_f32(obj + 0x98)), '|', hx(emu.xmm0_f32()))
        except RuntimeError:
            pass
        celsius = rng.uniform(-60, 45)
        try:
            emu.call(0x141ba64e0, ints=[obj], floats=[0, alt, celsius])
            print('D', hx(alt), hx(celsius), hx(emu.read_f32(obj + 0x98)), hx(emu.read_f32(obj + 0x9c)), '|', hx(emu.xmm0_f32()))
        except RuntimeError:
            pass


if __name__ == '__main__':
    main()
