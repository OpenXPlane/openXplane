#!/usr/bin/env python3
"""Complete concrete fallback and its actual property gate; wrapper ctor/dtor unchanged."""
import hashlib
import itertools
import random
import sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9


def cases():
    for engine,mode,first,second,cache in itertools.product([0,1,255],[0,2,3],[0,1,255],[0,1,255],[0,1,255]):
        yield engine,mode,first,second,cache,-1,42
    yield 0,0,0,1,0,255,42
    yield 0,0,1,0,0,255,42
    yield 0,0,1,0,1,255,42
    rng=random.Random(2026101409)
    for _ in range(500):yield rng.randrange(256),rng.choice([-2147483648,-1,0,1,2,3,2147483647]),rng.randrange(256),rng.randrange(256),rng.randrange(256),rng.choice([-1,0,1,255]),rng.getrandbits(64)


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
    e=Emulator(path);panel=e.alloc(0x300);e.write(panel,bytes(0x300));e.write_u64(panel,0x1426064f0);e.write_u64(panel+0x28,0x1426066b0);e.write_u64(panel+0x170,0x1426067a8)
    object_vt=e.alloc(8);e.write_u64(object_vt,RET_ADDR+0x100);e.write(RET_ADDR+0x100,b'\xc3');top=e.heap_top
    log,values,callbacks,query_index,allocated=[],[],[],[0],[0]
    def pending(emu):
        i=query_index[0];query_index[0]+=1;assert i<2
        log.append(f'P:{i}');emu.uc.reg_write(UC_X86_REG_RAX,values[2+i])
        if i==0 and values[5]>=0:emu.write(0x142f2d770+0x525,bytes([values[5]]))
    def alloc(emu):
        size=emu.reg(UC_X86_REG_RCX);assert size in [0x320,0x28]
        p=emu.alloc(size);emu.write(p,bytes(size))
        if size==0x320:allocated[0]=p;log.append('A:800')
        emu.uc.reg_write(UC_X86_REG_RAX,p)
    def construct(emu):
        assert emu.reg(UC_X86_REG_RCX)==allocated[0] and emu.reg(UC_X86_REG_RDX)==0
        assert [emu.reg(UC_X86_REG_R8),emu.reg(UC_X86_REG_R9)]==callbacks[::-1]
        for cb in callbacks:assert [emu.read_u64(cb+offset) for offset in [0x10,0x18,0x20,0x60]]==[0,0,0,0]
        p=allocated[0];emu.write_u64(p,object_vt);emu.write_u64(p+0x2d0,17);log.append('C:1:0');emu.uc.reg_write(UC_X86_REG_RAX,p)
    def observe_callback(uc,a,s,u):
        cb=e.reg(UC_X86_REG_RCX);assert [e.reg(reg) for reg in [UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9]]==[0,0,0]
        callbacks.append(cb);log.append(f'H:{len(callbacks)-1}')
    def observe_drop(uc,a,s,u):log.append(f'D:{callbacks.index(e.reg(UC_X86_REG_RCX))}')
    def resolve(emu):
        assert [emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX)]==[18,19];log.append('R:18:19');emu.uc.reg_write(UC_X86_REG_RAX,values[6])
    def destroy(emu):
        assert emu.reg(UC_X86_REG_RCX)==allocated[0] and emu.reg(UC_X86_REG_RDX)==1;log.append('X:1')
    def submit(emu):
        assert emu.reg(UC_X86_REG_RCX)==panel+0x170
        word=emu.read_u32(emu.reg(UC_X86_REG_RDX));assert word==0
        log.append(f'S:{word}:{emu.reg(UC_X86_REG_R8)}')
    def registry(emu):
        assert emu.reg(UC_X86_REG_RCX)==17;log.append('I:17');emu.uc.reg_write(UC_X86_REG_RAX,19)
    def cache(emu):assert emu.reg(UC_X86_REG_RCX)==0x1460fc940;log.append('Q');emu.uc.reg_write(UC_X86_REG_RAX,values[4])
    def direct(emu):assert emu.reg(UC_X86_REG_RCX)==panel and emu.reg(UC_X86_REG_RDX)==0;log.append('V:0')
    e.stubs.update({0x140f0a1b0:pending,0x1417e2640:cache,0x1422e7bf0:alloc,0x1422e7c2c:lambda emu:None,0x140ec9c10:construct,0x1417e9f50:lambda emu:(log.append('M'),emu.uc.reg_write(UC_X86_REG_RAX,18)) and None,0x140d569e0:registry,0x1417e8a60:resolve,RET_ADDR+0x100:destroy,0x140ef67c0:submit,0x140f0a2c0:direct})
    e.uc.hook_add(UC_HOOK_CODE,observe_callback,begin=0x1410c7d20,end=0x1410c7d20);e.uc.hook_add(UC_HOOK_CODE,observe_drop,begin=0x1410c8430,end=0x1410c8430)
    print('# reference_sha256',REFERENCE_SHA256)
    print('# full fallback/property2/engine byte/UI enum/virtual direct thunk/empty wrapper ctor+dtor unchanged; pending/cache/actual construction+resolution+submission/allocator service bodies controlled')
    e.write_u64(0x145a0dfc8,panel);e.write_u32(panel+0x18,0xfff)
    print('# F engine_byte ui_mode pending0 pending1 cache update_engine result -> placeholder final_engine trace* | key_return final_engine trace* | focused_return final_engine trace*')
    count=0
    for case in cases():
        values[:]=case;outputs=[]
        for entry,args in [(0x140f00310,[panel]),(0x1412e1de0,[panel,27,27,8]),(0x1412c6300,[27,27,8])]:
            e.heap_top=top;log.clear();callbacks.clear();query_index[0]=0;allocated[0]=0
            e.write(0x142f2d770+0x525,bytes([case[0]]));e.write_u32(0x145a12118+0x18,case[1])
            raw=e.call(entry,ints=args,max_instructions=100000)
            result=0 if entry==0x140f00310 else raw&255
            outputs.append(' '.join(map(str,[result,e.read(0x142f2d770+0x525,1)[0],*log])))
        print('F',*case,'->',' | '.join(outputs));count+=1
    print('# cases',count)


if __name__=='__main__':main(sys.argv[1])
