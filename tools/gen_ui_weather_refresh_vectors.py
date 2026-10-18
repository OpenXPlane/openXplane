#!/usr/bin/env python3
"""Complete original weather refresh on bounded synthetic state; local corpus."""
import hashlib,itertools,random,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from ui_weather_refresh_harness import install
from unicorn.x86_const import UC_X86_REG_RIP

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x3e8);h=install(e);rows=[]
 for cli,engine,active,available,zero,mode in itertools.product(range(3),[0,255],[0,7],[0,1],range(4),range(3)):
  i=len(rows);rows.append(dict(seed=[0,0xa5,255][i%3],cli1=255 if cli==1 else 0,cli2=255 if cli==2 else 0,engine=engine,active=active,available=available,zero1=255 if zero&1 else 0,zero2=255 if zero&2 else 0,mode7=255 if mode==1 else 0,mode3=255 if mode==2 else 0,index=[5,77,88,0xffffffff][i%4],alias=i%4,altitude=[0x80000000,0x41480000,0x7fc12345][i%3],listeners=i%4,mutate=i%2,rebind=0x55556666 if i%3==0 else 0))
 rng=random.Random(2026101803)
 for _ in range(96):
  rows.append(dict(seed=rng.choice([0,0xa5,255]),cli1=rng.choice([0,1,255,256]),cli2=rng.choice([0,1,255,256]),engine=rng.choice([0,1,255]),active=rng.choice([0,7]),available=rng.randrange(2),zero1=rng.choice([0,1,255,256]),zero2=rng.choice([0,1,255,256]),mode7=rng.choice([0,1,255,256]),mode3=rng.choice([0,1,255,256]),index=rng.choice([5,77,88,0xffffffff]),alias=rng.randrange(4),altitude=rng.getrandbits(32),listeners=rng.randrange(4),mutate=rng.randrange(2),rebind=0))
 print('# reference_sha256',REFERENCE_SHA256)
 print('# complete original141139fe0; actual engine/query getters; weather/context/option/notify effects controlled; exact object/maps/live operands; no GPU or registry/ownership proof')
 for i,v in enumerate(rows):
  e.write(obj,bytes([v['seed']])*0x3e8);h.prepare(obj,i,v);e.call(0x141139fe0,ints=[obj],max_instructions=100000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and h.row;print(h.row)
 print('# cases',len(rows));print('# original query/getter returns',h.native_totals)
if __name__=='__main__':main(sys.argv[1])
