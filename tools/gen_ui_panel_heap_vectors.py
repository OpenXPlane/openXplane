#!/usr/bin/env python3
"""Full common-panel constructors with mixed inline/heap string ownership.
Original bodies unchanged; allocator/free/CRT copy controlled. Corpus local-only.
"""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RAX

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x170);names=[e.alloc(32) for _ in range(3)];top=e.heap_top;poison=[0];events=[];allocs=[]
 def allocate(emu):
  size=emu.reg(UC_X86_REG_RCX);assert 0<size<0x20000
  p=emu.alloc(size);emu.write(p,bytes([poison[0]])*size);events.append(f'A:{size}:{p}:{emu.read(obj,0x170).hex()}');allocs.append((p,size));emu.uc.reg_write(UC_X86_REG_RAX,p)
 def free(emu):events.append(f'F:{emu.reg(UC_X86_REG_RCX)}:{emu.reg(UC_X86_REG_RDX)}:{emu.read(obj,0x170).hex()}')
 def copy(emu):
  dst,src,size=[emu.reg(r) for r in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8]]
  assert 0<size<0x20000;emu.write(dst,emu.read(src,size))
 e.stubs.update({0x1422e7bf0:allocate,0x1422e7c2c:free,0x1423c0280:copy})
 lengths=[0,1,15,16,20,22,23,31,32,63,255,4079,4080,4095,4096]
 profiles=[(n,lengths[(i+3)%len(lengths)],lengths[(i+7)%len(lengths)]) for i,n in enumerate(lengths)]+[(20,10,0),(20,10,15),(20,10,16)]
 print('# reference_sha256',REFERENCE_SHA256)
 print('# full common/wrapper/input/callback/string ctors unchanged; only allocator/free/CRT copy controlled; includes aligned >=4096-byte storage and forced heap short sources')
 count=0
 for sizes,seed,mode,flags in itertools.product(profiles,[0,0xa5,255],[0,1],[(0,0,1),(1,0,1),(255,128,255)]):
  e.heap_top=top;events.clear();allocs.clear();poison[0]=seed^0x5a;raws=[];backings=[]
  for j,n in enumerate(sizes):
   raw=bytearray([seed^0x33^j]*32);data=bytearray((65+(i+j)%26) for i in range(max(16,n+1)));data[n]=0
   if n>1:data[n//2]=0
   if n<=15 and mode==0:
    raw[:16]=bytes([seed^0x33^j])*16;raw[:n]=data[:n];raw[n]=0;cap=15
   else:
    cap=max(31,n|15)+(32 if mode else 0);bytes_=cap+1;large=bytes_>=4096;p=e.alloc(bytes_+(39 if large else 0));ptr=(p+39)&~31 if large else p
    e.write(p,bytes([poison[0]])*(bytes_+(39 if large else 0)))
    if large:e.write_u64(ptr-8,p)
    e.write(ptr,bytes(data));raw[:8]=ptr.to_bytes(8,'little');backings.append((p,bytes_+(39 if large else 0),e.read(p,bytes_+(39 if large else 0)).hex()))
   raw[16:24]=n.to_bytes(8,'little');raw[24:32]=cap.to_bytes(8,'little');raws.append(bytes(raw));e.write(names[j],bytes(raw))
  e.write(obj,bytes([seed])*0x170);result=e.call(0x14129c7a0,ints=[obj,*names],stack=list(flags),max_instructions=500000);assert result==obj
  for p,size,raw in backings:assert e.read(p,size).hex()==raw
  print('C',seed,','.join(map(str,flags)),*[s.hex() for s in raws],';'.join(f'{p}:{raw}' for p,size,raw in backings) or '-', '->',e.read(obj,0x170).hex(),*[e.read(p,32).hex() for p in names],';'.join(f'{p}:{size}:{e.read(p,size).hex()}' for p,size in allocs), '|'.join(events));count+=1
 print('# cases',count)
if __name__=='__main__':main(sys.argv[1])
