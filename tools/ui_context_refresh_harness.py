#!/usr/bin/env python3
"""Whole original1411396d0, controlled engine/context/options/CRT/notifications.
Local-only corpus. Original branch, float conversion/divides, cleanup and loop run.
"""
import hashlib,itertools,random,struct
from types import SimpleNamespace
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RAX,UC_X86_REG_XMM0
GLOBAL=0x145a10798
KEYS={0x1426432d8:0,0x1426432e8:1}
def bits(v):return struct.unpack('<Q',struct.pack('<d',v))[0]
def harness(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);obj=e.alloc(96);heads=[e.alloc(120) for _ in range(2)];nodes=[e.alloc(120) for _ in range(4)];all_links=sorted(heads+nodes);resource=e.alloc(0x6190);alternate=e.alloc(32);default=e.alloc(32);top=e.heap_top;values={};events=[]
 def links():return ','.join(f'{p}={e.read_u64(p)}' for p in all_links) or '-'
 def state():return f'{e.read(obj,96).hex()};{e.read_u64(GLOBAL)};{links()}'
 def log(kind,args='-'):events.append(f'{kind}~{args}~{state()}')
 def result(v):e.uc.reg_write(UC_X86_REG_RAX,v&((1<<64)-1))
 def engine(emu):assert emu.reg(UC_X86_REG_RCX)==0x142f2d770;log('E');emu.write_u64(GLOBAL,values['v'][2]);result(values['v'][0])
 def get_resource(emu):assert (emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX))==(0x146124398,0);log('R','0');result(resource)
 def alt(emu):log('A');emu.write(emu.reg(UC_X86_REG_RCX),values['alt']);result(alternate)
 def manager(emu):log('M');result(12345)
 def get_default(emu):assert emu.reg(UC_X86_REG_RCX)==12345;log('B','12345');emu.write(emu.reg(UC_X86_REG_RDX),values['default']);result(default)
 def has(emu):
  assert emu.reg(UC_X86_REG_RCX)==0x142f02ba8;k=KEYS[emu.reg(UC_X86_REG_RDX)];log('H',str(k));result(values['v'][4+k])
 def parent(emu):log('C',str(emu.reg(UC_X86_REG_RCX)));emu.write(emu.reg(UC_X86_REG_RDX),values['context']);result(emu.reg(UC_X86_REG_RDX))
 def context_day(emu):
  assert emu.reg(UC_X86_REG_RCX)==obj+0x38;context=emu.read(emu.reg(UC_X86_REG_RDX),16);log('D',f'{emu.read(obj+0x38,32).hex()},{context.hex()}');result(values['code'])
 def option(emu):
  assert emu.reg(UC_X86_REG_RCX)==0x142f02ba8;k=KEYS[emu.reg(UC_X86_REG_R8)];dst=emu.reg(UC_X86_REG_RDX);log('G',str(k));emu.write(dst,values['strings'][k][0]);values['key']=k;values['dest']=dst;result(dst)
 def parse(emu):
  k=values['key'];raw=values['strings'][k][0];cap=int.from_bytes(raw[24:],'little');expected=int.from_bytes(raw[:8],'little') if cap>15 else values['dest'];assert emu.reg(UC_X86_REG_RCX)==expected and emu.reg(UC_X86_REG_RDX)==0
  log('P',f'{k},{raw.hex()}');emu.uc.reg_write(UC_X86_REG_XMM0,values['numbers'][k])
 def free(emu):
  p,n=emu.reg(UC_X86_REG_RCX),emu.reg(UC_X86_REG_RDX);log('F',f'{p},{n}')
  if values['key']==1 and values['v'][10]:
   emu.write(obj+0x50,bytes([1 if values['v'][10]==1 else 255]))
   if values['v'][10]==2:emu.write_u64(obj+0x58,emu.read_u64(obj+0x58)+1)
 def context_hour(emu):
  assert emu.reg(UC_X86_REG_RCX)==obj+0x38;context=emu.read(emu.reg(UC_X86_REG_R8),16);log('V',f'{emu.read(obj+0x38,32).hex()},{context.hex()}');emu.write_u32(emu.reg(UC_X86_REG_RDX),values['hour'])
 def notify(emu):
  wrapper=emu.reg(UC_X86_REG_RCX);log('N',str(wrapper));p=wrapper-16;mode=values['v'][9]
  if p==nodes[0]:
   if mode==1:emu.write_u64(p,emu.read_u64(obj+8))
   elif mode==2:emu.write_u64(obj+8,heads[1]);emu.write_u64(p,heads[1])
   elif mode==3:emu.write_u64(p,nodes[3])
  emu.write(obj+0x18,b'\xa5')
 e.stubs.update({0x1417fb320:engine,0x141a64260:get_resource,0x1417f74f0:alt,0x1414d1170:manager,0x1414cda30:get_default,0x140d776a0:has,0x141126f60:parent,0x1417f9180:context_day,0x140d77340:option,0x142311c58:parse,0x1422e7c2c:free,0x1417f8960:context_hour,0x1410c9d10:notify})
 return SimpleNamespace(e=e,obj=obj,heads=heads,nodes=nodes,all_links=all_links,resource=resource,alternate=alternate,default=default,top=top,values=values,events=events,links=links,state=state)

