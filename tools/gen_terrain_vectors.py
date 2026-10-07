#!/usr/bin/env python3
"""Vectors for terrain::probe (the original 0x141960dc0, a vertical ray against a triangle mesh).

    python3 tools/gen_terrain_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/terrain_probe.txt

The header of a case is `mesh p out normal extra grid`; the point `p`, the outputs and the mesh live in recorded
regions. The grid height 0x14194dc20 and the triangle normal 0x1406ed6a0 are replayed.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX  # noqa: E402
from xp_vmcase import GRID_GLOBAL, VmCase, entry_rsp, setup_terrain, stub_terrain_helpers  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])


def main():
    case = VmCase(EXE, SEED, [(0x141960000, 0x141962000), (0x14194d000, 0x14194e000)])
    rng = case.rng
    stub_terrain_helpers(case, caller=(0x141960dc0, 0x141961200))
    print('# terrain probe vectors (tools/gen_terrain_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        mesh = case.region('F', 0x200)
        sp = entry_rsp(2)
        case.fz.region('S', sp - 0x800, 0x900)
        case.fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        case.fz.preset('S', 0x804, RET_ADDR >> 32, record=False)
        setup_terrain(case, rng, 'F', 0)
        P = case.region('P', 16)
        OUT = case.region('OUT', 64)
        NORMAL = case.region('NRM', 32)
        EXTRA = case.region('EX', 16)
        for i, v in enumerate((rng.choice([0.0, rng.uniform(-6, 6)]), rng.choice([0.0, rng.uniform(-8, 12)]), rng.uniform(-6, 6))):
            case.fz.preset_f32('P', 4 * i, v)
        out = rng.choice([0, OUT, OUT])
        normal = rng.choice([0, NORMAL, NORMAL])
        extra = rng.choice([0, EXTRA])
        if extra:
            case.fz.preset_f32('EX', 8, rng.uniform(-3, 3))
        grid = case.emu.read_u64(GRID_GLOBAL)
        for k, value in enumerate((extra, grid)):
            off = sp + 0x28 + 8 * k - (sp - 0x800)
            case.fz.preset('S', off, value & 0xffffffff)
            case.fz.preset('S', off + 4, value >> 32)
        try:
            case.run(0x141960dc0, ints=[mesh, P, out, normal], stack=[extra, grid], max_instructions=200000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        result = case.emu.reg(UC_X86_REG_RAX) & 0xff
        header = f'{mesh:x} {P:x} {out:x} {normal:x} {extra:x} {grid:x} {result:x}'
        text = case.dump(header)
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
