#!/usr/bin/env python3
"""Replay original bounds/size block with explicit synthetic point vectors; no stubs."""
import hashlib
import random
import struct
import sys
from pathlib import Path
from emulate_xp import Emulator, STACK_TOP
from unicorn.x86_const import UC_X86_REG_R12, UC_X86_REG_RDI, UC_X86_REG_RSP, UC_X86_REG_XMM11

SHA256 = '2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
BEGIN, END = 0x141a89096, 0x141a8923e
OFFSETS = [0x64dc, 0x64e4, 0x64ec, 0x64e0, 0x64e8, 0x64f0, 0x64d8, 0x64f4, 0x64f8, 0x64fc]

def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]

def cases():
    yield []
    for v in [0., -0., 1., -1., .001, 1e-30, 1e30]:
        yield [[v, v, v]]
    yield [[-0., 0., -0.], [0., -0., 0.]]
    yield [[0., -0., 0.], [-0., 0., -0.]]
    yield [[1., 2., 3.], [4., 5., 6.]]
    yield [[-1., -2., -3.], [-4., -5., -6.]]
    yield [[-2., 4., -6.], [2., -4., 6.]]
    rng = random.Random(20261013)
    for _ in range(1000):
        yield [[rng.uniform(-1000., 1000.) for _ in range(3)] for _ in range(rng.randrange(0, 33))]

def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != SHA256:
        raise ValueError('reference SHA256 mismatch')
    emu = Emulator(path)
    obj = emu.alloc(0x7000)
    points = emu.alloc(4096)
    print('# reference_sha256', SHA256)
    print(f'# original_range {BEGIN:#x} {END:#x} exclusive; no stubs; explicit synthetic point list')
    print('# B count xyz_f32bits... -> min_xyz max_xyz planar_halfspan size_xyz (10 f32 words)')
    count = 0
    for case in cases():
        raw = struct.pack('<' + 'f' * (3*len(case)), *[v for p in case for v in p])
        emu.write(points, raw)
        emu.write_u64(obj+0x6478, points)
        emu.write_u64(obj+0x6480, points+len(raw))
        for reg, value in [(UC_X86_REG_R12,obj),(UC_X86_REG_RDI,0),(UC_X86_REG_RSP,STACK_TOP-0x1000),(UC_X86_REG_XMM11,0)]:
            emu.uc.reg_write(reg,value)
        emu.run_range(BEGIN,END)
        assert not emu.stubs
        words = struct.unpack('<'+'I'*(3*len(case)),raw)
        output = [emu.read_u32(obj+off) for off in OFFSETS]
        print('B',len(case),*(f'{v:08x}' for v in [*words,*output]))
        count += 1
    print('# cases',count)

if __name__ == '__main__':
    main(sys.argv[1])
