#!/usr/bin/env python3
"""Paired EXE/DLL keyboard routing with actual registered mutable sniffer callbacks."""
import hashlib
import random
import struct
import sys
from pathlib import Path
from emulate_xp import Emulator, RET_ADDR
from extract_commands import REFERENCE_SHA256
from gen_keyboard_dispatch_vectors import map_sdk
from gen_xplm_sniffer_dispatch_vectors import main, cases as sniffer_cases
from gen_ui_capture_vectors import install_allocator, build_registry
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_XMM0


class Paired:
    def __init__(self, path):
        self.path=path
        self.rng=random.Random(2026101407)
        self.settings=[]
    def cases(self):
        yield from sniffer_cases()
        for action,result in [(1,1),(1,1),(3,0),(1,1)]:
            yield 27,27,8,15,0,1,0x80000000,[(0,1,action,result)],[0,0]
    def emulator(self,sdk):
        assert hashlib.sha256(Path(self.path).read_bytes()).hexdigest()==REFERENCE_SHA256
        emu=Emulator(self.path);map_sdk(emu,sdk,initialize=False);return emu
    def setup(self,e,log,phase):
        self.log=log
        self.owner,self.vector,self.root,self.vt,self.mode=e.alloc(64),e.alloc(48),e.alloc(64),e.alloc(96),e.alloc(4)
        e.write(self.owner,bytes(64));e.write(self.root,bytes(64));e.write(self.vt,bytes(96))
        e.write_u64(self.root,self.vt);e.write_u64(self.vt+0x50,0x1412ee8a0);e.write_u64(self.vt+0x38,RET_ADDR+0xf40)
        e.write_u64(self.owner+8,self.vector);e.write_u64(self.owner+16,self.vector+48);e.write_u64(0x142f01eb0,self.mode)
        self.manager,self.implementation=e.alloc(0x180),e.alloc(0x180);e.write_u64(0x145a0e308,self.manager)
        install_allocator(e)
        e.write_u64(0x146104f88,0x141917f20);e.write_u64(0x146106038,0x18001a670)
        e.write_u64(0x146105010,RET_ADDR+0xf00);e.write_u64(0x146105018,RET_ADDR+0xf20);e.write_u32(0x14610551c,0)
        for address in [RET_ADDR+0xe00,RET_ADDR+0xf00,RET_ADDR+0xf20,RET_ADDR+0xf40]:e.write(address,b'\xc3')
        e.stubs[RET_ADDR+0xf00]=lambda emu:emu.uc.reg_write(UC_X86_REG_RAX,0)
        e.stubs[RET_ADDR+0xf20]=lambda emu:None
        e.stubs[0x1413fa080]=lambda emu:emu.uc.reg_write(UC_X86_REG_XMM0,0)
        def own(emu):
            assert emu.reg(UC_X86_REG_RCX)==self.root
            log.append('UIO');self.mutate(emu);emu.uc.reg_write(UC_X86_REG_RAX,self.settings[2])
        e.stubs[RET_ADDR+0xf40]=own
        def command(kind):
            def run(emu):
                assert emu.reg(UC_X86_REG_R8)==0
                log.append(f'{kind}:{emu.reg(UC_X86_REG_RDX)&0xffffffff}')
            return run
        e.stubs[0x140c7fac0]=command('BEGIN');e.stubs[0x140c7fc40]=command('END')
        e.uc.hook_add(UC_HOOK_CODE,lambda uc,a,s,u:log.append('UIQ'),begin=0x1412e3470,end=0x1412e3470)
        e.uc.hook_add(UC_HOOK_CODE,lambda uc,a,s,u:log.append('UI:'+':'.join(str(e.reg(reg)&0xffffffff) for reg in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8])),begin=0x1412c6300,end=0x1412c6300)
        e.uc.hook_add(UC_HOOK_CODE,lambda uc,a,s,u:log.append('IP:'+':'.join(str(e.reg(reg)&0xffffffff) for reg in [UC_X86_REG_RCX,UC_X86_REG_R8,UC_X86_REG_RDX,UC_X86_REG_R9])),begin=0x141913800,end=0x141913800)
        def export(uc,address,size,user):
            phase[0]=e.reg(UC_X86_REG_R9)&0xffffffff
            log.append(f'XD:{phase[0]}')
        e.uc.hook_add(UC_HOOK_CODE,export,begin=0x18001a670,end=0x18001a670)
        print('# reference_sha256',REFERENCE_SHA256)
        print('# paired full EXE entry/capture/UI filter/interceptor/binding loops + registered mutable DLL sniffer routing; callback and UI-own effects synthetic; EXE timing/context/Begin/End and DLL external services controlled; no live loader/menu/visual claim')
        print('# K capture focus own mask mode block suppression update_mode update_block + P inputs -> zero event return sentinels + P SDK state + suppression active[3] mode block trace*')
    def prepare(self,e,case,index):
        rng=self.rng
        if index<124:
            # Keep most fixed callback cases reachable, with interleaved capture/UI paths.
            capture=int(index%12==0);focus=int(index%6==1);own=0;mask=0xffffffff;mode=block=0;suppression=0;update_mode=update_block=-1
        else:
            capture=rng.randrange(5)==0;focus=rng.randrange(2);own=rng.choice([0,0,1,255]);mask=rng.choice([0,8,0x400,0xffffffff]);mode=rng.choice([0,0,1000,1001]);block=rng.choice([0,0,1,255]);suppression=rng.choice([-1,0,1]);update_mode=rng.choice([-1,-1,0,1000,1001]);update_block=rng.choice([-1,-1,0,255])
        if index>=524:
            capture=focus=own=mode=block=suppression=0;mask=0xffffffff;update_mode=update_block=-1
            if index==524:update_mode=1000
            if index==525:update_block=255
            if index==527:capture=focus=1;own=255;mask=8
        self.settings=[int(capture),focus,own,mask,mode,block,suppression,update_mode,update_block]
        registry=[None,(9,2,[1.])] if capture else [(9,1,[1.]),(10,2,[0.])]
        build_registry(e,self.manager,self.implementation,registry)
        char,key,flags=case[:3]
        entries=[(key,flags&7,3,0),(key,flags&7,29,1),(key,flags&7,30,-1)]
        e.write(self.vector,b''.join(struct.pack('<IIii',*entry) for entry in entries))
        e.write_u32(self.owner+32,suppression);e.write_u32(self.mode,mode);e.write(0x145a0df02,bytes([block]));e.write_u64(0x145a0dfc8,self.root if focus else 0);e.write_u32(self.root+0x18,mask)
    def mutate(self,e):
        if self.settings[7]>=0:e.write_u32(self.mode,self.settings[7])
        if self.settings[8]>=0:e.write(0x145a0df02,bytes([self.settings[8]]))
    def run(self,e,case):
        char,key,flags=case[:3]
        e.call(0x140c627a0,ints=[self.owner,char,key,flags],max_instructions=500000)
    def emit(self,e,case,output,log):
        specs,events=case[7:]
        active=[e.read_u32(self.vector+i*16+12) for i in range(3)]
        tail=[e.read_u32(self.owner+32),*active,e.read_u32(self.mode),e.read(0x145a0df02,1)[0]]
        print('K',*self.settings,*case[:7],len(specs),*(word for spec in specs for word in spec),len(events),*events,'->',*output,*tail,*log)


if __name__=='__main__':main(sys.argv[2],Paired(sys.argv[1]))
