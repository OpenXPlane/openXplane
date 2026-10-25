#!/usr/bin/env python3
"""Execute actual mode/header/singleton and their secondary caller; synthetic leaves.

Original executable and numeric corpus stay local. Provider boundaries compare
declared maps and live operand descriptors; M/H also compare all local bytes.
"""
import hashlib
import itertools
import sys
from pathlib import Path
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_RIP
from emulate_xp import Emulator, RET_ADDR, STACK_TOP
from extract_commands import REFERENCE_SHA256

SINGLETON = 0x145a12340
FUNCTIONS = {'L': 0x1414cc7f0, 'M': 0x1414cedd0, 'H': 0x1411222e0, 'S': 0x14113ff10, 'G': 0x14114d330}
SP = STACK_TOP - 0x1028
MASK = (1 << 64) - 1


def main(path):
    assert hashlib.sha256(Path(path).read_bytes()).hexdigest() == REFERENCE_SHA256
    e = Emulator(path)
    receiver = e.alloc(0x1c0)
    owner = e.alloc(0x2d0)
    head, node = e.alloc(40), e.alloc(40)
    settings, other, catalog = e.alloc(64), e.alloc(64), e.alloc(64)
    heaps = [e.alloc(0x1100) for _ in range(6)]
    array = e.alloc(0x6668 * 2)
    headers = [e.alloc(32) for _ in range(4)]
    ranges = {receiver: 0x1c0, head: 40, node: 40, settings: 64, other: 64,
              catalog: 64, SINGLETON: 8, 0x142640504: 8, 0x146124398: 8,
              0x145899fd0: 16, 0x1424ff288: 1,
              **{p: 64 for p in heaps}, **{p: 32 for p in headers},
              array + 0x6178: 32, array + 0x6668 + 0x6178: 32}
    h = {}
    counts = {k: 0 for k in FUNCTIONS}
    entries = dict.fromkeys(FUNCTIONS, 0)
    returns = dict.fromkeys(FUNCTIONS, 0)
    calls = {}

    def snap(extra=None):
        return '/'.join(f'{p}:{e.read(p,n).hex()}' for p, n in sorted({**h['ranges'], **(extra or {})}.items()))

    def string(p, text, profile=0, heap=0):
        raw = bytearray(32)
        cap = [15, 31, 4095, 4127][profile]
        raw[16:24] = len(text).to_bytes(8, 'little')
        raw[24:32] = cap.to_bytes(8, 'little')
        if profile:
            data = heap + (32 if profile >= 2 else 0)
            if profile >= 2:
                e.write_u64(data - 8, heap + (h['case'] % 4) * 8)
            raw[:8] = data.to_bytes(8, 'little')
            e.write(data, text + b'\0')
        else:
            raw[:len(text)] = text
        e.write(p, raw)

    def leaf(fn, n):
        a = [e.reg(r) for r in [UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9][:n]]
        before_extra, after_extra = {}, {}
        if fn == 0x1414d45f0:
            before_extra = {a[2]: 32, a[3]: 32}
            after_extra = {a[1]: 32, a[2]: 32, a[3]: 32}
        elif fn == 0x1414ccc80:
            after_extra = {a[1]: 32}
        elif fn == 0x1417e8a60:
            before_extra = after_extra = {a[1]: 32}
        elif fn == 0x1417e89c0:
            before_extra = after_extra = {a[1]: 16}
        elif fn == 0x1423c0180:
            before_extra = {p: a[2] for p in a[:2] if a[2] and p >= STACK_TOP - 0x2000}
        elif fn == 0x1422e7c2c and h.get('mode_stack') is not None and h['mutate']:
            sp = h['mode_stack']
            after_extra = {sp + 0x20: 32, sp + 0x40: 32}
        before = snap(before_extra)
        result = 0
        c = h['case']
        if fn == 0x1422e7bf0:
            assert a == [1]
            result = 0 if h['null_alloc'] else settings + c % 16
            # A reentrant allocator may publish another value; the caller must overwrite it.
            e.write_u64(SINGLETON, other)
        elif fn == 0x1414d45e0:
            result = settings + h['queries'] % 16
        elif fn == 0x1414d45f0:
            h['queries'] += 1
            profile = h['profile']
            text = h['texts'][min(h['queries'] - 1, len(h['texts']) - 1)]
            string(a[1], text, profile, heaps[0])
            result = a[1]
            if h['alias']:
                string(headers[0], text, 1 if c % 2 else 0, heaps[3])
                result = headers[0]
            if h['mutate']:
                string(a[2], b'key', [1, 2, 3][c % 3], heaps[1])
                string(a[3], b'default', [3, 2, 1][c % 3], heaps[2])
                if h['kind'] in ['S', 'G']:
                    if h['queries'] == 1:
                        e.write_u32(0x145899fd8, 0x100 if c % 3 == 0 else 0)
                        e.write_u32(0x145899fdc, 0xffffffff if c % 5 == 0 else 0)
                    elif h['queries'] == 2:
                        e.write_u32(0x145899fd0, 0x100 if c % 3 == 1 else 0)
                        e.write_u32(0x145899fd4, 0xffffffff if c % 5 == 1 else 0)
                    elif h['queries'] == 3:
                        index = [0, 1, 0xffffffff][c % 3]
                        e.write_u32(0x142640508, index)
                        e.write_u64(0x146124398, array + (0x6668 if index == 0xffffffff else 0))
        elif fn == 0x1423c0180:
            result = int(e.read(a[0], a[2]) != e.read(a[1], a[2])) if a[2] else 0
            if h['override'] is not None:
                result = h['override']
        elif fn == 0x1414ccc80:
            assert a[2] == receiver + 0x20
            string(a[1], b'tow', h['profile'], heaps[4])
            # Return identity is deliberately different; header ignores RAX here.
            result = other
        elif fn == 0x1417e9f50:
            h['catalogs'] += 1
            result = catalog + h['catalogs'] % 16
        elif fn in [0x1417e8a60, 0x1417e89c0, 0x1417e9a40]:
            index = h['lookups']
            h['lookups'] += 1
            result = headers[index] if index == h['found'] else 0
            if fn == 0x1417e9a40:
                assert a[1] == 0
            if h['mutate']:
                # Cache the chosen resource address but observe its late contents.
                string(array + 0x6668 + 0x6178, b'late')
                e.write_u64(0x146124398, array + 0x6668)
        elif fn == 0x1422e7c2c:
            h['frees'] += 1
            if h.get('mode_stack') is not None and h['mutate']:
                # Exercise live cleanup reloads rather than a cached three-string list.
                sp = h['mode_stack']
                string(sp + 0x20, b'free-key', 2, heaps[1])
                string(sp + 0x40, b'free-default', 3, heaps[2])
            result = 0xbadc0de000000000 | h['frees']
        else:
            raise AssertionError(hex(fn))
        calls[hex(fn)] = calls.get(hex(fn), 0) + 1
        h['events'].append(f'I~{fn}{"" if not a else "," + ",".join(map(str,a))}~{before}~{snap(after_extra)}~{result}')
        e.uc.reg_write(UC_X86_REG_RAX, result)

    for fn, n in [(0x1422e7bf0, 1), (0x1414d45e0, 0), (0x1414d45f0, 4),
                  (0x1423c0180, 3), (0x1414ccc80, 3), (0x1417e9f50, 0),
                  (0x1417e8a60, 2), (0x1417e89c0, 2), (0x1417e9a40, 2), (0x1422e7c2c, 2)]:
        e.stubs[fn] = lambda emu, fn=fn, n=n: leaf(fn, n)
    points = {a: (k, False) for k, a in FUNCTIONS.items()}
    for k, addresses in {'L': [0x1414cc813], 'M': [0x1414cef5c],
                         'H': [0x1411223fa, 0x141122402],
                         'G': [0x14114d390, 0x14114d39d],
                         'S': [0x14113ff89, 0x14113ffdb, 0x14113ffe3]}.items():
        points.update({a: (k, True) for a in addresses})

    def count(uc, a, size, user):
        k, ret = points[a]
        (returns if ret else entries)[k] += 1
        if k == 'M':
            from unicorn.x86_const import UC_X86_REG_RSP
            h['mode_stack'] = None if ret else e.reg(UC_X86_REG_RSP) - 0x88

    for p in points:
        e.uc.hook_add(UC_HOOK_CODE, count, begin=p, end=p)

    def run(kind, profile=0, text=b'1', alias=False, mutate=False, found=0,
            cached=True, null_alloc=False, member=True, override=None, sequence=None, flags=0):
        c = sum(counts.values())
        stack = SP - {'L': 0x28, 'M': 0x88, 'H': 0x58, 'S': 0x28, 'G': 0x28}[kind]
        local = {stack + 0x20: 96 if kind == 'M' else 48} if kind in ['M', 'H'] else {}
        h.clear()
        gate = {owner: 0x2d0, 0x142f2dc95: 1} if kind == 'G' else {}
        h.update(case=c, kind=kind, ranges={**ranges, **local, **gate}, events=[], profile=profile,
                 texts=sequence or [text], alias=alias, mutate=mutate, found=found,
                 null_alloc=null_alloc, override=override, queries=0, catalogs=0, lookups=0, frees=0)
        for p, n in h['ranges'].items():
            e.write(p, bytes([[0, 0xa5, 255][c % 3]]) * n)
        e.write(STACK_TOP - 0x2000, bytes([[0, 0xa5, 255][c % 3]]) * 0x1000)
        e.write_u64(SINGLETON, settings if cached else 0)
        if kind == 'G':
            e.write_u64(owner + 0xc8, receiver)
            e.write(owner + 0xc0, b'\1')
            e.write(0x142f2dc95, b'\1')
        e.write(0x1424ff288, b'1')
        e.write_u64(receiver + 0x1a0, head)
        e.write(head, bytes(40)); e.write(node, bytes(40))
        e.write(head + 0x19, b'\1'); e.write_u64(head + 8, node if member else head)
        e.write_u64(node, head); e.write_u64(node + 0x10, head); e.write_u32(node + 0x1c, 2)
        e.write(0x142640504, bytes(8)); e.write_u32(0x142640508, 1)
        e.write_u64(0x146124398, array)
        for i in range(4): e.write_u32(0x145899fd0 + 4 * i, 0x100 if flags & (1 << i) else 0)
        for p in [receiver + 0x20, array + 0x6178, array + 0x6668 + 0x6178, *headers]:
            string(p, b'')
        initial = snap()
        argument = owner if kind == 'G' else receiver
        result = e.call(FUNCTIONS[kind], ints=[argument], max_instructions=10000)
        assert e.reg(UC_X86_REG_RIP) == RET_ADDR
        if kind == 'G':
            result &= 255
            assert result in [0, 1]
        counts[kind] += 1
        print(f'V {kind} {argument},{stack} {initial} -> {snap()} {result} {"|".join(h["events"]) or "-"}')

    print('# reference_sha256', REFERENCE_SHA256)
    for cached, null in itertools.product([False, True], repeat=2):
        for _ in range(12): run('L', cached=cached, null_alloc=null)
    for profile, text, alias, mutate in itertools.product(range(4), [b'', b'1', b'0', b'11', b'\0', b'\xff'], [False, True], [False, True]):
        run('M', profile, text, alias, mutate)
    for value in [0, 0xffffffff, 0x100000000, 0x7654321000000100, MASK]:
        for profile in range(4): run('M', profile, override=value, mutate=True)
    for profile, found, cached, member, mutate in itertools.product(range(4), range(5), [False, True], [False, True], [False, True]):
        run('H', profile, found=found, cached=cached, member=member, mutate=mutate)
    for bits, flags, profile, found, mutate in itertools.product(range(8), [0, 1, 2, 4, 8, 15], range(4), range(4), [False, True]):
        run('S', profile, found=found, cached=False, mutate=mutate,
            sequence=[b'1' if bits & (1 << i) else b'0' for i in range(3)], flags=flags)
    for bits, profile, found in itertools.product(range(8), range(4), range(4)):
        run('G', profile, found=found, cached=False, mutate=True,
            sequence=[b'1' if bits & (1 << i) else b'0' for i in range(3)])
    assert entries == returns, (entries, returns)
    print('# counts', counts)
    print('# actual_entries', entries)
    print('# actual_returns', returns)
    print('# controlled_leaves', calls)


if __name__ == '__main__':
    main(sys.argv[1])
