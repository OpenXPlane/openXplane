#!/usr/bin/env python3
"""Whole final root refresh, controlled leaf computations. Numeric corpus local only."""
import hashlib,itertools,random,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RSP,UC_X86_REG_RIP
ACTIVE,INDEX=0x145a10798,0x142640504
COUNTS={0x1417fb320:1,0x141a64260:2,0x1405dd5c0:2,0x141a3b600:2,0x141141de0:3,0x1405ddde0:1,0x141100c40:1,0x1417f6dc0:1,0x1410fa920:2,0x1408acd90:1,0x1414cc7f0:0,0x1414d0900:2,0x1414ce750:2,0x141110050:1,0x140d776a0:2,0x1417e9f50:0,0x140d77340:3,0x1417e8460:2,0x1405df240:3,0x1414ccf50:2,0x1415404e0:1,0x1411490b0:2,0x1417e9a40:2,0x1405de1d0:1,0x141138650:2,0x1411399c0:1,0x140f59550:1,0x14113a300:1,0x1410c9d10:1}
def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x3e0);resource=e.alloc(0x6200);borrowed=e.alloc(32);heaps=[e.alloc(4135)for _ in range(5)];entrydata=e.alloc(240);items=e.alloc(32);nodes=[e.alloc(120)for _ in range(3)];head2=e.alloc(120);temp_setup=e.alloc(104);top=e.heap_top;v={};events=[];live=set();ranges=[]
 def ret(x):e.uc.reg_write(UC_X86_REG_RAX,x)
 def blocks():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(ranges))
 def snap():return f'{blocks()};{e.read_u64(ACTIVE)};{e.read_u32(INDEX)}'
 def chunks(xs):return '/'.join(f'{p}:{b.hex()}'for p,b in xs)or'-'
 def allocator(emu):
  n=emu.reg(UC_X86_REG_RCX);assert n in [120,40,32,72];p=emu.alloc(n);emu.write(p,bytes([v['seed']^0x5a])*n)
  if v['phase']=='body':events.append(f'L~120,{p}'if n==120 else 'A~40');assert n in [120,40]
  ret(p)
 def free(emu):
  if v['phase']=='body'and emu.reg(UC_X86_REG_RDX)==40:events.append('D~40')
  else:leaf(0x1422e7c2c,[emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX)])
 def text(out,role,data=None):
  n=3 if v['strings']==0 else 17
  if data is None:data=bytes((65+role+k)%90+32 for k in range(n))
  n=len(data);raw=bytearray([v['seed']^0x33]*32);cap=[15,31,4095][v['strings']]
  if cap==15:raw[:16]=bytes(16);raw[:n]=data
  else:
   base=heaps[role];p=(base+39)&~31 if cap==4095 else base
   if cap==4095:e.write_u64(p-8,base)
   e.write(p,data+b'\0');raw[:8]=p.to_bytes(8,'little')
  raw[16:24]=n.to_bytes(8,'little');raw[24:32]=cap.to_bytes(8,'little');write(out,bytes(raw));return out
 writes=[];reads=[]
 def write(p,data):
  e.write(p,data)
  if v.get('stack',0)<=p and p+len(data)<=v.get('stack',0)+0x300:writes.append((p,data))
 def read(p,n):
  data=e.read(p,n);reads.append((p,data));return data
 def vector(p):write(p,entrydata.to_bytes(8,'little')+(entrydata+80).to_bytes(8,'little')+(entrydata+240).to_bytes(8,'little'));return p
 def leaf(fn,args):
  if fn==0x141141de0:args[2]&=255
  before=snap();writes.clear();reads.clear();result=0;sp=v['stack'];a,b=sp+0x20,sp+0x40
  if fn==0x1417fb320:e.write_u64(ACTIVE,v['active']);result=v['engine']
  elif fn==0x141a64260:result=resource
  elif fn==0x1405dd5c0:
   raw=read(args[1],32);length=int.from_bytes(raw[16:24],'little');p=int.from_bytes(raw[:8],'little')if int.from_bytes(raw[24:32],'little')>15 else args[1];result=text(args[0],1,e.read(p,length))
  elif fn==0x141a3b600:write(args[0],read(args[1],32));result=args[0]
  elif fn==0x141141de0:e.write(obj+0x1d8,bytes([v['flag']]));e.write_u32(INDEX,77)
  elif fn==0x1405ddde0:read(args[0],32)
  elif fn==0x141100c40:result=v['test']
  elif fn==0x1417f6dc0:result=vector(args[0])
  elif fn==0x1410fa920:
   raw=read(args[1],24);read(args[0],24);write(args[0],raw);write(args[1],bytes(24));result=args[0]
  elif fn==0x1408acd90:read(args[0],24);write(args[0],bytes(24))
  elif fn==0x1414cc7f0:result=0x12340000
  elif fn==0x1414d0900:read(args[1],24)
  elif fn==0x1414ce750:
   vector(b)
   if v['alias_vector']:vector(obj+0x1c0);result=obj+0x1c0
   else:result=b
  elif fn==0x141110050:write(args[0],bytes(24))
  elif fn==0x140d776a0:
   result={0x142642dd0:v['cli']!=0,0x1425de020:v['keys']==1,0x1425de030:v['keys']==2}[args[1]]
  elif fn==0x1417e9f50:result=0x56780000
  elif fn==0x140d77340:result=text(args[1],1)
  elif fn==0x1417e8460:
   read(args[1],32)
   if v['cli']==2:
    if v['alias_text']:result=text(a,2)
    else:result=borrowed
  elif fn==0x1405df240:read(args[0],32);data=read(args[1],args[2]);result=text(args[0],2,data)
  elif fn==0x1414ccf50:
   text(b,2)
   result=text(a,2)if v['alias_text']else b
  elif fn==0x1415404e0:
   read(args[0],32);v['validation']+=1;result=1 if v['validation']>v['fallback']else 0
  elif fn==0x1411490b0:
   text(b,3);result=text(a,3)if v['alias_text']else b
  elif fn==0x1417e9a40:result=borrowed
  elif fn==0x1405de1d0:
   read(args[0],32)
   if v['mutate']:e.write_u64(obj+0x1c0,entrydata+240)
  elif fn==0x141138650:
   table=int.from_bytes(read(args[1],8),'little');capture=int.from_bytes(read(args[1]+8,8),'little');read(args[1]+0x38,8)
   if table==0x14264fab0:read(capture,4);write(capture,(0xffffffff if v['mismatch']else 2).to_bytes(4,'little'))
   else:assert table==0x14264fa78 and capture==obj+0x268;e.write_u64(obj+0x358,e.read_u64(obj+0x350)+4)
  elif fn==0x1411399c0:e.write(obj+0x211,bytes([v['seed']^0x12]))
  elif fn==0x140f59550:
   if v['mutate']:e.write_u64(items+8,0);e.write_u64(obj+0x3c8,items+24)
  elif fn==0x14113a300:e.write(obj+0x398,b'\0')
  elif fn==0x1410c9d10:
   if v['listeners']==2 and args[0]==nodes[0]+16:e.write_u64(obj+8,head2);e.write_u64(nodes[0],head2)
  elif fn==0x1422e7c2c:pass
  else:raise AssertionError(hex(fn))
  result=int(result)
  events.append(f'I~{fn},{",".join(map(str,args))or"-"}~{before}~{snap()}~{result}~{chunks(reads)}~{chunks(writes)}');ret(result)
 for fn,n in COUNTS.items():
  def stub(emu,fn=fn,n=n):leaf(fn,[emu.reg(r)for r in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8][:n]])
  e.stubs[fn]=stub
 e.stubs.update({0x1422e7bf0:allocator,0x1422e7c2c:free})
 def capture(uc,address,size,user):
  if v['phase']=='body':v['stack']=e.reg(UC_X86_REG_RSP);v['initial_stack']=e.read(v['stack'],0x300).hex()
 e.uc.hook_add(UC_HOOK_CODE,capture,begin=0x141138d0b,end=0x141138d0b)
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
   base=p==obj+0x288;kind='B' if base else 'W';a,b=(0,0) if base else (e.read_u64(p+24),e.read_u64(p+32));ps=peers(p);assert all(x in live for x in ps)
   out.append(f'{p}:{kind}:{a}:{b}:{",".join(map(str,ps)) or "-"}')
  return ';'.join(out) or '-'

 print('# reference_sha256',REFERENCE_SHA256)
 print('# whole141138cf0 normal return; all leaf providers/parser/string/vector effects controlled; callback bind/list/copy/destroy bodies original; raw root/declared memory/global/index snapshots, leaf args/input reads/local writes, logical callbacks/receiver links; stack snapshot only initial and live provider operands; no full factory or GPU evidence')
 rows=[]
 for cli,fallback,keys,mismatch in itertools.product(range(3),range(3),range(3),range(2)):
  i=len(rows);rows.append(dict(engine=0,active=i%2*7,available=1,flag=i%2,test=i%2,cli=cli,fallback=fallback,keys=keys,mismatch=mismatch,strings=i%3,bound=i%2,listeners=i%3,count=[0,1,3][i%3],alias_text=(i//2)%2,alias_vector=(i//3)%2,mutate=i%2))
 for flag,test,bound,strings in itertools.product([0,255],[0,255],[0,1],range(3)):
  i=len(rows);rows.append(dict(engine=255,active=0,available=1,flag=flag,test=test,cli=0,fallback=0,keys=0,mismatch=0,strings=strings,bound=bound,listeners=i%3,count=[0,1,3][i%3],alias_text=0,alias_vector=i%2,mutate=i%2))
 rng=random.Random(2026101704)
 for i in range(40):rows.append(dict(engine=255,active=rng.choice([0,7]),available=rng.randrange(2),flag=rng.choice([0,255]),test=rng.randrange(2),cli=rng.randrange(3),fallback=rng.randrange(3),keys=rng.randrange(3),mismatch=rng.randrange(2),strings=rng.randrange(3),bound=rng.randrange(2),listeners=rng.randrange(3),count=rng.choice([0,1,3]),alias_text=rng.randrange(2),alias_vector=rng.randrange(2),mutate=rng.randrange(2)))
 for i,inputs in enumerate(rows):
  seed=[0,0xa5,255][i%3];e.heap_top=top;v.clear();v.update(inputs);v.update(seed=seed,phase='setup',stack=0,validation=0);live.clear();events.clear()
  e.write(obj,bytes([seed])*0x3e0)
  saved=e.stubs.pop(0x14113a300);e.stubs[0x14113a300]=lambda emu:None;e.stubs[0x141138cf0]=lambda emu:None
  e.call(0x1410d6a40,ints=[obj,1]);del e.stubs[0x141138cf0];e.stubs[0x14113a300]=saved
  parent=obj+0x1e0;parent_head=e.read_u64(parent+8);head=e.read_u64(obj+8);live.add(obj+0x288)
  if inputs['bound']:
   e.call(0x1410c7d20,ints=[temp_setup,obj+0x288,0,0]);e.call(0x141149460,ints=[parent,temp_setup]);live.add(e.read_u64(parent_head+8)+16);e.call(0x1410c8430,ints=[temp_setup])
  e.write(obj+0x1d9,bytes([inputs['bound']]));e.write_u64(obj+0x210,i%2);e.write_u64(obj+0x350,obj+0x368);e.write_u64(obj+0x358,obj+0x370)
  for j,p in enumerate(heaps):e.write(p,bytes([seed^j])*4135)
  text(resource+0x6178,0);text(borrowed,4)
  if not inputs['available']:e.write_u64(resource+0x6188,0)
  for j in range(3):
   e.write(entrydata+j*80,bytes([seed])*80);e.write(entrydata+j*80+0x48,bytes([1 if j!=1 else 0]));raw=bytearray(32);raw[24:32]=(15).to_bytes(8,'little');e.write(entrydata+j*80+8,bytes(raw))
  e.write_u64(obj+0x1c0,entrydata);e.write_u64(obj+0x1c8,entrydata+inputs['count']*80);e.write_u64(obj+0x1d0,entrydata+240)
  for j,p in enumerate(nodes):e.write(p,bytes([seed])*120);e.write_u64(p,nodes[j+1]if j<2 else head)
  e.write(head2,bytes([seed])*120);e.write_u64(head2,head2);e.write_u64(head,nodes[0]if inputs['listeners']else head);e.write_u64(obj+16,3 if inputs['listeners']else 0)
  for j,n in enumerate([111,222,0,444]):e.write_u64(items+j*8,n)
  e.write_u64(obj+0x3c8,items);e.write_u64(obj+0x3d0,items+inputs['count']*8);e.write_u64(obj+0x3d8,items+32);e.write_u64(ACTIVE,999);e.write_u32(INDEX,5)
  ranges[:]=[(obj,0x3e0),(resource+0x6178,32),(borrowed,32),(entrydata,240),(items,32),(head,120),(head2,120),*[(p,120)for p in nodes],*[(p,4135)for p in heaps]]
  initial=snap();initial_entries=entries(parent,parent_head);initial_graph=graph();events.clear();v.update(phase='body',stack=0,validation=0)
  e.call(0x141138cf0,ints=[obj],max_instructions=200000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR
  live.update(p+16 for p in entries(parent,parent_head));e.write_u64(ACTIVE,e.read_u64(ACTIVE))
  print('C',obj,v['stack'],initial,v['initial_stack'],parent_head,','.join(map(str,initial_entries))or'-',initial_graph,'->',snap(),graph(),list_image(parent,parent_head),'|'.join(events))
 print('# cases',len(rows))
if __name__=='__main__':main(sys.argv[1])