def cases():
 rows=[]
 for eng,active,res,flags,mode,layout in itertools.product([0,255],[0,7],[0,1],[(0,0),(255,0),(0,255),(255,255)],[0,1,2,255],range(3)):
  i=len(rows);rows.append(([eng,active,active,res,*flags,mode,layout,i%4,(i//4)%4,(i//16)%3],bits(3600.125),bits(366.75)))
 zs=[bits(x) for x in [-float('inf'),-1.,-0.,0.,1.,59.999,60.,3599.999,3600.,86399.999,86400.,86401.,float('inf'),1e300]]+[0x7ff8000000000001,0xfff8000012345678,0x7ff0000000000001,0xfff0000000000001]+[bits(x) for x in [0.1,12345.6789,86400.0039,1e-300,1e30,-1e300]]
 ds=[bits(x) for x in [-float('inf'),-1.,-0.,0.,.9,1.,365.9,366.,367.,2147483520.,2147483647.,2147483648.,float('inf'),1e300]]+[0x7ff8000000000001,0xfff8000012345678,0x7ff0000000000001,0xfff0000000000001]+[bits(x) for x in [366.999,255.75,1e-300,1e30,-1e30,-2147483649.]]
 for (z,d),mode,layout in itertools.product(zip(zs,ds),[0,1,2,255],range(3)):
  i=len(rows);rows.append(([0,0,0,0,255,255,mode,layout,i%4,(i//4)%4,0],z,d))
 rng=random.Random(2026101601)
 for i in range(300):
  v=[rng.choice([0,1,255]),rng.choice([0,7]),rng.choice([0,7]),rng.randrange(2),rng.choice([0,1,255]),rng.choice([0,1,255]),rng.choice([0,1,2,128,255]),rng.randrange(3),rng.randrange(4),rng.randrange(4),rng.randrange(3)]
  rows.append((v,rng.choice(zs+[bits(rng.uniform(-1e10,1e10))]),rng.choice(ds+[bits(rng.uniform(-1e10,1e10))])))
 return rows

def prepare(h,i,row):
 e,obj,heads,nodes,all_links,resource,alternate,default,top,values,events,links=(h.e,h.obj,h.heads,h.nodes,h.all_links,h.resource,h.alternate,h.default,h.top,h.values,h.events,h.links)
 v,z,d=row
 seed=[0,0xa5,255][i%3];e.heap_top=top;events.clear();image=bytearray([seed]*96);image[8:16]=heads[0].to_bytes(8,'little');image[0x58:0x60]=(98765).to_bytes(8,'little');e.write(obj,bytes(image));e.write_u64(GLOBAL,v[1])
 for p in all_links:e.write_u64(p,p if p in heads else heads[0])
 for j in range(v[8]):e.write_u64(nodes[j],nodes[j+1] if j+1<v[8] else heads[0])
 e.write_u64(heads[0],nodes[0] if v[8] else heads[0]);initial_links=links()
 a=bytearray((seed+17+j)&255 for j in range(32));a[24]=v[6]^0x55;b=bytearray((seed+31+j)&255 for j in range(32));b[24]=v[6];context=bytes((seed+51+j)&255 for j in range(16));e.write(alternate,bytes(a));e.write(default,bytes(b));e.write_u64(resource+0x6188,v[3]);strings=[]
 for key in range(2):
  length=[5,16,4096][v[7]];raw=bytearray([seed^0x33]*32);data=b'123.5'+b' '*(length-5)+b'\0';base=0;storage=b'';cap=15 if length<=15 else length|15
  if cap==15:raw[:len(data)]=data
  else:
   size=cap+1;large=size>=4096;base=e.alloc(size+(39 if large else 0));p=(base+39)&~31 if large else base;e.write(base,bytes([seed^0x5a])*(size+(39 if large else 0)))
   if large:e.write_u64(p-8,base)
   e.write(p,data);storage=e.read(base,size+(39 if large else 0));raw[:8]=p.to_bytes(8,'little')
  raw[16:24]=length.to_bytes(8,'little');raw[24:32]=cap.to_bytes(8,'little');strings.append((bytes(raw),base,storage))
 values.update(v=v,alt=bytes(a),default=bytes(b),context=context,code=[-1,0,42,2147483647][i%4],hour=[0x80000000,0x41480000,0x7fc12345][i%3],strings=strings,numbers=[z,d])
 records=[f'{raw.hex()}:{base}:{storage.hex() if storage else "-"}' for raw,base,storage in strings]
 return [seed,','.join(map(str,v)),z,d,values['code'],values['hour'],bytes(image).hex(),initial_links,bytes(a).hex(),bytes(b).hex(),context.hex(),*records]
