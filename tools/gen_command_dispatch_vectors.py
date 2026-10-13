#!/usr/bin/env python3
"""Full native Begin/Continue/End, gate, override lookup, observer snapshot/traversal; callback boundaries logged."""
import itertools
import random
import struct
import sys
from emulate_xp import Emulator, RET_ADDR
from extract_commands import detailed_records, REFERENCE_SHA256
from gen_ui_capture_vectors import install_allocator
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_XMM2, UC_X86_REG_XMM3


def bits(value):
    return struct.unpack('<Q', struct.pack('<d', value))[0]


def cases():
    for phase, command, count, block, mask, override, primary, before in itertools.product(
            [0, 1, 2], [3, 29, 30, 70], [0, 1, -1], [0, 255], [0, 0xffffffff], [0, 1], [0, 5], [0, 1]):
        yield phase, command, count, block, mask, 0, override, primary, before, 1.25, 0, 0, 0.125
    rng = random.Random(2026101307)
    for _ in range(1000):
        yield rng.randrange(3), rng.choice([0, 3, 29, 30, 70, 3137]), rng.choice([-2147483648, -1, 0, 1, 2, 2147483647]), rng.choice([0, 0, 255]), rng.getrandbits(32), rng.getrandbits(32), rng.randrange(2), rng.choice([0, 5]), rng.choice([-1, 0, 1]), rng.uniform(-1e10, 1e10), rng.randrange(4), rng.randrange(2), rng.choice([0., 0.125, -0.5, 1.])


def tree(e, values, key_format, value_format=None):
    head = e.alloc(48)
    e.write(head, bytes(48)); e.write(head + 25, b'\1')
    def build(rows, parent):
        if not rows: return head
        i = len(rows)//2
        key, value = rows[i]
        node = e.alloc(48)
        e.write(node, bytes(48))
        e.write_u64(node + 8, parent)
        e.write_u64(node, build(rows[:i], node))
        e.write_u64(node + 16, build(rows[i+1:], node))
        offset = 28 if key_format == 'i' else 32
        e.write(node + offset, struct.pack('<'+key_format, key))
        if value_format: e.write(node + (32 if key_format == 'i' else 40), struct.pack('<'+value_format, value))
        return node
    root = build(values, head)
    e.write_u64(head + 8, root)
    if root != head:
        left, right = root, root
        while e.read_u64(left) != head: left = e.read_u64(left)
        while e.read_u64(right + 16) != head: right = e.read_u64(right + 16)
        e.write_u64(head, left); e.write_u64(head + 16, right)
    else: e.write_u64(head, head); e.write_u64(head + 16, head)
    return head


def main(path):
    detailed_records(path)
    e = Emulator(path)
    owner, counts, times, primary = e.alloc(0x100), e.alloc(3138*4), e.alloc(3138*4), e.alloc(3138*8)
    e.write_u64(owner + 0x80, counts); e.write_u64(owner + 0x98, times); e.write_u64(owner + 0x68, primary)
    vt = e.alloc(24)
    e.write_u64(vt, RET_ADDR+0x100); e.write_u64(vt+16, RET_ADDR+0x200); e.write_u64(vt+8, RET_ADDR+0x300)
    objects = [0] + [e.alloc(16) for _ in range(5)]
    for obj in objects[1:]: e.write_u64(obj, vt)
    ids = {obj: i for i, obj in enumerate(objects)}
    install_allocator(e)
    persistent = e.heap_top
    values, log, live = [], [], [1, 2, 3]

    def update_live():
        e.write_u64(owner + 0xd8, tree(e, [(objects[i], 0) for i in live], 'Q'))
        e.write_u64(owner + 0xe0, len(live))

    def callback(phase):
        def run(emu):
            obj = ids[emu.reg(UC_X86_REG_RCX)]
            command = emu.reg(UC_X86_REG_RDX) & 0xffffffff
            assert command == values[1]
            count = struct.unpack('<i', e.read(counts+command*4,4))[0]
            if phase == 2:
                log.append(f'F:{obj}:{emu.reg(UC_X86_REG_XMM2)&0xffffffff:08x}:{emu.reg(UC_X86_REG_XMM3)&0xffffffff:08x}:{count}')
            else:
                log.append(f'C:{obj}:{phase}:{count}')
            if obj == 1:
                if values[10] == 1: live[:] = [1, 3]
                elif values[10] == 2: live[:] = [1]
                elif values[10] == 3: live[:] = [1, 2, 3, 4]
                update_live()
                if values[10] == 3: e.write_f32(0x142f018b0, -0.375)
        return run

    def intercept(emu):
        command, begin, end, before = [emu.reg(r) & 0xffffffff for r in [UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9]]
        assert command == values[1] and (begin, end) == ((1,0) if begin else ((0,1) if values[0] == 1 else (0,0)))
        log.append(f'P:{before}')
        if before and values[11]:
            e.write_u64(primary + command*8, objects[4])
            e.write_f32(0x142f018b0, 0.25)
        emu.uc.reg_write(UC_X86_REG_RAX, values[8] & 0xffffffff)

    e.stubs[RET_ADDR+0x100] = callback(0)
    e.stubs[RET_ADDR+0x200] = callback(1)
    e.stubs[RET_ADDR+0x300] = callback(2)
    e.stubs[0x14190ff10] = intercept
    print('# reference_sha256', REFERENCE_SHA256)
    print('# full Begin/Continue/End, masks, override map, observer clone/iteration/destruction unchanged; callback/plugin and alloc/free endpoints stubbed')
    print('# D phase command count block mask source override primary before now_bits observer_mutation primary_mutation dt -> count started_bits primary observers rate_bits dt_bits trace*')
    total = 0
    for case in cases():
        values[:] = case
        phase, command, count, block, mask, source, override, original_primary, before, now, _, _, dt = case
        e.heap_top = persistent
        e.write_u32(counts + command*4, count)
        e.write_u32(times + command*4, 0x3f400000)
        e.write_u64(primary + command*8, objects[original_primary])
        e.write(0x145a0df02, bytes([block])); e.write_u32(0x145777508, mask)
        e.write_u64(0x145777510, tree(e, [], 'i', 'I'))
        e.write_u64(0x1460fddc8, tree(e, [(command, objects[1])] if override else [], 'i', 'Q'))
        # Override map uses key +0x20, unlike the mask map +0x1c.
        if override:
            head=e.read_u64(0x1460fddc8); node=e.read_u64(head+8)
            e.write(node+32,struct.pack('<i',command));e.write_u64(node+40,objects[1])
        e.write_f32(owner, 0.75)
        e.write_f32(0x142f018b0, dt)
        e.write(0x142f01910, struct.pack('<d', now))
        live[:] = [1,2,3]; update_live(); log.clear()
        e.call([0x140c7fac0, 0x140c7fc40, 0x140c7f8c0][phase], ints=[owner, command, source], max_instructions=100000)
        fields=list(case);fields[9]=f'{bits(now):016x}'
        print('D',*fields,'->',struct.unpack('<i',e.read(counts+command*4,4))[0],f'{e.read_u32(times+command*4):08x}',ids[e.read_u64(primary+command*8)],','.join(map(str,live)),f'{e.read_u32(owner):08x}',f'{e.read_u32(0x142f018b0):08x}',*log)
        total += 1
    print('# cases',total)


if __name__ == '__main__':
    main(sys.argv[1])
