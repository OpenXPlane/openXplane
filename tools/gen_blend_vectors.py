#!/usr/bin/env python3
"""Vectors for blend::blend_contact (the original 0x1411cb1e0, the contact of one quad of a body surface).

    python3 tools/gen_blend_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/blend_contact.txt

The header is `F s g base i0 i1 out1..out5 result`.
"""
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX  # noqa: E402
from xp_vmcase import VmCase, entry_rsp, setup_strut_world  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])


def main():
    case = VmCase(EXE, SEED, [(0x1411cb000, 0x1411cd000), (0x14120c000, 0x14120d000), (0x1407ac000, 0x1407ad000),
                              (0x141960000, 0x141961000), (0x1407ce000, 0x1407cf000), (0x1419f7000, 0x1419fa000),
                              (0x1408e3000, 0x1408e4000), (0x141296000, 0x141297000), (0x1411d0000, 0x1411e0000),
                              (0x140f2a000, 0x140f2c000), (0x140816000, 0x140817000), (0x141a64000, 0x141a65000),
                              (0x1406ea000, 0x1406eb000), (0x140862000, 0x140863000), (0x140cb9000, 0x140cbc000),
                              (0x14093c000, 0x14093d000), (0x1407b3000, 0x1407b4000), (0x14062c000, 0x14062d000),
                              (0x140cc3000, 0x140cc4000), (0x1419f8000, 0x1419f9000), (0x14089c000, 0x14089d000)])
    rng = case.rng
    print('# blend contact vectors (tools/gen_blend_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        F = case.region('F', 0x44000)
        B = case.region('B', 0x6700)
        sp = entry_rsp(8)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)
        setup_strut_world(case, rng, F, B, mesh_triangles=(20, 40), mesh_extent=rng.choice([2.0, 4.0]), mesh_heights=(0.0, 3.0))

        def put(name, off, v):
            fz.preset(name, off, v & 0xffffffff)
            fz.preset(name, off + 4, v >> 32)
        def normal(call, rng):
            v = [rng.uniform(-1, 1) for _ in range(3)]
            length = sum(c * c for c in v) ** 0.5 or 1.0
            for k in range(3):
                call.put_f32(call.ints[3] + 4 * k, v[k] / length)
        case.stub(0x1406ed6a0, normal)
        for off in (0xbcc8, 0xbcd0):
            fz.preset('F', off, 0)
        fz.preset('F', 0x28, 0)
        case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.choice([0.05, 0.2, rng.uniform(0, 0.5)])))
        case.stub(0x1417f12c0, lambda call, rng: call.ret_int(rng.choice([0, 0, 1])))
        CX = case.region('CX', 0x300)
        case.stub(0x14193ae40, lambda call, rng: call.ret_int(CX))
        case.stub(0x1419f8ff0, lambda call, rng: call.ret_f64(rng.uniform(-60, 60)))
        fz.preset_f64('CX', 0xb0, 6378137.0)
        fz.preset_f64('CX', 0xb8, rng.uniform(6350000.0, 6378000.0))
        fz.preset_f64('CX', 0xc8, rng.uniform(0.0, 0.01))
        fz.preset_f64('CX', 0xd0, rng.uniform(0.0, 0.01))
        for base in (0x200, 0x280):
            for r in range(4):
                for c in range(4):
                    v = (1.0 if r == c and r < 3 else 0.0) + (rng.uniform(-0.1, 0.1) if r < 3 and c < 3 else (rng.uniform(-5, 5) if r == 3 and c < 3 else 0.0))
                    if r == 3 and c == 3:
                        v = 1.0
                    fz.preset_f64('CX', base + 0x20 * r + 8 * c, v)
        fz.preset_f64('F', 0x390, rng.uniform(-80, 80))
        fz.preset_f64('F', 0x398, rng.uniform(-170, 170))
        fz.region('GRID', 0x1461179f8, 8)
        fz.preset('GRID', 0, 0)
        fz.preset('GRID', 4, 0)
        fz.region('IDX', 0x1426552ac, 4)
        fz.preset('IDX', 0, 0)
        for off, lo, hi in ((0x64f4, -30, 30), (0x64f8, -30, 30), (0x64fc, -30, 30)):
            fz.preset_f32('B', off, rng.uniform(lo, hi))
        fz.preset_f32('F', 0x3e0, rng.uniform(-180, 180))
        fz.region('LV', 0x146125780, 0x20)
        LREC = case.region('LREC', 0x28 * 6)
        m = rng.randrange(0, 3)
        put('LV', 0, LREC)
        put('LV', 8, LREC + 0x28 * m)
        put('LV', 16, LREC + 0x28 * (m + rng.choice([0, 1])))
        NEWL = case.region('NEWL', 0x28 * 8)

        def grow(call, rng):
            vec, rec = call.ints[0], call.ints[2]
            old_begin = case.emu.read_u64(vec)
            end = case.emu.read_u64(vec + 8)
            count = (end - old_begin) // 0x28
            for k in range(count * 10):
                call.put(NEWL + 4 * k, case.emu.read_u32(old_begin + 4 * k))
            for k in range(10):
                call.put(NEWL + 0x28 * count + 4 * k, case.emu.read_u32(rec + 4 * k))
            for off, v in ((0, NEWL), (8, NEWL + 0x28 * (count + 1)), (16, NEWL + 0x28 * (count + 4))):
                call.put(vec + off, v & 0xffffffff)
                call.put(vec + off + 4, v >> 32)
        case.stub(0x141167000, grow)
        S = case.region('SB', 0x700)
        G = case.region('GB', 0xb0)
        BASE = case.region('BASE', 12 * 140)
        fz.preset('SB', 0x2c, rng.choice([0, 1]))
        fz.preset_f32('SB', 0x38, rng.uniform(0, 3))
        fz.preset_f32('SB', 0x44, rng.uniform(-1, 1))
        for off in (0x90, 0x94, 0x98):
            fz.preset_f32('GB', off, rng.uniform(-1, 1))
        for off in (0x9c, 0xa0, 0xa4):
            fz.preset_f32('GB', off, rng.uniform(-40, 40))
        for k in range(140 * 3):
            fz.preset_f32('BASE', 4 * k, rng.uniform(-1.5, 1.5))
        OUT = case.region('OUT', 0x40)
        outs = [OUT + 0x10 * k for k in range(5)]
        for o in outs:
            fz.preset_f32('OUT', o - OUT, rng.uniform(-1, 1))
        i0 = rng.randrange(0, 3)
        i1 = rng.randrange(0, 10)
        stack = [0, i0, i1] + outs
        for k, value in enumerate(stack):
            off = 0x800 + 0x28 + 8 * k
            fz.preset('S', off, value & 0xffffffff)
            fz.preset('S', off + 4, value >> 32)
        try:
            case.run(0x1411cb1e0, ints=[F, S, G, BASE], stack=stack, max_instructions=1000000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        result = case.emu.reg(UC_X86_REG_RAX) & 0xff
        import os
        if os.environ.get('XP_FRAME') == str(done):
            rbp = sp - 0x218
            for off in list(range(-0x80, 0x100, 4)) + list(range(0x200, 0x280, 4)):
                w = case.emu.read_u32(rbp + off)
                sys.stderr.write(f'frame {off:+#x}: {w:08x} {struct.unpack("<f", struct.pack("<I", w))[0]!r}\n')
        text = case.dump(' '.join(f'{x:x}' for x in (F, S, G, BASE, i0, i1, *outs, result)))
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
