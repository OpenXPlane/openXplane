#!/usr/bin/env python3
"""Replay original inclusive wing-boundary point emission with preallocated vector storage."""
import hashlib
import random
import struct
import sys
from pathlib import Path
from emulate_xp import Emulator, STACK_TOP
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RSI, UC_X86_REG_R12, UC_X86_REG_RBX, UC_X86_REG_R15, UC_X86_REG_RBP, UC_X86_REG_RSP, UC_X86_REG_XMM9, UC_X86_REG_XMM10
SHA256 = '2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
BEGIN, END = 0x141a88000, 0x141a88318

def cases():
    for n in [1,4,10]:
        for a,b in [(0.,0.),(1.,1.),(3.,1.),(1.,0.)]:
            yield n, [[a+(b-a)*i/n for i in range(n+1)], [i*.7 for i in range(n+1)], [-0. for i in range(n+1)], [-i*.2 for i in range(n+1)]]
    rng = random.Random(20261014)
    for _ in range(500):
        n = rng.randrange(1,11)
        yield n, [[rng.uniform(0.,10.) for _ in range(n+1)], *[[rng.uniform(-100.,100.) for _ in range(n+1)] for _ in range(3)]]

def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=SHA256: raise ValueError('reference SHA256 mismatch')
    emu = Emulator(path)
    wing, vector, points, frame = emu.alloc(0x4000), emu.alloc(24), emu.alloc(1024), emu.alloc(0x1000)
    print('# reference_sha256',SHA256)
    print(f'# original_range {BEGIN:#x} {END:#x} exclusive; no stubs; preallocated vector avoids allocator branch')
    print('# P elements chord[] x[] y[] z[] -> 2*(elements+1) emitted xyz triples (all f32bits)')
    count=0
    for n,arrays in cases():
        emu.write_u32(wing+4,n)
        for offset,values in zip([0x70,0x5bc,0x5e8,0x614],arrays):
            emu.write(wing+offset,struct.pack('<'+'f'*(n+1),*values))
        emu.write_u64(vector,points); emu.write_u64(vector+8,points); emu.write_u64(vector+16,points+1024)
        for reg,value in [(UC_X86_REG_RAX,n),(UC_X86_REG_RSI,wing),(UC_X86_REG_R12,wing+0x70),(UC_X86_REG_RBX,vector),(UC_X86_REG_R15,0),(UC_X86_REG_RBP,frame),(UC_X86_REG_RSP,STACK_TOP-0x1000)]:
            emu.uc.reg_write(reg,value)
        for reg,value in [(UC_X86_REG_XMM9,.75),(UC_X86_REG_XMM10,.25)]:
            emu.uc.reg_write(reg,struct.unpack('<Q',struct.pack('<d',value))[0])
        emu.run_range(BEGIN,END)
        assert not emu.stubs
        assert emu.read_u64(vector+8)==points+(n+1)*24
        raw=struct.pack('<'+'f'*(4*(n+1)),*[v for a in arrays for v in a])
        words=struct.unpack('<'+'I'*(4*(n+1)),raw)
        output=struct.unpack('<'+'I'*(6*(n+1)),emu.read(points,24*(n+1)))
        print('P',n,*(f'{v:08x}' for v in [*words,*output]))
        count+=1
    print('# cases',count)

if __name__=='__main__': main(sys.argv[1])
