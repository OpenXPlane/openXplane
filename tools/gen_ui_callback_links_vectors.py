#!/usr/bin/env python3
"""Whole original callback connection ctor/copy/bind/base-dtor sequences.
Only allocator/free are controlled. Compares logical links, parameters and
allocation/free sizes/counts; red-black layout/free-pointer order is not exported.
"""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);slots=[e.alloc(0x68) for _ in range(8)];top=e.heap_top;events=[];live={};poison=[0]
 def allocate(emu):
  size=emu.reg(UC_X86_REG_RCX);assert size==40
  p=emu.alloc(size);emu.write(p,bytes([poison[0]])*size);events.append('A');emu.uc.reg_write(UC_X86_REG_RAX,p)
 def free(emu):assert emu.reg(UC_X86_REG_RDX)==40;events.append('F')
 e.stubs.update({0x1422e7bf0:allocate,0x1422e7c2c:free})
 def links(p):
  sentinel=e.read_u64(p+8);seen=set();keys=[]
  def walk(node):
   if e.read(node+25,1)[0]:assert node==sentinel;return
   assert node not in seen;seen.add(node);walk(e.read_u64(node));keys.append(e.read_u64(node+32));walk(e.read_u64(node+16))
  walk(e.read_u64(sentinel+8));assert keys==sorted(set(keys));assert len(keys)==e.read_u64(p+16)
  return keys
 def snapshot():
  out=[]
  for p,kind in sorted(live.items()):
   peers=links(p);assert all(x in live for x in peers)
   if kind=='W':assert e.read_u64(p+96)==0;args=[e.read_u64(p+24),e.read_u64(p+32)]
   else:args=[0,0]
   out.append(':'.join(map(str,[p,kind,*args,','.join(map(str,peers)) or '-'])))
  return ';'.join(out) or '-'
 orders=[[2,4,0,7,3,1,5,6],list(range(8)),list(reversed(range(8))),[0,4,1,5,2,6,3,7],[7,0,5,1,6,2,4,3],[3,5,7,2,6,4,1,0]]
 print('# reference_sha256',REFERENCE_SHA256)
 print('# unchanged base/wrapper/copy/registration/RB insert+erase/base teardown; only allocator/free controlled; null custom callable; stable nonself links')
 cases=steps=0
 for seed,pair,order,reverse,parent in itertools.product([0,0xa5,255],[(0,0),(3,2),(2**64-1,2**63+7)],range(6),[0,1],[-1,0,1]):
  e.heap_top=top;live.clear();poison[0]=seed^0x5a;ids=slots[::-1] if reverse else slots
  for p in ids:e.write(p,bytes([seed])*0x68)
  print('C',seed,','.join(map(str,ids)))
  ops=[('B',0),('B',1),('C',2,parent,*pair),('C',3,0,pair[1],pair[0]),('L',2,1),('P',4,2),('P',5,4),('C',6,1,*pair),('C',7,2,*pair),('L',2,1)]+[('D',i) for i in orders[order]]
  for op in ops:
   events.clear();kind,i,*args=op;p=ids[i]
   if kind=='B':assert e.call(0x1410c7b50,ints=[p])==p;live[p]='B'
   elif kind=='C':
    parent,f,s=args;assert e.call(0x1410c7d20,ints=[p,0 if parent<0 else ids[parent],f,s])==p;live[p]='W'
   elif kind=='P':assert e.call(0x1410c7bf0,ints=[p,ids[args[0]]])==p;live[p]='W'
   elif kind=='L':e.call(0x1410c8de0,ints=[p,ids[args[0]]])
   elif kind=='D':e.call(0x1410c8430,ints=[p]);del live[p]
   print('T',':'.join(map(str,op)),'->',snapshot(),','.join(events) or '-');steps+=1
  assert not live;cases+=1
 print('# cases',cases,'steps',steps)
if __name__=='__main__':main(sys.argv[1])
