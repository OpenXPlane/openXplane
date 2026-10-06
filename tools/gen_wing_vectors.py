#!/usr/bin/env python3
"""Generates test vectors for src/wing_element.rs by running the original 0x1411a0230 and 0x1411a00f0.

The functions read two boundary points of one span element (coordinate arrays at +0x5bc, +0x5e8, +0x614,
chord at +0x70) and two float fields (+0x18, +0x1c) of a wing object. A synthetic object is built in the
emulator for each random case. Output per line (float32 bit patterns in hex):
  x0 x1 y0 y1 z0 z1 c0 c1 f18 f1c | sweep weight

    python3 tools/gen_wing_vectors.py Xplane12/X-Plane.exe > tests/data/wing_geometry.txt
Needs: pip install unicorn numpy
"""
import math
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402

SWEEP = 0x1411a0230
DELTA = 0x1411a00f0


def bits(x):
    return f'{struct.unpack("<I", struct.pack("<f", x))[0]:08x}'


def main():
    emu = Emulator(sys.argv[1])
    wing = emu.alloc(0x7000)
    rng = random.Random(20261006)
    print('# x0 x1 y0 y1 z0 z1 c0 c1 f18 f1c | sweep delta_weight  (f32 bits hex; original 0x1411a0230 / 0x1411a00f0)')
    for _ in range(1500):
        lateral = rng.choice([0.0, rng.uniform(0.05, 4.0)])
        angle = math.radians(rng.choice([rng.uniform(-85, 85), rng.uniform(30, 80), 0.0, 45.0]))
        vertical = rng.uniform(-0.5, 0.5) * lateral
        x0, y0 = rng.uniform(-3, 3), rng.uniform(-1, 1)
        x1, y1 = x0 + lateral, y0 + vertical
        c0, c1 = rng.uniform(0.2, 5.0), rng.uniform(0.2, 5.0)
        dist = math.hypot(x1 - x0, y1 - y0)
        z0 = rng.uniform(-2, 8)
        # place z1 so the quarter-chord line has the chosen sweep
        z1 = z0 - 0.25 * c0 + 0.25 * c1 + dist * math.tan(angle) if dist > 0 else z0 + rng.uniform(-2, 2)
        z1 = max(min(z1, 1e4), -1e4)
        f18 = rng.choice([rng.uniform(-2, 8), 0.0, 1.0, rng.uniform(0, 3)])
        f1c = rng.choice([rng.uniform(0, 1.2), 0.0, 1.0, 0.5])
        for k, (a, b) in enumerate([(x0, x1), (y0, y1), (z0, z1), (c0, c1)]):
            base = [0x5bc, 0x5e8, 0x614, 0x70][k]
            emu.write_f32(wing + base, a)
            emu.write_f32(wing + base + 4, b)
        emu.write_f32(wing + 0x18, f18)
        emu.write_f32(wing + 0x1c, f1c)
        sweep = emu.call_float(SWEEP, ints=[wing, 0])
        weight = emu.call_float(DELTA, ints=[wing, 0])
        print(*(bits(v) for v in (x0, x1, y0, y1, z0, z1, c0, c1, f18, f1c)), '|', bits(sweep), bits(weight))


if __name__ == '__main__':
    main()
