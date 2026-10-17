#!/usr/bin/env python3
"""Whole fourth dialog child; weather/provider/refresh bodies controlled. Corpus local only."""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RSP,UC_X86_REG_XMM0,UC_X86_REG_XMM2

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x3e8);parents=[e.alloc(24) for _ in range(2)];heads=[e.alloc(120) for _ in range(2)];selected=e.alloc(32);top=e.heap_top;v={};events=[];live=set()
 def ret(x):e.uc.reg_write(UC_X86_REG_RAX,x)
 def effect(kind,args,fn,result='-'):
  before=e.read(obj,0x3e8).hex();fn();events.append(f'{kind}~{args}~{before}~{e.read(obj,0x3e8).hex()}~{result}')
 def allocate(emu):
  n=emu.reg(UC_X86_REG_RCX);assert n in [120,40];p=emu.alloc(n);emu.write(p,bytes([v['seed']^0x5a])*n)
  if v['phase']=='body':
   i=v['alloc'];v['alloc']+=1
   if i<2:
    assert n==[120,40][i];events.append(f'H~{n},{p}~{emu.read(obj,0x3e8).hex()}')
    if i==0:v['base']=p
   else:events.append(f'L~120,{p}' if n==120 else 'A~40')
  ret(p)
 def free(emu):
  assert emu.reg(UC_X86_REG_RDX)==40
  if v['phase']=='body':events.append('D~40')
 def weather(emu):
  off=emu.reg(UC_X86_REG_RCX)-obj;index=emu.reg(UC_X86_REG_RDX)&0xffffffff;bits=emu.reg(UC_X86_REG_XMM2)&0xffffffff;assert off in [0x58,0x230] and index==0
  def apply():
   if v['mode']&1:
    # Distinct source bytes make full copying and post-call sentinel stores visible.
    emu.write(obj+off,bytes((v['seed']+k+bits%251)%256 for k in range(0x1a8)))
   if v['mode']&2:emu.write_u32(obj+off+12,7)
  effect('W',f'{off},{index},{bits}',apply)
 def altitude(emu):
  p=emu.reg(UC_X86_REG_RCX);assert p==v['first'];effect('A0',str(p),lambda:emu.write(obj+0x219,bytes([v['seed']^0x31])),str(v['bits']));emu.uc.reg_write(UC_X86_REG_XMM0,v['bits'])
 def selection(emu):
  p=emu.reg(UC_X86_REG_RCX);assert p==v['second'];data=bytes((v['seed']+k*7)%256 for k in range(32));emu.write(selected,data);effect('S',str(p),lambda:emu.write(obj+0x220,bytes([v['seed']^0x41])),data.hex());ret(selected)
 def refresh(emu):
  assert emu.reg(UC_X86_REG_RCX)==obj
  def apply():
   emu.write_u64(obj+0x3d8,parents[1]);emu.write_u64(obj+0x3e0,parents[v['rebind']]);emu.write_u32(obj+0x23c,7 if v['special'] else 3)
   emu.write(obj+0x1f8,bytes([v['seed']^0x51]))
  effect('R','-',apply)
 def special(emu):
  assert emu.reg(UC_X86_REG_RCX)==obj;effect('F','-',lambda:emu.write_u64(obj+0x228,0x123456789abcdef0))
 e.stubs.update({0x1422e7bf0:allocate,0x1422e7c2c:free,0x141b77b60:weather,0x14111d6a0:altitude,0x141135970:selection,0x141139fe0:refresh,0x141149360:special})
 def temp(uc,a,size,user):
  if v['phase']=='body':
   retaddr=e.read_u64(e.reg(UC_X86_REG_RSP));assert retaddr in [0x1410d775f,0x1410d77ba];v['temps'][0 if retaddr==0x1410d775f else 1]=e.reg(UC_X86_REG_RCX)
 e.uc.hook_add(UC_HOOK_CODE,temp,begin=0x1410c7d20,end=0x1410c7d20)
 def entries(parent,head):
  out=[];p=e.read_u64(head);previous=head
  while p!=head:assert p not in out and e.read_u64(p+8)==previous;out.append(p);previous=p;p=e.read_u64(p)
  assert len(out)==e.read_u64(parent+16) and e.read_u64(head+8)==previous;return out
 def list_image(parent,head):return ';'.join(f'{p}:{e.read_u64(p)}:{e.read_u64(p+8)}' for p in [head,*entries(parent,head)])
 def peers(p):
  head=e.read_u64(p+8);seen=set();out=[]
  def walk(n):
   if e.read(n+25,1)[0]:assert n==head;return
   assert n not in seen;seen.add(n);walk(e.read_u64(n));out.append(e.read_u64(n+32));walk(e.read_u64(n+16))
  walk(e.read_u64(head+8));assert out==sorted(set(out)) and len(out)==e.read_u64(p+16);return out
 def graph():
  out=[]
  for p in sorted(live):
   base=p==obj+0x20;kind='B' if base else 'W';a,b=(0,0) if base else (e.read_u64(p+24),e.read_u64(p+32));ps=peers(p);assert all(x in live for x in ps)
   out.append(f'{p}:{kind}:{a}:{b}:{",".join(map(str,ps)) or "-"}')
  return ';'.join(out) or '-'

 print('# reference_sha256',REFERENCE_SHA256)
 print('# whole1410d75a0 + original140e206c0/raw float/callback/list/copy/temporary teardown; weather141b77b60, altitude14111d6a0, selection141135970, refresh141139fe0, special141149360 and allocation/free controlled; logical graph, no callback RB heap byte claim')
 rows=list(itertools.product([0,0xa5,255],range(4),[0,0x80000000,0x3f800000,0x7f800000,0x7fc12345],[0,1],[0,1],[0,1,3]))
 for i,(seed,mode,bits,rebind,sp,count) in enumerate(rows):
  e.heap_top=top;events.clear();live.clear();v.update(seed=seed,mode=mode,bits=bits,rebind=rebind,special=sp,first=parents[0],second=parents[i%2],phase='setup',alloc=0,temps=[0,0])
  for j in range(2):
   e.write(parents[j],bytes(24));e.write_u64(parents[j]+8,heads[j]);e.write(heads[j],bytes([seed])*120);e.write_u64(heads[j],heads[j]);e.write_u64(heads[j]+8,heads[j])
   for _ in range(count if j==0 else max(0,count-1)):
    p=e.alloc(104);e.write(p,bytes([seed])*104);e.call(0x1410c7d20,ints=[p,0,7,9]);e.call(0x141149460,ints=[parents[j],p]);live.add(e.read_u64(heads[j]+8)+16);e.call(0x1410c8430,ints=[p])
  initial=[entries(parents[j],heads[j]) for j in range(2)];initial_graph=graph();e.write(obj,bytes([seed])*0x3e8);events.clear();v.update(phase='body',alloc=0,temps=[0,0])
  result=e.call(0x1410d75a0,ints=[obj,v['first'],v['second']],max_instructions=300000);assert result==obj
  new=set(entries(parents[0],heads[0])+entries(parents[1],heads[1]))-set(sum(initial,[]));live.add(obj+0x20);live.update(p+16 for p in new);assert peers(obj+0x20)==sorted(p+16 for p in new) and len(new)==2
  print('C',seed,obj,v['first'],v['second'],','.join(map(str,parents)),','.join(map(str,heads)),*[','.join(map(str,x)) or '-' for x in initial],initial_graph,'->',e.read(obj,0x3e8).hex(),v['base'],e.read(v['base'],120).hex(),*[list_image(parents[j],heads[j]) for j in range(2)],list_image(obj,v['base']),graph(),','.join(map(str,v['temps'])),'|'.join(events))
 print('# cases',len(rows))
if __name__=='__main__':main(sys.argv[1])
