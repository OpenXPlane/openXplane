#!/usr/bin/env python3
"""Execute original registry extraction/insertion on synthetic nonnull valid lists.
Only allocation/free endpoints controlled. Corpus contains numeric synthetic state.
"""
import hashlib
import random
import sys
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_RAX


def cases():
    for flag in [0,1,255,256,257,0xffffffff]:
        for order in [[0],[0,1,2],[2,1,0],[0,1,0,2],[1,0,2,1]]:
            for token in [0,1]:
                yield flag,(token if token in [([0,1,0])[i] for i in order] else 0),[0,0,-1],[0,1,0],order
    rng=random.Random(2026101420)
    for _ in range(640):
        n=rng.randrange(1,9)
        classes=[rng.choice([-2147483648,-2,0,1,2,2147483647]) for _ in range(n)]
        tokens=[rng.randrange(4) for _ in range(n)]
        order=[rng.randrange(n) for _ in range(rng.randrange(1,12))]
        yield rng.choice([0,1,2,255,256,257,0xffffffff]),tokens[rng.choice(order)],classes,tokens,order


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==REFERENCE_SHA256
    e=Emulator(path);impl=e.alloc(0x180);top=e.heap_top
    log=[]
    def free(emu):
        assert emu.reg(UC_X86_REG_RDX)==24
        log.append('F')
    def alloc(emu):
        assert emu.reg(UC_X86_REG_RCX)==24
        log.append('A');emu.uc.reg_write(UC_X86_REG_RAX,emu.alloc(24))
    e.stubs.update({0x1422e7c2c:free,0x1422e7bf0:alloc})
    print('# reference_sha256',REFERENCE_SHA256)
    print('# original extract/insert unchanged; allocator/free controlled; valid nonnull registry, registered target, finite size')
    count=0
    for flag,token,classes,tokens,order in cases():
        e.heap_top=top;log.clear();e.write(impl,bytes(0x180))
        entries=[]
        for cl,t in zip(classes,tokens):
            p=e.alloc(0x178);e.write(p,bytes(0x178));e.write_u32(p+0x170,cl);e.write_u64(p+0x138,t);entries.append(p)
        head=e.alloc(24);links=[e.alloc(24) for _ in order];all_links=[head]+links
        for i,p in enumerate(all_links):
            e.write_u64(p,all_links[(i+1)%len(all_links)]);e.write_u64(p+8,all_links[(i-1)%len(all_links)])
            e.write_u64(p+16,0 if i==0 else entries[order[i-1]])
        e.write_u64(impl+0x150,head);e.write_u64(impl+0x158,len(order))
        entry=e.call(0x1412eecc0,ints=[impl,token],max_instructions=100000)
        picked=entries.index(entry);after_extract=e.read_u64(impl+0x158)
        e.call(0x1412e4220,ints=[impl,entry,flag],max_instructions=100000)
        out=[];p=e.read_u64(head)
        while p!=head:
            assert len(out)<len(order)+1
            assert e.read_u64(e.read_u64(p)+8)==p
            out.append(entries.index(e.read_u64(p+16)));p=e.read_u64(p)
        assert e.read_u64(e.read_u64(head+8))==head
        print('C',flag,token,','.join(map(str,classes)),','.join(map(str,tokens)),','.join(map(str,order)),'->',picked,after_extract,e.read_u64(impl+0x158),','.join(map(str,out)),''.join(log));count+=1
    print('# cases',count)


if __name__=='__main__':main(sys.argv[1])
