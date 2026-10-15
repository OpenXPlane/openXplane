#!/usr/bin/env python3
"""Original whole dialog child-factory1410d6ca0, including inline fifth object.
Four child ctors/resource lookup/final refresh/allocator controlled. Original
empty hash vector resize executes unchanged. Corpus remains local-only.
"""
import hashlib,itertools,sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RAX,UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8

def main(path):
 assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
 e=Emulator(path);bundle=e.alloc(40);top=e.heap_top;blocks=[];events=[];values={};outer=[0]
 def snap():return '~'.join([e.read(bundle,40).hex(),*[f'{p}={e.read(p,n).hex()}' for p,n in sorted(blocks)]])
 def allocate(emu):
  n=emu.reg(UC_X86_REG_RCX);assert n in [0x3e0,0x2d0,0x60,0x3e8,0xb0,120,32,128]
  fail=False
  if n in [0x3e0,0x2d0,0x60,0x3e8,0xb0]:fail=bool(values['mask']&(1<<outer[0]));outer[0]+=1
  before=snap();p=0 if fail else emu.alloc(n)
  events.append(f'A:{n}:{p}:{before}')
  if p:emu.write(p,bytes([values['seed']^0x5a])*n);blocks.append((p,n))
  emu.uc.reg_write(UC_X86_REG_RAX,p)
 def nested(kind,emu):
  p=emu.reg(UC_X86_REG_RCX);args=[emu.reg(UC_X86_REG_RDX)]
  if kind==0:args[0]&=255
  if kind==3:args.append(emu.reg(UC_X86_REG_R8))
  result=p+values['shift'];events.append(f'N:{kind}:{p}:{",".join(map(str,args))}:{result}:{snap()}')
  for off,value in [(0,0x11110000+kind),(8,args[0]),(16,args[1] if len(args)>1 else 0)]:emu.write_u64(p+off,value)
  emu.uc.reg_write(UC_X86_REG_RAX,result)
 def resource(emu):
  assert emu.reg(UC_X86_REG_RCX)==0x146124398 and emu.reg(UC_X86_REG_RDX)==20
  events.append(f'R:20:{values["resource"]}:{snap()}');emu.uc.reg_write(UC_X86_REG_RAX,values['resource'])
 def refresh(emu):
  p=emu.reg(UC_X86_REG_RCX);events.append(f'Z:{p}:{values["seed"]}:{snap()}');emu.write_u64(p+0x98,values['seed']);emu.write(p+0x28,bytes([values['seed']]))
 e.stubs.update({0x1422e7bf0:allocate,0x141a64260:resource,0x141139460:refresh})
 for kind,addr in enumerate([0x1410d6a40,0x1410d6fc0,0x1410d7490,0x1410d75a0]):e.stubs[addr]=lambda emu,k=kind:nested(k,emu)
 print('# reference_sha256',REFERENCE_SHA256)
 print('# full outer factory+inline fifth/container/hash-vector resize original; four nested ctors/resource/final refresh/allocator controlled; outer allocation-null mask0..31; internal allocations successful')
 count=0
 for seed,mask,shift,resource_id in itertools.product([0,0xa5,255],range(32),[0,16],[0,7,2**64-1]):
  values.update(seed=seed,mask=mask,shift=shift,resource=resource_id);e.heap_top=top;blocks.clear();events.clear();outer[0]=0;e.write(bundle,bytes([seed])*40)
  result=e.call(0x1410d6ca0,ints=[bundle],max_instructions=100000);assert result==bundle and outer[0]==5
  print('C',seed,mask,shift,resource_id,'->',snap(),'|'.join(events));count+=1
 print('# cases',count)
if __name__=='__main__':main(sys.argv[1])
