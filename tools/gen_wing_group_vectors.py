#!/usr/bin/env python3
"""Replay the original 49-pass wing group scan, with only outputstream calls stubbed.
Root-vector initialization is explicit; both vector buffers are preallocated.
"""
import hashlib, random, struct, sys
from pathlib import Path
from emulate_xp import Emulator, STACK_TOP
from unicorn.x86_const import *
SHA256='2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
BEGIN,END=0x1411720dc,0x14117285b
LOGGING=[0x1405e18d0,0x1405ea2c0,0x1408943e0,0x140606f80]
STRIDE=0x36c8

def bits(v):return struct.unpack('<I',struct.pack('<f',v))[0]
def f32(v):return struct.unpack('<f',struct.pack('<f',v))[0]
def xmm_double(e,reg,value):e.uc.reg_write(reg,struct.unpack('<Q',struct.pack('<d',value))[0])
def make(slot,root,tip,angle=0.,enabled=True,n=2,sweep=0.,rc=3.,tc=1.,span=1.):
    arrays=[[f32(rc+(tc-rc)*i/n) for i in range(n+1)]]
    arrays += [[f32(root[a]+(tip[a]-root[a])*i/n) for i in range(n+1)] for a in range(3)]
    return slot,int(enabled),n,span,sweep,rc,tc,angle,arrays

def cases():
    yield 0,[make(0,[1,0,0],[2,0,0]),make(1,[2,0,0],[1,0,0])]
    yield 0,[make(0,[5,0,0],[6,0,0]),make(2,[6,0,0],[7,0,0],90.),make(1,[7,0,0],[8,0,0],90.)]
    yield 3,[make(3,[4,0,0],[5,0,0]),make(0,[3,0,0],[4,0,0]),make(2,[2,0,0],[3,0,0]),make(1,[5,0,0],[6,0,0])]
    yield 0,[make(0,[-2,0,0],[-1,0,0]),make(1,[-1,0,0],[1,0,0])]
    yield 0,[make(0,[4,0,0],[5,0,0],rc=1.,tc=1.),make(1,[3,0,0],[4,0,0],rc=4.,tc=2.),make(2,[2,0,0],[3.025,0,0],rc=2.,tc=.5)]
    rng=random.Random(20261025)
    for _ in range(120):
        count=rng.randrange(2,11);slots=rng.sample(range(48),count);root=rng.randrange(count);wings=[]
        for i,slot in enumerate(slots):
            wings.append(make(slot,[1+i,0,0],[2+i,0,0],rng.choice([0.,20.,45.,60.,90.]),i==root or rng.random()>.15,rng.randrange(1,11),rng.uniform(-30.,30.),rng.uniform(.125,6.),rng.uniform(.125,6.),rng.uniform(.125,12.)))
        yield slots[root],wings

