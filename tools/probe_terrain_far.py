"""Compare native FAR field arithmetic with float32 operations on synthetic inputs."""
import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from unicorn.x86_const import UC_X86_REG_RBP, UC_X86_REG_XMM13, UC_X86_REG_XMM14
from emulate_xp import Emulator
from probe_terrain_shader_registration import REFERENCE_SHA256


def probe(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != REFERENCE_SHA256:
        raise ValueError('unsupported reference identity')
    e = Emulator(path)
    frame = e.alloc(0x200)
    rng = np.random.default_rng(823)
    cases = [(5000.0, 20000.0, 0.03125, 1.0)]
    cases += [(float(x), float(x + delta), float(tiling), float(scale))
              for x, delta, tiling, scale in zip(rng.uniform(1, 10000, 256),
              rng.uniform(10, 20000, 256), rng.uniform(0.01, 2, 256),
              rng.uniform(0.1, 4, 256))]
    for near, (start, end, tiling, scale) in ((near, case) for near in (False, True) for case in cases):
        values = iter(map(np.float32, (start, end, tiling)))

        def read_number(emu):
            emu.set_xmm_f32(0, float(next(values)))

        e.stubs[0x1406ca600] = read_number
        e.uc.reg_write(UC_X86_REG_RBP, frame)
        e.uc.reg_write(UC_X86_REG_XMM13, 0xbf800000)
        e.uc.reg_write(UC_X86_REG_XMM14, 0)
        e.write_f32(0x14611d060, scale)
        begin, stop = ((0x1419aaa33, 0x1419aaa9f) if near
                       else (0x1419aa08e, 0x1419aa0fc))
        e.call(begin, until=stop, max_instructions=100)
        a = np.float32(np.float32(start) * np.float32(scale))
        b = np.float32(np.float32(end) * np.float32(scale))
        slope = np.float32(np.float32(-1.0 if near else 1.0) / np.float32(b - a))
        expected = np.array([slope, np.float32(np.float32(1.0 if near else 0.0) - np.float32(slope * a)),
                             np.float32(tiling)], dtype='<f4').tobytes()
        if e.read(frame + 0x10c, 12) != expected:
            raise ValueError(f'FAR field mismatch: {(start, end, tiling, scale)}')
    return {'reference_sha256': REFERENCE_SHA256, 'cases_per_branch': len(cases),
            'far_range': ['0x1419aa08e', '0x1419aa0fc'],
            'near_range': ['0x1419aaa33', '0x1419aaa9f'],
            'distance_scale_va': '0x14611d060',
            'limitations': 'Starts after recognition; substitutes numeric reader, distance scale and constant registers. Tests finite synthetic inputs, not texture loading, complete parsing or GPU rendering.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable')
    print(json.dumps(probe(parser.parse_args().executable), indent=2))
