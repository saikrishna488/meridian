#!/usr/bin/env python3
"""Test the actual native session launcher, including its labwc startup command."""
import os
from pathlib import Path
import signal
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
SESSION = ROOT / 'tools/dev-native-session.sh'


def stop(process):
    if process.poll() is None:
        os.killpg(process.pid, signal.SIGTERM)
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()


with tempfile.TemporaryDirectory(prefix='marsos-session-startup-') as temporary:
    base = Path(temporary)
    runtime = base / 'runtime'
    runtime.mkdir(mode=0o700)
    config = base / 'config'
    config.mkdir()
    env = dict(os.environ, XDG_RUNTIME_DIR=str(runtime), XDG_CONFIG_HOME=str(config),
               WLR_BACKENDS='headless', WLR_HEADLESS_OUTPUTS='1', WLR_RENDERER='pixman',
               WLR_LIBINPUT_NO_DEVICES='1', GDK_BACKEND='wayland', GSK_RENDERER='cairo', GTK_A11Y='none', GVFS_DISABLE_FUSE='1',
               MERIDIAN_NATIVE_SESSION_LOG=str(base / 'native.log'))
    for name in ('WAYLAND_DISPLAY', 'DISPLAY', 'MERIDIAN_NATIVE_PRIVATE_BUS', 'MERIDIAN_NATIVE_RUNNER', 'MERIDIAN_NATIVE_BINARY'):
        env.pop(name, None)

    # Check failures must happen before a compositor is opened.
    missing = subprocess.run(['bash', str(SESSION)], env=dict(env, MERIDIAN_NATIVE_BINARY=str(base / 'absent')),
                             capture_output=True, text=True, timeout=10)
    assert missing.returncode != 0 and 'Build first:' in missing.stderr
    assert not list(runtime.glob('wayland-*')), 'Failed preflight started a compositor'

    # Exercise labwc's argv parsing with a real executable path containing spaces.
    marker = base / 'started'
    client = base / 'client with spaces.sh'
    client.write_text('#!/bin/sh\n[ "$1" = --check ] && exit 0\n'
                      '[ "$1" = --session ] || exit 2\n'
                      f'printf "%s\\n" "$WAYLAND_DISPLAY" "$DBUS_SESSION_BUS_ADDRESS" > "{marker}"\n'
                      'echo fixture-started\n')
    client.chmod(0o755)
    fixture = subprocess.run(['bash', str(SESSION)], env=dict(env, MERIDIAN_NATIVE_RUNNER=str(client)),
                             capture_output=True, text=True, timeout=15)
    assert fixture.returncode == 0, fixture.stdout + fixture.stderr
    assert marker.exists() and 'fixture-started' in fixture.stdout, 'labwc did not execute its session client'
    address = marker.read_text().splitlines()
    assert address[0].startswith('wayland-')
    assert address[1] != os.environ.get('DBUS_SESSION_BUS_ADDRESS'), 'Nested session reused the parent application bus'

    # Test the production launch path rather than starting the binary ourselves.
    capture = base / 'visible.ppm'
    with (base / 'launcher.log').open('w+') as log:
        process = subprocess.Popen(['bash', str(SESSION)], env=env, stdout=log, stderr=log, start_new_session=True)
        try:
            deadline = time.monotonic() + 20
            visible = False
            while time.monotonic() < deadline and process.poll() is None:
                sockets = [path for path in runtime.glob('wayland-*') if path.is_socket()]
                if sockets:
                    probe = dict(env, WAYLAND_DISPLAY=sockets[0].name)
                    shot = subprocess.run(['grim', '-t', 'ppm', str(capture)], env=probe,
                                          capture_output=True, timeout=3)
                    if shot.returncode == 0:
                        with capture.open('rb') as frame:
                            assert frame.readline().strip() == b'P6'
                            frame.readline()  # dimensions
                            frame.readline()  # max channel value
                            visible = any(frame.read())
                        if visible:
                            break
                time.sleep(.2)
            log.seek(0)
            assert visible, 'Native session stayed black or exited:\n' + log.read()
        finally:
            stop(process)
            # Private portal services may leave FUSE mountpoints after their bus
            # exits. Unmount only this test's runtime paths before temp cleanup.
            if shutil.which('fusermount3'):
                for name in ('doc', 'gvfs'):
                    subprocess.run(['fusermount3', '-u', '-z', str(runtime / name)],
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=5)
print('Session preflight, quoted startup, private bus, client exit and visible Swift frame passed')
