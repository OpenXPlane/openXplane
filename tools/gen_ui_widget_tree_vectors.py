#!/usr/bin/env python3
"""Original reparent/find/dirty body; binding/callbacks/memmove controlled.
Finite valid node chains, first matching child, spare vector capacity.
"""
import hashlib
import itertools
import random
import sys
from pathlib import Path
from emulate_xp import Emulator, RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8


def cases():
    for old,new,override,effect,childflags in itertools.product([-1,2,3],[-1,2,3],[0,777],range(4),[0,3]):
        yield old,new,override,effect,childflags,[0,0,0,0,childflags,0],0xffffffff,2
    rng=random.Random(2026101422)
    for _ in range(600):
        flags=[rng.choice([0,1,2,3,4,8,13,15,0xffffffff,rng.getrandbits(32)]) for _ in range(6)]
        yield rng.choice([-1,2,3]),rng.choice([-1,2,3]),rng.choice([0,777]),rng.randrange(4),flags[4],flags,rng.getrandbits(32),rng.choice([2,3])


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
    e=Emulator(path);vtable=e.alloc(0x190);e.write(vtable,bytes(0x190));e.write_u64(vtable+0x188,RET_ADDR+0x100);e.write_u64(vtable+0x180,RET_ADDR+0x110)
    e.write(RET_ADDR+0x100,b'\xc3');e.write(RET_ADDR+0x110,b'\xc3');top=e.heap_top
    nodes=[];log=[];values=[]
    def idx(p):return -1 if p==0 else nodes.index(p)
    def bind(emu):
        assert idx(emu.reg(UC_X86_REG_RCX))==4
        target=emu.reg(UC_X86_REG_RDX);log.append(f'B:{777 if target==777 else idx(target)+1}')
        if values[3]==3:emu.write_u64(nodes[4]+0x60,nodes[values[7]])
    def callback(emu,add):
        parent=idx(emu.reg(UC_X86_REG_RCX));assert idx(emu.reg(UC_X86_REG_RDX))==4
        log.append(f'{"A" if add else "D"}:{parent}:4')
        if (values[3]==1 and not add) or (values[3]==2 and add):
            emu.write_u64(nodes[4]+0x60,nodes[values[7]])
        if values[3]==2 and add:emu.write_u32(nodes[4]+0xf8,values[6])
    def move(emu):
        dst,src,size=emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX),emu.reg(UC_X86_REG_R8)
        assert size<=128 and size%8==0
        if size:emu.write(dst,emu.read(src,size))
    e.stubs.update({0x1412f6250:bind,RET_ADDR+0x100:lambda emu:callback(emu,False),RET_ADDR+0x110:lambda emu:callback(emu,True),0x1423c0280:move})
    def csv(v):return ','.join(map(str,v)) if v else '-'
    print('# reference_sha256',REFERENCE_SHA256)
    print('# reparent/find/dirty original; bind/virtual callbacks/memmove controlled; spare capacity; stable valid nodes; finite parent chains')
    count=0
    for old,new,override,effect,childflags,flags,mutflags,redirect in cases():
        values[:]=[old,new,override,effect,childflags,flags,mutflags,redirect];e.heap_top=top;log.clear();nodes[:]=[e.alloc(0x100) for _ in range(6)]
        parents=[-1,-1,0,1,old,-1];children=[[2],[3],[5,4,4,5],[4,5,4],[],[]]
        for i,p in enumerate(nodes):
            e.write(p,bytes(0x100));e.write_u64(p,vtable);e.write_u64(p+0x60,0 if parents[i]<0 else nodes[parents[i]]);e.write_u32(p+0xf8,flags[i]);e.write_u64(p+0x68,override if i==4 else 0)
            vec=e.alloc(128);e.write(vec,bytes(128))
            for k,j in enumerate(children[i]):e.write_u64(vec+8*k,nodes[j])
            e.write_u64(p+0x70,vec);e.write_u64(p+0x78,vec+8*len(children[i]));e.write_u64(p+0x80,vec+128)
        e.call(0x1412f77a0,ints=[nodes[4],0 if new<0 else nodes[new]],max_instructions=100000)
        outchildren=[]
        for p in nodes:
            begin,end=e.read_u64(p+0x70),e.read_u64(p+0x78);assert 0<=end-begin<=128
            outchildren.append([idx(e.read_u64(a)) for a in range(begin,end,8)])
        print('C',old,new,override,effect,csv(flags),mutflags,redirect,'->',csv([idx(e.read_u64(p+0x60)) for p in nodes]),csv([e.read_u32(p+0xf8) for p in nodes]),';'.join(csv(v) for v in outchildren),*log);count+=1
        before=[e.read_u32(p+0xf8) for p in nodes];parent_ids=[idx(e.read_u64(p+0x60)) for p in nodes]
        e.call(0x1412e48f0,ints=[nodes[4],mutflags],max_instructions=100000)
        print('D',csv(parent_ids),csv(before),mutflags,'->',csv([e.read_u32(p+0xf8) for p in nodes]))
    print('# reparent cases',count,'standalone dirty cases',count)


if __name__=='__main__':main(sys.argv[1])
