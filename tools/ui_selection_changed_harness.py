#!/usr/bin/env python3
"""Whole selection-change body; only notification callbacks controlled."""
from types import SimpleNamespace
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RAX

def install(e,before=None,on_finish=None):
 head=e.alloc(120);nodes=[e.alloc(120)for _ in range(3)];head2=e.alloc(120);external={head:120,head2:120,**{p:120 for p in nodes}};old=e.stubs.get(0x1410c9d10)
 h=SimpleNamespace(active=False,enabled=False,obj=0,events=[],ranges={},row=None,v={},entries=0,returns=0,total_returns=0,before=before,on_finish=on_finish)
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(h.ranges.items()))
 def prepare(obj,case,v,owned=None):
  h.obj=obj;h.v=dict(v);h.v['case']=case;h.events.clear();h.row=None;h.enabled=True;h.entries=h.returns=0;h.ranges={**(owned or {obj:0x2d0}),**external}
  for p,n in external.items():e.write(p,bytes([v['seed']^0x33])*n)
  e.write_u64(head2,head2);e.write_u64(head2+8,head2)
  if not owned:
   count=[0,1,3,3,3,3,1][v['layout']];e.write_u64(obj+8,head);e.write_u64(obj+16,count);e.write(obj+0x18,bytes([v['dirty']]));e.write_u64(head,nodes[0]if count else head);e.write_u64(head+8,nodes[count-1]if count else head)
   for i,p in enumerate(nodes):e.write_u64(p,nodes[i+1]if i+1<count else head);e.write_u64(p+8,nodes[i-1]if i else head)
 h.prepare=prepare
 def notify(emu):
  if not h.active:
   if old:return old(emu)
   return True
  wrapper=emu.reg(UC_X86_REG_RCX);p=wrapper-16;assert p in nodes;initial=snap();v=h.v;o=h.obj
  if v['effect']:e.write(o+0x18,bytes([0 if v['effect']==1 else 0xa5]));e.write_u64(o+0xd0,0x12340000+v['case'])
  if p==nodes[0]:
   if v['layout']==3:e.write_u64(p,nodes[2]);e.write_u64(nodes[2]+8,p);e.write_u64(o+16,2)
   if v['layout']==4:e.write_u64(o+8,head2);e.write_u64(o+16,0);e.write_u64(p,head2)
   if v['layout']==5:e.write_u64(o+8,nodes[1]);e.write_u64(o+16,0);e.write_u64(p,nodes[1]);e.write_u64(nodes[1],nodes[1]);e.write_u64(nodes[1]+8,nodes[1])
   if v['layout']==6:e.write_u64(p,nodes[1]);e.write_u64(nodes[1],nodes[2]);e.write_u64(nodes[1]+8,p);e.write_u64(nodes[2],head);e.write_u64(nodes[2]+8,nodes[1]);e.write_u64(head+8,nodes[2]);e.write_u64(o+16,3)
  h.events.append(f'I~{wrapper}~{initial}~{snap()}');emu.uc.reg_write(UC_X86_REG_RAX,0)
 def begin(uc,a,size,user):
  obj=e.reg(UC_X86_REG_RCX)
  if h.before:h.before(obj,h)
  if h.enabled:assert obj==h.obj;h.active=True;h.entries+=1;h.initial=snap()
 def finish(uc,a,size,user):
  if not h.active:return
  h.returns+=1;h.total_returns+=1;assert h.entries==h.returns==1
  h.row=' '.join(map(str,['C',h.obj,h.initial,'->',snap(),'|'.join(h.events)or'-']));h.active=False;h.enabled=False
  if h.on_finish:h.on_finish(h.row)
 e.stubs[0x1410c9d10]=notify;e.stubs.pop(0x14113ba10,None)
 e.uc.hook_add(UC_HOOK_CODE,begin,begin=0x14113ba10,end=0x14113ba10);e.uc.hook_add(UC_HOOK_CODE,finish,begin=0x14113ba4c,end=0x14113ba4c)
 return h
