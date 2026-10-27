"""Straight-line SPIR-V arithmetic probe; samples and memory are substituted.

Not a complete shader interpreter: no branches, specialization or derivatives.
Unsupported executed instructions fail. Original code is never emitted.
"""
import struct
import numpy as np
from inspect_spirv_archive import inspect_module


def instructions(raw):
    inspect_module(raw)
    words = struct.unpack('<' + 'I' * (len(raw) // 4), raw)
    offset = 5
    while offset < len(words):
        count = words[offset] >> 16
        yield offset * 4, words[offset] & 65535, words[offset + 1:offset + count]
        offset += count


def evaluate(raw, begin, end, inputs, memory, sample):
    ops = list(instructions(raw))
    boundaries = {offset for offset, _, _ in ops} | {len(raw)}
    if begin not in boundaries or end not in boundaries or begin >= end:
        raise ValueError('range must use ordered instruction boundaries')
    values, types, imports = {}, {}, {}
    for offset, opcode, args in ops:
        if offset >= begin:
            break
        if opcode == 11:
            imports[args[0]] = struct.pack('<' + 'I' * len(args[1:]), *args[1:]).split(b'\0')[0]
        elif opcode in (21, 22):
            types[args[0]] = (opcode, args[1:])
        elif opcode == 43:
            kind, shape = types[args[0]]
            if shape[0] != 32:
                raise ValueError('only 32-bit constants supported')
            values[args[1]] = (np.float32(struct.unpack('<f', struct.pack('<I', args[2]))[0])
                               if kind == 22 else args[2])
        elif opcode == 44:
            values[args[1]] = np.array([values[i] for i in args[2:]], dtype=np.float32)
    values.update(inputs)
    for root in memory:
        values[root] = (root, ())
    samples, executed = 0, 0
    for offset, opcode, a in ops:
        if not begin <= offset < end:
            continue
        executed += 1
        get = lambda i: values[i]
        if opcode == 65:
            root, indices = get(a[2])
            result = (root, indices + tuple(int(get(i)) for i in a[3:]))
        elif opcode == 61:
            root, indices = get(a[2])
            result = memory[root]
            for index in indices:
                result = result[index]
        elif opcode == 87:
            result = np.asarray(sample(a[1], get(a[2]), get(a[3])), dtype=np.float32)
            samples += 1
        elif opcode == 127:
            result = -get(a[2])
        elif opcode in (129, 131, 133, 136, 142):
            left, right = get(a[2]), get(a[3])
            result = {129: np.add, 131: np.subtract, 133: np.multiply,
                      136: np.divide, 142: np.multiply}[opcode](left, right)
        elif opcode == 79:
            joined = np.concatenate((np.atleast_1d(get(a[2])), np.atleast_1d(get(a[3]))))
            result = joined[list(a[4:])]
        elif opcode == 80:
            result = np.concatenate([np.atleast_1d(get(i)) for i in a[2:]])
        elif opcode == 81:
            result = get(a[2])
            for index in a[3:]:
                result = result[index]
        elif opcode == 12:
            if imports.get(a[2]) != b'GLSL.std.450':
                raise ValueError('unsupported extended instruction set')
            operation = a[3]
            x, y, z = [get(i) for i in a[4:]]
            if operation == 43:
                result = np.minimum(np.maximum(x, y), z)
            elif operation == 46:
                result = x * (np.float32(1) - z) + y * z
            elif operation == 49:
                t = np.minimum(np.maximum((z - x) / (y - x), np.float32(0)), np.float32(1))
                result = t * t * (np.float32(3) - np.float32(2) * t)
            else:
                raise ValueError(f'unsupported extended operation {operation}')
        else:
            raise ValueError(f'unsupported opcode {opcode} at {offset:#x}')
        if isinstance(result, (np.ndarray, np.floating)):
            result = np.asarray(result, dtype=np.float32)
        values[a[1]] = result
    return values, {'executed_instructions': executed, 'substituted_samples': samples}
