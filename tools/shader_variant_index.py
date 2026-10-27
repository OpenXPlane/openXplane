"""Recovered mixed-radix slot arithmetic; parameter meanings are caller-owned."""


def encode(ranges, values, prefix=None):
    if len(ranges) != len(values):
        raise ValueError("range/value length mismatch")
    digits = list(values)
    radices = list(ranges)
    if prefix is not None:
        if len(prefix) != 2:
            raise ValueError("expected two prefix values")
        digits = list(prefix) + digits
        radices = [2, 2] + radices
    slot, multiplier = 0, 1
    for radix, digit in zip(radices, digits):
        if radix <= 0 or not 0 <= digit < radix:
            raise ValueError("digit outside positive radix")
        slot = (slot + multiplier * digit) & 0xffffffff
        multiplier = (multiplier * radix) & 0xffffffff
    return slot


def decode(ranges, slot, with_prefix=True):
    radices = ([2, 2] if with_prefix else []) + list(ranges)
    if slot < 0 or any(r <= 0 for r in radices):
        raise ValueError("invalid slot or radix")
    digits = []
    for radix in radices:
        slot, digit = divmod(slot, radix)
        digits.append(digit)
    if slot:
        raise ValueError("slot outside radix product")
    return (digits[:2], digits[2:]) if with_prefix else (None, digits)
