#!/usr/bin/env python3
"""Original append + growth, numeric corpus local, controlled leaf bodies."""
import hashlib,itertools,random,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from ui_string_append_harness import install,APPEND,GROW
from unicorn.x86_const import UC_X86_REG_RIP

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(128);h=install(e);rows=[]
 for cap,choice,count,alias,mutate in itertools.product([15,31,63,4094,4095,8191],range(4),[0,1,2,16,33],range(4),range(2)):
  length=[0,1,max(0,cap-1),cap][choice];rows.append(('A',cap,length,count,count,alias,mutate))
 assert len(rows)==960
 for cap,choice,amount,variant,alias,mutate in itertools.product([0,15,31,4094,4095,8191,(1<<64)-1,(1<<63)-1],range(3),[0,1,2,16,33],range(3),range(4),range(2)):
  length=[0,1,min(cap,15)][choice];count=[amount,0,1][variant];rows.append(('G',cap,length,amount,count,alias,mutate))
 rng=random.Random(140624)
 for _ in range(96):
  mode=rng.choice(['A','G']);cap=rng.choice([0,15,31,63,4094,4095,8191]);length=rng.randrange(min(cap,64)+1);count=rng.randrange(34);rows.append((mode,cap,length,count,count,rng.randrange(4),rng.randrange(2)))
 assert len(rows)==3936
 print('# reference_sha256',REFERENCE_SHA256);print('# whole append/growth normal-return callers; allocation/move/free controlled; synthetic metadata and alias/mutation stress; no allocator ownership or exception proof')
 for i,(mode,cap,length,amount,count,alias,mutate)in enumerate(rows):
  args=[obj,0,count]if mode=='A'else[obj,amount,[0,1,255,0x123456789abcdef0][i%4],0,count]
  h['prepare'](mode,args,i,dict(seed=[0,0xa5,255][i%3],length=length,capacity=cap,alias=alias,mutate=mutate))
  e.call(APPEND if mode=='A'else GROW,ints=h['args'][:4],stack=h['args'][4:],max_instructions=10000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and h['row'];print(h['row'])
 print('# cases',len(rows));print('# stats',h['stats'])
if __name__=='__main__':main(sys.argv[1])
