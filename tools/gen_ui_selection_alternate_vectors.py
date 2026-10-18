#!/usr/bin/env python3
"""Whole alternate caller; local synthetic numeric evidence, not game data."""
import hashlib,itertools,random,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from ui_selection_alternate_harness import install
from unicorn.x86_const import UC_X86_REG_RIP

def profile(i,major,tag,optional,result,scenario):
 return dict(seed=[0,0xa5,255][i%3],active=0,index=[5,77,88,0xffffffff][i%4],major=major,tag=tag,optional=optional,result=result,ready=255 if scenario else 0,count=[0,0,1,3,3,3][scenario],capacity=[0,0,1,4,511,512][scenario],found=scenario>=4,null=i%2,zero1=255 if scenario&1 else 0,zero2=255 if scenario&2 else 0,pref=[0,1,128,255,0x80000000][i%5],flag=[0,1,0x80000000,0xffffffff][i//3%4],alias=i%3,mutate=i%2)
def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x2d0);h=install(e);rows=[]
 for args in itertools.product([0,1],[0,1,2,3,4,255],[0,1],range(3),range(6)):rows.append(profile(len(rows),*args))
 rng=random.Random(2026101805)
 for _ in range(96):
  v=profile(len(rows),rng.choice([0,1,0x80000000]),rng.choice([0,1,2,3,4,255]),rng.choice([0,1,255]),rng.randrange(3),rng.randrange(6));v.update(mutate=rng.randrange(2),zero1=rng.choice([0,1,255,256]),zero2=rng.choice([0,1,255,256]),ready=rng.choice([0,1,255,256]),pref=rng.getrandbits(32));rows.append(v)
 print('# reference_sha256',REFERENCE_SHA256);print('# whole original14111bb90 + real settings getter/temp destructor; data/search/cleanup effects controlled; initial scratch/home bytes supplied; valid tags/aligned mapped profiles, no GPU/registry/ownership/fault proof')
 for i,v in enumerate(rows):
  e.write(obj,bytes([v['seed']])*0x2d0);h.prepare(obj,i,v);e.call(0x14111bb90,ints=[obj],max_instructions=100000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and h.row;print(h.row)
 print('# cases',len(rows));print('# actual returns',h.total_returns,'settings',h.get_totals,'temp destructors',h.temp_dtor_calls)
if __name__=='__main__':main(sys.argv[1])
