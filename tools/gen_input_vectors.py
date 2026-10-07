#!/usr/bin/env python3
"""Vectors for input::key_slot (0x1417da870) and input::bank_available (0x141185030).

    python3 tools/gen_input_vectors.py Xplane12/X-Plane.exe keyslot|avail TRIALS SEED > crates/xp-app/tests/data/input_KIND.txt

Header: `KIND obj arg result`. Stack words are not compared.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from xp_vmcase import VmCase, entry_rsp  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX  # noqa: E402

EXE, KIND, TRIALS, SEED = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])


def main():
    case = VmCase(EXE, SEED, [(0x1417da000, 0x1417db000), (0x141185000, 0x141186000)])
    case.stub(0x1407ace10, lambda call, rng: call.ret_int(rng.choice([0, 0, 1])))
    rng = case.rng
    print(f'# input vectors {KIND} (tools/gen_input_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        OBJ = case.region('OBJ', 0xd000)
        B = case.region('B', 0x3000)
        sp = entry_rsp(0)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff)
        fz.preset('S', 0x804, RET_ADDR >> 32)
        arg = 0
        if KIND == 'keyslot':
            arg = rng.randrange(0, 8)
            for k in range(500):
                fz.preset('OBJ', 0x8cf8 + 4 * k, rng.randrange(0, 8))
                fz.preset('OBJ', 0x8cf8 + 0xba44 + 4 * k, rng.choice([0, 1, 1]))
            run = (0x1417da870, [OBJ, arg])
        else:
            fz.preset('OBJ', 0x7c9c, rng.choice([-1, 0, 1, 2]))
            fz.preset('OBJ', 0x7d34, rng.choice([0, 1, 1]))
            fz.preset('OBJ', 0x7b90, OBJ + 0x100)
            fz.preset('OBJ', 0x7b94, 0)
            fz.preset('OBJ', 0x20, B & 0xffffffff, record=True)
            fz.preset('OBJ', 0x24, B >> 32, record=True)
            fz.preset('B', 0x2270, rng.choice([0, 1, 2, 3]))
            run = (0x141185030, [OBJ])
        try:
            case.run(run[0], ints=run[1], floats=[], stack=[], max_instructions=100000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        result = case.emu.reg(UC_X86_REG_RAX) & 0xffffffff
        text = case.dump(f'{KIND} {OBJ:x} {arg} {result:x}')
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
