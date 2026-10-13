#!/usr/bin/env python3
"""Local synthetic original-code body bounds point corpus; not published automatically."""
import hashlib,random,struct,sys
from pathlib import Path
from emulate_xp import Emulator,STACK_TOP
from unicorn.x86_const import UC_X86_REG_R12,UC_X86_REG_RBX,UC_X86_REG_RSI,UC_X86_REG_RDI,UC_X86_REG_RBP,UC_X86_REG_RSP
SHA='2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
def bits(x):return struct.unpack('<I',struct.pack('<f',x))[0]
def main(path):
 if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=SHA:raise ValueError('reference digest mismatch')
 e=Emulator(path);aircraft=e.alloc(0x6600);bodies=e.alloc(39*0x34c8);vector=e.alloc(24);points=e.alloc(100000);frame=e.alloc(0x1000)
 e.write_u64(aircraft+0x6040,bodies);rng=random.Random(20261015)
 print('# reference_sha256',SHA)
 print('# original 0x141a88340..0x141a88735 exclusive; one body; no stubs; preallocated scalar vector avoids allocator branch')
 print('# P enabled station_count ring_count slot origin_xyz grid[stations*18*3] -> point_count xyz... (f32 bits)')
 for k in range(120):
  stations=[0,1,2,3,17,20][k%6];rings=[0,1,2,7,18][k%5];enabled=int(k%11!=0);slot=k%39;body=bodies+slot*0x34c8
  e.write(body,bytes(0x34c8));e.write(body+0x588,bytes([enabled]));e.write_u32(body+0x654,stations);e.write_u32(body+0x658,rings)
  origin=[rng.uniform(-100,100) for _ in range(3)];grid=[rng.uniform(-20,20) for _ in range(stations*18*3)]
  if k==14:origin=[-0.,0.,-0.];grid=[-0. if i%2 else 0. for i in range(stations*18*3)]
  for axis,v in enumerate(origin):e.write_f32(body+0x600+axis*4,v)
  if grid:e.write(body+0x65c,struct.pack('<'+'f'*len(grid),*grid))
  e.write_u64(vector,points);e.write_u64(vector+8,points);e.write_u64(vector+16,points+100000);e.write_u64(frame+0x4c0,aircraft)
  for reg,val in [(UC_X86_REG_R12,aircraft),(UC_X86_REG_RBX,vector),(UC_X86_REG_RSI,slot*0x34c8),(UC_X86_REG_RDI,0),(UC_X86_REG_RBP,frame),(UC_X86_REG_RSP,STACK_TOP-0x2000)]:e.uc.reg_write(reg,val)
  e.run_range(0x141a88340,0x141a88735,max_instructions=1000000)
  count=(e.read_u64(vector+8)-points)//12
  assert count==(stations-1)*rings*4 if enabled and stations>=2 and rings else count==0
  assert not e.stubs
  output=struct.unpack('<'+'I'*(count*3),e.read(points,count*12)) if count else []
  print('P',enabled,stations,rings,slot,*[f'{bits(v):08x}' for v in [*origin,*grid]],'->',count,*[f'{v:08x}' for v in output])
 print('# cases',120)
if __name__=='__main__':main(sys.argv[1])
