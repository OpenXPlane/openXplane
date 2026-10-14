#!/usr/bin/env python3
"""Actual menu panel own key slot and focused/filter path; cancel action only logged."""
import hashlib
import itertools
import random
import sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RCX


def cases():
    for accept,confirm,key,flags,mask in itertools.product([0,1,255],[0,1,255],[0,13,27,0xbb,0xbc,65,255,0xffffffff],[0,8,12,16,0xffffffff],[0,8,2,0xfff]):
        yield accept,confirm,27,key,flags,mask,0,0
    rng=random.Random(2026101408)
    for _ in range(500):
        yield rng.randrange(256),rng.randrange(256),rng.choice([-1,0,13,27,65,128,255]),rng.choice([13,27,0xbb,0xbc,65,255]),rng.getrandbits(32),rng.getrandbits(32),rng.randrange(256),rng.randrange(256)


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
    e=Emulator(path);panel=e.alloc(0x300);e.write(panel,bytes(0x300));e.write_u64(panel,0x1426064f0);e.write_u64(0x145a0dfc8,panel)
    values,log=[],[]
    def cancel(emu):
        assert emu.reg(UC_X86_REG_RCX)==panel
        log.append('C');emu.write(panel+0x168,bytes(values[6:8]))
    e.stubs[0x140f00310]=cancel
    # Observe real getter/validation/no-op bodies; do not replace them.
    for address,label in [(0x1412e33e0,'A'),(0x1412e33d0,'F'),(0x140643a20,'V'),(0x1405df230,'N'),(0x1412e1de0,'K')]:
        e.uc.hook_add(UC_HOOK_CODE,lambda uc,a,s,u,label=label:log.append(label),begin=address,end=address)
    print('# reference_sha256',REFERENCE_SHA256)
    print('# actual panel vtable, complete own callback/getters/constant-validation/no-ops and focused wrapper/base/filter/CRT unchanged; cancel 0x140f00310 logged with explicit flag mutation; no child graph/rendering claim')
    print('# M accept confirm char key flags mask update_accept update_confirm -> own_return accept confirm trace* | focused_return accept confirm trace*')
    count=0
    for case in cases():
        values[:]=case;accept,confirm,char,key,flags,mask,_,_=case
        outputs=[]
        for entry,args in [(0x1412e1de0,[panel,char,key,flags]),(0x1412c6300,[char,key,flags])]:
            e.write(panel+0x168,bytes([accept,confirm]));e.write_u32(panel+0x18,mask);log.clear()
            result=e.call(entry,ints=args,max_instructions=50000)&255
            outputs.append(' '.join(map(str,[result,*e.read(panel+0x168,2),*log])))
        print('M',*case,'->',outputs[0],'|',outputs[1]);count+=1
    print('# cases',count)


if __name__=='__main__':main(sys.argv[1])
