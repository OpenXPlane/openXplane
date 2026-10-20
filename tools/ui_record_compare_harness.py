#!/usr/bin/env python3
"""Whole equality/wrapper callers; derived strings, memcmp and free are controlled."""
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RSP
EQUAL,DIFFERENT=0x141a3c910,0x141a3cbc0
OFFSETS=[0,0x20,0x40,0x60,0x80,0xa8,0xc8,0xe8,0x108]
def install(e,before=None,on_finish=None):
 old=e.stubs.copy();heaps=[e.alloc(4200)for _ in range(22)]
 private={p:128 for p in heaps}
 h=dict(active=False,enabled=False,row=None,ranges={},events=[],stack=0,v={},records=[],compares=0,derived=0)
 stats=dict(cases=0,equal_entries=0,equal_returns=0,wrapper_entries=0,wrapper_returns=0,compares=0,derived=0,free=0,zero=0,one=0)
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(h['ranges'].items()))
 def string(p,text,kind,which):
  raw=bytearray(32);capacity=[0,15,16,31,4094,4095,(1<<64)-1][kind]
  raw[16:24]=len(text).to_bytes(8,'little');raw[24:32]=capacity.to_bytes(8,'little')
  if capacity<=15:raw[:len(text)]=text
  else:
   base=heaps[which];data=base+([8,39][h['v']['case']%2]if capacity==4095 else 8)
   e.write_u64(data-8,base);e.write(data,text+b'\0');raw[:8]=data.to_bytes(8,'little')
  e.write(p,raw)
 def record(p,which,v):
  for i,off in enumerate(OFFSETS):string(p+off,v.get('text',b'value'),v['kind'],which*9+i)
  e.write_u32(p+0xa0,0x87654321)
 def prepare(left,right,case,v,owned=None):
  h.update(active=False,enabled=True,row=None,ranges={**private,**(owned or {left:0x198,right:0x198})},events=[],stack=0,v={**v,'case':case},records=[left,right],compares=0,derived=0)
  if owned is not None:return
  for p,n in h['ranges'].items():e.write(p,bytes([v['seed']])*n)
  record(left,0,v);record(right,1,v)
  stage=v['stage'];fields={1:0,2:0x20,3:0x60,6:0xa8,7:0x80,8:0x108}
  if stage in fields:
   off=fields[stage];text=v.get('text',b'value');string(right+off,text[:-1]+b'X'if not v['length']else text[:-1],v['kind'],9+OFFSETS.index(off))
  elif stage==5:e.write_u32(right+0xa0,0x76543210)
  elif stage==9:
   # Explicitly ignored record storage, not a bytewise full-record comparison.
   e.write(right+0x128,bytes([v['seed']^0x11])*0x70);string(right+0x40,b'other',v['kind'],11)
 h['prepare']=prepare;h['record']=record;h['private']=private
 def leaf(fn,args):
  initial=snap();v=h['v'];sp=h['stack'];left,right=h['records'];result=0
  if fn==0x1423c0180:
   h['compares']+=1;stats['compares']+=1
   result=int(e.read(args[0],args[2])!=e.read(args[1],args[2]))
   if h['compares']==3 and v['override']is not None:result=v['override']
   if v['mutate']:e.write(left+0xa4,b'\x7e')
  elif fn==0x141a3df30:
   i=h['derived'];h['derived']+=1;stats['derived']+=1;assert i<2 and args==[[left,right][i],sp+[0x40,0x20][i]]
   text=b'diff'if i and v['stage']==4 else b'same'
   string(args[1],text,v['temp_kind'],18+i)
   result=[args[1],[left,right][i]+0x40,sp+0x20][v['alias']]
   string(result,text,v['temp_kind'],20+i)
  elif fn==0x1422e7c2c:
   stats['free']+=1;result=0xffffffffffffffff
   if v['mutate']:
    # Later free must reload the physical left temporary after this dependency.
    string(sp+0x40,b'reload',2,18);e.write_u32(left+0xa0,0xabcdef01)
  else:raise AssertionError(hex(fn))
  h['events'].append(f'I~{fn},{",".join(map(str,args))}~{initial}~{snap()}~{result}')
  e.uc.reg_write(UC_X86_REG_RAX,result&((1<<64)-1))
 for fn,n in[(0x1423c0180,3),(0x141a3df30,2),(0x1422e7c2c,2)]:
  def dispatch(emu,fn=fn,n=n):
   if not h['active']:return old[fn](emu)if fn in old else True
   leaf(fn,[emu.reg(r)for r in[UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8][:n]])
  e.stubs[fn]=dispatch
 def enter(uc,a,size,user):
  if a==DIFFERENT:
   if before:before(e.reg(UC_X86_REG_RCX),e.reg(UC_X86_REG_RDX),h)
   if h['enabled']:h['active']=True;h['v']['inverted']=1;stats['wrapper_entries']+=1
  elif h['active']:stats['equal_entries']+=1
  elif h['enabled']:h['active']=True;h['v']['inverted']=0;stats['equal_entries']+=1
 def frame(uc,a,size,user):
  if not h['active']:return
  h['stack']=e.reg(UC_X86_REG_RSP);h['ranges'].update({h['stack']+0x20:64,h['stack']+0x70:4});h['initial']=snap()
 def finish(uc,a,size,user):
  if not h['active']:return
  if a==0x141a3cba7:
   stats['equal_returns']+=1
   if h['v']['inverted']:return
  else:stats['wrapper_returns']+=1
  result=e.reg(UC_X86_REG_RAX);assert result in[0,1]
  stats['one'if result else'zero']+=1;stats['cases']+=1
  h['row']=' '.join(map(str,['C',*h['records'],h['stack'],h['v']['inverted'],h['initial'],'->',snap(),result,'|'.join(h['events'])or'-']))
  h['active']=False;h['enabled']=False
  if on_finish:on_finish(h['row'])
 for a,f in[(EQUAL,enter),(DIFFERENT,enter),(0x141a3c924,frame),(0x141a3cba7,finish),(0x141a3cbcf,finish)]:e.uc.hook_add(UC_HOOK_CODE,f,begin=a,end=a)
 e.stubs.pop(EQUAL,None);e.stubs.pop(DIFFERENT,None);h['stats']=stats;return h
