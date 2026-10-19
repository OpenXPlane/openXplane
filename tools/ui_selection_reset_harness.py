#!/usr/bin/env python3
"""Complete original reset through tail finish with actual gate/query/tag/primary/getter."""
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RSP,UC_X86_REG_RIP
BEGIN,END,INDEX,ENGINE,TABLE=0x145e4dab8,0x145e4dac0,0x142640504,0x142f2dc95,0x146124398
FUNCS={0x1410fe0f0:3,0x140d776a0:2,0x140d77340:3,0x1414cf5e0:0,0x1414cce40:2,0x1405dd5c0:2,0x14163b530:1,0x1405df240:3,0x1414cd660:2,0x1410d3690:3,0x1405de1d0:1,0x141146740:3,0x1422e7c2c:2,0x1410c9d10:1,0x14113d660:1,0x14113ff10:1,0x1423c0180:3}
def install(e,tables=None,before=None,on_finish=None,native_initial=False,native_recursive=False):
 obj=0;old=e.stubs.copy();receiver=e.alloc(0x90);resource=e.alloc(0x6668);external=e.alloc(32);variant=e.alloc(0x70)
 catalogs=[e.alloc(168*3)for _ in range(3)];heaps=[e.alloc(4200)for _ in range(4)];head=e.alloc(120);head2=e.alloc(120);nodes=[e.alloc(120)for _ in range(3)]
 base_ranges={receiver:0x90,resource+0x6178:32,external:32,variant:0x70,BEGIN:8,END:8,INDEX:4,ENGINE:1,TABLE:8,head:120,head2:120,**{p:120 for p in nodes},**{p:168*3 for p in catalogs},**{p:128 for p in heaps}}
 if tables:
  base_ranges.pop(resource+0x6178);base_ranges.update(tables.maps);base_ranges[0x145a10798]=8
 h=dict(active=False,enabled=False,stack=0,ranges={},events=[],v={},native_entry={},native_return={},totals={})
 contexts=[];initial_scope=None;recursive_stats=dict(calls=0,max_reset_depth=1)
 stats=dict(cases=0,assign=0,fallback=0,cleanup=0,notify=0,finish=0,free=0)
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(h['ranges'].items()))
 def string(p,kind,which,text=b'input'):
  raw=bytearray(32);raw[16:24]=len(text).to_bytes(8,'little');raw[24:32]=[15,31,4094,4095][kind].to_bytes(8,'little')
  if not kind:raw[:len(text)]=text
  else:
   base=heaps[which];data=base+(32 if kind==3 else 8);e.write_u64(data-8,base);e.write(data,text+b'\0');raw[:8]=data.to_bytes(8,'little')
  e.write(p,raw)
 def leaf(fn,args):
  s=h['stack'];o=obj;v=h['v'];before=snap();result=0;a=s+0x20;b=s+0x40;t=s+0x60
  if fn==0x1410fe0f0:
   assert args==[o+0x38,0x1e1b9,0]
   if v['mutate'] and v['case']%11==0:e.write(o+0xc0,b'\0');e.write(o+0x18,b'\xff')
  elif fn==0x140d776a0:assert args==[0x142f02ba8,0x142643120];result=v['cli']
  elif fn==0x1414cf5e0:assert args==[];result=12345
  elif fn in [0x140d77340,0x1414cce40]:
   assert args==([0x142f02ba8,b,0x142643120]if fn==0x140d77340 else[12345,b]);string(b,(v['kind']+1)%4,1,b'secondary');result=[a,b,external][v['alias']];string(result,v['kind'],0)
  elif fn==0x1405dd5c0:assert args==[b,a];e.write(b,e.read(a,32));result=b if v['case']%2 else a
  elif fn==0x14163b530:assert args[0]in[a,b];result=v['index']&((1<<64)-1)
  elif fn==0x1405df240:
   name=e.read_u64(BEGIN)+0x58;n=0
   while e.read(name+n,1)!=b'\0':n+=1
   assert args==[a,name,n];string(a,v['kind'],3,b'first');stats['fallback']+=1
  elif fn==0x1414cd660:
   assert args==[12345,t];e.write(t,bytes([v['seed']^0x11])*0x70);e.write(t+0x68,bytes([v['tag']]));e.write(t+0x60,bytes([v['case']%2]));result=[t,variant,o+0x50][v['variant_alias']];e.write(result+0x68,bytes([v['returned_tag']]))
  elif fn==0x1410d3690:
   assert args[1:]==[s+0xe0,[t,variant,o+0x50][v['variant_alias']]];assert e.read_u64(args[1])==o+0x50
   tag=e.read(args[2]+0x68,1)[0];selector=(tag if tag<128 else tag-256)+1;assert args[0]==selector&((1<<64)-1)
   if v['mutate']:e.write_u64(BEGIN,catalogs[1]);e.write(t+0x68,b'\0');e.write(t+0x60,b'\x01')
  elif fn==0x1405de1d0:
   assert args==[t+0x20];stats['cleanup']+=1
   if v['mutate']:e.write_u64(BEGIN,catalogs[2])
  elif fn==0x141146740:
   assert args[0:2]==[o,v['argument']&0xffffffff];assert args[2]in[e.read_u64(BEGIN)+i*168 for i in range(3)];stats['assign']+=1
   if v['mutate'] and v['case']%3==0:string(a,(v['kind']+1)%4,2,b'changed')
   e.write_u64(o+0x1b0,args[2])
  elif fn==0x1422e7c2c:
   assert args[1]in[32,4095,4135];stats['free']+=1
  elif fn==0x1410c9d10:
   p=args[0]-16;assert p in nodes;stats['notify']+=1;e.write(o+0x18,b'\xa5')
   if p==nodes[0] and v['listeners']==3:e.write_u64(o+8,head2);e.write_u64(p,head2)
   if p==nodes[0] and v['listeners']==4:
    e.write_u64(p,nodes[1]);e.write_u64(nodes[1],nodes[2]);e.write_u64(nodes[2],head);e.write_u64(o+16,3)
  elif fn==0x14113d660:
   assert args==[o] and e.read(o+0x18,1)==b'\0';stats['finish']+=1;e.write(o+0x18,b'\x7e');e.write_u64(o+0x1b8,stats['cases'])
  elif fn==0x14113ff10:assert args==[e.read_u64(o+0xc8)];result=v['case']%2
  elif fn==0x1423c0180:result=int(e.read(args[0],args[2])!=e.read(args[1],args[2]))if args[2]else 0
  else:raise AssertionError(hex(fn))
  h['events'].append(f'I~{fn},{",".join(map(str,args))or"-"}~{before}~{snap()}~{result}');e.uc.reg_write(UC_X86_REG_RAX,result)
 for fn in [*FUNCS,0x14114d330,0x1417fb320,0x1417fb390,0x14113fff0,0x141a64260]:
  def dispatch(emu,fn=fn):
   if not h['active']:
    return old[fn](emu)if fn in old else True
   if fn not in FUNCS:return True
   leaf(fn,[emu.reg(r)for r in[UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8][:FUNCS[fn]]])
   if fn==0x14113d660:
    assert h['native_entry']==h['native_return'] and h['native_return'][0x14114d330]==1
    h['row']=' '.join(map(str,['C',obj,h['stack'],h['v']['argument'],h['initial'],'->',snap(),'|'.join(h['events'])]))
    stats['cases']+=1;h['active']=False;h['enabled']=False
    if contexts:
     completed=h['row'];initial_scope['resume'](completed);h.update(contexts.pop())
    else:
     if native_recursive:assert not initial_scope['suspended'] and not initial_scope['active']
     if on_finish:on_finish(h['row'])
  e.stubs[fn]=dispatch
 def frame(uc,a,size,user):
  if not h['active']:return
  s=e.reg(UC_X86_REG_RSP);h['stack']=s;h['ranges'].update({s+0x20:0xb0,s+0xe0:8});h['initial']=snap()
 e.uc.hook_add(UC_HOOK_CODE,frame,begin=0x14113a47c,end=0x14113a47c)
 returns={0x1417fb327:0x1417fb320,0x1417fb39b:0x1417fb390,0x14114d390:0x14114d330,0x14114d39d:0x14114d330,0x141140047:0x14113fff0,0x14114004f:0x14113fff0,0x141a6426d:0x141a64260}
 def observe(uc,a,size,user):
  if not h['active']:return
  if a in returns:
   fn=returns[a];h['native_return'][fn]=h['native_return'].get(fn,0)+1;h['totals'][fn]=h['totals'].get(fn,0)+1
  else:h['native_entry'][a]=h['native_entry'].get(a,0)+1
 for p in [*set(returns.values()),*returns]:e.uc.hook_add(UC_HOOK_CODE,observe,begin=p,end=p)
 def prepare(identity,case,v,owned=None,preserve=False):
  nonlocal obj
  obj=identity
  h.update(active=False,enabled=True,row=None,events=[],v={**v,'case':case},native_entry={},native_return={},ranges={**(owned or {obj:0x2d0}),**base_ranges})
  protected=set(owned or {})|set(tables.maps if tables else {})|({BEGIN,END,INDEX,0x145a10798}if owned else set())
  for p,n in h['ranges'].items():
   if not preserve and p not in protected:e.write(p,bytes([v['seed']])*n)
  for p in catalogs:
   if not preserve:
    for j in range(3):e.write(p+j*168+0x58,b'first\0suffix')
  if owned:
   # A nested call inherits physical regions; descriptor arithmetic is not allocation metadata.
   if not preserve:h['ranges'][e.read_u64(BEGIN)]=e.read_u64(END)-e.read_u64(BEGIN)
   if not preserve:e.write(ENGINE,bytes([v['engine']&255]))
  else:
   e.write_u64(BEGIN,catalogs[0]);e.write_u64(END,catalogs[0]+3*168);e.write_u32(INDEX,0);e.write_u64(TABLE,resource);e.write(ENGINE,bytes([0 if v['path']==2 else 255]));e.write(obj+0xc0,bytes([int(v['path']==0)]));e.write_u64(obj+0xc8,receiver if v['receiver']else 0);e.write(obj+0xb8,bytes([1 if v['path']==1 else 2 if v['path']==3 else 3]));string(receiver+0x20,0,0,b'gate');string(resource+0x6178,0,0,b'gate'if case%3 else b'other')
   count=[0,1,3,3,1][v['listeners']];e.write_u64(obj+8,head);e.write_u64(obj+16,count);e.write_u64(head,nodes[0]if count else head);e.write_u64(head+8,nodes[count-1]if count else head);e.write_u64(head2,head2);e.write_u64(head2+8,head2)
   for i,p in enumerate(nodes):e.write_u64(p,nodes[i+1]if i+1<count else head);e.write_u64(p+8,nodes[i-1]if i else head)
 h['prepare']=prepare
 def begin(uc,a,size,user):
  # Another scope may have returned from a controlled nested request already.
  if e.reg(UC_X86_REG_RIP)!=a:return
  if native_recursive and h['active']:
   assert initial_scope and initial_scope['active'] and e.reg(UC_X86_REG_RCX)==obj and e.reg(UC_X86_REG_RDX)==1
   assert e.read(obj+0xc0,1)==b'\0'
   fields=['active','enabled','stack','ranges','events','v','native_entry','native_return','initial','row']
   contexts.append({k:h[k]for k in fields if k in h});initial_scope['suspend']()
   if len(contexts)>16:raise AssertionError('unverified recursive profile exceeds16 nested resets')
   recursive_stats['calls']+=1;recursive_stats['max_reset_depth']=max(recursive_stats['max_reset_depth'],len(contexts)+1)
   profile={**h['v'],'argument':1,'engine':e.read(ENGINE,1)[0]}
   prepare(obj,profile['case'],profile,dict(initial_scope['ranges']),preserve=True)
  elif before:before(e.reg(UC_X86_REG_RCX),h)
  if h['enabled']:h['active']=True
 e.uc.hook_add(UC_HOOK_CODE,begin,begin=0x14113a460,end=0x14113a460)
 e.stubs.pop(0x14113a460,None)
 if native_initial:
  from ui_selection_initial_harness import install as install_initial
  def initial_begin(receiver,scope):
   assert h['active'] and receiver==obj+0x38
   v=h['v'];i=v['case'];profile=dict(seed=v['seed'],controlled=0,first=[0,1,256,257][i%4],second=1,float=i%8,float_alias=(i//3)%3,mutate=False,force=i%3==1,secondary=i%2)
   scope['prepare'](obj,i,profile,dict(h['ranges']))
  initial_scope=install_initial(e,before=initial_begin,on_finish=lambda row:h['events'].append('K~'+row.replace(' ','@').replace('|','%')),native_recursive=native_recursive)
  h['recursive_callback_stats']=initial_scope['recursive_stats'];h['initial_stats']=initial_scope['stats'];h['initial_totals']=initial_scope['totals']
 h['recursive_stats']=recursive_stats;h['contexts']=contexts
 h['stats']=stats
 return h
