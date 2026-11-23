#!/usr/bin/env python3
"""Parse one `/oritech` reply line into shell assignments.

The lines are (see `src/support.rs`, the readout section):

    Basic Generator at [0, 100, 0]: 1940/50000 FE, burning 1503/1600 ticks, fuel minecraft:coal x4, t=324
    Pulverizer at [20, 100, 0]: 10000/25000 FE, progress 0 recipe none, input minecraft:bone x1, out0 empty, out1 empty, addons redstone redstone disabled, t=7
    Pulverizer at [1, 100, 0]: 3600/25000 FE, progress 9 recipe oritech:pulverizer/bone, input minecraft:bone x1, out0 empty, out1 empty, t=488
    Powered Furnace at [8, 100, 0]: 3200/10000 FE, assembled yes core oritech:machine_core_2 quality 2, progress 11 recipe …, input …, out …, t=424
    Industrial Chiller at [12, 100, 0]: 43600/50000 FE, assembled yes core oritech:machine_core_2 quality 2, progress 11 recipe oritech:cooler/ice, tank minecraft:water 1000/4000 mB, out empty, t=424
    Lava Generator at [20, 100, 4]: 3840/100000 FE, assembled yes core oritech:machine_core_2 quality 2, burning 60/120 ticks recipe oritech:lavagen/lava, tank minecraft:lava 100/4000 mB, t=424
    Assembler at [0, 100, 12]: 6144/50000 FE, assembled yes core oritech:machine_core_1+oritech:machine_core_2+oritech:machine_core_3 quality 2, progress 0 recipe oritech:assembler/motor, in0 oritech:nickel_ingot x1, in1 oritech:steel_ingot x2, in2 oritech:magnetic_coil x2, in3 empty, out0 empty, addons none, t=424
    Centrifuge at [8, 100, 10]: 3200/10000 FE, assembled yes core oritech:machine_core_2 quality 2, progress 0 recipe oritech:centrifuge/carbon, tank minecraft:water 250/8000 mB, input empty, out0 oritech:carbon_fibre_strands x1, out1 empty, addons none, t=424
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
    "IN0",
    "IN1",
    "IN2",
    "IN3",
    "ASSEMBLED",
    "CORE",
    "QUALITY",
    "TANK_FLUID",
    "TANK_AMOUNT",
    "TANK_CAP",
    "ASKED",
    "ACCEPTED",
    "NAME",
    "ADDONS",
    "BURST",
    "BURST_MAX",
    "REDSTONE",
    "T",
)

# Slice 5: every upgradable machine's line ends `addons <names>` and, only when a
# burst addon sets a maximum, `burst <remaining>/<max>`. Slice 6 adds
# `redstone <enabled|disabled>` after it, only while a Control Unit Addon sits in
# the machine's slot.
ADDON_FIELD = (
    r"addons (?P<ADDONS>[^\s,]+?)"
    r"(?: burst (?P<BURST>-?\d+)/(?P<BURST_MAX>\d+))?"
    r"(?: redstone (?P<REDSTONE>disabled|enabled))?, t=(?P<T>\d+)$"
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
    r"input (?P<INPUT>.+?), out0 (?P<OUT0>.+?), out1 (?P<OUT1>.+?), " + ADDON_FIELD
)

FURNACE = re.compile(
    r"^Powered Furnace at \[(?P<POS>[^\]]+)\]: "
    r"(?P<ENERGY>\d+)/(?P<CAPACITY>\d+) FE, "
    r"assembled (?P<ASSEMBLED>yes|no) core (?P<CORE>.+?) quality (?P<QUALITY>-?\d+), "
    r"progress (?P<PROGRESS>\d+) recipe (?P<RECIPE>.+?), "
    r"input (?P<INPUT>.+?), out (?P<OUT0>.+?), " + ADDON_FIELD
)

COOLER = re.compile(
    r"^Industrial Chiller at \[(?P<POS>[^\]]+)\]: "
    r"(?P<ENERGY>\d+)/(?P<CAPACITY>\d+) FE, "
    r"assembled (?P<ASSEMBLED>yes|no) core (?P<CORE>.+?) quality (?P<QUALITY>-?\d+), "
    r"progress (?P<PROGRESS>\d+) recipe (?P<RECIPE>.+?), "
    r"tank (?P<TANK_FLUID>\S+) (?P<TANK_AMOUNT>-?\d+)/(?P<TANK_CAP>\d+) mB, "
    r"out (?P<OUT0>.+?), " + ADDON_FIELD
)

FLUID = re.compile(
    r"^(?P<NAME>\S+) at \[(?P<POS>[^\]]+)\]: "
    r"tank (?P<TANK_FLUID>\S+) (?P<TANK_AMOUNT>-?\d+)/(?P<TANK_CAP>\d+) mB "
    r"\(asked (?P<ASKED>-?\d+), accepted (?P<ACCEPTED>-?\d+)\)$"
)

LAVA = re.compile(
    r"^Lava Generator at \[(?P<POS>[^\]]+)\]: "
    r"(?P<ENERGY>\d+)/(?P<CAPACITY>\d+) FE, "
    r"assembled (?P<ASSEMBLED>yes|no) core (?P<CORE>.+?) quality (?P<QUALITY>-?\d+), "
    r"burning (?P<BURN>-?\d+)/(?P<MAXBURN>\d+) ticks recipe (?P<RECIPE>.+?), "
    r"tank (?P<TANK_FLUID>\S+) (?P<TANK_AMOUNT>-?\d+)/(?P<TANK_CAP>\d+) mB, " + ADDON_FIELD
)

# Slice 8: the assembler's line. Its `core` field is the three cores' ids joined
# with `+` (the group is non-greedy, so the ` quality ` that follows is the end),
# and its four input slots are `in0`–`in3`.
ASSEMBLER = re.compile(
    r"^Assembler at \[(?P<POS>[^\]]+)\]: "
    r"(?P<ENERGY>\d+)/(?P<CAPACITY>\d+) FE, "
    r"assembled (?P<ASSEMBLED>yes|no) core (?P<CORE>.+?) quality (?P<QUALITY>-?\d+), "
    r"progress (?P<PROGRESS>\d+) recipe (?P<RECIPE>.+?), "
    r"in0 (?P<IN0>.+?), in1 (?P<IN1>.+?), in2 (?P<IN2>.+?), in3 (?P<IN3>.+?), "
    r"out0 (?P<OUT0>.+?), " + ADDON_FIELD
)

# Slice 9: the centrifuge's line carries the assembly, its one tank, its one input
# and its two output slots (the pulverizer's field names, so `machine_holds` works).
CENTRIFUGE = re.compile(
    r"^Centrifuge at \[(?P<POS>[^\]]+)\]: "
    r"(?P<ENERGY>\d+)/(?P<CAPACITY>\d+) FE, "
    r"assembled (?P<ASSEMBLED>yes|no) core (?P<CORE>.+?) quality (?P<QUALITY>-?\d+), "
    r"progress (?P<PROGRESS>\d+) recipe (?P<RECIPE>.+?), "
    r"tank (?P<TANK_FLUID>\S+) (?P<TANK_AMOUNT>-?\d+)/(?P<TANK_CAP>\d+) mB, "
    r"input (?P<INPUT>.+?), out0 (?P<OUT0>.+?), out1 (?P<OUT1>.+?), " + ADDON_FIELD
)


def main(argv: list[str]) -> int:
    line = " ".join(argv).strip()
    for kind, pattern in (
        ("generator", GENERATOR),
        ("pulverizer", PULVERIZER),
        ("furnace", FURNACE),
        ("cooler", COOLER),
        ("lava", LAVA),
        ("assembler", ASSEMBLER),
        ("centrifuge", CENTRIFUGE),
        ("fluid", FLUID),
    ):
        match = pattern.match(line)
        if not match:
            continue
        out = {"ok": 1, "KIND": kind}
        out.update(
            {key: value for key, value in match.groupdict().items() if value is not None}
        )
        if kind == "generator":
            out.setdefault("INPUT", out["FUEL"])
        if kind == "assembler":
            # The four input slots are one field each; `INPUT` is the first one,
            # so `machine_holds` can look at the machine like the others.
            out.setdefault("INPUT", out["IN0"])
        for field in FIELDS:
            out.setdefault(field, "")
        for key, value in out.items():
            print(f"{key}={shlex.quote(str(value))}")
        return 0
    print("ok=0")
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
