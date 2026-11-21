#!/usr/bin/env python3
"""Counterfactual edits for the slice-6 glue tests (the Control Unit Addon).

The glue's tests pin the saved redstone flag and the exact lines the readout
prints. This script breaks each pinned fact by one small mutation so the tests
must go red; `restore` puts the file back from `support.rs.bak`. Every mutation
must be found exactly once, so a stale pattern fails loudly instead of silently
skipping.

    cp src/support.rs src/support.rs.bak
    python3 tools/slice6-glue-counterfactual.py mutate
    cargo test --offline redstone   # expect the slice-6 glue tests red
    python3 tools/slice6-glue-counterfactual.py restore
"""
import sys
from pathlib import Path

SRC = Path("/home/adlf/Desktop/all_the_mods_10_rust_rewrite/ports/oritech/src/support.rs")

MUTATIONS = [
    # the saved boolean is not read back
    (
        "    matches!(nbt.get(&key.into()), Some(Tag::Byte(value)) if *value != 0)\n",
        "    let _ = (nbt, key);\n    false\n",
    ),
    # ... and is written inverted
    (
        "    write_bool(nbt, KEY_REDSTONE, disabled);\n",
        "    write_bool(nbt, KEY_REDSTONE, !disabled);\n",
    ),
    # the readout's gate field disappears even with a control unit connected
    (
        "            redstone: found\n                .contains(&Some(REDSTONE_ADDON))\n                .then_some(disabled),\n",
        "            redstone: None,\n",
    ),
    # the gate reads the wrong word
    (
        '                " redstone disabled"\n',
        '                " redstone gated"\n',
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
