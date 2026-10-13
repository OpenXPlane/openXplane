#!/usr/bin/env python3
"""Local full 20x18x3 body grid conversion replay, with original SSE instructions."""
import hashlib,random,struct,sys
from pathlib import Path
from emulate_xp import Emulator
from unicorn.x86_const import UC_X86_REG_RBX,UC_X86_REG_RSI
SHA='2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
def main(path):
 if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=SHA:raise ValueError('reference digest mismatch')
 e=Emulator(path);mesh=e.alloc(0x1200);rng=random.Random(20261019)
 print('# reference_sha256',SHA);print('# U 1080 input words -> 1080 outputs; original 0x141aac3bf..0x141aac4aa exclusive; no stubs; full fixed grid and untouched guards')
 edge=[0,0x80000000,1,0x80000001,0x00800000,0x80800000,0x7f7fffff,0xff7fffff,0x3f800000,0xbf800000]
 for case in range(64):
  words=[edge[i%len(edge)] for i in range(1080)] if case==0 else [struct.unpack('<I',struct.pack('<f',rng.uniform(-1000,1000)))[0] for _ in range(1080)]
  e.write(mesh,b'\xa5'*0x1200);e.write(mesh+0x2c,struct.pack('<1080I',*words))
  e.uc.reg_write(UC_X86_REG_RBX,mesh);e.uc.reg_write(UC_X86_REG_RSI,20)
  e.run_range(0x141aac3bf,0x141aac4aa,max_instructions=10000)
  assert not e.stubs and e.read(mesh,0x2c)==b'\xa5'*0x2c and e.read(mesh+0x110c,0xf4)==b'\xa5'*0xf4
  output=struct.unpack('<1080I',e.read(mesh+0x2c,4320))
  print('U',*[f'{x:08x}' for x in words],'->',*[f'{x:08x}' for x in output])
 print('# cases',64)
if __name__=='__main__':main(sys.argv[1])
