#!/usr/bin/env python3
"""Counterfactual edits for the slice-9 glue tests (the Centrifuge).

The glue's tests pin the centrifuge's declarations, its saved compound, its line,
its place in the core-offset and addon-slot tables, and the Fluid Addon's switch.
This script breaks each pinned fact by one small mutation so the tests must go
red; `restore` puts the file back from `support.rs.bak`. Every mutation must be
found exactly once, so a stale pattern fails loudly instead of silently skipping.

    cp src/support.rs src/support.rs.bak
    python3 tools/slice9-glue-counterfactual.py mutate
    cargo test --offline centrifuge   # expect the slice-9 glue tests red
    python3 tools/slice9-glue-counterfactual.py restore
"""
import sys
from pathlib import Path

SRC = Path("/home/adlf/Desktop/all_the_mods_10_rust_rewrite/ports/oritech/src/support.rs")

MUTATIONS = [
    # no input slot on the centrifuge
    (
        "            vec![INPUT_SLOT],\n            vec![CENTRIFUGE_OUTPUT, CENTRIFUGE_OUTPUT + 1],\n",
        "            vec![],\n            vec![CENTRIFUGE_OUTPUT, CENTRIFUGE_OUTPUT + 1],\n",
    ),
    # the buffer's capacity is the pulverizer's
    (
        "        CENTRIFUGE => (CENTRIFUGE_CAPACITY, CENTRIFUGE_MAX_INSERT, 0),\n",
        "        CENTRIFUGE => (PULVERIZER_CAPACITY, CENTRIFUGE_MAX_INSERT, 0),\n",
    ),
    # the tank's capacity is the chiller's
    (
        "        CENTRIFUGE => Some(CENTRIFUGE_TANK),\n",
        "        CENTRIFUGE => Some(COOLER_TANK),\n",
    ),
    # the saved buffer is not read back
    (
        "        machine: Centrifuge {\n            energy: clamp_energy(read_int(nbt, KEY_ENERGY), capacity),\n",
        '        machine: Centrifuge {\n            energy: clamp_energy(read_int(nbt, "not_energy"), capacity),\n',
    ),
    # the line prints its two output slots the wrong way round
    (
        "        slot_text(out0),\n        slot_text(out1),\n",
        "        slot_text(out1),\n        slot_text(out0),\n",
    ),
    # the centrifuge is not in the core-offset table
    (
        "        CENTRIFUGE => Some(&[CENTRIFUGE_CORE_OFFSET]),\n",
        "        CENTRIFUGE => None,\n",
    ),
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
