#!/usr/bin/env python3
"""Original append/growth with controlled allocation, move and sized-free effects."""
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9,UC_X86_REG_RSP
APPEND,GROW=0x140624040,0x1405eca90

def merged(ranges):
 out={}
 for p,n in sorted(ranges.items()):
  if out:
   last=next(reversed(out));end=last+out[last]
   if p<end:out[last]=max(end,p+n)-last;continue
  out[p]=n
 return out

def install(e,before=None,on_finish=None,on_leaf=None):
 old=e.stubs.copy();pools=[e.alloc(16384)for _ in range(24)];old_buffers=[e.alloc(16384)for _ in range(2)];external=e.alloc(128)
 private={**{p:128 for p in pools},**{p:128 for p in old_buffers},external:128}
 h=dict(active=False,enabled=False,mode='A',ranges={},events=[],row=None,stack=0,args=[],v={},allocations=0,moves=0,entries={},returns={})
 stats=dict(cases=0,append=0,append_returns=0,grow=0,grow_returns=0,allocate=0,move=0,free=0,large_allocate=0)
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(h['ranges'].items()))
 def reset():h['allocations']=0
 h.update(private=private,pools=pools,reset=reset)
 def shape(length,capacity,amount):
  maximum=(1<<63)-1;new=(length+amount)&((1<<64)-1);rounded=new|15;half=capacity>>1
  result=maximum if rounded>maximum or capacity>maximum-half else max(rounded,capacity+half)
  return result+1
 def prepare(mode,args,case,v,owned=None):
  h.update(active=False,enabled=True,mode=mode,ranges={**private,**(owned or {args[0]:128})},events=[],row=None,stack=0,args=args,v={**v,'case':case},moves=0,entries={},returns={})
  if owned is not None:return
  reset();length=v['length'];capacity=v['capacity'];amount=args[1]if mode=='G'else args[2];count=args[4]if mode=='G'else args[2]
  h['ranges'].update({p:max(128,length+128)for p in old_buffers})
  for p,n in h['ranges'].items():e.write(p,bytes([v['seed']])*n)
  for j,p in enumerate(old_buffers):
   data=p+[8,16,32,39][case%4];e.write_u64(data-8,p);e.write(data,bytes((i*7+j*61+v['seed'])%256 for i in range(max(64,length+1))))
  e.write(external,bytes((i*11+v['seed'])%256 for i in range(128)))
  obj=args[0];e.write(obj,bytes((i*13+v['seed'])%256 for i in range(128)));data=old_buffers[0]+[8,16,32,39][case%4]
  if capacity>15:e.write_u64(obj,data)
  e.write_u64(obj+16,length);e.write_u64(obj+24,capacity)
  newbase=pools[0]+case%32;needed=shape(length,capacity,amount);newdata=(newbase+39)&~31 if needed>=4096 else newbase
  current=data if capacity>15 else obj
  source=[external,current+min(case%8,max(0,length-count)),obj+16,newdata][v['alias']]
  if mode=='A':args[1]=source
  else:args[3]=source
  h['old_data']=[p+[8,16,32,39][case%4]for p in old_buffers]
 h['prepare']=prepare
 def leaf(fn,args):
  initial=snap();v=h['v'];obj=h['args'][0];result=0
  if fn==0x1422e7bf0:
   stats['allocate']+=1;n=args[0];h['allocations']+=1;assert h['allocations']<=len(pools)
   p=pools[h['allocations']-1];offset=v['case']%32;result=p+offset
   count=h['args'][4]if h['mode']=='G'else h['args'][2]
   needed=max(128,offset+(n if n<=100000 else e.read_u64(obj+16)+count+80));assert needed<=16384
   prior=h['ranges'].get(p,128)
   if needed>prior:e.write(p+prior,bytes([v['seed']])*(needed-prior))
   h['ranges'][p]=max(prior,needed)
   if n>=4096:stats['large_allocate']+=1
   if v['mutate']:
    e.write_u64(obj,h['old_data'][1]);e.write_u64(obj+16,0x1111);e.write_u64(obj+24,0x2222)
  elif fn==0x1423c0280:
   stats['move']+=1;h['moves']+=1;destination,source,n=args;assert n<=16384;e.write(destination,e.read(source,n));result=destination
   if v['mutate']:
    e.write_u64(obj,h['old_data'][0]);e.write_u64(obj+16,0x3333);e.write_u64(obj+24,0x4444)
    if h['moves']==2:
     data=h['old_data'][1];e.write_u64(data-8,data-8)
  elif fn==0x1422e7c2c:
   stats['free']+=1;result=0xffffffffffffffff
   if v['mutate']:e.write_u64(obj+16,0x5555);e.write_u64(obj+24,0x6666);e.write_u64(obj,0x7777)
  else:raise AssertionError(hex(fn))
  if on_leaf:on_leaf(fn,args,h)
  h['events'].append(f'I~{fn},{",".join(map(str,args))}~{initial}~{snap()}~{result}');e.uc.reg_write(UC_X86_REG_RAX,result)
 for fn,n in[(0x1422e7bf0,1),(0x1423c0280,3),(0x1422e7c2c,2)]:
  def dispatch(emu,fn=fn,n=n):
   if not h['active']:return old[fn](emu)if fn in old else True
   leaf(fn,[emu.reg(r)for r in[UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8][:n]])
  e.stubs[fn]=dispatch
 def entry(uc,a,size,user):
  if h['active']:
   assert a==GROW and h['mode']=='A';h['entries'][GROW]=h['entries'].get(GROW,0)+1;stats['grow']+=1;return
  if before:
   args=[e.reg(r)for r in[UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9][:3 if a==APPEND else 4]]
   if a==GROW:args.append(e.read_u64(e.reg(UC_X86_REG_RSP)+0x28))
   before('A'if a==APPEND else'G',args,h)
  if h['enabled']:
   h['active']=True;h['entries'][a]=1;stats['append'if a==APPEND else'grow']+=1
 def frame(uc,a,size,user):
  if not h['active']:return
  if (a==0x14062404a)!=(h['mode']=='A'):return
  h['stack']=e.reg(UC_X86_REG_RSP);s=h['stack']
  if h['mode']=='A':
   ranges={**h['ranges'],s+0x20:8,s+0x40:1}
   if not before:e.write(s+0x40,bytes([h['v']['case']%256]))
  else:ranges={**h['ranges'],s+0x70:8}
  # Preserve larger inherited regions when a child argument aliases their base.
  for p,n in h['ranges'].items():ranges[p]=max(ranges.get(p,0),n)
  h['ranges']=merged(ranges);h['initial']=snap()
 def returns(uc,a,size,user):
  if not h['active']:return
  fn=GROW if a==0x1405ecc01 else APPEND;h['returns'][fn]=h['returns'].get(fn,0)+1;stats['grow_returns'if fn==GROW else'append_returns']+=1
  if fn==GROW and h['mode']=='A':return
  assert h['entries']==h['returns'];assert e.reg(UC_X86_REG_RAX)==h['args'][0];stats['cases']+=1
  h['row']=' '.join(map(str,['S',h['mode'],h['args'][0],h['stack'],','.join(map(str,h['args'])),','.join(map(str,pools)),h['initial'],'->',snap(),e.reg(UC_X86_REG_RAX),'|'.join(h['events'])]));h['active']=False;h['enabled']=False
  if on_finish:on_finish(h['row'],h)
 for a,f in[(APPEND,entry),(GROW,entry),(0x14062404a,frame),(0x1405eca9d,frame),(0x1406240a1,returns),(0x1406240c5,returns),(0x1405ecc01,returns)]:e.uc.hook_add(UC_HOOK_CODE,f,begin=a,end=a)
 for fn in[APPEND,GROW]:e.stubs.pop(fn,None)
 h['stats']=stats;return h
