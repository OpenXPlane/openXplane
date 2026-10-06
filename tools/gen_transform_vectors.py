#!/usr/bin/env python3
"""Generates test vectors for transform::to_aircraft_frame by running the ORIGINAL 0x141296750.

The engine-flag test 0x1417f12c0 is replaced by a stub returning a chosen value.

Line: shift flag | o0 o1 o2 (double hex) | s0 c0 s1 c1 s2 c2 | x y z | out1 out2 out3     (float32 hex)

    python3 tools/gen_transform_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/transform.txt
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
    rng = random.Random(20261014)
    flag = {'v': 0}
    emu.stubs[0x1417f12c0] = lambda e: e.uc.reg_write(UC_X86_REG_RAX, flag['v'])
    F = emu.alloc(0x1000)
    outs = [emu.alloc(16) for _ in range(3)]
    print('# transform vectors from the original 0x141296750 (see tools/gen_transform_vectors.py)')
    for _ in range(500):
        origin = [rng.choice([rng.uniform(-5000, 5000), 0.0]) for _ in range(3)]
        rot = [rng.uniform(-1, 1) for _ in range(6)]
        for off, v in zip((0x378, 0x380, 0x388), origin):
            emu.write(F + off, struct.pack('<d', v))
        for off, v in zip((0x430, 0x434, 0x440, 0x444, 0x450, 0x454), rot):
            emu.write_f32(F + off, v)
        x, y, z = (rng.uniform(-100, 100) for _ in range(3))
        shift = rng.choice([0, 1, 1])
        flag['v'] = rng.randrange(2)
        emu.call(0x141296750, ints=[F, 0, outs[0]], floats=[0.0, x, 0.0, y], stack=[outs[1], z, outs[2], shift])
        print(shift, flag['v'], '|', *map(hd, origin), '|', *map(hx, rot), '|', hx(x), hx(y), hx(z), '|',
              *[hx(emu.read_f32(o)) for o in outs])


if __name__ == '__main__':
    main()
