#!/usr/bin/env python3
"""Vectors for controls::engine_controls (the original 0x141260090) up to a checkpoint.

    python3 tools/gen_controls_vectors.py Xplane12/X-Plane.exe STAGE TRIALS SEED > crates/xp-app/tests/data/controls_N.txt
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, STAGE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
UNTIL = {1: 0x1412605eb, 2: 0x1412609bb, 3: 0x141262177, 4: None}
FUNC = 0x141260090


def bool_stub(call, rng):
    call.ret_int(rng.randrange(2))


def void_stub(call, rng):
    pass


def resize_stub(case):
    def stub(call, rng):
        vec, count = call.ints[0], call.ints[1] & 0xffffffff
        base = case.region(f'V{len(case.regions)}', max(16, 4 * count + 16))
        for off, v in ((0, base), (8, base + 4 * count), (16, base + 4 * count)):
            call.put(vec + off, v & 0xffffffff)
            call.put(vec + off + 4, v >> 32)
    return stub


def main():
    case = VmCase(EXE, SEED, [(0x141190000, 0x1412a0000), (0x140800000, 0x140900000)])
    for addr in (0x1407ace10, 0x140822620, 0x1411854a0, 0x1411c5a90):
        case.stub(addr, bool_stub)
    case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.uniform(0.005, 0.06)))
    for addr in (0x1411da6c0, 0x14119ac90, 0x141197b00, 0x14119a570, 0x1411bd470, 0x1411975a0, 0x141190ed0, 0x1406307a0):
        case.stub(addr, void_stub)
    case.stub(0x1406fd4d0, resize_stub(case))
    print('# controls vectors (tools/gen_controls_vectors.py), stage', STAGE)
    done = attempts = 0
    while done < TRIALS and attempts < TRIALS * 20:
        attempts += 1
        case.reset()
        F = case.region('F', 0xc000)
        B = case.region('B', 0x7000)
        E = case.region('E', 0x68 * 4)
        P = case.region('P', 0x3770 * 4)
        M = case.region('M', 0x2cc * 4)
        N = case.region('N', 0x388 * 4)
        D = case.region('D', 0x400)
        fz = case.fz
        for base, off, ptr in ((F, 0x20, B), (B, 0x5ff8, E), (B, 0x6010, P), (B, 0x61f8, D), (F, 0x68b0, M), (F, 0x68c8, N)):
            name = {F: 'F', B: 'B'}[base]
            fz.preset(name, off, ptr & 0xffffffff, record=True)
            fz.preset(name, off + 4, ptr >> 32, record=True)
        fz.preset('F', 0xbce0, F & 0xffffffff)
        fz.preset('F', 0xbce4, F >> 32)
        fz.preset('F', 0xbce8, B & 0xffffffff)
        fz.preset('F', 0xbcec, B >> 32)
        for off in (0xbcc8, 0xbcd0):
            fz.preset('F', off, 0)
        fz.preset('F', 0x6760, case.rng.choice([0, 0, 0, 1]))
        fz.preset('F', 0x6764, case.rng.choice([0, 0, 1]))
        for off in (0x91c, 0x920):
            fz.preset('B', off, case.rng.randrange(1, 4))
        for e in range(4):
            fz.preset('E', 0x68 * e, case.rng.choice([0, 1, 2, 4, 5, 6, 7]))
            fz.preset('P', 0x3770 * e, case.rng.choice([0, 3, 5, 6, 6]))
        fz.preset('B', 0xb78, case.rng.randrange(0, 3))
        fz.preset('B', 0xc90, case.rng.randrange(0, 3))
        fz.preset('B', 0xc9c, case.rng.randrange(0, 3))
        fz.preset('B', 0xca0, case.rng.randrange(0, 3))
        try:
            case.run(FUNC, ints=[F], until=UNTIL[STAGE], max_instructions=600000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        print(case.dump(f'{F:x} {entry_rsp(0):x}'))
        done += 1


if __name__ == '__main__':
    main()
