#!/usr/bin/env python3
"""Local original propeller reference point corpus. No stubs or automatic publishing."""
import hashlib,random,struct,sys
from pathlib import Path
from emulate_xp import Emulator,STACK_TOP
from unicorn.x86_const import UC_X86_REG_R12,UC_X86_REG_RDI,UC_X86_REG_RBP,UC_X86_REG_RSP
SHA='2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def main(path):
 if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=SHA:raise ValueError('reference digest mismatch')
 e=Emulator(path);aircraft=e.alloc(0x6600);parts=e.alloc(8*0x3770);points=e.alloc(4096);frame=e.alloc(0x1000)
 e.write_u64(aircraft+0x6010,parts);rng=random.Random(20261017)
 print('# reference_sha256',SHA);print('# P count; each kind,elements,radii[12],center[3] -> point_count xyz; original 0x141a8879b..0x141a88dc6 exclusive; no stubs; preallocated vector')
 for k in range(400):
  count=k%9;inputs=[];active=0;e.write_u32(aircraft+0x920,count)
  for slot in range(count):
   p=parts+slot*0x3770;kind=[0,1,9,-1][(k+slot)%4];n=[1,2,4,10,11,12,100][(k+slot)%7]
   radii=[rng.uniform(-10,10) for _ in range(12)];center=[rng.uniform(-100,100) for _ in range(3)]
   if k==1:radii=[-0.]*12;center=[-0.,0.,-0.]
   e.write_u32(p,kind&0xffffffff);e.write_u32(p+0x8c,n)
   e.write(p+0x644,struct.pack('<12f',*radii))
   for axis,v in enumerate(center):e.write_f32(p+0x778+axis*4,v)
   inputs += [kind&0xffffffff,n,*[bits(v) for v in radii+center]];active+=kind!=9
  for off,v in [(0x6478,points),(0x6480,points),(0x6488,points+4096)]:e.write_u64(aircraft+off,v)
  for reg,val in [(UC_X86_REG_R12,aircraft),(UC_X86_REG_RDI,0),(UC_X86_REG_RBP,frame),(UC_X86_REG_RSP,STACK_TOP-0x2000)]:e.uc.reg_write(reg,val)
  e.run_range(0x141a8879b,0x141a88dc6,max_instructions=1000000)
  assert not e.stubs and e.read_u64(aircraft+0x6480)==points+active*96
  out=struct.unpack('<'+'I'*(active*24),e.read(points,active*96)) if active else []
  print('P',count,*[f'{v:08x}' for v in inputs],'->',active*8,*[f'{v:08x}' for v in out])
 print('# cases',400)
if __name__=='__main__':main(sys.argv[1])
