#!/usr/bin/env python3
"""Complete original reset through tail finish with actual gate/query/tag/primary/getter."""
import hashlib,itertools,random,sys
from pathlib import Path
from unicorn.x86_const import UC_X86_REG_RIP
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from ui_selection_reset_harness import install
def main(path,native_initial=False):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x2d0);h=install(e,native_initial=native_initial)
 tags=[255,0,1,2,3,4];rows=[]
 for tag,cli,alias,kind,ix in itertools.product(tags,[0,255],range(3),range(4),[-1,1,99]):
  i=len(rows);rows.append(dict(tag=tag,cli=cli,alias=alias,kind=kind,index=ix,seed=[0,0xa5,255][i%3],path=i%5,variant_alias=i%3,returned_tag=tags[(i//3)%6],listeners=i%5,mutate=i%2,argument=[0,1,0xffffffff,0x123456789abcdef0][i%4],receiver=0))
 rng=random.Random(14113)
 for _ in range(96):rows.append(dict(tag=rng.choice(tags),cli=rng.choice([0,255,256,257]),alias=rng.randrange(3),kind=rng.randrange(4),index=rng.choice([-1,0,1,2,3,0x80000000,0x100000001]),seed=rng.randrange(256),path=rng.randrange(5),variant_alias=rng.randrange(3),returned_tag=rng.choice(tags),listeners=rng.randrange(5),mutate=rng.randrange(2),argument=rng.getrandbits(64),receiver=rng.randrange(2)))
 assert len(rows)==528
 print('# reference_sha256',REFERENCE_SHA256);print('# whole reset caller including controlled tail finish; actual gate/engine/tag/primary/getter; provider effects controlled; no UI/GPU claim')
 for case,v in enumerate(rows):
  h['prepare'](obj,case,v)
  e.call(0x14113a460,ints=[obj,v['argument']],max_instructions=100000)
  assert e.reg(UC_X86_REG_RIP)==RET_ADDR and h['row']
  print(h['row'])
 if native_initial:print('# actual initial callback stats/helper returns',h['initial_stats'],h['initial_totals'])
 print('# cases',len(rows));print('# stats',h['stats']);print('# actual_returns',h['totals'])
if __name__=='__main__':main(sys.argv[1],'--native-initial' in sys.argv[2:])
