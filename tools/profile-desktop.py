#!/usr/bin/env python3
"""Sample explicitly selected desktop processes without collecting command arguments.

CPU is percent of one logical CPU. RSS double-counts shared mappings; PSS apportions
those mappings. Unreadable PSS is reported as missing, never silently as zero.
"""
import argparse
import json
import os
from pathlib import Path
import time


def processes(roots):
    result = {}
    for path in Path('/proc').iterdir():
        if not path.name.isdigit():
            continue
        try:
            raw = (path / 'stat').read_text()
            name = raw[raw.index('(') + 1:raw.rindex(')')]
            fields = raw[raw.rindex(')') + 2:].split()
            status = dict(line.split(':', 1) for line in (path / 'status').read_text().splitlines() if ':' in line)
            result[int(path.name)] = {
                'pid': int(path.name), 'ppid': int(fields[1]), 'name': name,
                'start_ticks': int(fields[19]), 'cpu_ticks': int(fields[11]) + int(fields[12]),
                'rss_kib': int(status.get('VmRSS', '0 kB').split()[0]), 'pss_kib': None,
                'context_switches': sum(int(status.get(key, '0')) for key in
                                        ('voluntary_ctxt_switches', 'nonvoluntary_ctxt_switches')),
            }
        except (OSError, ValueError, IndexError):
            continue  # A process can exit between any two reads.
    # Read expensive mapping data only for the selected desktop, and exclude
    # this observer so it cannot inflate the shell's RAM/CPU measurements.
    for pid in descendants(result, roots) - {os.getpid()}:
        try:
            for line in Path(f'/proc/{pid}/smaps_rollup').read_text().splitlines():
                if line.startswith('Pss:'):
                    result[pid]['pss_kib'] = int(line.split()[1])
                    break
        except (OSError, ValueError):
            pass
    result.pop(os.getpid(), None)
    return result


def descendants(table, roots):
    selected = set(roots) & table.keys()
    while True:
        extra = {pid for pid, row in table.items() if row['ppid'] in selected} - selected
        if not extra:
            return selected
        selected.update(extra)


def sample(table, previous, elapsed, shell_pid, compositor_pid, service_pids):
    shell = descendants(table, [shell_pid])
    stack = shell | descendants(table, [compositor_pid] if compositor_pid else []) | descendants(table, service_pids)
    rows = []
    ticks = os.sysconf('SC_CLK_TCK')
    for pid in sorted(stack):
        row = table[pid].copy()
        old = previous.get(pid)
        comparable = old is not None and old['start_ticks'] == row['start_ticks'] and elapsed > 0
        row['cpu_percent'] = round(100 * (row['cpu_ticks'] - old['cpu_ticks']) / ticks / elapsed, 3) if comparable else None
        row['context_switches_per_second'] = round((row['context_switches'] - old['context_switches']) / elapsed, 3) if comparable else None
        row['group'] = 'shell' if pid in shell else 'desktop_other'
        rows.append(row)
    def total(items):
        missing = sum(r['pss_kib'] is None for r in items)
        return {'processes': len(items), 'rss_kib': sum(r['rss_kib'] for r in items),
                'pss_kib': None if missing else sum(r['pss_kib'] for r in items),
                'pss_unreadable_processes': missing,
                'cpu_percent': sum(r['cpu_percent'] or 0 for r in items) if elapsed else None,
                'cpu_unmeasured_processes': sum(r['cpu_percent'] is None for r in items)}
    return {'timestamp_unix': time.time(), 'interval_seconds': elapsed,
            'shell': total([r for r in rows if r['group'] == 'shell']),
            'desktop_stack': total(rows), 'processes': rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--shell-pid', type=int, required=True)
    parser.add_argument('--compositor-pid', type=int)
    parser.add_argument('--service-pid', type=int, action='append', default=[])
    parser.add_argument('--samples', type=int, default=30)
    parser.add_argument('--interval', type=float, default=1)
    args = parser.parse_args()
    if args.samples < 1 or args.interval <= 0:
        parser.error('samples and interval must be positive')
    previous, timestamp = {}, time.monotonic()
    for index in range(args.samples):
        if index:
            time.sleep(args.interval)
        now = time.monotonic()
        table = processes([args.shell_pid, *args.service_pid, *([args.compositor_pid] if args.compositor_pid else [])])
        if args.shell_pid not in table:
            raise SystemExit('Selected shell process exited or cannot be read')
        print(json.dumps(sample(table, previous, now - timestamp if index else 0,
                                args.shell_pid, args.compositor_pid, args.service_pid)), flush=True)
        previous, timestamp = table, now


if __name__ == '__main__':
    main()
