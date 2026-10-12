#!/usr/bin/env python3
"""Derives the ACF -> aircraft-object schema by running the original loader (0x141a972a0) in the emulator.

    python3 tools/derive_acf_schema.py Xplane12/X-Plane.exe OUT.tsv [VARIANTS]

The reference files (`Xplane12/Cessna 172 SP/*.acf`) and VARIANTS randomised copies (every decimal value replaced
by a random one) are loaded; each reader call gives a key, its destination in the aircraft object or a
sub-object region and the word the loader left there. Keys that differ only in integer path components form one
template; the destination is linear in those indices. The transform from the parsed value to the stored word is
chosen from: the value itself, the value times a constant, the value times a constant minus a centre-of-gravity
word; the choice must reproduce every instance bit for bit. Templates that no rule explains are written with the
transform `?` and listed on stderr.

Output columns: template, reader kind, region, base offset, strides (comma separated, one per index), transform,
and for a scaled read the reader's coefficient (float32 bits).
"""
import collections
import glob
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from run_acf_loader import parse_acf, run  # noqa: E402


def f32(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


def tof(bits):
    return struct.unpack('<f', struct.pack('<I', bits & 0xffffffff))[0]


def randomise(entries, rng):
    out = {}
    for key, value in entries.items():
        text = value.decode(errors='replace')
        if '.' in text:
            try:
                float(text)
                out[key] = f'{rng.uniform(-6, 12) if rng.random() < 0.3 else rng.uniform(0.2, 9):.9f}'.encode()
                continue
            except ValueError:
                pass
        out[key] = value
    return out


def split_key(key):
    """The key with integer components (and comma-separated integer lists) replaced by `{}`, and the integers."""
    idx, parts = [], []
    for p in key.split('/'):
        if p and all(c.isdigit() or c == ',' for c in p) and p[0].isdigit() and p[-1].isdigit():
            nums = p.split(',')
            if all(n.isdigit() for n in nums):
                idx.extend(int(n) for n in nums)
                parts.append(','.join('{}' for _ in nums))
                continue
        parts.append(p)
    return '/'.join(parts), tuple(idx)


def template_of(key):
    return split_key(key)[0]


def collect(exe, variants):
    emu = Emulator(exe)
    instances = collections.defaultdict(list)  # (template, kind, region) -> [(indices, offset, raw text, final, cg)]
    for entries in variants:
        words, reads, error = run(exe, entries, trace_reads=True, emu=emu)
        if error:
            sys.stderr.write(f'loader stopped: {error}\n')
        cg = (words.get(('B', 0x2850), 0), words.get(('B', 0x2854), 0))
        for kind, key, region, offset, coeff in reads:
            if region == '?':
                continue
            raw = entries.get(key.encode())
            if raw is None:
                continue
            idx = split_key(key)[1]
            instances[(template_of(key), kind, region)].append(
                (idx, offset, raw.decode(errors='replace'), words.get((region, offset), 0), cg, coeff))
    return instances


def fit_layout(items):
    """Base and per-index strides of the destination offset, or None when it is not linear."""
    d = {}
    for idx, off, *_ in items:
        if d.setdefault(idx, off) != off:
            return None
    n = len(next(iter(d)))
    z = min(d)
    base = d[z]
    strides = []
    for k in range(n):
        e = list(z)
        e[k] += 1
        strides.append(d[tuple(e)] - base if tuple(e) in d else 0)
    for idx, off in d.items():
        if base + sum(s * (i - j) for s, i, j in zip(strides, idx, z)) != off:
            return None
    return base - sum(s * j for s, j in zip(strides, z)), strides


def choose(kind, items, floors=()):
    """The transform that reproduces every instance, as a string."""
    if kind == 'string':
        return 'string'
    if kind == 'int':
        try:
            ok = all(int(float(t)) & 0xffffffff == fin for _, _, t, fin, _, _ in items if _is_number(t))
        except ValueError:
            return '?'
        return 'int' if ok else '?'
    values = []
    for idx, off, text, fin, cg, coeff in items:
        if not _is_number(text):
            continue
        raw = tof(f32(float(text)))
        if kind == 'coeff':
            raw = tof(f32(raw * tof(coeff)))
        values.append((raw, fin, cg))
    if not values:
        return '?'
    if all(f32(r) == fin for r, fin, _ in values):
        return 'raw'
    # a constant factor: estimate from the largest values, then search the neighbouring float32 values
    big = max(values, key=lambda v: abs(v[0]))
    if abs(big[0]) > 1e-6:
        estimate = f32(tof(big[1]) / big[0])
        for delta in range(-8, 9):
            c = tof(estimate + delta)
            if all(f32(r * c) == fin for r, fin, _ in values):
                return f'scale:{f32(c):08x}'
            for which, name in ((0, 'cgy'), (1, 'cgz')):
                if all(f32(tof(f32(r * c)) - tof(cg[which])) == fin for r, fin, cg in values):
                    return f'scale_minus_{name}:{f32(c):08x}'
    # a value below a limit replaced by a default (`extract_acf_floors.py`), optionally after scaling
    scales = [None]
    if abs(big[0]) > 1e-6:
        kept = [(r, fin) for r, fin, _ in values if fin not in {d for _, _, _, d in floors}]
        if kept:
            r0, f0 = max(kept, key=lambda v: abs(v[0]))
            if abs(r0) > 1e-6:
                est = f32(tof(f0) / r0)
                scales += [tof(est + k) for k in range(-4, 5)]
    for width, limit, default in [(w, l, dflt) for w, l, _, dflt in floors for l in [l]]:
        lim = struct.unpack('<d', struct.pack('<Q', limit))[0] if width == 'd' else tof(limit)
        for scale in scales:
            def stored(r):
                v = r if scale is None else tof(f32(r * scale))
                return default if lim > v else f32(v)
            if all(stored(r) == fin for r, fin, _ in values):
                tag = 'raw' if scale is None else f'{f32(scale):08x}'
                return f'floor:{width}:{limit:x}:{default:08x}:{tag}'
    # a constant factor with a centre-of-gravity term whose scale is not the ratio of the largest value
    for c in (0x3e9c0ebf, 0x3f800000):
        for which, name in ((0, 'cgy'), (1, 'cgz')):
            if all(f32(tof(f32(r * tof(c))) - tof(cg[which])) == fin for r, fin, cg in values):
                return f'scale_minus_{name}:{c:08x}'
    return '?'


def _is_number(text):
    try:
        float(text)
        return True
    except ValueError:
        return False


def load_floors(path):
    rows = {}
    if Path(path).exists():
        for line in open(path):
            f = line.rstrip('\n').split('\t')
            if len(f) == 6:
                rows.setdefault(int(f[1], 16), []).append((f[3], int(f[4], 16), f[2], int(f[5], 16)))
    return rows


def main():
    exe, out = sys.argv[1], sys.argv[2]
    floors = load_floors('crates/xp-acf/data/object_floors.tsv')
    count = int(sys.argv[3]) if len(sys.argv) > 3 else 6
    rng = random.Random(1200)
    bases = [parse_acf(p) for p in sorted(glob.glob('Xplane12/Cessna 172 SP/*.acf'))]
    variants = list(bases)
    for i in range(count):
        variants.append(randomise(bases[i % len(bases)], rng))
    instances = collect(exe, variants)
    rows, unresolved = [], []
    for (template, kind, region), items in sorted(instances.items()):
        layout = fit_layout(items)
        if layout is None:
            unresolved.append((template, 'layout'))
            continue
        base, strides = layout
        fl = [(w, l, r, dflt) for w, l, r, dflt in floors.get(base, [])]
        transform = choose(kind, items, fl)
        if transform == '?' and layout[1] and kind != 'int':
            # the transform may depend on the last index (x, y, z of a point) ...
            last = sorted({idx[-1] for idx, *_ in items})
            if len(last) <= 4:
                parts = [choose(kind, [it for it in items if it[0][-1] == v], fl) for v in last]
                if '?' not in parts:
                    transform = 'last:' + '|'.join(f'{v}={t}' for v, t in zip(last, parts))
        if transform == '?' and len(layout[1]) >= 2 and kind != 'int':
            # ... or on the first one (an object that is converted differently from the others)
            first = sorted({idx[0] for idx, *_ in items})
            parts = {v: choose(kind, [it for it in items if it[0][0] == v], fl) for v in first}
            if '?' not in parts.values():
                default = collections.Counter(parts.values()).most_common(1)[0][0]
                transform = 'first:' + default + ''.join(f'|{v}={t}' for v, t in parts.items() if t != default)
        if transform == '?':
            unresolved.append((template, kind))
        coeffs = {c for *_, c in items}
        if kind == 'coeff' and len(coeffs) > 1:
            unresolved.append((template, 'coefficient varies'))
        coefficient = f'{next(iter(coeffs)):08x}' if kind == 'coeff' else ''
        rows.append((template, kind, region, f'{base:x}', ','.join(str(s) for s in strides), transform, coefficient))
    # the words the loader sets when no property is present: the defaults of absent keys
    words, _, error = run(exe, {}, emu=Emulator(exe))
    with open(out + '.defaults', 'w') as f:
        for (region, off), w in sorted(words.items()):
            f.write(f'{region}\t{off:x}\t{w:08x}\n')
    sys.stderr.write(f'{len(words)} default words\n')
    with open(out, 'w') as f:
        for r in rows:
            f.write('\t'.join(r) + '\n')
    sys.stderr.write(f'{len(rows)} templates, {len(unresolved)} unresolved\n')
    for t, why in unresolved[:80]:
        sys.stderr.write(f'  {why}: {t}\n')


if __name__ == '__main__':
    main()
