#!/usr/bin/env python3
"""Original show/hide/toggle/reparent/dirty on one shared widget graph.
Stable manager, empty listeners, finite ancestry, spare vector capacity; remaining effects controlled.
"""
import hashlib
import itertools
import random
import sys
from pathlib import Path
from emulate_xp import Emulator, RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RAX
PATHS=[[1.],[0.],[-1.],[-1.,-1.],[0.,1.],[1.,0.],[.5,.5],[1e-30,.5]]


def cases():
    for w,p,root,attached in itertools.product([0,1,256,0xffffffff],range(8),range(2),range(2)):
        yield root,w,0,p,0,0,[2,2,-1],[0,1,2],[0,0,2],[attached,0,1]
    rng=random.Random(2026101421)
    for _ in range(600):
        yield rng.randrange(2),rng.choice([0,1,256,0xffffffff]),rng.choice([0,1,256,0xffffffff]),rng.randrange(8),rng.randrange(8),rng.randrange(8),[rng.choice([-2147483648,-1,0,2,2147483647]) for _ in range(3)],rng.sample(range(3),3),[rng.randrange(3) for _ in range(rng.randrange(5))],[rng.randrange(2) for _ in range(3)]


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
    e=Emulator(path);manager=e.alloc(0x180);impl=e.alloc(0x180);vtable=e.alloc(0x190);e.write(vtable,bytes(0x190));e.write_u64(vtable+0x188,RET_ADDR+0x100);e.write_u64(vtable+0x180,RET_ADDR+0x110);e.write(RET_ADDR+0x100,b'\xc3');e.write(RET_ADDR+0x110,b'\xc3');top=e.heap_top
    entries=[];widgets=[];graph=[];log=[];allocs=[0];frees=[0]
    def idx(p):return entries.index(p)
    def alloc(emu):
        assert emu.reg(UC_X86_REG_RCX)==24
        allocs[0]+=1;emu.uc.reg_write(UC_X86_REG_RAX,emu.alloc(24))
    def free(emu):
        assert emu.reg(UC_X86_REG_RDX)==24;frees[0]+=1
    def owner(emu):
        p=emu.reg(UC_X86_REG_RCX);parent=emu.reg(UC_X86_REG_RDX)
        assert parent in [0,impl]
        if p in entries:log.append(f'O:{idx(p)}:{int(parent!=0)}')
        else:log.append(f'B:{graph.index(p)}:{0 if parent==0 else graph.index(parent)+1}')
    def prepare(emu):
        assert emu.reg(UC_X86_REG_RDX)&255==1
        log.append(f'H:{idx(emu.reg(UC_X86_REG_RCX)-0x80)}')
    def active(emu,show):
        assert emu.reg(UC_X86_REG_RDX)&255==1
        log.append(f'B:{idx(emu.reg(UC_X86_REG_RCX))}:{int(show)}')
    def callback(emu,add):
        log.append(f'{"A" if add else "D"}:{graph.index(emu.reg(UC_X86_REG_RCX))}:{graph.index(emu.reg(UC_X86_REG_RDX))}')
    def move(emu):
        dst,src,size=emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX),emu.reg(UC_X86_REG_R8)
        assert size<=128 and size%8==0
        if size:emu.write(dst,emu.read(src,size))
    def refresh(emu):assert emu.reg(UC_X86_REG_RCX)==impl;log.append('R')
    def event(emu):
        assert emu.reg(UC_X86_REG_RCX)==4 and emu.reg(UC_X86_REG_RDX)==1 and emu.reg(UC_X86_REG_R8)==11
        log.append('E:4:1:11')
    e.stubs.update({0x1422e7bf0:alloc,0x1422e7c2c:free,0x1412f6250:owner,0x1412fc4b0:prepare,0x1412e47a0:lambda emu:active(emu,True),0x1412e4680:lambda emu:active(emu,False),0x1412fa3b0:refresh,0x1410c90c0:event,RET_ADDR+0x100:lambda emu:callback(emu,False),RET_ADDR+0x110:lambda emu:callback(emu,True),0x1423c0280:move})
    def linked(owner,offset,values):
        head=e.alloc(24);nodes=[e.alloc(24) for _ in values];chain=[head]+nodes
        for i,p in enumerate(chain):
            e.write_u64(p,chain[(i+1)%len(chain)]);e.write_u64(p+8,chain[(i-1)%len(chain)]);e.write_u64(p+16,0 if i==0 else values[i-1])
        e.write_u64(owner+offset,head);e.write_u64(owner+offset+8,len(values));return head
    def read_list(head):
        out=[];p=e.read_u64(head)
        while p!=head:
            assert len(out)<20 and e.read_u64(e.read_u64(p)+8)==p
            out.append(idx(e.read_u64(p+16)));p=e.read_u64(p)
        return out
    def csv(v):return ','.join(map(str,v)) if v else '-'
    print('# reference_sha256',REFERENCE_SHA256)
    print('# full show/hide/reparent/find/dirty/opacity original on shared graph; bind/activation/callbacks/memmove/alloc/refresh/event controlled; spare capacity; stable manager; empty listeners')
    count=0
    for case_no,(root,w1,w2,p0,p1,p2,classes,order,members,attached) in enumerate(cases()):
        e.heap_top=top;log.clear();allocs[0]=frees[0]=0;entries.clear();widgets.clear();graph.clear();e.write(manager,bytes(0x180));e.write(impl,bytes(0x180))
        e.write_u64(manager+0x128,impl);e.write_u64(impl+0x120,manager);e.write_u64(0x145a0def0,manager if root else 0);e.write_u64(0x14577b938,0)
        ialpha=case_no%4
        def node(p,alpha,parent):
            graph.append(p);e.write_u64(p,vtable);e.write_u32(p+0xf8,(len(graph)-1)%4);e.write_f32(p+0x58,alpha);e.write_u64(p+0x60,parent)
            vec=e.alloc(128);e.write(vec,bytes(128));e.write_u64(p+0x70,vec);e.write_u64(p+0x78,vec);e.write_u64(p+0x80,vec+128)
            if parent:
                end=e.read_u64(parent+0x78);e.write_u64(end,p);e.write_u64(parent+0x78,end+8)
        node(impl,[1.,0.,-1.,.5][ialpha],0)
        for i,pid in enumerate([p0,p1,p2]):
            item=e.alloc(0x178);e.write(item,bytes(0x178));entries.append(item);e.write_u32(item+0x170,classes[i]);e.write_u64(item+0x138,[11,12,11][i]);e.write_u64(item+0x80,item+0x80);e.write_u64(item+0x88,manager if attached[i] else 0)
            parent=0
            for alpha in PATHS[pid]:
                widget=e.alloc(0x100);e.write(widget,bytes(0x100));node(widget,alpha,parent);parent=widget
            e.write_u64(item+0x140,parent);widgets.append(parent)
        h=linked(impl,0x150,[entries[i] for i in order]);m=linked(manager,0x90,[entries[i] for i in members])
        queries=[]
        for w in [w1,w2]:
            e.call(0x1412f97e0,ints=[manager,11,w,123],stack=[-5.],max_instructions=100000)
            queries.append(e.call(0x1412fd160,ints=[manager,11],max_instructions=100000)&255)
        e.call(0x1412fb9a0,ints=[manager,11,0,0],max_instructions=100000)
        queries.append(e.call(0x1412fd160,ints=[manager,11],max_instructions=100000)&255)
        ro,rm=read_list(h),read_list(m);assert len(ro)==e.read_u64(impl+0x158) and len(rm)==e.read_u64(manager+0x98)
        print('C',root,w1,w2,p0,p1,p2,csv(classes),csv(order),csv(members),csv(attached),ialpha,'->',csv(ro),csv(rm),csv([int(e.read_u64(p+0x88)!=0) for p in entries]),csv([e.read_u32(p+0x58) for p in widgets]),csv([-1 if e.read_u64(p+0x60)==0 else graph.index(e.read_u64(p+0x60)) for p in graph]),csv([e.read_u32(p+0xf8) for p in graph]),';'.join(csv([graph.index(e.read_u64(a)) for a in range(e.read_u64(p+0x70),e.read_u64(p+0x78),8)]) for p in graph),csv(queries),*log);count+=1
    print('# contexts',count,'native visibility executions',count*3)


if __name__=='__main__':main(sys.argv[1])
