#!/usr/bin/env python3
"""Local ten-slot original-code gear bounds corpus, without stubs."""
import hashlib,random,struct,sys
from pathlib import Path
from emulate_xp import Emulator,STACK_TOP
from unicorn.x86_const import UC_X86_REG_R12,UC_X86_REG_RBX,UC_X86_REG_RDI,UC_X86_REG_RBP,UC_X86_REG_RSP
SHA='2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def main(path):
 if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=SHA:raise ValueError('reference digest mismatch')
 e=Emulator(path);aircraft=e.alloc(0x6600);gears=e.alloc(0x550);vector=e.alloc(24);points=e.alloc(1024);frame=e.alloc(0x1000)
 e.write_u64(aircraft+0x6080,gears);rng=random.Random(20261016)
 print('# reference_sha256',SHA);print('# G ten slots: kind,leg_length,tire_radius,center_xyz -> point_count xyz; original 0x141a87e30..0x141a87fb2 exclusive; no stubs; preallocated scalar vector')
 for k in range(500):
  e.write(gears,bytes(0x550));inputs=[];active=0
  for slot in range(10):
   kind=[0,-1,1,2,9][(k+slot)%5] if k else 0
   values=[rng.uniform(-5,5),rng.uniform(-2,2),*[rng.uniform(-100,100) for _ in range(3)]]
   if k==1:values=[-0.,0.,-0.,0.,-0.]
   e.write_u32(gears+slot*0x88,kind&0xffffffff)
   for off,value in zip([0x18,0x58,0x64,0x70,0x7c],values):e.write_f32(gears+slot*0x88+off,value)
   inputs += [kind&0xffffffff,*[bits(x) for x in values]];active+=kind!=0
  for off,v in [(0,points),(8,points),(16,points+1024)]:e.write_u64(vector+off,v)
  for reg,val in [(UC_X86_REG_R12,aircraft),(UC_X86_REG_RBX,vector),(UC_X86_REG_RDI,0),(UC_X86_REG_RBP,frame),(UC_X86_REG_RSP,STACK_TOP-0x2000)]:e.uc.reg_write(reg,val)
  e.run_range(0x141a87e30,0x141a87fb2,max_instructions=100000)
  assert not e.stubs and e.read_u64(vector+8)==points+active*24
  output=struct.unpack('<'+'I'*(active*6),e.read(points,active*24)) if active else []
  print('G',*[f'{v:08x}' for v in inputs],'->',active*2,*[f'{v:08x}' for v in output])
 print('# cases',500)
if __name__=='__main__':main(sys.argv[1])
