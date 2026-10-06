#!/usr/bin/env python3
"""Generates test vectors for the engine functions ported in crates/xp-airfoil/src/engine.rs by running the
originals: 0x1408625a0 (signed power), 0x14082b800 (curve) and 0x1411e39a0 (ram power factor).

Lines: P x p | r      C a0 v0 a1 v1 x p | r
       R b984 b980 b950 b940 b988 b98c desc1c sigma f64 f400 f41c mach | r       (float32 hex)

    python3 tools/gen_engine_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/engine.txt
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
    rng = random.Random(20261015)
    B = emu.alloc(0x8000)
    desc = emu.alloc(0x68 * 16)
    emu.write_u64(B + 0x5ff8, desc)
    print('# engine function vectors from the originals (see tools/gen_engine_vectors.py)')
    for _ in range(300):
        x = rng.choice([rng.uniform(-3, 3), 0.0, rng.uniform(0, 2)])
        p = rng.choice([rng.uniform(0.2, 3), 1.0, 0.5, 2.0])
        emu.call(0x1408625a0, floats=[x, p])
        print('P', hx(x), hx(p), '|', hx(emu.xmm0_f32()))
    for _ in range(400):
        a0, a1 = rng.uniform(-2, 2), rng.uniform(-2, 2)
        if rng.random() < 0.1:
            a1 = a0
        v0, v1 = rng.uniform(-3, 3), rng.uniform(-3, 3)
        x = rng.uniform(-4, 4)
        p = rng.choice([rng.uniform(0.2, 3), 1.0, 0.5])
        emu.call(0x14082b800, floats=[a0, v0, a1, v1], stack=[x, p])
        print('C', *map(hx, (a0, v0, a1, v1, x, p)), '|', hx(emu.xmm0_f32()))
    for _ in range(600):
        b984 = rng.choice([rng.uniform(0.3, 0.99), rng.uniform(0.3, 0.99), 1.0, rng.uniform(1.0, 1.8)])
        vals = dict(b980=rng.uniform(0.3, 1.2), b950=rng.uniform(0.3, 2.0), b940=rng.uniform(0.2, 3.0),
                    b988=rng.uniform(0.5, 3.0), b98c=rng.uniform(0.5, 3.0))
        idx = rng.randrange(4)
        d1c = rng.uniform(0.05, 5.0)
        sigma, f64v = rng.uniform(0.3, 1.3), rng.uniform(0.2, 3.0)
        f400, f41c = rng.uniform(0.1, 50.0), rng.uniform(0.2, 3.0)
        mach = rng.choice([rng.uniform(0, 0.99), rng.uniform(0, 2.5), rng.uniform(0.1, 3.0)])
        emu.write_f32(B + 0x984, b984)
        for off, k in ((0x980, 'b980'), (0x950, 'b950'), (0x940, 'b940'), (0x988, 'b988'), (0x98c, 'b98c')):
            emu.write_f32(B + off, vals[k])
        emu.write_f32(desc + 0x68 * idx + 0x1c, d1c)
        emu.call(0x1411e39a0, ints=[B, idx], floats=[0, 0, sigma, f64v], stack=[f400, f41c, mach])
        print('R', *map(hx, (b984, vals['b980'], vals['b950'], vals['b940'], vals['b988'], vals['b98c'], d1c, sigma,
                              f64v, f400, f41c, mach)), '|', hx(emu.xmm0_f32()))


if __name__ == '__main__':
    main()
