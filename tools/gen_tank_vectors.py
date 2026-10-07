#!/usr/bin/env python3
"""Vectors for mass::tank_position (0x141a6ba30, with the part transform 0x141294240 and the lag 0x1412941d0).

    python3 tools/gen_tank_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/tank_position.txt

The header of a case is `B tank out`.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])


def main():
    case = VmCase(EXE, SEED, [(0x141a6b000, 0x141a6c000), (0x141294000, 0x141295000)])
    rng = case.rng
    print('# tank position vectors (tools/gen_tank_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        B = case.region('B', 0x6300)
        sp = entry_rsp(0)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)
        L = case.region('L', 0xbd00)
        W = case.region('W', 0x36c8 * 2)
        OUT = case.region('OUT', 16)
        tank = rng.randrange(0, 9)
        fz.preset('B', 0x61f8, L & 0xffffffff)
        fz.preset('B', 0x61fc, L >> 32)
        fz.preset('B', 0x6028, W & 0xffffffff)
        fz.preset('B', 0x602c, W >> 32)
        scale = rng.uniform(0.5, 3)
        size = rng.choice([0.0, rng.uniform(1, 200)])
        fz.preset_f32('B', 0x2888, scale)
        fz.preset_f32('B', 0x3f7c + 4 * tank, size)
        import struct as _s
        capacity = max(_s.unpack('<f', _s.pack('<f', size))[0] * _s.unpack('<f', _s.pack('<f', scale))[0], 0.01)
        fz.preset_f32('L', 0xbbb4 + 4 * tank, rng.choice([0.0, capacity, rng.uniform(0, 1.2) * capacity]))
        for k in range(3):
            fz.preset_f32('B', 0x400c + 12 * tank + 4 * k, rng.uniform(-5, 5))
            fz.preset_f32('B', 0x4078 + 12 * tank + 4 * k, rng.uniform(-5, 5))
        fz.preset('B', 0x40e4 + 4 * tank, rng.choice([-1, 0, 1]) & 0xffffffff)
        for off, lo, hi in ((0x30, -90, 90), (0x20, -90, 90), (0x38, -90, 90), (0x28, -90, 90), (0x40, -3, 3), (0x54, -3, 3), (0x10, -3, 3), (0, -2, 2)):
            for part in range(2):
                fz.preset_f32('W', 0x36c8 * part + off, rng.uniform(lo, hi))
        for part in range(2):
            fz.preset('W', 0x36c8 * part + 0x3c, rng.randrange(-3, 4) & 0xffffffff)
            for off in (0x6f0, 0x6f4, 0x6f8):
                fz.preset_f32('W', 0x36c8 * part + off, rng.uniform(-5, 5))
        for off, lo, hi in ((0x6070, -2, 2), (0x6074, -2, 2), (0x6078, 0.5, 4)):
            fz.preset_f32('B', off, rng.choice([rng.uniform(lo, hi)] + ([0.0] if off == 0x6070 else [])))
        try:
            case.run(0x141a6ba30, ints=[B, tank, OUT], max_instructions=100000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        text = case.dump(f'{B:x} {tank:x} {OUT:x}')
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
