#!/usr/bin/env python3
"""Original full global interface ctor/destructor; only sized free endpoint controlled.
Valid stable chains, fresh construction, single destruction per node.
"""
import hashlib,sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RDX

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);top=e.heap_top;nodes=[];log=[]
 def idx(p):return -1 if p==0 else nodes.index(p)
 def free(emu):
  assert emu.reg(UC_X86_REG_RDX)==24
  log.append(f'F:{idx(emu.reg(UC_X86_REG_RCX))}:24:{idx(emu.read_u64(0x14588f558))}')
 e.stubs[0x1422e7c2c]=free
 print('# reference_sha256',REFERENCE_SHA256)
 print('# complete ctor/destructor original; sized free controlled; valid stable chains; detached link memory retained by controlled free')
 count=0
 for size in range(5):
  for added in [1,2]:
   order=list(range(size));new=list(range(size,size+added))
   for position in range(size+added):
    for flags in [0,1,2,255,256,257]:
     e.heap_top=top;nodes[:]=[e.alloc(24) for _ in range(6)];log.clear();e.write_u64(0x14588f558,0)
     for p in nodes:e.write(p,bytes([0xa5])*24)
     for i in reversed(order):assert e.call(0x1410c7bb0,ints=[nodes[i]],max_instructions=100000)==nodes[i];e.write_u64(nodes[i],0x1426067c8)
     for i in new:assert e.call(0x1410c7bb0,ints=[nodes[i]],max_instructions=100000)==nodes[i];e.write_u64(nodes[i],0x1426067c8)
     active=new[::-1]+order;target=active[position]
     assert e.call(0x1410c8d30,ints=[nodes[target],flags],max_instructions=100000)==nodes[target]
     print('C',size,added,position,flags,'->',idx(e.read_u64(0x14588f558)),','.join(str(e.read_u64(nodes[i])) for i in range(size+added)),','.join(str(idx(e.read_u64(nodes[i]+8))) for i in range(size+added)),','.join(str(idx(e.read_u64(nodes[i]+16))) for i in range(size+added)),*log);count+=1
 print('# cases',count)
if __name__=='__main__':main(sys.argv[1])
