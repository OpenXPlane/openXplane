#!/usr/bin/env python3
"""Exact signed record-address arithmetic; no callee stubs, local corpus only."""
import hashlib,itertools,random,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RIP

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);table=e.alloc(8);rows=[]
 bases=[0,1,0x6f0000000000,0x7fffffffffffffff,0x8000000000000000,0xffffffffffffffff]
 indices=[0,1,20,77,88,89,0x7fffffff,0x80000000,0xfffffffe,0xffffffff]
 for kind,base,index,upper in itertools.product(range(2),bases,indices,[0,0x12345678,0xffffffff]):rows.append((kind,base,index|(upper<<32)))
 rng=random.Random(2026101802)
 for _ in range(512):rows.append((rng.randrange(2),rng.getrandbits(64),rng.getrandbits(64)))
 print('# reference_sha256',REFERENCE_SHA256)
 print('# complete original141a64250/141a64260, no stubs; computed RAX/table word unchanged; no dereference/bounds/lifecycle proof')
 for kind,base,index in rows:
  e.write_u64(table,base);result=e.call([0x141a64260,0x141a64250][kind],ints=[table,index]);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and e.read_u64(table)==base
  print('C',kind,base,index,'->',result)
 print('# cases',len(rows))
if __name__=='__main__':main(sys.argv[1])
