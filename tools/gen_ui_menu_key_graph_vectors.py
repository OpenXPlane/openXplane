#!/usr/bin/env python3
"""Original direct/close/query/getter/toggle/show/hide/reparent/dirty on one graph.
Stable manager, empty listeners, finite ancestry, spare vector capacity; remaining effects controlled.
"""
import hashlib
import itertools
import random
import sys
from pathlib import Path
from emulate_xp import Emulator, RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9,UC_X86_REG_RAX
PATHS=[[1.],[0.],[-1.],[-1.,-1.],[0.,1.],[1.,0.],[.5,.5],[1e-30,.5]]


def cases():
    for w,p,root,attached in itertools.product([0,1,256,0xffffffff],range(8),range(2),range(2)):
        yield root,w,0,p,0,0,[2,2,-1],[0,1,2],[0,0,2],[attached,0,1]
    rng=random.Random(2026101421)
    for _ in range(600):
        yield rng.randrange(2),rng.choice([0,1,256,0xffffffff]),rng.choice([0,1,256,0xffffffff]),rng.randrange(8),rng.randrange(8),rng.randrange(8),[rng.choice([-2147483648,-1,0,2,2147483647]) for _ in range(3)],rng.sample(range(3),3),[rng.randrange(3) for _ in range(rng.randrange(5))],[rng.randrange(2) for _ in range(3)]


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
    e=Emulator(path);panel=e.alloc(0x300);panel_vt=e.alloc(0x180);e.write(panel_vt,e.read(0x1426064f0,0x180));e.write_u64(panel_vt+0x118,RET_ADDR+0x200);e.write(RET_ADDR+0x200,b'\xc3');controller=e.alloc(0x2b0);manager=e.alloc(0x180);impl=e.alloc(0x180);vtable=e.alloc(0x190);e.write(vtable,bytes(0x190));e.write_u64(vtable+0x188,RET_ADDR+0x100);e.write_u64(vtable+0x180,RET_ADDR+0x110);e.write(RET_ADDR+0x100,b'\xc3');e.write(RET_ADDR+0x110,b'\xc3');object_vt=e.alloc(8);e.write_u64(object_vt,RET_ADDR+0x300);e.write(RET_ADDR+0x300,b'\xc3');top=e.heap_top
    values=[];documents=[b'',b'a',b'test.sit',b'x'*15];callbacks=[];allocated=[0];config=[];mapnode=[0];updates=[0];vis=[0];closes=[0];directs=[0];phase=[0];fq=[0];fallback=[];kind=[0];entries=[];widgets=[];graph=[];log=[];allocs=[0];frees=[0]
    def idx(p):return entries.index(p)
    def observe_direct(uc,address,size,user):directs[0]+=1;phase[0]=1
    e.uc.hook_add(UC_HOOK_CODE,observe_direct,begin=0x140f0a2c0,end=0x140f0a2c0)
    def prepare_panel(emu):
        assert emu.reg(UC_X86_REG_RCX)==panel
        log.append(f'P:{emu.read(panel+0x2c9,1)[0]}')
    def get_controller(emu):log.append('M');emu.uc.reg_write(UC_X86_REG_RAX,controller)
    def observe_close(uc,address,size,user):closes[0]+=1
    e.uc.hook_add(UC_HOOK_CODE,observe_close,begin=0x14150ce20,end=0x14150ce20)
    def update(emu):
        assert emu.reg(UC_X86_REG_RCX)==controller and emu.reg(UC_X86_REG_RDX)==11
        log.append(f'U:{emu.reg(UC_X86_REG_R8)&255}:{emu.read_u64(mapnode[0]+40)}')
        if updates[0]==0 and config[1]:emu.write_u64(mapnode[0]+40,12)
        updates[0]+=1
    def reset(emu):
        t=emu.reg(UC_X86_REG_RDX);assert emu.reg(UC_X86_REG_R8)==0
        assert emu.reg(UC_X86_REG_R9)&0xffffffff==0xffffffff
        order_entries=read_list(e.read_u64(impl+0x150));i=next(i for i in order_entries if [11,12,11][i]==t)
        log.append(f'S:{t}:0:-1')
        if config[6]>=0:emu.write_u32(entries[i]+0x1ac,config[6])
        if config[7]>=0:emu.write_f32(widgets[i]+0x58,float(config[7]))
    def delete(emu):
        assert emu.reg(UC_X86_REG_R8)&255==1;log.append(f'X:{emu.reg(UC_X86_REG_RDX)}:1')
    def environment(emu):log.append('Q');emu.uc.reg_write(UC_X86_REG_RAX,config[5])
    def final(emu):
        assert emu.reg(UC_X86_REG_RCX)==controller and emu.reg(UC_X86_REG_RDX)==11
        log.append(f'F:{emu.read_u64(mapnode[0]+40)}')
    def observe(uc,address,size,user):vis[0]+=1
    e.uc.hook_add(UC_HOOK_CODE,observe,begin=0x1412f97e0,end=0x1412f97e0)
    def alloc(emu):
        assert emu.reg(UC_X86_REG_RCX)==24
        allocs[0]+=1;emu.uc.reg_write(UC_X86_REG_RAX,emu.alloc(24))
    def free(emu):
        assert emu.reg(UC_X86_REG_RDX) in [24,40];frees[0]+=1
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
        assert emu.reg(UC_X86_REG_RCX)==4 and emu.reg(UC_X86_REG_RDX)==1 and emu.reg(UC_X86_REG_R8) in [11,12]
        log.append(f'E:4:1:{emu.reg(UC_X86_REG_R8)}')
    e.stubs.update({0x1422e7bf0:alloc,0x1422e7c2c:free,0x1412f6250:owner,0x1412fc4b0:prepare,0x1412e47a0:lambda emu:active(emu,True),0x1412e4680:lambda emu:active(emu,False),0x1412fa3b0:refresh,0x1410c90c0:event,RET_ADDR+0x100:lambda emu:callback(emu,False),RET_ADDR+0x110:lambda emu:callback(emu,True),RET_ADDR+0x200:prepare_panel,0x14150d020:get_controller,0x1423c0280:move,0x14150e420:update,0x1412f9480:reset,0x1412c6070:delete,0x14150cf80:environment,0x140c7f790:lambda emu:log.append('C'),0x1414d7470:lambda emu:log.append('P'),0x14150fee0:final})
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
        size=emu.reg(UC_X86_REG_RCX);
        if size==24:return alloc(emu)
        assert size in [0x28,0x320]
        p=emu.alloc(size);emu.write(p,bytes(size));emu.uc.reg_write(UC_X86_REG_RAX,p)
        if size==0x320:allocated[0]=p;log.append('A:800')
    def bind(emu):
        assert emu.reg(UC_X86_REG_RDX)==panel+0x48
        p=emu.reg(UC_X86_REG_RCX);assert p in callbacks;log.append(f'B:{emu.read_u64(p+0x18)}')
    def construct(emu):
        assert emu.reg(UC_X86_REG_RCX)==allocated[0]
        assert [emu.reg(UC_X86_REG_R8),emu.reg(UC_X86_REG_R9)]==callbacks[::-1]
        p=allocated[0];emu.write_u64(p,object_vt);emu.write_u64(p+0x2c8,17);emu.write_u64(p+0x2d8,18);emu.write_u64(p+0x2d0,17)
        if kind[0]==0:
            assert emu.reg(UC_X86_REG_RDX)==0
            for cbp in callbacks:assert [emu.read_u64(cbp+off) for off in [0x10,0x18,0x20,0x60]]==[0,0,0,0]
            log.append('C:1:0')
        else:
            assert emu.reg(UC_X86_REG_RDX)==values[9]
            assert [emu.read_u64(cbp+0x18) for cbp in callbacks]==[0,3]
            log.append(f'T:{values[9]}:3:0')
        emu.uc.reg_write(UC_X86_REG_RAX,p)
    def cb(uc,a,s,u):
        empty=e.reg(UC_X86_REG_RDX)==0;kind[0]=0 if empty else 1
        if empty:assert [e.reg(reg) for reg in [UC_X86_REG_R8,UC_X86_REG_R9]]==[0,0]
        else:assert e.reg(UC_X86_REG_RDX)==panel+0x48 and e.reg(UC_X86_REG_R9)==0
        callbacks.append(e.reg(UC_X86_REG_RCX));log.append(f'H:{len(callbacks)-1 if empty else e.reg(UC_X86_REG_R8)}')
    def drop(uc,a,s,u):
        i=callbacks.index(e.reg(UC_X86_REG_RCX));log.append(f'D:{i if kind[0]==0 else [0,3][i]}')
    def field(emu):
        assert emu.reg(UC_X86_REG_RCX)==17 and emu.read(0x1457770c6,1)[0]==1
        log.append('E:c8:1')
    def component(emu):assert emu.reg(UC_X86_REG_RCX)==allocated[0]+0x260;log.append('E:260')
    def field2(emu):assert emu.reg(UC_X86_REG_RCX)==18;log.append('E:d8')
    def destroy(emu):assert emu.reg(UC_X86_REG_RCX)==allocated[0] and emu.reg(UC_X86_REG_RDX)==1;log.append('X:1')
    def query_document(emu):
        if phase[0]:log.append('Q');emu.uc.reg_write(UC_X86_REG_RAX,values[4])
        else:
            i=fq[0];fq[0]+=1;assert i<2;log.append(f'P:{i}');emu.uc.reg_write(UC_X86_REG_RAX,fallback[1+i])
            if i==0 and fallback[4]>=0:emu.write(0x142f2d770+0x525,bytes([fallback[4]]))
    def cache(emu):assert emu.reg(UC_X86_REG_RCX)==0x1460fc940;log.append('Q');emu.uc.reg_write(UC_X86_REG_RAX,fallback[3])
    def registry(emu):assert emu.reg(UC_X86_REG_RCX)==17;log.append('I:17');emu.uc.reg_write(UC_X86_REG_RAX,19)
    def resolve(emu):
        assert [emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX)]==[18,19]
        log.append('R:18:19');emu.uc.reg_write(UC_X86_REG_RAX,42)
    def submit(emu):
        assert emu.reg(UC_X86_REG_RCX)==panel+0x170 and emu.read_u32(emu.reg(UC_X86_REG_RDX))==0
        log.append(f'S:0:{emu.reg(UC_X86_REG_R8)}')
    def notify(emu):assert emu.reg(UC_X86_REG_RCX)==0x142609340;log.append('N')
    e.stubs.update({0x1417e2640:cache,0x1417e9f50:lambda emu:(log.append('M'),emu.uc.reg_write(UC_X86_REG_RAX,18)) and None,0x140d569e0:registry,0x1417e8a60:resolve,0x140ef67c0:submit,0x140658af0:notify,0x140f0a1b0:query_document,0x140efe0d0:doc,0x140efdda0:validate,0x1417fa7a0:load,0x140f0a210:lambda emu:log.append('FAIL'),0x1422e7bf0:allocate,0x1410c8de0:bind,0x140ec9c10:construct,0x1411103e0:field,0x1410c9d10:component,0x14111bb90:field2,RET_ADDR+0x300:destroy})
    e.uc.hook_add(UC_HOOK_CODE,cb,begin=0x1410c7d20,end=0x1410c7d20);e.uc.hook_add(UC_HOOK_CODE,drop,begin=0x1410c8430,end=0x1410c8430)
    e.uc.hook_add(UC_HOOK_CODE,lambda uc,a,s,u:log.append('DROP'),begin=0x140f0a404,end=0x140f0a404)
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
    print('# own/focused key/filter/full fallback/property2/thunk/direct/close/query/getter/mode/toggle/show/hide/reparent/find/dirty/map-store original; argument0; ui_mode0/1/2; inline document profile; document/dialog/binding effects controlled; preparation/controller getter controlled; close effects controlled; bind/activation/callbacks/memmove/alloc/refresh/event controlled; spare capacity; stable manager; empty listeners')
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
            item=e.alloc(0x1b0);e.write(item,bytes(0x1b0));e.write_u32(item+0x1ac,[0,4,6][(case_no+i)%3]);entries.append(item);e.write_u32(item+0x170,classes[i]);e.write_u64(item+0x138,[11,12,11][i]);e.write_u64(item+0x80,item+0x80);e.write_u64(item+0x88,manager if attached[i] else 0)
            parent=0
            for alpha in PATHS[pid]:
                widget=e.alloc(0x100);e.write(widget,bytes(0x100));node(widget,alpha,parent);parent=widget
            e.write_u64(item+0x140,parent);widgets.append(parent)
        h=linked(impl,0x150,[entries[i] for i in order]);m=linked(manager,0x90,[entries[i] for i in members])
        config[:]=[0 if case_no%7==0 else 11,int(case_no%5==0),case_no%2,case_no%3,case_no%4,case_no%2,6 if case_no%2==0 else -1,0 if case_no%3==0 else -1]
        updates[0]=0;e.write(controller,bytes(0x2b0));e.write_u64(0x145a0e308,manager);e.write_u64(0x14577b930,0)
        head=e.alloc(48);n=e.alloc(48);mapnode[0]=n;e.write(head,bytes(48));e.write(n,bytes(48));e.write(head+0x19,b'\1')
        for off in [0,8,16]:e.write_u64(head+off,n);e.write_u64(n+off,head)
        e.write_u32(n+32,11);e.write_u64(n+40,config[0]);e.write_u64(controller+0x148,head);e.write_u64(controller+0x150,1)
        vec=e.alloc(4);e.write_u32(vec,11);e.write_u64(controller+0x90,vec);e.write_u64(controller+0x98,vec+4 if config[2] else vec)
        e.write_u32(0x145a11f48,config[3]);e.write(controller+0x2a8,bytes([config[4]]))
        e.write(panel,bytes(0x300));e.write_u64(panel,panel_vt)
        e.write(panel+0x2c9,bytes([255 if case_no%4==0 else 0]));e.write(panel+0x2c8,bytes([255 if case_no%3==0 else 0]))
        e.write_u32(0x145a12118+0x18,case_no%3)
        values[:]=[0,0,0,case_no%3,(case_no%2)*255,case_no%256,0 if case_no%5==0 else 255,-1,-1,case_no+7,0,0,2]
        e.write_u64(panel+0x1a8,values[9]);e.write(0x1457770c6,b'\0')
        e.write_u64(panel+0x28,0x1426066b0);e.write_u64(panel+0x170,0x1426067a8)
        fallback[:]=[[0,1,255,0][case_no%4],(case_no%2)*255,0 if case_no%7==0 else 255,0 if case_no%5==0 else 255,255 if case_no%11==0 else -1]
        e.write(0x142f2d770+0x525,bytes([fallback[0]]))
        e.write(panel+0x168,bytes([255 if case_no%13==0 else 0]));e.write(panel+0x169,bytes([255 if case_no%11==0 else 0]))
        mask=0 if case_no%17==0 else 0xfff;e.write_u32(panel+0x18,mask);e.write_u64(0x145a0dfc8,panel)
        vk=[27,13,0xbb,0xbc,65][case_no%5];flags=[8,16,8,0,8,8,8][case_no%7];results=[]
        queries=[]
        for route in range(2):
            callbacks.clear();allocated[0]=0;phase[0]=0;fq[0]=0
            raw=e.call(0x1412e1de0 if route==0 else 0x1412c6300,ints=[panel,27,vk,flags] if route==0 else [27,vk,flags],max_instructions=100000);results.append(raw&255)
            queries.append(e.call(0x141512eb0,ints=[controller,11],max_instructions=100000)&255)
        ro,rm=read_list(h),read_list(m);assert len(ro)==e.read_u64(impl+0x158) and len(rm)==e.read_u64(manager+0x98)
        print('C',root,w1,w2,p0,p1,p2,csv(classes),csv(order),csv(members),csv(attached),ialpha,csv(config),'->',csv(ro),csv(rm),csv([int(e.read_u64(p+0x88)!=0) for p in entries]),csv([e.read_u32(p+0x58) for p in widgets]),csv([-1 if e.read_u64(p+0x60)==0 else graph.index(e.read_u64(p+0x60)) for p in graph]),csv([e.read_u32(p+0xf8) for p in graph]),';'.join(csv([graph.index(e.read_u64(a)) for a in range(e.read_u64(p+0x70),e.read_u64(p+0x78),8)]) for p in graph),csv(queries),csv([e.read_u32(p+0x1ac) for p in entries]),e.read_u64(n+40),csv([e.read(panel+0x2c9,1)[0],e.read(panel+0x2c8,1)[0],e.read_u32(0x145a12118+0x18),e.read(0x1457770c6,1)[0],e.read(0x142f2d770+0x525,1)[0]]),csv(results),*log);count+=1
    print('# contexts',count,'native key executions',count*2,'native direct executions',directs[0],'native close executions',closes[0],'native visibility executions',vis[0])


if __name__=='__main__':main(sys.argv[1])
