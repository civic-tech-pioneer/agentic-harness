"""Test-only (S128, S131 P2.2): the stage D load test's rewrite of one stdout line of dev's fake claude.

Each `delta <n> ` or `entry <n> ` text gets a marker unique to this fake process, so the page's
items pair with their source write; an entry keeps its length (the marker takes the place of
padding), and the fixed `fake-session` id becomes this process's own (Watane binds a Claude session
id to its first owner).
"""

import re

ORIGINAL = b'"session_id":"fake-session"'
TEXT = re.compile(rb'"text":"(delta|entry) ([0-9]+) ')


def closing_quote(line: bytes, start: int) -> int:
    """The index of the first quote at or after `start` that no backslash escapes."""
    i = start
    while i < len(line):
        if line[i] == 0x22:
            return i
        i += _width(line, i)
    raise ValueError("no closing quote")


def _width(line: bytes, i: int) -> int:
    """The bytes of the JSON string character at `i`: 6 for a \\u escape, 2 for another escape, else 1."""
    if line[i] != 0x5C:
        return 1
    return 6 if line[i + 1 : i + 2] == b"u" else 2


def _boundary(line: bytes, start: int, at: int) -> int:
    """The last character boundary at or before `at`, scanning from `start`: a cut there splits no escape."""
    i = start
    while True:
        step = _width(line, i)
        if i + step > at:
            return i
        i += step


def rewrite(line: bytes, identity: bytes):
    """(the line to write, its marker or None)."""
    marker = None
    match = TEXT.search(line)
    if match:
        marker = b"cl-" + identity + b"-" + match[1] + b"-" + match[2]
        end = closing_quote(line, match.end())
        extra = len(marker) - (match.end(2) - match.start(1))
        suffix = _boundary(line, match.end(2), end - extra) if match[1] == b"entry" else end
        line = line[: match.start(1)] + marker + line[match.end(2) : suffix] + line[end:]
    return line.replace(ORIGINAL, b'"session_id":"' + identity + b'"'), marker
