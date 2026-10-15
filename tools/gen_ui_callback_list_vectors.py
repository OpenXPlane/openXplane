#!/usr/bin/env python3
"""Whole callback-list append and actual wrapper copy/registration, local corpus.
Only allocator/free controlled; custom callable null; native RB tree layout not exported.
"""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RAX,UC_X86_REG_RDX

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);parent=e.alloc(24);sentinel=e.alloc(120);slots=[e.alloc(104) for _ in range(5)];top=e.heap_top;blocks=[];events=[];values={};live=set();entries=[]
 def allocate(emu):
  size=emu.reg(UC_X86_REG_RCX);assert size in [40,120];p=emu.alloc(size);emu.write(p,bytes([values['seed']^0x5a])*size)
  events.append(f'A:{size}:{p}' if size==120 else 'A:40');blocks.append((p,size));emu.uc.reg_write(UC_X86_REG_RAX,p)
 def free(emu):
  assert emu.reg(UC_X86_REG_RDX)==40
  events.append('F:40')
 e.stubs.update({0x1422e7bf0:allocate,0x1422e7c2c:free})
 def peers(p):
  head=e.read_u64(p+8);seen=set();out=[]
  def walk(n):
   if e.read(n+25,1)[0]:assert n==head;return
   assert n not in seen;seen.add(n);walk(e.read_u64(n));out.append(e.read_u64(n+32));walk(e.read_u64(n+16))
  walk(e.read_u64(head+8));assert out==sorted(set(out)) and len(out)==e.read_u64(p+16);return out
 def graph(containers):
  result=[]
  for p in sorted(live):
   kind='B' if p in containers else 'W';a,b=(0,0) if kind=='B' else (e.read_u64(p+24),e.read_u64(p+32))
   if kind=='W':assert e.read_u64(p+96)==0
   links=peers(p);assert all(q in live for q in links)
   result.append(f'{p}:{kind}:{a}:{b}:{",".join(map(str,links)) or "-"}')
  return ';'.join(result)
 def list_image():
  assert e.read_u64(parent+16)==len(entries);p=e.read_u64(sentinel);seen=[]
  while p!=sentinel:assert p not in seen;seen.append(p);p=e.read_u64(p)
  assert seen==entries
  return ';'.join(f'{p}:{e.read_u64(p)}:{e.read_u64(p+8)}' for p in [sentinel,*entries])
 orders=[[0,1,2,0,2,1,1,0],[2,2,1,1,0,0,2,0],[0]*8,[2,1,0,2,1,0,2,1]]
 print('# reference_sha256',REFERENCE_SHA256)
 print('# whole141149460/copy1410c7bf0/bind/RB/base-teardown bodies original; allocator/free controlled; null callable; list links/count + wrapper parameters/peer sets + allocation requests compared; no internal RB bytes')
 count=steps=0
 for seed,profile,order,reverse in itertools.product([0,0xa5,255],range(3),range(4),range(2)):
  values['seed']=seed;e.heap_top=top;events.clear();blocks.clear();live.clear();entries.clear();ids=slots[::-1] if reverse else slots;containers=ids[:2];sources=ids[2:]
  e.write(parent,bytes([seed])*24);e.write_u64(parent+8,sentinel);e.write_u64(parent+16,0);e.write(sentinel,bytes([seed])*120);e.write_u64(sentinel,sentinel);e.write_u64(sentinel+8,sentinel)
  for p in ids:e.write(p,bytes([seed])*104)
  for p in containers:assert e.call(0x1410c7b50,ints=[p])==p;live.add(p)
  for i,p in enumerate(sources):
   ancestor=0 if profile==0 or (profile==2 and i==2) else containers[1 if profile==2 and i==1 else 0]
   assert e.call(0x1410c7d20,ints=[p,ancestor,3+i,(2**64-1)-i])==p;live.add(p)
   if profile==2 and i==0:e.call(0x1410c8de0,ints=[p,containers[1]])
  print('C',seed,profile,','.join(map(str,containers)),','.join(map(str,sources)),sentinel)
  for index in orders[order]:
   events.clear();start=len(blocks);e.call(0x141149460,ints=[parent,sources[index]],max_instructions=100000)
   new=blocks[start][0];assert blocks[start][1]==120;entries.append(new);live.add(new+16)
   print('T',index,new,'->',list_image(),graph(containers),'|'.join(events));steps+=1
  for p in [*sources,*containers]:
   events.clear();e.call(0x1410c8430,ints=[p],max_instructions=100000);live.remove(p)
   print('D',p,'->',list_image(),graph(containers),'|'.join(events));steps+=1
  count+=1
 print('# cases',count,'steps',steps)
if __name__=='__main__':main(sys.argv[1])
