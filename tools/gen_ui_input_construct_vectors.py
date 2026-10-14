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
 e=Emulator(path);obj=e.alloc(0x128);name=e.alloc(32);top=e.heap_top;values=[];blocks=[];snapshots=[]
 def allocate(emu):
  size=emu.reg(UC_X86_REG_RCX);assert size in [24,40,48]
  snapshots.append(emu.read(obj,0x128).hex());p=emu.alloc(size);emu.write(p,bytes([values[2]])*size);blocks.append((p,size));emu.uc.reg_write(UC_X86_REG_RAX,p)
 e.stubs[0x1422e7bf0]=allocate
 print('# reference_sha256',REFERENCE_SHA256)
 print('# input ctor/callback ctor/string copy original; only allocator controlled; valid inline cap15 length0..15; poisoned padding')
 count=0
 for length in range(16):
  for seed in [0,0x55,0xa5,255]:
   values[:]=[length,seed,seed^0x5a];blocks.clear();snapshots.clear();e.heap_top=top
   raw=bytearray([seed^0x33]*32);raw[:length]=bytes(65+(i%26) for i in range(length));raw[length]=0;raw[16:24]=length.to_bytes(8,'little');raw[24:32]=(15).to_bytes(8,'little')
   e.write(obj,bytes([seed])*0x128);e.write(name,bytes(raw));result=e.call(0x1412a70a0,ints=[obj,name],max_instructions=100000);assert result==obj
   assert [size for _,size in blocks]==[40,24,40,48]
   print('C',length,seed,seed^0x5a,bytes(raw).hex(),'->',e.read(obj,0x128).hex(),e.read(name,32).hex(),*[f'{size}:{p}:{e.read(p,size).hex()}:{snap}' for (p,size),snap in zip(blocks,snapshots)]);count+=1
 print('# cases',count)
if __name__=='__main__':main(sys.argv[1])
