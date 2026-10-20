#!/usr/bin/env python3
"""Complete original finish caller on synthetic state. Numeric corpus stays local."""
import hashlib,itertools,random,sys
import unicorn
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from ui_selection_finish_harness import install,FP
from unicorn.x86_const import UC_X86_REG_RIP

def profile(i):
 layout=i%3
 return dict(seed=[0,0xa5,255][i%3],ready1=0,ready2=0,token=0,catalog=i%4 if i%4<3 else 3,fp=i%len(FP),different=[0,1,256,257][i%4],float_alias=(i//3)%3,items=[0,2,2][layout],capacity=[8,8,2][layout],mutate=i%2,parent=int(i%7!=0),secondary_parent=i%2,pending=[0,1,255][i%3],engine=[0,1,255][i%3],index=[0,20][i%2],equal=(i//2)%2,secondary=[0,1,256,257][(i//4)%4],denom=[0x4ba83308,0,0x80000000,0x7f800000,0x7fc12345,0x7fa00001][(i//2)%6])
def main(path,native_compare=False):
 assert unicorn.__version__=='2.1.4','raw caller oracle requires the audited Unicorn2.1.4 NaN policy'
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x2d0);h=install(e,native_compare=native_compare);rows=[]
 for ready1,ready2,token,catalog,layout,fp in itertools.product([0,255],[0,255],[0,1],[0,1,3],range(3),[0,3,6,8,10,11]):
  v=profile(len(rows));v.update(ready1=ready1,ready2=ready2,token=token,catalog=catalog,items=[0,2,2][layout],capacity=[8,8,2][layout],fp=fp);rows.append(v)
 rng=random.Random(14113)
 for _ in range(128):
  v=profile(len(rows));v.update(ready1=rng.choice([0,1,256,257]),ready2=rng.choice([0,1,256,257]),token=rng.randrange(2),catalog=rng.choice([0,1,3]),fp=rng.randrange(len(FP)),different=rng.getrandbits(64),mutate=rng.randrange(2),parent=rng.randrange(2));rows.append(v)
 assert len(rows)==560
 print('# reference_sha256',REFERENCE_SHA256);print('# whole14113d660; real gate/query/primary/indexed getters; record/config/float/CRT/growth/output effects controlled, no GPU/ownership proof')
 for i,v in enumerate(rows):
  h['prepare'](obj,i,v);e.call(0x14113d660,ints=[obj],max_instructions=100000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and h['row'];print(h['row'])
 if native_compare:print('# actual record compare stats',h['compare_stats'])
 print('# cases',len(rows));print('# stats',h['stats']);print('# actual returns',h['totals'])
if __name__=='__main__':main(sys.argv[1], '--native-compare' in sys.argv[2:])
