#!/usr/bin/env python3
"""Fake greetd, for testing meridian-greeter without touching the real login.

Speaks greetd's IPC protocol (length-prefixed JSON on $GREETD_SOCK). Accepts
any username with the password "meridian", and only *prints* the session it
would start. Usage: fake-greetd.py <greeter command...>
"""
import json
import os
import socket
import struct
import subprocess
import sys
import tempfile
import threading

PASSWORD = "meridian"


def recv(conn):
    header = conn.recv(4, socket.MSG_WAITALL)
    if len(header) < 4:
        return None
    (length,) = struct.unpack("=I", header)
    return json.loads(conn.recv(length, socket.MSG_WAITALL))


def send(conn, message):
    body = json.dumps(message).encode()
    conn.sendall(struct.pack("=I", len(body)) + body)


def serve(conn):
    user = None
    with conn:
        while (request := recv(conn)) is not None:
            kind = request["type"]
            if kind == "create_session":
                user = request["username"]
                send(conn, {"type": "auth_message", "auth_message_type": "secret", "auth_message": "Password: "})
            elif kind == "post_auth_message_response":
                if request.get("response") == PASSWORD:
                    send(conn, {"type": "success"})
                else:
                    print(f"fake-greetd: wrong password for {user!r}", flush=True)
                    send(conn, {"type": "error", "error_type": "auth_error", "description": "pam_authenticate: AUTH_ERR"})
            elif kind == "start_session":
                print(f"fake-greetd: would start {request['cmd']} for {user!r} with {request['env']}", flush=True)
                send(conn, {"type": "success"})
            elif kind == "cancel_session":
                user = None
                send(conn, {"type": "success"})
            else:
                send(conn, {"type": "error", "error_type": "error", "description": f"unknown request {kind}"})


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    path = os.path.join(tempfile.mkdtemp(prefix="fake-greetd-"), "greetd.sock")
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(path)
    server.listen()

    def accept_loop():
        while True:
            conn, _ = server.accept()
            threading.Thread(target=serve, args=(conn,), daemon=True).start()

    threading.Thread(target=accept_loop, daemon=True).start()
    print(f"fake-greetd: listening on {path}; password is {PASSWORD!r}", flush=True)
    child = subprocess.run(sys.argv[1:], env={**os.environ, "GREETD_SOCK": path})
    sys.exit(child.returncode)


if __name__ == "__main__":
    main()
