#!/usr/bin/env python3
"""Vectors for terrain::segment_probe (the original 0x14195ffc0, a segment against the mesh with moving cells).

    python3 tools/gen_segment_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/segment_probe.txt

The header of a case is `mesh p0 p1 out normal moved id result`. The triangle normal 0x1406ed6a0 is replayed.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX  # noqa: E402
from xp_vmcase import VmCase, entry_rsp, setup_segment_mesh  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
SURFACES = 0x14611ac88


def main():
    case = VmCase(EXE, SEED, [(0x14195f000, 0x141962000), (0x14194c000, 0x14194d000)])
    rng = case.rng
    inside = (0x14195ffc0, 0x141960dc0)

    def normal(call, rng):
        for k in range(3):
            call.put_f32(call.ints[3] + 4 * k, rng.uniform(-1, 1))
    case.stub(0x1406ed6a0, normal, caller=inside)
    print('# segment probe vectors (tools/gen_segment_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        mesh = case.region('MS', 0x200)
        sp = entry_rsp(3)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)
        setup_segment_mesh(case, rng)
        P = case.region('P', 32)
        p0 = [rng.uniform(-6, 6), rng.uniform(0, 30), rng.uniform(-6, 6)]
        p1 = [v + rng.uniform(-10, 10) for v in p0]
        if rng.random() < 0.65:
            p1[1] = p0[1] - rng.uniform(5, 40)
        for i in range(3):
            fz.preset_f32('P', 4 * i, p0[i])
            fz.preset_f32('P', 16 + 4 * i, p1[i])
        OUT = case.region('OUT', 16)
        NRM = case.region('NRM', 16)
        MV = case.region('MV', 16)
        ID = case.region('ID', 16)
        out = rng.choice([0, OUT, OUT])
        normal_ptr = rng.choice([0, NRM, NRM])
        moved = rng.choice([0, MV, MV])
        ident = rng.choice([0, ID, ID])
        for k, value in enumerate((normal_ptr, moved, ident)):
            off = sp + 0x28 + 8 * k - (sp - 0x800)
            fz.preset('S', off, value & 0xffffffff)
            fz.preset('S', off + 4, value >> 32)
        try:
            case.run(0x14195ffc0, ints=[mesh, P, P + 16, out], stack=[normal_ptr, moved, ident], max_instructions=400000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        result = case.emu.reg(UC_X86_REG_RAX) & 0xff
        header = f'{mesh:x} {P:x} {P + 16:x} {out:x} {normal_ptr:x} {moved:x} {ident:x} {result:x}'
        text = case.dump(header)
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
