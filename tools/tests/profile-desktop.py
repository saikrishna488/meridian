#!/usr/bin/env python3
"""Regression checks for process grouping, incomplete PSS and PID reuse."""
import importlib.util
from pathlib import Path
import unittest
import sys
sys.dont_write_bytecode = True

spec = importlib.util.spec_from_file_location('profile_desktop', Path(__file__).parents[1] / 'profile-desktop.py')
profile = importlib.util.module_from_spec(spec)
spec.loader.exec_module(profile)


def row(pid, parent, *, pss=20, ticks=200, start=1):
    return dict(pid=pid, ppid=parent, name='fixture', start_ticks=start,
                cpu_ticks=ticks, rss_kib=40, pss_kib=pss, context_switches=10)


class ProfileTest(unittest.TestCase):
    def test_overlapping_roots_are_counted_once_and_unrelated_processes_excluded(self):
        table = {1: row(1, 0), 2: row(2, 1), 3: row(3, 2), 4: row(4, 1), 5: row(5, 0)}
        result = profile.sample(table, {}, 0, 2, 1, [4])
        self.assertEqual(result['shell']['processes'], 2)
        self.assertEqual(result['desktop_stack']['processes'], 4)
        self.assertEqual(result['desktop_stack']['pss_kib'], 80)
        self.assertEqual({r['pid'] for r in result['processes']}, {1, 2, 3, 4})

    def test_missing_pss_and_reused_pid_do_not_report_false_measurements(self):
        old = {1: row(1, 0), 2: row(2, 1)}
        current = {1: row(1, 0, pss=None, ticks=300), 2: row(2, 1, ticks=1, start=99)}
        result = profile.sample(current, old, 1, 1, None, [])
        self.assertIsNone(result['shell']['pss_kib'])
        self.assertEqual(result['shell']['pss_unreadable_processes'], 1)
        self.assertEqual(result['shell']['cpu_unmeasured_processes'], 1)
        self.assertGreater(result['processes'][0]['cpu_percent'], 0)
        self.assertIsNone(result['processes'][1]['cpu_percent'])


if __name__ == '__main__':
    unittest.main()
