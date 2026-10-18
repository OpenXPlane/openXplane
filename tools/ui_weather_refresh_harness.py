#!/usr/bin/env python3
"""Whole weather refresh with real getters/query and controlled effect providers."""
from types import SimpleNamespace
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RSP,UC_X86_REG_XMM0,UC_X86_REG_XMM2
from ui_indexed_record_tables import Tables,INDICES
ACTIVE,INDEX,ENGINE=0x145a10798,0x142640504,0x142f2dc95
FUNCS={0x140d776a0:2,0x140e206c0:1,0x141b75880:1,0x1410d3a20:3,0x1417f12c0:1,0x1414d16d0:0,0x14111d6a0:1,0x1414cdc80:3,0x141b77b60:3,0x141126f60:2,0x1410c9d10:1}
def install(e,tables=None,before=None,on_finish=None):
 tables=tables or Tables(e,root_strings=True);head=e.alloc(120);nodes=[e.alloc(120)for _ in range(3)];head2=e.alloc(120);record=e.alloc(0x1a8);context=e.alloc(16)
 external={**tables.maps,ENGINE:1,head:120,head2:120,record:0x1a8,context:16,**{p:120 for p in nodes},**{tables.settings(i)+0x390:16 for i in INDICES}}
 h=SimpleNamespace(active=False,enabled=False,obj=0,stack=0,row=None,v={},events=[],ranges={},native_entries={},native_returns={},native_totals={},last_settings=0,before=before,on_finish=on_finish);old=e.stubs.copy();reads=[];writes=[]
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(h.ranges.items()))+f';{e.read_u64(ACTIVE)};{e.read_u32(INDEX)}'
 def chunks(xs):return '/'.join(f'{p}:{b.hex()}'for p,b in xs)or'-'
 def write(p,b):
  e.write(p,b)
  if h.stack<=p and p+len(b)<=h.stack+0x1e0:writes.append((p,b))
 def read(p,n):b=e.read(p,n);reads.append((p,b));return b
 def pattern(seed):return bytes((seed+i*13)%256 for i in range(0x1a8))
 doubles=[0,0x8000000000000000,0x3ff4000000000000,0x7ff0000000000000,0xfff0000000000000,0x7ff0000012345678,0xfff8123456789abc]
 def prepare(obj,case,v,owned=None):
  h.obj=obj;h.v=dict(v);h.v.update(case=case,query=0);h.events.clear();h.stack=0;h.row=None;h.enabled=True;h.native_entries.clear();h.native_returns.clear();h.ranges={**(owned or {obj:0x3e8}),**external}
  for p,n in external.items():
   if p not in tables.maps and p!=ENGINE:e.write(p,bytes([v['seed']^0x33])*n)
  for i in INDICES:
   e.write_u64(tables.settings(i)+0x390,doubles[(case+i)%len(doubles)]);e.write_u64(tables.settings(i)+0x398,doubles[(case+i+2)%len(doubles)])
  e.write(ENGINE,bytes([v['engine']&255]));e.write_u64(ACTIVE,v['active']);e.write_u32(INDEX,v['index']);e.write(context,bytes((v['seed']+i*7)%256 for i in range(16)))
  if not owned:
   raw=bytearray(32);raw[:7]=b'weather';raw[16:24]=(7 if v['available']else 0).to_bytes(8,'little');raw[24:32]=(15).to_bytes(8,'little');e.write(tables.resource(88)+0x6178,raw);e.write_u32(tables.settings(88)+0xdac,0);tables.prepare(case)
   e.write_u64(obj+8,head);e.write_u64(obj+0x3d8,0x11112222);e.write_u64(obj+0x3e0,0x33334444)
   count=3 if v['listeners']else 0;e.write_u64(obj+16,count);e.write_u64(head,nodes[0]if count else head);e.write_u64(head+8,nodes[-1]if count else head)
   for i,p in enumerate(nodes):e.write_u64(p,nodes[i+1]if i<2 else head);e.write_u64(p+8,nodes[i-1]if i else head)
  e.write_u64(head2,head2);e.write_u64(head2+8,head2)
 h.prepare=prepare
 def leaf(fn,args):
  initial=snap();reads.clear();writes.clear();o=h.obj;sp=h.stack;v=h.v;result=0
  if fn==0x140d776a0:
   assert args[0]==0x142f02ba8;result={0x1425de020:v['cli1'],0x1425de030:v['cli2'],0x1426469a8:v['mode7'],0x1426469b8:v['mode3']}[args[1]]
  elif fn in [0x140e206c0,0x141b75880]:assert args==[sp+0x30];write(args[0],pattern(v['seed']^(0x11 if fn==0x140e206c0 else 0x22)))
  elif fn==0x1410d3a20:
   assert args==[o,o+0x230,sp+0x30];read(args[2],0x1a8);write(args[1],pattern(v['seed']^0x44))
   if v.get('rebind'):e.write_u64(o+0x3e0,v['rebind'])
  elif fn==0x1417f12c0:
   assert args==[0x1424f5648];q=v['query'];v['query']+=1;assert q<2;result=v['zero1']if q==0 else v['zero2']
   if v['mutate']:
    e.write_u64(h.last_settings+(0x390 if q==0 else 0x398),doubles[(v['case']+q+3)%len(doubles)]);e.write_u32(INDEX,88 if e.read_u32(INDEX)!=88 else 77);e.write_u64(ACTIVE,7)
  elif fn==0x1414d16d0:assert not args;result=12345
  elif fn==0x14111d6a0:assert args==[e.read_u64(o+0x3d8)];result=v['altitude'];e.uc.reg_write(UC_X86_REG_XMM0,result)
  elif fn==0x1414cdc80:
   assert args==[12345,sp+0x30,v['altitude']];target=[sp+0x30,record,o+0x230,o+0x220][v['alias']];write(target,pattern(v['seed']^0x55));result=target
   if v.get('rebind'):e.write_u64(o+0x3e0,v['rebind'])
   if v['mutate']:e.write_u64(o+0x3d8,0x77778888)
  elif fn==0x141b77b60:
   assert args==[o+0x230,0,0];raw=bytearray(pattern(v['seed']^0x66));raw[12:16]=(3).to_bytes(4,'little');write(args[0],raw)
  elif fn==0x141126f60:
   assert args==[e.read_u64(o+0x3d8),sp+0x20];write(args[1],e.read(context,16));result=args[1]if v['case']%2 else context
  elif fn==0x1410c9d10:
   p=args[0]-16;assert p in nodes;e.write(o+0x18,b'\xa5')
   if v['listeners']==2 and p==nodes[0]:e.write_u64(o+8,head2);e.write_u64(p,head2)
   if v['listeners']==3 and p==nodes[0]:e.write_u64(p,nodes[2])
  else:raise AssertionError(hex(fn))
  h.events.append(f'I~{fn},{",".join(map(str,args))or"-"}~{initial}~{snap()}~{result}~{chunks(reads)}~{chunks(writes)}');e.uc.reg_write(UC_X86_REG_RAX,result&((1<<64)-1))
 for fn in [*FUNCS,0x1417fb320,0x141a64260,0x141a64250]:
  def dispatch(emu,fn=fn):
   if h.active:
    if fn not in FUNCS:return True
    args=[emu.reg(reg)for reg in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8][:FUNCS[fn]]]
    if fn in [0x1414cdc80,0x141b77b60]:args[2]=emu.reg(UC_X86_REG_XMM2)&0xffffffff
    leaf(fn,args)
   elif fn in old:return old[fn](emu)
   else:return True
  e.stubs[fn]=dispatch
 def begin(uc,a,size,user):
  obj=e.reg(UC_X86_REG_RCX)
  if h.before:h.before(obj,h)
  if h.enabled:assert obj==h.obj;h.active=True;h.initial=snap()
 def frame(uc,a,size,user):
  if h.active:h.stack=e.reg(UC_X86_REG_RSP);h.initial_stack=e.read(h.stack,0x1e0).hex()
 def finish(uc,a,size,user):
  if not h.active:return
  assert h.native_entries==h.native_returns
  h.row=' '.join(map(str,['C',h.obj,h.stack,h.initial,h.initial_stack,'->',snap(),'|'.join(h.events)]));h.active=False;h.enabled=False
  if h.on_finish:h.on_finish(h.row)
 def entry(uc,a,size,user):
  if h.active:h.native_entries[a]=h.native_entries.get(a,0)+1
 def ret(uc,a,size,user):
  if h.active:
   fn={0x1417fb327:0x1417fb320,0x141a6425d:0x141a64250,0x141a6426d:0x141a64260}[a];h.native_returns[fn]=h.native_returns.get(fn,0)+1;h.native_totals[fn]=h.native_totals.get(fn,0)+1
   if fn==0x141a64250:h.last_settings=e.reg(UC_X86_REG_RAX)
 for a,hook in [(0x141139fe0,begin),(0x141139fef,frame),(0x14113a2f6,finish),*[(a,entry)for a in [0x1417fb320,0x141a64250,0x141a64260]],*[(a,ret)for a in [0x1417fb327,0x141a6425d,0x141a6426d]]]:e.uc.hook_add(UC_HOOK_CODE,hook,begin=a,end=a)
 e.stubs.pop(0x141139fe0,None)
 return h
