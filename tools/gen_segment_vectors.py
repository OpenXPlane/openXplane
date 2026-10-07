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
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

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
        fz.region('NZ', 0x14578f1f0, 0x100000)
        count = rng.randrange(10, 26)
        tri = case.region('TR', 36 * count)
        for k in range(count):
            for i in range(9):
                lo, hi = (-8.0, 25.0) if i % 3 == 1 else (-12.0, 12.0)
                fz.preset_f32('TR', 36 * k + 4 * i, rng.uniform(lo, hi))
        flags = case.region('FL', 4 * count)
        for k in range(count):
            fz.preset('FL', 4 * k, rng.randrange(0, 4) | rng.choice([0, 0, 0x8000]) | rng.randrange(0, 4) << 16)
        bounds = sorted(rng.randrange(0, count + 1) for _ in range(5))
        if rng.random() < 0.5:
            bounds[0] = 0
        for off, v in zip((0x70, 0x74, 0x78, 0x7c, 0x80), bounds):
            fz.preset('MS', off, v)
        fz.preset('MS', 0x28, tri & 0xffffffff)
        fz.preset('MS', 0x2c, tri >> 32)
        fz.preset('MS', 0x40, flags & 0xffffffff)
        fz.preset('MS', 0x44, flags >> 32)
        fz.preset_f32('MS', 0x88, rng.choice([100.0, 100.0, rng.uniform(-5, 30)]))
        cells = rng.randrange(0, 4)
        spheres = case.region('SPH', 16 * max(cells, 1))
        records = case.region('CEL', 0x50 * max(cells, 1))
        for c in range(cells):
            for i, (lo, hi) in enumerate(((-8, 8), (-8, 8), (-8, 8), (2, 15))):
                fz.preset_f32('SPH', 16 * c + 4 * i, rng.uniform(lo, hi))
            for off in range(0, 0x48, 4):
                fz.preset_f32('CEL', 0x50 * c + off, rng.uniform(-1.5, 1.5))
            a = rng.randrange(0, count)
            fz.preset('CEL', 0x50 * c + 0x48, a)
            fz.preset('CEL', 0x50 * c + 0x4c, rng.randrange(a, count + 1))
        fz.preset('MS', 0x90, spheres & 0xffffffff)
        fz.preset('MS', 0x94, spheres >> 32)
        fz.preset('MS', 0xa8, records & 0xffffffff)
        fz.preset('MS', 0xac, records >> 32)
        end = records + 0x50 * cells
        fz.preset('MS', 0xb0, end & 0xffffffff)
        fz.preset('MS', 0xb4, end >> 32)
        table = case.region('SR', 36 * 4)
        for r in range(4):
            fz.preset_f32('SR', 36 * r, rng.uniform(0.01, 0.3))
            fz.preset_f32('SR', 36 * r + 4, rng.choice([0.0, rng.uniform(0.1, 3)]))
        fz.region('GS', SURFACES, 8)
        fz.preset('GS', 0, table & 0xffffffff)
        fz.preset('GS', 4, table >> 32)
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
