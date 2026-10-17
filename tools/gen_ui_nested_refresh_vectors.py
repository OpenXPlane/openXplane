#!/usr/bin/env python3
"""Whole14113a300 with controlled leaf effects; generated corpus local only."""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RIP
ACTIVE,INDEX=0x145a10798,0x142640504

def main(path,native_payload=False):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x158);heads=[e.alloc(120) for _ in range(2)];nodes=[e.alloc(120) for _ in range(3)];slots=e.alloc(32);resources=[e.alloc(0x6190) for _ in range(2)];settings=e.alloc(0xdb0);dynamic=e.alloc(64);top=e.heap_top;locations=sorted(heads+nodes+[slots+i*8 for i in range(4)]);v={};events=[]
 def links():return ','.join(f'{p}:{e.read_u64(p)}' for p in locations)
 def snapshot():return f'{e.read(obj,0x158).hex()};{e.read_u64(ACTIVE)};{e.read_u32(INDEX)};{links()}'
 def effect(kind,args,fn,result='-'):
  before=snapshot();fn();events.append(f'{kind}~{args}~{before}~{snapshot()}~{result}')
 def ret(x):e.uc.reg_write(UC_X86_REG_RAX,x)
 def release(emu):
  p=emu.reg(UC_X86_REG_RCX)
  def apply():
   if v['mutation']:emu.write_u64(obj+0x140,slots+24);emu.write_u64(obj+0x148,slots+24);emu.write_u64(slots+8,0)
  effect('X',str(p),apply)
 def engine(emu):
  assert emu.reg(UC_X86_REG_RCX)==0x142f2d770
  effect('E','-',lambda:emu.write_u64(ACTIVE,v['active_after']),str(v['engine']));ret(v['engine'])
 def resource(emu):
  index=emu.reg(UC_X86_REG_RDX);assert emu.reg(UC_X86_REG_RCX)==0x146124398;p=resources[v['resource_calls']%2];v['resource_calls']+=1
  effect('R',str(index),lambda:emu.write(obj+0x129,bytes([v['seed']^0x11])),str(p));ret(p)
 def assign_resource(emu):
  p=emu.reg(UC_X86_REG_RDX);assert emu.reg(UC_X86_REG_RCX)==obj
  effect('A',str(p),lambda:(emu.write_u32(INDEX,77),emu.write(obj+0x130,b'\xff')))
 def get_settings(emu):
  index=emu.reg(UC_X86_REG_RDX);assert emu.reg(UC_X86_REG_RCX)==0x1461243b0
  effect('S',str(index),lambda:emu.write(obj+0x131,bytes([v['seed']^0x22])),str(settings));ret(settings)
 def temporary(emu):
  p,resource=emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX);v['temp']=p
  raw=bytearray((v['seed']+k)%256 for k in range(0xe0));raw[0xa8:0xb0]=(p+0xc0).to_bytes(8,'little')
  if native_payload:
   raw[0x30:0x50]=bytes(32);raw[0x30:0x34]=b'abc\0';raw[0x40:0x48]=(3).to_bytes(8,'little');raw[0x48:0x50]=(15).to_bytes(8,'little');raw[0x58]=[0,1,255][v['temp_mode']]
   raw[0xb0:0xb8]=(p+0xc0+[0,2,8][v['temp_mode']]*4).to_bytes(8,'little');raw[0xb8:0xc0]=(p+0xe0).to_bytes(8,'little')
  emu.write(p,bytes(raw))
  effect('T',str(resource),lambda:None,f'{p}:{raw.hex()}');ret(p)
 def assignment(emu):
  p=emu.reg(UC_X86_REG_RDX);assert emu.reg(UC_X86_REG_RCX)==obj+0x40 and p==v['temp'];before=f'{p}:{emu.read(p,0xe0).hex()}'
  def apply():
   emu.write(obj+0x40,emu.read(p,16));emu.write_u64(p+0xa8,[p+0xc0,dynamic,0][v['temp_mode']]);emu.write(p+0x58,b'\0')
  effect('P',before,apply)
  events[-1]=events[-1].rsplit('~',1)[0]+'~'+emu.read(p,0xe0).hex()
 def free(emu):
  address=emu.reg(UC_X86_REG_RCX);assert address in [dynamic,0]
  effect('F',str(address),lambda:emu.write(obj+0x132,bytes([v['seed']^0x33])))
 def destroy(emu):
  p=v['temp'];assert emu.reg(UC_X86_REG_RCX)==p+0x30
  effect('D',f'{p}:{emu.read(p,0xe0).hex()}',lambda:emu.write(obj+0x133,bytes([v['seed']^0x44])),e.read(p,0xe0).hex())
 def notify(emu):
  wrapper=emu.reg(UC_X86_REG_RCX)
  def apply():
   if v['listeners']==2 and wrapper==nodes[0]+16:emu.write_u64(obj+8,heads[1]);emu.write_u64(nodes[0],heads[1])
   emu.write(obj+0x134,bytes([v['seed']^0x55]))
  effect('N',str(wrapper),apply)
 e.stubs.update({0x140f59550:release,0x1417fb320:engine,0x141a64260:resource,0x141142ce0:assign_resource,0x141a64250:get_settings,0x1417f7610:temporary,0x1410faec0:assignment,0x142301120:free,0x1408ab620:destroy,0x1410c9d10:notify})
 if native_payload:
  del e.stubs[0x1410faec0]
  def copy(emu):
   d,src,n=emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX),emu.reg(UC_X86_REG_R8)
   if n:emu.write(d,emu.read(src,n))
   ret(d)
  e.stubs[0x1423c0280]=copy
  def observe(uc,a,size,user):
   if a==0x1410faec0:
    p=e.reg(UC_X86_REG_RDX);assert e.reg(UC_X86_REG_RCX)==obj+0x40 and p==v['temp'];v['assignment_before']=snapshot();v['assignment_args']=f'{p}:{e.read(p,0xe0).hex()}'
   else:events.append(f"P~{v['assignment_args']}~{v['assignment_before']}~{snapshot()}~{e.read(v['temp'],0xe0).hex()}")
  e.uc.hook_add(UC_HOOK_CODE,observe,begin=0x1410faec0,end=0x1410faec0)
  e.uc.hook_add(UC_HOOK_CODE,observe,begin=0x14113a3fb,end=0x14113a3fb)
 print('# reference_sha256',REFERENCE_SHA256)
 print('# object_identity',obj)
 print('# original nested payload assignment with inline strings/vectors' if native_payload else '# payload assignment controlled')
 print('# whole14113a300 normal returns; providers/leaf destructors/context construction/notification bodies controlled; payload assignment controlled unless explicitly native; full image/global/index/link snapshots and temporary bytes; no native GUI/GPU claim')
 rows=list(itertools.product([0,0xa5,255],[0,255],[0,7],[0,1],[0,7],[0,1,3],[0,1,2]))
 for i,(seed,engine_value,active,available,mode,count,listeners) in enumerate(rows):
  e.heap_top=top;v.update(seed=seed,engine=engine_value,active_after=active,mutation=i%2,temp_mode=i%3,listeners=listeners,resource_calls=0,temp=0);events.clear();e.write(obj,bytes([seed])*0x158)
  for p in locations:e.write_u64(p,0)
  e.write_u64(ACTIVE,9 if active==0 else 0);e.write_u32(INDEX,5);e.write_u64(obj+8,heads[0]);e.write_u64(heads[0],heads[0] if listeners==0 else nodes[0]);e.write_u64(heads[1],heads[1])
  for j,p in enumerate(nodes):e.write_u64(p,nodes[j+1] if j<2 else heads[0])
  for j in range(4):e.write_u64(slots+j*8,[111,222,0,444][j])
  e.write_u64(obj+0x140,slots);e.write_u64(obj+0x148,slots+count*8)
  for p in resources:e.write_u64(p+0x6188,available)
  if native_payload:
   e.write(obj+0x70,bytes(32));e.write(obj+0x70,b'xyz\0');e.write_u64(obj+0x80,3);e.write_u64(obj+0x88,15);e.write(obj+0x98,bytes([0 if i%2 else 255]))
   e.write_u64(obj+0xe8,obj+0x100);e.write_u64(obj+0xf0,obj+0x10c);e.write_u64(obj+0xf8,obj+0x120)
  e.write_u32(settings+0xdac,mode);initial=snapshot();events.clear()
  e.call(0x14113a300,ints=[obj],max_instructions=100000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR
  print('C',initial,','.join(map(str,resources)),available,settings,mode,'->',snapshot(),'|'.join(events))
 print('# cases',len(rows))
if __name__=='__main__':main(sys.argv[1], '--native-payload' in sys.argv[2:])
