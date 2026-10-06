#!/usr/bin/env python3
"""Vectors for functions of the engine family taking (M, F, e): 0x1411975a0 (apply_engine_thrust).

    python3 tools/gen_engine_funcs_vectors.py Xplane12/X-Plane.exe FUNC TRIALS SEED > crates/xp-app/tests/data/engine_funcs_FUNC.txt
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase  # noqa: E402

EXE, FUNC, TRIALS, SEED = sys.argv[1], int(sys.argv[2], 16), int(sys.argv[3]), int(sys.argv[4])


def main():
    case = VmCase(EXE, SEED, [(0x141190000, 0x1411a0000), (0x1411760000 >> 4, 0x141177100)])
    print('# engine function vectors', hex(FUNC))
    for _ in range(TRIALS):
        case.reset()
        F = case.region('F', 0xc000)
        B = case.region('B', 0x7000)
        P = case.region('P', 0x3770 * 4)
        M = case.region('M', 0x2cc * 4)
        fz = case.fz
        fz.preset('F', 0x20, B & 0xffffffff)
        fz.preset('F', 0x24, B >> 32)
        fz.preset('B', 0x6010, P & 0xffffffff)
        fz.preset('B', 0x6014, P >> 32)
        for off in (0xbcc8, 0xbcd0):
            fz.preset('F', off, 0)
        e = case.rng.randrange(3)
        case.run(FUNC, ints=[M + 0x2cc * e, F, e])
        print(case.dump(f'{M + 0x2cc * e:x} {F:x} {e}'))


if __name__ == '__main__':
    main()
