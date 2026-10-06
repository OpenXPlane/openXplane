#!/usr/bin/env python3
"""Vectors for piston::update_engine_piston (the original 0x14119ac90(M, F, n, inputs)).

    python3 tools/gen_piston_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/piston.txt

The unported callees are stubbed (their calls and results are recorded and replayed by the Rust test); the
interpolation, curve and engine-ratio helpers run as the original code.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
FUNC = 0x14119ac90
SEED_GLOBAL = 0x142f01918
CLOCK_GLOBAL = 0x142f01910


def bool_stub(call, rng):
    call.ret_int(rng.randrange(2))


def void_stub(call, rng):
    pass


def main():
    case = VmCase(EXE, SEED, [(0x141190000, 0x1411a3000), (0x140800000, 0x140900000), (0x1406e0000, 0x1406f0000), (0x140820000, 0x140830000), (0x141230000, 0x141240000), (0x140c40000, 0x140c50000)])
    for addr in (0x1407ace10, 0x1417f12c0):
        case.stub(addr, bool_stub)
    case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.uniform(0.005, 0.06)))
    case.stub(0x141ba6750, lambda call, rng: call.ret_f32(rng.uniform(-40.0, 45.0)))
    case.stub(0x141ba64e0, lambda call, rng: call.ret_f32(rng.uniform(0.3, 1.4)))
    case.stub(0x1411dd610, lambda call, rng: call.ret_f32(rng.uniform(0.0, 1.0)))
    case.stub(0x141a6a650, lambda call, rng: call.ret_f32(rng.uniform(0.1, 1.5)))
    case.stub(0x1410c9c60, lambda call, rng: call.ret_int(0x6f0000100000))
    case.stub(0x14067b2f0, lambda call, rng: call.ret_f32(rng.uniform(0.0, 1.0)))
    case.stub(0x1408bd9d0, lambda call, rng: call.ret_f32(rng.uniform(-1.0, 1.0)))
    case.stub(0x140984e50, lambda call, rng: call.ret_f32(rng.uniform(-1.0, 1.0)))
    for addr in (0x14119bc00, 0x14119d380, 0x14117c380):
        case.stub(addr, void_stub)
    print('# piston engine vectors (tools/gen_piston_vectors.py)')
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
        I = case.region('I', 0x100)
        fz = case.fz
        rng = case.rng
        for base, off, ptr in ((F, 0x20, B), (B, 0x5ff8, E), (B, 0x6010, P), (F, 0x68c8, N), (F, 0x68b0, M)):
            name = {F: 'F', B: 'B'}[base]
            fz.preset(name, off, ptr & 0xffffffff, record=True)
            fz.preset(name, off + 4, ptr >> 32, record=True)
        for off in (0xbcc8, 0xbcd0):
            fz.preset('F', off, 0)
        fz.preset('F', 0x28, rng.choice([0, 1]))
        fz.preset('F', 0x6880, rng.choice([0, 1]))
        for off in (0x91c, 0x920):
            fz.preset('B', off, rng.randrange(1, 4))
        n = rng.randrange(3)
        for e in range(4):
            fz.preset('E', 0x68 * e, rng.choice([0, 1, 2, 3, 4, 4, 3, 5]))
            fz.preset('B', 0xb7c + 4 * e, rng.randrange(0, 3))
            fz.preset('B', 0xd38 + 4 * e, rng.randrange(0, 8))
        for j in range(4):
            fz.preset('B', 0xbbc + 4 * j, rng.randrange(0, 3))
        fz.preset('B', 0xa9c, rng.choice([0, 2, 3, 3, 2]))
        fz.preset('B', 0xc24, rng.choice([0, 1]))
        fz.preset('M', 0x298, rng.choice([0, 1, 2, 3]))
        fz.preset('M', 0x228, rng.choice([0, 1]))
        fz.preset('M', 0x70, rng.choice([0, 1, 1]))
        fz.preset('M', 0x74, rng.randrange(0, 3))
        seed = struct.unpack('<Q', struct.pack('<d', rng.uniform(-100.0, 100.0)))[0]
        case.emu.write_u32(SEED_GLOBAL, seed & 0xffffffff)
        case.emu.write_u32(SEED_GLOBAL + 4, seed >> 32)
        clock = struct.unpack('<Q', struct.pack('<d', rng.uniform(0.0, 1000.0)))[0]
        case.emu.write_u32(CLOCK_GLOBAL, clock & 0xffffffff)
        case.emu.write_u32(CLOCK_GLOBAL + 4, clock >> 32)
        for i in range(0x803 * 2):
            case.emu.write_u32(0x14612bd90 + 4 * i, struct.unpack('<I', struct.pack('<f', 0.3 + ((i * 37) % 101) / 100.0))[0])
        try:
            case.run(FUNC, ints=[M + 0x2cc * n, F, n, I], max_instructions=600000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        print(case.dump(f'{M + 0x2cc * n:x} {F:x} {n} {I:x}', extra_words={SEED_GLOBAL: seed & 0xffffffff, SEED_GLOBAL + 4: seed >> 32, CLOCK_GLOBAL: clock & 0xffffffff, CLOCK_GLOBAL + 4: clock >> 32}))
        done += 1


if __name__ == '__main__':
    main()
