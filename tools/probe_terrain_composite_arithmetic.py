"""Compare a terrain1202 arithmetic region with an independent scalar formula.

This is a CPU interpretation with substituted samples, not original GPU output.
"""
import argparse
import hashlib
import json
from zipfile import ZipFile
import numpy as np
from spirv_arithmetic import evaluate

DIGEST = '6c1161db1f9fd629d481b3e282c8535dc090d48e86371cb141d6edf69b6bd95b'


def compare(path):
    with ZipFile(path) as archive:
        raw = archive.read('terrain_1202.spv')
    if hashlib.sha256(raw).hexdigest() != DIGEST:
        raise ValueError('unsupported shader identity')
    rng = np.random.default_rng(1202)
    worst = 0.0
    for _ in range(256):
        base, comp, detail = rng.random((3, 4), dtype=np.float32)
        noise = rng.random((4, 4), dtype=np.float32)
        uv = rng.random(2, dtype=np.float32)
        params = np.zeros((34, 4), dtype=np.float32)
        params[0] = [1, 1, rng.uniform(.05, .8), rng.uniform(0, 1)]
        params[1] = [rng.uniform(.1, 4), rng.uniform(1, 3), rng.uniform(0, 1), rng.uniform(0, 1)]
        params[2] = [rng.uniform(-.5, .5), rng.uniform(-.02, .02), rng.uniform(-1, 1), 1]
        eye = np.array([0, 0, rng.uniform(-100, -1), 1], dtype=np.float32)
        sample_ids = {13847: 0, 13848: 1, 23490: 2, 23491: 3}
        seen = []

        def sample(result, handle, coordinate):
            i = sample_ids[result]
            expected_uv = uv.astype(np.float64) * float(params[1, 0]) * float(params[1, 1]) ** i
            if handle != 'noise' or not np.allclose(coordinate, expected_uv, rtol=1e-6, atol=1e-6):
                raise ValueError('noise coordinate mismatch')
            seen.append(i)
            return noise[i]

        values, evidence = evaluate(raw, 0x2a9c, 0x3100,
                                   {11951: uv, 8219: base, 22181: comp, 22182: detail},
                                   {9116: params, 4230: 'noise', 4959: eye}, sample)
        # Compute separately in float64 to expose rounding differences honestly.
        n = sum((float(noise[i, 2]) - .5) * 2 * float(params[1, 2]) *
                float(params[1, 3]) ** i for i in range(4))
        threshold, width, center = float(params[0, 2]), float(params[0, 3]), float(params[2, 0])
        t = np.clip((n - 2 * width * (float(comp[3]) - .5) - (center - threshold)) /
                    (2 * threshold), 0, 1)
        blend = t * t * (3 - 2 * t)
        distance = np.clip(float(params[2, 1]) * -float(eye[2]) + float(params[2, 2]), 0, 1)
        first = comp.astype(np.float64).copy()
        first[:3] = first[:3] * (1 - blend) + base[:3] * blend
        expected = first * (1 - distance) + detail * distance
        actual = np.asarray(values[10159])
        error = max(float(np.max(np.abs(actual - expected))),
                    abs(float(values[15340]) - blend * (1 - distance)),
                    abs(float(values[9843]) - (1 - blend) * (1 - distance)))
        worst = max(worst, error)
        if error > 3e-6 or seen != [0, 1, 2, 3]:
            raise ValueError(f'arithmetic mismatch {error}')
    return {'module_sha256': DIGEST, 'byte_range': ['0x2a9c', '0x3100'],
            'cases': 256, 'maximum_absolute_error': worst, 'tolerance': 3e-6,
            'last_case_execution': evidence,
            'limitations': 'CPU float32 interpretation against float64 formula; samples and uniforms synthetic. No GPU execution, sampler/filter parity, specialization or full shader equivalence.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive')
    print(json.dumps(compare(parser.parse_args().archive), indent=2))
