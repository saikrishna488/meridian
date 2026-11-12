#!/usr/bin/env python3
"""Exercise Swift workspace input and session surfaces in private headless labwc."""
import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, default=ROOT / 'target/swift-ui/debug/meridian-native-applications')
parser.add_argument('--wtype', default='wtype')
parser.add_argument('--screenshots', type=Path)
args = parser.parse_args()
for tool in ('labwc', 'dbus-run-session', args.wtype):
    if not shutil.which(tool):
        raise SystemExit(f'Required tool missing: {tool}')
if os.environ.get('MERIDIAN_WORKSPACE_PRIVATE_BUS') != '1':
    result = subprocess.run(['dbus-run-session', '--', sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]],
                            env=dict(os.environ, MERIDIAN_WORKSPACE_PRIVATE_BUS='1'))
    raise SystemExit(result.returncode)

with tempfile.TemporaryDirectory(prefix='marsos-workspace-') as temporary:
    base = Path(temporary)
    runtime = base / 'runtime'
    runtime.mkdir(mode=0o700)
    config = base / 'config'
    config.mkdir()
    (config / 'rc.xml').write_text('<labwc_config/>\n')
    files = base / 'files'
    files.mkdir()
    (files / 'note.txt').write_text('Native workspace fixture\n')
    data = base / 'data'
    data.mkdir()
    env = dict(os.environ, XDG_RUNTIME_DIR=str(runtime), XDG_CONFIG_HOME=str(config),
               XDG_DATA_HOME=str(data), XDG_DATA_DIRS=str(data),
               MERIDIAN_NATIVE_DIRECTORY=str(files), WLR_BACKENDS='headless', WLR_RENDERER='pixman',
               WLR_HEADLESS_OUTPUTS='1', WLR_LIBINPUT_NO_DEVICES='1', GSK_RENDERER='cairo',
               GDK_BACKEND='wayland', GTK_A11Y='none', G_DEBUG='fatal-criticals', MERIDIAN_NATIVE_WAYLAND_TEST='1')
    env.pop('DISPLAY', None)
    env.pop('WAYLAND_DISPLAY', None)
    with (base / 'session.log').open('w+') as log:
        compositor = subprocess.Popen(['labwc', '-C', str(config)], env=env, stdout=log, stderr=log, start_new_session=True)
        app = None
        try:
            deadline = time.monotonic() + 10
            sockets = []
            while time.monotonic() < deadline and compositor.poll() is None:
                sockets = [p for p in runtime.glob('wayland-*') if p.is_socket()]
                if sockets:
                    break
                time.sleep(.05)
            if not sockets:
                raise RuntimeError('Private compositor did not start')
            env['WAYLAND_DISPLAY'] = sockets[0].name
            subprocess.run([str(ROOT / 'target/native/surface-test')], env=env, check=True, timeout=20)

            def start(session=False):
                command = [str(args.binary.resolve()), '--desktop']
                if session:
                    command.append('--session')
                process = subprocess.Popen(command, env=env, stdout=log, stderr=log)
                time.sleep(2)
                if process.poll() is not None:
                    raise RuntimeError('Swift workspace exited during startup')
                return process

            def keys(*actions):
                subprocess.run([args.wtype, *actions], env=env, check=True, timeout=10)
                time.sleep(.5)
                if app.poll() is not None:
                    raise RuntimeError('Swift workspace crashed during input')

            def tabs(count):
                actions = []
                for _ in range(count):
                    actions.extend(['-k', 'Tab', '-s', '100'])
                keys(*actions)

            def screenshot(name):
                if args.screenshots:
                    args.screenshots.mkdir(parents=True, exist_ok=True)
                    subprocess.run(['grim', '-o', 'HEADLESS-1', str((args.screenshots / name).resolve())],
                                   env=env, check=True, timeout=5)

            app = start()
            screenshot('applications.png')
            tabs(2)
            keys('-k', 'Return')  # Files navigation button
            screenshot('files.png')
            tabs(8)  # New folder name, after Settings/Home/Up/path/Go/hidden/Refresh
            keys('Created in Swift')
            tabs(1)
            keys('-k', 'Return')
            if not (files / 'Created in Swift').is_dir():
                raise RuntimeError('Swift Files did not create the requested folder')
            # Return from Create folder to Settings through the header controls.
            keys('-M', 'shift', *(['-k', 'Tab', '-s', '100'] * 8), '-m', 'shift', '-k', 'Return')  # Settings
            screenshot('settings.png')
            tabs(1)
            keys('-k', 'space')  # Include settings: true -> false
            tabs(1)
            keys('-k', 'space')  # Calculator: false -> true
            tabs(2)  # Reload, Save
            keys('-k', 'Return')
            screenshot('settings-saved.png')
            saved = json.loads((config / 'meridian/search.json').read_text())
            # Versioned stores wrap the payload.
            if 'data' in saved:
                saved = saved['data']
            if saved.get('settings') is not False or saved.get('calculator') is not True:
                raise RuntimeError(f'Swift Settings did not persist changes: {saved}')
            app.terminate()
            app.wait(timeout=5)
            app = start(session=True)
            screenshot('session.png')
            time.sleep(1)
            if app.poll() is not None:
                raise RuntimeError('Swift layer-shell session crashed')
            print('Swift folder creation, persisted preferences and desktop session startup passed')
        except Exception:
            log.seek(0)
            print(log.read(), file=sys.stderr)
            raise
        finally:
            if app and app.poll() is None:
                app.terminate()
                app.wait(timeout=5)
            try:
                os.killpg(compositor.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                compositor.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(compositor.pid, signal.SIGKILL)
                compositor.wait()
