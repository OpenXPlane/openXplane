#!/usr/bin/env python3
"""Original derived class string + label; executable and numeric corpus stay local."""
import hashlib,itertools,random,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from ui_record_class_string_harness import install,BUILD,LABEL
from unicorn.x86_const import UC_X86_REG_RIP

def main(path,native_strings=False):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);record=e.alloc(0x198);out_buffer=e.alloc(512);h=install(e,native_strings=native_strings);rows=[]
 for mask,shape,mutate,out_mode,tail in itertools.product([0,1,2,1<<8,1<<11,0x7ff,0xfff,0x555,0xaaa],range(3),range(2),range(4),range(4)):
  i=len(rows);rows.append(dict(mask=mask,other=(mask^0x555)&0xfff,shape=shape,mutate=mutate,out_mode=out_mode,tail=tail,seed=[0,0xa5,255][i%3],dirty=i%2,grow_alias=(i//2)%3))
 rng=random.Random(1413)
 for _ in range(96):rows.append(dict(mask=rng.randrange(4096),other=rng.randrange(4096),shape=rng.randrange(3),mutate=rng.randrange(2),out_mode=rng.randrange(4),tail=rng.randrange(4),seed=rng.randrange(256),dirty=rng.randrange(2),grow_alias=rng.randrange(3)))
 assert len(rows)==960
 print('# reference_sha256',REFERENCE_SHA256);print('# actual whole derived string + label + reached shrink branch; allocation/free controlled; '+('actual append/growth with controlled memmove' if native_strings else 'controlled append/growth')+'; no GPU, producer or ownership proof')
 for i,v in enumerate(rows):
  if native_strings:v.update(tail=0,grow_alias=0,dirty=0)
  out=record+0x40 if v['out_mode']==3 else out_buffer+[0,15,255][v['out_mode']]
  # Declare the full buffer to preserve out-pointer low-byte variations.
  owned=None;h['prepare'](record,out,i,v);h['ranges'][out_buffer]=512
  if out!=out_buffer and out in h['ranges']and out!=record+0x40:h['ranges'].pop(out)
  if out==record+0x40:h['ranges'].pop(out)
  e.call(BUILD,ints=[record,out],max_instructions=100000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and h['row'];print(h['row'])
 indices=[-1,*range(13),0x7fffffff,0x80000000,0xffffffff,0x100000000,0x100000001,0xffffffffffffffff]
 assert len(indices)==20
 for seed,index in itertools.product([0,0xa5,255],indices):
  h['prepare'](record,out_buffer,seed,dict(seed=seed,mutate=seed%2),mode='L',index=index);e.call(LABEL,ints=[out_buffer,index]);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and h['row'];print(h['row'])
 print('# derived_cases',len(rows));print('# label_cases',60);print('# stats',h['stats'])
 if native_strings:print('# string_stats',h['strings']['stats'])
if __name__=='__main__':main(sys.argv[1],'--native-strings'in sys.argv[2:])
