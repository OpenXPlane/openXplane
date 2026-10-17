#!/usr/bin/env python3
"""Whole first dialog child1410d6a40 and original record/callback initializers.
Only allocator and two refresh bodies controlled; numeric corpus stays local.
"""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX

def main(path,native_nested=False):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);objects=[e.alloc(0x3e0) for _ in range(2)];top=e.heap_top;blocks=[];events=[];v={}
 def heap():return '/'.join(f'{p}:{e.read(p,n).hex()}' for p,n in blocks) or '-'
 def snapshot():return f'{e.read(v["obj"],0x3e0).hex()};{heap()}'
 def allocate(emu):
  n=emu.reg(UC_X86_REG_RCX);assert n in [120,32,72,40];before=snapshot();p=emu.alloc(n);emu.write(p,bytes([v['seed']^0x5a])*n);blocks.append((p,n));events.append(f'A~{n},{p}~{before}');emu.uc.reg_write(UC_X86_REG_RAX,p)
 def nested(emu):
  obj,mode=v['obj'],v['mode'];target=emu.reg(UC_X86_REG_RCX);assert target==obj+0x268;before=snapshot()
  if mode&1:emu.write_u32(obj+0x2d0,0x12345678);emu.write_u64(obj+0x3c8,0xdeadbeef);emu.write(obj+0x1d8,b'\xee')
  if mode&2:emu.write(blocks[2][0]+100,bytes([v['seed']^0x22]));emu.write(blocks[-1][0]+39,bytes([v['seed']^0x33]));emu.write(obj+0x3c0,b'\xff');emu.write_u64(obj+0x3d8,0xabcdef)
  events.append(f'N~{target}~{before}~{snapshot()}')
 def root(emu):
  obj,mode=v['obj'],v['mode'];target=emu.reg(UC_X86_REG_RCX);assert target==obj;before=snapshot()
  assert emu.read(obj+0x3c0,1)==b'\0' and all(emu.read_u64(obj+o)==0 for o in [0x3c8,0x3d0,0x3d8])
  if mode&1:emu.write(obj+0x3c0,bytes([v['seed']^0x77]));emu.write_u64(obj+0x3d0,0x33445566)
  if mode&2:emu.write(blocks[1][0]+31,bytes([v['seed']^0x66]));emu.write_u32(obj+0x260,0x89abcdef);emu.write(obj+0x3df,bytes([v['seed']^0x44]))
  events.append(f'R~{target}~{before}~{snapshot()}')
 e.stubs.update({0x1422e7bf0:allocate,0x14113a300:nested,0x141138cf0:root})
 if native_nested:
  del e.stubs[0x14113a300]
  def engine(emu):emu.uc.reg_write(UC_X86_REG_RAX,0)
  e.stubs[0x1417fb320]=engine
  def observe(uc,a,size,user):
   if a==0x14113a300:
    assert e.reg(UC_X86_REG_RCX)==v['obj']+0x268;v['nested_before']=snapshot()
   else:events.append(f"N~{v['obj']+0x268}~{v['nested_before']}~{snapshot()}")
  e.uc.hook_add(UC_HOOK_CODE,observe,begin=0x14113a300,end=0x14113a300)
  e.uc.hook_add(UC_HOOK_CODE,observe,begin=0x1410d6c6a,end=0x1410d6c6a)

 print('# reference_sha256',REFERENCE_SHA256)
 print('# native nested refresh14113a300 with engine0; root refresh controlled' if native_nested else '# nested/root refresh controlled')
 print('# whole1410d6a40 + record141a3c750/empty clear/callback ctor unchanged; allocator and root refresh141138cf0 controlled; nested refresh original with controlled engine0' if native_nested else '# whole1410d6a40 + record141a3c750/empty clear/callback ctor unchanged; only allocator and refresh14113a300/141138cf0 controlled')
 print('# whole object/all blocks and call-entry snapshots; no live rendering; original nested composition applies only to fresh lists/engine0')
 count=0
 for seed,flag,mode,slot in itertools.product([0,0xa5,255],[0,1,128,255,256,257,0x1234567887654321,2**64-1],range(4),range(2)):
  obj=objects[slot];v.update(seed=seed,mode=mode,obj=obj);e.heap_top=top;blocks.clear();events.clear();e.write(obj,bytes([seed])*0x3e0)
  result=e.call(0x1410d6a40,ints=[obj,flag],max_instructions=100000);assert result==obj;assert [n for p,n in blocks]==[120,32,120,72,72,120,40]
  receivers=[]
  for o in [0,0x1e0,0x268]:
   p=obj+o;head=e.read_u64(p+8);assert e.read_u64(p+16)==0 and e.read_u64(head)==head and e.read_u64(head+8)==head
   receivers.append(f'{p}:{head}:0')
  assert e.read_u64(obj+0x298)==0
  print('C',seed,flag,mode,obj,'->',e.read(obj,0x3e0).hex(),heap(),'/'.join(receivers),'|'.join(events));count+=1
 print('# cases',count)
if __name__=='__main__':main(sys.argv[1], '--native-nested' in sys.argv[2:])
