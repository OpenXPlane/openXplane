#!/usr/bin/env python3
"""Vectors for weather::wind_at (the original 0x141ba80a0, the wind at a world position).

    python3 tools/gen_weather_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/wind_at.txt

The header is `W x y z out1 out2 out3` (doubles as bits).
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from unicorn.x86_const import UC_X86_REG_XMM1, UC_X86_REG_XMM2, UC_X86_REG_XMM3  # noqa: E402
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])


def dbits(v):
    return struct.unpack('<Q', struct.pack('<d', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x141ba8000, 0x141ba9000), (0x141244000, 0x141245000), (0x1411e1000, 0x1411e2000),
                              (0x14122d000, 0x14122e000), (0x1419f8000, 0x1419f9000)])
    rng = case.rng
    print('# weather vectors (tools/gen_weather_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        sp = entry_rsp(3)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)
        W = case.region('W', 0x80)
        CX = case.region('CX', 0x300)
        case.stub(0x14193ae40, lambda call, rng: call.ret_int(CX))
        case.stub(0x1419f8ff0, lambda call, rng: call.ret_f64(rng.uniform(-60, 60)))
        radius = rng.uniform(20.0, 60.0)
        fz.preset_f64('CX', 0xb0, radius)
        fz.preset_f64('CX', 0xb8, radius * rng.uniform(0.9, 1.0))
        fz.preset_f64('CX', 0xc8, rng.uniform(0.0, 0.05))
        for off in range(0x280, 0x2f8, 8):
            fz.preset_f64('CX', off, rng.uniform(-1.0, 1.0))
        fz.region('NZ', 0x14578f1f0, 0x100000)
        fz.region('G1', 0x142f01898, 8)
        fz.preset_f64('G1', 0, rng.choice([0.5, 1.0, 1.0]))
        fz.region('G2', 0x142fe7be8, 4)
        fz.preset_f32('G2', 0, rng.choice([0.0, rng.uniform(-0.5, 1.5)]))
        fz.region('G3', 0x14632b870, 4)
        fz.preset_f32('G3', 0, rng.choice([-0.5, 0.0, rng.uniform(0, 1.0), 1.1]))
        fz.region('G4', 0x14632b820, 4)
        fz.preset_f32('G4', 0, rng.uniform(0.5, 2.0))
        fz.region('G5', 0x142fe84e8, 0x10)
        fz.preset_f64('G5', 0, rng.choice([0.0, 5.0, 10.0]))
        fz.preset_f64('G5', 8, rng.uniform(50, 200))
        fz.region('G6', 0x142f01918, 8)
        fz.preset_f64('G6', 0, rng.choice([4.0, 20.0, 60.0, 400.0]))
        fz.region('G7', 0x142fe8500, 0x20)
        for off in (0, 8, 0x10):
            fz.preset_f64('G7', off, rng.uniform(-40, 40))
        fz.preset('W', 0x2c, rng.choice([0, 1]))
        for off, lo, hi in ((0x14, -5, 5), (0x18, -5, 5), (0x1c, -5, 5), (0x20, -50, 50), (0x24, -50, 50),
                            (0x28, -50, 50), (0x30, -30, 30), (0x38, -30, 30), (0x40, -30, 30), (0x48, -1, 1),
                            (0x4c, -1, 1), (0x50, -1, 1), (0x54, -1, 1), (0x58, -1, 1), (0x5c, -1, 1),
                            (0x60, 10, 200), (0x64, 0, 10), (0x4, -0.2, 1.3)):
            fz.preset_f32('W', off, rng.uniform(lo, hi))
        OUT = case.region('OUT', 0x30)
        outs = [OUT, OUT + 0x10, OUT + 0x20]
        x = rng.uniform(-60, 60) + rng.choice([0, 0, 20])
        y = rng.uniform(-60, 60)
        z = rng.uniform(-60, 60)
        stack = outs
        for k, value in enumerate(stack):
            off = 0x800 + 0x28 + 8 * k
            fz.preset('S', off, value & 0xffffffff)
            fz.preset('S', off + 4, value >> 32)
        case.emu.uc.reg_write(UC_X86_REG_XMM1, dbits(x))
        case.emu.uc.reg_write(UC_X86_REG_XMM2, dbits(y))
        case.emu.uc.reg_write(UC_X86_REG_XMM3, dbits(z))
        import os
        if os.environ.get('XP_STAGE') == str(done):
            from unicorn import UC_HOOK_CODE
            def hook(uc, address, size, user):
                if address == 0x141ba8887:
                    from unicorn.x86_const import UC_X86_REG_XMM8, UC_X86_REG_XMM9, UC_X86_REG_XMM11, UC_X86_REG_XMM10
                    f = lambda r: struct.unpack('<f', struct.pack('<I', case.emu.uc.reg_read(r) & 0xffffffff))[0]
                    sys.stderr.write(f'pc {address:x}: dx={f(UC_X86_REG_XMM11)} dy={f(UC_X86_REG_XMM8)} dz={f(UC_X86_REG_XMM9)}\n')
                if address in (0x141ba8a63, 0x141ba8a90, 0x141ba8a36, 0x141ba8a0c, 0x141ba8aa1):
                    from unicorn.x86_const import UC_X86_REG_XMM0, UC_X86_REG_XMM1, UC_X86_REG_XMM8, UC_X86_REG_XMM14
                    f = lambda r: struct.unpack('<f', struct.pack('<I', case.emu.uc.reg_read(r) & 0xffffffff))[0]
                    sys.stderr.write(f'pc {address:x}: x0={f(UC_X86_REG_XMM0)} x1={f(UC_X86_REG_XMM1)} x8={f(UC_X86_REG_XMM8)} x14={f(UC_X86_REG_XMM14)}\n')
                if address in (0x141ba83e5, 0x141ba8732, 0x141ba8af9):
                    vals = [struct.unpack('<f', struct.pack('<I', case.emu.read_u32(o)))[0] for o in outs]
                    sys.stderr.write(f'stage {address:x}: {vals}\n')
            case.emu.uc.hook_add(UC_HOOK_CODE, hook)
        try:
            case.run(0x141ba80a0, ints=[W, 0, 0, 0], stack=stack, max_instructions=200000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        text = case.dump(' '.join(f'{v:x}' for v in (W, dbits(x), dbits(y), dbits(z), *outs)))
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
