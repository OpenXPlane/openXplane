#!/usr/bin/env python3
"""Complete original member ctor/capture/registration/vector growth.
Allocator/free/CRT-copy controlled; small valid vector states.
"""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RAX

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x58);parent=e.alloc(0x100);top=e.heap_top;log=[];allocations=[];poison=[0]
 def allocate(emu):
  size=emu.reg(UC_X86_REG_RCX);assert 0<size<0x1000
  p=emu.alloc(size);emu.write(p,bytes([poison[0]])*size);allocations.append((p,size));log.append(f'A:{size}');emu.uc.reg_write(UC_X86_REG_RAX,p)
 def free(emu):log.append(f'F:{emu.reg(UC_X86_REG_RDX)}')
 def copy(emu):
  dst,src,size=[emu.reg(r) for r in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8]]
  if size:emu.write(dst,emu.read(src,size))
  log.append(f'M:{size}')
 e.stubs.update({0x1422e7bf0:allocate,0x1422e7c2c:free,0x1423c0280:copy})
 print('# reference_sha256',REFERENCE_SHA256)
 print('# original member/capture/temporary destructor/registration/growth bodies unchanged; allocator/free/CRT copy controlled; small valid vectors')
 count=0
 for variant,mapped,cap,seed in itertools.product(range(2),range(2),[0,1,2,3,4,8],[0,0xa5,255]):
  for length in sorted(set([0,cap,max(0,cap-1)])):
   e.heap_top=top;log.clear();allocations.clear();poison[0]=seed^0x5a;e.write(obj,bytes([seed])*0x58);e.write(parent,bytes(0x100))
   old=0 if cap==0 else e.alloc(cap*8)
   if old:e.write(old,bytes([poison[0]])*(cap*8))
   values=[100+i for i in range(length)]
   for i,value in enumerate(values):e.write_u64(old+8*i,value)
   e.write_u64(parent+0xd0,old);e.write_u64(parent+0xd8,old+length*8);e.write_u64(parent+0xe0,old+cap*8)
   result=e.call(0x140ec3330 if variant else 0x140ec3280,ints=[obj,parent if mapped else 0],max_instructions=100000);assert result==obj
   begin,end,last=[e.read_u64(parent+off) for off in [0xd0,0xd8,0xe0]];capacity=(last-begin)//8
   out=[e.read_u64(p) for p in range(begin,end,8)];storage='-' if begin==0 else e.read(begin,capacity*8).hex()
   allocated='-' if not allocations else ','.join(f'{p}:{n}' for p,n in allocations)
   print('C',variant,mapped,cap,length,seed,obj,parent,old,'->',e.read(obj,0x58).hex(),capacity,begin,','.join(map(str,out)) or '-',storage,allocated,*log);count+=1
 print('# cases',count)
if __name__=='__main__':main(sys.argv[1])
