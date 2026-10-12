#!/usr/bin/env python3
"""Vectors for engine_small (0x1411dd610 ceiling, 0x1411b0a30 lever, 0x1411d9d80 pilot activity).

    python3 tools/gen_small_vectors.py Xplane12/X-Plane.exe KIND TRIALS SEED > crates/xp-app/tests/data/small_KIND.txt

KIND is `ceiling`, `lever`, `pilot` or `wave` (header `x-bits seed result`); the header of a case is `F-or-B n result` (a float result as bits).
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_XMM0  # noqa: E402
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, KIND, TRIALS, SEED = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
ADDRESS = {'ceiling': 0x1411dd610, 'lever': 0x1411b0a30, 'pilot': 0x1411d9d80, 'wave': 0x1408bd9d0}[KIND]


def main():
    case = VmCase(EXE, SEED, [(0x1411dd000, 0x1411de000), (0x1411b0000, 0x1411b1000), (0x1411d9000, 0x1411da000)])
    rng = case.rng
    print(f'# small vectors {KIND} (tools/gen_small_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        F = case.region('F', 0x7000)
        B = case.region('B', 0x6700)
        sp = entry_rsp(0)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)

        def put(name, off, v):
            fz.preset(name, off, v & 0xffffffff)
            fz.preset(name, off + 4, v >> 32)
        put('F', 0x20, B)
        n = rng.randrange(0, 3)
        if KIND == 'ceiling':
            E = case.region('E', 0x2cc * 3)
            put('F', 0x68b0, E)
            for k in range(3):
                fz.preset_f32('E', 0x2cc * k + 0x4c, rng.choice([0.0, 1.0, rng.uniform(-0.5, 1.5)]))
            fz.preset('B', 0xa98, rng.choice([0, 1]))
            fz.preset('F', 0x24c, rng.choice([0, 1]))
            lo = rng.uniform(-1, 1)
            fz.preset_f32('B', 0x9d8, lo)
            fz.preset_f32('B', 0x9dc, rng.choice([lo, rng.uniform(-1, 2)]))
            run = (F, n)
        elif KIND == 'lever':
            R = case.region('R', 0x68 * 3)
            put('B', 0x5ff8, R)
            for k in range(3):
                fz.preset('R', 0x68 * k, rng.randrange(0, 10))
                for off in (4, 8, 0x1c, 0x24):
                    fz.preset_f32('R', 0x68 * k + off, rng.uniform(-3, 3))
            for off in (0xb74, 0x928, 0xb70, 0xb64, 0xb08, 0xb24):
                fz.preset_f32('B', off, rng.uniform(-3, 3))
            run = (B, n)
        elif KIND == 'wave':
            fz.region('NZ', 0x14578f1f0, 0x100000)
            x = rng.choice([rng.uniform(-3, 3), rng.uniform(-300, 300), rng.uniform(0, 5)])
            x = struct.unpack('<f', struct.pack('<f', x))[0]
            seed = rng.randrange(0, 40)
            run = (struct.unpack('<I', struct.pack('<f', x))[0], seed)
            for scale in (1, 2, 4, 8, 16, 32):
                v = struct.unpack('<f', struct.pack('<f', abs(x) * scale))[0]
                i = int(v) & 0xffffffff
                for offset in (0, 1):
                    index = ((seed * 100 + i + offset) & 0xffffffff) & 0x3ffff
                    fz.preset_f32('NZ', 4 * index, rng.uniform(0.0, 1.0))
        else:
            fz.region('KT', 0x1460e9708, 4 * 500)
            fz.region('KF', 0x1460e9708 + 0xba44, 4 * 500)
            fz.region('IG', 0x1460e0a1c, 0x54)
            hot = rng.randrange(0, 500)
            for k in range(500):
                a, b = rng.choice([0, 0, 0, 0, 0, 0, 0, 1, 2, 5, 7]), rng.choice([0, 0, 0, 0, 0, 1])
                fz.preset('KT', 4 * k, a, record=a != 0)
                fz.preset('KF', 4 * k, b, record=b != 0)
            if rng.random() < 0.5:
                fz.preset('KT', 4 * hot, rng.choice([3, 0x46, 0x4a, 0x4b]))
                fz.preset('KF', 4 * hot, 1)
            for off in (0, 0x4c, 0x50):
                fz.preset('IG', off, rng.choice([0, 0, 0, 0, 1]))
            fz.preset('F', 0x3c, rng.choice([0, 1, 1, 1, 1, 1, 1, 1]))
            fz.preset('F', 0x28, rng.choice([0, 0, 1]))
            fz.preset('F', 0x67cc, rng.choice([0, 1]))
            fz.preset('F', 0x67fc, rng.choice([0, 1]))
            run = (F, 0)
        try:
            if KIND == 'wave':
                case.run(ADDRESS, ints=[0, run[1], 0, 0], floats=[struct.unpack('<f', struct.pack('<I', run[0]))[0]], max_instructions=200000)
            else:
                case.run(ADDRESS, ints=[run[0], run[1], 0, 0], max_instructions=200000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        if KIND == 'pilot':
            result = case.emu.reg(UC_X86_REG_RAX) & 0xff
        else:
            result = case.emu.uc.reg_read(UC_X86_REG_XMM0) & 0xffffffff
        text = case.dump(f'{run[0]:x} {run[1]:x} {result:x}')
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
