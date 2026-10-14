#!/usr/bin/env python3
"""Full close helper with original query/getter/mode/toggle/accessor/map store.
Registered-only profile; window/update/environment effects controlled.
"""
import hashlib
import itertools
import random
import sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9,UC_X86_REG_RAX,UC_X86_REG_RSP
PATHS=[[1.],[0.],[-1.],[-1.,-1.],[1.,0.],[.5,.5],[-0.],[1e-30,.5]]


def cases():
    for mode,path,delete,flag,env in itertools.product([0,4,6],[0,1,3],[0,1],[0,255],[0,255]):
        yield 11,1,path,mode,delete,flag,1,255,env,-1,-1,-1,-1,0,0
    rng=random.Random(2026101411)
    for _ in range(500):
        yield rng.choice([-2147483648,-10,11,12,2147483647]),rng.randrange(2),rng.randrange(len(PATHS)),rng.choice([-1,0,4,6]),rng.randrange(2),rng.randrange(256),rng.choice([-1,0,1]),rng.randrange(256),rng.randrange(256),rng.choice([-1,0,4,6]),rng.choice([-1,0,1]),rng.choice([-1,0,255]),rng.choice([-1,0,1]),rng.randrange(2),rng.randrange(2)
    yield 11,1,0,4,1,255,1,255,0,6,-1,-1,-1,0,0
    yield 11,1,0,0,0,255,1,255,0,-1,1,0,0,0,1


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
    e=Emulator(path);owner,manager,impl=e.alloc(0x2b0),e.alloc(0x180),e.alloc(0x180)
    e.write(owner,bytes(0x2b0));e.write_u64(0x145a0e308,manager);e.write_u64(manager+0x128,impl);e.write_u64(0x14577b930,0)
    top=e.heap_top;values,log,handles,items,leaves,node,updates=[],[],[],[],[],[0],[0]
    def token(p):return 0 if p==0 else handles.index(p)+1
    def update(emu):
        assert emu.reg(UC_X86_REG_RCX)==owner and emu.reg(UC_X86_REG_RDX)&0xffffffff==values[0]&0xffffffff
        log.append(f'U:{values[0]}:{emu.reg(UC_X86_REG_R8)&255}:{token(emu.read_u64(node[0]+40))}')
        if updates[0]==0 and values[13]:emu.write_u64(node[0]+40,handles[1])
        updates[0]+=1
    def reset(emu):
        t=token(emu.reg(UC_X86_REG_RDX));assert emu.reg(UC_X86_REG_R8)==0 and emu.reg(UC_X86_REG_R9)&0xffffffff==0xffffffff
        log.append(f'R:{t}:0:-1')
        if values[9]>=0:emu.write_u32(items[t-1]+0x1ac,values[9])
    def visibility(emu):
        t=token(emu.reg(UC_X86_REG_RDX));assert emu.reg(UC_X86_REG_R9)==0 and emu.read_u32(emu.reg(UC_X86_REG_RSP)+40)==0
        log.append(f'V:{t}:{emu.reg(UC_X86_REG_R8)&255}:0:0')
        if values[10]>=0:emu.write_f32(leaves[t-1],float(values[10]))
    def delete(emu):
        t=token(emu.reg(UC_X86_REG_RDX));assert emu.reg(UC_X86_REG_R8)&255==1;log.append(f'D:{t}:1')
    def environment(emu):
        log.append('Q');emu.uc.reg_write(UC_X86_REG_RAX,values[8])
        if values[11]>=0:emu.write(owner+0x2a8,bytes([values[11]]))
        if values[12]>=0:emu.write_u32(0x145a11f48,values[12])
    def final(emu):
        assert emu.reg(UC_X86_REG_RCX)==owner
        log.append(f'F:{values[0]}:{token(emu.read_u64(node[0]+40))}')
    def command(emu):assert emu.reg(UC_X86_REG_RCX)==0x14577734c;log.append('C')
    e.stubs.update({0x14150e420:update,0x1412f9480:reset,0x1412f97e0:visibility,0x1412c6070:delete,0x14150cf80:environment,0x140c7f790:command,0x1414d7470:lambda emu:log.append('P'),0x14150fee0:final})
    print('# reference_sha256',REFERENCE_SHA256)
    print('# full close/query/registered getter/mode4+6/toggle/accessor/map-store unchanged; window/update/reset/show/delete/environment/command/final endpoints controlled; protected+observer lists empty; finite paths')
    print('# C kind mapped path mode delete flag global controllerbyte environment reset_mode alpha_update controllerbyte_update global_update switch_token token2_alpha -> map_token modes[2] leaf_bits[2] global controllerbyte query trace*')
    count=0
    for case in cases():
        values[:]=case;kind,mapped,path,mode,delete_flag,flag,global_flag,controller_byte,_,_,_,_,_,_,alpha2=case
        e.heap_top=top;log.clear();updates[0]=0;handles[:]=[e.alloc(16),e.alloc(16)]
        sentinel=e.alloc(48);n=e.alloc(48);node[0]=n
        e.write(sentinel,bytes(48));e.write(n,bytes(48));e.write(sentinel+0x19,b'\1')
        for offset in [0,8,16]:e.write_u64(sentinel+offset,n)
        e.write_u64(n,sentinel);e.write_u64(n+8,sentinel);e.write_u64(n+16,sentinel);e.write_u32(n+32,kind);e.write_u64(n+40,handles[0] if mapped else 0)
        e.write_u64(owner+0x148,sentinel);e.write_u64(owner+0x150,1)
        vec=e.alloc(4);e.write_u32(vec,kind);e.write_u64(owner+0x90,vec);e.write_u64(owner+0x98,vec+4 if delete_flag else vec)
        head=e.alloc(24);links=[e.alloc(24),e.alloc(24)];e.write_u64(head,links[0]);e.write_u64(impl+0x150,head)
        items.clear();leaves.clear()
        for i,opacity in enumerate([PATHS[path],[float(alpha2)]]):
            item=e.alloc(0x1b0);e.write(item,bytes(0x1b0));items.append(item)
            e.write_u64(links[i],links[i+1] if i==0 else head);e.write_u64(links[i]+16,item);e.write_u64(item+0x138,handles[i]);e.write_u32(item+0x1ac,mode if i==0 else 6)
            parent=0
            for alpha in opacity:
                widget=e.alloc(0x70);e.write(widget,bytes(0x70));e.write_u64(widget+0x60,parent);e.write_f32(widget+0x58,alpha);parent=widget
            e.write_u64(item+0x140,parent);leaves.append(parent+0x58)
        e.write_u32(0x145a11f48,global_flag);e.write(owner+0x2a8,bytes([controller_byte]))
        e.call(0x14150ce20,ints=[owner,kind,flag],max_instructions=100000)
        query=e.call(0x141512eb0,ints=[owner,kind],max_instructions=100000)&255
        print('C',*case,'->',token(e.read_u64(n+40)),*(e.read_u32(p+0x1ac) for p in items),*(e.read_u32(p) for p in leaves),e.read_u32(0x145a11f48),e.read(owner+0x2a8,1)[0],query,*log);count+=1
    print('# cases',count)


if __name__=='__main__':main(sys.argv[1])
