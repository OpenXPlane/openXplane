#!/usr/bin/env python3
"""Complete original1417fb320, every byte with three poisoned receiver images."""
import hashlib,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RIP

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);receiver=e.alloc(0x600)
 print('# reference_sha256',REFERENCE_SHA256)
 print('# complete original read-only1417fb320, full RAX and receiver unchanged; producer/lifetime/semantic meaning unproven')
 for poison in [0,0xa5,0xff]:
  for value in range(256):
   image=bytearray([poison])*0x600;image[0x525]=value;e.write(receiver,image);e.uc.reg_write(UC_X86_REG_RAX,0xffffffffffffffff)
   result=e.call(0x1417fb320,ints=[receiver]);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and e.read(receiver,len(image))==image
   print('C',receiver,poison,value,'->',result)
 print('# cases',3*256)
if __name__=='__main__':main(sys.argv[1])
