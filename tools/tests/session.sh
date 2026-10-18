#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
python3 - "$ROOT" <<'PY'
import os, pathlib, subprocess, sys, tempfile
root = pathlib.Path(sys.argv[1])
with tempfile.TemporaryDirectory(prefix='meridian-session-test-') as temp:
    fixture = pathlib.Path(temp)
    fake = fixture / 'shell'
    fake.write_text('#!/usr/bin/python3\nimport os,pathlib\np=pathlib.Path(os.environ["TEST_COUNT"])\nn=int(p.read_text())+1 if p.exists() else 1\np.write_text(str(n))\nraise SystemExit(1 if n==1 else 0)\n')
    fake.chmod(0o755)
    count = fixture / 'count'
    env = {**os.environ, 'MERIDIAN_SHELL': str(fake), 'TEST_COUNT': str(count)}
    subprocess.run(['sh', str(root / 'session/shell-supervisor')], env=env, check=True, timeout=10)
    assert count.read_text() == '2'
    fake.write_text('#!/bin/sh\nexec sleep 30\n')
    process = subprocess.Popen(['sh', str(root / 'session/shell-supervisor')], env=env)
    try:
        process.terminate()
        process.wait(timeout=5)
    finally:
        if process.poll() is None: process.kill(); process.wait()
    assert process.returncode in (0, -15)
print('Shell crash restart and supervisor shutdown passed.')
PY
