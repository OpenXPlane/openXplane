#!/usr/bin/env python3
"""Original wing span weights, including original CRT byte comparisons; no stubs."""
import hashlib
import random
import struct
import sys
from pathlib import Path
from emulate_xp import Emulator, STACK_TOP
from gen_wing_vectors import SHA256, bits, set_float
from unicorn.x86_const import (UC_X86_REG_RSI, UC_X86_REG_RBP, UC_X86_REG_RSP,
    UC_X86_REG_RAX, UC_X86_REG_XMM0, UC_X86_REG_XMM6, UC_X86_REG_XMM7,
    UC_X86_REG_XMM8, UC_X86_REG_XMM10, UC_X86_REG_XMM11,
    UC_X86_REG_XMM12, UC_X86_REG_XMM13)


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != SHA256:
        raise ValueError('Reference SHA256 mismatch')
    e=Emulator(path); wing=e.alloc(0x4000); frame=e.alloc(1024)+128; ref=e.alloc(0x6600)
    buffers=[e.alloc(1024) for _ in range(3)]
    e.write_f32(ref+0x64fc,1.0)
    def write_string(slot,text,pool):
        raw=text.encode('utf-8');e.write(slot,b'\0'*32)
        if len(raw)<=15:
            e.write(slot,raw);capacity=15
        else:
            e.write(pool,raw+b'\0'*32);e.write_u64(slot,pool);capacity=len(raw)
        e.write_u64(slot+0x10,len(raw));e.write_u64(slot+0x18,capacity)
    names=[('r','m','t'),('same','same','same'),('r','r','t'),('r','t','t'),('a','b','a'),('','',''),
           ('a'*15,'a'*15,'b'*15),('a'*16,'a'*16,'b'*16),('a'*64,'b'*64,'b'*64),
           ('a\0b','a\0c','a\0b'),('корень','середина','край')]
    rng=random.Random(20261009)
    ratios=[(0.0,.2,.8,1.0),(.2,.2,.8,.8),(.8,.2,.3,.1),(.1,.9,.1,.9)]
    cases=[(t,r,n) for t in [-.1,-0.0,0.0,.1,.2,.5,.8,.9,1.0,1.1] for r in ratios for n in names]
    cases += [(rng.uniform(-.5,1.5),tuple(rng.uniform(-.5,1.5) for _ in range(4)),names[i%len(names)]) for i in range(2000)]
    print('# reference_sha256',SHA256)
    print('# W: t ratios[4] root_equals_middle middle_equals_tip -> root middle tip (bits)')
    for t,ratios,n in cases:
        for reg,val in [(UC_X86_REG_RSI,wing),(UC_X86_REG_RBP,frame),(UC_X86_REG_RSP,STACK_TOP-0x2000),(UC_X86_REG_RAX,ref)]:e.uc.reg_write(reg,val)
        set_float(e,UC_X86_REG_XMM0,0.0);set_float(e,UC_X86_REG_XMM10,0.0);set_float(e,UC_X86_REG_XMM11,1.0);set_float(e,UC_X86_REG_XMM13,t)
        e.uc.reg_write(UC_X86_REG_XMM12,struct.unpack('<Q',struct.pack('<d',1.0))[0])
        for i,r in enumerate(ratios):e.write_f32(wing+0x5c+i*4,r)
        for i,name in enumerate(n):write_string(wing+0x3618+i*32,name,buffers[i])
        e.run_range(0x1411b6d3b,0x1411b6e8f)
        values=[bits(t)]+[bits(r) for r in ratios]+[int(n[0]==n[1]),int(n[1]==n[2])]
        values += [e.uc.reg_read(reg)&0xffffffff for reg in [UC_X86_REG_XMM8,UC_X86_REG_XMM6,UC_X86_REG_XMM7]]
        print('W',*(f'{v:08x}' for v in values))
if __name__=='__main__':main(sys.argv[1])
