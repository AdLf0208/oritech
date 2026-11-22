#!/usr/bin/env python3
"""Counterfactual edits for the slice-8 glue tests (the Assembler).

The glue's tests pin the assembler's declarations, its saved compound, its line
and its place in the core-offset table. This script breaks each pinned fact by
one small mutation so the tests must go red; `restore` puts the file back from
`support.rs.bak`. Every mutation must be found exactly once, so a stale pattern
fails loudly instead of silently skipping.

    cp src/support.rs src/support.rs.bak
    python3 tools/slice8-glue-counterfactual.py mutate
    cargo test --offline assembler   # expect the slice-8 glue tests red
    python3 tools/slice8-glue-counterfactual.py restore
"""
import sys
from pathlib import Path

SRC = Path("/home/adlf/Desktop/all_the_mods_10_rust_rewrite/ports/oritech/src/support.rs")

MUTATIONS = [
    # one input slot instead of four
    (
        "            (0..ASSEMBLER_INPUTS).collect(),\n",
        "            (0..1).collect(),\n",
    ),
    # the buffer's capacity is the pulverizer's
    (
        "        ASSEMBLER => (ASSEMBLER_CAPACITY, ASSEMBLER_MAX_INSERT, 0),\n",
        "        ASSEMBLER => (PULVERIZER_CAPACITY, ASSEMBLER_MAX_INSERT, 0),\n",
    ),
    # the assembler's saved buffer is not read back
    (
        "        machine: Assembler {\n            energy: clamp_energy(read_int(nbt, KEY_ENERGY), capacity),\n",
        '        machine: Assembler {\n            energy: clamp_energy(read_int(nbt, "not_energy"), capacity),\n',
    ),
    # the line prints its third and fourth input slots the wrong way round
    (
        "        slot_text(in2),\n        slot_text(in3),\n",
        "        slot_text(in3),\n        slot_text(in2),\n",
    ),
    # the assembler is not in the core-offset table
    (
        "        ASSEMBLER => Some(&ASSEMBLER_CORE_OFFSETS),\n",
        "        ASSEMBLER => None,\n",
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
