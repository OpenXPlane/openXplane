#!/usr/bin/env python3
"""Vectors for flight_init::flight_init (the original 0x1411ce9c0, the initialisation of a flight object).

    python3 tools/gen_init_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/flight_init.txt

The header is `ctx scale-bits`. The callees outside the function are stubbed and replayed; a few of them write words
(`put`) so the order of the stores against the calls is checked.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])

STUBBED = [0x14087e590, 0x1411aedb0, 0x1411dd2c0, 0x141a64250, 0x14120ad10, 0x14122cbc0, 0x1417f12c0, 0x141ba6750,
           0x14121a690, 0x1411ddfd0, 0x14082be10, 0x1411b0420]
VTABLE_FN_A, VTABLE_FN_B = 0x1410c9620, 0x1411a0900
TABLE, INDEX = 0x1461243b0, 0x1426552ac


def pages(addresses):
    return sorted({(a & ~0xfff, (a & ~0xfff) + 0x1000) for a in addresses})


def main():
    ranges = [(0x1411ce000, 0x1411d1000)] + pages(STUBBED + [VTABLE_FN_A, VTABLE_FN_B])
    case = VmCase(EXE, SEED, ranges)
    rng = case.rng
    print('# flight init vectors (tools/gen_init_vectors.py)')
    f32 = lambda v: struct.unpack('<I', struct.pack('<f', v))[0]
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        F = case.region('F', 0x43400)
        B = case.region('B', 0x6700)
        sp = entry_rsp(0)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)

        def put(name, off, v):
            fz.preset(name, off, v & 0xffffffff)
            fz.preset(name, off + 4, v >> 32)

        CTX = case.region('CTX', 0x10)
        put('CTX', 0, F)
        put('CTX', 8, B)
        n_engines, n_parts = rng.randrange(0, 4), rng.randrange(0, 4)
        ENG = case.region('ENG', 0x68 * 4)
        AERO = case.region('AERO', 0x3770 * 4)
        GEAR = case.region('GEAR', 0x88 * 10)
        E = case.region('E', 0x2cc * 4)
        P = case.region('P', 0x388 * 4)
        X = case.region('X', 0x1d00)
        TO = case.region('TO', 0x7e00)
        V1 = case.region('V1', 0x10)
        V2 = case.region('V2', 0x10)
        OW = case.region('OW', 0x10)
        ANIM = case.region('ANIM', 0x90 * 10)
        put('F', 0, V1)
        put('V1', 0, VTABLE_FN_A)
        put('TO', 0x6f90 + 0xc10, OW if rng.random() < 0.7 else 0)
        put('OW', 0, V2)
        put('V2', 8, VTABLE_FN_B)
        put('F', 0x20, B)
        put('F', 0x68b0, E)
        put('F', 0x68c8, P)
        put('F', 0xbce8, X)
        put('F', 0x6958, ANIM)
        put('F', 0x6960, ANIM + 0x90 * 10)
        put('F', 0x42f98, rng.choice([0, X]))
        for off in (0xbd00, 0xbd50, 0xbda8, 0xbdd8):
            put('F', off, F)
            put('F', off + 8, B)
        put('B', 0x5ff8, ENG)
        put('B', 0x6010, AERO)
        put('B', 0x6080, GEAR)
        fz.preset('B', 0xc9c, rng.randrange(0, 4))
        fz.preset('B', 0x91c, n_engines)
        fz.preset('B', 0x920, n_parts)
        fz.preset('B', 0x1eb8, rng.randrange(0, 6))
        fz.preset('B', 0xa9c, rng.choice([0, 1, 2, 3, 2, 3]))
        fz.preset('F', 0x6970, 0x1000)
        fz.preset('F', 0x6978, rng.choice([0x1000, 0x2000]))
        for off in (0x28, 0x68a0, 0x68a4, 0x1f0):
            fz.preset('F', off, rng.choice([0, 0, 1, 2]))
        for off in (0x914, 0xae0, 0x1cb0, 0xdc4, 0xdc8, 0xdcc, 0x1cf8):
            fz.preset('B', off, rng.choice([0, 0, 1, 3]))
        fz.preset('B', 0x1d18, rng.choice([0, 0x1234]))
        for off in (0x22b0, 0x22cc, 0xea8, 0x9e0, 0xa54, 0xa58, 0xd84):
            fz.preset_f32('B', off, rng.choice([0.0, 0.0, 1.0, -0.5, rng.uniform(-3, 3), rng.uniform(100, 5000)]))
        # divisors and numerators of the ratios: 0/0 would give a NaN whose sign depends on the host
        for off in (0x950, 0x9f8):
            fz.preset_f32('B', off, rng.choice([1.0, -0.5, rng.uniform(0.1, 3), rng.uniform(100, 5000)]))
        fz.preset_f32('B', 0xeb8, rng.uniform(-3, 3))
        for off in (0x1ca4, 0x1ca8):
            fz.preset('X', off, rng.choice([0, 0, 1]))
        fz.preset_f32('X', 0xc04, rng.choice([0.0, 0.0, 12.5]))
        for k in range(4):
            fz.preset('ENG', 0x68 * k, rng.choice([0, 3, 4, 7, 5]))
            fz.preset_f32('ENG', 0x68 * k + 0x1c, rng.uniform(-2, 2))
            fz.preset_f32('ENG', 0x68 * k + 0x18, rng.uniform(-2, 2))
            fz.preset_f32('AERO', 0x3770 * k + 0x20, rng.choice([0.0, 0.5, 2.0, rng.uniform(0.1, 4)]))
            fz.preset_f32('AERO', 0x3770 * k + 0x790, rng.choice([0.0, 0.3, 0.99, 1.0, 2.0]))
        density = rng.choice([0.05, 0.2, 0.5])
        for k in range(10):
            fz.preset('GEAR', 0x88 * k, rng.choice([1, 2, 3, -1]) if rng.random() < density else 0)
            fz.preset('GEAR', 0x88 * k + 4, 1 if rng.random() < density else 0)
            fz.preset('GEAR', 0x88 * k + 0xc, 1 if rng.random() < density else 0)
        fz.region('IDX', INDEX, 4)
        fz.preset('IDX', 0, 0)

        def ret_object(call, rng):
            call.ret_int(TO)

        def with_effect(addr_of):
            def handler(call, rng):
                call.put(addr_of(call), rng.randrange(0, 1 << 32))
            return handler

        case.stub(0x14087e590, lambda call, rng: None)
        case.stub(0x1411aedb0, with_effect(lambda c: c.ints[0] + 0xb0))
        case.stub(0x1411dd2c0, with_effect(lambda c: c.ints[0] + 4))
        case.stub(0x141a64250, ret_object)
        case.stub(0x14120ad10, with_effect(lambda c: c.ints[0] + 0x10))
        case.stub(0x14122cbc0, with_effect(lambda c: E + 0x2cc * (c.ints[1] & 0xffffffff) + 0x2a8))
        case.stub(0x1417f12c0, lambda call, rng: call.ret_int(rng.choice([0, 0, 1])))
        case.stub(0x141ba6750, lambda call, rng: call.ret_f32(rng.uniform(-60, 60)))
        case.stub(0x14121a690, with_effect(lambda c: c.ints[0] + 4))
        case.stub(0x1411ddfd0, with_effect(lambda c: c.ints[1] + 4))
        case.stub(0x14082be10, lambda call, rng: call.ret_int(rng.choice([0, 1, 0x5555])))
        case.stub(0x1411b0420, with_effect(lambda c: c.ints[0] + 0x10 + 4 * (c.ints[3] & 0xffffffff)))
        case.stub(VTABLE_FN_A, with_effect(lambda c: c.ints[0] + 0x600))
        case.stub(VTABLE_FN_B, with_effect(lambda c: c.ints[0] + 0x4))
        scale = rng.choice([0.0, 0.1, 0.2, 1.0, -2.0, 10.0, 3000.0, rng.uniform(-1000, 5000)])
        try:
            case.run(0x1411ce9c0, ints=[CTX, 0, 0, 0], floats=[0.0, 0.0, 0.0, scale], max_instructions=400000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        text = case.dump(f'{CTX:x} {f32(scale):x}')
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
