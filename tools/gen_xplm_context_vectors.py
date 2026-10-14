#!/usr/bin/env python3
"""Full initialized-TLS context ctor/activation/dtor and enabled query, no context-result stubs."""
import hashlib
import itertools
import random
import sys
from pathlib import Path
from emulate_xp import Emulator
from gen_xplm_key_hook_vectors import SHA256
from xplm_reference_state import install_tls, read_stack
from unicorn.x86_const import UC_X86_REG_RCX


def cases():
    for plugin in [-2147483648,-2,-1,0,1,2,3,4,2147483647]:
        for enabled in [[],[0,0,0,0],[1,2,255,1]]:
            for pending in [-1,0,1,7]:
                yield plugin, pending, [0,2], enabled, 0, 0
    rng=random.Random(2026101402)
    for _ in range(1000):
        yield rng.choice([-2,-1,0,1,2,3,4,2147483647]),rng.choice([-1,0,1,2,7]),rng.choices([-1,0,1,2,7],k=rng.randrange(0,12)),[rng.choice([0,1,2,255]) for _ in range(rng.randrange(0,8))],rng.randrange(3),rng.randrange(2)


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==SHA256
    e=Emulator(path);guard=e.alloc(16);persistent=e.heap_top;log=[];values=[];pointers=[]
    def load(emu):
        plugin=emu.reg(UC_X86_REG_RCX)&0xffffffff
        if plugin>=0x80000000:plugin-=0x100000000
        log.append(f'L:{plugin}')
        if values[4]==1 and 0<=plugin<len(values[3]):emu.write(pointers[2]+plugin*0x110+0x88,b'\xff')
        if values[4]==2:
            tls=pointers[0];end=emu.read_u64(tls+0x18);emu.write_u32(end-4,0)
    def unload(emu):
        plugin=emu.reg(UC_X86_REG_RCX)&0xffffffff
        if plugin>=0x80000000:plugin-=0x100000000
        log.append(f'U:{plugin}')
        if values[5]:
            tls=pointers[0];end=emu.read_u64(tls+0x18);emu.write_u32(end,9);emu.write_u64(tls+0x18,end+4)
    e.stubs[0x1800213f0]=load
    e.stubs[0x180015470]=lambda emu: log.append('V')
    e.stubs[0x18002e470]=unload
    e.stubs[0x1800210c0]=lambda emu: log.append('N')
    print('# dll_sha256',SHA256)
    print('# native context ctor/activation/dtor, enabled export and pending-ID search unchanged; initialized TLS/reserved vector; loader/validation/unload/notification logged')
    print('# T plugin pending depth stack* enabled_count bytes* load_mutation unload_mutation -> active depth stack* enabled_count bytes* pending trace*')
    count=0
    for case in cases():
        values[:]=case;plugin,pending,stack,enabled,load_mutation,unload_mutation=case;e.heap_top=persistent
        pointers[:]=install_tls(e,stack,enabled,pending);log.clear()
        e.call(0x18001cc90,ints=[guard,plugin],max_instructions=50000)
        active=e.call(0x18001d1d0,ints=[guard],max_instructions=50000)&255
        e.call(0x18001cf60,ints=[guard],max_instructions=50000)
        final=read_stack(e,pointers[0]);final_enabled=[e.read(pointers[2]+i*0x110+0x88,1)[0] for i in range(len(enabled))]
        print('T',plugin,pending,len(stack),*stack,len(enabled),*enabled,load_mutation,unload_mutation,'->',active,len(final),*final,len(final_enabled),*final_enabled,e.read_u32(0x180068290),*log);count+=1
    print('# cases',count)
    # Complete GetMyID export/body runs without result or helper stubs on ready TLS.
    stacks = [list(v) for n in range(7) for v in itertools.product([0, -1, 7], repeat=n)]
    stacks += [[0] * n for n in range(64)]
    rng = random.Random(2026101403)
    words = [-2147483648, -7, -1, 0, 0, 0, 1, 2, 7, 2147483647]
    stacks += [rng.choices(words, k=rng.randrange(64)) for _ in range(500)]
    print('# O depth stack* -> signed_my_id depth stack*; initialized TLS, no callee stubs reached')
    for stack in stacks:
        e.heap_top=persistent
        tls, _, _ = install_tls(e, stack)
        log.clear()
        result=e.call(0x180022e50,max_instructions=50000)&0xffffffff
        if result >= 0x80000000: result -= 0x100000000
        final=read_stack(e,tls)
        assert final == stack and not log
        print('O',len(stack),*stack,'->',result,len(final),*final)
    print('# owner_cases',len(stacks))


if __name__=='__main__':main(sys.argv[1])
