#!/usr/bin/env python3
"""Exercise stale, missing, unexpected and nondeterministic binding rejection."""
import contextlib
import importlib.util
import io
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('bindings', Path(__file__).resolve().parents[1] / 'check-bindings.py')
bindings = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bindings)

class BindingsCheck(unittest.TestCase):
    def check(self, actual, generated, succeeds):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            directory = root / 'ui/src/lib/generated'
            directory.mkdir(parents=True)
            for name, value in actual.items():
                (directory / name).write_text(value)
            calls = []
            def generate(*args, **kwargs):
                out = Path(kwargs['env']['TS_RS_EXPORT_DIR'])
                out.mkdir()
                for name, value in generated[min(len(calls), len(generated)-1)].items():
                    (out / name).write_text(value)
                calls.append(out)
            with patch.object(bindings, 'ROOT', root), patch.object(bindings.subprocess, 'run', side_effect=generate), contextlib.redirect_stdout(io.StringIO()):
                if succeeds:
                    bindings.main()
                else:
                    with self.assertRaises(SystemExit):
                        bindings.main()
            self.assertEqual(len(calls), 2)
            self.assertEqual(bindings.contents(directory), {k: v.encode() for k, v in actual.items()})
    def test_uncommitted_but_current_outputs_pass(self):
        self.check({'A.ts': 'current'}, [{'A.ts': 'current'}], True)
    def test_stale_missing_and_unexpected_outputs_fail(self):
        for actual in ({'A.ts': 'old'}, {}, {'A.ts': 'current', 'Old.ts': 'stale'}):
            with self.subTest(actual=actual):
                self.check(actual, [{'A.ts': 'current'}], False)
    def test_empty_exports_fail(self):
        self.check({}, [{}], False)
    def test_nondeterministic_exports_fail(self):
        self.check({'A.ts': 'one'}, [{'A.ts': 'one'}, {'A.ts': 'two'}], False)

if __name__ == '__main__':
    unittest.main()
