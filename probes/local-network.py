#!/usr/bin/env python3
"""V27 fixture. Never contacts external hosts or real metadata services."""

import ipaddress
import platform
import socket
from http.server import BaseHTTPRequestHandler, HTTPServer
from threading import Thread

RANGES = tuple(map(ipaddress.ip_network, (
    "127.0.0.0/8", "::1/128", "10.0.0.0/8", "172.16.0.0/12",
    "192.168.0.0/16", "fc00::/7", "169.254.0.0/16", "fe80::/10",
)))
NAME = "permitted.example"
CASES = {
    "loopback-v4": "127.1.2.3", "loopback-v6": "::1",
    "private-10": "10.1.2.3", "private-172": "172.31.1.2",
    "private-192": "192.168.1.2", "private-v6": "fd00::1",
    "link-local-v4": "169.254.1.2", "link-local-v6": "fe80::1",
    "metadata": "169.254.169.254", "mapped-private": "::ffff:10.1.2.3",
    "mapped-loopback": "::ffff:7f00:1", "mapped-metadata": "::ffff:a9fe:a9fe",
}


def refused(address):
    address = ipaddress.ip_address(address)
    if isinstance(address, ipaddress.IPv6Address) and address.ipv4_mapped:
        address = address.ipv4_mapped
    return any(address in network for network in RANGES)


def reject_entry(name):
    canonical = name.lower().rstrip(".")
    if canonical == "localhost" or canonical.endswith(".localhost"):
        return True
    try:
        return refused(canonical)
    except ValueError:
        return False  # Fixture checks entries after syntax validation.


def broker(name, resolve, dial, *, allow_local):
    if name != NAME:
        return "name-denied"
    for address in resolve(name):
        if allow_local or not refused(address):
            return dial(address)
    return "address-denied"


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_GET(self):
        self.server.hits += 1
        self.send_response(204)
        self.end_headers()


def main():
    print(f"platform: {platform.system()} {platform.machine()}, Python {platform.python_version()}")
    for name in ("localhost", "X.LocalHost.", *CASES.values()):
        assert reject_entry(name), name
    assert not reject_entry(NAME)
    print(f"entry check: 14 obvious entries denied; {NAME} accepted")

    def record_dial(address):
        return f"dial {address}"

    for label, address in CASES.items():
        result = broker(NAME, lambda _: [address], record_dial, allow_local=False)
        assert result == "address-denied", (label, result)
        print(f"fixture DNS {label} {address}: {result}")

    public = "93.184.216.34"  # Recorded dial only.
    answers = iter(([CASES["metadata"], public], [CASES["mapped-loopback"]]))

    def resolve(name):
        assert name == NAME
        return next(answers)

    first = broker(NAME, resolve, record_dial, allow_local=False)
    second = broker(NAME, resolve, record_dial, allow_local=False)
    assert (first, second) == (f"dial {public}", "address-denied")
    assert next(answers, None) is None
    print(f"mixed answer then changed answer: {first}; {second}")

    with HTTPServer(("127.0.0.1", 0), Handler) as server:
        server.hits = 0
        thread = Thread(target=server.serve_forever)
        thread.start()
        try:
            def dial_loopback(address):
                assert address == "127.0.0.1"
                with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
                    sock.settimeout(2)
                    sock.connect((address, server.server_port))
                    sock.sendall(b"GET / HTTP/1.0\r\n\r\n")
                    with sock.makefile("rb") as reply:
                        return reply.readline().decode().split()[1]

            def local_answer(_):
                return ["127.0.0.1"]

            denied = broker(NAME, local_answer, dial_loopback, allow_local=False)
            assert denied == "address-denied" and server.hits == 0
            allowed = broker(NAME, local_answer, dial_loopback, allow_local=True)
            assert allowed == "204" and server.hits == 1
            direct = dial_loopback("127.0.0.1")
            assert direct == "204" and server.hits == 2
            print(f"loopback fixture: denied={denied} allowed=HTTP {allowed} direct=HTTP {direct} hits={server.hits}")
        finally:
            server.shutdown()
            thread.join()


if __name__ == "__main__":
    main()
