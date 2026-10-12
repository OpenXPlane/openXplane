#!/usr/bin/env python3
"""Vectors for mass::weight_and_balance (the original 0x1411ddfd0, the sum of the aircraft's masses and inertia).

    python3 tools/gen_balance_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/balance.txt

The header of a case is `F acc w6 w9 w12 stop` (the floats as bits). The registry lookups 0x140f5c540/0x140f42620
and the binding query 0x1407ace10 are replayed; the reference counting and the label strings are no-ops.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def f32(v):
    return struct.unpack('<f', struct.pack('<f', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x1411dd000, 0x1411df000), (0x141a6b000, 0x141a6c000), (0x141294000, 0x141295000)])
    rng = case.rng
    for address in (0x140d72460, 0x140f59550, 0x1405f3830, 0x1422e7c2c):
        case.stub(address, lambda call, rng: None, record=False)
    print('# balance vectors (tools/gen_balance_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        F = case.region('F', 0xc000)
        B = case.region('B', 0x6300)
        sp = entry_rsp(8)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)
        ACC = case.region('ACC', 0x40)
        FLOW = case.region('FLOW', 0x6600)
        L = case.region('L', 0xbd00)
        W = case.region('W', 0x36c8 * 2)
        ITEMS = case.region('ITM', 0x68 * 4)
        ATT = case.region('ATT', 0x3770 * 4)
        KEYS = case.region('KEYS', 16 * 4)
        CNT = case.region('CNT', 16 * 4)
        RECS = case.region('RECS', 0x90 * 4)
        OBJ = case.region('OBJ', 0x3e40 * 2)
        ENT = case.region('ENT', 0x200)
        SH = case.region('SH', 0x3e40)
        fz.region('KG', 0x142f03778, 0x10)

        def put(name, off, v):
            fz.preset(name, off, v & 0xffffffff)
            fz.preset(name, off + 4, v >> 32)
        for off in (0xbcc8, 0xbcd0):
            fz.preset('F', off, 0)
        put('F', 0x20, B)
        put('F', 0xb588, FLOW)
        for off in (0x64f4, 0x64f8, 0x64fc):
            fz.preset_f32('FLOW', off, rng.uniform(-60, 60))
        for off in (0xb79c, 0xb798, 0xb7a0, 0xb7a4, 0xb838, 0x28c, 0x6534, 0x284, 0x118, 0x11c):
            fz.preset_f32('F', off, rng.choice([0.0, rng.uniform(0, 100)]))
        for k in range(9):
            fz.preset_f32('F', 0x260 + 4 * k, rng.uniform(0, 100))
            fz.preset_f32('F', 0xbbb4 + 4 * k, rng.choice([0.0, rng.uniform(0, 50)]))
        for off in (0x288c, 0x28ac, 0x28b0, 0x28b4, 0x28c8, 0x28cc, 0x28d0, 0x2914, 0x2918, 0x2920, 0x2928, 0x2268, 0x226c):
            fz.preset_f32('B', off, rng.uniform(-3, 100) if off == 0x288c else rng.uniform(-3, 3))
        fz.preset('B', 0x28d4, rng.choice([0, 0, 1]))
        fz.preset('B', 0x28d8, rng.choice([0, 0, 1]))
        for k in range(9):
            fz.preset_f32('B', 0x412c + 4 * k, rng.choice([0.0, rng.uniform(0, 100)]))
            fz.preset('B', 0x4108 + 4 * k, rng.choice([0, 1, 2]))
            for j in range(3):
                fz.preset_f32('B', 0x4150 + 12 * k + 4 * j, rng.uniform(-5, 5))
        # tanks (positions through the part transform)
        scale = rng.uniform(0.5, 3)
        fz.preset_f32('B', 0x2888, scale)
        put('B', 0x61f8, L)
        put('B', 0x6028, W)
        for k in range(9):
            size = rng.choice([0.0, rng.uniform(1, 200)])
            fz.preset_f32('B', 0x3f7c + 4 * k, size)
            capacity = max(f32(size) * f32(scale), 0.01)
            fz.preset_f32('L', 0xbbb4 + 4 * k, rng.choice([0.0, capacity, rng.uniform(0, 1.2) * capacity]))
            fz.preset('B', 0x40e4 + 4 * k, rng.choice([-1, -1, 0, 1]) & 0xffffffff)
            for j in range(3):
                fz.preset_f32('B', 0x400c + 12 * k + 4 * j, rng.uniform(-5, 5))
                fz.preset_f32('B', 0x4078 + 12 * k + 4 * j, rng.uniform(-5, 5))
        for off, lo, hi in ((0x30, -90, 90), (0x20, -90, 90), (0x38, -90, 90), (0x28, -90, 90), (0x40, -3, 3), (0x54, -3, 3), (0x10, -3, 3), (0, -2, 2)):
            for part in range(2):
                fz.preset_f32('W', 0x36c8 * part + off, rng.uniform(lo, hi))
        for part in range(2):
            fz.preset('W', 0x36c8 * part + 0x3c, rng.randrange(-3, 4) & 0xffffffff)
            for off in (0x6f0, 0x6f4, 0x6f8):
                fz.preset_f32('W', 0x36c8 * part + off, rng.uniform(-5, 5))
        for off, lo, hi in ((0x6070, -2, 2), (0x6074, -2, 2), (0x6078, 0.5, 4)):
            fz.preset_f32('B', off, rng.choice([rng.uniform(lo, hi)] + ([0.0] if off == 0x6070 else [])))
        # removed items
        fz.preset('B', 0x91c, rng.randrange(0, 4))
        put('B', 0x5ff8, ITEMS)
        put('B', 0x6010, ATT)
        for k in range(4):
            fz.preset_f32('ITM', 0x68 * k + 0x38, rng.uniform(0, 30))
            for j in range(3):
                fz.preset_f32('ATT', 0x3770 * k + 0x790 + 4 * j, rng.uniform(-5, 5))
        # attached objects
        entries = rng.randrange(0, 4)
        put('F', 0x69b8, KEYS)
        put('F', 0x69c0, KEYS + 16 * entries)
        put('F', 0x69a0, CNT)
        put('B', 0x60c8, RECS)
        for off, v in ((0, 0xffffffff), (4, 0), (8, 0xffffffff), (12, 0xffffffff)):
            fz.preset('KG', off, v)
        for k in range(4):
            same = rng.random() < 0.3
            fz.preset('KEYS', 16 * k, 0xffffffff if same else rng.randrange(0, 1 << 30))
            fz.preset('KEYS', 16 * k + 4, 0)
            fz.preset('KEYS', 16 * k + 8, 0xffffffff if same else rng.randrange(0, 1 << 30))
            fz.preset('KEYS', 16 * k + 12, 0xffffffff if same else 0)
            fz.preset('CNT', 16 * k, rng.randrange(0, 20))
            for j in (0x6c, 0x74, 0x7c):
                fz.preset_f32('RECS', 0x90 * k + j, rng.uniform(-5, 5))
            if rng.random() < 0.7:
                base = 0x3e40 * (k % 2)
                put('RECS', 0x90 * k + 0x30, ENT)
                put('RECS', 0x90 * k + 0x88, OBJ + base)
                fz.preset_f32('OBJ', base + 0x37d8, rng.uniform(0, 50))
                fz.preset('OBJ', base + 0x36e8, rng.choice([0, 1, 4, 5, 6, 7, 8, 9]))
                fz.preset_f32('OBJ', base + 0xa8, rng.uniform(0, 3))
                for off in (0x3e08, 0x3e0c, 0x3e10):
                    fz.preset_f32('OBJ', base + off, rng.uniform(-5, 5))
                for off, v in ((0x37b0, 5), (0x37b4, 0), (0x37b8, 15), (0x37bc, 0)):
                    fz.preset('OBJ', base + off, v)
            else:
                put('RECS', 0x90 * k + 0x30, 0)
                put('RECS', 0x90 * k + 0x88, 0)
        put('ENT', 0x178, SH)
        fz.preset_f32('ENT', 0xbc, rng.uniform(0, 50))
        for off in (0x788, 0x78c, 0x790):
            fz.preset_f32('SH', off, rng.uniform(-5, 5))
        for off, v in ((0x37b0, 5), (0x37b4, 0), (0x37b8, 15), (0x37bc, 0)):
            fz.preset('SH', off, v)
        case.stub(0x140f5c540, lambda call, rng: call.ret_int(rng.choice([0, 1, 1])))
        case.stub(0x140f42620, lambda call, rng: call.ret_int(ENT))
        case.stub(0x1407ace10, lambda call, rng: call.ret_int(rng.choice([0, 1, 1])))
        w6, w9, w12 = (rng.uniform(-1, 1) for _ in range(3))
        stop = rng.choice([0, 0, 0, 1])
        total, i7, i10, i13, m8, m11, m14 = (ACC + o for o in (0, 4, 8, 12, 16, 20, 24))
        for off in range(0, 0x20, 4):
            fz.preset_f32('ACC', off, rng.uniform(-50, 50))
        stack = [m8, bits(w9), i10, m11, bits(w12), i13, m14, stop]
        for k, value in enumerate(stack):
            off = sp + 0x28 + 8 * k - (sp - 0x800)
            fz.preset('S', off, value & 0xffffffff)
            fz.preset('S', off + 4, value >> 32)
        try:
            case.run(0x1411ddfd0, ints=[F, total, 0, i7], floats=[0.0, 0.0, w6, 0.0], stack=stack, max_instructions=400000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        text = case.dump(f'{F:x} {ACC:x} {bits(w6):x} {bits(w9):x} {bits(w12):x} {stop:x}')
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
