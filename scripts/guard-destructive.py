#!/usr/bin/env python3
"""PreToolUse guard for Bash commands.

Blocks only operations that destroy work with no way back. Everything else —
ordinary pushes, tag pushes, branch renames, soft resets, deleting build
artifacts — runs untouched, because the PR workflow and review are what keep the
repo honest.

Two lessons are baked in:

1. The original guard was `type: prompt`, asking a model to judge each command.
   That was non-deterministic: the same `git push` was allowed one hour and
   refused the next, and it blocked `git push origin <branch>` even though
   `permissions.allow` grants exactly that.

2. Matching a bare substring is not enough either. A command that merely
   *mentions* `git reset --hard` — writing documentation about it, or editing
   this file — is not running it. Patterns must therefore match at a command
   position: the start of the command, or just after a separator.

Contract: exit 0 to allow, exit 2 to block with the reason on stderr.
"""
import json
import re
import sys

# Start of string, or immediately after a shell command separator.
START = r"(?:^|[;&|\n(]|&&|\|\|)\s*(?:sudo\s+)?"

RULES = [
    (
        re.compile(START + r"git\s+push\b[^;&|\n]*(?:--force\b|(?<=\s)-f\b)"),
        "force push (rewrites published history)",
    ),
    (
        re.compile(START + r"git\s+reset\b[^;&|\n]*--hard\b"),
        "hard reset (discards uncommitted work)",
    ),
    (
        # Recursive delete aimed at a root or home path. Deleting build
        # artifacts under a relative path stays allowed.
        re.compile(START + r"rm\s+(?:-\w+\s+)*-\w*[rR]\w*\b[^;&|\n]*\s(?:/|~)(?:\s|/|$)"),
        "recursive delete of a root or home path",
    ),
]


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        return 0  # Never block because the guard itself failed to parse.

    command = payload.get("tool_input", {}).get("command", "")
    if not isinstance(command, str) or not command:
        return 0

    for pattern, reason in RULES:
        if pattern.search(command):
            print(
                f"Blocked: {reason}. If you intend this, run it yourself.",
                file=sys.stderr,
            )
            return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
