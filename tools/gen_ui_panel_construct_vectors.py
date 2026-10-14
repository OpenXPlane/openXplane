#!/usr/bin/env python3
"""Original input-node constructor incl callback ctor and inline string copy.
Allocator controlled; bytes poisoned to identify retained padding/payload.
"""
import hashlib,sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RAX

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x170);names=[e.alloc(32) for _ in range(3)];top=e.heap_top;values=[];blocks=[];snapshots=[]
 def allocate(emu):
  size=emu.reg(UC_X86_REG_RCX);assert size in [24,40,48]
  snapshots.append(emu.read(obj,0x170).hex());p=emu.alloc(size);emu.write(p,bytes([values[2]])*size);blocks.append((p,size));emu.uc.reg_write(UC_X86_REG_RAX,p)
 e.stubs[0x1422e7bf0]=allocate
 print('# reference_sha256',REFERENCE_SHA256)
 print('# panel/common/input/callback/string constructors original; only allocator controlled; valid inline cap15 length0..15; poisoned padding')
 count=0
 for length in range(16):
  for seed in [0,0x55,0xa5,255]:
   for flags in [(0,0,1),(255,255,255),(1,0,255),(0,1,0)]:
    values[:]=[length,seed,seed^0x5a];blocks.clear();snapshots.clear();e.heap_top=top;raws=[]
    for j,n in enumerate([length,(length+5)%16,(length+9)%16]):
     raw=bytearray([seed^0x33^j]*32);raw[:n]=bytes(65+(j+i)%26 for i in range(n));raw[n]=0;raw[16:24]=n.to_bytes(8,'little');raw[24:32]=(15).to_bytes(8,'little');raws.append(bytes(raw));e.write(names[j],bytes(raw))
    e.write(obj,bytes([seed])*0x170);result=e.call(0x14129c7a0,ints=[obj,*names],stack=list(flags),max_instructions=100000);assert result==obj
    assert [size for _,size in blocks]==[40,24,40,48]
    print('C',length,seed,seed^0x5a,','.join(map(str,flags)),*[raw.hex() for raw in raws],'->',e.read(obj,0x170).hex(),*[e.read(p,32).hex() for p in names],*[f'{size}:{p}:{e.read(p,size).hex()}:{snap}' for (p,size),snap in zip(blocks,snapshots)]);count+=1
 print('# cases',count)
if __name__=='__main__':main(sys.argv[1])
