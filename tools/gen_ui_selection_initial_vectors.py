#!/usr/bin/env python3
"""Complete original selection callback, local synthetic corpus only."""
import hashlib,itertools,random,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from ui_selection_initial_harness import install
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RIP

def profile(i):
 tags=[255,0,1,2,3,4]
 return dict(seed=[0,0xa5,255][i%3],tag=tags[(i//8)%6],optional=[0,1,255][(i//3)%3],pending=[0,1,255][i%3],tail=[0,1,255][i%3],parent=0 if i%17==0 else 1,key=0x1e1b9,allow=[0,1,256,257][(i//3)%4],alias=i%3,float_alias=(i//3)%3,secondary=[0,1,256,257][i%4],force=i%3==1)
def main(path,native_compare=False):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x2d0);h=install(e,native_compare=native_compare);rows=[]
 for controlled,first,second,float_case,mutate in itertools.product([0,1],[0,1,256,257],[0,1,256,257],range(8),[0,1]):
  v=profile(len(rows));v.update(controlled=controlled,first=first,second=second,float=float_case,mutate=mutate);rows.append(v)
 for tag,alias,optional,mutate in itertools.product([255,0,1,2,3,4],range(3),[0,1],[0,1]):
  v=profile(len(rows));v.update(controlled=1,first=1,second=1,float=0,mutate=mutate,tag=tag,alias=alias,optional=optional,parent=1,allow=1);rows.append(v)
 rng=random.Random(0x1410fe)
 for _ in range(128):
  v=profile(len(rows));v.update(controlled=rng.randrange(2),first=rng.getrandbits(64),second=rng.getrandbits(64),float=rng.randrange(8),mutate=rng.randrange(2),tag=rng.choice([255,0,1,2,3,4]),parent=rng.randrange(2),key=rng.choice([0x1e1b9,0x10001e1b9,0,0xffffffffffffffff]),secondary=rng.getrandbits(64));rows.append(v)
 assert len(rows)==712
 print('# reference_sha256',REFERENCE_SHA256);print('# whole1410fe0f0; actual or explicitly controlled tag/payload helpers; record comparison '+('actual'if native_compare else'controlled')+'; float/finish/cleanup/secondary/reset controlled; no GPU proof')
 for i,v in enumerate(rows):
  h['prepare'](obj,i,v);e.call(0x1410fe0f0,ints=[obj+0x38,v['key'],0xffffffffffffffff],max_instructions=100000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and h['row'];print(h['row'])
 if native_compare:print('# actual record compare stats',h['compare_stats'])
 print('# cases',len(rows));print('# stats',h['stats']);print('# actual helper returns',h['totals'])
 receiver=e.alloc(0x90)
 for poison in[0,0xa5,0xff]:
  for tag in range(256):
   image=bytearray([poison])*0x90;image[0x68]=tag;e.write(receiver,image);e.uc.reg_write(UC_X86_REG_RAX,0xffffffffffffffff);a=e.call(0x1417fb370,ints=[receiver])&255;e.uc.reg_write(UC_X86_REG_RAX,0xffffffffffffffff);b=e.call(0x1417f8940,ints=[receiver]);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and e.read(receiver,len(image))==image;print('Q',receiver,poison,tag,'->',a,b)
 print('# helper_pairs',768)
if __name__=='__main__':main(sys.argv[1], '--native-compare' in sys.argv[2:])
