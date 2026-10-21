#!/usr/bin/env python3
"""Actual class-string, label constructor and reached shrink branch. Leaf effects controlled."""
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9,UC_X86_REG_RSP
BUILD,LABEL,RESIZE=0x141a3df30,0x141a3cd30,0x14064e290
LABEL_RETS=[0x141a3cd8a,0x141a3cdcc,0x141a3ce02,0x141a3ce3a,0x141a3ce74,0x141a3cea4,0x141a3ced4,0x141a3cf21,0x141a3cf56,0x141a3cf7f,0x141a3cfb9,0x141a3cff2,0x141a3d02a]
def install(e,before=None,on_finish=None,native_strings=False):
 old=e.stubs.copy();heads=[e.alloc(40)for _ in range(2)];nodes=[[e.alloc(40)for _ in range(20)]for _ in range(2)];label_heap=e.alloc(64);grow_heap=e.alloc(64);extra=e.alloc(4200);alias=e.alloc(32);outputs=[e.alloc(1024)for _ in range(2)]
 private={**{p:40 for p in heads},**{p:40 for group in nodes for p in group},label_heap:64,grow_heap:64,extra:128,alias:32,**{p:1024 for p in outputs},0x142505110:1}
 h=dict(active=False,enabled=False,mode='D',ranges={},events=[],row=None,stack=0,record=0,out=0,index=0,v={},labels=0,label_returns=0,resizes=0,resize_returns=0)
 stats=dict(cases=0,builds=0,labels=0,label_returns=0,resize=0,resize_returns=0,allocate=0,grow=0,append=0,free=0)
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(h['ranges'].items()))
 def string(p,text,data=None,capacity=15):
  b=bytearray(32);b[16:24]=len(text).to_bytes(8,'little');b[24:32]=capacity.to_bytes(8,'little')
  if capacity>15:b[:8]=data.to_bytes(8,'little');e.write(data,text+b'\0')
  else:b[:len(text)]=text
  e.write(p,b)
 def contents(p):
  n=e.read_u64(p+16);data=e.read_u64(p)if e.read_u64(p+24)>15 else p;assert n<=1023
  return e.read(data,n)
 def tree(slot,mask,shape=0,nil=1):
  head=heads[slot];keys=sorted([-2147483648,-1,*[i for i in range(12)if mask&(1<<i)],12,2147483647]);pool=iter(nodes[slot])
  def branch(items,parent):
   if not items:return head
   k=[len(items)//2,0,len(items)-1][shape];p=next(pool);e.write(p,bytes(40));e.write_u64(p+8,parent);e.write_u32(p+0x1c,items[k]&0xffffffff)
   e.write_u64(p,branch(items[:k],p));e.write_u64(p+0x10,branch(items[k+1:],p));return p
  e.write(head,bytes(40));e.write(head+0x19,bytes([nil]));root=branch(keys,head);e.write_u64(head+8,root);return head
 def seed(record,mask,other,shape=0,nil=1):
  for slot,bits in enumerate([mask,other]):tree(slot,bits,shape,nil)
  e.write_u64(record+0x180,heads[0])
 h.update(private=private,heads=heads,seed=seed,tree=tree)
 def prepare(record,out,case,v,owned=None,mode='D',index=0):
  h.update(active=False,enabled=True,mode=mode,ranges={**private,**(owned or {record:0x198,out:32})},events=[],row=None,stack=0,record=record,out=out,index=index,v={**v,'case':case},labels=0,label_returns=0,resizes=0,resize_returns=0,selector=out&255,label_out=0)
  if native_strings:strings['reset']()
  if owned is not None:return
  for p,n in h['ranges'].items():
   if p!=0x142505110:e.write(p,bytes([v['seed']])*n)
  if mode=='D':
   seed(record,v['mask'],v['other'],v['shape'],[1,255][case%2])
   if not v['mask']:e.write_u64(heads[0]+8,heads[0])
 h['prepare']=prepare
 def leaf(fn,args):
  initial=snap();v=h['v'];sp=h['stack'];result=0
  if fn==0x1422e7bf0:
   assert args==[32];stats['allocate']+=1;result=label_heap
   if v['mutate']:
    e.write(h['label_out']+8,b'\x7e'*8)
    if h['mode']=='D':e.write_u64(h['record']+0x180,heads[1])
  elif fn==0x1405eca90:
   assert args==[sp+0x58,1,h['selector'],0x142505110,1]and e.read(0x142505110,1)==b'/';stats['grow']+=1
   text=contents(args[0])+b'/';result=sp+0x30 if v['grow_alias']==2 else alias if v['grow_alias']else args[0];string(result,text,grow_heap,31)
  elif fn==0x140624040:
   assert args[0]==h['out'];stats['append']+=1
   text=contents(h['out'])+e.read(args[1],args[2]);tail=v['tail']
   if tail==1:text=text.replace(b'/',b'')
   elif tail==2:text=text+b'tail'
   elif tail==3:text=b''
   if len(text)>15:string(h['out'],text,outputs[(h['out']>>5)&1],1023)
   else:string(h['out'],text)
   result=h['out']
   if v['mutate']:e.write_u64(h['record']+0x180,heads[1]);e.write_u64(sp+0xa8,0xfedcba9876543200)
   if v['dirty']:
    data=extra+39;e.write_u64(data-8,extra);string(sp+0x58,b'dirty',data,4095)
  elif fn==0x1422e7c2c:
   stats['free']+=1;result=0xffffffffffffffff
   if h['mode']=='D'and v['dirty']:string(sp+0x58,b'reload',extra+8,16)
  else:raise AssertionError(hex(fn))
  h['events'].append(f'I~{fn},{",".join(map(str,args))}~{initial}~{snap()}~{result}');e.uc.reg_write(UC_X86_REG_RAX,result)
 for fn,n in[(0x1422e7bf0,1),(0x1405eca90,5),(0x140624040,3),(0x1422e7c2c,2)]:
  def dispatch(emu,fn=fn,n=n):
   if not h['active']:return old[fn](emu)if fn in old else True
   args=[emu.reg(r)for r in[UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9][:min(n,4)]]
   if n==5:args.append(e.read_u64(emu.reg(UC_X86_REG_RSP)+0x28))
   leaf(fn,args)
  e.stubs[fn]=dispatch
 def entry(uc,a,size,user):
  if a==BUILD:
   if before:before(e.reg(UC_X86_REG_RCX),e.reg(UC_X86_REG_RDX),h)
   if h['enabled']:h['active']=True;stats['builds']+=1
  elif a==LABEL:
   if not h['active']and h['enabled']and h['mode']=='L':h['active']=True;h['initial']=snap()
   if h['active']:h['labels']+=1;stats['labels']+=1;h['label_out']=e.reg(UC_X86_REG_RCX)
  elif h['active']:
   assert e.reg(UC_X86_REG_RCX)==h['out']and e.reg(UC_X86_REG_R8)==0;h['resizes']+=1;stats['resize']+=1
 def frame(uc,a,size,user):
  if not h['active']or h['mode']!='D':return
  h['stack']=e.reg(UC_X86_REG_RSP);h['ranges'].update({h['stack']+0x20:0x58,h['stack']+0xa8:8});assert e.read_u64(h['stack']+0xa8)==h['out'];h['initial']=snap()
 def done():
  assert h['labels']==h['label_returns']and h['resizes']==h['resize_returns'];result=e.reg(UC_X86_REG_RAX);assert result==h['out'];stats['cases']+=1
  start=['L',h['out'],h['index']]if h['mode']=='L'else['D',h['record'],h['out'],h['stack']]
  h['row']=' '.join(map(str,[*start,h['initial'],'->',snap(),result,'|'.join(h['events'])or'-']));h['active']=False;h['enabled']=False
  if on_finish:on_finish(h['row'])
 def returns(uc,a,size,user):
  if not h['active']:return
  if a in LABEL_RETS:
   h['label_returns']+=1;stats['label_returns']+=1
   if h['mode']=='L':done()
  elif a==0x14064e2b3:h['resize_returns']+=1;stats['resize_returns']+=1
  elif a==0x141a3e180:done()
  else:raise AssertionError('unverified growing resize branch')
 for a,f in[(BUILD,entry),(LABEL,entry),(RESIZE,entry),(0x141a3df50,frame),*[(a,returns)for a in[LABEL_RETS[0],*LABEL_RETS[1:],0x14064e2b3,0x14064e305,0x14064e325,0x141a3e180]]]:e.uc.hook_add(UC_HOOK_CODE,f,begin=a,end=a)
 for fn in[BUILD,LABEL,RESIZE]:e.stubs.pop(fn,None)
 if native_strings:
  from ui_string_append_harness import install as install_strings
  def strings_before(mode,args,child):
   assert h['active'] and h['mode']=='D'
   child['prepare'](mode,args,h['v']['case'],dict(seed=h['v']['seed'],mutate=False),owned=h['ranges'])
   h['active']=False
  def strings_done(row,child):
   for p in child['pools']:h['ranges'][p]=child['ranges'][p]
   h['events'].append('S~'+row.encode().hex());h['active']=True
  strings=install_strings(e,before=strings_before,on_finish=strings_done)
  private.update(strings['private']);h['strings']=strings
 h['stats']=stats;return h
