#!/usr/bin/env python3
"""Counterfactual edits for the slice-12 glue tests (the Atomic Forge).

The glue's tests pin the forge's three inputs/one output, its recipe-sized 5,120 FE
buffer (not the 1,024 config default), its eight-core assembly table and its saved
compound. This script breaks each by one mutation so the tests must go red;
`restore` puts the file back from `support.rs.bak`. Every mutation must be found
exactly once, so a stale pattern fails loudly instead of silently skipping.

    cp src/support.rs src/support.rs.bak
    python3 tools/slice12-glue-counterfactual.py mutate
    cargo test --offline atomic   # expect the slice-12 glue tests red
    python3 tools/slice12-glue-counterfactual.py restore
"""
import sys
from pathlib import Path

SRC = Path("/home/adlf/Desktop/all_the_mods_10_rust_rewrite/ports/oritech/src/support.rs")

MUTATIONS = [
    # the one output slot is not declared
    (
        "            vec![ATOMIC_OUTPUT],\n",
        "            vec![],\n",
    ),
    # the active 5,120 FE charge is replaced by the config's 1,024 default
    (
        "        ATOMIC => (ATOMIC_ACTIVE_CHARGE, ATOMIC_MAX_INSERT, 0),\n",
        "        ATOMIC => (1_024, ATOMIC_MAX_INSERT, 0),\n",
    ),
    # the eight-core assembly gate is gone
    (
        "        ATOMIC => Some(&ATOMIC_CORE_OFFSETS),\n",
        "        ATOMIC => None,\n",
    ),
    # the saved buffer is read from a key nobody writes
    (
        "        machine: AtomicForge {\n            energy: clamp_energy(read_int(nbt, KEY_ENERGY), capacity),\n",
        '        machine: AtomicForge {\n            energy: clamp_energy(read_int(nbt, "not_energy"), capacity),\n',
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