def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=SHA256:raise ValueError('reference SHA256 mismatch')
    e=Emulator(path);aircraft=e.alloc(0x7000);base=e.alloc(48*STRIDE);frame=e.alloc(4096)+128;ids=e.alloc(1024);ptrs=e.alloc(1024);stream=e.alloc(256)
    e.write_u64(aircraft+0x6028,base)
    print('# reference_sha256',SHA256)
    print('# original_range1411720dc..14117285b;stubs ONLY outputstream1405e18d0/1405ea2c0/1408943e0/140606f80;preallocated vectors;root init explicit')
    print('# G root_id count | each: id enabled elements span sweep rootchord tipchord dihedral chord[] x[] y[] z[] areas[] | -> nmembers ids[] endpoints6 area refchords2 refangles2')
    count=0;logging_calls=0
    for root_id,wings in cases():
        e.stubs.clear();e.write(base,bytes(48*STRIDE));e.write(frame-128,bytes(4096));areas=[]
        for slot,enabled,n,span,sweep,rc,tc,angle,arrays in wings:
            w=base+slot*STRIDE;e.write(w+0x678,bytes([enabled]));e.write_u32(w+4,n)
            for off,v in [(8,rc),(12,tc),(16,span),(0x20,angle),(0x30,sweep)]:e.write_f32(w+off,v)
            for off,values in zip([0x70,0x5bc,0x5e8,0x614],arrays):e.write(w+off,struct.pack('<'+'f'*(n+1),*values))
            areas.append([e.call_float(0x1411a0080,ints=[w,i]) for i in range(n)])
        root_index=next(i for i,w in enumerate(wings) if w[0]==root_id);root=wings[root_index];slot,enabled,n,span,sweep,rc,tc,angle,arrays=root;w=base+slot*STRIDE
        def endpoint(tip):
            index=n if tip else 0;lo=min(index,n-1)
            return [e.call_float(0x1406ea0b0,floats=[float(lo),a[lo],float(lo+1),a[lo+1]],stack=[float(index)]) for a in arrays[1:]]
        ends=endpoint(False)+endpoint(True)
        for i,v in enumerate(ends):e.write_f32(w+0x36b0+4*i,v)
        initial=0.
        for area in areas[root_index]:initial=f32(initial+area)
        e.write_u32(ids,root_id);e.write_u64(ptrs,w)
        for off,value in [(0x3690,ids),(0x3698,ids+4),(0x36a0,ids+1024)]:e.write_u64(w+off,value)
        for off,value in [(-0x60,ptrs),(-0x58,ptrs+8),(-0x50,ptrs+1024),(0x8b0,aircraft)]:e.write_u64(frame+off,value)
        rsp=STACK_TOP-0x1000;e.write_u32(rsp+0x60,root_id);e.write_f32(rsp+0x7c,tc);e.write_f32(rsp+0x78,initial)
        for reg,value in [(UC_X86_REG_RDI,w),(UC_X86_REG_R15,aircraft),(UC_X86_REG_R12,w+0x6c0),(UC_X86_REG_RBP,frame),(UC_X86_REG_RSP,rsp)]:e.uc.reg_write(reg,value)
        for reg,v in [(UC_X86_REG_XMM8,tc),(UC_X86_REG_XMM9,initial),(UC_X86_REG_XMM10,angle),(UC_X86_REG_XMM11,angle),(UC_X86_REG_XMM12,rc),(UC_X86_REG_XMM13,0.),(UC_X86_REG_XMM14,struct.unpack('<f',bytes.fromhex('36fa8e3c'))[0])]:e.uc.reg_write(reg,bits(v))
        xmm_double(e,UC_X86_REG_XMM7,.5)
        def log_stub(em):
            nonlocal logging_calls
            logging_calls+=1;em.uc.reg_write(UC_X86_REG_RAX,stream)
        e.stubs={a:log_stub for a in LOGGING}
        e.run_range(BEGIN,END,max_instructions=4000000)
        assert set(e.stubs)==set(LOGGING)
        id_end=e.read_u64(w+0x3698);p_end=e.read_u64(frame-0x58)
        members=list(struct.unpack('<'+'I'*((id_end-ids)//4),e.read(ids,id_end-ids)))
        pointer_ids=[(e.read_u64(ptrs+i*8)-base)//STRIDE for i in range((p_end-ptrs)//8)]
        assert pointer_ids==members and 1<=len(members)<=97
        output=[e.read_u32(w+0x36b0+i*4) for i in range(6)]+[e.read_u32(rsp+0x78),e.read_u32(rsp+0x68),e.read_u32(rsp+0x7c),e.reg(UC_X86_REG_XMM10)&0xffffffff,e.reg(UC_X86_REG_XMM11)&0xffffffff]
        tokens=['G',str(root_id),str(len(wings))]
        for wing,area in zip(wings,areas):
            slot,en,n,span,sweep,rc,tc,angle,arrays=wing
            tokens+=['|',str(slot),str(en),str(n),*[f'{bits(v):08x}' for v in [span,sweep,rc,tc,angle,*[v for a in arrays for v in a],*area]]]
        tokens+=['|',str(len(members)),*map(str,members),*[f'{v:08x}' for v in output]]
        print(*tokens);count+=1
    print('# cases',count,'logging_calls',logging_calls)

if __name__=='__main__':main(sys.argv[1])
