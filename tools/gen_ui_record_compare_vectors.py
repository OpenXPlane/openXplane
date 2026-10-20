#!/usr/bin/env python3
"""Synthetic comparison callers; raw numerical corpus stays local."""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from ui_record_compare_harness import install,EQUAL,DIFFERENT
from unicorn.x86_const import UC_X86_REG_RIP

def main(path,native_derived=False):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);left=e.alloc(0x198);right=e.alloc(0x198);h=install(e,native_derived=native_derived);count=0
 print('# reference_sha256',REFERENCE_SHA256);print('# whole equality/negation callers; derived '+('actual with actual label/shrink'if native_derived else'controlled')+'; other leaf effects controlled; no ownership, exception or GPU proof')
 for inverted,stage,kind,temp_kind,alias in itertools.product(range(2),range(10),[0,1,2,5,6],[1,3,4,5,6],range(3)):
  v=dict(text=[b'value',b'va\0ue',b'\xff\x80\0\xa0\xfe',b''][count%4],seed=[0,0xa5,255][count%3],stage=stage,kind=kind,temp_kind=temp_kind,alias=alias,length=count%2,override=[None,0,1,0x100000000,0xffffffff][(count//3)%5],mutate=count%2)
  h['prepare'](left,right,count,v);result=e.call(DIFFERENT if inverted else EQUAL,ints=[left,right],max_instructions=10000)
  assert e.reg(UC_X86_REG_RIP)==RET_ADDR and h['row'];print(h['row']);count+=1
 assert count==1500
 if native_derived:print('# actual class-string stats',h['class_stats'])
 print('# cases',count);print('# stats',h['stats'])
if __name__=='__main__':main(sys.argv[1], '--native-derived' in sys.argv[2:])
