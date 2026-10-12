#!/usr/bin/env python3
"""Vectors for wheel::wheel_axis_point (0x141240fc0).

    python3 tools/gen_wheel_axis_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/wheel_axis.txt

The header is `aircraft p u w angle dir_ptr out1 out2 out3` (floats as bits).
"""
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from unicorn.x86_const import UC_X86_REG_XMM1, UC_X86_REG_XMM2, UC_X86_REG_XMM3  # noqa: E402
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
TABLE = 0x1461243b0


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def f32(v):
    return struct.unpack('<f', struct.pack('<f', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x141240000, 0x141242000), (0x140cc3000, 0x140cc4000), (0x140cb9000, 0x140cba000),
                              (0x140cba000, 0x140cbc000), (0x14089c000, 0x14089d000), (0x141296000, 0x141297000),
                              (0x141a64000, 0x141a65000)])
    rng = case.rng
    print('# wheel axis vectors (tools/gen_wheel_axis_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        F = case.region('F', 0x1000)
        sp = entry_rsp(5)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)
        D = case.region('D', 0x20)
        O = case.region('O', 0x30)
        fz.region('TB', TABLE, 8)
        fz.preset('TB', 0, F & 0xffffffff)
        fz.preset('TB', 4, F >> 32)
        for off in (0x430, 0x434, 0x440, 0x444, 0x450, 0x454):
            fz.preset_f32('F', off, rng.uniform(-1, 1))
        for off in (0x378, 0x380, 0x388):
            fz.preset_f64('F', off, rng.uniform(-1000, 1000))
        mode = rng.randrange(5)
        if mode == 0:
            d = [0.0, rng.choice([1.0, -1.0]), 0.0]
        elif mode == 1:
            d = [0.0, 0.0, rng.choice([1.0, -1.0])]
        else:
            d = [rng.uniform(-1, 1) for _ in range(3)]
        for k in range(3):
            fz.preset_f64('D', 8 * k, d[k])
        aircraft = 0
        p, u, w = (f32(rng.uniform(-5, 5)) for _ in range(3))
        angle = f32(rng.choice([rng.uniform(-180, 180), 0.0, 90.0, -90.0, 45.0]))
        outs = [O, O + 0x10, O + 0x20]
        stack = [bits(angle), D, outs[0], outs[1], outs[2]]
        for k, value in enumerate(stack):
            off = 0x800 + 0x28 + 8 * k
            fz.preset('S', off, value & 0xffffffff)
            fz.preset('S', off + 4, value >> 32)
        for reg, value in ((UC_X86_REG_XMM1, p), (UC_X86_REG_XMM2, u), (UC_X86_REG_XMM3, w)):
            case.emu.uc.reg_write(reg, bits(value))
        try:
            case.run(0x141240fc0, ints=[aircraft, 0, 0, 0], stack=stack, max_instructions=200000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        text = case.dump(' '.join(f'{x:x}' for x in (aircraft, bits(p), bits(u), bits(w), bits(angle), D, *outs)))
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
