#!/usr/bin/env python3
"""Counterfactual edits for the slice-11 glue tests (the Fragment Forge).

The glue's tests pin the grinder's three output slots, its 50,000/2,048/0 buffer,
its seven-core assembly table, its saved compound and the recipe-type registration.
This script breaks the first four by one mutation each so the tests must go red;
`restore` puts the file back from `support.rs.bak`. Every mutation must be found
exactly once, so a stale pattern fails loudly instead of silently skipping.

    cp src/support.rs src/support.rs.bak
    python3 tools/slice11-glue-counterfactual.py mutate
    cargo test --offline grinder   # expect the slice-11 glue tests red
    python3 tools/slice11-glue-counterfactual.py restore
"""
import sys
from pathlib import Path

SRC = Path("/home/adlf/Desktop/all_the_mods_10_rust_rewrite/ports/oritech/src/support.rs")

MUTATIONS = [
    # only two of the three output slots are declared
    (
        "            vec![GRINDER_OUTPUT, GRINDER_OUTPUT + 1, GRINDER_OUTPUT + 2],\n",
        "            vec![GRINDER_OUTPUT, GRINDER_OUTPUT + 1],\n",
    ),
    # the 2,048 FE insertion becomes zero
    (
        "        GRINDER => (GRINDER_CAPACITY, GRINDER_MAX_INSERT, 0),\n",
        "        GRINDER => (GRINDER_CAPACITY, 0, 0),\n",
    ),
    # the seven-core assembly gate is gone
    (
        "        GRINDER => Some(&GRINDER_CORE_OFFSETS),\n",
        "        GRINDER => None,\n",
    ),
    # the saved buffer is read from a key nobody writes
    (
        "        machine: Grinder {\n            energy: clamp_energy(read_int(nbt, KEY_ENERGY), capacity),\n",
        '        machine: Grinder {\n            energy: clamp_energy(read_int(nbt, "not_energy"), capacity),\n',
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
