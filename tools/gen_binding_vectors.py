#!/usr/bin/env python3
"""Vectors for binding::input_active (the original 0x1407ace10, the input-binding query of the flight object).

    python3 tools/gen_binding_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/binding.txt

The header of a case is `F mode a b amount result`. The static tables at 0x142515560 and 0x142515366 are
recorded with the cases; 0x141238b40 and 0x14082be10 are replayed.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX  # noqa: E402
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
IDS = [0x45, 0x47, 0x48, 0x49, 0x44, 0x4a, 0xfa, 0x43, 0x10f, 0x110, 0x111, 0x112, 0x113, 0x2fa, 0x102, 0x103,
       0x104, 0x105, 0x106, 0x107, 0x108, 0x109, 0x10a, 0x10b, 0x12f, 0x130, 0x10e]


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x1407ac000, 0x1407ae000)])
    rng = case.rng
    print('# binding vectors (tools/gen_binding_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        F = case.region('F', 0x6000)
        B = case.region('B', 0x6100)
        sp = entry_rsp(1)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)
        ENG = case.region('ENG', 0x68)
        OBJ = case.region('OBJ', 0x40)

        def put(name, off, v):
            fz.preset(name, off, v & 0xffffffff)
            fz.preset(name, off + 4, v >> 32)
        put('F', 0x20, B)
        put('B', 0x5ff8, ENG)
        fz.preset('ENG', 0, rng.choice([0, 1, 2, 3, 5]))
        put('B', 0x1d18, rng.choice([0, OBJ]))
        ident = rng.choice(IDS + IDS + [0x45, 0xfa, 0x106] + [rng.randrange(0, 0x302)])
        b = rng.choice([0, 0, 1, 5])
        a = ident - b
        mode = rng.choice([1, 1, 2, 2, 0, 3])
        amount = rng.uniform(-1, 1)
        for off in (0x55c, 0x564, 0x5ac, 0x5b0, 0x5b4, 0x5b8, 0x5bc, 0x5c0, 0x5c4, 0x5f0, 0x5f4):
            fz.preset('F', off, rng.choice([0, 0, 1, 2, 5]))
        fz.preset('F', 0xdc0 + 4 * ident, rng.choice([0, 0, 0, 6, 1]))
        byte_at = 0x55f0 + ident
        fz.preset('F', byte_at & ~3, rng.choice([0, 0, 1]) << (8 * (byte_at & 3)), record=True)
        for k in range(6):
            fz.preset_f32('F', 0xd18 + 4 * k, rng.choice([-1.0, 0.0, rng.uniform(-0.5, 1.5), 1.0]))
            fz.preset_f32('F', 0xd30 + 4 * k, rng.uniform(-2, 2))
        fz.preset('B', 0x918, rng.choice([0, 1, 1]))
        fz.preset('B', 0x91c, rng.choice([0, 1, 1, 2]))
        fz.preset('B', 0xad8, rng.choice([-1, 0, 3, 12, 13, 40]) & 0xffffffff)
        fz.preset('B', 0xc90, rng.randrange(0, 8))
        fz.preset_f32('B', 0x1b7c, rng.choice([0.0, 0.5, 1.0, 0.2]))
        mask_at = 0x2998 + ident
        fz.preset('B', mask_at & ~3, rng.choice([0, 1, 2, 3, 4, 8, 0x10, 0x20, 0x3f, 0x25, rng.randrange(0, 64)]) << (8 * (mask_at & 3)), record=True)
        fz.preset_f32('B', 0x2c9c + 4 * ident, rng.uniform(-2, 2))
        if ident == 0x45:
            fz.preset('F', 0xdc0 + 4 * 0xfa, rng.choice([0, 0, 6]))
            second = 0x2998 + 0xfa
            fz.preset('B', second & ~3, rng.choice([0, 1, 3, 0x20]) << (8 * (second & 3)), record=True)
            fz.preset_f32('B', 0x2c9c + 4 * 0xfa, rng.uniform(-2, 2))
            fz.preset('F', (0x55f0 + 0xfa) & ~3, rng.choice([0, 1]) << (8 * ((0x55f0 + 0xfa) & 3)), record=True)
        # the static tables of the executable
        fz.region('T2', 0x142515560, 4 * 0x302)
        fz.region('T3', 0x142515364, 40 * 13 + 4)
        for entry in {ident, 0xfa}:
            fz.preset('T2', 4 * entry, case.emu.read_u32(0x142515560 + 4 * entry))
        for k in range(13):
            word = case.emu.read_u32(0x142515364 + 40 * k)
            fz.preset('T3', 40 * k, word)
        case.stub(0x141238b40, lambda call, rng: call.ret_int(rng.choice([0, 1])))
        case.stub(0x14082be10, lambda call, rng: call.ret_int(rng.choice([0, 1])))
        off = sp + 0x28 - (sp - 0x800)
        fz.preset('S', off, bits(amount))
        fz.preset('S', off + 4, 0)
        try:
            case.run(0x1407ace10, ints=[F, mode, a & 0xffffffff, b], stack=[amount], max_instructions=100000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        result = case.emu.reg(UC_X86_REG_RAX) & 0xffffffff
        text = case.dump(f'{F:x} {mode:x} {a & 0xffffffff:x} {b:x} {bits(amount):x} {result:x}')
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
