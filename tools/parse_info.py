#!/usr/bin/env python3
"""Parse one `/oritech info` line into shell assignments.

The line is one of (see `mixins/oritech/support.rs`, `readout`):

    Basic Generator at [0, 100, 0]: 1940/50000 FE, burning 1503/1600 ticks, fuel minecraft:coal x4, t=324
    Pulverizer at [1, 100, 0]: 3600/25000 FE, progress 9 recipe oritech:pulverizer/bone, input minecraft:bone x1, out0 empty, out1 empty, t=488

Usage: `eval "$(python3 tools/parse_info.py "$line")"`; `ok=1` when it parsed.
"""
from __future__ import annotations

import re
import shlex
import sys

GENERATOR = re.compile(
    r"^Basic Generator at \[(?P<POS>[^\]]+)\]: "
    r"(?P<ENERGY>\d+)/(?P<CAPACITY>\d+) FE, "
    r"burning (?P<BURN>-?\d+)/(?P<MAXBURN>\d+) ticks, "
    r"fuel (?P<FUEL>.+?), t=(?P<T>\d+)$"
)

PULVERIZER = re.compile(
    r"^Pulverizer at \[(?P<POS>[^\]]+)\]: "
    r"(?P<ENERGY>\d+)/(?P<CAPACITY>\d+) FE, "
    r"progress (?P<PROGRESS>\d+) recipe (?P<RECIPE>.+?), "
    r"input (?P<INPUT>.+?), out0 (?P<OUT0>.+?), out1 (?P<OUT1>.+?), t=(?P<T>\d+)$"
)

FURNACE = re.compile(
    r"^Powered Furnace at \[(?P<POS>[^\]]+)\]: "
    r"(?P<ENERGY>\d+)/(?P<CAPACITY>\d+) FE, "
    r"assembled (?P<ASSEMBLED>yes|no) core (?P<CORE>.+?) quality (?P<QUALITY>-?\d+), "
    r"progress (?P<PROGRESS>\d+) recipe (?P<RECIPE>.+?), "
    r"input (?P<INPUT>.+?), out (?P<OUT0>.+?), t=(?P<T>\d+)$"
)


def main(argv: list[str]) -> int:
    line = " ".join(argv).strip()
    for kind, pattern in (
        ("generator", GENERATOR),
        ("pulverizer", PULVERIZER),
        ("furnace", FURNACE),
    ):
        match = pattern.match(line)
        if not match:
            continue
        fields = {key: value for key, value in match.groupdict().items()}
        out = {"ok": 1, "KIND": kind}
        for key, value in fields.items():
            out[key] = value
        if kind == "generator":
            out.setdefault("PROGRESS", "")
            out.setdefault("RECIPE", "")
            out.setdefault("INPUT", fields["FUEL"])
            out.setdefault("OUT0", "")
            out.setdefault("OUT1", "")
            out.setdefault("ASSEMBLED", "")
            out.setdefault("CORE", "")
            out.setdefault("QUALITY", "")
        elif kind == "pulverizer":
            out.setdefault("BURN", "")
            out.setdefault("MAXBURN", "")
            out.setdefault("FUEL", "")
            out.setdefault("ASSEMBLED", "")
            out.setdefault("CORE", "")
            out.setdefault("QUALITY", "")
        else:
            out.setdefault("BURN", "")
            out.setdefault("MAXBURN", "")
            out.setdefault("FUEL", "")
            out.setdefault("OUT1", "")
        for key, value in out.items():
            print(f"{key}={shlex.quote(str(value))}")
        return 0
    print("ok=0")
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
