#!/usr/bin/env python3
"""Whole original membership and Favorites tail, synthetic ordered trees only."""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_RIP

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);record=e.alloc(0x198);head=e.alloc(40);nodes=[e.alloc(40)for _ in range(20)];singleton=e.alloc(32)
 ranges={record:0x198,head:40,**{p:40 for p in nodes},singleton:32};events=[];case={};stats=dict(cases=0,singleton=0,favorites=0)
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(ranges.items()))
 def singleton_call(emu):
  initial=snap();v=case['i'];value=singleton+v%16;stats['singleton']+=1
  e.write_u64(record+0x180,0xfedcba9876543210);e.write(singleton,bytes([v%256])*32)
  events.append(f'I~{0x1414cc7f0}~{initial}~{snap()}~{value}');e.uc.reg_write(UC_X86_REG_RAX,value)
 def favorites_call(emu):
  args=[e.reg(UC_X86_REG_RCX),e.reg(UC_X86_REG_RDX)];assert args==[singleton+case['i']%16,record];initial=snap();stats['favorites']+=1
  value=[0,1,0xff,0x100,0x100000000,0xffffffffffffffff][case['i']%6];e.write(record+0x188,value.to_bytes(8,'little'))
  events.append(f'I~{0x1414cc820},{args[0]},{args[1]}~{initial}~{snap()}~{value}');e.uc.reg_write(UC_X86_REG_RAX,value)
 e.stubs[0x1414cc7f0]=singleton_call;e.stubs[0x1414cc820]=favorites_call
 indices=[-2147483648,-1,*range(14),2147483647,0x80000000,0xffffffff,0x100000000,0x100000002,0x10000000b,0xffffffffffffffff]
 print('# reference_sha256',REFERENCE_SHA256);print('# actual whole membership body; Favorites singleton/handler controlled; no tree producer, faults or ownership proof')
 for i,(mask,shape,nil,index,seed)in enumerate(itertools.product([0,1,4,0x800,0x7ff,0xfff,0x555,0xaaa],range(3),[1,255],indices,[0,0xa5,255])):
  case['i']=i;events.clear()
  for p,n in ranges.items():e.write(p,bytes([seed])*n)
  keys=sorted([-2147483648,-1,*[k for k in range(12)if mask&(1<<k)],12,2147483647]);pool=iter(nodes)
  def branch(items,parent):
   if not items:return head
   k=[len(items)//2,0,len(items)-1][shape];p=next(pool);e.write(p,bytes(40));e.write_u64(p+8,parent);e.write_u32(p+0x1c,items[k]&0xffffffff)
   e.write_u64(p,branch(items[:k],p));e.write_u64(p+0x10,branch(items[k+1:],p));return p
  e.write(head,bytes(40));e.write(head+0x19,bytes([nil]));root=branch(keys,head);e.write_u64(head+8,root if mask else head);e.write_u64(record+0x180,head)
  initial=snap();result=e.call(0x141a400f0,ints=[record,index],max_instructions=10000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR;stats['cases']+=1
  print(' '.join(map(str,['M',record,index,initial,'->',snap(),result,'|'.join(events)or'-'])))
 print('# stats',stats)
if __name__=='__main__':main(sys.argv[1])
