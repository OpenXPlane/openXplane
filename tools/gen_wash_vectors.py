#!/usr/bin/env python3
"""Vectors for wash::wash (the original 0x14117d970), run from the entry to a checkpoint address.

    python3 tools/gen_wash_vectors.py Xplane12/X-Plane.exe STAGE TRIALS SEED > crates/xp-app/tests/data/wash_STAGE.txt

The header is `F rbp x z out1`; the stack frame words are not compared.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, STAGE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
UNTIL = {1: 0x14117e266, 2: 0x14117f79f, 3: 0x1411819c2, 4: 0x141181f0d}
FUNC = 0x14117d970


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x14117d000, 0x141182000), (0x1408b0000, 0x1408c0000), (0x140860000, 0x140870000),
                              (0x14120c000, 0x14120d000), (0x141296000, 0x141297000), (0x1407ac000, 0x1407ad000),
                              (0x1406e0000, 0x1406f0000)])
    case.stub(0x1407ace10, lambda call, rng: call.ret_int(rng.randrange(2) if rng.random() < 0.3 else 0))
    def shadow(call, rng):
        flag = rng.choice([0, 0, 0, 1])
        call.put(call.ints[1], flag)
        call.put_f32(call.ints[1] + 4, rng.uniform(-0.5, 1.5))
    case.stub(0x141186930, shadow)
    print('# wash vectors (tools/gen_wash_vectors.py), stage', STAGE)
    rng = case.rng
    done = attempts = 0
    while done < TRIALS and attempts < TRIALS * 30:
        attempts += 1
        case.reset()
        F = case.region('F', 0xc000)
        B = case.region('B', 0x7000)
        E = case.region('E', 0x68 * 4)
        P = case.region('P', 0x3770 * 4)
        M = case.region('M', 0x2cc * 4)
        N = case.region('N', 0x388 * 4)
        O = case.region('O', 0x40)
        if STAGE >= 3:
            W = case.region('W', 0x36c8 * 48)
            X = case.region('X', 0x2d8 * 48)
            D = case.region('D', 0x34c8 * 2)
            A = case.region('A', 0x100 * 3)
        fz = case.fz
        sp = entry_rsp(6)
        fz.region('S', sp - 0x800, 0x900)
        for base, off, ptr in ((F, 0x20, B), (B, 0x5ff8, E), (B, 0x6010, P), (F, 0x68b0, M), (F, 0x68c8, N)):
            name = {F: 'F', B: 'B'}[base]
            fz.preset(name, off, ptr & 0xffffffff, record=True)
            fz.preset(name, off + 4, ptr >> 32, record=True)
        if STAGE >= 3:
            for base, off, ptr in ((B, 0x6028, W), (F, 0x6940, X), (B, 0x6040, D)):
                name = {F: 'F', B: 'B'}[base]
                fz.preset(name, off, ptr & 0xffffffff, record=True)
                fz.preset(name, off + 4, ptr >> 32, record=True)
            live = set(rng.sample(range(48), 4))
            for j in range(48):
                if j not in live:
                    fz.preset('W', 0x36c8 * j + 0x678, 0)
                    continue
                fz.preset('W', 0x36c8 * j + 4, rng.randrange(1, 4))
                fz.preset('W', 0x36c8 * j + 0x678, 1)
                fz.preset_f32('W', 0x36c8 * j + 0x710, rng.uniform(-6, 6))
                for k, off in enumerate((0x3678, 0x3680, 0x3688)):
                    ptr = rng.choice([0, A + 0x100 * k, A + 0x100 * rng.randrange(3)])
                    fz.preset('W', 0x36c8 * j + off, ptr & 0xffffffff, record=True)
                    fz.preset('W', 0x36c8 * j + off + 4, ptr >> 32, record=True)
                for k in range(4):
                    fz.preset_f32('W', 0x36c8 * j + 0x70 + 4 * k, rng.uniform(0.3, 3.0))
                    fz.preset_f32('W', 0x36c8 * j + 0x74 + 4 * k, rng.uniform(0.0, 3.0))
                    fz.preset_f32('W', 0x36c8 * j + 0x144 + 4 * k, rng.uniform(0.0, 1.5))
                    fz.preset_f32('W', 0x36c8 * j + 0x16c + 4 * k, rng.uniform(-30.0, 30.0))
                    fz.preset_f32('X', 0x2d8 * j + 0xf4 + 4 * k, rng.uniform(0.0, 1.5))
                    fz.preset_f32('X', 0x2d8 * j + 0x16c + 4 * k, rng.uniform(-0.5, 2.0))
                    fz.preset_f32('X', 0x2d8 * j + 4 + 4 * k, rng.uniform(0.5, 8.0))
                for k in range(5):
                    fz.preset_f32('W', 0x36c8 * j + 0x5bc + 4 * k, rng.uniform(-6, 6))
                    fz.preset_f32('W', 0x36c8 * j + 0x5e8 + 4 * k, rng.uniform(-6, 6))
                    fz.preset_f32('W', 0x36c8 * j + 0x614 + 4 * k, rng.uniform(-6, 6))
                fz.preset_f32('W', 0x36c8 * j + 0x36a8, rng.uniform(0.5, 10.0))
                fz.preset_f32('X', 0x2d8 * j + 0x290, rng.uniform(0.0, 2.0))
            fz.preset_f32('F', 0x74, rng.uniform(100.0, 400.0))
            fz.preset_f32('F', 0x524, rng.uniform(-120.0, 120.0))
        for off in (0xbcc8, 0xbcd0):
            fz.preset('F', off, 0)
        fz.preset('B', 0x91c, rng.randrange(1, 4))
        fz.preset('B', 0x920, rng.randrange(1, 4))
        for e in range(4):
            fz.preset('E', 0x68 * e, rng.choice([0, 5, 6, 5, 6, 7]))
            fz.preset_f32('M', 0x2cc * e + 0x278, rng.choice([-1.0, 1.0]) * rng.uniform(0.0, 3.0))
            fz.preset_f32('E', 0x68 * e + 0x4c, rng.uniform(-3, 3))
            fz.preset_f32('E', 0x68 * e + 0x50, rng.uniform(-3, 3))
            fz.preset_f32('E', 0x68 * e + 0x54, rng.uniform(-3, 3))
        fz.preset_f32('B', 0x950, rng.uniform(0.5, 30.0))
        for p_ in range(4):
            base = 0x3770 * p_
            fz.preset('P', base, rng.choice([0, 3, 5, 6, 0, 3]))
            fz.preset_f32('P', base + 0x98, rng.uniform(0.5, 3.0))
            for off in (0x790, 0x794, 0x798):
                fz.preset_f32('P', base + off, rng.uniform(-3.0, 3.0))
            for off in (0x79c, 0x7a0, 0x7a4):
                fz.preset_f32('P', base + off, rng.uniform(-90.0, 90.0))
            fz.preset_f32('N', 0x388 * p_ + 0x80, rng.uniform(-3.0, 3.0))
            fz.preset_f32('N', 0x388 * p_ + 0x84, rng.uniform(-3.0, 3.0))
        for off in (0x688,):
            pass
        for k in range(3):
            fz.preset_f32('O', 4 * k, rng.uniform(-5, 5))
        x, y, z = (rng.uniform(-8, 8) for _ in range(3))
        i1, i2, i3 = rng.choice([-1, 0, 1, 5]), rng.choice([-1, 0, 1]), rng.choice([-1, -1, -1, 0])
        if STAGE >= 3:
            i1, i2, i3 = rng.choice([-1, 0, 1, 5]), rng.choice([-1, 0, 1] + sorted(live)), rng.choice([-1, -1, 0, 1])
            import os
            if os.environ.get('NOEX'):
                i2 = i3 = -1
            if os.environ.get('ONLYWING'):
                i3 = -1
            if os.environ.get('ONLYBODY'):
                i2 = -1
        # the stack arguments are written by the call; preset them so the lazy fill keeps them
        args = [O + 4, bits(y), O + 8, i1 & 0xffffffff, i2 & 0xffffffff, i3 & 0xffffffff]
        for k, value in enumerate(args):
            off = sp + 0x28 + 8 * k - (sp - 0x800)
            fz.preset('S', off, value & 0xffffffff)
            fz.preset('S', off + 4, value >> 32)
        try:
            case.run(FUNC, ints=[F, 0, O, 0], floats=[0.0, x, 0.0, z], stack=[O + 4, y, O + 8, i1 & 0xffffffff, i2 & 0xffffffff, i3 & 0xffffffff],
                     until=UNTIL[STAGE], max_instructions=800000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        rbp = sp - 0x638
        out = case.dump(f'{F:x} {rbp:x} {bits(x):08x} {bits(z):08x} {O:x}')
        lines = out.split('\n')
        import os
        keep = os.environ.get('KEEP_STACK')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if keep or not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
