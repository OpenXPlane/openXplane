#!/usr/bin/env python3
"""Whole payload/string/vector moves and vector growth. Numeric corpus local only."""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RIP

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);objects=[e.alloc(0xe0) for _ in range(2)];top=e.heap_top;blocks=[];events=[];v={}
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}' for p,n in sorted(blocks))
 def add(n,fill):p=e.alloc(n);e.write(p,bytes([fill])*n);blocks.append((p,n));return p
 def allocate(emu):
  n=emu.reg(UC_X86_REG_RCX);before=snap()
  if v['mutation']:emu.write_u32(v['source']+0x60,0x7fc12345)
  after=snap();p=add(n,v['seed']^0x5a);events.append(f'A~{n},{p}~{before}~{after}~{e.read(p,n).hex()}');emu.uc.reg_write(UC_X86_REG_RAX,p)
 def free(emu,sized):
  p=emu.reg(UC_X86_REG_RCX);n=str(emu.reg(UC_X86_REG_RDX)) if sized else '-';before=snap()
  if v['mutation']:emu.write_u32(v['source']+0x60,0xff812345)
  events.append(f'F~{p},{n}~{before}~{snap()}')
 def copy(emu):
  d,s,n=[emu.reg(r) for r in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8]]
  if n:emu.write(d,emu.read(s,n))
  emu.uc.reg_write(UC_X86_REG_RAX,d)
 e.stubs.update({0x142301130:allocate,0x142301120:lambda emu:free(emu,False),0x1422e7c2c:lambda emu:free(emu,True),0x1423c0280:copy})
 def string(obj,mode,j,seed):
  raw=bytearray([seed^0x33]*32);large=mode==3;heap=(mode==1 and j==0)or(mode==2 and j==1)or large
  cap=4095 if large else 31 if heap else 15;length=(17+j*6) if large else 20 if heap else 5+j*2
  data=bytearray((k*7+seed+j)%256 for k in range(max(length+1,16)));data[length]=0
  if heap:
   base=add(cap+1+(39 if large else 0),seed^0x66);p=(base+39)&~31 if large else base
   if large:e.write_u64(p-8,base)
   e.write(p,data);raw[:8]=p.to_bytes(8,'little')
  else:raw[:16]=data[:16]
  raw[16:24]=length.to_bytes(8,'little');raw[24:32]=cap.to_bytes(8,'little');e.write(obj+0x30,bytes(raw))
 def vector(obj,heap,length,cap,seed,null=False):
  p=0 if null else add(cap*4,seed^0x77) if heap else obj+0xc0
  for o,x in [(0xa8,p),(0xb0,p+length*4),(0xb8,p+cap*4)]:e.write_u64(obj+o,x)
  if p and length:e.write(p,bytes((seed+k*9)%256 for k in range(length*4)))
 profiles=[(False,0,8,False,0,8),(False,5,8,False,2,8),(False,2,8,False,6,8),(False,0,1,False,6,8),(True,1,4,False,8,8),(False,3,8,True,5,12),(True,2,10,True,7,12),(False,4,8,False,4,8),(True,0,0,False,0,8),(True,2,10,True,0,0)]
 print('# reference_sha256',REFERENCE_SHA256)
 print('# whole1410faec0 + original1405de290/1410fa780/1408b0660; only successful allocator, free, standard memcpy controlled; full mapped object/backing bytes and allocator/free snapshots; no corrupt/partial-overlap/allocation-failure profile')
 rows=list(itertools.product([0,0xa5],[0,1,255],[0,1,255],range(4),range(10)))
 for i,(seed,df,sf,sm,vp) in enumerate(rows):
  e.heap_top=top;blocks[:]=[(p,0xe0)for p in objects];events.clear()
  for j,p in enumerate(objects):
   e.write(p,bytes((seed+j*3+k)%256 for k in range(0xe0)));string(p,sm,j,seed);e.write(p+0x58,bytes([df if j==0 else sf]))
  dh,dl,dc,sh,sl,sc=profiles[vp];vector(objects[0],dh,dl,dc,seed);vector(objects[1],sh,sl,sc,seed^0x11,null=vp==9)
  source=objects[0] if vp==7 else objects[1];v.update(seed=seed,source=source,mutation=i%2);initial=snap();events.clear()
  result=e.call(0x1410faec0,ints=[objects[0],source],max_instructions=100000);assert result==objects[0] and e.reg(UC_X86_REG_RIP)==RET_ADDR
  print('C',objects[0],source,initial,'->',snap(),'|'.join(events)or'-')
 print('# cases',len(rows))
if __name__=='__main__':main(sys.argv[1])
