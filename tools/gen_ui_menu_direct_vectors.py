#!/usr/bin/env python3
"""Full direct menu action with native guard, branch selection and wrapper lifecycle.
Concrete close/document/binding/dialog/effect bodies remain controlled endpoints.
"""
import hashlib
import itertools
import random
import struct
import sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9


def cases():
    for arg,guard,c8,mode,query,load in itertools.product([-1,0,1,12],[0,255],[0,255],[0,1,2],[0,255],[0,255]):
        yield arg,guard,c8,mode,query,2,load,-1,-1,7,0,0,1
    # Direct evidence for re-entry suppression and post-close mode selection.
    yield 0,0,0,1,0,2,0,255,2,7,255,1,1
    yield 0,0,255,2,1,255,0,0,1,7,0,1,3
    rng=random.Random(2026101410)
    for _ in range(500):
        yield rng.choice([-2147483648,-1,0,1,12,2147483647]),rng.choice([0,0,1,255]),rng.randrange(256),rng.choice([-1,0,1,2]),rng.randrange(256),rng.randrange(256),rng.randrange(256),rng.choice([-1,0,1,255]),rng.choice([-1,0,1,2]),rng.getrandbits(64),rng.randrange(256),rng.randrange(2),rng.randrange(4)


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
    e=Emulator(path);panel=e.alloc(0x300);e.write(panel,bytes(0x300));vt=e.alloc(0x180);e.write(vt,bytes(0x180));e.write_u64(panel,vt);e.write_u64(vt+0x118,RET_ADDR+0x100)
    controller=e.alloc(16);object_vt=e.alloc(8);e.write_u64(object_vt,RET_ADDR+0x300)
    for address in [RET_ADDR+0x200,RET_ADDR+0x300]:e.write(address,b'\xc3')
    top=e.heap_top;values,log,callbacks,allocated=[],[],[],[0]
    documents=[b'',b'a',b'test.sit',b'x'*15]
    def guard():return e.read(panel+0x2c9,1)[0]
    def prepare(emu):
        log.append(f'P:{guard()}')
        if values[7]>=0:emu.write(panel+0x2c8,bytes([values[7]]))
    def close(emu):
        assert emu.reg(UC_X86_REG_RCX)==controller and emu.reg(UC_X86_REG_RDX)==11
        log.append(f'C:11:{emu.reg(UC_X86_REG_R8)&255}:{guard()}')
        if values[8]>=0:emu.write_u32(0x145a12118+0x18,values[8])
    def doc(emu):
        p=emu.reg(UC_X86_REG_RCX);data=documents[values[12]]
        emu.write(p,bytes(32));emu.write(p,data);emu.write_u64(p+16,len(data));emu.write_u64(p+24,15)
        log.append('F');emu.uc.reg_write(UC_X86_REG_RAX,p)
    def validate(emu):
        p=emu.reg(UC_X86_REG_RCX);assert emu.read(emu.read_u64(p),emu.read_u64(p+8))==documents[values[12]]
        log.append(f'V:{documents[values[12]].hex() or "-"}');emu.uc.reg_write(UC_X86_REG_RAX,values[5])
    def load(emu):
        assert emu.reg(UC_X86_REG_RCX)==0x142f2d770
        p=emu.reg(UC_X86_REG_RDX);assert emu.read(p,emu.read_u64(p+16))==documents[values[12]]
        log.append(f'L:{emu.reg(UC_X86_REG_R8)&255}');emu.uc.reg_write(UC_X86_REG_RAX,values[6])
    def allocate(emu):
        size=emu.reg(UC_X86_REG_RCX);assert size in [0x28,0x320]
        p=emu.alloc(size);emu.write(p,bytes(size));emu.uc.reg_write(UC_X86_REG_RAX,p)
        if size==0x320:allocated[0]=p;log.append('A:800')
    def bind(emu):
        assert emu.reg(UC_X86_REG_RDX)==panel+0x48
        p=emu.reg(UC_X86_REG_RCX);assert p in callbacks;log.append(f'B:{emu.read_u64(p+0x18)}')
    def construct(emu):
        assert emu.reg(UC_X86_REG_RCX)==allocated[0] and emu.reg(UC_X86_REG_RDX)==values[9]
        assert [emu.reg(UC_X86_REG_R8),emu.reg(UC_X86_REG_R9)]==callbacks[::-1]
        assert [emu.read_u64(cb+0x18) for cb in callbacks]==[0,3]
        p=allocated[0];emu.write_u64(p,object_vt);emu.write_u64(p+0x2c8,17);emu.write_u64(p+0x2d8,18)
        log.append(f'T:{values[9]}:3:0');emu.uc.reg_write(UC_X86_REG_RAX,p)
    def cb(uc,a,s,u):
        assert e.reg(UC_X86_REG_RDX)==panel+0x48 and e.reg(UC_X86_REG_R9)==0
        callbacks.append(e.reg(UC_X86_REG_RCX));log.append(f'H:{e.reg(UC_X86_REG_R8)}')
    def drop(uc,a,s,u):
        i=callbacks.index(e.reg(UC_X86_REG_RCX));log.append(f'D:{[0,3][i]}')
    def field(emu):
        assert emu.reg(UC_X86_REG_RCX)==17 and emu.read(0x1457770c6,1)[0]==1
        log.append('E:c8:1')
    def component(emu):assert emu.reg(UC_X86_REG_RCX)==allocated[0]+0x260;log.append('E:260')
    def field2(emu):assert emu.reg(UC_X86_REG_RCX)==18;log.append('E:d8')
    def destroy(emu):assert emu.reg(UC_X86_REG_RCX)==allocated[0] and emu.reg(UC_X86_REG_RDX)==1;log.append('X:1')
    def manager(emu):log.append('M');emu.uc.reg_write(UC_X86_REG_RAX,controller)
    def query(emu):log.append('Q');emu.uc.reg_write(UC_X86_REG_RAX,values[4])
    def notify(emu):assert emu.reg(UC_X86_REG_RCX)==0x142609340;log.append('N')
    e.stubs.update({RET_ADDR+0x200:prepare,0x14150d020:manager,0x14150ce20:close,0x140658af0:notify,0x140f0a1b0:query,0x140efe0d0:doc,0x140efdda0:validate,0x1417fa7a0:load,0x140f0a210:lambda emu:log.append('FAIL'),0x1422e7bf0:allocate,0x1422e7c2c:lambda emu:None,0x1410c8de0:bind,0x140ec9c10:construct,0x1411103e0:field,0x1410c9d10:component,0x14111bb90:field2,RET_ADDR+0x300:destroy})
    e.uc.hook_add(UC_HOOK_CODE,cb,begin=0x1410c7d20,end=0x1410c7d20);e.uc.hook_add(UC_HOOK_CODE,drop,begin=0x1410c8430,end=0x1410c8430)
    e.uc.hook_add(UC_HOOK_CODE,lambda uc,a,s,u:log.append('DROP'),begin=0x140f0a404,end=0x140f0a404)
    print('# reference_sha256',REFERENCE_SHA256)
    print('# full direct action/enum predicate/wrapper ctor+dtor unchanged; binding/controller-close/document/dialog/effect/allocator endpoints controlled; optional recursive prep calls native action; inline strings only')
    print('# D arg guard c8 mode query validation loaded update_c8 update_mode source aux recursive doc_index -> guard c8 mode aux trace*')
    count=0
    for case in cases():
        values[:]=case;e.heap_top=top;callbacks.clear();log.clear();allocated[0]=0
        arg,g,c8,mode,_,_,_,_,_,source,aux,recur,_=case
        e.write(panel+0x2c9,bytes([g]));e.write(panel+0x2c8,bytes([c8]));e.write_u64(panel+0x1a8,source);e.write(0x1457770c6,bytes([aux]));e.write_u32(0x145a12118+0x18,mode)
        thunk=b'\x48\x83\xec\x28\x48\xb8'+struct.pack('<Q',RET_ADDR+0x200)+b'\xff\xd0'
        if recur:thunk+=b'\x48\xb9'+struct.pack('<Q',panel)+b'\xba\xfb\xff\xff\xff\x48\xb8'+struct.pack('<Q',0x140f0a2c0)+b'\xff\xd0'
        thunk+=b'\x48\x83\xc4\x28\xc3';e.write(RET_ADDR+0x100,thunk);e.uc.ctl_remove_cache(RET_ADDR+0x100,RET_ADDR+0x180)
        e.call(0x140f0a2c0,ints=[panel,arg],max_instructions=100000)
        print('D',*case,'->',guard(),e.read(panel+0x2c8,1)[0],e.read_u32(0x145a12118+0x18),e.read(0x1457770c6,1)[0],*log);count+=1
    print('# cases',count)


if __name__=='__main__':main(sys.argv[1])
