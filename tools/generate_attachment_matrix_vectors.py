#!/usr/bin/env python3
"""Synthetic vectors for original attachment affine builder 0x141775d10.

python tools/generate_attachment_matrix_vectors.py <X-Plane.exe> <output.tsv>
All builder and CRT instructions execute originally; no callees are stubbed.
"""
import hashlib
import random
import struct
import sys
from pathlib import Path

from emulate_xp import Emulator
from generate_attachment_vectors import SHA256
from unicorn.x86_const import UC_X86_REG_XMM0


def f32(v):
    return struct.unpack('<f', struct.pack('<f', v))[0]


def words(vs):
    return ','.join(f'{struct.unpack("<I", struct.pack("<f", v))[0]:08x}' for v in vs)


def main():
    exe, output = map(Path, sys.argv[1:])
    assert hashlib.sha256(exe.read_bytes()).hexdigest() == SHA256
    e = Emulator(exe)
    assert e.read_u32(0x14250e1a8) == 0x3c8efa36
    out = e.alloc(48)
    angles = [(0,0,0)]
    for axis in range(3):
        for angle in (-360,-180,-90,-45,-0.0,30,90,180,360):
            v=[0,0,0];v[axis]=angle;angles.append(tuple(v))
    angles += [(15,25,35),(-75,40,-120),(90,90,90),(180,90,-180)]
    rng = random.Random(0x775d10)
    angles += [tuple(rng.uniform(-720,720) for _ in range(3)) for _ in range(96)]
    rows=['# position_xyz,angles_psi_the_phi bits\toriginal sin/cos pairs\toriginal 3x4 row-major matrix bits']
    for a in angles:
        position=[f32(rng.uniform(-20,20)) for _ in range(3)]
        a=list(map(f32,a))
        assert e.call(0x141775d10,ints=(out,),floats=(0,*position),stack=tuple(a)) == out
        expected=','.join(f'{e.read_u32(out+4*i):08x}' for i in range(12))
        trig=[]
        for angle in a:
            radians=f32(angle*f32(struct.unpack('<f',struct.pack('<I',0x3c8efa36))[0]))
            for fn in (0x14230be80,0x14230b380):
                e.call(fn,floats=(radians,))
                trig.append(e.reg(UC_X86_REG_XMM0)&0xffffffff)
        rows.append('\t'.join((words(position+a),','.join(f'{w:08x}' for w in trig),expected)))
    assert not e.stubs
    output.write_text('\n'.join(rows)+'\n',encoding='utf-8')
    print(f'{len(angles)} native matrices; original sinf/cosf; no stubs')


if __name__=='__main__':
    main()
