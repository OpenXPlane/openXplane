#!/usr/bin/env python3
"""Vectors for mass::weight_and_balance (the original 0x1411ddfd0, the sum of the aircraft's masses and inertia).

    python3 tools/gen_balance_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/balance.txt

The header of a case is `F acc w6 w9 w12 stop` (the floats as bits). The registry lookups 0x140f5c540/0x140f42620
and the binding query 0x1407ace10 are replayed; the reference counting and the label strings are no-ops.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from xp_vmcase import VmCase, entry_rsp, setup_balance  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def f32(v):
    return struct.unpack('<f', struct.pack('<f', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x1411dd000, 0x1411df000), (0x141a6b000, 0x141a6c000), (0x141294000, 0x141295000)])
    rng = case.rng
    print('# balance vectors (tools/gen_balance_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        F = case.region('F', 0xc000)
        B = case.region('B', 0x6300)
        sp = entry_rsp(8)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)
        ACC = case.region('ACC', 0x40)
        setup_balance(case, rng, F, B)
        w6, w9, w12 = (rng.uniform(-1, 1) for _ in range(3))
        stop = rng.choice([0, 0, 0, 1])
        total, i7, i10, i13, m8, m11, m14 = (ACC + o for o in (0, 4, 8, 12, 16, 20, 24))
        for off in range(0, 0x20, 4):
            fz.preset_f32('ACC', off, rng.uniform(-50, 50))
        stack = [m8, bits(w9), i10, m11, bits(w12), i13, m14, stop]
        for k, value in enumerate(stack):
            off = sp + 0x28 + 8 * k - (sp - 0x800)
            fz.preset('S', off, value & 0xffffffff)
            fz.preset('S', off + 4, value >> 32)
        try:
            case.run(0x1411ddfd0, ints=[F, total, 0, i7], floats=[0.0, 0.0, w6, 0.0], stack=stack, max_instructions=400000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        text = case.dump(f'{F:x} {ACC:x} {bits(w6):x} {bits(w9):x} {bits(w12):x} {stop:x}')
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
