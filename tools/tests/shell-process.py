#!/usr/bin/env python3
"""Crash the supervised shell; verify a separate Wayland application survives."""
import os
import pathlib
import signal
import subprocess
import tempfile
import time


def wait_for(predicate, label):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(0.05)
    raise RuntimeError(f"Timed out: {label}")


def shell_child(supervisor):
    try:
        children = pathlib.Path(f"/proc/{supervisor.pid}/task/{supervisor.pid}/children").read_text().split()
        for child in children:
            try:
                if pathlib.Path(f"/proc/{child}/exe").resolve().name == "meridian-shell":
                    return int(child)
            except FileNotFoundError:
                pass
    except FileNotFoundError:
        pass
    return None


root = pathlib.Path(os.environ["MERIDIAN_TEST_ROOT"])
with tempfile.TemporaryDirectory(prefix="meridian-process-check-") as temp:
    folder = pathlib.Path(temp)
    with (folder / "shell.log").open("w+") as output:
        env = {**os.environ, "MERIDIAN_SHELL": str(root / "target/debug/meridian-shell")}
        supervisor = subprocess.Popen(["sh", str(root / "session/shell-supervisor")], env=env, stdout=output, stderr=output)
        client = subprocess.Popen([os.environ["MERIDIAN_TEST_CLIENT"], str(folder / "ready")], stdout=output, stderr=output)
        try:
            wait_for(lambda: (folder / "ready").exists(), "independent Wayland window")
            old = wait_for(lambda: shell_child(supervisor), "initial shell process")
            wait_for(lambda: "meridian shell started" in (folder / "shell.log").read_text(), "initial shell startup")
            os.kill(old, signal.SIGKILL)
            replacement = wait_for(lambda: (pid if (pid := shell_child(supervisor)) and pid != old else None), "replacement shell process")
            wait_for(lambda: (folder / "shell.log").read_text().count("meridian shell started") >= 2, "replacement shell startup")
            assert client.poll() is None, "independent application died with the shell"
            assert supervisor.poll() is None, "shell supervisor died"
            print(f"phase1_process_smoke shell_before={old} shell_after={replacement} independent_app_survived=true", flush=True)
        except BaseException:
            print((folder / "shell.log").read_text(), flush=True)
            raise
        finally:
            supervisor.terminate()
            client.terminate()
            for process in [supervisor, client]:
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
