#!/usr/bin/env python3
"""Exercise Swift workspace input and session surfaces in private headless labwc."""
import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import select
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, default=ROOT / 'target/swift-ui/debug/meridian-native-applications')
parser.add_argument('--wtype', default='wtype')
parser.add_argument('--screenshots', type=Path)
parser.add_argument('--chrome-only', action='store_true')
parser.add_argument('--pointer', type=Path, help='optional compiled native-pointer input fixture')
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
    (config / 'rc.xml').write_text('<labwc_config><keyboard><keybind key="A-F4"><action name="Close"/></keybind></keyboard></labwc_config>\n')
    files = base / 'files'
    files.mkdir()
    (files / 'note.txt').write_text('Native workspace fixture\n')
    data = base / 'data'
    data.mkdir()
    (data / "icons").symlink_to("/usr/share/icons", target_is_directory=True)
    applications = data / 'applications'
    applications.mkdir()
    launch_marker = base / 'dock-launched'
    (applications / 'fixture.desktop').write_text(
        '[Desktop Entry]\nType=Application\nName=Dock Fixture\n'
        f'Exec=/usr/bin/touch {launch_marker}\n'
    )
    env = dict(os.environ, XDG_RUNTIME_DIR=str(runtime), XDG_CONFIG_HOME=str(config),
               XDG_DATA_HOME=str(data), XDG_DATA_DIRS=str(data),
               MERIDIAN_NATIVE_DIRECTORY=str(files), MERIDIAN_NATIVE_DESKTOP_DIRECTORY=str(files), WLR_BACKENDS='headless', WLR_RENDERER='pixman',
               WLR_HEADLESS_OUTPUTS='1', WLR_LIBINPUT_NO_DEVICES='1', GSK_RENDERER='cairo',
               GDK_BACKEND='wayland', GTK_A11Y='none', G_DEBUG='fatal-criticals', MERIDIAN_NATIVE_WAYLAND_TEST='1')
    env.pop('DISPLAY', None)
    env.pop('WAYLAND_DISPLAY', None)
    with (base / 'session.log').open('w+') as log:
        compositor = subprocess.Popen(['labwc', '-C', str(config)], env=env, stdout=log, stderr=log, start_new_session=True)
        app = None
        pointer = None
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

            if not args.chrome_only:
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
            if args.pointer:
                pointer = subprocess.Popen([str(args.pointer.resolve())], env=env, stdin=subprocess.PIPE,
                                           stdout=subprocess.PIPE, stderr=log, text=True)
                def click(x, y, button):
                    pointer.stdin.write(f"{x} {y} {button}\n")
                    pointer.stdin.flush()
                    if not select.select([pointer.stdout], [], [], 5)[0] or pointer.stdout.readline().strip() != 'done':
                        raise RuntimeError('Private virtual pointer failed')
                    time.sleep(.3)
                click(1045, 232, 'left')  # Pin the isolated application fixture.
                screenshot('pinned-dock.png')
                app.terminate()
                app.wait(timeout=5)
                app = start(session=True)  # Pins must survive a runtime restart.
                keys('-M', 'alt', '-k', 'F4', '-m', 'alt')
                click(708, 674, 'left')
                screenshot('restored-dock.png')
                deadline = time.monotonic() + 3
                while not launch_marker.exists() and time.monotonic() < deadline:
                    time.sleep(.05)
                if not launch_marker.exists():
                    raise RuntimeError('Persisted dock pin did not launch the application fixture')
                screenshot('desktop.png')
                click(600, 300, 'right')
                time.sleep(.5)
                screenshot('desktop-menu.png')
                click(600, 347, 'left')
                time.sleep(.5)
                screenshot('new-folder.png')
                click(640, 345, 'left')
                keys('Desktop Menu Folder')
                screenshot('new-folder-filled.png')
                keys('-k', 'Return')
                screenshot('new-folder-result.png')
                if not (files / 'Desktop Menu Folder').is_dir():
                    raise RuntimeError('Desktop context menu did not create a folder')
                # Repeated popup/dismiss catches stale GTK ownership and grabs.
                click(650, 350, 'right')
                time.sleep(.5)
                keys('-k', 'Escape')
            print('Swift desktop, persistent dock launch and context-menu input passed' if args.chrome_only else
                  'Swift folder creation, persisted preferences and desktop session input passed')
        except Exception:
            log.seek(0)
            print(log.read(), file=sys.stderr)
            raise
        finally:
            if pointer:
                pointer.stdin.close()
                pointer.wait(timeout=5)
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
