#!/usr/bin/env python3
"""Actual registered SDK sniffer callbacks invoke native register/unregister from synthetic thunks.
Full DLL routing/context/lifecycle code runs unchanged; no nested emulator.call in callbacks.
"""
import hashlib
import random
import struct
import sys
from pathlib import Path
from emulate_xp import Emulator, RET_ADDR
from gen_xplm_key_hook_vectors import SHA256
from xplm_reference_state import install_tls, read_stack
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RBX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_XMM0
S,H=0x180069660,0x180069690


def cases():
    for phase in [0,2]:
        for action in range(5):
            for result in [0,1]:
                for plugin in [0,1,2]:
                    for enabled in [0,15]:
                        before=1 if phase==0 else 0
                        yield 27,27,8,enabled,0,before,0x80000000,[(0,before,0,1),(plugin,before,action,result),(0,before,0,1)],[phase,phase]
    for phase in [0,2]:
        for result in [0,1]:
            before=1 if phase==0 else 0
            yield 27,27,8,15,0,before,0x80000000,[(0,before,0,1),(1,before,1,result),(0,before,1,1)],[phase,phase]
    rng=random.Random(2026101406)
    for _ in range(400):
        specs=[(rng.choice([-1,0,1,2]),rng.choice([-1,0,1,2]),rng.randrange(5),rng.choice([-1,0,0,1,255])) for _ in range(rng.randrange(7))]
        yield rng.choice([0,27,255]),rng.choice([0,27,255]),rng.choice([0,8,0xffffffff]),rng.randrange(16),rng.randrange(3),rng.randrange(2),rng.getrandbits(32),specs,rng.choice([[0,0],[2,2],[0,2],[2,0]])


def callback_address(cb):return RET_ADDR+(0x800 if cb==999 else 0x100+cb*0x80)

