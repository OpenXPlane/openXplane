#!/usr/bin/env python3
"""Whole original fifth refresh/hash clear on synthetic mapped state. Local only."""
import hashlib,itertools,random,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from ui_fifth_refresh_harness import install
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RAX,UC_X86_REG_RIP

def main(path,native_predicate=False,native_tables=False,native_engine=False):
 native_tables=native_tables or native_engine
 native_predicate=native_predicate or native_tables
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0xb0);resource=e.alloc(0x6190);setup=[];current={};tables=None
 if native_tables:
  from ui_indexed_record_tables import Tables
  tables=Tables(e);resource=tables.resource(88)
 def allocate(emu):
  n=emu.reg(UC_X86_REG_RCX);assert n in [120,32,128];p=emu.alloc(n);emu.write(p,bytes([current['seed']^0x5a])*n);setup.append((p,n));emu.uc.reg_write(UC_X86_REG_RAX,p)
 e.stubs.update({0x1422e7bf0:allocate,0x141a64260:lambda emu:emu.uc.reg_write(UC_X86_REG_RAX,resource)})
 h=install(e,resource,native_predicate=native_predicate,tables=tables,native_engine=native_engine);top=e.heap_top;rows=[]
 for engine,active,available,variant,items,capacity,hash_mode in itertools.product([0,255],[0,7],[0,1],[0,255],[0,1,3],[0,1,4],range(3)):
  i=len(rows);rows.append(dict(seed=[0,0xa5,255][i%3],engine=engine,active=active,available=available,variant=variant,items=items,capacity=capacity,hash=hash_mode,listeners=i%3,mutate=i%2,override=[0,1,0x80000000][i%3],bits=[0x80000000,0x3f800000,0x7fc12345][i%3],index=[0,77,88,0xffffffff][i%4]))
 rng=random.Random(2026101801)
 for _ in range(96):rows.append(dict(seed=rng.choice([0,0xa5,255]),engine=rng.choice([0,1,255,256,2**64-1]),active=rng.choice([0,7]),available=rng.randrange(2),variant=rng.choice([0,1,255,256]),items=rng.choice([0,1,3]),capacity=rng.choice([0,1,4]),hash=rng.randrange(3),listeners=rng.randrange(3),mutate=rng.randrange(2),override=rng.choice([0,1,0x80000000]),bits=rng.choice([0x80000000,0x3f800000,0x7f800000,0x7fc12345]),index=rng.choice([0,77,88,0xffffffff])))
 print('# reference_sha256',REFERENCE_SHA256)
 if native_tables:print('# both indexed accessors original inside fifth; distinct real-stride cells/low32 signed indices; registry loading not established')
 if native_predicate:print('# predicate14113b9a0 also original; remaining provider scope depends on flags; repeated gates/index reloads')
 if native_engine:print('# engine query1417fb320 original read-only byte; initial gates prepared before entry; sort effects controlled and recorded, including byte/global/index/base changes')
 print('# whole141139460 + hash clear14062b900/bucket fill original; range-erase/provider/iterator/growth/sort/payload/free/notify controlled; valid bounded mapped profiles; scratch only initial/live provider operands; no native factory/GPU claim')
 for i,v in enumerate(rows):
  if native_predicate:v.update(gate_engine=[0,1,255,256,2**64-1][i%5],gate_active=[0,7][i//5%2],gate_index=[0,77,88,0xffffffff][i//3%4])
  if native_tables:v['table_shift']=i%2
  e.heap_top=top;setup.clear();current.update(v);e.write(obj,bytes([v['seed']])*0xb0);e.stubs[0x141139460]=lambda emu:None;e.call(0x1410d6eb0,ints=[obj]);del e.stubs[0x141139460]
  h.prepare(obj,i,v,{obj:0xb0,**dict(setup)},mutate=True);e.call(0x141139460,ints=[obj],max_instructions=200000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and h.row;print(h.row)
 print('# cases',len(rows))
 if native_tables:print('# original getter returns',h.native_totals)
if __name__=='__main__':main(sys.argv[1], '--native-predicate' in sys.argv[2:], '--native-tables' in sys.argv[2:], '--native-engine' in sys.argv[2:])
