#!/usr/bin/env python3
"""Vectors for strut::strut_contact (the original 0x1411c7a50, the ground contact of a strut or body point).

    python3 tools/gen_strut_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/strut_contact.txt

The header of a case is `F wheel x a4 a5 a6 a7 a8 a9 a10 out1 out2 result` (floats as bits). Replayed: the time
source, the engine flag, the debug query, the sound trigger, the vector growth and the weight-and-balance lookups.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX  # noqa: E402
from xp_vmcase import VmCase, entry_rsp, setup_balance, setup_segment_mesh  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x1411c7000, 0x1411c9000), (0x141182000, 0x141184000), (0x1412029d0, 0x141203000),
                              (0x1411dd000, 0x1411df000), (0x141a6b000, 0x141a6c000), (0x141294000, 0x141295000),
                              (0x141964000, 0x141965000), (0x1408e2000, 0x1408e4000), (0x141296000, 0x141297000),
                              (0x1407d6000, 0x1407d7000)])
    rng = case.rng
    print('# strut contact vectors (tools/gen_strut_vectors.py)')
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
        setup_segment_mesh(case, rng, 'F', 0x42e40, triangles=(8, 20), heights=(-3.0, 3.0))
        setup_balance(case, rng, F, B)

        def put(name, off, v):
            fz.preset(name, off, v & 0xffffffff)
            fz.preset(name, off + 4, v >> 32)
        fz.region('TB', 0x1461243b0, 8)
        put('TB', 0, F)
        fz.region('TA', 0x146124398, 8)
        put('TA', 0, B)
        WH = case.region('WH', 0x90 * 4)
        count = rng.randrange(1, 4)
        put('F', 0x6958, WH)
        put('F', 0x6960, WH + 0x90 * count)
        for k in range(4):
            fz.preset('WH', 0x90 * k + 0x4c, rng.choice([0, 1]))
            fz.preset('WH', 0x90 * k + 0x7c, rng.choice([0, 1]))
            for off in (0x80, 0x88):
                fz.preset_f64('WH', 0x90 * k + off, rng.uniform(-8, 8))
        VEC = case.region('VEC', 0x18 * 6)
        n = rng.randrange(0, 3)
        put('F', 0x430d8, VEC)
        put('F', 0x430e0, VEC + 0x18 * n)
        put('F', 0x430e8, VEC + 0x18 * (n + rng.choice([0, 1, 2])))
        fz.region('RV', 0x146125768, 0x20)
        RCD = case.region('RCD', 0x50 * 4)
        m = rng.randrange(0, 3)
        put('RV', 0, RCD)
        put('RV', 8, RCD + 0x50 * m)
        put('RV', 16, RCD + 0x50 * (m + rng.choice([0, 1])))
        fz.region('RID', 0x142f2e3dc, 4)
        fz.preset('RID', 0, rng.choice([0, 1]))
        fz.region('SND', 0x142f01920, 8)
        fz.preset_f64('SND', 0, rng.choice([0.0, 2.0]))
        fz.region('WET', 0x142fe8518, 16)
        for off in (0, 4, 8, 12):
            fz.preset_f32('WET', off, rng.choice([0.0, rng.uniform(0, 1.2)]))
        ST = case.region('ST', 36 * 4)
        fz.region('SU', 0x14611ac80, 0x10)
        put('SU', 8, ST)
        for k in range(4):
            for off in (0x18, 0x1c, 0x20):
                fz.preset_f32('ST', 36 * k + off, rng.uniform(0.2, 2))
        import math
        for sn, cs in ((0x430, 0x434), (0x440, 0x444), (0x450, 0x454)):
            angle = rng.uniform(-0.6, 0.6)
            fz.preset_f32('F', sn, math.sin(angle))
            fz.preset_f32('F', cs, math.cos(angle))
        for off in (0x3cc, 0x3d0, 0x3d4):
            fz.preset_f32('F', off, rng.uniform(-1, 1))
        for off in (0x368, 0x36c, 0x370, 0x35c, 0x360, 0x364):
            fz.preset_f32('F', off, rng.uniform(-3, 3))
        for off in (0x378, 0x380, 0x388):
            fz.preset_f64('F', off, rng.uniform(-3, 3))
        fz.preset('F', 0x28, rng.choice([0, 0, 1]))
        for off in (0x3e0, 0x3dc, 0x3d8):
            fz.preset_f32('F', off, rng.uniform(-30, 30))
        fz.preset_f32('F', 0x288, rng.uniform(50, 1000))
        for off, lo, hi in ((0x2898, 0.5, 20), (0x27c0, 0.2, 2), (0x27c4, 0.2, 2), (0x64d8, 0.5, 5), (0x2828, 10, 500)):
            fz.preset_f32('B', off, rng.uniform(lo, hi))
        case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.choice([0.05, 0.2, rng.uniform(0, 0.5)])))
        case.stub(0x1417f12c0, lambda call, rng: call.ret_int(rng.choice([0, 0, 1])))
        case.stub(0x140c51370, lambda call, rng: call.ret_int(0))
        case.stub(0x141a21200, lambda call, rng: None)

        def normal(call, rng):
            v = [rng.uniform(-1, 1) for _ in range(3)]
            length = sum(c * c for c in v) ** 0.5 or 1.0
            for k in range(3):
                call.put_f32(call.ints[3] + 4 * k, v[k] / length)
        case.stub(0x1406ed6a0, normal, caller=(0x14195ffc0, 0x141960dc0))
        NEW = case.region('NEW', 0x18 * 8)

        def grow(call, rng):
            vec, end, rec = call.ints[0], call.ints[1], call.ints[2]
            old_begin = case.emu.read_u64(vec)
            count = (end - old_begin) // 0x18
            for k in range(count * 6):
                call.put(NEW + 4 * k, case.emu.read_u32(old_begin + 4 * k))
            for k in range(6):
                call.put(NEW + 0x18 * count + 4 * k, case.emu.read_u32(rec + 4 * k))
            for off, v in ((0, NEW), (8, NEW + 0x18 * (count + 1)), (16, NEW + 0x18 * (count + 4))):
                call.put(vec + off, v & 0xffffffff)
                call.put(vec + off + 4, v >> 32)
        case.stub(0x1405e73d0, grow)
        NEWR = case.region('NEWR', 0x50 * 8)

        def grow_rec(call, rng):
            vec, rec = call.ints[0], call.ints[2]
            old_begin = case.emu.read_u64(vec)
            end = case.emu.read_u64(vec + 8)
            count = (end - old_begin) // 0x50
            for k in range(count * 20):
                call.put(NEWR + 4 * k, case.emu.read_u32(old_begin + 4 * k))
            for k in range(20):
                call.put(NEWR + 0x50 * count + 4 * k, case.emu.read_u32(rec + 4 * k))
            for off, v in ((0, NEWR), (8, NEWR + 0x50 * (count + 1)), (16, NEWR + 0x50 * (count + 4))):
                call.put(vec + off, v & 0xffffffff)
                call.put(vec + off + 4, v >> 32)
        case.stub(0x140f0f660, grow_rec)
        wheel = rng.choice([-1, 0, 0, 1, 2]) if count > 2 else rng.choice([-1, 0, 0])
        x = rng.uniform(-3, 3)
        a4 = rng.choice([0.0, rng.uniform(-1, 1)])
        a5, a6, a8, a9 = (rng.uniform(-3, 3) for _ in range(4))
        a6 = rng.uniform(2, 30)
        a7 = rng.uniform(0, 60000)
        a10 = rng.uniform(0, 5000)
        OA = case.region('OA', 16)
        OB = case.region('OB', 16)
        out1 = rng.choice([0, OA])
        out2 = rng.choice([0, OB])
        stack = [bits(a5), bits(a6), bits(a7), bits(a8), bits(a9), bits(a10), out1, out2]
        for k, value in enumerate(stack):
            off = sp + 0x28 + 8 * k - (sp - 0x800)
            fz.preset('S', off, value & 0xffffffff)
            fz.preset('S', off + 4, value >> 32)
        try:
            case.run(0x1411c7a50, ints=[F, wheel & 0xffffffff, 0, 0], floats=[0.0, 0.0, x, a4], stack=stack, max_instructions=800000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        result = case.emu.reg(UC_X86_REG_RAX) & 0xff
        header = ' '.join(f'{v:x}' for v in (F, wheel & 0xffffffff, bits(x), bits(a4), bits(a5), bits(a6), bits(a7), bits(a8), bits(a9), bits(a10), out1, out2, result))
        text = case.dump(header)
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
