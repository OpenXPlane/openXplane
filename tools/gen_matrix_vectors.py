#!/usr/bin/env python3
"""Vectors for matrix::{mul, rigid_inverse, axis_rotation, rotate_by} (the originals 0x1407b33a0, 0x14093c4f0,
0x140cbabc0, 0x140cb9860): 4 x 4 matrices of doubles (16 doubles, 0x80 bytes, translation in the last row).

    python3 tools/gen_matrix_vectors.py Xplane12/X-Plane.exe mul|rigid|axis|rotate TRIALS SEED > crates/xp-app/tests/data/matrix_KIND.txt

The header is `KIND` followed by the addresses of the matrices and the float arguments as double bits (angle in
degrees, axis x y z).
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from xp_vmcase import VmCase, entry_rsp  # noqa: E402
from unicorn.x86_const import UC_X86_REG_XMM1, UC_X86_REG_XMM2, UC_X86_REG_XMM3  # noqa: E402

EXE, KIND, TRIALS, SEED = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])


def dbits(v):
    return struct.unpack('<Q', struct.pack('<d', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x1407b3000, 0x1407b4000), (0x14093c000, 0x14093d000), (0x140cba000, 0x140cbb000),
                              (0x140cb9000, 0x140cba000), (0x1419f7000, 0x1419f9000), (0x14062c000, 0x14062d000), (0x1419f6000, 0x1419f7000), (0x14089c000, 0x14089d000)])
    rng = case.rng
    print(f'# matrix vectors {KIND} (tools/gen_matrix_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        M = case.region('M', 0x80)
        A = case.region('A', 0x80)
        B = case.region('B', 0x80)
        R = case.region('R', 0xc0)
        CX = case.region('CX', 0x300)
        lat = rng.uniform(-89, 89)
        lon = rng.uniform(-179, 179)
        case.stub(0x1419f8ff0, lambda call, rng: call.ret_f64(rng.uniform(-60, 60)))
        fz.preset_f64('CX', 0xb0, 6378137.0)
        fz.preset_f64('CX', 0xb8, rng.uniform(6350000.0, 6378000.0))
        fz.preset_f64('CX', 0xc8, rng.uniform(0.0, 0.01))
        for off in range(0x200, 0x240, 8):
            fz.preset_f64('CX', off, rng.uniform(-1, 1))
        angle = rng.choice([rng.uniform(-360, 360), rng.uniform(-90, 90), 0.0, 90.0])
        axis = [rng.choice([0.0, rng.uniform(-3, 3)]) for _ in range(3)]
        mode = rng.randrange(6)
        if mode == 0:
            axis = [rng.choice([0.0, 1.0, -1.0, 2.5, -0.3]), 0.0, 0.0]
        elif mode == 1:
            axis = [0.0, rng.choice([1.0, -1.0, 0.7]), 0.0]
        elif mode == 2:
            axis = [0.0, 0.0, rng.choice([1.0, -1.0, 4.0])]
        elif mode == 3 and rng.random() < 0.3:
            axis = [0.0, 0.0, 0.0]
        for base, name in ((M, 'M'), (A, 'A'), (B, 'B')):
            for k in range(16):
                fz.preset_f64(name, 8 * k, rng.uniform(-3, 3))
        header_args = ''
        stack = []
        if KIND == 'mul':
            run = (0x1407b33a0, [R, A, B], [])
            header_args = f'{R:x} {A:x} {B:x}'
        elif KIND == 'rigid':
            run = (0x14093c4f0, [R, A], [])
            header_args = f'{R:x} {A:x}'
        elif KIND == 'axis':
            run = (0x140cbabc0, [R], [dbits(axis[2])])
            header_args = f'{R:x} {dbits(angle):016x} {dbits(axis[0]):016x} {dbits(axis[1]):016x} {dbits(axis[2]):016x}'
        elif KIND == 'euler':
            P = case.region('P', 0x10)
            seeds = [rng.uniform(-180, 180) for _ in range(3)]
            for k in range(3):
                fz.preset_f32('P', 4 * k, seeds[k])
            import math

            def rot(h, p, r):  # a rotation matrix of floats from three angles (degrees)
                h, p, r = (math.radians(v) for v in (h, p, r))
                ch, sh, cp, sp, cr, sr = math.cos(h), math.sin(h), math.cos(p), math.sin(p), math.cos(r), math.sin(r)
                return [[ch * cr + sh * sp * sr, -ch * sr + sh * sp * cr, sh * cp, 0], [sr * cp, cr * cp, -sp, 0],
                        [-sh * cr + ch * sp * sr, sh * sr + ch * sp * cr, ch * cp, 0], [0, 0, 0, 1]]

            if rng.random() < 0.7:
                mtx = rot(rng.uniform(-180, 180), rng.choice([rng.uniform(-90, 90), 90.0, -90.0, 0.0]), rng.uniform(-180, 180))
            else:
                mtx = [[rng.uniform(-1, 1) for _ in range(3)] + [0] for _ in range(3)] + [[0, 0, 0, 1]]
            for k in range(16):
                fz.preset_f32('M', 4 * k, mtx[k // 4][k % 4])
            run = (0x1419f6fd0, [M, P, P + 4, P + 8], [])
            header_args = f'{M:x} {P:x}'
        elif KIND == 'axes':
            run = (0x1419f7ee0, [CX, R], [])
            header_args = f'{CX:x} {R:x} {dbits(lat):016x} {dbits(lon):016x}'
        else:
            run = (0x140cb9860, [M], [dbits(axis[2])])
            header_args = f'{M:x} {dbits(angle):016x} {dbits(axis[0]):016x} {dbits(axis[1]):016x} {dbits(axis[2]):016x}'
        sp = entry_rsp(len(run[2]))
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff)
        fz.preset('S', 0x804, RET_ADDR >> 32)
        for k, value in enumerate(run[2]):
            off = sp + 0x28 + 8 * k - (sp - 0x800)
            fz.preset('S', off, value & 0xffffffff)
            fz.preset('S', off + 4, value >> 32)
        for reg, value in ((UC_X86_REG_XMM1, angle), (UC_X86_REG_XMM2, axis[0] if KIND != 'axes' else lat), (UC_X86_REG_XMM3, axis[1] if KIND != 'axes' else lon)):
            case.emu.uc.reg_write(reg, dbits(value))
        try:
            case.run(run[0], ints=run[1], floats=[], stack=run[2], max_instructions=20000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        text = case.dump(f'{KIND} {header_args}')
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
