#!/usr/bin/env python3
"""Generates test vectors for profile::outer by running the ORIGINAL profile function 0x141a44350.

Synthetic airfoil objects (1 to 4 tables, tables built by the formulas of gen_profile_vectors.py, header
scalar at +0x5c) are placed in the emulator; the real 0x141a44350 (and the real 0x141a412c0 and helpers it
calls) runs on random arguments, with a synthetic noise table and running time as in gen_profile_vectors.py.

Line: n_tables { k p0 p1 p2 p3 p4 } scalar | x y z retain re mach alpha mult div dac time stall_in |
      cl cd cm ratio ret stall_out        (float32 bit patterns in hex, time as double bits)

    python3 tools/gen_profile_outer_vectors.py Xplane12/X-Plane.exe > tests/data/profile_outer.txt
"""
import random
import struct
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from gen_profile_vectors import NOISE_TABLE, RUNNING_TIME, noise_table, table_arrays  # noqa: E402

FUNC = 0x141a44350


def hx(x):
    return f'{struct.unpack("<I", struct.pack("<f", float(x)))[0]:08x}'


def main():
    emu = Emulator(sys.argv[1])
    emu.write(NOISE_TABLE, noise_table().tobytes())
    rng = random.Random(20261007)
    cache = {}
    outs = emu.alloc(0x100)
    name = emu.alloc(32)
    emu.write(name, b'root foil\0')
    print('# profile outer vectors from the original 0x141a44350 (see tools/gen_profile_outer_vectors.py)')
    count = 0
    while count < 600:
        n = rng.randrange(1, 5)
        F = emu.alloc(0x400)
        T = emu.alloc(0x2230 * n)
        scalar = rng.uniform(0.5, 2.0)
        emu.write_u64(F + 0xd0, T)
        emu.write_u64(F + 0xd8, T + 0x2230 * n)
        emu.write_f32(F + 0x5c, scalar)
        tokens = [str(n)]
        base_p0 = rng.sample([0.05, 0.1, 0.3, 0.5, 1.0, 1.5, 3.0, 5.0], n)
        for t in range(n):
            k = rng.randrange(3)
            p = [base_p0[t], rng.choice([0.0, rng.uniform(0.02, 0.15)]), rng.uniform(-0.5, 0.5),
                 rng.choice([0.0, rng.uniform(8, 25)]), rng.choice([0.0, rng.uniform(8, 25)])]
            if k not in cache:
                cache[k] = table_arrays(k)
            cl, cd, cm = cache[k]
            b = T + 0x2230 * t
            for i, v in enumerate(p):
                emu.write_f32(b + 4 * i, v)
            emu.write(b + 0x14, cl.tobytes())
            emu.write(b + 0xb58, cd.tobytes())
            emu.write(b + 0x169c, cm.tobytes())
            tokens += [str(k), *map(hx, p)]
        tokens.append(hx(scalar))
        x, y, z = rng.uniform(-5, 5), rng.uniform(-5, 5), rng.uniform(-5, 5)
        retain = rng.randrange(2)
        re = rng.choice([rng.uniform(0.01, 8.0), 0.5, 3.0, 0.05, 10.0])
        mach = rng.choice([rng.uniform(-0.2, 1.0), 0.0, 0.3, 0.7, 0.95])
        alpha = rng.choice([rng.uniform(-200, 200), rng.uniform(-30, 30), 0.0, 20.0])
        mult = rng.choice([1.0, rng.uniform(0.4, 1.6)])
        div = rng.choice([1.0, rng.uniform(0.5, 2.0)])
        dac = rng.randrange(2)
        t_sim = rng.choice([0.0, rng.uniform(0, 3000)])
        stall_in = rng.randrange(2)
        emu.write(RUNNING_TIME, struct.pack('<d', t_sim))
        emu.write_u32(outs + 0x40, stall_in)
        emu.call(FUNC, ints=[F, name], floats=[0.0, 0.0, x, y],
                 stack=[z, retain, 0, re, mach, alpha, mult, div, dac, outs, outs + 4, outs + 8, outs + 12, outs + 0x40])
        res = [emu.read_f32(outs + 4 * i) for i in range(4)]
        ret = emu.xmm0_f32()
        tokens += ['|', *map(hx, (x, y, z)), str(retain), *map(hx, (re, mach, alpha, mult, div)), str(dac),
                   f'{struct.unpack("<Q", struct.pack("<d", t_sim))[0]:016x}', str(stall_in), '|',
                   *map(hx, res), hx(ret), str(emu.read_u32(outs + 0x40))]
        print(*tokens)
        count += 1
    print(f'# {count} cases', file=sys.stderr)


if __name__ == '__main__':
    main()
