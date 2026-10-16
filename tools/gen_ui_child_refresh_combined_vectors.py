#!/usr/bin/env python3
"""Whole original child + refresh + retained callback construction, local corpus.
Shared refresh providers controlled; neither child nor refresh body is replaced.
"""
import sys
from ui_context_refresh_harness import harness,cases,prepare,GLOBAL,REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_RAX

def main(path):
 h=harness(path);e=h.e;h.all_links.clear();ctx=[];v={};live=set()
 refresh_free=e.stubs[0x1422e7c2c]
 def alloc(emu):
  n=emu.reg(UC_X86_REG_RCX);assert n in [40,120];p=emu.alloc(n);emu.write(p,bytes([v['seed']^0x5a])*n)
  if v['phase']=='child':
   i=v['alloc'];v['alloc']+=1
   if i<2:
    assert n==[120,40][i];ctx.append(f'H:{n}:{p}:{emu.read(h.obj,96).hex()}')
    if i==0:v['base']=p
   else:ctx.append(f'L:120:{p}' if n==120 else 'A:40')
  else:assert v['phase']=='setup'
  emu.uc.reg_write(UC_X86_REG_RAX,p)
 def free(emu):
  if v['phase']=='refresh':refresh_free(emu)
  else:
   assert emu.reg(UC_X86_REG_RDX)==40
   if v['phase']=='child':ctx.append('F:40')
 def enter(uc,a,size,user):
  assert v['phase']=='child';ctx.append(f'R:{e.read(h.obj,96).hex()}');v['phase']='refresh';h.all_links.append(e.read_u64(h.obj+8))
 def leave(uc,a,size,user):
  assert v['phase']=='refresh';ctx.append(f'S:{h.state()}');v['phase']='child'
 def temporary(uc,a,size,user):
  if v['phase']=='child':v['temporary']=e.reg(UC_X86_REG_RCX)
 e.stubs.update({0x1422e7bf0:alloc,0x1422e7c2c:free})
 assert 0x1411396d0 not in e.stubs and 0x1410d7490 not in e.stubs
 for address,fn in [(0x1411396d0,enter),(0x1410d7522,leave),(0x1410c7d20,temporary)]:e.uc.hook_add(UC_HOOK_CODE,fn,begin=address,end=address)
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
   base=p==h.obj+32;kind='B' if base else 'W';a,b=(0,0) if base else (e.read_u64(p+24),e.read_u64(p+32));ps=peers(p);assert all(x in live for x in ps)
   if not base:assert e.read_u64(p+96)==0
   out.append(f'{p}:{kind}:{a}:{b}:{",".join(map(str,ps)) or "-"}')
  return ';'.join(out) or '-'
 print('# reference_sha256',REFERENCE_SHA256)
 print('# whole original child1410d7490 + refresh1411396d0 + callback/list/copy/teardown bodies unchanged; only leaf providers/CRT/allocator/free controlled; new empty observer list; callback RB heap bytes not exported')
 rows=cases()
 for i,(inputs,z,d) in enumerate(rows):
  inputs=list(inputs);inputs[8]=inputs[9]=0;inputs[10]=min(inputs[10],1);h.all_links.clear();fields=prepare(h,i,(inputs,z,d));seed=fields[0];v.update(seed=seed,phase='setup');live.clear()
  parents=[e.alloc(24) for _ in range(2)];heads=[e.alloc(120) for _ in range(2)];count=[0,1,3][(i//3)%3]
  for j in range(2):
   e.write(parents[j],bytes(24));e.write_u64(parents[j]+8,heads[j]);e.write(heads[j],bytes([seed])*120);e.write_u64(heads[j],heads[j]);e.write_u64(heads[j]+8,heads[j])
   for _ in range(count if j==0 else max(0,count-1)):
    temp=e.alloc(104);e.write(temp,bytes([seed])*104);e.call(0x1410c7d20,ints=[temp,0,7,9]);e.call(0x141149460,ints=[parents[j],temp]);live.add(e.read_u64(heads[j]+8)+16);e.call(0x1410c8430,ints=[temp])
  parent=parents[i%2];initial=[','.join(map(str,entries(parents[j],heads[j]))) or '-' for j in range(2)];initial_graph=graph();e.write(h.obj,bytes([seed])*96);fields[6]=e.read(h.obj,96).hex();ctx.clear();h.events.clear();v.update(phase='child',alloc=0,temporary=0)
  result=e.call(0x1410d7490,ints=[h.obj,parent],max_instructions=200000);assert result==h.obj and v['phase']=='child' and v['temporary'];assert any(x.startswith('E~') for x in h.events)
  new=entries(parent,heads[i%2])[-1];live.update([h.obj+32,new+16]);assert peers(h.obj+32)==[new+16]
  print('C',*fields,parent,','.join(map(str,parents)),','.join(map(str,heads)),*initial,initial_graph,'->',e.read(h.obj,96).hex(),v['base'],e.read(v['base'],120).hex(),*[list_image(parents[j],heads[j]) for j in range(2)],graph(),'|'.join(ctx),'|'.join(h.events),e.read_u64(GLOBAL),h.links(),v['temporary'],h.obj)
 print('# cases',len(rows))
if __name__=='__main__':main(sys.argv[1])
