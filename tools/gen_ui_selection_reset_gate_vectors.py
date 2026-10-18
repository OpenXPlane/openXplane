#!/usr/bin/env python3
"""Whole reset gate with actual engine/primary/getter; only memcmp and secondary controlled."""
import hashlib
import itertools
import random
import sys
from pathlib import Path
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_RIP
from emulate_xp import Emulator, RET_ADDR
from extract_commands import REFERENCE_SHA256

INDEX = 0x142640504
ENGINE = 0x142f2dc95
TABLE = 0x146124398

def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest() == REFERENCE_SHA256
    e = Emulator(path)
    owner = e.alloc(0x2d0)
    receiver, decoy = e.alloc(0x90), e.alloc(0x90)
    records = e.alloc(0x6668 * 3)
    heap_left, heap_right = e.alloc(64), e.alloc(64)
    ranges = {owner: 0x2d0, receiver: 0x90, decoy: 0x90,
              records + 0x6178: 32, records + 0x6668 + 0x6178: 32,
              heap_left: 64, heap_right: 64, INDEX: 4, ENGINE: 1, TABLE: 8}
    def snapshot():
        return '/'.join(f'{p}:{e.read(p,n).hex()}' for p,n in sorted(ranges.items()))
    current = {}
    totals = dict(cases=0, engine=0, engine_entry=0, primary=0, primary_return=0, getter=0, getter_entry=0, memcmp=0, secondary=0, skips=0, mutations=0)
    def effect():
        mode = current['mutation']
        if mode:
            e.write(owner + 0xc0, bytes([0 if mode == 1 else 0xff]))
            e.write_u64(owner + 0xc8, decoy)
            e.write_u32(INDEX, 1)
            totals['mutations'] += 1
    def memcmp(emu):
        a,b,n = [emu.reg(r) for r in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8]]
        before = snapshot()
        # The CRT body is controlled. Equality is calculated from the declared bytes.
        result = int(emu.read(a,n) != emu.read(b,n)) if n else 0
        effect()
        current['events'].append(f'M~{a}~{b}~{n}~{before}~{snapshot()}~{result}')
        totals['memcmp'] += 1
        emu.uc.reg_write(UC_X86_REG_RAX,result)
    def secondary(emu):
        p = emu.reg(UC_X86_REG_RCX)
        assert p == receiver, 'gate must retain the original receiver across memcmp effects'
        before = snapshot()
        effect()
        result = current['secondary']
        current['events'].append(f'S~{p}~{before}~{snapshot()}~{result}')
        totals['secondary'] += 1
        emu.uc.reg_write(UC_X86_REG_RAX,result)
    e.stubs[0x1423c0180] = memcmp
    e.stubs[0x14113ff10] = secondary
    for address,name in [(0x1417fb320,'engine_entry'),(0x1417fb327,'engine'),
                         (0x14113fff0,'primary'),(0x141140047,'primary_return'),
                         (0x14114004f,'primary_return'),(0x141a64260,'getter_entry'),
                         (0x141a6426d,'getter')]:
        def count(uc,a,size,user,key=name):
            totals[key] += 1
        e.uc.hook_add(UC_HOOK_CODE,count,begin=address,end=address)
    def string(p, value, heap):
        raw = bytearray(32)
        if heap:
            e.write(heap,value + bytes(64-len(value)))
            raw[:8] = heap.to_bytes(8,'little')
        else:
            raw[:len(value)] = value
        raw[16:24] = len(value).to_bytes(8,'little')
        raw[24:32] = (63 if heap else 15).to_bytes(8,'little')
        e.write(p,raw)
    def run(case, present, flag, engine, profile, second, mutation):
        current.update(events=[],secondary=second,mutation=mutation)
        poison = (case * 37) & 255
        for p,n in ranges.items():
            e.write(p,bytes([poison])*n)
        e.write_u64(owner+0xc8,receiver if present else 0)
        e.write(owner+0xc0,bytes([flag]))
        index = [0,1,0xffffffff][case % 3]
        e.write(ENGINE,bytes([engine]));e.write_u32(INDEX,index)
        e.write_u64(TABLE,records + (0x6668 if index==0xffffffff else 0))
        record = records + (0x6668 if index==1 else 0)
        other = records + (0 if index==1 else 0x6668)
        values = [(b'ab\0cd',b'ab\0cd'),(b'ab\0cd',b'ab\0ce'),(b'ab',b'abc'),
                  (b'0123456789abcdefghij',b'0123456789abcdefghij'),(b'',b''),
                  (b'mixed',b'mixed'),(b'mixed',b'mixed'),(b'',b'')]
        left,right = values[profile]
        string(receiver+0x20,left,heap_left if profile in (3,6,7) else 0)
        string(record+0x6178,right,heap_right if profile in (3,5,7) else 0)
        string(other+0x6178,b'decoy',0)
        if profile==7:
            e.write_u64(receiver+0x20,0);e.write_u64(record+0x6178,0)
        before = snapshot()
        e.uc.reg_write(UC_X86_REG_RAX,0xffffffffffffffff)
        result = e.call(0x14114d330,ints=[owner]) & 255
        assert e.reg(UC_X86_REG_RIP)==RET_ADDR and result in (0,1)
        totals['cases']+=1;totals['skips']+=result
        print('C',owner,mutation,decoy,before,'->',result,snapshot(),'|'.join(current['events']) or '-')
    print('# reference_sha256',REFERENCE_SHA256)
    print('# actual14114d330/14113fff0/1417fb320/141a64260; controlled CRT memcmp and secondary14113ff10; AL result only')
    cases = itertools.product([0,1],[0,1,255],[0,1,2,255],range(4),[0,1,256,257],range(3))
    count = 0
    for count,values in enumerate(cases,1):
        run(count,*values)
    for engine in range(256):
        count+=1;run(count,1,1,engine,4,256,2 if engine&1 else 0)
    rng=random.Random(14114)
    for _ in range(128):
        count+=1;run(count,rng.randrange(2),rng.randrange(256),rng.randrange(256),rng.randrange(8),rng.getrandbits(64),rng.randrange(3))
    assert count==1536
    assert totals['engine_entry']==totals['engine']
    assert totals['primary']==totals['primary_return']==totals['getter']==totals['getter_entry']
    print('# gate_totals',totals)
    for poison in [0,0xa5,0xff]:
        for tag in range(256):
            image=bytearray([poison])*0x90;image[0x68]=tag;e.write(receiver,image)
            e.uc.reg_write(UC_X86_REG_RAX,0xffffffffffffffff)
            result=e.call(0x1417fb390,ints=[receiver])
            assert e.reg(UC_X86_REG_RIP)==RET_ADDR and e.read(receiver,len(image))==image
            print('T',receiver,poison,tag,'->',result)
    print('# tag_cases',768)

if __name__=='__main__':
    main(sys.argv[1])