def main(path, paired=None):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest()==SHA256
    e=Emulator(path) if paired is None else paired.emulator(path)
    guard=e.alloc(16)
    nodes,tracks,freed,log,values,tls_pointer,phase=[],{},set(),[],[],[0],[0]
    callback_returns={}
    def check_callback_return(uc,address,size,user):
        if address in callback_returns:
            assert uc.reg_read(UC_X86_REG_RAX)&0xffffffff == callback_returns[address], "synthetic callback return mismatch"
    e.uc.hook_add(UC_HOOK_CODE,check_callback_return,begin=RET_ADDR+0x100,end=RET_ADDR+0x900)
    def token(p):return 0 if p==0 else nodes.index(p)+1
    def pending():return token(e.read_u64(S+8))
    def context():return read_stack(e,tls_pointer[0])[-1]
    def allocate(emu):
        assert emu.reg(UC_X86_REG_RCX)==64
        p=emu.alloc(64);emu.write(p,bytes([0xcc])*64);nodes.append(p)
        log.append(f'A:{len(nodes)}:64:{pending()}:{context()}');emu.uc.reg_write(UC_X86_REG_RAX,p)
    def clock(emu):
        log.append(f'C:{pending()}:{context()}');emu.uc.reg_write(UC_X86_REG_XMM0,values[6])
    def track(emu):
        assert emu.reg(UC_X86_REG_RCX)==0x1800680a0
        t=token(emu.read_u64(emu.reg(UC_X86_REG_R8)))
        if t not in tracks:
            p=emu.alloc(0x28);emu.write(p,bytes([0xcc])*0x28);tracks[t]=p
        out=emu.reg(UC_X86_REG_RDX);emu.write_u64(out,tracks[t]);emu.uc.reg_write(UC_X86_REG_RAX,out)
        log.append(f'T:{t}:{pending()}:{context()}')
    def free(emu):
        t=token(emu.reg(UC_X86_REG_RCX));assert emu.reg(UC_X86_REG_RDX)==64
        freed.add(t);log.append(f'F:N:{t}:64:{pending()}:{context()}')
    def callback_log(emu):
        assert [emu.reg(reg)&0xffffffff for reg in [UC_X86_REG_RCX,UC_X86_REG_RDX,UC_X86_REG_R8]]==[values[0] if paired is None or values[0]<=127 else 0,values[2],values[1]]
        t=token(emu.reg(UC_X86_REG_RBX));cb=emu.reg(UC_X86_REG_R9)-100
        log.append(f'K:{t}:{cb}:{phase[0]}:{context()}')
        if paired is not None: paired.mutate(e)
    reasons={0x180050468:0,0x1800504b8:1,0x1800504f0:2,0x180050520:3}
    e.stubs.update({0x18002f904:allocate,0x1800294f0:clock,0x180007400:track,0x18002f948:free,0x1800156a0:lambda emu:log.append('V'),0x180021e10:lambda emu:log.append('Q'),0x180015470:lambda emu:log.append('B'),0x1800213f0:lambda emu:log.append(f'Z:{emu.reg(UC_X86_REG_RCX)&0xffffffff}'),0x1800124c0:lambda emu:log.append(f'D:{reasons[emu.reg(UC_X86_REG_RDX)]}:{token(emu.reg(UC_X86_REG_R9))}:{pending()}:{context()}'),0x1800210c0:lambda emu:log.append(f'L:{pending()}:{context()}'),0x180008610:lambda emu:log.append(f'X:{token(emu.reg(UC_X86_REG_RCX))}:{pending()}:{context()}'),RET_ADDR+0xe00:callback_log})
    def call_bytes(address,args):
        out=b''
        for opcode,value in zip([b'\x48\xb9',b'\x48\xba',b'\x49\xb8'],args):out+=opcode+struct.pack('<Q',value&0xffffffffffffffff)
        return out+b'\x48\xb8'+struct.pack('<Q',address)+b'\xff\xd0'
    if paired is not None: paired.setup(e,log,phase)
    top=e.heap_top
    print('# dll_sha256',SHA256)
    print('# complete export/key hook, actual registration/remove/drain and ready-TLS context/enabled helpers unchanged; synthetic callback thunks call real APIs; external validation/loading/clock/tracking/allocator controlled; no hotkeys/windows, no recursion/OS lifetime claim')
    print('# P char key flags enabled_bits initial_depth new_before time_bits n [plugin before action result]* event_n phases* -> results* head deferred depth pending n [next prev deferred_next owner deleted cbid plugin before refcon freed tracked tracking_plugin flag time_bits]* trace*')
    count=0
    for index,case in enumerate(cases() if paired is None else paired.cases()):
        values[:]=case;char,key,flags,enabled,depth,new_before,time_bits,specs,events=case
        e.heap_top=top;tls_pointer[0],_,_=install_tls(e,[0],[int(bool(enabled&(1<<i))) for i in range(4)])
        e.write_u64(0x1800696e8,0);e.write(S,bytes(24));e.write(H,bytes(24));e.write_u64(0x1800696d8,0)
        nodes.clear();tracks.clear();freed.clear();log.clear()
        if paired is not None: paired.prepare(e,case,index)
        callback_returns.clear()
        for i,(_,before,action,result) in enumerate(specs,1):
            calls=[]
            if action in [1,4]:calls.append((0x1800121d0,[callback_address(i),before,100+i]))
            if action==2 and specs:
                target=i-1 if i>1 else len(specs);target_before=specs[target-1][1]
                calls.append((0x1800121d0,[callback_address(target),target_before,100+target]))
            if action==3:calls.append((0x180011230,[callback_address(999),new_before,1099]))
            if action==4:calls.append((0x180011230,[callback_address(i),before,100+i]))
            thunk=b'\x48\x83\xec\x28'+b'\x48\xb8'+struct.pack('<Q',RET_ADDR+0xe00)+b'\xff\xd0'
            for address,args in calls:thunk+=call_bytes(address,args)
            thunk+=b'\xb8'+struct.pack('<I',result&0xffffffff)+b'\x48\x83\xc4\x28\xc3'
            assert len(thunk)<=0x80;e.write(callback_address(i),thunk)
            callback_returns[callback_address(i)+len(thunk)-1]=result&0xffffffff
        e.write(callback_address(999),b'\x48\x83\xec\x28\x48\xb8'+struct.pack('<Q',RET_ADDR+0xe00)+b'\xff\xd0\xb8\x01\x00\x00\x00\x48\x83\xc4\x28\xc3')
        # Unicorn caches translated code: invalidate only rewritten synthetic callback bytes.
        e.uc.ctl_remove_cache(RET_ADDR+0x100,RET_ADDR+0x900)
        for i,(plugin,before,_,_) in enumerate(specs,1):
            e.call(0x18001cc90,ints=[guard,plugin],max_instructions=50000)
            assert e.call(0x180011230,ints=[callback_address(i),before,100+i],max_instructions=500000)&0xffffffff==1
            e.call(0x18001cf60,ints=[guard],max_instructions=50000)
        e.write_u32(S+16,depth)
        results=[]
        for value in events:
            phase[0]=value
            if paired is None:
                results.append(e.call(0x18001a670,ints=[char,flags,key,value],stack=[0],max_instructions=500000)&0xffffffff)
            else:
                paired.run(e,case)
                results.append(0)
        assert read_stack(e,tls_pointer[0])==[0]
        assert (e.read_u32(H+16),e.read(H+20,1)[0])==(0,0)
        output=[*results,token(e.read_u64(S)),token(e.read_u64(S+8)),e.read_u32(S+16),e.read(S+20,1)[0],len(nodes)]
        addresses={callback_address(i):i for i in range(1,len(specs)+1)};addresses[callback_address(999)]=999
        for i,p in enumerate(nodes):
            record=tracks.get(i+1)
            output.extend([token(e.read_u64(p)),token(e.read_u64(p+0x10)),token(e.read_u64(p+0x18)),e.read_u32(p+0x20),e.read(p+8,1)[0],addresses[e.read_u64(p+0x28)],e.read_u32(p+0x30),e.read_u32(p+0x34),e.read_u64(p+0x38),int(i+1 in freed),int(record is not None),e.read_u32(record+0x18) if record else 0,e.read(record+0x1c,1)[0] if record else 0,e.read_u32(record+0x20) if record else 0])
        if paired is None:
            print('P',*case[:7],len(specs),*(word for spec in specs for word in spec),len(events),*events,'->',*output,*log)
        else: paired.emit(e,case,output,log)
        count+=1
    print('# cases',count)


if __name__=='__main__':main(sys.argv[1])
