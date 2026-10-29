#!/usr/bin/env python3
"""Count the datarefs an installed aircraft's OBJ animations reference against the reference
registration table (tools/extract_datarefs.py output) and against names present in the port sources.

Usage: dataref_object_coverage.py <registration.tsv> <aircraft-dir> [--top N]
"Named in the port sources" is a plain substring search over crates/**/*.rs and *.tsv. It says a name
appears somewhere, not that a value is backed by simulation state.
"""
import argparse
import collections
import glob
import re


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('table')
    ap.add_argument('aircraft')
    ap.add_argument('--top', type=int, default=40)
    args = ap.parse_args()
    ref = {}
    for line in open(args.table, encoding='utf-8'):
        parts = line.rstrip('\n').split('\t')
        if len(parts) >= 2:
            ref[parts[0]] = parts[1]
    used = collections.Counter()
    for path in glob.glob(args.aircraft + '/**/*.obj', recursive=True):
        for line in open(path, errors='ignore'):
            words = line.split()
            if words and words[0].startswith('ANIM_'):
                for arg in words[1:]:
                    if '/' in arg and arg != 'none':
                        used[re.sub(r'\[\d+\]$', '', arg)] += 1
    source = ''
    for path in glob.glob('crates/**/*.rs', recursive=True) + glob.glob('crates/**/*.tsv', recursive=True):
        source += open(path, errors='ignore').read()
    in_ref = [d for d in used if d in ref]
    named = [d for d in used if d in source]
    print(f'distinct datarefs referenced by animations: {len(used)}')
    print(f'in the reference registration table: {len(in_ref)}')
    print(f'named in the port sources: {len(named)}')
    print(f'not in the table (custom, command or dynamically registered): {len(used) - len(in_ref)}')
    unknown = sorted((d for d in in_ref if d not in source), key=lambda d: -used[d])
    print(f'in the table but not named by the port: {len(unknown)}')
    for d in unknown[:args.top]:
        print(f'  {used[d]:4d} {d} {ref[d]}')


if __name__ == '__main__':
    main()
