#!/usr/bin/env python3
"""Generate test vectors for the profile stage by running the ORIGINAL machine code of 0x141a412c0.

The function (angle correction, fixed-grid lookup, normalised angle, stall flag, active-stall noise
perturbation) is executed in the Unicorn emulator (tools/emulate_xp.py) on synthetic airfoil tables and
a synthetic noise table, and its outputs are written as bit patterns. The Rust test
crates/xp-app/tests/original_vectors.rs recomputes every case with the port and requires identical bits.

Synthetic data (so both sides build the same inputs without shipping any airfoil file):
  table k : Cl[i] = ((i*131 + k*977) % 4001 - 2000) / 1000
            Cd[i] = ((i*37 + k*53) % 997) / 10000
            Cm[i] = ((i*61 + k*389) % 2003 - 1000) / 5000
  noise   : v[i] = ((i * 2654435761) mod 2^32 >> 8) / 2^24 * 2 - 1          (262144 float32 values)

Usage: python3 tools/gen_profile_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/profile_stage.txt
Needs: pip install unicorn numpy
"""
import random
import struct
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402

F = np.float32
FUNC = 0x141a412c0
NOISE_TABLE = 0x14578f1f0
RUNNING_TIME = 0x142f01918
ROWS = 721
TABLES = 3


def bits(x):
    return struct.unpack('<I', struct.pack('<f', float(x)))[0]


def table_arrays(k):
    i = np.arange(ROWS, dtype=np.int64)
    cl = (((i * 131 + k * 977) % 4001 - 2000).astype(np.float32)) / F(1000)
    cd = (((i * 37 + k * 53) % 997).astype(np.float32)) / F(10000)
    cm = (((i * 61 + k * 389) % 2003 - 1000).astype(np.float32)) / F(5000)
    return cl, cd, cm


def noise_table():
    i = np.arange(262144, dtype=np.uint64)
    h = ((i * np.uint64(2654435761)) & np.uint64(0xffffffff)) >> np.uint64(8)
    return (h.astype(np.float32) / F(16777216)) * F(2) - F(1)


def main():
    emu = Emulator(sys.argv[1])
    emu.write(NOISE_TABLE, noise_table().tobytes())
    tables = []
    for k in range(TABLES):
        addr = emu.alloc(0x2230)
        cl, cd, cm = table_arrays(k)
        emu.write(addr + 0x14, cl.tobytes())
        emu.write(addr + 0xb58, cd.tobytes())
        emu.write(addr + 0x169c, cm.tobytes())
        tables.append(addr)
    outs = emu.alloc(64)  # norm, stall, cl, cd, cm
    rng = random.Random(20261006)
    print('# profile stage vectors from the original 0x141a412c0 (see tools/gen_profile_vectors.py)')
    print('# table p0 p1 p2 p3 p4 alpha mult div retain prev_stall x y phase time_f64 | norm stall cl cd cm  (f32 bits hex, time f64 bits)')
    count = 0
    for case in range(1200):
        k = rng.randrange(TABLES)
        p = [rng.uniform(0.1, 3.0), rng.choice([0.0, rng.uniform(0.02, 0.15)]), rng.uniform(-0.5, 0.5),
             rng.choice([0.0, rng.uniform(8, 25)]), rng.choice([0.0, rng.uniform(8, 25)])]
        alpha = rng.choice([rng.uniform(-200, 200), rng.uniform(-30, 30), rng.uniform(-3, 3),
                            float(rng.randrange(-180, 181)), 20.0, -20.0, 0.0])
        mult = rng.choice([1.0, rng.uniform(0.2, 2.0)])
        div = rng.choice([1.0, 2.0, rng.uniform(0.3, 3.0)])
        retain = rng.randrange(2)
        prev = rng.randrange(2)
        x, y, phase = (rng.uniform(-50, 50), rng.uniform(-50, 50), rng.uniform(0, 10))
        t = rng.choice([0.0, rng.uniform(0, 5000), 16777217.0])
        a = tables[k]
        for i, v in enumerate(p):
            emu.write_f32(a + 4 * i, v)
        emu.write(RUNNING_TIME, struct.pack('<d', t))
        emu.write_u32(outs + 4, prev)
        emu.call(FUNC, ints=[a], floats=[0.0, x, y, phase],
                 stack=[retain, 0, alpha, mult, div, outs, outs + 4, outs + 8, outs + 12, outs + 16])
        norm = emu.read_f32(outs)
        stall = emu.read_u32(outs + 4)
        cl, cd, cm = (emu.read_f32(outs + 8), emu.read_f32(outs + 12), emu.read_f32(outs + 16))
        fb = lambda v: f'{bits(v):08x}'
        print(k, *(fb(v) for v in p), fb(alpha), fb(mult), fb(div), retain, prev,
              fb(x), fb(y), fb(phase), f'{struct.unpack("<Q", struct.pack("<d", t))[0]:016x}', '|',
              fb(norm), stall, fb(cl), fb(cd), fb(cm))
        count += 1
    print(f'# {count} cases', file=sys.stderr)


if __name__ == '__main__':
    main()
