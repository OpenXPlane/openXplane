#!/usr/bin/env python3
"""Replay Windows DLL key hook on explicit lists with context/callback/flush endpoints logged."""
import hashlib
import random
import sys
from pathlib import Path
from emulate_xp import Emulator, RET_ADDR
from xplm_reference_state import install_tls, read_stack
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_RSP
SHA256 = '706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f'
S, H = 0x180069660, 0x180069690


def cases():
    for phase in [0, 1, 2, 3, 0xffffffff]:
        for active in [0, 15]:
            for flags in [0, 8, 12]:
                for target in [0, 1, 2]:
                    yield phase, 27, 27, flags, active, target, 1, 0, 1, 0, 1, [(0, 1, 0, 1), (1, 0, 1, 0)], [(0, 27, flags, 2)]
    rng = random.Random(2026101401)
    for _ in range(1000):
        phase = rng.choice([0, 0, 1, 2, 3, 0xffffffff])
        key = rng.choice([0, 27, 65, 255]); flags = rng.choice([0, 8, 12, 16, 0xffffffff])
        sniffers = [(rng.choice([0,1,255]), rng.choice([0,1,-1]), rng.randrange(4), rng.choice([0,1,-1,255])) for _ in range(rng.randrange(0,7))]
        hotkeys = [(rng.choice([0,0,1]), rng.choice([key,27]), rng.choice([flags,8]), rng.randrange(4)) for _ in range(rng.randrange(0,5))]
        yield phase, rng.choice([0,27,65,255]), key, flags, rng.randrange(16), rng.randrange(3), rng.randrange(2), rng.randrange(3), rng.randrange(2), rng.randrange(3), rng.randrange(2), sniffers, hotkeys


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest() == SHA256
    e = Emulator(path); top = e.heap_top
    log = []; values = []; tls_pointer = [0]
    def validate(stage):
        return lambda emu: log.append(f'V:{stage}')
    def sniff(emu):
        refcon = emu.reg(UC_X86_REG_R9); i = refcon-100
        assert [emu.reg(r)&0xffffffff for r in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8]] == [values[1],values[3],values[2]]
        log.append(f'S:{i}:{refcon}'); emu.uc.reg_write(UC_X86_REG_RAX, values[11][i][3]&0xffffffff)
    def hotkey(emu):
        refcon=emu.reg(UC_X86_REG_RCX);log.append(f'H:{refcon-200}:{refcon}')
    def window(emu):
        assert [emu.reg(r)&0xffffffff for r in [UC_X86_REG_RDX,UC_X86_REG_R8,UC_X86_REG_R9]] == [values[1],values[3],values[2]]
        rsp=emu.reg(UC_X86_REG_RSP);assert emu.read_u64(rsp+40)==300 and emu.read_u32(rsp+48)==0
        log.append('W:1:300')
    def flush(step):
        def run(emu):
            base=emu.reg(UC_X86_REG_RCX);assert base in [S,H]
            depth=emu.read_u32(base+16);pending=e.read(base+20,1)[0]
            if step==1:assert emu.reg(UC_X86_REG_RDX)==base+8
            log.append(f'F:{0 if base==S else 1}:{step}:{depth}:{pending}')
        return run
    # List consistency validation runs twice; infer its step from the pending byte.
    def check_list(emu):
        base=emu.reg(UC_X86_REG_RCX);flush(0 if e.read(base+20,1)[0] else 2)(emu)
    e.stubs.update({0x180021e10:validate(0),0x1800156a0:validate(1),RET_ADDR+0x100:sniff,RET_ADDR+0x200:hotkey,RET_ADDR+0x300:window,0x180008950:check_list,0x1800094f0:flush(1),0x1800091a0:flush(1)})
    def context_enter_trace(uc, address, size, user):
        log.append(f'G:{e.reg(UC_X86_REG_RDX) & 0xffffffff}')
    def context_trace(kind):
        return lambda uc, address, size, user: log.append(f'{kind}:{read_stack(e, tls_pointer[0])[-1]}')
    e.uc.hook_add(UC_HOOK_CODE, context_enter_trace, begin=0x18001cc90, end=0x18001cc90)
    e.uc.hook_add(UC_HOOK_CODE, context_trace('A'), begin=0x18001d1d0, end=0x18001d1d0)
    e.uc.hook_add(UC_HOOK_CODE, context_trace('E'), begin=0x18001cf60, end=0x18001cf60)
    e.stubs[0x1800213f0] = lambda emu: log.append(f'L:{emu.reg(UC_X86_REG_RCX) & 0xffffffff}')
    e.stubs[0x180015470] = lambda emu: log.append('C')
    print('# dll_sha256',SHA256)
    print('# full export/internal key hook and consumed-sniffer EndCall unchanged; context ctor/activation/dtor and enabled query unchanged on ready TLS; validation/lazy-loader/user callbacks/deferred-list helpers logged')
    print('# P phase char key flags active target focus sdepth spending hdepth hpending sn [deleted before plugin result]* hk [deleted key flags plugin]* -> result sdepth spending hdepth hpending trace*')
    count=0
    for case in cases():
        values[:]=case;e.heap_top=top;log.clear()
        phase,char,key,flags,active_bits,target,focus,sd,sp,hd,hp,sniffers,hotkeys=case
        tls_pointer[0], _, _ = install_tls(e, stack=[-1], enabled=[int(bool(active_bits & (1 << i))) for i in range(4)], pending=-1)
        for base,depth,pending in [(S,sd,sp),(H,hd,hp)]:
            e.write(base,bytes(24));e.write_u32(base+16,depth);e.write(base+20,bytes([pending]))
        nodes=[e.alloc(64) for _ in sniffers]
        e.write_u64(S,nodes[0] if nodes else 0)
        for i,(deleted,before,plugin,result) in enumerate(sniffers):
            n=nodes[i];e.write(n,bytes(64));e.write_u64(n,nodes[i+1] if i+1<len(nodes) else 0);e.write(n+8,bytes([deleted]));e.write_u64(n+40,RET_ADDR+0x100);e.write_u32(n+48,plugin);e.write_u32(n+52,before);e.write_u64(n+56,100+i)
        nodes=[e.alloc(104) for _ in hotkeys]
        e.write_u64(H,nodes[0] if nodes else 0)
        for i,(deleted,k,f,plugin) in enumerate(hotkeys):
            n=nodes[i];e.write(n,bytes(104));e.write_u64(n,nodes[i+1] if i+1<len(nodes) else 0);e.write(n+8,bytes([deleted]));e.write_u32(n+40,plugin);e.write(n+44,bytes([k]));e.write_u32(n+48,f);e.write_u64(n+88,RET_ADDR+0x200);e.write_u64(n+96,200+i)
        w=e.alloc(136);e.write(w,bytes(136));e.write_u64(w+80,RET_ADDR+0x300);e.write_u32(w+120,3);e.write_u64(w+128,300);e.write_u64(0x1800696d8,w if focus else 0)
        result=e.call(0x18001a670,ints=[char,flags,key,phase],stack=[0 if target==0 else (w if target==1 else w+8)],max_instructions=100000)&0xffffffff
        assert read_stack(e, tls_pointer[0]) == [-1]
        print('P',*case[:11],len(sniffers),*(v for row in sniffers for v in row),len(hotkeys),*(v for row in hotkeys for v in row),'->',result,e.read_u32(S+16),e.read(S+20,1)[0],e.read_u32(H+16),e.read(H+20,1)[0],*log);count+=1
    print('# cases',count)


if __name__=='__main__':main(sys.argv[1])
