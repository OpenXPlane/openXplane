#!/usr/bin/env python3
"""Replay original boundary loop and complete point transform, including native CRT."""
import hashlib
import random
import struct
import sys
from pathlib import Path
from emulate_xp import Emulator, STACK_TOP
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RSP, UC_X86_REG_RBX, UC_X86_REG_R14, UC_X86_REG_R12, UC_X86_REG_RSI, UC_X86_REG_XMM12, UC_X86_REG_XMM14, UC_X86_REG_XMM0
SHA='2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
OUTER={0x14124e554:0,0x14124e55f:1,0x14124e575:2,0x14124e581:3}
RETURNS={0x1412942ca:0,0x1412942e5:1,0x1412942f7:2,0x14129430d:3,0x141294333:4,0x141294345:5,0x141294367:6,0x141294391:7,0x14129439d:8,0x1412943b3:9,0x1412943bf:10,0x1412943e5:11,0x141294212:12,0x1412944e9:13}


def bits(v):return struct.unpack('<I',struct.pack('<f',v))[0]


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=SHA:raise ValueError('reference digest mismatch')
    e=Emulator(path);aircraft=e.alloc(0x6600);wings=e.alloc(0x36c8*48);context=e.alloc(0x40)
    e.write_u64(aircraft+0x6028,wings);e.write_u64(context+0x20,aircraft)
    records=[]; current=[]; outer=[0]*4
    def observe(uc,address,size,user):
        if address==0x141294240:
            current[:]=[0]*14;records.append(current.copy())
        elif address in OUTER:
            outer[OUTER[address]]=uc.reg_read(UC_X86_REG_XMM0)&0xffffffff
        elif address in RETURNS:
            records[-1][RETURNS[address]]=uc.reg_read(UC_X86_REG_XMM0)&0xffffffff
    hooks=[e.uc.hook_add(UC_HOOK_CODE,observe,begin=a,end=a) for a in [0x141294240,*RETURNS,*OUTER]]
    rng=random.Random(20261013)
    print('# reference_sha256',SHA)
    print('# original boundary range 0x14124e542..0x14124e643 exclusive; full 0x141294240 point transform; original CRT; no stubs')
    print('# B n slot fields[13]:span,design_dihedral,design_sweep,rotation,dihedral_delta,side,var_sweep,rotation_offset,retract,origin_xyz,curve; curvature_amplitude,curvature_scale; chords[n+1],offsets[n+1]; per rib: original_xyz,recorded_trig[14]')
    cases=0
    for k in range(240):
        n=k%11;slot=k%48;wing=wings+slot*0x36c8
        fields=[rng.uniform(.01,50),rng.uniform(-90,90),rng.uniform(-75,75),rng.uniform(-45,45),rng.uniform(-30,30),[-1.,1.][k%2],float(k%2),rng.uniform(-5,5),rng.uniform(-.25,1),rng.uniform(-20,20),rng.uniform(-20,20),rng.uniform(-20,20),[0.,.0009999,.001,.02,-.15][k%5]]
        if k<11:fields=[10.,0.,0.,0.,0.,1.,0.,0.,0.,0.,0.,0.,0.]
        amp=rng.uniform(.1,100);scale=rng.uniform(.1,100)
        fields=[struct.unpack('<f',struct.pack('<f',x))[0] for x in fields]
        amp,scale=[struct.unpack('<f',struct.pack('<f',x))[0] for x in [amp,scale]]
        e.write(wing,bytes(0x36c8));e.write(wing+4,struct.pack('<i',n))
        for off,value in zip([0x10,0x20,0x30,0x38,0x28,0,0x3c,0x40,0x54,0x6f0,0x6f4,0x6f8],fields[:12]):
            if off==0x3c:e.write(wing+off,struct.pack('<i',int(value)))
            else:e.write_f32(wing+off,value)
        for off,value in zip([0x6070,0x6074,0x6078],[fields[12],amp,scale]):e.write_f32(aircraft+off,value)
        chords=[rng.uniform(-2,8) for _ in range(n+1)];offsets=[rng.uniform(-.5,.5) for _ in range(n+1)]
        for i,(c,o) in enumerate(zip(chords,offsets)):e.write_f32(wing+0x70+i*4,c);e.write_f32(wing+0x9c+i*4,o)
        records.clear()
        for reg,value in [(UC_X86_REG_RSP,STACK_TOP-0x2000),(UC_X86_REG_RBX,wing),(UC_X86_REG_R14,context),(UC_X86_REG_R12,slot),(UC_X86_REG_RSI,0),(UC_X86_REG_XMM12,bits(1.)),(UC_X86_REG_XMM14,0x3c8efa36)]:e.uc.reg_write(reg,value)
        e.run_range(0x14124e542,0x14124e643,max_instructions=500_000)
        assert len(records)==n+1 and not e.stubs
        result=[bits(x) for x in [*fields,amp,scale,*chords,*offsets]]
        for i,trig in enumerate(records):
            assert [trig[j] for j in [3,0,2,1]]==outer
            assert trig[0]==trig[4] and trig[1]==trig[5] and trig[2]==trig[11] and trig[3]==trig[6]
            result += [e.read_u32(wing+off+i*4) for off in [0x5bc,0x5e8,0x614]]+trig
        print('B',n,slot,*[f'{x:08x}' for x in result]);cases+=1
    for h in hooks:e.uc.hook_del(h)
    print('# cases',cases)


if __name__=='__main__':main(sys.argv[1])
