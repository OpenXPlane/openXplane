#!/usr/bin/env python3
"""Generate ACF object fixtures only after every original loader call completes.

Usage: python tools/gen_acf_object_vectors.py reference.exe output_dir [--acf file.acf ...]
Without --acf, use Xplane12/*/*.acf. No partial emulator state is published.
"""
import argparse
import glob
import hashlib
from pathlib import Path
from run_acf_loader import parse_acf, run

REFERENCE_SHA = '2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'


def generate(exe, out, acfs):
    exe, out = Path(exe), Path(out)
    if hashlib.sha256(exe.read_bytes()).hexdigest() != REFERENCE_SHA:
        raise ValueError('Reference executable SHA256 mismatch')
    if not acfs:
        raise ValueError('No ACF inputs found; specify --acf or run from the reference workspace')
    pending = []
    names = set()
    for path in acfs:
        path = Path(path)
        if path.stem in names:
            raise ValueError(f'Duplicate fixture stem: {path.stem}')
        names.add(path.stem)
        words, _, error = run(str(exe), parse_acf(path))
        if error:
            raise RuntimeError(f'{path}: original loader did not complete: {error}')
        text = (f'# words left by the original loader for {path.name} (tools/gen_acf_object_vectors.py)\n'
                f'# reference_sha256 {REFERENCE_SHA}; original return reached; budget 200000000\n')
        text += ''.join(f'{region} {off:x}={word:08x}\n' for (region, off), word in sorted(words.items()))
        pending.append((path.stem, text, len(words)))
    # Finish and validate all replays before replacing any existing fixture.
    out.mkdir(parents=True, exist_ok=True)
    for stem, text, count in pending:
        target = out / f'acf_object_{stem}.txt'
        temporary = target.with_suffix('.txt.tmp')
        temporary.write_text(text)
        temporary.replace(target)
        print(stem, count)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--acf', type=Path, action='append')
    args = parser.parse_args()
    generate(args.binary, args.output, args.acf or sorted(glob.glob('Xplane12/*/*.acf')))


if __name__ == '__main__':
    main()
