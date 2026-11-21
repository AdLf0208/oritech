#!/usr/bin/env python3
"""Counterfactual edits for the slice-4 glue tests.

The glue's tests pin what it *declares* and the exact lines it prints. This
script breaks each pinned fact by one small mutation so the tests must go red;
`restore` puts the file back from `support.rs.bak`. Every mutation must be found
exactly once, so a stale pattern fails loudly instead of silently skipping.

    cp src/support.rs src/support.rs.bak
    python3 tools/slice4-glue-counterfactual.py mutate
    cargo test --offline      # expect the slice-4 glue tests red
    python3 tools/slice4-glue-counterfactual.py restore
"""
import sys
from pathlib import Path

SRC = Path("/home/adlf/Desktop/all_the_mods_10_rust_rewrite/ports/oritech/src/support.rs")

MUTATIONS = [
    # the lava generator's FE buffer (100,000 / 0 / 512)
    ("        LAVA => (LAVA_CAPACITY, 0, LAVA_MAX_EXTRACT),\n", ""),
    # the lava generator's tank (4,000 mB, fillable)
    ("        LAVA => Some(LAVA_TANK),\n", "        LAVA => None,\n"),
    # the tank amount clamp is the block's own capacity
    ("        tank: tank_state(nbt, LAVA_TANK),\n", "        tank: tank_state(nbt, 1),\n"),
    # the saved burn, which the readout and a reload both read
    ("    write_int(nbt, KEY_STORED_BURN, saved.machine.max_burn);\n", ""),
    # the energy push: the lava generator gives 512 a call, not the basic 256
    (
        "        LAVA => Some((LAVA_MAX_EXTRACT, lava_output_targets().to_vec())),\n",
        "        LAVA => Some((GENERATOR_MAX_EXTRACT, lava_output_targets().to_vec())),\n",
    ),
    # the readout's recipe
    (
        "         burning {}/{} ticks recipe {recipe}, tank {}, t={t}\",",
        "         burning {}/{} ticks, tank {}, t={t}\",",
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
