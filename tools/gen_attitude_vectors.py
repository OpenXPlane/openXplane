#!/usr/bin/env python3
"""Vectors for attitude.rs: `e2q` (0x1408816e0), `q2e` (0x140889cb0) and `integrate` (0x140f5fb10).

    python3 tools/gen_attitude_vectors.py Xplane12/X-Plane.exe e2q|q2e|integrate TRIALS SEED > crates/xp-app/tests/data/attitude_KIND.txt

Headers: e2q `a b c out`; q2e `q a b c`; integrate `w1 w2 w3 q a b c` (floats as bits, the rest addresses).
"""
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase  # noqa: E402

EXE, KIND, TRIALS, SEED = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x140880000, 0x14088b000), (0x140f5f000, 0x140f60000), (0x1406e0000, 0x1406f0000)])
    rng = case.rng
    print('# attitude vectors', KIND)
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        if KIND == 'e2q':
            out = case.region('O', 0x10)
            a, b, c = (rng.choice([rng.uniform(-200, 200), rng.uniform(-3, 3), 0.0]) for _ in range(3))
            case.run(0x1408816e0, ints=[0, 0, 0, out], floats=[a, b, c, 0.0])
            print(case.dump(f'{bits(a):08x} {bits(b):08x} {bits(c):08x} {out:x}'))
        elif KIND == 'q2e':
            q = case.region('Q', 0x10)
            outs = [case.region(n, 0x10) for n in ('A', 'B', 'C')]
            for k in range(4):
                fz.preset_f32('Q', 4 * k, rng.choice([rng.uniform(-1, 1), rng.uniform(-3, 3), 0.0, rng.uniform(-0.2, 0.2)]))
            case.run(0x140889cb0, ints=[q, outs[0], outs[1], outs[2]])
            print(case.dump(f'{q:x} ' + ' '.join(f'{o:x}' for o in outs)))
        else:
            q = case.region('Q', 0x10)
            outs = [case.region(n, 0x10) for n in ('A', 'B', 'C')]
            w = [rng.choice([rng.uniform(-0.2, 0.2), rng.uniform(-3, 3), 0.0]) for _ in range(3)]
            norm = [rng.uniform(-1, 1) for _ in range(4)]
            length = math.sqrt(sum(v * v for v in norm)) or 1.0
            for k in range(4):
                fz.preset_f32('Q', 4 * k, norm[k] / length * rng.choice([1.0, 1.0, 1.0, 0.5]))
            case.run(0x140f5fb10, ints=[0, 0, 0, outs[0]], floats=[w[0], w[1], w[2], 0.0],
                     stack=[outs[1], outs[2], q])
            print(case.dump(' '.join(f'{bits(v):08x}' for v in w) + f' {q:x} ' + ' '.join(f'{o:x}' for o in outs)))
        done += 1


if __name__ == '__main__':
    main()
