#!/usr/bin/env python3
"""Vectors for flight_step::wing_chain_factor (the original 0x14121a9b0).

    python3 tools/gen_chain_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/chain.txt

The header is `W X list_a list_b`.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])


def main():
    case = VmCase(EXE, SEED, [(0x14121a000, 0x14121b000), (0x1406e0000, 0x1406f0000)])
    case.stub(0x1422e7c2c, lambda call, rng: None)
    print('# wing chain factor vectors (tools/gen_chain_vectors.py)')
    rng = case.rng
    done = attempts = 0
    while done < TRIALS and attempts < TRIALS * 20:
        attempts += 1
        case.reset()
        W = case.region('W', 0x36c8 * 3)
        X = case.region('X', 0x2d8 * 3)
        PA = case.region('PA', 0x40)
        PB = case.region('PB', 0x40)
        VA = case.region('VA', 0x20)
        VB = case.region('VB', 0x20)
        fz = case.fz
        count = rng.randrange(0, 4)

        def put64(name, off, value):
            fz.preset(name, off, value & 0xffffffff)
            fz.preset(name, off + 4, value >> 32)

        for k in range(3):
            put64('PA', 8 * k, W + 0x36c8 * k)
            put64('PB', 8 * k, X + 0x2d8 * k)
            fz.preset('W', 0x36c8 * k + 4, rng.randrange(1, 7))
            for off in (0x36b0, 0x36b4, 0x36bc, 0x36c0):
                fz.preset_f32('W', 0x36c8 * k + off, rng.uniform(-3.0, 3.0))
            for off in range(0x5bc, 0x5bc + 4 * 8, 4):
                fz.preset_f32('W', 0x36c8 * k + off, rng.uniform(-3.0, 3.0))
            for off in range(0x5e8, 0x5e8 + 4 * 8, 4):
                fz.preset_f32('W', 0x36c8 * k + off, rng.uniform(-3.0, 3.0))
            for off in range(0x54, 0x54 + 4 * 8, 4):
                fz.preset_f32('X', 0x2d8 * k + off, rng.uniform(0.0, 2.0))
            for off in range(0xf4, 0xf4 + 4 * 8, 4):
                fz.preset_f32('X', 0x2d8 * k + off, rng.uniform(-1.0, 1.0))
        for name, base in (('VA', PA), ('VB', PB)):
            put64(name, 0, base if count or name == 'VB' else 0)
            put64(name, 8, base + 8 * count)
            put64(name, 0x10, base + 0x40)
        if count == 0:
            put64('VA', 0, PA)
        try:
            case.run(0x14121a9b0, ints=[W, X, VA, VB], stack=[0], max_instructions=500000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        done += 1
        print(case.dump(f'{W:x} {X:x} {VA:x} {VB:x}'))


if __name__ == '__main__':
    main()
