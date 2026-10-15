#!/usr/bin/env python3
"""Execute unchanged concrete menu prefix, stopping before singleton dialogs.
Only translation result, allocator, sized free and CRT copy are controlled.
Numeric corpus is local-only; no executable/asset bytes are exported.
"""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RAX,UC_X86_REG_RIP
START,STOP,HEAD=0x140ecb340,0x140ecb592,0x14588f558

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x300);parent=e.alloc(0x100);nodes=[e.alloc(24) for _ in range(2)];top=e.heap_top
 state={};allocs=[];events=[]
 def translate(emu):
  dst=emu.reg(UC_X86_REG_RCX);assert emu.read(emu.reg(UC_X86_REG_RDX),len(b'/_lookup_12_/help/main/\0'))==b'/_lookup_12_/help/main/\0'
  emu.write(dst,state['text']);emu.uc.reg_write(UC_X86_REG_RAX,dst)
 def allocate(emu):
  size=emu.reg(UC_X86_REG_RCX);assert 0<size<0x1000
  snap=emu.read(obj,0x300).hex();head=emu.read_u64(HEAD)
  p=0 if len(allocs)==4 and state['null'] else emu.alloc(size)
  if p:emu.write(p,bytes([state['poison']])*size)
  allocs.append((size,p,snap,head));events.append(f'A:{size}:{p}');emu.uc.reg_write(UC_X86_REG_RAX,p)
 def free(emu):events.append(f'F:{emu.reg(UC_X86_REG_RCX)}:{emu.reg(UC_X86_REG_RDX)}')
 def copy(emu):
  dst,src,n=[emu.reg(r) for r in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8]]
  if n:emu.write(dst,emu.read(src,n))
  events.append(f'M:{n}')
 e.stubs.update({0x140c238a0:translate,0x1422e7bf0:allocate,0x1422e7c2c:free,0x1423c0280:copy})
 print('# reference_sha256',REFERENCE_SHA256)
 print('# original concrete prefix START=0x140ecb340 STOP=0x140ecb592 asserted RIP; NOT normal ctor return; translation/allocator/free/CRT controlled')
 count=0
 for profile,heads,seed,flag,length,null in itertools.product([(0,0,0),(1,0,0),(1,2,1),(1,2,2),(1,8,3)],[0,2],[0,0xa5,255],[0,1,128,255],[0,15],[0,1]):
  mapped,cap,n=profile;e.heap_top=top;allocs.clear();events.clear();poison=seed^0x5a
  text=bytearray([seed^0x33]*32);text[:length]=bytes(65+i%26 for i in range(length));text[length]=0;text[16:24]=length.to_bytes(8,'little');text[24:32]=(15).to_bytes(8,'little')
  state.update(poison=poison,text=bytes(text),null=null)
  e.write(obj,bytes([seed])*0x300);e.write(parent,bytes(0x100))
  old=0 if not cap else e.alloc(cap*8)
  if old:e.write(old,bytes([poison])*cap*8)
  for i in range(n):e.write_u64(old+i*8,100+i)
  for off,value in [(0xd0,old),(0xd8,old+n*8),(0xe0,old+cap*8)]:e.write_u64(parent+off,value)
  for i,node in enumerate(nodes):
   for off,val in [(0,0x1426401f8),(8,nodes[i-1] if i else 0),(16,nodes[i+1] if i+1<heads else 0)]:e.write_u64(node+off,val)
  e.write_u64(HEAD,nodes[0] if heads else 0)
  e.call(START,ints=[obj,parent if mapped else 0,flag],until=STOP,max_instructions=200000)
  assert e.reg(UC_X86_REG_RIP)==STOP
  begin,end,last=[e.read_u64(parent+off) for off in [0xd0,0xd8,0xe0]]
  chain=','.join(f'{node}:{e.read(node,24).hex()}' for node in nodes[:heads]) or '-'
  storage=e.read(begin,last-begin).hex() if begin else '-'
  print('C',seed,flag,mapped,cap,n,heads,null,obj,parent,old,','.join(map(str,nodes)),bytes(text).hex(),'->',e.read(obj,0x300).hex(),e.read_u64(HEAD),chain,begin,(last-begin)//8,(end-begin)//8,storage,
   '|'.join(f'{size}:{p}:{snap}:{head}:{e.read(p,size).hex() if p else "-"}' for size,p,snap,head in allocs),','.join(events));count+=1
 print('# cases',count)
if __name__=='__main__':main(sys.argv[1])
