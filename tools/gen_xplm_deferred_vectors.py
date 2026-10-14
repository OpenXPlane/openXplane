#!/usr/bin/env python3
"""Complete Windows SDK deferred deletion helpers; synthetic mapped lists/ready TLS.
Diagnostics, tracking maps, clock and frees are controlled; no original bytes are emitted.
"""
import hashlib
import random
import sys
from pathlib import Path
from emulate_xp import Emulator
from gen_xplm_key_hook_vectors import SHA256
from xplm_reference_state import install_tls, read_stack
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_XMM0


def cases():
    contexts = [[], [0], [7], [7, 0], [0, 7, 0], [-1, 0], [-2147483648, 0]]
    for kind in range(2):
        for stack in contexts:
            for owner in [0, -1, 7, 2147483647]:
                for live in [[0, 1, 2], [2, 0, 1], [1]]:
                    for queue in [[0, 2, 1], [2], []]:
                        yield kind, stack, 0x80000000, [(owner, 15, 7, 99, 8)] * 3, live, queue
    rng = random.Random(2026101404)
    for kind in range(2):
        for _ in range(600):
            n = rng.randrange(10)
            nodes = [(rng.choice([-2147483648, -1, 0, 1, 7, 2147483647]), rng.choice([0, 15, 16, 254, 4094, 4095, 4096, 8191, 0xffffffffffffffff]), rng.randrange(16), rng.randrange(256), rng.choice([8, 9, 31, 39])) for _ in range(n)]
            live = rng.sample(range(n), rng.randrange(n+1))
            queue = rng.sample(range(n), rng.randrange(n+1))
            stack = rng.choices([-2147483648, -1, 0, 0, 1, 7, 2147483647], k=rng.randrange(12))
            yield kind, stack, rng.getrandbits(32), nodes, live, queue


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest() == SHA256
    e = Emulator(path)
    control = e.alloc(24)
    top = e.heap_top
    log, values, nodes, tracks, free_map, freed = [], [], [], {}, {}, set()
    reasons = {0x180050468: 0, 0x1800504b8: 1, 0x1800504f0: 2, 0x180050520: 3}
    def token(address):
        return 0 if address == 0 else nodes.index(address)+1
    def pending(): return token(e.read_u64(control+8))
    def diagnostic(emu):
        log.append(f'D:{reasons[emu.reg(UC_X86_REG_RDX)]}:{token(emu.reg(UC_X86_REG_R9))}:{pending()}')
    def bad_free(emu):
        log.append(f'B:{token(emu.reg(UC_X86_REG_RCX))}:{pending()}')
    def clock(emu):
        log.append(f'C:{pending()}')
        emu.uc.reg_write(UC_X86_REG_XMM0, values[2])
    def tracking(emu):
        assert emu.reg(UC_X86_REG_RCX) == [0x1800680a0, 0x180068130][values[0]]
        t = token(emu.read_u64(emu.reg(UC_X86_REG_R8)))
        record = emu.alloc(0x28)
        emu.write(record, bytes([0xcc])*0x28)
        tracks[t] = record
        out = emu.reg(UC_X86_REG_RDX)
        emu.write_u64(out, record)
        emu.uc.reg_write(UC_X86_REG_RAX, out)
        log.append(f'T:{t}:{pending()}')
    def free(emu):
        t, category, base = free_map[emu.reg(UC_X86_REG_RCX)]
        size = emu.reg(UC_X86_REG_RDX)
        if category == 'N':
            if values[0] == 1:
                p = nodes[t-1]
                assert (emu.read_u64(p+0x48), emu.read_u64(p+0x50), emu.read(p+0x38, 1)[0]) == (0, 15, 0)
            freed.add(t)
            log.append(f'F:N:{t}:{size}:{pending()}')
        else:
            log.append(f'F:S:{t}:{base}:{size}:{pending()}')
    e.stubs.update({0x1800124c0: diagnostic, 0x1800210c0: lambda emu: log.append(f'L:{pending()}'), 0x180008610: bad_free, 0x180008470: bad_free, 0x1800294f0: clock, 0x180007400: tracking, 0x18002f948: free})
    print('# dll_sha256', SHA256)
    print('# whole sniffer/hotkey drain, GetMyID and disabled assertion predicate unchanged; clock/diagnostics/tracking/free services controlled; ready TLS, consistent live links, acyclic queue; no OS lifetime/concurrency claim')
    print('# D kind stack_n stack* time_bits n [owner capacity length first alignment]* live_n tokens* queue_n tokens* -> head deferred [next prev deferred_next length capacity first freed tracked owner flag time_bits]* trace*')
    count = 0
    for case in cases():
        values[:] = case
        kind, stack, time_bits, specs, live, queue = case
        e.heap_top = top
        tls, _, _ = install_tls(e, stack)
        e.write_u64(0x1800696e8, 0)
        nodes[:] = [e.alloc(0x68) for _ in specs]
        free_map.clear(); tracks.clear(); freed.clear(); log.clear()
        inputs = []
        for i, (owner, capacity, length, first, alignment) in enumerate(specs):
            p = nodes[i]; e.write(p, bytes(0x68))
            e.write_u32(p+[0x20, 0x28][kind], owner)
            e.write(p+8, bytes([1 if i in queue else 0]))
            if kind == 1 and capacity > 15:
                allocation = e.alloc(128)
                data = allocation+alignment
                e.write_u64(data-8, allocation)
                e.write_u64(p+0x38, data)
                first = data & 255
                free_map[data] = (i+1, 'S', 0)
                free_map[allocation] = (i+1, 'S', 1)
            elif kind == 1:
                e.write(p+0x38, bytes([first]))
            if kind == 1:
                e.write_u64(p+0x48, length); e.write_u64(p+0x50, capacity)
            else:
                capacity = length = first = 0
            inputs.extend([owner, capacity, length, first, alignment])
            free_map[p] = (i+1, 'N', 0)
        for j, i in enumerate(live):
            e.write_u64(nodes[i], nodes[live[j+1]] if j+1 < len(live) else 0)
            e.write_u64(nodes[i]+0x10, nodes[live[j-1]] if j else 0)
        for j, i in enumerate(queue):
            e.write_u64(nodes[i]+0x18, nodes[queue[j+1]] if j+1 < len(queue) else 0)
        e.write_u64(control, nodes[live[0]] if live else 0)
        e.write_u64(control+8, nodes[queue[0]] if queue else 0)
        e.call([0x1800094f0, 0x1800091a0][kind], ints=[control, control+8], max_instructions=500000)
        assert read_stack(e, tls) == stack
        output = [token(e.read_u64(control)), token(e.read_u64(control+8))]
        for i, p in enumerate(nodes):
            assert e.read_u32(p+[0x20, 0x28][kind]) == specs[i][0] & 0xffffffff
            record = tracks.get(i+1)
            output.extend([token(e.read_u64(p)), token(e.read_u64(p+0x10)), token(e.read_u64(p+0x18)), e.read_u64(p+0x48) if kind else 0, e.read_u64(p+0x50) if kind else 0, e.read(p+0x38, 1)[0] if kind else 0, int(i+1 in freed), int(record is not None), e.read_u32(record+0x18) if record else 0, e.read(record+0x1c, 1)[0] if record else 0, e.read_u32(record+0x20) if record else 0])
        print('D',kind,len(stack),*stack,time_bits,len(nodes),*inputs,len(live),*(i+1 for i in live),len(queue),*(i+1 for i in queue),'->',*output,*log)
        count += 1
    print('# cases', count)


if __name__ == '__main__': main(sys.argv[1])
