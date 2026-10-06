#!/usr/bin/env python3
"""Generates test vectors for fuel::Tanks::draw by running the ORIGINAL 0x14117c380.

Line: flag18 flag1c used34 used38 used3c cap40 cap44 cap48 amount interval mode | result used34' used38' used3c'
(floats as float32 hex, flags and mode decimal)

    python3 tools/gen_fuel_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/fuel.txt
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
    rng = random.Random(20261017)
    obj = emu.alloc(0x100)
    print('# fuel tank vectors from the original 0x14117c380 (see tools/gen_fuel_vectors.py)')
    for _ in range(1500):
        flags = [rng.randrange(2), rng.randrange(2)]
        used = [rng.uniform(0, 30) for _ in range(3)]
        cap = [rng.uniform(5, 60) for _ in range(3)]
        amount = rng.uniform(0, 0.05) * rng.choice([1, 10, 100])
        interval = rng.choice([1.0, 0.5, rng.uniform(0.1, 3)])
        mode = rng.choice([1, 2, 3, 5, 5, 4, 0, 1, 2, 3])
        emu.write_u32(obj + 0x18, flags[0])
        emu.write_u32(obj + 0x1c, flags[1])
        for off, v in zip((0x34, 0x38, 0x3c), used):
            emu.write_f32(obj + off, v)
        for off, v in zip((0x40, 0x44, 0x48), cap):
            emu.write_f32(obj + off, v)
        result = emu.call(0x14117c380, ints=[obj, 0, mode], floats=[0, amount, 0, interval]) & 0xff
        after = [emu.read_f32(obj + off) for off in (0x34, 0x38, 0x3c)]
        print(*flags, *map(hx, used), *map(hx, cap), hx(amount), hx(interval), mode, '|', result, *map(hx, after))


if __name__ == '__main__':
    main()
