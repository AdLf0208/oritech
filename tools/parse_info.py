#!/usr/bin/env python3
"""Parse one `/oritech` reply line into shell assignments.

The lines are (see `src/support.rs`, the readout section):

    Basic Generator at [0, 100, 0]: 1940/50000 FE, burning 1503/1600 ticks, fuel minecraft:coal x4, t=324
    Pulverizer at [1, 100, 0]: 3600/25000 FE, progress 9 recipe oritech:pulverizer/bone, input minecraft:bone x1, out0 empty, out1 empty, t=488
    Powered Furnace at [8, 100, 0]: 3200/10000 FE, assembled yes core oritech:machine_core_2 quality 2, progress 11 recipe …, input …, out …, t=424
    Industrial Chiller at [12, 100, 0]: 43600/50000 FE, assembled yes core oritech:machine_core_2 quality 2, progress 11 recipe oritech:cooler/ice, tank minecraft:water 1000/4000 mB, out empty, t=424
    oritech:cooler_block at [12, 100, 0]: tank minecraft:water 1000/4000 mB (asked 1000, accepted 1000)

Usage: `eval "$(python3 tools/parse_info.py "$line")"`; `ok=1` when it parsed.
Every field name is always defined (empty when the line does not carry it), so a
`set -u` scenario can `expect` any of them.
"""
from __future__ import annotations

import re
import shlex
import sys

FIELDS = (
    "POS",
    "ENERGY",
    "CAPACITY",
    "BURN",
    "MAXBURN",
    "FUEL",
    "PROGRESS",
    "RECIPE",
    "INPUT",
    "OUT0",
    "OUT1",
    "ASSEMBLED",
    "CORE",
    "QUALITY",
    "TANK_FLUID",
    "TANK_AMOUNT",
    "TANK_CAP",
    "ASKED",
    "ACCEPTED",
    "NAME",
    "T",
)

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

COOLER = re.compile(
    r"^Industrial Chiller at \[(?P<POS>[^\]]+)\]: "
    r"(?P<ENERGY>\d+)/(?P<CAPACITY>\d+) FE, "
    r"assembled (?P<ASSEMBLED>yes|no) core (?P<CORE>.+?) quality (?P<QUALITY>-?\d+), "
    r"progress (?P<PROGRESS>\d+) recipe (?P<RECIPE>.+?), "
    r"tank (?P<TANK_FLUID>\S+) (?P<TANK_AMOUNT>-?\d+)/(?P<TANK_CAP>\d+) mB, "
    r"out (?P<OUT0>.+?), t=(?P<T>\d+)$"
)

FLUID = re.compile(
    r"^(?P<NAME>\S+) at \[(?P<POS>[^\]]+)\]: "
    r"tank (?P<TANK_FLUID>\S+) (?P<TANK_AMOUNT>-?\d+)/(?P<TANK_CAP>\d+) mB "
    r"\(asked (?P<ASKED>-?\d+), accepted (?P<ACCEPTED>-?\d+)\)$"
)


def main(argv: list[str]) -> int:
    line = " ".join(argv).strip()
    for kind, pattern in (
        ("generator", GENERATOR),
        ("pulverizer", PULVERIZER),
        ("furnace", FURNACE),
        ("cooler", COOLER),
        ("fluid", FLUID),
    ):
        match = pattern.match(line)
        if not match:
            continue
        out = {"ok": 1, "KIND": kind}
        out.update({key: value for key, value in match.groupdict().items()})
        if kind == "generator":
            out.setdefault("INPUT", out["FUEL"])
        for field in FIELDS:
            out.setdefault(field, "")
        for key, value in out.items():
            print(f"{key}={shlex.quote(str(value))}")
        return 0
    print("ok=0")
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
