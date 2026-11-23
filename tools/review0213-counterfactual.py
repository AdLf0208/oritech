#!/usr/bin/env python3
"""The review fix's fail-capability: the two "no room for the result" tests must
fail when the completion stops asking whether the result fits.

Run from the port dir (`ports/oritech`): mutates `src/runtime.rs` in place,
runs `cargo test --offline centrifuge`, prints the outcome and restores the file
byte-identically (the sha256 before and after is printed and must match).

    python3 tools/review0213-counterfactual.py
"""
from __future__ import annotations

import hashlib
import pathlib
import subprocess
import sys

RUNTIME = pathlib.Path("src/runtime.rs")
ARM = """    if !centrifuge_output_fits(&recipe.results, outputs, max_stack) {
        return None;
    }
"""


def main() -> int:
    original = RUNTIME.read_text()
    before = hashlib.sha256(original.encode()).hexdigest()
    if ARM not in original:
        print("the fit arm is not in src/runtime.rs: the file moved, fix this script", file=sys.stderr)
        return 2
    mutant = original.replace(ARM, "")
    RUNTIME.write_text(mutant)
    try:
        run = subprocess.run(
            ["cargo", "test", "--offline", "centrifuge"],
            capture_output=True,
            text=True,
        )
        print(run.stdout[-6000:])
        print(run.stderr[-6000:], file=sys.stderr)
        failed = [line for line in run.stdout.splitlines() if line.startswith("test result:")]
        print(f"counterfactual exit={run.returncode} ({failed[-1] if failed else 'no result line'})")
    finally:
        RUNTIME.write_text(original)
    after = hashlib.sha256(RUNTIME.read_text().encode()).hexdigest()
    print(f"restored: {after == before} ({before})")
    return 0 if run.returncode != 0 and after == before else 1


if __name__ == "__main__":
    raise SystemExit(main())
