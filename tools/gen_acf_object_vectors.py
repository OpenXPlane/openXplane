#!/usr/bin/env python3
"""The words the original ACF loader leaves in the aircraft object for each reference aircraft.

    python3 tools/gen_acf_object_vectors.py Xplane12/X-Plane.exe crates/xp-acf/tests/data

Writes `acf_object_<file stem>.txt` with `REGION OFFSET=WORD` lines (see run_acf_loader.py).
"""
import glob
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from run_acf_loader import parse_acf, run  # noqa: E402

exe, out = sys.argv[1], Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
for path in sorted(glob.glob('Xplane12/*/*.acf')):
    words, _, error = run(exe, parse_acf(path))
    if error:
        sys.stderr.write(f'{path}: loader stopped: {error}\n')
    name = Path(path).stem
    with open(out / f'acf_object_{name}.txt', 'w') as f:
        f.write(f'# words left by the original loader for {Path(path).name} (tools/gen_acf_object_vectors.py)\n')
        for (region, off), w in sorted(words.items()):
            f.write(f'{region} {off:x}={w:08x}\n')
    print(name, len(words))
