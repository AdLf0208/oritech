#!/usr/bin/env python3
"""Counterfactual edits for the slice-10 glue tests (the Foundry).

The glue's tests pin the foundry's declarations, its saved compound, its line,
its place in the core-offset and addon-slot tables, and the four carried alloys.
This script breaks each pinned fact by one small mutation so the tests must go
red; `restore` puts the file back from `support.rs.bak`. Every mutation must be
found exactly once, so a stale pattern fails loudly instead of silently skipping.

    cp src/support.rs src/support.rs.bak
    python3 tools/slice10-glue-counterfactual.py mutate
    cargo test --offline foundry   # expect the slice-10 glue tests red
    python3 tools/slice10-glue-counterfactual.py restore
"""
import sys
from pathlib import Path

SRC = Path("/home/adlf/Desktop/all_the_mods_10_rust_rewrite/ports/oritech/src/support.rs")

MUTATIONS = [
    # the foundry's second input slot is gone (one input, not two)
    (
        "            (0..FOUNDRY_INPUTS).collect(),\n            vec![FOUNDRY_OUTPUT],\n",
        "            vec![0],\n            vec![FOUNDRY_OUTPUT],\n",
    ),
    # the buffer's capacity is the assembler's
    (
        "        FOUNDRY => (FOUNDRY_CAPACITY, FOUNDRY_MAX_INSERT, 0),\n",
        "        FOUNDRY => (ASSEMBLER_CAPACITY, FOUNDRY_MAX_INSERT, 0),\n",
    ),
    # the foundry is declared a tank it does not have
    (
        "        CENTRIFUGE => Some(CENTRIFUGE_TANK),\n",
        "        CENTRIFUGE => Some(CENTRIFUGE_TANK),\n        FOUNDRY => Some(CENTRIFUGE_TANK),\n",
    ),
    # the saved buffer is not read back
    (
        "        machine: Foundry {\n            energy: clamp_energy(read_int(nbt, KEY_ENERGY), capacity),\n",
        '        machine: Foundry {\n            energy: clamp_energy(read_int(nbt, "not_energy"), capacity),\n',
    ),
    # the line prints its two input slots the wrong way round
    (
        "        slot_text(in0),\n        slot_text(in1),\n        slot_text(view.output),\n",
        "        slot_text(in1),\n        slot_text(in0),\n        slot_text(view.output),\n",
    ),
    # the foundry is not in the core-offset table
    (
        "        FOUNDRY => Some(&FOUNDRY_CORE_OFFSETS),\n",
        "        FOUNDRY => None,\n",
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
