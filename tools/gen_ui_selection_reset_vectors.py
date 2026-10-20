#!/usr/bin/env python3
"""Complete original reset through tail finish with actual gate/query/tag/primary/getter."""
import hashlib,itertools,random,sys
from pathlib import Path
from unicorn.x86_const import UC_X86_REG_RIP
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from ui_selection_reset_harness import install,BEGIN,END
def main(path,native_initial=False,native_recursive=False,catalog_boundaries=False,native_finish=False):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 native_recursive=native_recursive or native_finish
 native_initial=native_initial or native_recursive
 e=Emulator(path);obj=e.alloc(0x2d0);h=install(e,native_initial=native_initial,native_recursive=native_recursive,native_finish=native_finish)
 tags=[255,0,1,2,3,4];rows=[]
 for tag,cli,alias,kind,ix in itertools.product(tags,[0,255],range(3),range(4),[-1,1,99]):
  i=len(rows);rows.append(dict(tag=tag,cli=cli,alias=alias,kind=kind,index=ix,seed=[0,0xa5,255][i%3],path=i%5,variant_alias=i%3,returned_tag=tags[(i//3)%6],listeners=i%5,mutate=i%2,argument=[0,1,0xffffffff,0x123456789abcdef0][i%4],receiver=0))
 rng=random.Random(14113)
 for _ in range(96):rows.append(dict(tag=rng.choice(tags),cli=rng.choice([0,255,256,257]),alias=rng.randrange(3),kind=rng.randrange(4),index=rng.choice([-1,0,1,2,3,0x80000000,0x100000001]),seed=rng.randrange(256),path=rng.randrange(5),variant_alias=rng.randrange(3),returned_tag=rng.choice(tags),listeners=rng.randrange(5),mutate=rng.randrange(2),argument=rng.getrandbits(64),receiver=rng.randrange(2)))
 assert len(rows)==528
 if catalog_boundaries:
  assert not native_initial
  source=rows;rows=[]
  distances=[-505,-504,-503,-169,-168,-167,-1,0,1,167,168,169,503,504,505,-(1<<63),(1<<63)-1,168*(1<<31),168*((1<<32)-1),168*(1<<32),168*((1<<32)+1)]
  for distance,index,cli,alias in itertools.product(distances,[-1,0,1,2],[0,255],range(3)):
   i=len(rows);v=dict(source[i%len(source)]);v.update(catalog_delta=distance,index=index,cli=cli,alias=alias,path=2,mutate=False,receiver=0);rows.append(v)
  assert len(rows)==504
 print('# reference_sha256',REFERENCE_SHA256);print('# whole reset caller; explicit corpus events distinguish actual/controlled finish; actual gate/engine/tag/primary/getter; provider effects controlled; no UI/GPU claim')
 for case,v in enumerate(rows):
  h['prepare'](obj,case,v)
  if catalog_boundaries:e.write_u64(END,(e.read_u64(BEGIN)+v['catalog_delta'])&((1<<64)-1))
  e.call(0x14113a460,ints=[obj,v['argument']],max_instructions=100000)
  assert e.reg(UC_X86_REG_RIP)==RET_ADDR and h['row']
  print(h['row'])
 if native_finish:print('# actual finish stats/returns',h['finish_stats'],h['finish_totals'])
 if native_initial:print('# actual initial callback stats/helper returns',h['initial_stats'],h['initial_totals'])
 if native_recursive:print('# actual recursive resets',h['recursive_stats'],h['recursive_callback_stats'])
 print('# cases',len(rows));print('# stats',h['stats']);print('# actual_returns',h['totals'])
if __name__=='__main__':main(sys.argv[1],'--native-initial' in sys.argv[2:], '--native-recursive' in sys.argv[2:], '--catalog-boundaries' in sys.argv[2:], '--native-finish' in sys.argv[2:])
