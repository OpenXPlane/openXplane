#!/usr/bin/env python3
"""Vectors for the body aerodynamic functions: 0x141a51600 (body::body_aero), 0x141a522d0 (body::body_wave_drag).

    python3 tools/gen_body_vectors.py Xplane12/X-Plane.exe FUNC TRIALS SEED > crates/xp-app/tests/data/body_FUNC.txt

The header is `R q a6 a7 a8 a9 out1 out2 out3 returned-bits` (the arguments as float32 bits); for 0x141a522d0 `R a b c
returned-bits`.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase  # noqa: E402
from unicorn.x86_const import UC_X86_REG_XMM0  # noqa: E402

EXE, FUNC, TRIALS, SEED = sys.argv[1], int(sys.argv[2], 16), int(sys.argv[3]), int(sys.argv[4])


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x141a50000, 0x141a60000), (0x1406e0000, 0x1406f0000)])
    print('# body function vectors', hex(FUNC))
    rng = case.rng
    done = attempts = 0
    while done < TRIALS and attempts < TRIALS * 20:
        attempts += 1
        case.reset()
        R = case.region('R', 0x700 if FUNC == 0x141a51600 else 0x1400)
        O = case.region('O', 0x40)
        fz = case.fz
        if rng.random() < 0.9:
            for off in (0x10, 0x14, 0x18):
                fz.preset_f32('R', off, rng.uniform(0.05, 20.0))
            # the end point is past the start point on every axis (a negative ratio goes through the sqrt domain
            # handler, which the emulator does not run)
            for off in (0x58, 0x5c, 0x60):
                fz.preset_f32('R', off, rng.uniform(-5.0, 0.0))
            for off in (0x64, 0x68, 0x6c):
                fz.preset_f32('R', off, rng.uniform(0.0, 5.0))
        if FUNC == 0x141a522d0:
            fz.preset('R', 0x654, rng.randrange(1, 6))
            fz.preset('R', 0x658, rng.randrange(2, 19))
            for row in range(5):
                for col in range(18):
                    for axis in range(3):
                        fz.preset_f32('R', 0x65c + row * 0xd8 + col * 12 + 4 * axis, rng.uniform(-3.0, 3.0))
            a, b, c = rng.uniform(0.0, 1.5), rng.uniform(100.0, 400.0), rng.uniform(0.0, 1000.0)
            try:
                case.run(FUNC, ints=[R], floats=[0.0, a, b, c])
            except RuntimeError as err:
                sys.stderr.write(f'trial failed: {err}\n')
                continue
            done += 1
            ret = case.emu.uc.reg_read(UC_X86_REG_XMM0) & 0xffffffff
            print(case.dump(f'{R:x} {bits(a):08x} {bits(b):08x} {bits(c):08x} {ret:08x}'))
            continue
        q = rng.uniform(0.0, 3000.0)
        a6, a7, a8, a9 = (rng.uniform(-3.5, 3.5) for _ in range(4))
        a9 = rng.uniform(0.0, 100.0)
        try:
            if FUNC == 0x141a51600:
                case.run(FUNC, ints=[R, 0, O, O + 8], floats=[0.0, q], stack=[O + 4, a6, a7, a8, a9, 0])
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        done += 1
        ret = case.emu.uc.reg_read(UC_X86_REG_XMM0) & 0xffffffff
        print(case.dump(f'{R:x} {bits(q):08x} {bits(a6):08x} {bits(a7):08x} {bits(a8):08x} {bits(a9):08x} {O:x} {O + 4:x} {O + 8:x} {ret:08x}'))


if __name__ == '__main__':
    main()
