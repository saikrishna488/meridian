#!/usr/bin/env python3
"""Check fresh ts-rs exports against the working tree, without requiring a commit.

Run `cargo test -p meridian-protocol` to regenerate working-tree bindings.
Two independent clean exports also catch nondeterministic generation and stale files.
"""
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent

def contents(directory):
    return {str(p.relative_to(directory)): p.read_bytes() for p in directory.rglob('*.ts')}

def main():
    with tempfile.TemporaryDirectory(prefix='meridian-bindings-') as temp:
        results = []
        for name in ('first', 'second'):
            directory = Path(temp) / name
            env = {**os.environ, 'TS_RS_EXPORT_DIR': str(directory)}
            subprocess.run(['cargo', 'test', '-p', 'meridian-protocol', '--lib', 'export_bindings_'], cwd=ROOT, env=env, check=True)
            results.append(contents(directory))
        if not results[0] or results[0] != results[1]:
            raise SystemExit('error: clean protocol exports are empty or nondeterministic')
        actual = contents(ROOT / 'ui/src/lib/generated')
        expected = results[0]
        differences = sorted(key for key in actual.keys() | expected.keys() if actual.get(key) != expected.get(key))
        if differences:
            print('\n'.join(differences))
            raise SystemExit('error: protocol bindings are stale; run cargo test -p meridian-protocol')
        print(f'Checked {len(expected)} deterministic protocol bindings against the working tree.')

if __name__ == '__main__':
    main()
