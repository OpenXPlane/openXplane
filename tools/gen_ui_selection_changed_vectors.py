#!/usr/bin/env python3
"""Complete original selection change; local synthetic numeric corpus only."""
import hashlib,itertools,random,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from ui_selection_changed_harness import install
from unicorn.x86_const import UC_X86_REG_RIP

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x2d0);h=install(e);rows=[dict(seed=seed,dirty=dirty,layout=layout,effect=effect)for seed,dirty,layout,effect in itertools.product([0,0xa5,255],[0,1,128,255],range(7),range(3))];rng=random.Random(2026101804)
 for _ in range(96):rows.append(dict(seed=rng.randrange(256),dirty=rng.randrange(256),layout=rng.randrange(7),effect=rng.randrange(3)))
 print('# reference_sha256',REFERENCE_SHA256);print('# complete original14113ba10; dirty set1, live next/head, no implicit dirty clear; notification effects controlled; valid mapped finite profiles; no GPU/ownership/reentrancy proof')
 for i,v in enumerate(rows):
  e.write(obj,bytes([v['seed']])*0x2d0);h.prepare(obj,i,v);e.call(0x14113ba10,ints=[obj],max_instructions=10000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and h.row;print(h.row)
 print('# cases',len(rows));print('# original returns',h.total_returns)
if __name__=='__main__':main(sys.argv[1])
