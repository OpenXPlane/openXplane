#!/usr/bin/env python3
"""Synthetic checks of the original attachment gate and its actual view helper.

python tools/generate_attachment_gate_vectors.py <X-Plane.exe> <output.tsv>
Only the bounded gate is executed, with controlled registers and record state.
"""
import hashlib
import random
import sys
from pathlib import Path

from emulate_xp import Emulator, RET_ADDR, STACK_TOP
from generate_attachment_vectors import SHA256
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import (UC_X86_REG_R13, UC_X86_REG_R8, UC_X86_REG_RDI,
                              UC_X86_REG_R15, UC_X86_REG_RSI, UC_X86_REG_RSP,
                              UC_X86_REG_RIP)


def main():
    exe, output = map(Path, sys.argv[1:])
    assert hashlib.sha256(exe.read_bytes()).hexdigest() == SHA256
    e = Emulator(exe)
    aircraft, objects = e.alloc(0x6400), e.alloc(0x240)
    e.write(aircraft, bytes(0x6400))
    e.write(objects, bytes(0x240))
    e.write_u64(aircraft + 0x60e0, objects)
    observed = []

    def endpoint(uc, address, size, user):
        observed.append(address == 0x141a7b72c)
        uc.reg_write(UC_X86_REG_RIP, RET_ADDR)

    for address in (0x141a7b72c, 0x141a7b931):
        e.uc.hook_add(UC_HOOK_CODE, endpoint, begin=address, end=address)
    cases = []
    for flag in (0, 2, 0x100, 0x200, 0x800, 0x1000, 0x1800, 0x2002):
        for mode in range(6):
            for render_pass in (1, 0x19, 0x100):
                cases.append((flag, render_pass, mode, 1, 1, 1, 1))
    rng = random.Random(0xb6a0)
    for _ in range(384):
        cases.append((rng.randrange(0x4000), rng.choice((0,1,8,16,0x19,0x100,0x119)),
                      rng.randrange(-1, 7), *[rng.randrange(2) for _ in range(4)]))
    rows = ["# flags\tpass\tmode\tloaded\tselected-cockpit-loaded\tcockpit-enabled\tmain-view-enabled\taccepted"]
    for flags, render_pass, mode, loaded, selected, cockpit, main in cases:
        e.write_u32(objects + 0x88, flags)
        e.write_u64(objects + 0x90, loaded)
        e.write_u64(objects + 0x120 + 0x90, selected)
        e.write_u32(aircraft + 0x622c, 1)
        e.write_u32(aircraft + 0x6230, 1)
        e.write_u32(aircraft + 0x888, cockpit)
        e.write_u32(aircraft + 0x88c, main)
        regs = ((UC_X86_REG_R13, aircraft), (UC_X86_REG_R8, objects),
                (UC_X86_REG_RDI, 0), (UC_X86_REG_R15, render_pass),
                (UC_X86_REG_RSI, mode & 0xffffffff), (UC_X86_REG_RSP, STACK_TOP - 0x1000))
        for reg, value in regs:
            e.uc.reg_write(reg, value)
        observed.clear()
        e.run_range(0x141a7b6a0, RET_ADDR)
        assert len(observed) == 1, observed
        rows.append("\t".join(map(str, (flags, render_pass, mode, loaded, selected, cockpit, main, int(observed[0])))))
    output.write_text("\n".join(rows) + "\n", encoding="utf-8")
    print(f"{len(cases)} native gate cases; actual view helper, no callee stubs")


if __name__ == "__main__":
    main()
