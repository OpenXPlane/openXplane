#!/usr/bin/env python3
"""Whole1410fe0f0, scope-aware actual or explicitly controlled tag/payload helpers."""
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_RSP
FUNCS={0x141a3cbc0:2,0x1411363f0:2,0x14113d660:1,0x1405de1d0:1,0x14113ff10:1,0x14113a460:2,0x1417fb370:1,0x1417f8940:1}
PAIRS=[(0,0x80000000),(0x3f800000,0x3f800000),(0x3f800000,0x40000000),(0x7fc12345,0),(0x7fa00001,0x3f800000),(0x7f800000,0x7f800000),(0x7f800000,0xff800000),(0x80000000,0)]
def install(e,before=None,on_finish=None,native_recursive=False):
 old=e.stubs.copy();parents=[e.alloc(0x3e0)for _ in range(2)];value=e.alloc(4);record=e.alloc(64);obj=0
 external={**{p:0x3e0 for p in parents},value:4,record:64}
 h=dict(active=False,enabled=False,stack=0,ranges={},events=[],row=None,v={},compare=0,entries={},returns={},totals={})
 suspended=[];recursive_stats=dict(calls=0,max_depth=0)
 stats=dict(cases=0,compare_first=0,compare_second=0,float=0,finish=0,cleanup=0,secondary=0,reset=0,copies=0)
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(h['ranges'].items()))
 def leaf(fn,args):
  o=obj;r=o+0x38;sp=h['stack'];v=h['v'];initial=snap();result=0
  if fn==0x141a3cbc0:
   i=h['compare'];h['compare']+=1;assert i<2;assert args==[r+0xf8,e.read_u64(r+0x90)+0x20]
   stats['compare_first'if i==0 else'compare_second']+=1;result=v['first'if i==0 else'second']
   if v['mutate']:
    e.write_u64(r+0x90,parents[1]if e.read_u64(r+0x90)==parents[0]else parents[0]);e.write_u64(r+0xd0,0x11110000+v['case'])
   if i==1 and v['force']:e.write(r+0x80,b'\x03')
  elif fn==0x1411363f0:
   assert args==[e.read_u64(r+0x90)+0x268,sp+0xa8];stats['float']+=1
   left,right=PAIRS[v['float']]
   if v['mutate']:left=PAIRS[(v['float']+1)%len(PAIRS)][0]
   e.write_u32(r+0x290,left);e.write_u32(args[1],right);e.write_u32(value,right);result=[args[1],value,r+0x290][v['float_alias']]
  elif fn==0x14113d660:
   assert args==[o] and e.read_u64(r+0xd0)==e.read_u64(r+0xd8);stats['finish']+=1
   if v['mutate']:e.write_u64(r+0x90,parents[1]if v['case']%4 else 0);e.write_u64(r+0xd0,0x22220000+v['case']);e.write(r+0x88,bytes([255 if v['case']%2 else 0]))
  elif fn==0x1417fb370:assert args==[r+0x18];result=v['allow']
  elif fn==0x1417f8940:
   assert args==[r+0x18];result=[record,r+0x18,r+0x20][v['alias']];raw=bytearray((v['seed']+i*7)%256 for i in range(56));raw[48]=v['tail'];e.write(result,raw);e.write(r+0x80,bytes([v['tag']]));e.write(r+0x78,bytes([v['optional']]))
  elif fn==0x1405de1d0:
   assert args==[r+0x38];stats['cleanup']+=1
   if v['mutate']:e.write(r+0x18,b'\xcc'*56);e.write(sp+0x40,bytes((v['seed']+i*11)%256 for i in range(24)));e.write(r+0x80,b'\xff')
  elif fn==0x14113ff10:
   assert args==[e.read_u64(r+0x90)];stats['secondary']+=1;result=v['secondary']
   if v['mutate']:e.write(r+0x88,b'\xff');e.write_u64(r+0x90,parents[0])
  elif fn==0x14113a460:
   assert args==[o,1] and e.read(r+0x88,1)==b'\0';stats['reset']+=1;e.write(r+0x88,b'\x7e');e.write_u64(o+0x1b0,v['case'])
  else:raise AssertionError(hex(fn))
  h['events'].append(f'I~{fn},{",".join(map(str,args))}~{initial}~{snap()}~{result}');e.uc.reg_write(UC_X86_REG_RAX,result&((1<<64)-1))
 for fn in FUNCS:
  def dispatch(emu,fn=fn):
   if not h['active']:return old[fn](emu)if fn in old else True
   if native_recursive and fn==0x14113a460:
    assert emu.reg(UC_X86_REG_RCX)==obj and emu.reg(UC_X86_REG_RDX)==1 and e.read(obj+0xc0,1)==b'\0'
    return True
   if not h['v']['controlled']and fn in[0x1417fb370,0x1417f8940]:return True
   leaf(fn,[emu.reg(r)for r in[UC_X86_REG_RCX,UC_X86_REG_RDX][:FUNCS[fn]]])
  e.stubs[fn]=dispatch
 def prepare(identity,case,v,owned=None):
  nonlocal obj
  obj=identity;h.update(active=False,enabled=True,stack=0,row=None,v={**v,'case':case},compare=0,events=[],entries={},returns={},ranges={**(owned or {obj:0x2d0}),**external})
  for p,n in h['ranges'].items():
   if not owned or p not in owned:e.write(p,bytes([v['seed']])*n)
  if not owned:
   r=obj+0x38;e.write_u64(r+0x90,parents[0]if v['parent']else 0);e.write_u64(r+0xd0,0x1000);e.write_u64(r+0xd8,0x1000 if case%2 else 0x2000);e.write_u32(r+0x290,PAIRS[v['float']][0]);e.write(r+0x80,bytes([v['tag']]));e.write(r+0x78,bytes([v['optional']]));e.write(r+0x88,bytes([v['pending']]));e.write(r+0x48,bytes([v['tail']]))
  h['initial']=snap()
 h['prepare']=prepare
 def begin(uc,a,size,user):
  if before:before(e.reg(UC_X86_REG_RCX),h)
  if h['enabled']:assert e.reg(UC_X86_REG_RCX)==obj+0x38;h['active']=True;h['key']=e.reg(UC_X86_REG_RDX)
 def frame(uc,a,size,user):
  if h['active']:h['stack']=e.reg(UC_X86_REG_RSP);h['ranges'].update({h['stack']+0x40:24,h['stack']+0xa8:4});h['initial']=snap()
 def finish(uc,a,size,user):
  if not h['active']:return
  assert h['entries']==h['returns'];h['row']=' '.join(map(str,['C',obj,h['stack'],h['key'],int(h['v']['controlled']),h['initial'],'->',snap(),'|'.join(h['events'])or'-']));h['active']=False;h['enabled']=False;stats['cases']+=1
  if on_finish:on_finish(h['row'])
 def observe(uc,a,size,user):
  if not h['active']:return
  if a in[0x1410fe214,0x1410fe29f]:stats['copies']+=1;return
  if h['v']['controlled']:return
  fn={0x1417fb377:0x1417fb370,0x1417f894a:0x1417f8940}.get(a)
  if fn:h['returns'][fn]=h['returns'].get(fn,0)+1;h['totals'][fn]=h['totals'].get(fn,0)+1
  else:h['entries'][a]=h['entries'].get(a,0)+1
 for a,hook in [(0x1410fe0f0,begin),(0x1410fe105,frame),(0x1410fe27c,finish),*[(a,observe)for a in[0x1417fb370,0x1417fb377,0x1417f8940,0x1417f894a,0x1410fe214,0x1410fe29f]]]:e.uc.hook_add(UC_HOOK_CODE,hook,begin=a,end=a)
 def suspend():
  assert h['active']
  fields=['active','enabled','stack','ranges','events','row','v','compare','entries','returns','initial','key']
  suspended.append({k:h[k]for k in fields if k in h});h['active']=False;h['enabled']=False
  recursive_stats['calls']+=1;recursive_stats['max_depth']=max(recursive_stats['max_depth'],len(suspended))
 def resume(row):
  assert not h['active'] and suspended
  h.update(suspended.pop());h['events'].append('R~'+row.encode().hex())
 h['suspend']=suspend;h['resume']=resume;h['suspended']=suspended;h['recursive_stats']=recursive_stats
 e.stubs.pop(0x1410fe0f0,None);h['stats']=stats;return h
