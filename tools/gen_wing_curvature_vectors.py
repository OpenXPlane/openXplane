#!/usr/bin/env python3
"""Replay original aircraft curvature setup; original CRT, no stubs.
Output goes to a caller-selected local file; it is not automatically published.
"""
import hashlib,random,struct,sys
from pathlib import Path
from emulate_xp import Emulator,STACK_TOP
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_R14,UC_X86_REG_RSI,UC_X86_REG_RSP,UC_X86_REG_RBP,UC_X86_REG_XMM13,UC_X86_REG_XMM14,UC_X86_REG_XMM15
SHA='2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def main(path):
 if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=SHA:raise ValueError('reference digest mismatch')
 e=Emulator(path);aircraft=e.alloc(0x6600);context=e.alloc(0x40);frame=e.alloc(0x700)
 e.write_u64(context+0x20,aircraft)
 rng=random.Random(20261014);degreebits=[0,0x80000000,*[bits(.001)+d for d in [-1,0,1]],*[bits(-.001)+d for d in [-1,0,1]]]
 degrees=[struct.unpack('<f',struct.pack('<I',x))[0] for x in degreebits]+[.01,-.01,1.,-1.,30.,60.,90.,120.,179.,180.,270.,360.,-360.]+[rng.uniform(-720,720) for _ in range(500)]
 print('# reference_sha256',SHA)
 print('# C degrees,halfspan,previous_scale -> curvature_angle,amplitude,scale; original 0x14124df67..0x14124e036 exclusive, no stubs')
 for k,d in enumerate(degrees):
  halfspan=[0.,1.,10.,50.][k%4];previous=rng.uniform(.1,100)
  e.write_f32(aircraft+0x28fc,d);e.write_f32(aircraft+0x64f4,halfspan);e.write_f32(aircraft+0x6078,previous)
  for reg,val in [(UC_X86_REG_RAX,aircraft),(UC_X86_REG_R14,context),(UC_X86_REG_RSI,0),(UC_X86_REG_RSP,STACK_TOP-0x2000),(UC_X86_REG_RBP,frame),(UC_X86_REG_XMM13,0x7fffffff),(UC_X86_REG_XMM14,0x3c8efa36),(UC_X86_REG_XMM15,struct.unpack('<Q',struct.pack('<d',.001))[0])]:e.uc.reg_write(reg,val)
  e.run_range(0x14124df67,0x14124e036,max_instructions=500000)
  assert not e.stubs
  print('C',*[f'{x:08x}' for x in [bits(d),bits(halfspan),bits(previous),*[e.read_u32(aircraft+off) for off in [0x6070,0x6074,0x6078]]]])
 print('# cases',len(degrees))
if __name__=='__main__':main(sys.argv[1])
