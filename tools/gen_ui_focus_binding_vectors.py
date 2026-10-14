#!/usr/bin/env python3
"""Full owner-binding native body on finite synthetic owner/selection graphs.
Virtual blur/stop callbacks controlled and may mutate graph fields.
"""
import hashlib,itertools,random,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RAX

def cases():
 for selected,focus,new,stop,blur in itertools.product([3,5,-1],[-1,0,4],[-1,2,4],[-1,0,1,2,4],[-2,-1,4]):
  yield 1,2,selected,focus,new,stop,255,blur,-1,-2,0,-2
 rng=random.Random(2026101428)
 for _ in range(640):
  yield rng.choice([-1,1,5]),rng.choice([-1,2,5]),rng.choice([-1,3,5]),rng.choice([-1,0,4]),rng.choice([-1,2,4]),rng.choice([-1,0,1,2,4]),rng.choice([0,1,255]),rng.choice([-2,-1,4]),rng.choice([-1,0,1,2,4]),rng.choice([-2,-1,4]),rng.randrange(2),rng.choice([-2,-1,4])

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);vt=e.alloc(0x48);e.write(vt,bytes(0x48));e.write_u64(vt+0x40,RET_ADDR+0x100);e.write_u64(vt+0x10,RET_ADDR+0x110)
 e.write(RET_ADDR+0x100,b'\xc3');e.write(RET_ADDR+0x110,b'\xc3');top=e.heap_top;nodes=[];v=[];log=[]
 def ptr(i):return 0 if i<0 else nodes[i]
 def idx(p):return -1 if p==0 else nodes.index(p)
 def blur(emu):
  i=idx(emu.reg(UC_X86_REG_RCX));log.append(f'B:{i}')
  if v[7]!=-2:emu.write_u64(nodes[3]+8,ptr(v[7]))
  if v[11]!=-2:emu.write_u64(0x145a0dfc8,ptr(v[11]))
 def stop(emu):
  i=idx(emu.reg(UC_X86_REG_RCX));assert emu.read_u64(nodes[i]+16)==0
  log.append(f'S:{i}')
  if i==v[8] and v[9]!=-2:emu.write_u64(nodes[i]+8,ptr(v[9]))
  if v[10]:emu.write_u64(nodes[i]+16,nodes[5])
  emu.uc.reg_write(UC_X86_REG_RAX,v[6] if i==v[5] else 0)
 e.stubs.update({RET_ADDR+0x100:blur,RET_ADDR+0x110:stop})
 def csv(x):return ','.join(map(str,x))
 print('# reference_sha256',REFERENCE_SHA256)
 print('# full native bind unchanged; virtual callbacks controlled; stable nodes/finite chains; no allocation')
 count=0
 for case in cases():
  v[:]=case;log.clear();e.heap_top=top;nodes[:]=[e.alloc(24) for _ in range(6)]
  owners=[-1,0,1,-1 if v[0]==-1 else 2,-1,-1];selected=[v[0],v[1],v[2],4,-1,-1]
  for i,p in enumerate(nodes):e.write_u64(p,vt);e.write_u64(p+8,ptr(owners[i]));e.write_u64(p+16,ptr(selected[i]))
  e.write_u64(0x145a0dfc8,ptr(v[3]));e.call(0x1412f6250,ints=[nodes[3],ptr(v[4])],max_instructions=100000)
  print('B',*case,'->',csv([idx(e.read_u64(p+8)) for p in nodes]),csv([idx(e.read_u64(p+16)) for p in nodes]),idx(e.read_u64(0x145a0dfc8)),*log);count+=1
 print('# cases',count)
if __name__=='__main__':main(sys.argv[1])
