#!/usr/bin/env python3
"""Whole child factory and four original constructors. Controlled leaf effects.
Numeric corpus remains local. Callback RB backing is deliberately opaque.
"""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RSP,UC_X86_REG_RIP,UC_X86_REG_XMM0,UC_X86_REG_XMM2
HEAD,BEGIN,END,ACTIVE=0x14588f558,0x145e4dab8,0x145e4dac0,0x145a10798
CTORS=[0x1410d6a40,0x1410d6fc0,0x1410d7490,0x1410d75a0]
SIZES=[0x3e0,0x2d0,0x60,0x3e8]
RETURNS=[0x1410d6cd7,0x1410d6d0a,0x1410d6d3d,0x1410d6d76]
def install(e,root_harness=None,native_context=False,native_fifth=False,native_predicate=False,native_tables=False):
 bundle=e.alloc(40);catalog=e.alloc(168*3);resource=root_harness['resource'] if root_harness else e.alloc(0x6190);selected=e.alloc(32);top=e.heap_top;v={};events=[];blocks={};objects=[];temps=[];original_stubs=e.stubs.copy()
 fifth_scope=None
 def ret(x):e.uc.reg_write(UC_X86_REG_RAX,x&((1<<64)-1))
 def snap():return e.read(bundle,40).hex()+';'+('/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(blocks.items()))or'-')
 def effect(fn,args,apply=lambda:None,result=0):
  kind=v['kind'];p=objects[kind] if kind>=0 else v['fifth'];n=SIZES[kind] if kind>=0 else 0xb0;before=e.read(p,n).hex();apply();events.append(f'I~{fn},{",".join(map(str,args))or"-"}~{before}~{e.read(p,n).hex()}~{result}');return result
 def allocate(emu):
  n=emu.reg(UC_X86_REG_RCX);kind=v['kind']
  if root_harness and kind==0:original_stubs[0x1422e7bf0](emu);return
  p=root_harness['identity'] if root_harness and kind==-1 and n==0x3e0 else emu.alloc(n)
  if kind>=0:
   i=v['allocation'];v['allocation']+=1;initial=[7,3,2,2][kind]
   if i<initial:
    assert n==[[120,32,120,72,72,120,40],[120,40,32],[120,40],[120,40]][kind][i]
    events.append(f'H~{n},{p}~{e.read(objects[kind],SIZES[kind]).hex()}')
    if n!=40:blocks[p]=n
   elif n==120:events.append(f'L~120,{p}');blocks[p]=16
   else:assert n==40;events.append('A~40')
  else:
   assert n in [*SIZES,0xb0,120,32,128];events.append(f'O~{n},{p}~{snap()}');blocks[p]=n
   if n==0xb0:v['fifth']=p
  emu.write(p,bytes([v['seed']^0x5a])*n);ret(p)
 def free(emu):
  if root_harness and v['kind']==0:original_stubs[0x1422e7c2c](emu);return
  assert emu.reg(UC_X86_REG_RDX)==40;events.append('D~40')
 e.stubs.update({0x1422e7bf0:allocate,0x1422e7c2c:free})
 def enter(uc,a,size,user):
  k=CTORS.index(a);assert k==len(objects);p=e.reg(UC_X86_REG_RCX);objects.append(p);v.update(kind=k,allocation=0)
  args=[e.reg(UC_X86_REG_RDX)];args[0]=args[0]&255 if k==0 else args[0]
  if k==3:args.append(e.reg(UC_X86_REG_R8))
  events.append(f'B~{k},{p},{",".join(map(str,args))}')
  if root_harness and k==0:root_harness['begin']()
 def leave(uc,a,size,user):
  k=RETURNS.index(a);assert e.reg(UC_X86_REG_RAX)==objects[k]
  if root_harness and k==0:
   row,owned=root_harness['finish']();events.append('Q~'+row.replace(' ','!').replace('|','^'));blocks.update(owned)
  events.append('E~'+snap());v['kind']=-1
 for a in CTORS:e.uc.hook_add(UC_HOOK_CODE,enter,begin=a,end=a)
 for a in RETURNS:e.uc.hook_add(UC_HOOK_CODE,leave,begin=a,end=a)
 def temporary(uc,a,size,user):
  if not root_harness or v['kind']!=0:temps.append(e.reg(UC_X86_REG_RCX))
 e.uc.hook_add(UC_HOOK_CODE,temporary,begin=0x1410c7d20,end=0x1410c7d20)
 def leaf(emu,fn):
  k=v['kind']
  if root_harness and k==0:original_stubs[fn](emu);return
  if native_tables and k==-1 and fn==0x141a64260:return True
  p=objects[k] if k>=0 else v['fifth'];rcx=emu.reg(UC_X86_REG_RCX);rdx=emu.reg(UC_X86_REG_RDX);r8=emu.reg(UC_X86_REG_R8);result=0
  if fn==0x14113a300:assert rcx==p+0x268;effect(fn,[rcx],lambda:emu.write(p+0x398,bytes([v['seed']^0x21])))
  elif fn==0x141138cf0:assert rcx==p;effect(fn,[p],lambda:emu.write(p+0x3c0,bytes([v['seed']^0x22])))
  elif fn==0x140c8f750:assert (rcx,rdx,r8)==(p+0x120,1,0x142643100);effect(fn,[rcx,1,r8],lambda:emu.write(p+0xb8,bytes([v['mode']])))
  elif fn==0x1417fb320:assert rcx==0x142f2d770;result=effect(fn,[rcx],result=v['engine'])
  elif fn==0x141a64260:
   assert rcx==0x146124398 and rdx==((e.read_u32(0x142640504) if root_harness else 0) if k==1 else 20),(k,hex(rcx),rdx)
   if k==1:result=effect(fn,[rcx,rdx],result=resource)
   else:events.append(f'R~{rdx}~{snap()}~{resource}');result=resource
  elif fn in [0x14111bb90,0x14113ba10]:
   assert rcx==p;effect(fn,[p],lambda:emu.write_u64(p+0xd0,objects[0]+0x1e0 if v['secondary']else 0))
  elif fn==0x140c80d50:assert rcx==0x145a120c0;result=effect(fn,[rcx],result=12345)
  elif fn==0x14163b530:assert rcx==12345;result=effect(fn,[rcx],result=v['index'])
  elif fn==0x14113a460:assert rcx==p and rdx==0;effect(fn,[p,0],lambda:emu.write_u32(p+0x2c8,0x11223344))
  elif fn==0x14113d660:assert rcx==p;effect(fn,[p],lambda:emu.write_u32(p+0x2cc,v['seed']))
  elif fn==0x1411396d0:
   assert rcx==p
   def apply():emu.write_u32(p+0x38,90);emu.write_u32(p+0x3c,v['bits']);emu.write(p+0x50,bytes([v['seed']]))
   effect(fn,[p],apply)
  elif fn==0x141b77b60:
   off=rcx-p;bits=emu.reg(UC_X86_REG_XMM2)&0xffffffff;assert off in [0x58,0x230] and rdx==0
   def apply():
    if v['weather']:emu.write(p+off,bytes((v['seed']+i+bits%251)%256 for i in range(0x1a8)))
   effect(fn,[off,0,bits],apply)
  elif fn==0x14111d6a0:
   assert rcx==objects[1];effect(fn,[rcx],result=v['bits']);emu.uc.reg_write(UC_X86_REG_XMM0,v['bits']);return
  elif fn==0x141135970:
   assert rcx==objects[2];effect(fn,[rcx],result=e.read(selected,32).hex());ret(selected);return
  elif fn==0x141139fe0:
   assert rcx==p
   def apply():emu.write_u64(p+0x3e0,objects[v['rebind']]);emu.write_u32(p+0x23c,7 if v['special']else 3)
   effect(fn,[p],apply)
  elif fn==0x141149360:assert rcx==p;effect(fn,[p],lambda:emu.write_u64(p+0x228,0x123456789abcdef0))
  elif fn==0x141139460:assert k==-1 and rcx==p;effect(fn,[p],lambda:emu.write(p+0x28,bytes([v['seed']])))
  else:raise AssertionError(hex(fn))
  ret(result)
 for fn in [0x14113a300,0x141138cf0,0x140c8f750,0x1417fb320,0x141a64260,0x14111bb90,0x14113ba10,0x140c80d50,0x14163b530,0x14113a460,0x14113d660,0x1411396d0,0x141b77b60,0x14111d6a0,0x141135970,0x141139fe0,0x141149360,0x141139460]:e.stubs[fn]=lambda emu,fn=fn:leaf(emu,fn)
 if root_harness:
  e.stubs.pop(0x14113a300,None);e.stubs.pop(0x141138cf0,None)
 if native_context:
  from ui_factory_context_refresh import install as install_context
  install_context(e,resource,objects,events,v)
 if native_fifth:
  from ui_fifth_refresh_harness import install as install_fifth,OVERRIDE,BITS
  def fifth_begin(p,h):
   case=v['case'];profile=dict(seed=v['seed'],engine=v['engine'],active=e.read_u64(ACTIVE),index=e.read_u32(0x142640504),available=int(e.read_u64(resource+0x6188)!=0),items=[0,1,3][case%3],variant=[0,255][case%2],mutate=case%2,listeners=0,override=[0,1,0x80000000][case%3],bits=[0x80000000,0x3f800000,0x7fc12345][case%3])
   h.prepare(p,case,profile,dict(blocks),mutate=False)
  fifth_scope=install_fifth(e,resource,before=fifth_begin,on_finish=lambda row:events.append('W~'+row.replace(' ','!').replace('|','^')),native_predicate=native_predicate,settings=root_harness.get('settings') if root_harness else None,tables=root_harness.get('tables') if root_harness and native_tables else None);top=e.heap_top
 if native_tables:
  def constructor_record_return(uc,a,size,user):
   if v['kind']==-1 and (not fifth_scope or not fifth_scope.active):v['constructor_returns']+=1
  e.uc.hook_add(UC_HOOK_CODE,constructor_record_return,begin=0x141a6426d,end=0x141a6426d)
 def entries(parent):
  head=e.read_u64(parent+8);out=[];p=e.read_u64(head);previous=head
  while p!=head:assert p not in out and e.read_u64(p+8)==previous;out.append(p);previous=p;p=e.read_u64(p)
  assert len(out)==e.read_u64(parent+16) and e.read_u64(head+8)==previous;return head,out
 def graph(parents):
  containers=[objects[0]+0x288,objects[1]+0x38,objects[2]+0x20,objects[3]+0x20];live=set(containers)
  for parent in parents:live.update(p+16 for p in entries(parent)[1])
  def peers(p):
   head=e.read_u64(p+8);seen=set();out=[]
   def walk(n):
    if e.read(n+25,1)[0]:assert n==head;return
    assert n not in seen;seen.add(n);walk(e.read_u64(n));out.append(e.read_u64(n+32));walk(e.read_u64(n+16))
   walk(e.read_u64(head+8));assert out==sorted(set(out)) and len(out)==e.read_u64(p+16);return out
  out=[]
  for p in sorted(live):
   base=p in containers;a,b=(0,0)if base else(e.read_u64(p+24),e.read_u64(p+32));ps=peers(p);assert all(x in live for x in ps);out.append(f'{p}:{"B"if base else"W"}:{a}:{b}:{",".join(map(str,ps))or"-"}')
  return ';'.join(out)
 def run_case(seed,mode,engine,secondary,index,case):
  e.heap_top=top;events.clear();blocks.clear();objects.clear();temps.clear();v.update(seed=seed,kind=-1,mode=mode,engine=engine,secondary=secondary,index=index,bits=[0,0x80000000,0x3f800000,0x7f800000,0x7fc12345][case%5],case=case,constructor_returns=0,weather=case%2,rebind=1+case%2,special=case%2);e.write(bundle,bytes([seed])*40)
  e.write(catalog,bytes(168*3))
  for j in range(3):e.write_u64(catalog+j*168+16,0x1122334400000000+j)
  e.write_u64(BEGIN,catalog);e.write_u64(END,catalog+168*3);e.write_u64(HEAD,0);
  if not root_harness:e.write_u64(ACTIVE,0);e.write_u64(resource+0x6188,1)
  e.write(selected,bytes((seed+j*7)%256 for j in range(32)))
  if native_fifth:e.write_u32(OVERRIDE,0x12345678);e.write_u32(BITS,0x11112222)
  result=e.call(0x1410d6ca0,ints=[bundle],max_instructions=300000);assert result==bundle and e.reg(UC_X86_REG_RIP)==RET_ADDR and len(objects)==4
  if native_tables:assert v['constructor_returns']==1
  parents=[objects[0],objects[0]+0x1e0,objects[0]+0x268,*objects[1:],v['fifth']]
  lists=';'.join(f'{p}:{entries(p)[0]}:{",".join(map(str,entries(p)[1]))or"-"}'for p in parents)
  # The selection may create one or two temporary wrappers; context one, weather two.
  assert len(temps)==4+secondary
  return ' '.join(map(str,['C',seed,catalog,','.join(map(str,temps)),'->',snap(),graph(parents),lists,e.read_u64(HEAD),'|'.join(events)]))
 run_case.native_getter_totals=fifth_scope.native_totals if fifth_scope else {}
 return run_case

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);run_case=install(e)
 print('# reference_sha256',REFERENCE_SHA256)
 print('# full factory1410d6ca0 + all four ctors + original callback/list/data/hash initialization; refresh/provider/allocator endpoints controlled; callback RB bytes opaque; successful allocations/fresh lists only')
 count=0
 for seed,mode,engine,secondary,index in itertools.product([0,0xa5,255],[0,1,2,3,4,255],[0,255],[0,1],[-2147483648,0,2147483647]):
  print(run_case(seed,mode,engine,secondary,index,count));count+=1
 print('# cases',count)
if __name__=='__main__':main(sys.argv[1])
