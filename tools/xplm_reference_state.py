"""Initialized synthetic Windows TLS/plugin-record state for pinned XPLM CPU replays."""
from unicorn.x86_const import UC_X86_REG_GS_BASE
import struct


def install_tls(emu, stack=(), enabled=(), pending=-1):
    teb, slots, tls, vector = emu.alloc(0x80), emu.alloc(8), emu.alloc(0x40), emu.alloc(256)
    emu.write(teb, bytes(0x80)); emu.write(tls, bytes(0x40)); emu.write(vector, bytes(256))
    emu.write_u64(teb + 0x58, slots); emu.write_u64(slots, tls)
    emu.write_u32(0x18006a0c0, 0)
    emu.uc.reg_write(UC_X86_REG_GS_BASE, teb)
    emu.write_u64(tls + 0x10, vector)
    emu.write_u64(tls + 0x18, vector + len(stack) * 4)
    emu.write_u64(tls + 0x20, vector + 256)
    emu.write(tls + 0x38, b'\1')
    if stack: emu.write(vector, struct.pack('<' + 'i' * len(stack), *stack))
    records = emu.alloc(max(1, len(enabled)) * 0x110)
    emu.write(records, bytes(max(1, len(enabled)) * 0x110))
    for i, byte in enumerate(enabled): emu.write(records + i * 0x110 + 0x88, bytes([byte]))
    emu.write_u64(0x180069c58, records)
    emu.write_u64(0x180069c60, records + len(enabled) * 0x110)
    emu.write_u32(0x180068290, pending)
    return tls, vector, records


def read_stack(emu, tls):
    begin, end = emu.read_u64(tls + 0x10), emu.read_u64(tls + 0x18)
    assert end >= begin and end - begin <= 256 and (end - begin) % 4 == 0
    n = (end - begin) // 4
    return list(struct.unpack('<' + 'i' * n, emu.read(begin, n * 4)))
