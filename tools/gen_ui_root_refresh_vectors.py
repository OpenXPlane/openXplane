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
def main(path,native_nested=False,construct=False,factory=False,native_context=False,native_fifth=False,native_predicate=False,native_tables=False,native_engine=False,native_root_tables=False,native_child_tables=False,native_weather=False,native_changed=False,native_alternate=False):
 native_changed=native_changed or native_alternate
 native_weather=native_weather or native_changed
 native_child_tables=native_child_tables or native_weather
 native_root_tables=native_root_tables or native_child_tables
 native_context=native_context or native_child_tables
 native_engine=native_engine or native_root_tables
 native_tables=native_tables or native_engine
 native_predicate=native_predicate or native_tables
 native_fifth=native_fifth or native_predicate
 factory=factory or native_context or native_fifth
 construct=construct or factory
 native_nested=native_nested or construct
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(0x3e0);resource=e.alloc(0x6200);borrowed=e.alloc(32);heaps=[e.alloc(4135)for _ in range(5)];entrydata=e.alloc(240);items=e.alloc(32);nodes=[e.alloc(120)for _ in range(3)];head2=e.alloc(120);temp_setup=e.alloc(104);settings=e.alloc(0xdb0)if native_nested else 0;nested_items=e.alloc(32)if native_nested else 0;nested_nodes=[e.alloc(16)for _ in range(3)]if native_nested else [];nested_head2=e.alloc(16)if native_nested else 0;top=e.heap_top;v={};events=[];live=set();ranges=[]
 tables=None
 if native_tables:
  from ui_indexed_record_tables import Tables
  tables=Tables(e,root_strings=native_root_tables);resource=tables.resource(88);settings=tables.settings(88);top=e.heap_top
 def ret(x):e.uc.reg_write(UC_X86_REG_RAX,x)
 def blocks():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(ranges))
 def snap():return f'{blocks()};{e.read_u64(ACTIVE)};{e.read_u32(INDEX)}'
 def chunks(xs):return '/'.join(f'{p}:{b.hex()}'for p,b in xs)or'-'
 def allocator(emu):
  n=emu.reg(UC_X86_REG_RCX);assert n in [120,40,32,72];before=snap() if construct and v['phase']=='body' else '';p=emu.alloc(n);poison=bytes([v['seed']^0x5a])*n;emu.write(p,poison)
  if construct and v['phase']=='body' and v['ctor_alloc']<7:
   assert n==[120,32,120,72,72,120,40][v['ctor_alloc']];v['ctor_alloc']+=1
   if n!=40:ranges.append((p,n))
   events.append(f'C~{n},{p}~{before}~{snap()}~{poison.hex()}');ret(p);return
  if construct and v['phase']=='body' and n==120:ranges.append((p,16))
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
  if v.get('stack',0)-(0x200 if native_nested else 0)<=p and p+len(data)<=v.get('stack',0)+0x300:writes.append((p,data))
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
   if v['mutate']:
    p=nested_items if v.get('in_nested')else items;off=0x3a8 if v.get('in_nested')else 0x3c8;e.write_u64(p+8,0);e.write_u64(obj+off,p+24)
  elif fn==0x14113a300:e.write(obj+0x398,b'\0')
  elif fn==0x1410c9d10:
   if v['listeners']==2 and args[0]==nodes[0]+16:e.write_u64(obj+8,head2);e.write_u64(nodes[0],head2)
   if native_nested and v['listeners']==2 and args[0]==nested_nodes[0]+16:e.write_u64(obj+0x270,nested_head2);e.write_u64(nested_nodes[0],nested_head2)
  elif fn==0x141142ce0:e.write(obj+0x398,b'\xff');e.write_u32(INDEX,88)
  elif fn==0x141a64250:result=settings
  elif fn==0x1417f7610:
   p=args[0];raw=bytearray((v['seed']+k)%256 for k in range(0xe0));raw[0x30:0x50]=bytes(32);raw[0x30:0x34]=b'abc\0';raw[0x40:0x48]=(3).to_bytes(8,'little');raw[0x48:0x50]=(15).to_bytes(8,'little');raw[0x58]=[0,1,255][v['case']%3]
   raw[0xa8:0xb0]=(p+0xc0).to_bytes(8,'little');raw[0xb0:0xb8]=(p+0xc0+[0,2,8][v['case']%3]*4).to_bytes(8,'little');raw[0xb8:0xc0]=(p+0xe0).to_bytes(8,'little');write(p,bytes(raw));result=p
  elif fn==0x1408ab620:read(args[0]-0x30,0xe0)
  elif fn==0x1422e7c2c:pass
  else:raise AssertionError(hex(fn))
  result=int(result)
  events.append(f'I~{fn},{",".join(map(str,args))or"-"}~{before}~{snap()}~{result}~{chunks(reads)}~{chunks(writes)}');ret(result)
 for fn,n in COUNTS.items():
  def stub(emu,fn=fn,n=n):
   if native_root_tables and v.get('root_scope') and fn in [0x141a64250,0x141a64260]:return True
   leaf(fn,[emu.reg(r)for r in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8][:n]])
  e.stubs[fn]=stub
 e.stubs.update({0x1422e7bf0:allocator,0x1422e7c2c:free})
 nested_stub=e.stubs[0x14113a300]
 if native_nested:
  for fn,n in {0x141142ce0:2,0x141a64250:2,0x1417f7610:2,0x1408ab620:1}.items():
   def stub(emu,fn=fn,n=n):
    if native_root_tables and v.get('root_scope') and fn in [0x141a64250,0x141a64260]:return True
    leaf(fn,[emu.reg(r)for r in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8][:n]])
   e.stubs[fn]=stub
  def copy(emu):
   d,src,n=emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX),emu.reg(UC_X86_REG_R8)
   if n:emu.write(d,emu.read(src,n))
   ret(d)
  e.stubs[0x1423c0280]=copy
  def nested_scope(uc,a,size,user):
   if v['phase']=='body':v['in_nested']=a==0x14113a300
  e.uc.hook_add(UC_HOOK_CODE,nested_scope,begin=0x14113a300,end=0x14113a300)
  e.uc.hook_add(UC_HOOK_CODE,nested_scope,begin=0x1411393c2,end=0x1411393c2)
  if construct:e.uc.hook_add(UC_HOOK_CODE,nested_scope,begin=0x1410d6c6a,end=0x1410d6c6a)

 def capture(uc,address,size,user):
  if v['phase']=='body':
   if construct:assert v['stack']==e.reg(UC_X86_REG_RSP)
   else:v['stack']=e.reg(UC_X86_REG_RSP);v['initial_stack']=e.read(v['stack']-(0x200 if native_nested else 0),0x500 if native_nested else 0x300).hex()
 e.uc.hook_add(UC_HOOK_CODE,capture,begin=0x141138d0b,end=0x141138d0b)
 if construct:
  def capture_ctor(uc,address,size,user):
   v['stack']=e.reg(UC_X86_REG_RSP)-0x2e0;v['initial_stack']=e.read(v['stack']-0x200,0x500).hex()
  e.uc.hook_add(UC_HOOK_CODE,capture_ctor,begin=0x1410d6a4f,end=0x1410d6a4f)
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
 if construct:print('# whole fresh ctor1410d6a40 with both refresh bodies and payload helpers; raw callback RB nodes opaque; fresh lists only; constructor flag low byte; remaining providers controlled')
 print('# actual nested14113a300 + payload1410faec0/string/vector helpers; remaining leaf providers controlled' if native_nested else '# nested refresh controlled')
 print('# whole141138cf0 normal return; parsing/string/vector/other effects controlled except enabled recovered helpers; callback bind/list/copy/destroy original; raw declared snapshots/live operands/receiver links; stack only initial/live provider operands; factory scope depends flags; no GPU evidence')
 rows=[]
 for cli,fallback,keys,mismatch in itertools.product(range(3),range(3),range(3),range(2)):
  i=len(rows);rows.append(dict(engine=0,active=i%2*7,available=1,flag=i%2,test=i%2,cli=cli,fallback=fallback,keys=keys,mismatch=mismatch,strings=i%3,bound=i%2,listeners=i%3,count=[0,1,3][i%3],alias_text=(i//2)%2,alias_vector=(i//3)%2,mutate=i%2))
 for flag,test,bound,strings in itertools.product([0,255],[0,255],[0,1],range(3)):
  i=len(rows);rows.append(dict(engine=255,active=0,available=1,flag=flag,test=test,cli=0,fallback=0,keys=0,mismatch=0,strings=strings,bound=bound,listeners=i%3,count=[0,1,3][i%3],alias_text=0,alias_vector=i%2,mutate=i%2))
 rng=random.Random(2026101704)
 for i in range(40):rows.append(dict(engine=255,active=rng.choice([0,7]),available=rng.randrange(2),flag=rng.choice([0,255]),test=rng.randrange(2),cli=rng.randrange(3),fallback=rng.randrange(3),keys=rng.randrange(3),mismatch=rng.randrange(2),strings=rng.randrange(3),bound=rng.randrange(2),listeners=rng.randrange(3),count=rng.choice([0,1,3]),alias_text=rng.randrange(2),alias_vector=rng.randrange(2),mutate=rng.randrange(2)))
 root_totals={};root_entries={};root_returns={}
 if native_root_tables:
  def root_entry(uc,a,size,user):
   if v.get('root_scope'):
    index=e.reg(UC_X86_REG_RDX)&0xffffffff;key=(a,index);root_entries[key]=root_entries.get(key,0)+1
  def root_return(uc,a,size,user):
   if v.get('root_scope'):
    fn={0x141a6425d:0x141a64250,0x141a6426d:0x141a64260}[a];key=(fn,e.reg(UC_X86_REG_RDX)&0xffffffff);root_returns[key]=root_returns.get(key,0)+1;root_totals[key]=root_totals.get(key,0)+1
  for a in [0x141a64250,0x141a64260]:e.uc.hook_add(UC_HOOK_CODE,root_entry,begin=a,end=a)
  for a in [0x141a6425d,0x141a6426d]:e.uc.hook_add(UC_HOOK_CODE,root_return,begin=a,end=a)
 factory_run=None
 if factory:
  from gen_ui_factory_composed_vectors import install
  def factory_begin():
   v['root_scope']=True;root_entries.clear();root_returns.clear()
   v['factory_initial']=snap();v['factory_ranges']={p for p,n in ranges};events.clear()
  def factory_finish():
   if native_root_tables:assert root_entries==root_returns
   v['root_scope']=False
   parent=obj+0x1e0;head=e.read_u64(parent+8);live.add(obj+0x288);live.update(p+16 for p in entries(parent,head))
   row=' '.join(map(str,['C',obj,v['stack'],v['factory_initial'],v['initial_stack'],head,'-','-','->',snap(),graph(),list_image(parent,head),'|'.join(events),1]))
   return row,[(p,n)for p,n in ranges if p not in v['factory_ranges']]
  factory_run=install(e,dict(identity=obj,resource=resource,settings=settings,tables=tables,begin=factory_begin,finish=factory_finish),native_context=native_context,native_fifth=native_fifth,native_predicate=native_predicate,native_tables=native_tables,native_engine=native_engine,native_child_tables=native_child_tables,native_weather=native_weather,native_changed=native_changed,native_alternate=native_alternate);top=e.heap_top
  print('# whole factory/all child constructors with actual root/nested/payload chain; remaining child refresh/provider effects controlled')
  if native_context:print('# context1411396d0 also original; context leaf providers/CRT/free controlled; shared resource index carried')
  if native_fifth:print('# fifth141139460/hash clear14062b900 also original; fifth range-erase/iterator/grow/sort/payload/free/notify providers controlled')
  if native_predicate:print('# payload predicate14113b9a0 also original; settings record shared with root refresh')
  if native_engine:print('# fifth engine query original with supplied input byte; weather query scope depends native-weather; root/nested/selection/context engine providers controlled')
  if native_tables:print('# indexed accessors original inside fifth/constructor20; root and selection/context scopes follow their enabled flags below')
  if native_alternate:print('# selection alternate14111bb90 also original; search/data/cleanup bodies controlled; both selection exits now actual, supplied scratch/home inputs')
  if native_changed:print('# selection changed14113ba10 also original; no synthetic second-parent write; dirty not cleared; fresh selection receiver')
  if native_weather:print('# weather141139fe0 also original with actual engine/getters; weather/context/weather-command providers controlled')
  if native_child_tables:print('# selection/context resource getters also original; context input table snapshot inherited from root; other leaf providers controlled')
  if native_root_tables:print('# root and both nested refresh getter bodies original; distinct valid per-index resource strings and settings cells; arrays shared into fifth')
 for i,inputs in enumerate(rows):
  seed=[0,0xa5,255][i%3];e.heap_top=top;v.clear();v.update(inputs);v.update(seed=seed,phase='setup',stack=0,validation=0,in_nested=False,case=i);live.clear();events.clear()
  e.write(obj,bytes([seed])*0x3e0)
  if construct:
   e.stubs.pop(0x14113a300,None)
   for j,p in enumerate(heaps):e.write(p,bytes([seed^j])*4135)
   text(resource+0x6178,0);text(borrowed,4)
   if not inputs['available']:e.write_u64(resource+0x6188,0)
   for j in range(3):
    e.write(entrydata+j*80,bytes([seed])*80);e.write(entrydata+j*80+0x48,bytes([1 if j!=1 else 0]));raw=bytearray(32);raw[24:32]=(15).to_bytes(8,'little');e.write(entrydata+j*80+8,bytes(raw))
   for j,n in enumerate([111,222,0,444]):e.write_u64(items+j*8,n);e.write_u64(nested_items+j*8,n)
   for p in nodes:e.write(p,bytes([seed])*120)
   e.write(head2,bytes([seed])*120)
   for p in [*nested_nodes,nested_head2]:e.write(p,bytes([seed])*16)
   e.write_u32(settings+0xdac,0 if i%2 else 7);e.write_u64(ACTIVE,999);e.write_u32(INDEX,5)
   ranges[:]=[(obj,0x3e0),(resource+0x6178,32),(borrowed,32),(entrydata,240),(items,32),(head2,120),*[(p,120)for p in nodes],*[(p,4135)for p in heaps],(nested_head2,16),(nested_items,32),(settings+0xdac,4),*[(p,16)for p in nested_nodes]]
   if tables:
    tables.prepare(i)
    known={p for p,n in ranges};ranges.extend((p,n)for p,n in (tables.maps.items() if native_root_tables else [(0x146124398,8),(0x1461243b0,8)]) if p not in known)
   initial=snap();events.clear();v.update(phase='body',stack=0,validation=0,ctor_alloc=0)
   if factory_run:
    print(factory_run(seed,[0,1,2,3,4,255][i%6],inputs['engine'],i%2,[-2147483648,0,2147483647][i%3],i));continue
   flag=[0,1,128,255,256,257,0x1234567887654321,0xffffffffffffffff][i%8]
   result=e.call(0x1410d6a40,ints=[obj,flag],max_instructions=200000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR and result==obj and v['ctor_alloc']==7
   parent=obj+0x1e0;parent_head=e.read_u64(parent+8);live.add(obj+0x288);live.update(p+16 for p in entries(parent,parent_head))
   print('C',obj,v['stack'],initial,v['initial_stack'],parent_head,'-','-','->',snap(),graph(),list_image(parent,parent_head),'|'.join(events),flag)
   continue
  saved=e.stubs.pop(0x14113a300,nested_stub);e.stubs[0x14113a300]=lambda emu:None;e.stubs[0x141138cf0]=lambda emu:None
  e.call(0x1410d6a40,ints=[obj,1]);del e.stubs[0x141138cf0];e.stubs[0x14113a300]=saved
  if native_nested:del e.stubs[0x14113a300]
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
  if native_nested:
   nested_head=e.read_u64(obj+0x270);e.write_u64(nested_head,nested_nodes[0]if inputs['listeners']else nested_head);e.write_u64(nested_head+8,nested_nodes[-1]if inputs['listeners']else nested_head);e.write_u64(obj+0x278,3 if inputs['listeners']else 0)
   for j,p in enumerate(nested_nodes):e.write_u64(p,nested_nodes[j+1]if j<2 else nested_head);e.write_u64(p+8,nested_head if j==0 else nested_nodes[j-1])
   e.write_u64(nested_head2,nested_head2);e.write_u64(nested_head2+8,nested_head2)
   for j,n in enumerate([111,222,0,444]):e.write_u64(nested_items+j*8,n)
   e.write_u64(obj+0x3a8,nested_items);e.write_u64(obj+0x3b0,nested_items+inputs['count']*8);e.write_u64(obj+0x3b8,nested_items+32);e.write_u32(settings+0xdac,0 if i%2 else 7)
   ranges.extend([(nested_head,16),(nested_head2,16),(nested_items,32),(settings+0xdac,4),*[(p,16)for p in nested_nodes]])
  initial=snap();initial_entries=entries(parent,parent_head);initial_graph=graph();events.clear();v.update(phase='body',stack=0,validation=0)
  e.call(0x141138cf0,ints=[obj],max_instructions=200000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR
  live.update(p+16 for p in entries(parent,parent_head));e.write_u64(ACTIVE,e.read_u64(ACTIVE))
  print('C',obj,v['stack'],initial,v['initial_stack'],parent_head,','.join(map(str,initial_entries))or'-',initial_graph,'->',snap(),graph(),list_image(parent,parent_head),'|'.join(events))
 print('# cases',len(rows))
 if native_tables:print('# original fifth getter returns',factory_run.native_getter_totals)
 if native_root_tables:print('# original root/nested getter returns by function/index',root_totals)
 if native_child_tables:print('# original selection/context getter returns by index',factory_run.native_child_getter_totals)
 if native_weather:print('# original weather query/getter returns',factory_run.native_weather_totals)
 if native_changed:print('# original selection changed returns',factory_run.native_changed_total())
 if native_alternate:print('# original selection alternate returns/settings/temp-dtors',factory_run.native_alternate_totals())
if __name__=='__main__':main(sys.argv[1], '--native-nested' in sys.argv[2:], '--construct' in sys.argv[2:], '--factory' in sys.argv[2:], '--native-context' in sys.argv[2:], '--native-fifth' in sys.argv[2:], '--native-predicate' in sys.argv[2:], '--native-tables' in sys.argv[2:], '--native-engine' in sys.argv[2:], '--native-root-tables' in sys.argv[2:], '--native-child-tables' in sys.argv[2:], '--native-weather' in sys.argv[2:], '--native-changed' in sys.argv[2:], '--native-alternate' in sys.argv[2:])
