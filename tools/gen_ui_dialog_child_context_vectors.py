#!/usr/bin/env python3
"""Whole child1410d7490 with original bound ctor/copy/list/teardown.
Only refresh/allocator/free controlled. Callback-tree bytes remain unexported.
"""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_RAX

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);objects=[e.alloc(96) for _ in range(2)];parents=[e.alloc(24) for _ in range(2)];heads=[e.alloc(120) for _ in range(2)];top=e.heap_top;events=[];values={};live=set()
 def allocate(emu):
  n=emu.reg(UC_X86_REG_RCX);assert n in [40,120];p=emu.alloc(n);emu.write(p,bytes([values['seed']^0x5a])*n)
  if values['phase']=='child':
   i=values['alloc'];values['alloc']+=1
   if i<2:
    assert n==[120,40][i];events.append(f'H:{n}:{p}:{emu.read(values["object"],96).hex()}')
    if i==0:values['base']=p
   else:events.append(f'L:120:{p}' if n==120 else 'A:40')
  emu.uc.reg_write(UC_X86_REG_RAX,p)
 def free(emu):
  assert emu.reg(UC_X86_REG_RDX)==40
  if values['phase']=='child':events.append('F:40')
 def refresh(emu):
  p=emu.reg(UC_X86_REG_RCX);assert p==values['object'];payload=bytearray((values['seed']+values['index']*17+i)&255 for i in range(32));payload[24]=values['mode'];parent=parents[values['changed']^values['index']]
  events.append(f'R:{emu.read(p,96).hex()}:{payload.hex()}:{parent}');emu.write(p+0x38,bytes(payload));emu.write_u64(p+0x58,parent)
 def temporary(uc,a,size,user):
  if values['phase']=='child':values['temporary']=e.reg(UC_X86_REG_RCX)
 e.stubs.update({0x1422e7bf0:allocate,0x1422e7c2c:free,0x1411396d0:refresh});e.uc.hook_add(UC_HOOK_CODE,temporary,begin=0x1410c7d20,end=0x1410c7d20)
 def peers(p):
  head=e.read_u64(p+8);seen=set();out=[]
  def walk(n):
   if e.read(n+25,1)[0]:assert n==head;return
   assert n not in seen;seen.add(n);walk(e.read_u64(n));out.append(e.read_u64(n+32));walk(e.read_u64(n+16))
  walk(e.read_u64(head+8));assert out==sorted(set(out)) and len(out)==e.read_u64(p+16);return out
 def graph():
  out=[]
  for p in sorted(live):
   base=p in [o+32 for o in objects];kind='B' if base else 'W';a,b=(0,0) if base else (e.read_u64(p+24),e.read_u64(p+32));links=peers(p);assert all(x in live for x in links)
   out.append(f'{p}:{kind}:{a}:{b}:{",".join(map(str,links)) or "-"}')
  return ';'.join(out) or '-'
 def entries(i):
  out=[];p=e.read_u64(heads[i]);prev=heads[i]
  while p!=heads[i]:assert p not in out and e.read_u64(p+8)==prev;out.append(p);prev=p;p=e.read_u64(p)
  assert len(out)==e.read_u64(parents[i]+16) and e.read_u64(heads[i]+8)==prev;return out
 def list_image(i):return ';'.join(f'{p}:{e.read_u64(p)}:{e.read_u64(p+8)}' for p in [heads[i],*entries(i)])
 print('# reference_sha256',REFERENCE_SHA256)
 print('# whole1410d7490 + original callback ctor/bind/copy/list append/base teardown; only refresh/allocator/free controlled; full96 object/base120 bytes, list links/counts, logical callback sets and request order; not callback RB heap bytes')
 cases=steps=0
 for seed,count,initial,changed,mode in itertools.product([0,0xa5,255],[0,1,3],range(2),range(2),[0,1,255]):
  e.heap_top=top;events.clear();live.clear();values.update(seed=seed,phase='setup',changed=changed,mode=mode)
  for i in range(2):
   e.write(parents[i],bytes(24));e.write_u64(parents[i]+8,heads[i]);e.write(heads[i],bytes([seed])*120);e.write_u64(heads[i],heads[i]);e.write_u64(heads[i]+8,heads[i])
   for _ in range(count if i==0 else max(0,count-1)):
    temp=e.alloc(104);e.write(temp,bytes([seed])*104);e.call(0x1410c7d20,ints=[temp,0,7,9]);e.call(0x141149460,ints=[parents[i],temp]);live.add(e.read_u64(heads[i]+8)+16);e.call(0x1410c8430,ints=[temp])
  print('C',seed,count,initial,changed,mode,','.join(map(str,objects)),','.join(map(str,parents)),','.join(map(str,heads)),','.join(map(str,entries(0))) or '-',','.join(map(str,entries(1))) or '-',graph())
  for index,p in enumerate(objects):
   e.write(p,bytes([seed^(index*0x11)])*96);events.clear();values.update(phase='child',object=p,index=index,alloc=0,temporary=0)
   result=e.call(0x1410d7490,ints=[p,parents[initial^index]],max_instructions=200000);assert result==p and values['temporary']
   target=changed^index;new=entries(target)[-1];live.update([p+32,new+16]);assert peers(p+32)==[new+16]
   print('T',index,values['temporary'],'->',e.read(p,96).hex(),values['base'],e.read(values['base'],120).hex(),list_image(0),list_image(1),graph(),'|'.join(events));steps+=1
  cases+=1
 print('# cases',cases,'steps',steps)
if __name__=='__main__':main(sys.argv[1])
