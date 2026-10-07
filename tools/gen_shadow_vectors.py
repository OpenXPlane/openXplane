#!/usr/bin/env python3
"""Vectors for shadow::ray_box (the original 0x141296c40) and shadow::body_shadow (0x141186930).

    python3 tools/gen_shadow_vectors.py Xplane12/X-Plane.exe box|shadow TRIALS SEED > crates/xp-app/tests/data/shadow_*.txt

The header of a `box` case is `a b o inv returned`; of a `shadow` case `F result x z y out1 out2 out3 excluded` (the
floats as float32 bits). The stack frame words are not compared.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase, entry_rsp  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX  # noqa: E402

EXE, KIND, TRIALS, SEED = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def box_cases(case):
    rng = case.rng
    done = 0
    print('# ray-box vectors (tools/gen_shadow_vectors.py)')
    while done < TRIALS:
        case.reset()
        regions = [case.region(name, 0x10) for name in ('A', 'B', 'O', 'I')]
        fz = case.fz
        pool = [-2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0]
        for name, base in zip(('A', 'B', 'O', 'I'), regions):
            for k in range(4):
                if name == 'I':
                    v = rng.choice([-2.0, -1.0, -0.5, 0.5, 1.0, 2.0, rng.uniform(-3, 3)])
                elif rng.random() < 0.3:
                    v = rng.choice(pool)
                else:
                    v = rng.uniform(-3, 3)
                fz.preset_f32(name, 4 * k, v)
        case.run(0x141296c40, ints=regions, max_instructions=2000)
        ret = case.emu.reg(UC_X86_REG_RAX) & 0xff
        print(case.dump(' '.join(f'{r:x}' for r in regions) + f' {ret:x}'))
        done += 1


def shadow_cases(case):
    rng = case.rng
    print('# body shadow vectors (tools/gen_shadow_vectors.py)')
    done = attempts = 0
    while done < TRIALS and attempts < TRIALS * 30:
        attempts += 1
        case.reset()
        F = case.region('F', 0xc000)
        B = case.region('B', 0x7000)
        D = case.region('D', 0x34c8 * 39)
        T = case.region('T', 36 * 64 * 5)
        R = case.region('R', 0x10)
        O = case.region('O', 0x10)
        fz = case.fz
        sp = entry_rsp(5)
        fz.region('S', sp - 0x800, 0x900)

        def put64(name, off, value):
            fz.preset(name, off, value & 0xffffffff, record=True)
            fz.preset(name, off + 4, value >> 32, record=True)

        put64('F', 0x20, B)
        put64('B', 0x6040, D)
        for off in (0xbcc8, 0xbcd0):
            fz.preset('F', off, 0)
        out = [rng.choice([-1, 1]) * rng.uniform(0.3, 3.0) for _ in range(3)]
        for k in range(3):
            fz.preset_f32('O', 4 * k, out[k])
        x, y, z = (rng.uniform(-3, 3) for _ in range(3))
        excluded = rng.choice([-1, -1, 0, 3, 7])
        live = set(rng.sample(range(39), 3))
        if rng.random() < 0.5:
            live.add(excluded if excluded >= 0 else 1)
        d = [-v for v in out]
        order = {j: n for n, j in enumerate(sorted(live | ({excluded} if excluded >= 0 else set())))}
        for j in range(39):
            base = 0x34c8 * j
            if j not in live and j != excluded:
                fz.preset('D', base + 0x5f0, 0x27)
                fz.preset('D', base + 0x54, 1)
                continue
            fz.preset('D', base + 0x5f0, rng.choice([0x27] * 6 + [3, 0x26]))
            fz.preset('D', base + 0x54, rng.choice([0] * 9 + [1]))
            fz.preset('D', base + 0x588, rng.choice([1] * 9 + [0]))
            fz.preset_f32('D', base + 0x10, rng.uniform(-1.0, 1.0))
            fz.preset_f32('D', base + 4, rng.uniform(0.0, 0.7))
            off = [rng.uniform(-1.0, 1.0) for _ in range(3)]
            for k, o in enumerate(off):
                fz.preset_f32('D', base + 0x618 + 4 * k, o)
            p = [x - off[0], z - off[1], y - off[2]]
            extent = [rng.uniform(0.8, 2.5) for _ in range(3)]
            shift = [p[a] + rng.uniform(-0.5, 0.5) if rng.random() < 0.7 else rng.uniform(-1.0, 1.0) for a in range(3)]
            for k in range(3):
                fz.preset_f32('D', base + 0x58 + 4 * k, shift[k] - extent[k])
                fz.preset_f32('D', base + 0x64 + 4 * k, shift[k] + extent[k])
            tri_base = T + 36 * 64 * order[j]
            put64('D', base + 0x70, tri_base)
            slot = 0
            forward = rng.random() < 0.5
            for k in range(32):
                r = base + 0xa0 + 0x28 * k
                count = rng.choice([0, 1, 2, 3, 0]) if k < 10 else 0
                fz.preset('D', r + 0xc, count)
                if not count:
                    continue
                fz.preset('D', r + 8, slot)
                back = rng.random() < 0.3
                tcb = rng.uniform(-1.5, -0.5)
                if back:
                    cb = [p[a] + tcb * d[a] for a in range(3)]
                    lo = [cb[a] - rng.uniform(0.3, 0.6) for a in range(3)]
                    hi = [cb[a] + rng.uniform(0.3, 0.6) for a in range(3)]
                else:
                    lo = [shift[a] - extent[a] * rng.uniform(0.2, 1.0) for a in range(3)]
                    hi = [shift[a] + extent[a] * rng.uniform(0.2, 1.0) for a in range(3)]
                for a in range(3):
                    fz.preset_f32('D', r - 0x18 + 4 * a, lo[a])
                    fz.preset_f32('D', r - 0x8 + 4 * a, hi[a])
                for w in (r - 0xc, r + 4):
                    fz.preset_f32('D', w, 0.0)
                for _ in range(count):
                    tc = tcb + rng.uniform(-0.2, 0.2) if back else (rng.uniform(0.1, 1.5) if forward else rng.uniform(-1.5, 1.5))
                    c = [p[a] + tc * d[a] for a in range(3)]
                    for v in range(3):
                        for a in range(3):
                            fz.preset_f32('T', 36 * (64 * order[j] + slot) + 12 * v + 4 * a,
                                          c[a] + rng.uniform(-1.2, 1.2))
                    slot += 1
        args = [R, bits(z), O + 4, bits(y), O + 8, excluded & 0xffffffff]
        for k, value in enumerate(args[:5]):
            off = sp + 0x28 + 8 * k - (sp - 0x800)
            fz.preset('S', off, value & 0xffffffff)
            fz.preset('S', off + 4, value >> 32)
        try:
            case.run(0x141186930, ints=[F, R, 0, O], floats=[0.0, 0.0, x, 0.0],
                     stack=[bits(z), O + 4, bits(y), O + 8, excluded & 0xffffffff], until=0x141187394, max_instructions=2000000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        header = f'{F:x} {R:x} {bits(x):08x} {bits(z):08x} {bits(y):08x} {O:x} {O + 4:x} {O + 8:x} {excluded & 0xffffffff:x}'
        text = case.dump(header)
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


def main():
    case = VmCase(EXE, SEED, [(0x141186000, 0x141188000), (0x141296000, 0x141297000), (0x1407ac000, 0x1407ad000),
                              (0x1406e0000, 0x1406f0000)])
    case.stub(0x1407ace10, lambda call, rng: call.ret_int(1 if rng.random() < 0.2 else 0))
    if KIND == 'box':
        box_cases(case)
    else:
        shadow_cases(case)


if __name__ == '__main__':
    main()
