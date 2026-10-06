#!/usr/bin/env python3
"""Vectors for the small engine helpers: 0x140822620 (engine_has_mode), 0x141192820 (int_power),
0x1411854a0 (group_query), 0x1411c5a90 (replay_active), 0x1411da6c0 (engine_start_state).

    python3 tools/gen_engine_small_vectors.py Xplane12/X-Plane.exe FUNC TRIALS SEED > crates/xp-app/tests/data/engine_small_FUNC.txt

The header is the arguments followed by the returned value.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase  # noqa: E402

EXE, FUNC, TRIALS, SEED = sys.argv[1], int(sys.argv[2], 16), int(sys.argv[3]), int(sys.argv[4])
SIM_TIME = 0x142f01918


def main():
    case = VmCase(EXE, SEED, [(0x140820000, 0x140830000), (0x141190000, 0x1411e0000)])
    case.stub(0x1407ace10, lambda call, rng: call.ret_int(rng.randrange(2)))
    print('# engine helper vectors', hex(FUNC))
    rng = case.rng
    for _ in range(TRIALS):
        case.reset()
        F = case.region('F', 0x7000)
        B = case.region('B', 0x7000)
        E = case.region('E', 0x68 * 4)
        P = case.region('P', 0x3770 * 4)
        M = case.region('M', 0x2cc * 4)
        D = case.region('D', 0x400)
        fz = case.fz
        for base, off, ptr in ((F, 0x20, B), (B, 0x5ff8, E), (B, 0x6010, P), (B, 0x61f8, D), (F, 0x68b0, M)):
            name = {F: 'F', B: 'B'}[base]
            fz.preset(name, off, ptr & 0xffffffff, record=True)
            fz.preset(name, off + 4, ptr >> 32, record=True)
        for off in (0x91c, 0x920):
            fz.preset('B', off, rng.randrange(0, 4))
        for e in range(4):
            fz.preset('E', 0x68 * e, rng.choice([0, 1, 2, 3, 4, 5, 6, 7, 8]))
            fz.preset('P', 0x3770 * e, rng.choice([0, 3, 5, 6, 9, 9]))
        t = struct.unpack('<Q', struct.pack('<d', rng.uniform(0.0, 100.0)))[0]
        case.emu.write_u32(SIM_TIME, t & 0xffffffff)
        case.emu.write_u32(SIM_TIME + 4, t >> 32)
        extra = {SIM_TIME: t & 0xffffffff, SIM_TIME + 4: t >> 32}
        if FUNC == 0x140822620:
            n, mode = rng.randrange(-1, 5), rng.randrange(0, 7)
            ret = case.run(FUNC, ints=[B, n & 0xffffffff, mode])
            hdr = f'{B:x} {n} {mode} {ret & 0xffffffff}'
        elif FUNC == 0x141192820:
            base, exp = rng.randrange(-5, 40), rng.randrange(0, 40)
            ret = case.run(FUNC, ints=[base & 0xffffffff, exp])
            hdr = f'{base} {exp} {ret & 0xffffffff}'
        elif FUNC == 0x1411854a0:
            n, g, mask = rng.randrange(0, 4), rng.randrange(0, 8), rng.randrange(0, 64)
            fz.preset('B', 0xcf8 + 4 * n, rng.choice([g, g, 0, 1, rng.randrange(0, 8)]))
            ret = case.run(FUNC, ints=[B, n, g, mask])
            hdr = f'{B:x} {n} {g} {mask} {ret & 0xff}'
        elif FUNC == 0x1411c5a90:
            fz.preset('F', 0x28, rng.choice([0, 1]))
            fz.preset('F', 0x6880, rng.choice([0, 1]))
            ret = case.run(FUNC, ints=[F])
            hdr = f'{F:x} {ret & 0xff}'
        elif FUNC == 0x1411da6c0:
            n = rng.randrange(0, 3)
            for off in (0xc34, 0xc24, 0xc28, 0xc2c, 0xc30):
                fz.preset('B', off, rng.choice([0, 1]))
            fz.preset('F', 0x58c, rng.choice([0, 1, 2, 0xff, 1, 0xff, 7]))
            fz.preset('F', 0x24c, rng.choice([0, 1]))
            case.run(FUNC, ints=[F, n])
            hdr = f'{F:x} {n} 0'
        else:
            raise SystemExit('unknown function')
        print(case.dump(hdr, extra_words=extra))


if __name__ == '__main__':
    main()
