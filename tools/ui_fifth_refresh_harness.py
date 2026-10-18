#!/usr/bin/env python3
"""Original fifth refresh/hash-clear with controlled external providers.
Snapshots and numeric traces are local only; no original bytes are exported.
"""
from types import SimpleNamespace
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9,UC_X86_REG_RSP
ACTIVE,INDEX,OVERRIDE,BITS=0x145a10798,0x142640504,0x142f069a0,0x142f069a4
COUNTS={0x1406215a0:3,0x1417fb320:1,0x141a64260:2,0x14193ae40:0,0x141660c60:2,0x1405dcc70:3,0x1406999d0:1,0x1422e7c2c:2,0x1407925b0:4,0x14113b9a0:0,0x1417f71d0:3,0x1410fac50:2,0x1405ddae0:1,0x1408ace50:1,0x1410d2f10:3,0x1410c9d10:1}
def install(e,resource=None,before=None,on_finish=None,native_predicate=False,settings=None,tables=None):
 resource=resource or e.alloc(0x6190);vec=e.alloc(64);grown=e.alloc(128);payload=e.alloc(32);records=[e.alloc(128)for _ in range(3)];tree=e.alloc(32);tnodes=[e.alloc(32)for _ in range(3)];tree2=e.alloc(32);hnodes=[e.alloc(32)for _ in range(3)];hhead2=e.alloc(32);buckets=e.alloc(1024);targets=[e.alloc(32)for _ in range(3)];listeners=[e.alloc(120)for _ in range(3)];head2=e.alloc(120)
 if tables:resource=tables.resource(88);settings=tables.settings(88)
 if native_predicate:settings=settings or e.alloc(0xdb0)
 external={resource+0x6188:8,vec:64,grown:128,payload:32,tree:32,tree2:32,hhead2:32,buckets:1024,head2:120,**{p:128 for p in records},**{p:32 for p in tnodes+hnodes+targets},**{p:120 for p in listeners}}
 if native_predicate:external[settings+0xdac]=4
 if tables:external.update(tables.maps)
 h=SimpleNamespace(active=False,enabled=False,before=before,on_finish=on_finish,obj=0,events=[],ranges={},v={},row=None,stack=0,in_clear=False,native_entries={},native_returns={},native_totals={});old=e.stubs.copy();writes=[];reads=[]
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(h.ranges.items()))+f';{e.read_u64(ACTIVE)};{e.read_u32(INDEX)};{e.read_u32(OVERRIDE)};{e.read_u32(BITS)}'
 def chunks(xs):return '/'.join(f'{p}:{b.hex()}'for p,b in xs)or'-'
 def write(p,b):
  e.write(p,b)
  if h.stack<=p and p+len(b)<=h.stack+0xe0:writes.append((p,b))
 def read(p,n):b=e.read(p,n);reads.append((p,b));return b
 def q(p):return int.from_bytes(read(p,8),'little')
 def put(p,n):write(p,(n&((1<<64)-1)).to_bytes(8,'little'))
 def prepare(p,case,v,owned,mutate=False):
  h.obj=p;h.v=dict(v);h.v.update(case=case,position=0,lookup=0,engine_calls=0,resource_calls=0);h.events.clear();h.ranges={**owned,**external};h.stack=0;h.enabled=True;h.in_clear=False;seed=v['seed'];h.row=None;h.native_entries.clear();h.native_returns.clear()
  for a,n in external.items():
   if (not tables or a not in tables.maps) and a!=resource+0x6188 and (not native_predicate or a!=settings+0xdac):e.write(a,bytes([seed^0x33])*n)
  for i,record in enumerate(records):
   index=[-1,0,2][case%3];e.write_u64(record+(index+5)*8,[9,7,9][i])
  def chain(head,nodes,n):
   e.write_u64(head,nodes[0]if n else head);e.write_u64(head+8,nodes[n-1]if n else head)
   for i,a in enumerate(nodes):e.write_u64(a,nodes[i+1]if i+1<n else head);e.write_u64(a+8,nodes[i-1]if i else head)
  chain(tree,tnodes,[0,1,3][case%3]);chain(tree2,[],0);chain(hhead2,[],0);chain(head2,[],0)
  if mutate:e.write_u32(OVERRIDE,0x12345678);e.write_u32(BITS,0x11112222)
  if mutate and native_predicate:e.write_u32(settings+0xdac,0 if v['variant']&255 else [1,7,0xffffffff][case%3])
  if mutate:
   count=[0,1+2*(case%2),3][v['hash']];bucket_count=64 if v['hash']==2 else 8;head=e.read_u64(p+0x58);chain(head,hnodes,count)
   e.write_u64(p+0x60,count);e.write_u64(p+0x68,buckets);e.write_u64(p+0x70,buckets+bucket_count*16);e.write_u64(p+0x78,buckets+bucket_count*16);e.write_u64(p+0x80,bucket_count-1);e.write_u64(p+0x88,bucket_count)
   for off in range(0,bucket_count*16,8):e.write_u64(buckets+off,head)
   e.write_u64(p+0x98,vec);e.write_u64(p+0xa0,vec+min([0,1,3][case%3],v['capacity'])*8);e.write_u64(p+0xa8,vec+v['capacity']*8)
   # Initial retained elements fit capacity; refresh clears them before appending.
   e.write_u64(p+0x38,payload);e.write_u64(p+0x40,payload+16);e.write_u64(p+0x48,payload+32);e.write(p+0x94,bytes([case%3*127]))
   head=e.read_u64(p+8);chain(head,listeners,3 if v['listeners']else 0);e.write_u64(p+16,3 if v['listeners']else 0)
   e.write_u64(ACTIVE,999);e.write_u32(INDEX,0x12345678);e.write_u64(resource+0x6188,v['available'])
  if tables:tables.prepare(case)
 h.prepare=prepare
 def leaf(fn,args):
  before=snap();writes.clear();reads.clear();o=h.obj;sp=h.stack;v=h.v;result=0
  if fn==0x1406215a0:
   table,first,head=args;assert first==e.read_u64(head);e.write_u64(head,head);e.write_u64(head+8,head);e.write_u64(table+16,0)
   for a in range(e.read_u64(table+24),e.read_u64(table+32),8):e.write_u64(a,head)
  elif fn==0x1417fb320:
   v['engine_calls']+=1;gate=native_predicate and v['engine_calls']>1
   e.write_u64(ACTIVE,v.get('gate_active',v['active']) if gate else v['active']);e.write_u32(INDEX,v.get('gate_index',v['index']) if gate else v['index']);result=v.get('gate_engine',v['engine']) if gate else v['engine']
   if native_predicate:e.write_u32(OVERRIDE,v['override']);e.write_u32(BITS,v['bits'])
   if tables and gate and v.get('table_shift'):
    e.write_u64(0x146124398,tables.resource_base+0x6668);e.write_u64(0x1461243b0,tables.settings_base+0x433c0)
  elif fn==0x141a64260:
   assert args==[0x146124398,e.read_u32(INDEX)];result=resource;v['resource_calls']+=1
   if native_predicate and v['mutate']:e.write_u32(INDEX,(e.read_u32(INDEX)+1)&0xffffffff)
  elif fn==0x141a64250:assert args==[0x1461243b0,e.read_u32(INDEX)];result=settings
  elif fn==0x14193ae40:result=12345
  elif fn==0x141660c60:
   assert args==[sp+0x20,12345];raw=bytearray([v['seed']^0x77]*64);raw[:8]=(records[0]if v['items']else 0).to_bytes(8,'little');raw[8:12]=([-1,0,2][v['case']%3]&0xffffffff).to_bytes(4,'little');raw[0x30:0x38]=tree.to_bytes(8,'little');write(sp+0x20,bytes(raw))
  elif fn==0x1405dcc70:
   vector,end,item=args;assert vector==o+0x98 and end==e.read_u64(vector+8);value=q(item);begin=e.read_u64(vector);n=(end-begin)//8
   if n:e.write(grown,e.read(begin,n*8))
   e.write_u64(grown+n*8,value);e.write_u64(vector,grown);e.write_u64(vector+8,grown+(n+1)*8);e.write_u64(vector+16,grown+128)
  elif fn==0x1406999d0:
   assert args==[sp+0x20];v['position']+=1;put(sp+0x20,records[v['position']]if v['position']<v['items']else 0)
   if v['mutate']and v['position']==1:
    end=e.read_u64(o+0xa0);cap=e.read_u64(o+0xa8)
    if end+8<=cap:e.write_u64(end,0xbeef);e.write_u64(o+0xa0,end+8)
  elif fn==0x1422e7c2c:
   p,n=args;assert n==32
   if v['mutate']:
    if h.in_clear and p==hnodes[0]:e.write_u64(p,hhead2);e.write_u64(o+0x58,hhead2)
    elif not h.in_clear and p==tnodes[0]:e.write_u64(p,tree2);put(sp+0x50,tree2)
  elif fn==0x1407925b0:
   begin,end,count,zero=args;assert count==(end-begin)//8 and zero==0
   values=sorted(e.read_u64(a)for a in range(begin,end,8))
   for i,n in enumerate(values):e.write_u64(begin+i*8,n)
  elif fn==0x14113b9a0:e.write_u32(OVERRIDE,v['override']);e.write_u32(BITS,v['bits']);result=v['variant']
  elif fn==0x1417f71d0:
   assert args==[sp+0x20,0,12345];write(sp+0x20,bytes((v['seed']+i)%256 for i in range(64)));result=payload if v['case']%2 else sp+0x20
  elif fn==0x1410fac50:assert args[0]==o+0x30;read(args[1],32);e.write(o+0x30,bytes((v['seed']^0x42,))+bytes(31))
  elif fn==0x1405ddae0:assert args==[sp+0x40];read(args[0],32)
  elif fn==0x1408ace50:assert args==[sp+0x28];read(args[0],24)
  elif fn==0x1410d2f10:
   assert args==[o+0x50,sp+0x90,sp+0xc0];value=q(args[2]);target=targets[value%3];put(args[1],target);e.write(target+24,b'\xff');result=args[1]
   if v['mutate']and v['lookup']==0:
    begin=e.read_u64(o+0x98);end=e.read_u64(o+0xa0)
    if begin+8<=end:e.write_u64(o+0x98,begin+8)
    if begin+16<=end:e.write_u64(begin+8,999)
   v['lookup']+=1
  elif fn==0x1410c9d10:
   p=args[0]-16;e.write(o+0x18,b'\xa5')
   if v['listeners']==2 and p==listeners[0]:e.write_u64(o+8,head2);e.write_u64(p,head2)
  else:raise AssertionError(hex(fn))
  h.events.append(f'I~{fn},{",".join(map(str,args))or"-"}~{before}~{snap()}~{result}~{chunks(reads)}~{chunks(writes)}');e.uc.reg_write(UC_X86_REG_RAX,result&((1<<64)-1))
 providers=dict(COUNTS)
 if native_predicate:
  assert 0x14113b9a0 not in e.stubs
  providers.pop(0x14113b9a0);providers[0x141a64250]=2
 for fn,n in providers.items():
  def dispatch(emu,fn=fn,n=n):
   if h.active and tables and fn in [0x141a64250,0x141a64260]:return True
   if h.active:leaf(fn,[emu.reg(r)for r in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9][:n]])
   elif fn in old:return old[fn](emu)
   else:return True
  e.stubs[fn]=dispatch
 def begin(uc,a,size,user):
  p=e.reg(UC_X86_REG_RCX)
  if h.before:h.before(p,h)
  if h.enabled:assert p==h.obj;h.active=True;h.initial=snap()
 def capture(uc,a,size,user):
  if h.active:h.stack=e.reg(UC_X86_REG_RSP);h.initial_stack=e.read(h.stack,0xe0).hex()
 def clear_scope(uc,a,size,user):
  if h.active:h.in_clear=a==0x14062b900
 def finish(uc,a,size,user):
  if not h.active:return
  if tables:assert h.native_entries==h.native_returns
  h.row=' '.join(map(str,['C',h.obj,h.stack,h.initial,h.initial_stack,'->',snap(),'|'.join(h.events)]));h.active=False;h.enabled=False
  if h.on_finish:h.on_finish(h.row)
 e.uc.hook_add(UC_HOOK_CODE,begin,begin=0x141139460,end=0x141139460);e.uc.hook_add(UC_HOOK_CODE,capture,begin=0x14113947b,end=0x14113947b);e.uc.hook_add(UC_HOOK_CODE,finish,begin=0x1411396b2,end=0x1411396b2)
 for a in [0x14062b900,0x14113948f]:e.uc.hook_add(UC_HOOK_CODE,clear_scope,begin=a,end=a)
 if tables:
  def record_entry(uc,a,size,user):
   if h.active:h.native_entries[a]=h.native_entries.get(a,0)+1
  def record_return(uc,a,size,user):
   if h.active:
    fn={0x141a6425d:0x141a64250,0x141a6426d:0x141a64260}[a];h.native_returns[fn]=h.native_returns.get(fn,0)+1;h.native_totals[fn]=h.native_totals.get(fn,0)+1
  for a in [0x141a64250,0x141a64260]:e.uc.hook_add(UC_HOOK_CODE,record_entry,begin=a,end=a)
  for a in [0x141a6425d,0x141a6426d]:e.uc.hook_add(UC_HOOK_CODE,record_return,begin=a,end=a)
 e.stubs.pop(0x141139460,None)
 return h
