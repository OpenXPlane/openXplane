#!/usr/bin/env python3
"""Whole finish14113d660; real indexed getter/gate/query/primary, scoped leaf effects."""
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9,UC_X86_REG_R13,UC_X86_REG_RSP,UC_X86_REG_RIP,UC_X86_REG_XMM0,UC_X86_REG_XMM1
ENGINE,INDEX,TABLE,BEGIN,END,DENOM=0x142f2dc95,0x142640504,0x146124398,0x145e4dab8,0x145e4dac0,0x142650450
FP=[0,0x80000000,0x3f800000,0xbf800000,0x7f800000,0xff800000,0x7fc12345,0xffc54321,0x7fa00001,0xffa12345,1,0x7f7fffff,0x3f9e0652,0xc2f6e979,0x3eaaaaab,0x00800001,0x4b000001]
FUNCS={0x1417f9060:1,0x1410d6520:2,0x141a3c750:1,0x1414cf5e0:0,0x1406d4ba0:1,0x141a3cbc0:2,0x1411363f0:2,0x1410fa970:2,0x140c8ef70:2,0x141100d80:2,0x1405de3d0:2,0x1405ddde0:1,0x1423c0280:3,0x1405e6a10:3,0x1423c0920:3,0x141bae6d0:1,0x141b9e840:2,0x141ba6df0:2,0x141ba63a0:2,0x141a66550:5,0x141a664d0:3,0x14111da80:8,0x14113ff10:1,0x1423c0180:3}
def install(e,before=None,on_finish=None,native_compare=False):
 import unicorn
 assert unicorn.__version__=='2.1.4','raw finish scope requires audited Unicorn2.1.4 NaN semantics'
 record_scope=None
 old=e.stubs.copy();obj=0;parents=[e.alloc(0x3e0)for _ in range(2)];controls=[e.alloc(168)for _ in range(2)];catalog=e.alloc(168*3);buffers=[e.alloc(512)for _ in range(5)];external_record=e.alloc(0x198);external_float=e.alloc(4);arrays=[e.alloc(21*0x6668)for _ in range(2)]
 maps={**{p:0x3e0 for p in parents},**{p:168 for p in controls},catalog:168*3,**{p:512 for p in buffers},external_record:0x198,external_float:4,ENGINE:1,INDEX:4,TABLE:8,BEGIN:8,END:8,DENOM:4,**{p+i*0x6668+0x6178:32 for p in arrays for i in[0,20]}}
 h=dict(active=False,enabled=False,stack=0,row=None,v={},ranges={},events=[],ready_calls=0,manager_calls=0,float_calls=0,grows=0,inactive=False,prepop=False,entries={},returns={},totals={})
 stats=dict(cases=0,ready=0,inactive=0,assign=0,append=0,shift=0,grow=0,work=0,output=0,record_destruct=0)
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(h['ranges'].items()))
 def string(p,text):
  b=bytearray(32);b[:len(text)]=text;b[16:24]=len(text).to_bytes(8,'little');b[24:32]=(15).to_bytes(8,'little');e.write(p,b)
 def pattern(seed,n):return bytes((seed+i*13)%256 for i in range(n))
 def leaf(fn,args):
  o=obj;s=h['stack'];v=h['v'];initial=snap();result=0;t=s+0x50;p=s+0x1f0;work=s+0x390
  if fn==0x1417f9060:assert args==[o+0x50];result=controls[0]if v['token']else 0
  elif fn==0x1410d6520:
   assert args[0]in[t,p] and args[1]==e.read_u64(o+0xc8)+0x20;e.write(args[0],e.read(args[1],0x198)if native_compare else pattern(v['seed']^0x33,0x198));result=args[0]
  elif fn==0x141a3c750:
   assert args[0]in[t,p];e.write(args[0],pattern(v['seed']^0x44,0x198));e.write(external_record,pattern(v['seed']^0x66,0x198))
   if native_compare:
    for dst in[args[0],external_record]:record_scope['record'](dst,0,dict(kind=1))
   result=external_record if v['case']%2 else args[0]
  elif fn==0x1414cf5e0:assert args==[];result=12345+h['manager_calls'];h['manager_calls']+=1
  elif fn==0x1406d4ba0:
   assert args==[12345+h['ready_calls']];i=h['ready_calls'];h['ready_calls']+=1;assert i<2;result=v['ready1'if i==0 else'ready2']
   if i==0:
    stats['ready'if result&255 else'inactive']+=1
    if v['mutate']:e.write_u32(o+0x128,0xa5000000+v['case'])
  elif fn==0x141a3cbc0:
   assert args==[p,o+0x130];result=v['different']
   if v['mutate']:e.write_u64(o+0xc8,parents[1])
  elif fn==0x1411363f0:
   assert args[0]==e.read_u64(o+0xc8)+0x268 and args[1]in[s+0x650,s+0x660];i=h['float_calls'];h['float_calls']+=1
   bits=FP[(v['fp']+i)%len(FP)];e.write_u32(args[1],bits);e.write_u32(external_float,bits);result=[args[1],external_float,o+0x2c8][v['float_alias']]
   if v['mutate']:e.write_u32(o+0x2c8,FP[(v['fp']+i+2)%len(FP)])
  elif fn==0x1410fa970:
   assert args==[o+0x130,p];stats['assign']+=1;e.write(o+0x130,e.read(p,0x198))
   if v['mutate']:e.write_u64(o+0xc8,parents[0]);e.write_u64(o+0x110,e.read_u64(o+0x108)+v['items']*8)
  elif fn==0x140c8ef70:assert args==[e.read_u64(o+0x120),0xffffffff];e.write_u32(o+0x124,0x11223344)
  elif fn==0x141100d80:
   assert args==[o,t]
   if v['mutate']:e.write_u64(o+0xc8,parents[1]);e.write_u64(s+0x658,controls[1]if v['case']%3 else 0)
  elif fn==0x1405de3d0:assert args[0]==o+0x130 and args[1]in[t,external_record];h['inactive']=True;e.write(o+0x130,e.read(args[1],0x198))
  elif fn==0x1405ddde0:
   assert args[0]in[t,p];stats['record_destruct']+=1
   if args[0]==t and h['inactive'] and not h['prepop']:
    h['prepop']=True;e.write_u64(o+0x110,e.read_u64(o+0x108)+v['items']*8)
    if v['mutate']:e.write_u64(s+0x658,controls[1]if v['case']%3 else 0)
  elif fn==0x1423c0280:
   d,src,n=args;assert n<=512;e.write(d,e.read(src,n));stats['shift']+=1;result=d
   if v['mutate']:e.write_u64(END,e.read_u64(BEGIN));e.write_u32(e.read_u64(BEGIN)+168+0x88,0xfedc1234)
  elif fn==0x1405e6a10:
   assert args[0]==o+0x108 and args[2]==s+0x650;stats['grow']+=1
   b=e.read_u64(o+0x108);end=e.read_u64(o+0x110);position=args[1];offset=position-b;assert 0<=offset<=end-b and (end-b)%8==0
   n=end-b;assert n+8<=512;new=buffers[1+h['grows']];h['grows']+=1;assert h['grows']<=4
   data=e.read(b,n);e.write(new,data[:offset]+e.read(args[2],8)+data[offset:]);e.write_u64(o+0x108,new);e.write_u64(o+0x110,new+n+8);e.write_u64(o+0x118,new+8*8);result=new+offset
   if v['mutate']:e.write_u64(END,e.read_u64(BEGIN));e.write_u32(e.read_u64(BEGIN)+168+0x88,0xabcd4321)
  elif fn==0x1423c0920:assert args==[work,0,0x258];e.write(work,bytes(0x258));result=work;stats['work']+=1
  elif fn==0x141bae6d0:assert args==[work];e.write(work,pattern(v['seed']^0x77,32))
  elif fn==0x141b9e840:assert args==[work,e.read_u64(o+0xd0)+0x230];e.write(work+32,pattern(v['seed']^0x88,64))
  elif fn==0x141ba6df0:assert args==[work,e.read_u32(h['cached_token']+0x48)];result=FP[v['fp']];e.uc.reg_write(UC_X86_REG_XMM0,result)
  elif fn==0x141ba63a0:
   assert args[0]==work;result=FP[(v['fp']+3)%len(FP)];e.uc.reg_write(UC_X86_REG_XMM0,result)
   if v['mutate']:e.write_u32(DENOM,FP[(v['fp']+4)%len(FP)])
  elif fn==0x141a66550:
   assert args[0]==e.read_u64(TABLE)+v['target_index']*0x6668 and args[1]==s+0x658 and args[2]==h['cached_base'];e.write_u32(args[1],FP[(v['fp']+5)%len(FP)])
   if v['mutate']:e.write_u64(TABLE,arrays[1]);e.write_u32(h['cached_token']+0x48,0x12345678)
  elif fn==0x141a664d0:
   assert args==[e.read_u64(TABLE)+v['target_index']*0x6668,s+0x650,h['cached_base']];e.write_u32(args[1],FP[(v['fp']+6)%len(FP)])
  elif fn==0x14111da80:
   assert args[0:2]==[o,t] and args[3:]==[1,h['cached_token'],work,o+0xd8,o+0xf0];stats['output']+=1;e.write_u32(o+0x2cc,args[2]);e.write(o+0x18,b'\xa5')
  elif fn==0x14113ff10:assert args==[e.read_u64(o+0xc8)];result=v['secondary']
  elif fn==0x1423c0180:result=int(e.read(args[0],args[2])!=e.read(args[1],args[2]))if args[2]else 0
  else:raise AssertionError(hex(fn))
  h['events'].append(f'I~{fn},{",".join(map(str,args))or"-"}~{initial}~{snap()}~{result}');e.uc.reg_write(UC_X86_REG_RAX,result&((1<<64)-1))
 for fn in[*FUNCS,0x14114d330,0x1417fb320,0x14113fff0,0x141a64260]:
  def dispatch(emu,fn=fn):
   if not h['active']:return old[fn](emu)if fn in old else True
   if fn not in FUNCS:return True
   if fn in[0x141ba6df0,0x141ba63a0]:args=[emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_XMM1)&0xffffffff]
   elif fn==0x141a66550:args=[emu.reg(r)for r in[UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9]]+[e.read_u32(emu.reg(UC_X86_REG_RSP)+0x28)]
   elif fn==0x14111da80:args=[emu.reg(r)for r in[UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8]]+[emu.reg(UC_X86_REG_R9)&255]+[e.read_u64(emu.reg(UC_X86_REG_RSP)+0x28+i*8)for i in range(4)]
   else:args=[emu.reg(r)for r in[UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8][:FUNCS[fn]]]
   leaf(fn,args)
  e.stubs[fn]=dispatch
 def merged(ranges):
  out={}
  for p,n in sorted(ranges.items()):
   if out:
    last=next(reversed(out));end=last+out[last]
    if p<end:out[last]=max(end,p+n)-last;continue
   out[p]=n
  return out
 def prepare(identity,case,v,owned=None,preserve=False):
  nonlocal obj
  obj=identity;h.update(active=False,enabled=True,stack=0,row=None,v={**v,'case':case},ranges={**maps,**(owned or {obj:0x2d0})},events=[],ready_calls=0,manager_calls=0,float_calls=0,grows=0,inactive=False,prepop=False,entries={},returns={})
  if native_compare:h['ranges'].update(record_scope['private'])
  if owned is not None:
   # Only new provider storage is initialized at the outer scenario boundary.
   # At actual nested entry, preserve every inherited physical region.
   if not preserve:
    for p,n in maps.items():
     if any(b<=p and p+n<=b+m for b,m in owned.items()):continue
     if p in[ENGINE,INDEX,TABLE,BEGIN,END]:continue
     e.write(p,bytes([v['seed']])*n)
    for parent in parents:string(parent+0x20,b'parent')
    for a in arrays:
     for i in[0,20]:string(a+i*0x6668+0x6178,b'parent')
    for i,p in enumerate(controls):e.write_u32(p+0x48,FP[(v['fp']+i+case)%len(FP)])
    if DENOM not in owned:e.write_u32(DENOM,0x4ba83308)
   base=e.read_u64(TABLE)
   h['ranges'].update({base+i*0x6668+0x6178:32 for i in[0,20]})
   h['ranges']=merged(h['ranges']);return
  for p,n in h['ranges'].items():e.write(p,bytes([v['seed']])*n)
  for parent in parents:string(parent+0x20,b'parent')
  for p in arrays:
   for i in[0,20]:string(p+i*0x6668+0x6178,b'parent'if v['equal']else b'other')
  e.write(ENGINE,bytes([v['engine']]));e.write_u32(INDEX,v['index']);e.write_u64(TABLE,arrays[0]);e.write_u64(BEGIN,catalog);e.write_u64(END,catalog+v['catalog']*168);e.write_u32(DENOM,v['denom'])
  for i in range(3):e.write_u32(catalog+i*168+0x88,0x12340000+case+i)
  for i,p in enumerate(controls):e.write_u32(p+0x48,FP[(v['fp']+i+case)%len(FP)])
  for i in range(8):e.write_u64(buffers[0]+i*8,catalog+(i%3)*168)
  for off in[0xd8,0xf0]:e.write_u64(obj+off,buffers[0]);e.write_u64(obj+off+8,buffers[0]+16)
  e.write_u64(obj+0x108,buffers[0]);e.write_u64(obj+0x110,buffers[0]+v['items']*8);e.write_u64(obj+0x118,buffers[0]+v['capacity']*8);e.write_u64(obj+0x120,0x12345678);e.write_u32(obj+0x2c8,FP[v['fp']]);e.write(obj+0xc0,bytes([v['pending']]));e.write_u64(obj+0xc8,parents[0]if v['parent']else 0);e.write_u64(obj+0xd0,parents[1]if v['secondary_parent']else 0)
  if native_compare:
   for dst in[obj+0x130,*[p+0x20 for p in parents]]:record_scope['record'](dst,0,dict(kind=1))
   e.write_u32(obj+0x1d0,0x87654321+case%2)
 h['prepare']=prepare
 def begin(uc,a,size,user):
  if before:before(e.reg(UC_X86_REG_RCX),h)
  if h['enabled']:assert e.reg(UC_X86_REG_RCX)==obj;h['active']=True
 def frame(uc,a,size,user):
  if h['active']:h['stack']=e.reg(UC_X86_REG_RSP);h['ranges'].update({h['stack']+0x20:0x5c8,h['stack']+0x650:24});h['ranges']=merged(h['ranges']);h['initial']=snap()
 def finish(uc,a,size,user):
  if not h['active']:return
  assert h['entries']==h['returns'];h['row']=' '.join(map(str,['C',obj,h['stack'],'U214',h['initial'],'->',snap(),'|'.join(h['events'])]));h['active']=False;h['enabled']=False;stats['cases']+=1
  if on_finish:on_finish(h['row'])
 def observe(uc,a,size,user):
  if not h['active']:return
  if a==0x14113d8b3:stats['append']+=1;return
  if a==0x14113d928:h['cached_token']=e.read_u64(h['stack']+0x658);return
  if a==0x14113d9bc:h['cached_base']=e.read_u32(h['cached_token']+0x48);h['v']['target_index']=e.reg(UC_X86_REG_R13)&0xffffffff;return
  fn={0x1417fb327:0x1417fb320,0x14114d390:0x14114d330,0x14114d39d:0x14114d330,0x141140047:0x14113fff0,0x14114004f:0x14113fff0,0x141a6426d:0x141a64260}.get(a)
  if fn:h['returns'][fn]=h['returns'].get(fn,0)+1;h['totals'][fn]=h['totals'].get(fn,0)+1
  else:h['entries'][a]=h['entries'].get(a,0)+1
 for a,hook in[(0x14113d660,begin),(0x14113d688,frame),(0x14113db48,finish),*[(a,observe)for a in[0x14113d8b3,0x14113d928,0x14113d9bc,0x14114d330,0x14114d390,0x14114d39d,0x1417fb320,0x1417fb327,0x14113fff0,0x141140047,0x14114004f,0x141a64260,0x141a6426d]]]:e.uc.hook_add(UC_HOOK_CODE,hook,begin=a,end=a)
 if native_compare:
  from ui_record_compare_harness import install as install_compare
  def compare_begin(left,right,scope):
   assert h['active'];case=h['v']['case']
   scope['prepare'](left,right,case,dict(seed=h['v']['seed'],stage=case%10,temp_kind=[1,3,4,5,6][case%5],alias=(case//5)%3,override=None,mutate=False),dict(h['ranges']))
  record_scope=install_compare(e,before=compare_begin,on_finish=lambda row:h['events'].append('P~'+row.encode().hex()))
  h['compare_stats']=record_scope['stats']
 e.stubs.pop(0x14113d660,None);h['stats']=stats;return h
