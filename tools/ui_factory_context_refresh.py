#!/usr/bin/env python3
"""Actual context refresh inside the child factory; providers stay controlled.
The subtrace uses the existing context replay's fields and adds shared index.
"""
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RAX,UC_X86_REG_XMM0
from ui_context_refresh_harness import KEYS,GLOBAL,bits
INDEX=0x142640504

def install(e,resource,objects,events,owner):
 scope={};trace=[];old=e.stubs.copy()
 def obj():return objects[2]
 def links():
  p=e.read_u64(obj()+8);assert e.read_u64(p)==p
  return f'{p}={e.read_u64(p)}'
 def state():return f'{e.read(obj(),96).hex()};{e.read_u64(GLOBAL)};{links()}'
 def log(kind,args='-'):trace.append(f'{kind}~{args}~{state()}')
 def result(x):e.uc.reg_write(UC_X86_REG_RAX,x&((1<<64)-1))
 def begin(uc,a,size,user):
  assert owner['kind']==2;scope.clear();trace.clear();seed=owner['seed'];case=owner['case'];active=e.read_u64(GLOBAL);index=e.read_u32(INDEX);flags=[255 if case&1 else 0,255 if case&2 else 0];mode=[0,1,2,255][case//4%4];layout=case%3;v=[owner['engine'],active,active,int(e.read_u64(resource+0x6188)!=0),*flags,mode,layout,0,0,case//16%2]
  alt=bytearray((seed+17+i)%256 for i in range(32));alt[24]=mode^0x55;default=bytearray((seed+31+i)%256 for i in range(32));default[24]=mode;context=bytes((seed+51+i)%256 for i in range(16));strings=[]
  for key in range(2):
   n=[5,16,4096][layout];raw=bytearray([seed^0x33]*32);data=b'123.5'+b' '*(n-5)+b'\0';base=0;storage=b'';cap=15 if n<=15 else n|15
   if cap==15:raw[:len(data)]=data
   else:
    size=cap+1;large=size>=4096;base=e.alloc(size+(39 if large else 0));p=(base+39)&~31 if large else base;e.write(base,bytes([seed^0x5a])*(size+(39 if large else 0)))
    if large:e.write_u64(p-8,base)
    e.write(p,data);storage=e.read(base,size+(39 if large else 0));raw[:8]=p.to_bytes(8,'little')
   raw[16:24]=n.to_bytes(8,'little');raw[24:32]=cap.to_bytes(8,'little');strings.append((bytes(raw),base,storage))
  zs=[bits(3600.125),bits(-float('inf')),bits(86400.0039),bits(float('inf')),0x7ff8000000000001,bits(-0.),bits(12345.6789)]
  ds=[bits(366.75),bits(-1.),bits(2147483648.),bits(float('inf')),0xfff8000012345678,bits(-0.),bits(365.9)]
  code=[-1,0,42,2147483647][case%4];hour=[0x80000000,0x41480000,0x7fc12345][case%3];numbers=[zs[case%len(zs)],ds[case%len(ds)]]
  fields=[seed,','.join(map(str,v)),*numbers,code,hour,e.read(obj(),96).hex(),links(),bytes(alt).hex(),bytes(default).hex(),context.hex(),*[f'{raw.hex()}:{base}:{storage.hex()if storage else"-"}'for raw,base,storage in strings]]
  scope.update(active=True,v=v,strings=strings,alt=bytes(alt),default=bytes(default),context=context,code=code,hour=hour,numbers=numbers,key=0,index=index,fields=fields)
 def end(uc,a,size,user):
  assert scope['active'];row=' '.join(map(str,['C',*scope['fields'],'->',e.read(obj(),96).hex(),e.read_u64(GLOBAL),links(),'|'.join(trace),scope['index'],e.read_u32(INDEX)]));events.append('U~'+row.replace(' ','!').replace('|','^'));scope['active']=False
 e.uc.hook_add(UC_HOOK_CODE,begin,begin=0x1411396d0,end=0x1411396d0)
 e.uc.hook_add(UC_HOOK_CODE,end,begin=0x1410d7522,end=0x1410d7522)
 def leaf(fn):
  c,d,r=e.reg(UC_X86_REG_RCX),e.reg(UC_X86_REG_RDX),e.reg(UC_X86_REG_R8)
  if fn==0x1417fb320:assert c==0x142f2d770;log('E');e.write_u64(GLOBAL,scope['v'][2]);result(scope['v'][0])
  elif fn==0x141a64260:assert (c,d)==(0x146124398,e.read_u32(INDEX));log('R',str(d));result(resource)
  elif fn==0x1417f74f0:log('A');e.write(c,scope['alt']);result(c)
  elif fn==0x1414d1170:log('M');result(12345)
  elif fn==0x1414cda30:assert c==12345;log('B','12345');e.write(d,scope['default']);result(d)
  elif fn==0x140d776a0:assert c==0x142f02ba8;k=KEYS[d];log('H',str(k));result(scope['v'][4+k])
  elif fn==0x141126f60:log('C',str(c));e.write(d,scope['context']);result(d)
  elif fn==0x1417f9180:assert c==obj()+0x38;log('D',f'{e.read(c,32).hex()},{e.read(d,16).hex()}');result(scope['code'])
  elif fn==0x140d77340:assert c==0x142f02ba8;k=KEYS[r];log('G',str(k));e.write(d,scope['strings'][k][0]);scope.update(key=k,dest=d);result(d)
  elif fn==0x142311c58:
   k=scope['key'];raw=scope['strings'][k][0];cap=int.from_bytes(raw[24:],'little');expected=int.from_bytes(raw[:8],'little')if cap>15 else scope['dest'];assert c==expected and d==0;log('P',f'{k},{raw.hex()}');e.uc.reg_write(UC_X86_REG_XMM0,scope['numbers'][k])
  elif fn==0x1422e7c2c:
   log('F',f'{c},{d}')
   if scope['key']==1 and scope['v'][10]:e.write(obj()+0x50,b'\x01')
  elif fn==0x1417f8960:assert c==obj()+0x38;log('V',f'{e.read(c,32).hex()},{e.read(r,16).hex()}');e.write_u32(d,scope['hour'])
  elif fn==0x1410c9d10:raise AssertionError('fresh context listener list must stay empty')
  else:raise AssertionError(hex(fn))
 for fn in [0x1417fb320,0x141a64260,0x1417f74f0,0x1414d1170,0x1414cda30,0x140d776a0,0x141126f60,0x1417f9180,0x140d77340,0x142311c58,0x1422e7c2c,0x1417f8960,0x1410c9d10]:
  def dispatch(emu,fn=fn):
   if scope.get('active'):leaf(fn)
   elif fn in old:return old[fn](emu)
   else:return True
  e.stubs[fn]=dispatch
 e.stubs.pop(0x1411396d0,None)
