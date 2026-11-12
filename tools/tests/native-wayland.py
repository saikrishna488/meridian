#!/usr/bin/env python3
"""Test only the new C++ surfaces inside a private headless Wayland session."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--launcher", type=Path, help="also exercise a compiled Swift launcher")
parser.add_argument("--wtype", default="wtype", help="virtual keyboard command for launcher checks")
parser.add_argument("--screenshot", type=Path, help="optional launcher screenshot via grim")
arguments = parser.parse_args()
if arguments.launcher and not shutil.which(arguments.wtype):
    raise SystemExit("Launcher checks require wtype (or --wtype /path/to/wtype)")
for tool in ("labwc", "dbus-run-session"):
    if not shutil.which(tool):
        raise SystemExit(f"Required tool missing: {tool}")

if os.environ.get("MERIDIAN_NATIVE_TEST_PRIVATE_BUS") != "1":
    private_environment = dict(os.environ, MERIDIAN_NATIVE_TEST_PRIVATE_BUS="1")
    result = subprocess.run(
        ["dbus-run-session", "--", sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]],
        env=private_environment, capture_output=True, text=True,
    )
    print(result.stdout, end="")
    if result.returncode:
        print(result.stderr, end="", file=sys.stderr)
    raise SystemExit(result.returncode)

with tempfile.TemporaryDirectory(prefix="meridian-native-wayland-") as temporary:
    fixture = Path(temporary)
    runtime = fixture / "runtime"
    runtime.mkdir(mode=0o700)
    config = fixture / "config"
    config.mkdir()
    (config / "rc.xml").write_text("<labwc_config/>\n")
    environment = dict(os.environ)
    environment.pop("DISPLAY", None)
    environment.pop("WAYLAND_DISPLAY", None)
    environment.update(
        XDG_RUNTIME_DIR=str(runtime), XDG_CONFIG_HOME=str(config),
        WLR_BACKENDS="headless", WLR_RENDERER="pixman", WLR_HEADLESS_OUTPUTS="2",
        WLR_LIBINPUT_NO_DEVICES="1", GSK_RENDERER="cairo", GDK_BACKEND="wayland",
        GTK_A11Y="none", G_DEBUG="fatal-criticals", MERIDIAN_NATIVE_WAYLAND_TEST="1",
    )
    with (fixture / "compositor.log").open("w+") as log:
        compositor = subprocess.Popen(
            ["labwc", "-C", str(config)],
            env=environment, stdout=log, stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        try:
            deadline = time.monotonic() + 10
            sockets = []
            while time.monotonic() < deadline and compositor.poll() is None:
                sockets = [path for path in runtime.glob("wayland-*") if path.is_socket()]
                if sockets:
                    break
                time.sleep(0.05)
            if not sockets:
                log.seek(0)
                raise RuntimeError(f"Headless labwc failed to start:\n{log.read()}")
            environment["WAYLAND_DISPLAY"] = sockets[0].name
            subprocess.run([str(ROOT / "target/native/surface-test")],
                           env=environment, timeout=20, check=True)
            if arguments.launcher:
                data = fixture / "data"
                apps = data / "applications"
                apps.mkdir(parents=True)
                marker = fixture / "launched"
                (apps / "fixture.desktop").write_text(
                    "[Desktop Entry]\nType=Application\nName=MarsOS Fixture\n"
                    f"Exec=/usr/bin/touch {marker}\n"
                )
                environment.update(XDG_DATA_HOME=str(data), XDG_DATA_DIRS=str(data))
                with (fixture / "launcher.log").open("w+") as launcher_log:
                    launcher = subprocess.Popen(
                        [str(arguments.launcher.resolve()), "--overlay"], env=environment,
                        stdout=launcher_log, stderr=subprocess.STDOUT,
                    )
                    try:
                        time.sleep(2)
                        if launcher.poll() is not None:
                            raise RuntimeError("Swift launcher exited before input")
                        subprocess.run([arguments.wtype, "-s", "200", "-d", "50", "marsos fixture"],
                                       env=environment, timeout=5, check=True)
                        time.sleep(0.5)
                        if arguments.screenshot:
                            subprocess.run(["grim", str(arguments.screenshot.resolve())],
                                           env=environment, timeout=5, check=True)
                        subprocess.run([arguments.wtype, "-k", "Return"],
                                       env=environment, timeout=5, check=True)
                        deadline = time.monotonic() + 5
                        while not marker.exists() and time.monotonic() < deadline:
                            if launcher.poll() is not None:
                                raise RuntimeError("Swift launcher exited during input")
                            time.sleep(0.05)
                        if not marker.exists():
                            raise RuntimeError("Typing a query and Return did not launch the fixture")
                        refreshed = fixture / "refreshed"
                        (apps / "refreshed.desktop").write_text(
                            "[Desktop Entry]\nType=Application\nName=Refreshed Fixture\n"
                            f"Exec=/usr/bin/touch {refreshed}\n"
                        )
                        # Shift+Tab returns from search to Refresh. Activation restores
                        # search focus; this also exercises Swift state updates.
                        subprocess.run([arguments.wtype, "-M", "shift", "-k", "Tab", "-m", "shift", "-s", "300", "-k", "Return"],
                                       env=environment, timeout=5, check=True)
                        time.sleep(0.5)
                        subprocess.run([arguments.wtype, "-s", "300", "-M", "ctrl", "-k", "a", "-m", "ctrl",
                                        "-s", "200", "-d", "50", "refreshed fixture", "-s", "300", "-k", "Return"],
                                       env=environment, timeout=5, check=True)
                        deadline = time.monotonic() + 5
                        while not refreshed.exists() and time.monotonic() < deadline:
                            time.sleep(0.05)
                        if not refreshed.exists():
                            if arguments.screenshot:
                                subprocess.run(["grim", str(arguments.screenshot.with_name("native-refresh-failure.png").resolve())],
                                               env=environment, timeout=5, check=True)
                            raise RuntimeError("Refresh did not discover and launch the added application")
                        subprocess.run([arguments.wtype, "-k", "Escape"],
                                       env=environment, timeout=5, check=True)
                        if launcher.wait(timeout=5) != 0:
                            raise RuntimeError("Swift launcher did not close cleanly")
                        print("Swift overlay search, refresh, Return launch and Escape dismissal passed")
                    except Exception:
                        launcher_log.seek(0)
                        print(launcher_log.read())
                        raise
                    finally:
                        if launcher.poll() is None:
                            launcher.terminate()
                            launcher.wait(timeout=5)
        finally:
            # All processes in this group were created by this test.
            import signal
            try:
                os.killpg(compositor.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                compositor.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(compositor.pid, signal.SIGKILL)
                compositor.wait()
