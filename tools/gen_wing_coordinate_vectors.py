#!/usr/bin/env python3
"""Replay original span position/coordinate block including original interpolation helper."""
import hashlib
import random
import struct
import sys
from pathlib import Path
from emulate_xp import Emulator, STACK_TOP
from gen_wing_vectors import SHA256, bits
from unicorn.x86_const import (UC_X86_REG_RSI, UC_X86_REG_RDI, UC_X86_REG_RBP,
    UC_X86_REG_RSP, UC_X86_REG_R13, UC_X86_REG_R15, UC_X86_REG_XMM13)


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=SHA256:
        raise ValueError('Reference SHA256 mismatch')
    e=Emulator(path);wing=e.alloc(4096);obj=e.alloc(128);frame=e.alloc(1024)+128;ref=e.alloc(0x6600)
    e.write_u64(obj+0x20,ref)
    rng=random.Random(20261012)
    cases=[(n,i) for n in range(1,11) for i in range(-2,n+3)]
    cases += [(rng.randint(1,10),rng.randint(-2,12)) for _ in range(1000)]
    print('# reference_sha256',SHA256)
    print('# P: elements index(decimal) references[3] corner points (xyz each) -> span_fraction normalized_xyz bits')
    for n,index in cases:
        reference=[rng.uniform(.01,500)*rng.choice([-1,1]) for _ in range(3)]
        points=[[rng.uniform(-1000,1000) for _ in range(3)] for _ in range(n+1)]
        for reg,val in [(UC_X86_REG_RSI,wing),(UC_X86_REG_RDI,obj),(UC_X86_REG_RBP,frame),(UC_X86_REG_RSP,STACK_TOP-0x2000),(UC_X86_REG_R13,index&0xffffffffffffffff),(UC_X86_REG_R15,0)]: e.uc.reg_write(reg,val)
        e.write_u32(wing+4,n)
        for axis,offset in enumerate([0x5bc,0x5e8,0x614]):
            for i,p in enumerate(points):e.write_f32(wing+offset+i*4,p[axis])
            e.write_f32(ref+0x64f4+axis*4,reference[axis])
        e.run_range(0x1411b6c10,0x1411b6d60)
        inputs=reference+[v for p in points for v in p]
        outputs=[e.uc.reg_read(UC_X86_REG_XMM13)&0xffffffff]+[e.read_u32(frame+o) for o in [0x128,0x120,0x158]]
        print('P',n,index,*(f'{bits(v):08x}' for v in inputs),*(f'{v:08x}' for v in outputs))
if __name__=='__main__':main(sys.argv[1])
