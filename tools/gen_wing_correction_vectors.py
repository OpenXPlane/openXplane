#!/usr/bin/env python3
"""Execute original Reynolds, flap/slat, drag and ice ranges; no stubs or math substitutions."""
import hashlib
import random
import struct
import sys
from pathlib import Path
from emulate_xp import Emulator, STACK_TOP
from gen_wing_vectors import SHA256, bits, set_float
from unicorn.x86_const import (
    UC_X86_REG_RSI, UC_X86_REG_RDI, UC_X86_REG_R12, UC_X86_REG_R13,
    UC_X86_REG_R14, UC_X86_REG_R15, UC_X86_REG_RBP, UC_X86_REG_RSP,
    UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_XMM0, UC_X86_REG_XMM1,
    UC_X86_REG_XMM3, UC_X86_REG_XMM4, UC_X86_REG_XMM6,
    UC_X86_REG_XMM10, UC_X86_REG_XMM11, UC_X86_REG_XMM12,
)


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != SHA256:
        raise ValueError('Reference SHA256 mismatch')
    e=Emulator(path)
    wing=e.alloc(4096); obj=e.alloc(0xbd00); flow=e.alloc(4096)
    frame=e.alloc(1024)+128; reference=e.alloc(0x2000); foil=e.alloc(256); foil_data=e.alloc(64)
    clp=e.alloc(16); cdp=e.alloc(16); dragp=e.alloc(16)
    e.write_u64(foil+0xd0,foil_data)
    rng=random.Random(20261008)
    def registers():
        for reg,val in [(UC_X86_REG_RSI,wing),(UC_X86_REG_RDI,obj),(UC_X86_REG_R12,flow),(UC_X86_REG_R13,0),(UC_X86_REG_R15,0),(UC_X86_REG_RBP,frame),(UC_X86_REG_RSP,STACK_TOP-0x2000)]: e.uc.reg_write(reg,val)
        set_float(e,UC_X86_REG_XMM10,0.0); set_float(e,UC_X86_REG_XMM11,1.0)
        e.uc.reg_write(UC_X86_REG_XMM12,struct.unpack('<Q',struct.pack('<d',1.0))[0])
    def emit(kind,inputs,outputs):
        print(kind,*(f'{bits(v):08x}' for v in inputs),*(f'{v:08x}' for v in outputs))
    def xmm(reg): return e.uc.reg_read(reg)&0xffffffff
    print('# reference_sha256',SHA256)
    temperatures=[-100,-50,-49.999,-0.001,-0.0,0.0,0.001,49.999,50,100]
    cases=[(t,a,b,s,d) for t in temperatures for a,b in [(1.0,1.0),(2.5,0.0),(0.25,1.0)] for s,d in [(0.0,1.225),(70.0,1.225)]]
    cases += [(rng.uniform(-100,100),rng.uniform(.01,8),rng.uniform(0,8),rng.uniform(0,500),rng.uniform(.01,2)) for _ in range(1000)]
    for t,a,b,s,d in cases:
        registers(); e.write_f32(obj+0x5c,t);e.write_f32(obj+0x6c,d);e.write_f32(wing+0x70,a);e.write_f32(wing+0x74,b);e.write_f32(flow+0x54,s)
        e.run_range(0x1411b6979,0x1411b6a44)
        re=xmm(UC_X86_REG_XMM6)
        # Obtain intermediates at the original boundaries, never recompute them in Python.
        registers();e.run_range(0x1411b6979,0x1411b69e7); viscosity=xmm(UC_X86_REG_XMM0)
        registers();e.run_range(0x1411b6979,0x1411b6a2c);chord=xmm(UC_X86_REG_XMM1)
        emit('R',[t,a,b,s,d],[viscosity,chord,re])
    for i in range(1024):
        amount=[-0.5,-0.0,0.0,1e-6,0.5,1.0,2.0,3.0][i] if i<8 else rng.uniform(-.5,3)
        registers();e.write_f32(frame+0x150,amount)
        e.run_range(0x1411b67b4,0x1411b6817)
        emit('A',[amount],[xmm(UC_X86_REG_XMM4),e.read_u32(frame-0x20),e.read_u32(frame-0x1c)])
    for flap,slat in [(0,0),(1,0),(0,1),(1,1)]:
        for i in range(256):
            inputs=[rng.uniform(-.5,1.5),rng.uniform(-.5,1.5),rng.uniform(.01,5),rng.uniform(-1,5),rng.uniform(-1,5)]
            registers();e.uc.reg_write(UC_X86_REG_RCX,reference);e.uc.reg_write(UC_X86_REG_RDX,foil);set_float(e,UC_X86_REG_XMM3,1.0)
            e.write_u32(wing+0x56c,flap);e.write_u32(wing+0x594,slat)
            for addr,val in zip([obj+0x1a0,obj+0x1a4,foil_data+0x10,reference+0x1f00,reference+0x1f3c],inputs):e.write_f32(addr,val)
            e.run_range(0x1411b6817,0x1411b689b)
            emit('S',[float(flap),float(slat)]+inputs,[xmm(UC_X86_REG_XMM3)])
    for i in range(1024):
        inputs=[rng.uniform(-3,3),rng.uniform(-.1,2),rng.uniform(.01,100),rng.uniform(.01,3),rng.uniform(0,2)]
        cl,cd,aspect,eff,scale=inputs
        registers();e.uc.reg_write(UC_X86_REG_R14,clp);e.uc.reg_write(UC_X86_REG_R15,cdp)
        e.write_f32(clp,cl);e.write_f32(cdp,cd);e.write_f32(wing+0x14,aspect);e.write_f32(frame-0x5c,eff);e.write_u64(frame+0x190,dragp);e.write_f32(flow+0x28c,scale)
        e.run_range(0x1411b7c14,0x1411b7c42);e.run_range(0x1411b7d87,0x1411b7da3)
        emit('I',inputs,[e.read_u32(dragp),e.read_u32(cdp)])
    for i in range(1024):
        inputs=[rng.uniform(-3,3),rng.uniform(-.1,2),[-1.0,-0.0,0.0,1e-6,1.0,float("nan"),float("inf"),float("-inf")][i%8],rng.uniform(0,2),rng.uniform(0,3)]
        cl,cd,amount,cm,dm=inputs
        registers();e.uc.reg_write(UC_X86_REG_R14,clp);e.uc.reg_write(UC_X86_REG_R15,cdp)
        e.write_f32(clp,cl);e.write_f32(cdp,cd);e.write_f32(frame+0x150,amount);e.write_f32(frame-0x20,cm);e.write_f32(frame-0x1c,dm)
        e.run_range(0x1411b8d15,0x1411b8da0)
        emit('C',inputs,[e.read_u32(clp),e.read_u32(cdp)])
if __name__=='__main__':main(sys.argv[1])
