#!/usr/bin/env python3
"""Whole original selection alternate; data/search/cleanup effects controlled."""
from types import SimpleNamespace
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9,UC_X86_REG_RSP,UC_X86_REG_XMM1,UC_X86_REG_XMM2,UC_X86_REG_XMM3
from emulate_xp import STACK_TOP
from ui_indexed_record_tables import Tables,INDICES
ACTIVE,INDEX=0x145a10798,0x142640504
FUNCS={0x1408aaf40:2,0x1417fb230:1,0x1417f12c0:1,0x14163ffa0:4,0x1410d4490:4,0x1408ad0f0:4,0x1417f9060:1,0x140915da0:4,0x14163f830:6,0x141640d80:5,0x1422e7c2c:2,0x1405de1d0:1,0x1410ca190:2,0x1417f63d0:2,0x1410ca0f0:2,0x1408abc10:1,0x1417f6240:2}
def install(e,tables=None,before=None,on_finish=None):
 tables=tables or Tables(e,root_strings=True);arena=e.alloc(8256);record=e.alloc(56)
 external={**tables.maps,arena:128,record:56,**{tables.settings(i)+0x24c:4 for i in INDICES},**{tables.settings(i)+0x390:16 for i in INDICES}}
 h=SimpleNamespace(active=False,enabled=False,obj=0,stack=0,row=None,v={},events=[],ranges={},entries=0,returns=0,total_returns=0,get_entries={},get_returns={},get_totals={},temp_dtor_calls=0,last_settings=0,before=before,on_finish=on_finish);old=e.stubs.copy();reads=[];writes=[]
 doubles=[0,0x8000000000000000,0x3ff4000000000000,0x7ff0000000000000,0xfff0000000000000,0x7ff0000012345678,0xfff8123456789abc]
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(h.ranges.items()))+f';{e.read_u64(ACTIVE)};{e.read_u32(INDEX)}'
 def chunks(xs):return '/'.join(f'{p}:{b.hex()}'for p,b in xs)or'-'
 def write(p,b):
  e.write(p,b)
  if h.stack<=p and p+len(b)<=h.stack+0x640:writes.append((p,b))
 def read(p,n):b=e.read(p,n);reads.append((p,b));return b
 def q(p):return int.from_bytes(read(p,8),'little')
 def put(p,n):write(p,(n&((1<<64)-1)).to_bytes(8,'little'))
 def strings():
  raw=bytearray(64)
  for off in [0,32]:raw[off:off+3]=b'abc';raw[off+16:off+24]=(3).to_bytes(8,'little');raw[off+24:off+32]=(15).to_bytes(8,'little')
  return bytes(raw)
 def prepare(obj,case,v,owned=None):
  h.obj=obj;h.v=dict(v);h.v.update(case=case,query=0,tokens=0);h.events.clear();h.row=None;h.stack=0;h.enabled=True;h.entries=h.returns=0;h.get_entries.clear();h.get_returns.clear();h.ranges={**(owned or {obj:0x2d0}),**external}
  e.write(arena,bytes([v['seed']^0x33])*128);e.write(record,bytes((v['seed']+j*13)%256 for j in range(56)))
  for i in INDICES:
   e.write_u32(tables.settings(i)+0x24c,v['major']);e.write_u64(tables.settings(i)+0x390,doubles[(case+i)%len(doubles)]);e.write_u64(tables.settings(i)+0x398,doubles[(case+i+2)%len(doubles)])
  e.write_u64(ACTIVE,v['active']);e.write_u32(INDEX,v['index'])
  if not owned:
   e.write(obj+0xb8,bytes([v['tag']]));e.write(obj+0xb0,bytes([v['optional']]));e.write(obj+0x70,strings());e.write_u64(tables.resource(88)+0x6188,7);e.write_u32(tables.settings(88)+0xdac,0);tables.prepare(case)
   # This byte is forwarded by native code before an explicit output write.
   # Treat caller-home/scratch bytes as supplied inputs, not recovered producers.
   e.write_u32(STACK_TOP-0x1018,v['pref'])
 h.prepare=prepare
 def mutate_index():
  if h.v['mutate']:e.write_u32(INDEX,88 if e.read_u32(INDEX)!=88 else 77)
 def output(firstptr,secondptr,flagptr,secondaryptr,first):
  put(firstptr,first);put(secondptr,0x33330000+h.v['case'] if h.v['result']==0 else 0);write(flagptr,h.v['flag'].to_bytes(4,'little'));put(secondaryptr,0x44440000+h.v['case'] if h.v['result']==1 else 0)
 def leaf(fn,args):
  initial=snap();reads.clear();writes.clear();v=h.v;sp=h.stack;o=h.obj;t=sp+0xe0;result=0
  if fn==0x1408aaf40:
   assert args==[t,0x142f2d7a8];assert q(sp+0x630)==q(sp+0x638)==q(sp+0x48)==0;write(t,bytes((v['seed']+i)%256 for i in range(64)))
  elif fn==0x1417fb230:assert args==[t];read(t,64);result=v['ready'];mutate_index()
  elif fn==0x1417f12c0:
   assert args==[0x1424f5648];k=v['query'];v['query']+=1;assert k<2;result=v['zero1']if k==0 else v['zero2']
   if v['mutate']:e.write_u64(h.last_settings+(0x398 if k==0 else 0x390),doubles[(v['case']+k+3)%len(doubles)]);mutate_index()
  elif fn==0x14163ffa0:
   assert args[0]==sp+0x30 and args[3]==0x41f00000;large=v['capacity']>=512;begin=arena+32 if large else arena+8
   if v['count']==0 and v['null']:begin=0
   end=begin+v['count']*8 if begin else 0;cap=begin+v['capacity']*8 if begin else 0;v.update(begin=begin,end=end,cap=cap)
   if begin:
    if large:e.write_u64(begin-8,arena)
    for i in range(v['count']):e.write_u64(begin+i*8,0x55550000+i)
   put(sp+0x30,begin);put(sp+0x38,end);put(sp+0x40,cap)
  elif fn==0x1410d4490:
   assert args[:3]==[sp+0x50,v['begin'],v['end']];read(args[1],v['count']*8);put(args[0],v['begin']+8);result=args[0]
   if v['mutate']:put(sp+0x38,v['end']-8)
  elif fn==0x1408ad0f0:
   assert args==[sp+0x30,sp+0x58,v['begin']+8,v['end']];read(sp+0x30,24);put(sp+0x38,v['begin']+8);put(args[1],v['begin']);result=args[1]
  elif fn==0x1417f9060:
   assert args==[t];read(t,16);v['tokens']+=1;result=0 if v['case']%11==0 else 0x11110000+v['tokens']*100+v['case'];mutate_index()
  elif fn==0x140915da0:
   assert args[:3]==[sp+0x58,e.read_u64(sp+0x30),e.read_u64(sp+0x38)] and args[3]==sp+0x50;q(args[3]);put(args[0],args[1]if v['found']else args[2]);result=args[0]
   if v['mutate']and not v['found']:put(sp+0x38,args[2]+8)
  elif fn==0x14163f830:
   assert args==[h.last_settings,e.read_u64(sp+0x630),sp+0x638,sp+0x628,sp+0x48,0];assert q(sp+0x20)==sp+0x48 and read(sp+0x28,1)==b'\0';put(args[2],0x33330000+v['case']if v['result']==0 else 0);write(args[3],v['flag'].to_bytes(4,'little'));put(args[4],0x44440000+v['case']if v['result']==1 else 0);mutate_index()
  elif fn==0x141640d80:
   assert args==[h.last_settings,sp+0x630,sp+0x638,sp+0x628,sp+0x48];assert q(sp+0x20)==sp+0x48;output(*args[1:],first=0 if v['case']%13==0 else 0x22220000+v['case']);mutate_index()
  elif fn==0x1422e7c2c:
   begin=v['begin'];size=v['capacity']*8;assert args==[arena if size>=4096 else begin,size+39 if size>=4096 else size]
   if v['mutate']:put(sp+0x630,0)
  elif fn==0x1405de1d0:
   p=args[0];assert p in [o+0x70,sp+0x90];read(p,64);write(p,strings());write(p+0x40,b'\0')
   if v['mutate']:
    e.write(o+0xb8,b'\x04');e.write(o+0xc3,b'\x7a')
    if p==o+0x70 and v['major']and v['result']==0:write(sp+0x90,strings());write(sp+0xd0,b'\x01')
  elif fn==0x1410ca190:
   assert args==[o+0x50,sp+0x58];raw=read(args[1],16);e.write(args[0],raw);e.write(o+0xb8,b'\x01')
  elif fn==0x1417f63d0:
   assert args==[sp+0x58,e.read_u32(INDEX)];write(args[0],bytes((v['seed']+i*7)%256 for i in range(24)));result=args[0]if v['alias']!=1 else record
  elif fn==0x1410ca0f0:
   assert args==[o+0x50,sp+0x30];raw=read(args[1],24);e.write(args[0],raw);e.write(o+0xb8,b'\x02')
  elif fn==0x1417f6240:
   assert args==[sp+0x70,e.read_u32(INDEX)];target=[args[0],record,o+0x50][v['alias']];write(target,bytes((v['seed']+i*13)%256 for i in range(56)));result=target
  elif fn==0x1408abc10:
   assert args==[t];read(t,64)
   if v['mutate']:e.write(o+0xc0,b'\xee\xee\xee\x5a');e.write_u64(ACTIVE,7)
  else:raise AssertionError(hex(fn))
  h.events.append(f'I~{fn},{",".join(map(str,args))or"-"}~{initial}~{snap()}~{result}~{chunks(reads)}~{chunks(writes)}');e.uc.reg_write(UC_X86_REG_RAX,result&((1<<64)-1))
 for fn,n in {**FUNCS,0x141a64250:2}.items():
  def dispatch(emu,fn=fn,n=n):
   if h.active:
    if fn==0x141a64250:return True
    args=[emu.reg(reg)for reg in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9][:min(n,4)]]
    if fn==0x14163ffa0:args=[args[0],emu.reg(UC_X86_REG_XMM1)&((1<<64)-1),emu.reg(UC_X86_REG_XMM2)&((1<<64)-1),emu.reg(UC_X86_REG_XMM3)&0xffffffff]
    if n>=5:args.append(e.read_u64(h.stack+0x20))
    if n>=6:args.append(e.read(h.stack+0x28,1)[0])
    leaf(fn,args)
   elif fn in old:return old[fn](emu)
   else:return True
  e.stubs[fn]=dispatch
 def begin(uc,a,size,user):
  obj=e.reg(UC_X86_REG_RCX)
  if h.before:h.before(obj,h)
  if h.enabled:assert obj==h.obj;h.active=True;h.entries+=1;h.initial=snap()
 def frame(uc,a,size,user):
  if h.active:h.stack=e.reg(UC_X86_REG_RSP);h.initial_stack=e.read(h.stack,0x640).hex()
 def finish(uc,a,size,user):
  if not h.active:return
  h.returns+=1;h.total_returns+=1;assert h.entries==h.returns==1 and h.get_entries==h.get_returns
  h.row=' '.join(map(str,['C',h.obj,h.stack,h.initial,h.initial_stack,'->',snap(),'|'.join(h.events)]));h.active=False;h.enabled=False
  if h.on_finish:h.on_finish(h.row)
 def getter_entry(uc,a,size,user):
  if h.active:h.get_entries[a]=h.get_entries.get(a,0)+1
 def getter_return(uc,a,size,user):
  if h.active:h.get_returns[0x141a64250]=h.get_returns.get(0x141a64250,0)+1;h.get_totals[0x141a64250]=h.get_totals.get(0x141a64250,0)+1;h.last_settings=e.reg(UC_X86_REG_RAX)
 def temp_dtor(uc,a,size,user):
  if h.active:h.temp_dtor_calls+=1
 for a,hook in [(0x14111bb90,begin),(0x14111bbba,frame),(0x14111c067,finish),(0x141a64250,getter_entry),(0x141a6425d,getter_return),(0x140e20d50,temp_dtor)]:e.uc.hook_add(UC_HOOK_CODE,hook,begin=a,end=a)
 def temp_dispatch(emu):
  if h.active:return True
  if 0x140e20d50 in old:return old[0x140e20d50](emu)
  return True
 e.stubs.pop(0x14111bb90,None);e.stubs[0x140e20d50]=temp_dispatch
 return h
