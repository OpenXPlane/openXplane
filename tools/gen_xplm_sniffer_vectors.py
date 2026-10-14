#!/usr/bin/env python3
"""Actual sniffer register/unregister/end-iteration, remove, drain and ready-TLS IDs.
Allocator, clock, tracking map, lazy loader and validation/diagnostics are controlled.
"""
import hashlib
import random
import sys
from pathlib import Path
from emulate_xp import Emulator
from gen_xplm_key_hook_vectors import SHA256
from xplm_reference_state import install_tls, read_stack
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_XMM0
S=0x180069660


def cases():
    for stack in [[],[0],[7],[7,0],[-1,0],[-2147483648,0]]:
        for depth in [-2,-1,0,1,2,2147483647,-2147483648]:
            for owner in [-1,0,7]:
                yield stack,depth,0,0x80000000,[(owner,0,42,owner,1,99)],[],[(0,42,1,99),(0,42,1,99),(1,42,1,99),(1,42,1,99),(2,0,0,0)]
    rng=random.Random(2026101405)
    for _ in range(1000):
        stack=rng.choices([-2147483648,-1,0,0,7,2147483647],k=rng.randrange(12))
        specs=[(rng.choice([-1,0,7]),rng.choice([0,1,255]),rng.choice([0,42,43,0xffffffffffffffff]),rng.choice([-1,0,7]),rng.choice([-1,0,1,2]),rng.randrange(3)) for _ in range(rng.randrange(7))]
        queue=rng.sample([i for i,s in enumerate(specs) if s[1]!=0],rng.randrange(1+sum(s[1]!=0 for s in specs)))
        if specs and rng.randrange(2):
            chosen=rng.choice(specs);binding=chosen[2],chosen[4],chosen[5]
        else: binding=rng.choice([0,42,43,0xffffffffffffffff]),rng.choice([-2147483648,-1,0,1,2,2147483647]),rng.randrange(3)
        sequences=[[0],[1],[0,0,1,1,2],[0,0,2],[1,1,2]]
        operations=[(op,*binding) for op in rng.choice(sequences)]
        yield stack,rng.choice([-2,-1,0,1,2,2147483647,-2147483648]),rng.choice([0,1,255]),rng.getrandbits(32),specs,queue,operations


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==SHA256
    e=Emulator(path);top=e.heap_top
    nodes,tracks,freed,log,values=[],{},set(),[],[]
    def token(p):return 0 if p==0 else nodes.index(p)+1
    def pending():return token(e.read_u64(S+8))
    def allocate(emu):
        assert emu.reg(UC_X86_REG_RCX)==0x40
        p=emu.alloc(0x40);emu.write(p,bytes([0xcc])*0x40);nodes.append(p)
        log.append(f'A:{len(nodes)}:64:{pending()}');emu.uc.reg_write(UC_X86_REG_RAX,p)
    def clock(emu):
        log.append(f'C:{pending()}');emu.uc.reg_write(UC_X86_REG_XMM0,values[3])
    def track(emu):
        assert emu.reg(UC_X86_REG_RCX)==0x1800680a0
        t=token(emu.read_u64(emu.reg(UC_X86_REG_R8)))
        if t not in tracks:
            p=emu.alloc(0x28);emu.write(p,bytes([0xcc])*0x28);tracks[t]=p
        out=emu.reg(UC_X86_REG_RDX);emu.write_u64(out,tracks[t]);emu.uc.reg_write(UC_X86_REG_RAX,out)
        log.append(f'T:{t}:{pending()}')
    def free(emu):
        t=token(emu.reg(UC_X86_REG_RCX));size=emu.reg(UC_X86_REG_RDX);assert size==0x40
        freed.add(t);log.append(f'F:N:{t}:{size}:{pending()}')
    reasons={0x180050468:0,0x1800504b8:1,0x1800504f0:2,0x180050520:3}
    e.stubs.update({0x18002f904:allocate,0x1800294f0:clock,0x180007400:track,0x18002f948:free,0x1800156a0:lambda emu:log.append('V'),0x1800213f0:lambda emu:log.append(f'Z:{emu.reg(UC_X86_REG_RCX)&0xffffffff}'),0x1800124c0:lambda emu:log.append(f'D:{reasons[emu.reg(UC_X86_REG_RDX)]}:{token(emu.reg(UC_X86_REG_R9))}:{pending()}'),0x1800210c0:lambda emu:log.append(f'L:{pending()}'),0x180008610:lambda emu:log.append(f'B:{token(emu.reg(UC_X86_REG_RCX))}:{pending()}')})
    print('# dll_sha256',SHA256)
    print('# complete RegisterKeySniffer/UnregisterKeySniffer/remove/end/drain/ready-TLS owner+callback-plugin helpers unchanged; validation/lazy-loader/allocator/clock/tracking/diagnostics controlled; consistent live links, acyclic queue')
    print('# S stack_n stack* depth pending time_bits n [owner deleted callback plugin before refcon]* queue_n tokens* op_n [op callback before refcon]* -> results* head deferred depth pending n [next prev deferred_next owner deleted callback plugin before refcon freed tracked tracking_plugin flag time_bits]* trace*')
    count=0
    for case in cases():
        values[:]=case;stack,depth,pending_byte,time_bits,specs,queue,operations=case
        e.heap_top=top;tls,_,_=install_tls(e,stack);e.write_u64(0x1800696e8,0)
        nodes[:]=[e.alloc(0x40) for _ in specs];tracks.clear();freed.clear();log.clear()
        for i,(owner,deleted,callback,plugin,before,refcon) in enumerate(specs):
            p=nodes[i];e.write(p,bytes(0x40));e.write_u64(p,nodes[i+1] if i+1<len(nodes) else 0);e.write(p+8,bytes([deleted]));e.write_u64(p+0x10,nodes[i-1] if i else 0)
            e.write_u32(p+0x20,owner);e.write_u64(p+0x28,callback);e.write_u32(p+0x30,plugin);e.write_u32(p+0x34,before);e.write_u64(p+0x38,refcon)
        for j,i in enumerate(queue):e.write_u64(nodes[i]+0x18,nodes[queue[j+1]] if j+1<len(queue) else 0)
        e.write_u64(S,nodes[0] if nodes else 0);e.write_u64(S+8,nodes[queue[0]] if queue else 0);e.write_u32(S+16,depth);e.write(S+20,bytes([pending_byte]))
        results=[]
        for op,callback,before,refcon in operations:
            if op==2:e.call(0x180008010,ints=[S],max_instructions=500000);results.append(0)
            else:results.append(e.call([0x180011230,0x1800121d0][op],ints=[callback,before,refcon],max_instructions=500000)&0xffffffff)
        assert read_stack(e,tls)==stack
        output=[*results,token(e.read_u64(S)),token(e.read_u64(S+8)),e.read_u32(S+16),e.read(S+20,1)[0],len(nodes)]
        for i,p in enumerate(nodes):
            record=tracks.get(i+1)
            output.extend([token(e.read_u64(p)),token(e.read_u64(p+0x10)),token(e.read_u64(p+0x18)),e.read_u32(p+0x20),e.read(p+8,1)[0],e.read_u64(p+0x28),e.read_u32(p+0x30),e.read_u32(p+0x34),e.read_u64(p+0x38),int(i+1 in freed),int(record is not None),e.read_u32(record+0x18) if record else 0,e.read(record+0x1c,1)[0] if record else 0,e.read_u32(record+0x20) if record else 0])
        print('S',len(stack),*stack,depth,pending_byte,time_bits,len(specs),*(word for spec in specs for word in spec),len(queue),*(i+1 for i in queue),len(operations),*(word for operation in operations for word in operation),'->',*output,*log);count+=1
    print('# cases',count)


if __name__=='__main__':main(sys.argv[1])
