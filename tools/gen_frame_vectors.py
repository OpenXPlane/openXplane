#!/usr/bin/env python3
"""Generates test vectors for transform::{rotate_pairs, rotate_euler_offset, from_aircraft_frame} and
wing_element::boundary_at by running the originals 0x1407ac180, 0x14120cf60, 0x1407ac020 and 0x1411c5950.

Lines (float32 hex):
  R a b c p7 p8 p9 p10 p11 p12 | o1 o2 o3
  E ang9c anga0 anga4 off90 off94 off98 flag a b c | o1 o2 o3          (a = xmm1, b = xmm3, c = 6th argument)
  A o0 o1 o2 (doubles) s0 c0 s1 c1 s2 c2 shift flag a b c | o1 o2 o3
  B elements u v0 .. v10 | r

    python3 tools/gen_frame_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/frame.txt
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX  # noqa: E402


def hx(x):
    return f'{struct.unpack("<I", struct.pack("<f", float(x)))[0]:08x}'


def hd(x):
    return f'{struct.unpack("<Q", struct.pack("<d", float(x)))[0]:016x}'


def main():
    emu = Emulator(sys.argv[1])
    rng = random.Random(20261021)
    flag = {'v': 0}
    emu.stubs[0x1417f12c0] = lambda e: e.uc.reg_write(UC_X86_REG_RAX, flag['v'])
    F = emu.alloc(0x1000)
    outs = [emu.alloc(16) for _ in range(3)]
    read = lambda: [hx(emu.read_f32(o)) for o in outs]
    print('# frame vectors from the originals (see tools/gen_frame_vectors.py)')
    for _ in range(300):
        a, b, c = (rng.uniform(-20, 20) for _ in range(3))
        p = [rng.uniform(-1, 1) for _ in range(6)]
        emu.call(0x1407ac180, ints=[0, 0, 0, outs[0]], floats=[a, b, c], stack=[outs[1], outs[2], *p])
        print('R', *map(hx, (a, b, c, *p)), '|', *read())
    for _ in range(300):
        ang = [rng.uniform(-180, 180) for _ in range(3)]
        off = [rng.uniform(-50, 50) for _ in range(3)]
        for o, v in zip((0x9c, 0xa0, 0xa4), ang):
            emu.write_f32(F + o, v)
        for o, v in zip((0x90, 0x94, 0x98), off):
            emu.write_f32(F + o, v)
        a, b, c = (rng.uniform(-20, 20) for _ in range(3))
        fl = rng.randrange(2)
        emu.call(0x14120cf60, ints=[F, 0, outs[0]], floats=[0, a, 0, b], stack=[outs[1], c, outs[2], fl])
        print('E', *map(hx, (*ang, *off)), fl, *map(hx, (a, b, c)), '|', *read())
    for _ in range(300):
        origin = [rng.uniform(-3000, 3000) for _ in range(3)]
        rot = [rng.uniform(-1, 1) for _ in range(6)]
        for off, v in zip((0x378, 0x380, 0x388), origin):
            emu.write(F + off, struct.pack('<d', v))
        for off, v in zip((0x440, 0x444, 0x430, 0x434, 0x450, 0x454), rot):
            emu.write_f32(F + off, v)
        a, b, c = (rng.uniform(-100, 100) for _ in range(3))
        shift, flag['v'] = rng.choice([0, 1, 1]), rng.randrange(2)
        emu.call(0x1407ac020, ints=[F, 0, outs[0]], floats=[0, a, 0, b], stack=[outs[1], c, outs[2], shift])
        print('A', *map(hd, origin), *map(hx, rot), shift, flag['v'], *map(hx, (a, b, c)), '|', *read())
    W = emu.alloc(0x1000)
    for _ in range(300):
        els = rng.randrange(1, 11)
        emu.write_u32(W + 4, els)
        vals = [rng.uniform(-5, 5) for _ in range(11)]
        for i, v in enumerate(vals):
            emu.write_f32(W + 0x5bc + 4 * i, v)
        u = rng.choice([rng.uniform(-2, els + 2), rng.uniform(0, els)])
        emu.call(0x1411c5950, ints=[W], floats=[0, u])
        print('B', els, hx(u), *map(hx, vals), '|', hx(emu.xmm0_f32()))


if __name__ == '__main__':
    main()
