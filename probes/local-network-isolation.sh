#!/bin/sh
# Compare an ordinary client and a client wrapped in the platform primitive.
set -eu
case "$(uname -s)" in
    Linux)
        bwrap --version
        set -- bwrap --unshare-net --ro-bind / / --
        ;;
    Darwin)
        set -- /usr/bin/sandbox-exec -p '(version 1)(allow default)(deny network*)'
        ;;
    *)
        echo "No isolation probe for $(uname -s)" >&2
        exit 2
        ;;
esac
python3 -I - "$@" <<'PY'
import socket
import subprocess
import sys

client = """
import socket
import sys

try:
    with socket.create_connection(("127.0.0.1", int(sys.argv[1])), timeout=2):
        print("client: connected")
except OSError as error:
    print(f"client: blocked (errno={error.errno})")
"""
with socket.socket() as listener:
    listener.bind(("127.0.0.1", 0))
    listener.listen()
    command = [sys.executable, "-I", "-c", client, str(listener.getsockname()[1])]
    control = subprocess.check_output(command, text=True).strip()
    print(f"control {control}", flush=True)
    assert control == "client: connected"
    wrapped = subprocess.run([*sys.argv[1:], *command], text=True, stdout=subprocess.PIPE)
    if wrapped.returncode:
        print(f"wrapped client did not complete (exit {wrapped.returncode})")
        sys.exit(2)
    print(f"wrapped {wrapped.stdout.strip()}")
    assert wrapped.stdout.startswith("client: blocked")
PY
