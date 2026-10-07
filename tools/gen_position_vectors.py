#!/usr/bin/env python3
"""Vectors for flight_step::set_position (the original 0x141a6ace0), run from the entry to its epilogue.

    python3 tools/gen_position_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/position.txt

The header is `F rbp x y z` (the doubles as 64-bit words); the terrain functions are replayed with random results.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import XMM, VmCase, entry_rsp  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
EPILOGUE = 0x141a6b5c5


def f64bits(v):
    return struct.unpack('<Q', struct.pack('<d', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x141a6a000, 0x141a6c000), (0x1407ce000, 0x1407cf000), (0x1406e0000, 0x1406f0000),
                              (0x141a65000, 0x141a66000)])
    rng = case.rng
    case.stub(0x1417f12c0, lambda call, rng: call.ret_int(rng.randrange(2) if rng.random() < 0.3 else 0))
    case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.uniform(0.005, 0.06)))
    for diagnostic in (0x1405dcad0, 0x141a67610):
        case.stub(diagnostic, lambda call, rng: None)

    def fwords(call):
        return call.ints[0] - 0x42e40

    def segment(call, rng):  # 0x141963d30: writes the plane terms through three pointers
        for pointer in (call.stack[0], call.stack[1], call.stack[2]):
            call.put_f32(pointer, rng.uniform(-3, 3))

    def init(call, rng):  # 0x141962b00: may mark the probe state valid
        call.put(fwords(call) + 0x42f88, rng.choice([0, 1]))

    def probe(call, rng):  # 0x14195f4b0: reports a hit or not and writes the ground cell
        hit = rng.random() < 0.4
        if rng.random() < 0.6:
            call.put_f32(call.ints[3], rng.uniform(-100, 3000))
        call.ret_int(1 if hit else 0)

    case.stub(0x141963d30, segment)
    case.stub(0x141962b00, init)
    case.stub(0x14195f4b0, probe)
    case.stub(0x141962750, lambda call, rng: call.ret_f32(rng.uniform(-50, 3000)))
    print('# set_position vectors (tools/gen_position_vectors.py)')
    done = attempts = 0
    while done < TRIALS and attempts < TRIALS * 30:
        attempts += 1
        case.reset()
        F = case.region('F', 0x44000)
        B = case.region('B', 0x7000)
        fz = case.fz
        sp = entry_rsp(0)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('F', 0x20, B & 0xffffffff, record=True)
        fz.preset('F', 0x24, B >> 32, record=True)
        for off in (0xbcc8, 0xbcd0):
            fz.preset('F', off, 0)
        for k in range(3):
            fz.preset_f32('B', 0x64f4 + 4 * k, rng.choice([0.0, rng.uniform(-3, 3), rng.uniform(-30, 30)]))
            fz.preset_f32('F', 0x368 + 4 * k, rng.uniform(-100, 100))
        fz.preset('F', 0x42f88, rng.choice([0, 1, 1]))
        fz.preset('F', 0x42f84, rng.randrange(-1, 5))
        fz.preset('F', 0x28, rng.randrange(0, 10))
        for off in (0x42f60, 0x42f64, 0x42f68, 0x42f6c, 0x42f74):
            fz.preset_f32('F', off, rng.uniform(-50, 50))
        fz.preset_f32('F', 0x42f70, rng.choice([0.0, 0.005, -0.005, 0.5, -0.7, 1.0, 3.0]))
        pos = [rng.choice([rng.uniform(-5000, 5000), rng.uniform(-1e6, 1e6), 0.0]) for _ in range(3)]
        if rng.random() < 0.05:
            pos[rng.randrange(3)] = rng.choice([float('nan'), float('inf'), 1e300])
        try:
            for index, value in enumerate(pos):
                case.emu.uc.reg_write(XMM[index + 1], f64bits(value))
            case.run(0x141a6ace0, ints=[F], until=EPILOGUE, max_instructions=300000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        rbp = sp - 0xb8
        text = case.dump(f'{F:x} {rbp:x} {f64bits(pos[0]):016x} {f64bits(pos[1]):016x} {f64bits(pos[2]):016x}')
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
