#!/usr/bin/env python3
"""Vectors for the callees of the flight object initialisation (flight_init.rs).

    python3 tools/gen_init_callee_vectors.py Xplane12/X-Plane.exe KIND TRIALS SEED > crates/xp-app/tests/data/init_KIND.txt

KIND: `throttle` (0x14122cbc0, header `F engine a-bits b-bits`), `resetb` (0x1411dd2c0, header `this`), `flags`
(0x14121a690, header `this flag`), `clear` (0x14087e590, header `object`).
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, KIND, TRIALS, SEED = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
ENTRY = {'throttle': 0x14122cbc0, 'resetb': 0x1411dd2c0, 'flags': 0x14121a690, 'clear': 0x14087e590}[KIND]
STUBS = [0x14122c470, 0x140b0c6d0, 0x141a6ba30, 0x14123d190]


def pages(addresses):
    return sorted({(a & ~0xfff, (a & ~0xfff) + 0x1000) for a in addresses})


def main():
    case = VmCase(EXE, SEED, pages([ENTRY, ENTRY + 0x800] + STUBS + [0x1423c0920, 0x1423c0920 + 0x1000]))
    rng = case.rng
    f32 = lambda v: struct.unpack('<I', struct.pack('<f', v))[0]
    print(f'# init callee vectors {KIND} (tools/gen_init_callee_vectors.py)')
    done = 0
    while done < TRIALS:
        state = rng.getstate()
        prefill = None
        for attempt in (0, 1):
            if attempt == 1:
                rng.setstate(state)
                prefill = dry
            case.reset()
            fz = case.fz
            F = case.region('F', 0xc000)
            B = case.region('B', 0x6700)
            sp = entry_rsp(0)
            fz.region('S', sp - 0x800, 0x900)
            fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
            fz.preset('S', 0x804, RET_ADDR >> 32, record=False)

            def put(name, off, v):
                fz.preset(name, off, v & 0xffffffff)
                fz.preset(name, off + 4, v >> 32)
            E = case.region('E', 0x2cc * 4)

            def fill(name, lo, hi):
                # words the function overwrites start nonzero, so that a missing or misplaced store shows
                for off in range(lo, hi, 4):
                    fz.preset(name, off, rng.randrange(1, 1 << 32))
            put('F', 0x20, B)
            put('F', 0x68b0, E)
            p1, p3 = rng.choice([0.0, 0.1, 0.5, 0.95]), rng.choice([0.0, 0.1, 0.5, 0.95])
            case.stub(0x14122c470, lambda call, rng: None)
            case.stub(0x140b0c6d0, lambda call, rng: None)
            case.stub(0x14123d190, lambda call, rng: call.ret_int(rng.choice([1, 7]) if rng.random() < (p1 if call.ints[2] == 1 else p3) else 0))
            case.stub(0x141a6ba30, lambda call, rng: call.put_f32(call.ints[2] + 8, rng.uniform(-2, 2)))
            floats = []
            if KIND == 'throttle':
                fz.preset('B', 0xa90, rng.choice([0, 1]))
                fz.preset('B', 0xae4, rng.choice([3, 3, 0]))
                for k in range(4):
                    fz.preset('E', 0x2cc * k + 0x298, rng.choice([1, 2, 3, 0]))
                    fz.preset_f32('E', 0x2cc * k + 0x280, rng.choice([0.0, 0.5, rng.uniform(-1, 2)]))
                    fz.preset_f32('E', 0x2cc * k, rng.uniform(-1, 2))
                engine = rng.randrange(0, 4)
                vals = [0.0, 0.05, 0.0505, 0.0504, 0.06, 0.5, 1.5, -0.3, 0.001, rng.uniform(-1, 2), rng.uniform(0, 0.2)]
                a, b = rng.choice(vals), rng.choice(vals)
                if rng.random() < 0.15:
                    a = 0.0
                if rng.random() < 0.1:
                    b = 0.0
                header = f'{F:x} {engine:x} {f32(a):x} {f32(b):x}'
                run = dict(ints=[F, engine, 0, 0], floats=[0.0, 0.0, a, b])
            elif KIND == 'resetb':
                this = F + 0xb580
                put('F', 0xb580, F)
                put('F', 0xb588, B)
                fz.preset('B', 0x91c, rng.randrange(0, 4))
                fz.preset('B', 0x920, rng.randrange(0, 4))
                n = rng.randrange(0, 4)
                VEC = case.region('VEC', 0x120 * 4)
                put('B', 0x60e0, VEC)
                put('B', 0x60e8, VEC + 0x120 * n)
                for k in range(4):
                    put('VEC', 0x120 * k + 0x118, rng.choice([0, 0x4444]))
                fz.preset_f32('B', 0x225c, rng.choice([0.0, 30.0, 29.99, 31.0, rng.uniform(0, 100)]))
                fz.preset_f32('B', 0x2260, rng.uniform(-5, 5))
                header = f'{this:x}'
                run = dict(ints=[this, 0, 0, 0])
            elif KIND == 'flags':
                this = F + 0xbb30
                put('F', 0xbb30, F)
                put('F', 0xbb38, B)
                density = rng.choice([0.0, 0.3, 0.7])
                fz.preset('B', 0x91c, rng.randrange(0, 4))
                ITEMS = case.region('ITEMS', 0x68 * 4)
                put('B', 0x5ff8, ITEMS)
                for k in range(4):
                    fz.preset_f32('ITEMS', 0x68 * k + 0x4c, rng.choice([0.0, -1.0, 1.0, rng.uniform(-1, 1)]))
                for off in (0x1cc4, 0x1cc0, 0x1cb4):
                    fz.preset('B', off, rng.choice([0, 1]) if rng.random() > density else 0)
                for k in range(6):
                    fz.preset('B', 0x1cc8 + 4 * k, rng.choice([0, 1, 4, 5]))
                for k in range(9):
                    fz.preset_f32('B', 0x3f7c + 4 * k, rng.uniform(-2, 2))
                    fz.preset_f32('F', 0xbb30 + 0x84 + 4 * k, rng.uniform(-2, 2))
                fz.preset_f32('B', 0x28a4, rng.choice([0.0, 0.1, 0.0999, 0.11, rng.uniform(0, 1)]))
                flag = rng.choice([0, 1, 5])
                header = f'{this:x} {flag:x}'
                run = dict(ints=[this, flag, 0, 0])
            else:
                OBJ = case.region('OBJ', 0x40)
                OWN = case.region('OWN', 0x80)
                n = rng.choice([0, 1, 2, 3, 4, 5, 8, 9, 12, 13, -2])
                put('OBJ', 0, OWN)
                put('OWN', 0x58, 0x1000)
                put('OWN', 0x60, 0x1000 + 0x180 * n)
                regions = []
                for k, off in enumerate((0x10, 0x18, 0x20)):
                    name = f'A{k}'
                    case.region(name, 4 * 16)
                    put('OBJ', off, case.regions[name])
                    for j in range(16):
                        fz.preset(name, 4 * j, rng.randrange(1, 1 << 32))
                header = f'{OBJ:x}'
                run = dict(ints=[OBJ, 0, 0, 0])
            # words the function overwrites start nonzero, so that a missing or misplaced store shows: a dry run finds
            # them, the second run presets those not otherwise set
            if prefill is not None:
                pr = random.Random(SEED + 99)
                for name, offsets in prefill.items():
                    if name not in fz.regions or name.startswith('S'):
                        continue
                    for off in offsets:
                        if off not in fz.initial()[name]:
                            fz.preset(name, off, pr.randrange(1, 1 << 32))
            try:
                case.run(ENTRY, max_instructions=400000, **run)
            except RuntimeError as err:
                sys.stderr.write(f'trial failed: {err}\n')
                break
            if attempt == 0:
                dry = {n: sorted(o) for n, o in case.fz.written().items()}
                continue
            text = case.dump(header)
            lines = text.split('\n')
            lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
            print('\n'.join(lines))
            done += 1


if __name__ == '__main__':
    main()
