#!/usr/bin/env python3
"""Mutation check: python3 tools/mutcheck.py FILE TESTFILTER "old=>new" ... (each mutant must make the test fail)."""
import subprocess
import sys

f, test = sys.argv[1], sys.argv[2]
orig = open(f).read()
try:
    for m in sys.argv[3:]:
        a, b = m.split('=>')
        if a not in orig:
            print('MISSING', a)
            continue
        open(f, 'w').write(orig.replace(a, b, 1))
        r = subprocess.run(['cargo', 'test', '--offline', '-p', 'xp-app', '--test', 'original_vectors', test],
                           capture_output=True, text=True)
        print('SURVIVED' if 'test result: ok' in r.stdout else 'killed', a, flush=True)
finally:
    open(f, 'w').write(orig)
