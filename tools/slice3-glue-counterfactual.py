#!/usr/bin/env python3
"""Counterfactual edits for the slice-3 glue tests.

The glue's tests pin what it *declares* and the exact lines it prints. This
script breaks each pinned fact by one small mutation so the tests must go red;
`run` restores the file from `support.rs.bak` afterwards. Every mutation must be
found exactly once, so a stale pattern fails loudly instead of silently skipping.
"""
import sys
from pathlib import Path

SRC = Path("/home/adlf/Desktop/all_the_mods_10_rust_rewrite/ports/oritech/src/support.rs")

MUTATIONS = [
    # the cooler's container
    ("        COOLER => (COOLER_SLOTS, vec![], vec![COOLER_OUTPUT]),\n", ""),
    # the cooler's FE buffer
    ("        COOLER => (COOLER_CAPACITY, COOLER_MAX_INSERT, 0),\n", ""),
    # the tank's fillable side
    ("    tanks.inputs = vec![0];\n", "    tanks.inputs = Vec::new();\n"),
    # the per-machine core offset
    ("        COOLER => Some(COOLER_CORE_OFFSET),\n", ""),
    # the saved tank
    ("        tank: tank_state(nbt),\n", "        tank: None,\n"),
    # the tank amount clamp
    ("    let amount = amount.clamp(0, COOLER_TANK);\n", "    let amount = amount.max(0);\n"),
    ("        Some((fluid, amount)) if amount > 0 => (fluid, amount.clamp(0, COOLER_TANK)),\n",
     "        Some((fluid, amount)) if amount > 0 => (fluid, amount),\n"),
    # the chiller's readout energy
    ("        saved.machine.energy,\n        COOLER_CAPACITY,", "        0,\n        COOLER_CAPACITY,"),
    # the fluid command's accepted count
    ("(asked {asked}, accepted {accepted})", "(asked {asked}, accepted 0)"),
]


def mutate() -> None:
    text = SRC.read_text()
    for old, new in MUTATIONS:
        count = text.count(old)
        if count != 1:
            raise SystemExit(f"pattern found {count} times: {old!r}")
        text = text.replace(old, new)
    SRC.write_text(text)
    print(f"applied {len(MUTATIONS)} mutations")


def restore() -> None:
    backup = SRC.with_suffix(".rs.bak")
    SRC.write_text(backup.read_text())
    print("restored")


if __name__ == "__main__":
    {"mutate": mutate, "restore": restore}[sys.argv[1]]()
