#!/usr/bin/env python3
"""Local combined original point producers and bounds reduction; allocator-only stubs."""
import hashlib,random,struct,sys
from pathlib import Path
from emulate_xp import Emulator,STACK_TOP
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RAX,UC_X86_REG_R12,UC_X86_REG_RBX,UC_X86_REG_RDI,UC_X86_REG_RBP,UC_X86_REG_RSP,UC_X86_REG_XMM11,UC_X86_REG_XMM12
SHA='2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
OFFSETS=[0x64dc,0x64e4,0x64ec,0x64e0,0x64e8,0x64f0,0x64d8,0x64f4,0x64f8,0x64fc]
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def main(path):
 if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=SHA:raise ValueError('reference digest mismatch')
 e=Emulator(path);aircraft=e.alloc(0x7000);gears=e.alloc(10*0x88);wings=e.alloc(48*0x36c8);bodies=e.alloc(39*0x34c8);parts=e.alloc(8*0x3770);points=e.alloc(200000);frame=e.alloc(0x1000)
 allocation_calls=[0,0]
 def allocate(e):
  allocation_calls[0]+=1;e.uc.reg_write(UC_X86_REG_RAX,e.alloc(max(e.reg(UC_X86_REG_RCX),16)+16,16))
 def release(e):allocation_calls[1]+=1
 e.stubs[0x1422e7bf0]=allocate;e.stubs[0x1422e7c2c]=release
 rng=random.Random(20261018)
 print('# reference_sha256',SHA);print('# combined original 0x141a87e30..0x141a8923e exclusive, operator new/delete stubs only; numerical helpers original; aux object vector empty; preallocated main vector')
 for case in range(80):
  for address,size in [(aircraft,0x7000),(gears,10*0x88),(wings,48*0x36c8),(bodies,39*0x34c8),(parts,8*0x3770),(frame,0x1000)]:e.write(address,bytes(size))
  for off,v in [(0x6080,gears),(0x6028,wings),(0x6040,bodies),(0x6010,parts),(0x6478,points),(0x6480,points),(0x6488,points+200000)]:e.write_u64(aircraft+off,v)
  inputs=[]
  for slot in range(10):
   kind=0 if case==0 else [-1,0,1,2][(case+slot)%4];values=[rng.uniform(-2,5),rng.uniform(-1,1),*[rng.uniform(-20,20) for _ in range(3)]]
   e.write_u32(gears+slot*0x88,kind&0xffffffff)
   for off,value in zip([0x18,0x58,0x64,0x70,0x7c],values):e.write_f32(gears+slot*0x88+off,value)
   inputs += [kind&0xffffffff,*[bits(v) for v in values]]
  wc=0 if case==0 else case%4;inputs.append(wc)
  for slot in sorted(rng.sample(range(48),wc)):
   n=[1,4,10][slot%3];p=wings+slot*0x36c8;e.write(p+0x678,b'\1');e.write_u32(p+4,n);inputs += [slot,n]
   for off in [0x70,0x5bc,0x5e8,0x614]:
    array=[rng.uniform(-10,20) for _ in range(n+1)];e.write(p+off,struct.pack('<'+'f'*(n+1),*array));inputs += [bits(v) for v in array]
  bc=0 if case==0 else case%3;inputs.append(bc)
  for slot in sorted(rng.sample(range(39),bc)):
   ns=[1,2,4,6][slot%4];rings=[0,1,3,7,18][slot%5];origin=[rng.uniform(-20,20) for _ in range(3)];grid=[rng.uniform(-5,5) for _ in range(ns*54)];p=bodies+slot*0x34c8
   e.write(p+0x588,b'\1');e.write_u32(p+0x654,ns);e.write_u32(p+0x658,rings)
   for axis,v in enumerate(origin):e.write_f32(p+0x600+axis*4,v)
   e.write(p+0x65c,struct.pack('<'+'f'*len(grid),*grid));inputs += [slot,ns,rings,*[bits(v) for v in origin+grid]]
  pc=0 if case==0 else case%9;inputs.append(pc);e.write_u32(aircraft+0x920,pc)
  for slot in range(pc):
   kind=[0,9,1,-1][(case+slot)%4];n=[1,4,10,11,100][slot%5];radii=[rng.uniform(-5,5) for _ in range(12)];center=[rng.uniform(-20,20) for _ in range(3)];p=parts+slot*0x3770
   e.write_u32(p,kind&0xffffffff);e.write_u32(p+0x8c,n);e.write(p+0x644,struct.pack('<12f',*radii))
   for axis,v in enumerate(center):e.write_f32(p+0x778+axis*4,v)
   inputs += [kind&0xffffffff,n,*[bits(v) for v in radii+center]]
  e.write_u64(frame+0x4c0,aircraft)
  for reg,val in [(UC_X86_REG_R12,aircraft),(UC_X86_REG_RBX,aircraft+0x6478),(UC_X86_REG_RDI,0),(UC_X86_REG_RBP,frame),(UC_X86_REG_RSP,STACK_TOP-0x2000),(UC_X86_REG_XMM11,0),(UC_X86_REG_XMM12,bits(1.))]:e.uc.reg_write(reg,val)
  e.run_range(0x141a87e30,0x141a8923e,max_instructions=20000000)
  assert set(e.stubs)=={0x1422e7bf0,0x1422e7c2c}
  count=(e.read_u64(aircraft+0x6480)-points)//12
  out=struct.unpack('<'+'I'*(count*3),e.read(points,count*12)) if count else []
  print('C',*[f'{x:08x}' for x in inputs],'->',count,*[f'{x:08x}' for x in out],'->',*[f'{e.read_u32(aircraft+off):08x}' for off in OFFSETS])
 print('# cases',80,'allocator_calls',*allocation_calls)
if __name__=='__main__':main(sys.argv[1])
