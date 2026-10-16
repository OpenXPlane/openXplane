#!/usr/bin/env python3
"""Whole second dialog child1410d6fc0, including original data/variant helpers.
Property/config/alternate/changed/final effects and allocator controlled.
Callback-tree backing remains unexported; corpus local-only.
"""
import hashlib,itertools,random,sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9,UC_X86_REG_RSP
HEAD,BEGIN,END,ACTIVE=0x14588f558,0x145e4dab8,0x145e4dac0,0x145a10798

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x2d0);parents=[e.alloc(24) for _ in range(2)];heads=[e.alloc(120) for _ in range(2)];oldnodes=[e.alloc(24) for _ in range(2)];tables=[e.alloc(168*5) for _ in range(2)];token=e.alloc(8);resource=e.alloc(0x6190);top=e.heap_top;v={};events=[];live=set()
 def catalog():
  base=e.read_u64(BEGIN);n=(e.read_u64(END)-base)//168;return base,[e.read_u64(base+i*168+16) for i in range(n)]
 def chain():return '/'.join(f'{p}:{e.read_u64(p)}:{e.read_u64(p+8)}:{e.read_u64(p+16)}' for p in v['addresses'])
 def snapshot():
  base,values=catalog();return f'{e.read(obj,0x2d0).hex()};{e.read_u64(ACTIVE)};{base};{",".join(map(str,values))};{e.read_u64(HEAD)};{chain()}'
 def ret(value):e.uc.reg_write(UC_X86_REG_RAX,value&((1<<64)-1))
 def effect(kind,args,fn,result=0):
  before=snapshot();fn();events.append(f'{kind}~{args}~{before}~{snapshot()}~{result}');ret(result)
 def allocate(emu):
  n=emu.reg(UC_X86_REG_RCX);assert n in [120,40,32];p=emu.alloc(n);emu.write(p,bytes([v['seed']^0x5a])*n)
  if v['phase']=='body':
   index=v['alloc'];v['alloc']+=1
   if index<3:
    assert n==[120,40,32][index];events.append(f'H~{n},{p}~{emu.read(obj,0x2d0).hex()};{emu.read_u64(HEAD)}')
    if index==0:v['base']=p
    if index==2:v['tree']=p
   else:events.append(f'L~120,{p}' if n==120 else 'A~40')
  ret(p)
 def free(emu):
  assert emu.reg(UC_X86_REG_RDX)==40
  if v['phase']=='body':events.append('D~40')
 def prop(emu):
  assert (emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX),emu.reg(UC_X86_REG_R8))==(obj+0x120,1,0x142643100)
  effect('P',f'{obj+0x120},1,&m_async_apt_filter_done',lambda:emu.write(obj+0xb8,bytes([v['inputs'][0]])))
 def engine(emu):assert emu.reg(UC_X86_REG_RCX)==0x142f2d770;effect('E','-',lambda:emu.write_u64(ACTIVE,v['inputs'][7]),v['inputs'][5])
 def resource_lookup(emu):assert (emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX))==(0x146124398,0);effect('R','0',lambda:None,resource)
 def targets():
  for off,choice in [(0xc8,v['inputs'][10]),(0xd0,v['inputs'][11])]:e.write_u64(obj+off,0 if choice==0 else parents[choice-1])
 def alternate(emu):
  assert emu.reg(UC_X86_REG_RCX)==obj
  def apply():e.write_u64(obj+0x50,0xabcdef1234567890);targets()
  effect('V','-',apply)
 def command(emu):assert emu.reg(UC_X86_REG_RCX)==0x145a120c0;effect('T','-',lambda:None,token)
 def selected(emu):
  assert emu.reg(UC_X86_REG_RCX)==token
  def apply():
   mode,match,raw,n,mut,*_=v['inputs']
   if mut:e.write_u64(BEGIN,tables[1]);e.write_u64(END,tables[1]+n*168)
   index=0 if raw<0 else min(raw,n-1);base,values=catalog();p=base+index*168;value=values[index]
   for off,val in [(0x50,p if match else 0),(0x58,value),(0x64,0)]:
    if off==0x64:e.write_u32(obj+off,val)
    else:e.write_u64(obj+off,val)
   e.write(obj+0x60,b'\0');e.write_u32(obj+0x68,0);e.write(obj+0xb0,b'\0')
   if match==2:e.write_u32(obj+0x64,0x80000000)
   if match==3:e.write_u32(obj+0x64,0x7fc12345)
   if match==4:e.write(obj+0x60,b'\xff')
   if match==5:e.write_u32(obj+0x68,17)
   if match==6:
    for off in [0x70,0x90]:
     rawstr=bytearray([v['seed']^0x33]*32);rawstr[:6]=b'hello\0';rawstr[16:24]=(5).to_bytes(8,'little');rawstr[24:32]=(15).to_bytes(8,'little');e.write(obj+off,bytes(rawstr))
    e.write(obj+0xb0,b'\x01')
   if match==7:e.write_u64(obj+0x58,value^1)
  effect('I',str(token),apply,v['inputs'][2])
 def changed(emu):assert emu.reg(UC_X86_REG_RCX)==obj;effect('C','-',targets)
 def reset(emu):assert (emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX))==(obj,0);effect('Z','0',lambda:emu.write_u32(obj+0x2c8,0x11223344))
 def finish(emu):assert emu.reg(UC_X86_REG_RCX)==obj;effect('F','-',lambda:emu.write_u32(obj+0x2cc,v['seed']))
 e.stubs.update({0x1422e7bf0:allocate,0x1422e7c2c:free,0x140c8f750:prop,0x1417fb320:engine,0x141a64260:resource_lookup,0x14111bb90:alternate,0x140c80d50:command,0x14163b530:selected,0x14113ba10:changed,0x14113a460:reset,0x14113d660:finish})
 def temp(uc,a,size,user):
  if v['phase']=='body':
   address=e.read_u64(e.reg(UC_X86_REG_RSP));assert address in [0x1410d739f,0x1410d73fd];v['temps'][0 if address==0x1410d739f else 1]=e.reg(UC_X86_REG_RCX)
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
   base=p==obj+0x38;kind='B' if base else 'W';a,b=(0,0) if base else (e.read_u64(p+24),e.read_u64(p+32));ps=peers(p);assert all(x in live for x in ps)
   out.append(f'{p}:{kind}:{a}:{b}:{",".join(map(str,ps)) or "-"}')
  return ';'.join(out) or '-'
 rows=[]
 for n,index,mode,match in itertools.product([1,2,5],[-2147483648,-1,0,1,2147483647],[0,1,2,3,4,255],range(8)):
  i=len(rows);rows.append([mode,match,index,n,i%2,0,i%2*7,i%2*7,0,i%2,i%3,(i//3)%3])
 rng=random.Random(2026101602)
 for _ in range(200):rows.append([rng.choice([0,1,2,3,4,255]),rng.randrange(8),rng.choice([-2147483648,-1,0,1,4,2147483647]),rng.choice([1,2,5]),rng.randrange(2),rng.choice([0,1,255]),rng.choice([0,7]),rng.choice([0,7]),rng.randrange(2),rng.randrange(2),rng.randrange(3),rng.randrange(3)])
 print('# reference_sha256',REFERENCE_SHA256)
 print('# whole1410d6fc0 + data141a3c750/empty-tree clear/view/assignment/inline optional cleanup/global interface/callback/list bodies original; property/config/alternate/change/final/allocator controlled; valid nonempty catalogs and tags-1..4; no callback RB heap bytes')
 for i,inputs in enumerate(rows):
  seed=[0,0xa5,255][i%3];oldcount=0 if i%2==0 else 2;e.heap_top=top;events.clear();live.clear();v.update(seed=seed,inputs=inputs,addresses=[*oldnodes[:oldcount],obj+0x20],phase='setup',alloc=0,temps=[0,0]);n=inputs[3]
  for j,table in enumerate(tables):
   e.write(table,bytes(168*5))
   for k in range(n):e.write_u64(table+k*168+16,(0x1111222200000000 if j==0 else 0x8888999900000000)+k)
  e.write_u64(BEGIN,tables[0]);e.write_u64(END,tables[0]+n*168);e.write_u64(ACTIVE,inputs[6]);e.write_u64(resource+0x6188,inputs[8]);e.write_u64(HEAD,oldnodes[0] if oldcount else 0)
  for j,p in enumerate(oldnodes[:oldcount]):
   for off,val in [(0,0x1426401f8),(8,oldnodes[j-1] if j else 0),(16,oldnodes[j+1] if j+1<oldcount else 0)]:e.write_u64(p+off,val)
  count=[0,1,3][(i//3)%3]
  for j in range(2):
   e.write(parents[j],bytes(24));e.write_u64(parents[j]+8,heads[j]);e.write(heads[j],bytes([seed])*120);e.write_u64(heads[j],heads[j]);e.write_u64(heads[j]+8,heads[j])
   for _ in range(count if j==0 else max(0,count-1)):
    p=e.alloc(104);e.write(p,bytes([seed])*104);e.call(0x1410c7d20,ints=[p,0,7,9]);e.call(0x141149460,ints=[parents[j],p]);live.add(e.read_u64(heads[j]+8)+16);e.call(0x1410c8430,ints=[p])
  initial_entries=[entries(parents[j],heads[j]) for j in range(2)];initial_graph=graph();parent=parents[0] if inputs[9] else 0;e.write(obj,bytes([seed])*0x2d0);events.clear();v.update(phase='body',alloc=0,temps=[0,0])
  result=e.call(0x1410d6fc0,ints=[obj,parent],max_instructions=300000);assert result==obj
  new=set(entries(parents[0],heads[0])+entries(parents[1],heads[1]))-set(sum(initial_entries,[]));live.add(obj+0x38);live.update(p+16 for p in new);assert peers(obj+0x38)==sorted(p+16 for p in new)
  base,vals=catalog();vals1=[e.read_u64(tables[0]+k*168+16) for k in range(n)];vals2=[e.read_u64(tables[1]+k*168+16) for k in range(n)]
  print('C',seed,','.join(map(str,inputs)),obj,parent,','.join(map(str,parents)),','.join(map(str,heads)),oldcount,','.join(map(str,v['addresses'])),','.join(map(str,tables)),','.join(map(str,vals1)),','.join(map(str,vals2)),*[','.join(map(str,x)) or '-' for x in initial_entries],initial_graph,'->',e.read(obj,0x2d0).hex(),v['base'],e.read(v['base'],120).hex(),v['tree'],e.read(v['tree'],32).hex(),e.read_u64(HEAD),chain(),*[list_image(parents[j],heads[j]) for j in range(2)],list_image(obj,v['base']),graph(),','.join(map(str,v['temps'])),'|'.join(events),e.read_u64(ACTIVE),base,','.join(map(str,vals)))
 print('# cases',len(rows))
if __name__=='__main__':main(sys.argv[1])
