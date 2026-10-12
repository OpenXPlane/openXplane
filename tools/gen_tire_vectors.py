#!/usr/bin/env python3
"""Vectors for tire::tire_motion (the original 0x141182450, a tire contact point on the ground).

    python3 tools/gen_tire_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/tire_motion.txt

The header of a case is `rec aircraft hoff scale refx refvx vx fa mu limit refz refvz vz fb` (doubles as 64-bit
words, floats as 32-bit words). The time source 0x140c448c0, the debug query 0x140c51370 and the lookups of the
weight and balance are replayed.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from unicorn.x86_const import UC_X86_REG_XMM3  # noqa: E402
from xp_vmcase import VmCase, entry_rsp, setup_balance  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
TABLE = 0x1461243b0


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def dbits(v):
    return struct.unpack('<Q', struct.pack('<d', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x141182000, 0x141184000), (0x1412029d0, 0x141203000), (0x1411dd000, 0x1411df000), (0x141a6b000, 0x141a6c000), (0x141294000, 0x141295000)])
    rng = case.rng
    print('# tire motion vectors (tools/gen_tire_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        F = case.region('F', 0xc000)
        B = case.region('B', 0x6300)
        sp = entry_rsp(15)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)
        REC = case.region('REC', 0x90)
        OA = case.region('OA', 16)
        OB = case.region('OB', 16)
        setup_balance(case, rng, F, B)
        fz.region('TB', TABLE, 8)
        fz.preset('TB', 0, F & 0xffffffff)
        fz.preset('TB', 4, F >> 32)
        case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.choice([0.05, 0.2, rng.uniform(0, 0.5)])))
        case.stub(0x140c51370, lambda call, rng: call.ret_int(rng.choice([0, 0, 1])))
        fz.region('DB', 0x142f2f21c, 4)
        fz.preset('DB', 0, rng.choice([0, 3, 4, 5, 3, 7]))
        for off in (0x3e0, 0x3dc, 0x3d8):
            fz.preset_f32('F', off, rng.uniform(-180, 180))
        for off in (0x368, 0x36c, 0x370, 0x35c, 0x360, 0x364):
            fz.preset_f32('F', off, rng.uniform(-5, 5))
        if rng.random() < 0.3:
            for off in (0x35c, 0x360, 0x364):
                fz.preset_f32('F', off, 0.0)
        fz.preset_f32('B', 0x2828, rng.uniform(10, 500))
        fz.region('TA', 0x146124398, 8)
        fz.preset('TA', 0, B & 0xffffffff)
        fz.preset('TA', 4, B >> 32)
        fz.preset_f32('F', 0x288, rng.uniform(50, 1000))
        # the weight and balance writes mass there when the record is attached; it also needs a readable F
        fz.preset('REC', 0x4c, rng.choice([0, 1]))
        fz.preset('REC', 0x7c, rng.choice([0, 1]))
        for off in (0x80, 0x88):
            lo = rng.uniform(-50, 50)
            fz.preset_f64('REC', off, lo)
        hoff = rng.choice([0.0, rng.uniform(-90, 90)])
        scale = rng.uniform(0.5, 30)
        refx, refvx, vx, refz, refvz, vz = (rng.uniform(-1.5, 1.5) for _ in range(6))
        mu = rng.uniform(0.5, 3)
        limit = rng.uniform(0.5, 10)
        fz.preset_f32('OA', 0, rng.uniform(-3, 3))
        fz.preset_f32('OB', 0, rng.uniform(-3, 3))
        # the argument words: rcx rec, edx aircraft, xmm3 heading offset, stack 5.. as the callee reads them
        stack = [dbits(scale), dbits(refx), dbits(refvx), dbits(vx), OA, bits(mu), dbits(rng.uniform(-5, 5)), 0, 0, 0, bits(limit), dbits(refz), dbits(refvz), dbits(vz), OB]
        for k, value in enumerate(stack):
            off = sp + 0x28 + 8 * k - (sp - 0x800)
            fz.preset('S', off, value & 0xffffffff)
            fz.preset('S', off + 4, value >> 32)
        try:
            case.emu.uc.reg_write(UC_X86_REG_XMM3, dbits(hoff))
            case.run(0x141182450, ints=[REC, 0, 0, 0], stack=stack, max_instructions=600000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        text = case.dump(' '.join(f'{x:x}' for x in (REC, 0, dbits(hoff), dbits(scale), dbits(refx), dbits(refvx), dbits(vx), OA, bits(mu), bits(limit), dbits(refz), dbits(refvz), dbits(vz), OB)))
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
