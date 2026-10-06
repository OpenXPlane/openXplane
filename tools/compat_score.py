#!/usr/bin/env python3
"""Computes the compatibility estimate from docs/compat.json and refreshes the generated text.

    python3 tools/compat_score.py          # print the summary
    python3 tools/compat_score.py --write  # also update README.md (between the compat markers) and docs/COMPATIBILITY.md

The numbers are a self-assessed rubric (see the 'method' field), not a measurement.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / 'docs' / 'compat.json'
START, END = '<!-- compat:start -->', '<!-- compat:end -->'


def load():
    data = json.loads(DATA.read_text())
    total = sum(s['weight'] for s in data['subsystems'])
    assert total == 100, f'weights sum to {total}, not 100'
    for s in data['subsystems']:
        assert 0 <= s['verified'] <= s['implemented'] <= 1, s['name']
    return data


def totals(data):
    impl = sum(s['weight'] * s['implemented'] for s in data['subsystems'])
    ver = sum(s['weight'] * s['verified'] for s in data['subsystems'])
    return impl, ver


def pct(x):
    return f'{x:.0f}%' if x >= 1 or x == 0 else f'{x:.1f}%'


def table(data):
    rows = ['| Subsystem | Weight | Implemented | Verified identical | Notes |', '|---|---:|---:|---:|---|']
    for s in data['subsystems']:
        rows.append(f"| {s['name']} | {s['weight']} | {s['implemented']*100:.0f}% | {s['verified']*100:.0f}% | {s['note']} |")
    return '\n'.join(rows)


def readme_block(data):
    impl, ver = totals(data)
    road = '\n'.join(f"{i}. **{r['title']}** ({r['state']}): {r['detail']}" for i, r in enumerate(data['roadmap'], 1))
    nxt = '\n'.join(f'- {n}' for n in data['next'])
    return f'''{START}
## How close to the original is it?

**About {impl:.0f}% implemented, about {ver:.0f}% verified identical to the original.** Early stage: it loads
the original content, draws it, and flies an approximation; it does not yet behave like X-Plane.

- *Implemented* counts work that exists in any form, including approximations (the flight model is one).
- *Verified identical* counts only what has been confirmed to match the original's own code, bit for bit.

This is a self-assessed, weighted rubric, not a measurement (method and per-subsystem numbers in
[docs/COMPATIBILITY.md](docs/COMPATIBILITY.md); data in [docs/compat.json](docs/compat.json), updated with
`python3 tools/compat_score.py --write`).

{table(data)}

## Roadmap

{road}

Next up:

{nxt}

Details and the target crate layout: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
{END}'''


def main():
    data = load()
    impl, ver = totals(data)
    print(f'implemented {impl:.1f}%  verified {ver:.1f}%')
    for s in data['subsystems']:
        print(f"  {s['name']:<40} w{s['weight']:>3}  impl {s['implemented']*100:>3.0f}%  verified {s['verified']*100:>3.0f}%")
    if '--write' not in sys.argv:
        return
    readme = ROOT / 'README.md'
    text = readme.read_text()
    block = readme_block(data)
    if START in text:
        text = re.sub(re.escape(START) + r'.*?' + re.escape(END), lambda _: block, text, flags=re.S)
    else:
        marker = '## Current state'
        assert marker in text, 'README has no "## Current state" heading to insert before'
        text = text.replace(marker, block + '\n\n' + marker, 1)
    readme.write_text(text)
    doc = f'''# Compatibility estimate

Updated {data['updated']}. **About {impl:.0f}% implemented, about {ver:.0f}% verified identical to the original.**

{data['method']}

{table(data)}

The generator is `tools/compat_score.py`; the data is `compat.json` in this folder.
'''
    (ROOT / 'docs' / 'COMPATIBILITY.md').write_text(doc)


if __name__ == '__main__':
    main()
