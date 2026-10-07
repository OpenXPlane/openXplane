#!/usr/bin/env python3
"""Vectors for mass::mass_point (the original 0x1411dd820, one point mass of the weight and balance).

    python3 tools/gen_mass_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/mass_point.txt

The header of a case is `v a b c mode w6 w9 w12 total i7 i10 i13 m8 m11 m14` (floats as bits, the accumulators as
addresses).
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x1411dd000, 0x1411de000)])
    rng = case.rng
    print('# mass point vectors (tools/gen_mass_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        sp = entry_rsp(13)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)
        A = case.region('ACC', 0x100)
        for off in range(0, 0x40, 4):
            fz.preset_f32('ACC', off, rng.uniform(-50, 50))
        total, i7, i10, i13, m8, m11, m14 = (A + o for o in (0, 4, 8, 12, 16, 20, 24))
        v, a, b, c = (rng.choice([0.0, rng.uniform(-3, 3), rng.uniform(-300, 300)]) for _ in range(4))
        w6, w9, w12 = (rng.uniform(-1, 1) for _ in range(3))
        mode = rng.choice([0, 1, 1, 2])
        # arg5 total, arg6 w6, arg7 i7, arg8 m8, arg9 w9, arg10 i10, arg11 m11, arg12 w12, arg13 i13, arg14 m14,
        # arg15 log flag, arg16 label, arg17 mode
        stack = [total, bits(w6), i7, m8, bits(w9), i10, m11, bits(w12), i13, m14, 0, A + 0x80, mode]
        for k, value in enumerate(stack):
            off = sp + 0x28 + 8 * k - (sp - 0x800)
            fz.preset('S', off, value & 0xffffffff)
            fz.preset('S', off + 4, value >> 32)
        try:
            case.run(0x1411dd820, floats=[v, a, b, c], stack=stack, until=None, max_instructions=100000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        header = ' '.join(f'{x:x}' for x in (bits(v), bits(a), bits(b), bits(c), mode, bits(w6), bits(w9), bits(w12), total, i7, i10, i13, m8, m11, m14))
        text = case.dump(header)
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
