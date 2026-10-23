#!/usr/bin/env python3
"""Actual secondary/primary/membership/equality/getters; four controlled leaves."""
import hashlib,itertools,sys
from pathlib import Path
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_RIP
from emulate_xp import Emulator,RET_ADDR
from extract_commands import REFERENCE_SHA256

FUNCTIONS={'S':0x14113ff10,'P':0x14113fff0,'E':0x14073ba40,'G':0x14114d330}
INDEX,TABLE,GLOBALS,ENGINE=0x142640504,0x146124398,0x145899fd0,0x142f2dc95

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);receiver=e.alloc(0x1c0);owner=e.alloc(0x2d0);heads=[e.alloc(40)for _ in range(2)];nodes=[e.alloc(40)for _ in range(2)]
 arrays=[e.alloc(0x6668*3)for _ in range(2)];left_heap=e.alloc(64);header=e.alloc(32);header_heap=e.alloc(64);right_heaps=[e.alloc(64)for _ in range(6)];singletons=[e.alloc(32)for _ in range(3)]
 ranges={receiver:0x1c0,owner:0x2d0,**{p:40 for p in heads+nodes},left_heap:64,header:32,header_heap:64,**{p:64 for p in right_heaps},**{p:32 for p in singletons},INDEX:8,TABLE:8,GLOBALS:16,ENGINE:1}
 for array in arrays:
  for i in range(3):ranges[array+i*0x6668+0x6178]=32
 h={};stats=dict(cases=0,S=0,P=0,E=0,G=0,secondary=0,secondary_return=0,primary=0,primary_return=0,member=0,member_return=0,equal=0,equal_return=0,getter=0,getter_return=0,singleton=0,query=0,header=0,memcmp=0)
 def snap():return '/'.join(f'{p}:{e.read(p,n).hex()}'for p,n in sorted(ranges.items()))
 def string(p,text,heap=0,empty_null=False):
  raw=bytearray(32);raw[16:24]=len(text).to_bytes(8,'little');raw[24:32]=(63 if heap else 15).to_bytes(8,'little')
  if heap:
   raw[:8]=(0 if empty_null and not text else heap).to_bytes(8,'little');e.write(heap,text+b'\0')
  else:raw[:len(text)]=text
  e.write(p,raw)
 def selected(array,index):return array+(0x6668 if index==1 else 0)
 def base(array,index):return array+(0x6668 if index==0xffffffff else 0)
 def leaf(fn,n):
  args=[e.reg(r)for r in[UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8][:n]];initial=snap();v=h['v'];result=0
  if fn==0x1423c0180:
   i=h['compares'];h['compares']+=1;stats['memcmp']+=1
   result=int(e.read(args[0],args[2])!=e.read(args[1],args[2]))if args[2]else 0
   override=v['overrides'][min(i,len(v['overrides'])-1)]
   if override is not None:result=override
   if v['mutation'] and h['mode']!='E':e.write_u64(receiver+0x1a0,heads[1]);e.write_u64(owner+0xc8,receiver+0x80);e.write(owner+0xc0,b'\xff')
  elif fn==0x1414cc7f0:
   i=h['queries'];assert i<3;stats['singleton']+=1;result=singletons[i]+v['case']%16
  elif fn==0x1414cedd0:
   i=h['queries'];assert args==[singletons[i]+v['case']%16];h['queries']+=1;stats['query']+=1;result=v['modes'][i]
   if v['mutation']:
    if i==0:e.write_u32(GLOBALS+8,0 if v['case']%2 else 0x100);e.write_u32(GLOBALS+12,0xffffffff if v['case']%3==0 else 0)
    if i==1:e.write_u32(GLOBALS,0x100 if v['case']%3==1 else 0);e.write_u32(GLOBALS+4,0xffffffff if v['case']%5==0 else 0)
    if i==2:e.write_u32(INDEX+4,v['late_index']);e.write_u64(TABLE,base(arrays[1],v['late_index']))
  elif fn==0x1411222e0:
   assert args==[receiver];stats['header']+=1;result=header
   if v['mutation']:
    e.write_u64(TABLE,arrays[0]);e.write_u32(INDEX+4,1)
    if v['mutation']==2:
     assert h['cached_resource'] is not None
     string(h['cached_resource']+0x6178,b'post-header',right_heaps[5])
  else:raise AssertionError(hex(fn))
  h['events'].append(f'I~{fn}{"" if not args else ","+",".join(map(str,args))}~{initial}~{snap()}~{result}');e.uc.reg_write(UC_X86_REG_RAX,result)
 for fn,n in[(0x1423c0180,3),(0x1414cc7f0,0),(0x1414cedd0,1),(0x1411222e0,1)]:
  def stub(emu,fn=fn,n=n):leaf(fn,n)
  e.stubs[fn]=stub
 def count(uc,a,size,user):
  name=points[a];stats[name]+=1
  if name=='primary':h['primary_active']=True
  if name=='primary_return':h['primary_active']=False
  if name=='getter_return'and not h['primary_active']:h['cached_resource']=e.reg(UC_X86_REG_RAX)
 points={0x14113ff10:'secondary',0x14113ff89:'secondary_return',0x14113ffdb:'secondary_return',0x14113ffe3:'secondary_return',0x14113fff0:'primary',0x141140047:'primary_return',0x14114004f:'primary_return',0x141a400f0:'member',0x141a4014d:'member_return',0x141a40155:'member_return',0x14073ba40:'equal',0x14073ba74:'equal_return',0x14073ba7b:'equal_return',0x141a64260:'getter',0x141a6426d:'getter_return'}
 for p in points:e.uc.hook_add(UC_HOOK_CODE,count,begin=p,end=p)
 for p in[*FUNCTIONS.values(),0x141a400f0,0x141a64260,0x1417fb320]:e.stubs.pop(p,None)
 def run(mode,v):
  case=stats['cases'];v={**v,'case':case,'late_index':[0,1,0xffffffff][case%3]};h.update(mode=mode,v=v,queries=0,compares=0,events=[],primary_active=False,cached_resource=None)
  for p,n in ranges.items():e.write(p,bytes([[0,0xa5,255][case%3]])*n)
  for i,p in enumerate(heads):
   e.write(p,bytes(40));e.write(p+0x19,bytes([[1,255][case%2]]));e.write_u64(p+8,nodes[i]if (v['present']or i)else p)
   e.write(nodes[i],bytes(40));e.write_u64(nodes[i],p);e.write_u64(nodes[i]+0x10,p);e.write_u32(nodes[i]+0x1c,2)
  e.write_u64(receiver+0x1a0,heads[0]);e.write_u64(owner+0xc8,receiver);e.write(owner+0xc0,bytes([case%256]));e.write(ENGINE,b'\x01')
  index=0 if v.get('literal_indices')else[0,1,0xffffffff][case%3];e.write_u32(INDEX,index);e.write_u32(INDEX+4,1 if v.get('literal_indices')else index);e.write_u64(TABLE,base(arrays[0],index))
  for i in range(4):e.write_u32(GLOBALS+i*4,([1,0x100,0xffffffff][case%3]if v['flags']&(1<<i)else 0))
  text=[b'ab\0cd',b'ab\0cd',b'ab\0cd',b'0123456789abcdefghij',b'',b'\xff\x80\0\xa0\xfe'][v['profile']]
  for a,array in enumerate(arrays):
   for j in range(3):
    value=(b'late'+text)if a and text else (b'literal'+text)if v.get('literal_indices')and j==1 and text else text
    string(array+j*0x6668+0x6178,value,right_heaps[a*3+j]if a or v['profile']in[3,4]else 0,empty_null=v['profile']==4)
  string(receiver+0x20,text,left_heap if v['profile']in[3,4,5]else 0,empty_null=v['profile']==4)
  if v['primary_diff']:string(receiver+0x20,b'different',0)
  final_text=b'post-header'if v['mutation']==2 else ((b'late'+text)if v['mutation']and text else ((b'literal'+text)if v.get('literal_indices')and text else text))
  if v['profile']==1:final_text=final_text[:-1]+b'X'
  if v['profile']==2:final_text=final_text[:-1]
  string(header,final_text,header_heap if len(final_text)>15 or v['profile']==4 else 0,empty_null=v['profile']==4)
  args=[receiver]if mode in['S','P']else[header,selected(arrays[0],index)+0x6178]if mode=='E'else[owner]
  initial=snap();result=e.call(FUNCTIONS[mode],ints=args,max_instructions=10000);assert e.reg(UC_X86_REG_RIP)==RET_ADDR
  if mode=='G':result&=255;assert result in[0,1]
  stats['cases']+=1;stats[mode]+=1
  print(' '.join(map(str,['V',mode,','.join(map(str,args)),initial,'->',snap(),result,'|'.join(h['events'])or'-'])))
 print('# reference_sha256',REFERENCE_SHA256);print('# actual secondary/primary/membership/resource/equality/gate/engine; singleton/mode/header/memcmp controlled; non-stack maps, synthetic index/global mutations; G compares AL only, other modes full RAX')
 modes=[[0,1][bool(bits&(1<<i))]|([0x1234567800000000,0,0xabcdef0000000000][i])for bits in range(8)for i in range(3)]
 sequences=[modes[i:i+3]for i in range(0,len(modes),3)]
 for bits,flags,profile,mutation,present in itertools.product(range(8),range(16),range(6),range(3),range(2)):
  run('S',dict(modes=sequences[bits],flags=flags,profile=profile,mutation=mutation,present=present,primary_diff=0,overrides=[None,None]))
 for mode,profile,diff,override in itertools.product(['S','P','E'],range(6),range(2),[None,0,0xffffffff,0x100000000,0x7654321000000100,0xffffffffffffffff]):
  run(mode,dict(modes=sequences[7],flags=0,profile=profile,mutation=0,present=1,primary_diff=diff,overrides=[override,override]))
 for bits,flags,profile,mutation,present in itertools.product(range(8),[0,1,2,4,8,15],range(6),range(3),range(2)):
  run('G',dict(modes=sequences[bits],flags=flags,profile=profile,mutation=mutation,present=present,primary_diff=0,overrides=[None,None,None]))
 for mode,bits,profile,mutation in itertools.product(['S','G'],range(8),range(6),range(3)):
  run(mode,dict(modes=sequences[bits],flags=0,profile=profile,mutation=mutation,present=1,primary_diff=0,overrides=[None,None,None],literal_indices=True))
 for mode,bits,flags,mutation in itertools.product(['S','G'],range(8),[0,1,2,4,8,15],range(3)):
  sequence=[([0xff,0x102,0xdeadbeef00000033][i]if bits&(1<<i)else[0x100,0x100000000,0xabcdef0000000000][i])for i in range(3)]
  run(mode,dict(modes=sequence,flags=flags,profile=stats['cases']%6,mutation=mutation,present=1,primary_diff=0,overrides=[None,None,None]))
 assert stats['secondary']==stats['secondary_return']==6984
 assert stats['primary']==stats['primary_return']and stats['member']==stats['member_return']and stats['equal']==stats['equal_return']and stats['getter']==stats['getter_return']
 print('# stats',stats)
if __name__=='__main__':main(sys.argv[1])
