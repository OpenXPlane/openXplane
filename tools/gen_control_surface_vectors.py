#!/usr/bin/env python3
"""Generates test vectors for control_surface_terms by running the ORIGINAL function 0x141221220.

The input-binding query 0x1407ace10 is replaced by a stub that returns bit (id - 0x2d9) of a mask word, so the
vectors cover both driven and undriven paths. The aircraft (A), wing (W) and element (X) objects are filled
only at the offsets the function reads (see control_surface_terms for their names).

Line: code element mask | first last a b | w0 w20 x2c x1bc angle | mode1d20 mode1d24 mode1d28 kind index |
      ratios[6] | table_a[5] table_b[5] | chord[20] | out_old[4] | out_new[4]     (floats as hex f32)

    python3 tools/gen_control_surface_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/control_surface.txt
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from gen_control_deflection_vectors import TABLE  # noqa: E402

FUNC = 0x141221220
QUERY = 0x1407ace10
MASK_AT = QUERY + 0x80


def hx(x):
    return f'{struct.unpack("<I", struct.pack("<f", float(x)))[0]:08x}'


def install_stub(emu):
    # mov ecx,r8d; sub ecx,0x2d9; mov eax,[rip+mask]; shr eax,cl; and eax,1; ret
    code = bytes([0x44, 0x89, 0xc1, 0x81, 0xe9, 0xd9, 0x02, 0x00, 0x00, 0x8b, 0x05])
    disp = MASK_AT - (QUERY + len(code) + 4)
    code += struct.pack('<i', disp) + bytes([0xd3, 0xe8, 0x83, 0xe0, 0x01, 0xc3])
    emu.write(QUERY, code)


def main():
    emu = Emulator(sys.argv[1])
    install_stub(emu)
    rng = random.Random(20261011)
    print('# control surface vectors from the original 0x141221220 (see tools/gen_control_surface_vectors.py)')
    dbg = emu.alloc(0xbd00)
    for n in range(1200):
        code = list(TABLE)[n % 13]
        a, w, x = emu.alloc(0x6300), emu.alloc(0x800), emu.alloc(0x800)
        emu.write_u64(a + 0x61f8, dbg)
        mask = rng.choice([0, rng.getrandbits(24), rng.getrandbits(24) & rng.getrandbits(24), 0xffffff])
        emu.write_u32(MASK_AT, mask)
        elem = rng.randrange(0, 8)
        fa, la, aa, ba = TABLE[code]
        first = rng.randrange(0, 8)
        last = first if rng.random() < 0.2 else rng.randrange(first, 8)
        ea, eb = rng.uniform(-1, 1), rng.uniform(-1, 1)
        emu.write_f32(w + fa, first)
        emu.write_f32(w + la, last)
        emu.write_f32(a + aa, ea)
        emu.write_f32(a + ba, eb)
        chords = [rng.uniform(0.2, 3) for _ in range(20)]
        for k, ch in enumerate(chords):
            emu.write_f32(w + 0x70 + 4 * k, ch)
        w0 = rng.choice([rng.uniform(-1, 1), rng.uniform(-1, 1), 0.0])
        w20 = rng.choice([rng.uniform(0, 90), rng.uniform(0, 45), 45.0, rng.uniform(-90, 0)])
        emu.write_f32(w, w0)
        emu.write_f32(w + 0x20, w20)
        x2c, x1bc = rng.uniform(-2, 2), rng.choice([rng.uniform(-1.5, 1.5), rng.uniform(-0.3, 0.3)])
        emu.write_f32(x + 0x2c + 4 * elem, x2c)
        emu.write_f32(x + 0x1bc + 4 * elem, x1bc)
        angle = rng.choice([rng.uniform(-60, 60), rng.uniform(-25, 25), rng.uniform(-10, 10), 0.0, rng.uniform(-170, 170)])
        modes = [rng.randrange(0, 5) for _ in range(3)]
        kind = rng.randrange(0, 7)
        index = rng.randrange(0, 5)
        for off, v in zip((0x1d20, 0x1d24, 0x1d28), modes):
            emu.write_u32(a + off, v)
        emu.write_u32(a + 0x1f74, kind)
        emu.write_u64(a + 0x1eb0, index)
        ratios = [rng.uniform(-2, 2) for _ in range(6)]
        for off, v in zip((0x1f78, 0x1f7c, 0x1f80, 0x1fb8, 0x1fbc, 0x1fc0), ratios):
            emu.write_f32(a + off, v)
        ta = [rng.choice([rng.uniform(-60, 60), rng.uniform(-0.5, 0.5), 0.0]) for _ in range(5)]
        tb = [rng.choice([rng.uniform(-60, 60), rng.uniform(-0.5, 0.5), 0.0]) for _ in range(5)]
        for k in range(5):
            emu.write_f32(a + 0x1f84 + 4 * k, ta[k])
            emu.write_f32(a + 0x1fc4 + 4 * k, tb[k])
        outs = [emu.alloc(16) for _ in range(4)]
        old = [rng.uniform(-3, 3) for _ in range(4)]
        for o, v in zip(outs, old):
            emu.write_f32(o, v)
        emu.call(FUNC, ints=[a, w, x, elem], stack=[code, angle, *outs, 0])
        new = [emu.read_f32(o) for o in outs]
        print(code, elem, f'{mask:06x}', '|', *map(hx, (first, last, ea, eb)), '|', *map(hx, (w0, w20, x2c, x1bc, angle)),
              '|', *modes, kind, index, '|', *map(hx, ratios), '|', *map(hx, ta), *map(hx, tb), '|', *map(hx, chords),
              '|', *map(hx, old), '|', *map(hx, new))


if __name__ == '__main__':
    main()
