#!/usr/bin/env python3
"""Make a private controlled ACF pose fixture, reading original assets in place.

Output must stay under research/local and must not already exist. Original
property values in the output are private research data and must not be committed.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('aircraft', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--angles', type=float, nargs=3, default=(-15, 20, 35),
                        metavar=('PSI', 'THETA', 'PHI'))
    args = parser.parse_args()
    local = Path(__file__).resolve().parent.parent / 'research' / 'local'
    output = args.output.resolve()
    metadata = output.with_suffix('.json')
    if not output.is_relative_to(local.resolve()) or output.suffix != '.acf':
        parser.error('output must be an .acf file under research/local')
    if output.exists() or metadata.exists():
        parser.error('probe or metadata already exists; choose a new output path')
    if not all(math.isfinite(v) for v in args.angles):
        parser.error('angles must be finite')
    source = args.aircraft.resolve()
    raw = source.read_bytes()
    lines = raw.decode('utf-8-sig', errors='replace').splitlines()
    if lines[:3] not in (['I', '1200 Version', 'ACF'], ['A', '1200 Version', 'ACF']):
        parser.error('probe currently requires ACF1200')
    angles = dict(zip(('_v10_att_psi_ref', '_v10_att_the_ref', '_v10_att_phi_ref'), args.angles))
    rows = []
    for line in lines:
        if not line.startswith('P _obja/'):
            continue
        key, _, value = line[2:].partition(' ')
        field = key.rsplit('/', 1)[-1]
        if field == '_v10_att_file_stl' and value:
            value = str((source.parent / 'objects' / value.replace('\\', '/')).resolve())
        if field in angles:
            value = str(angles[field])
        rows.append(f'P {key} {value}')
    if not rows:
        parser.error('no object attachment properties found')
    content = ('I\n1200 Version\nACF\nPROPERTIES_BEGIN\n' + '\n'.join(rows) + '\nPROPERTIES_END\n').encode()
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open('xb') as file:
        file.write(content)
    source_hash = hashlib.sha256(raw).hexdigest()
    assert hashlib.sha256(source.read_bytes()).hexdigest() == source_hash, 'source changed during probe creation'
    with metadata.open('x', encoding='utf-8') as file:
        json.dump({'source': str(source), 'source_sha256': source_hash,
                   'probe_sha256': hashlib.sha256(content).hexdigest(),
                   'angles_psi_the_phi': args.angles,
                   'note': 'Controlled local ACF fixture, not original runtime state; source unchanged.'}, file, indent=2)
    print(f'Created private probe {output.name}; installed source unchanged.')


if __name__ == '__main__':
    main()
