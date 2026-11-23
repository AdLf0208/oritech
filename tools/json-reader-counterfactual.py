#!/usr/bin/env python3
"""The JSON reader's fail-capability: the malformed-document and the escape tests must
fail when the reader stops checking them.

Run from the port dir (`ports/oritech`): mutates `src/json.rs` in place with two
targeted defects, runs `cargo test --offline json::tests`, prints the outcome and
restores the file byte-identically (the sha256 before and after is printed and must
match).

    python3 tools/json-reader-counterfactual.py
"""
from __future__ import annotations

import hashlib
import pathlib
import subprocess
import sys

JSON = pathlib.Path("src/json.rs")
MUTATIONS = [
    # 1. Accept trailing text after the document ("{\"a\":1} trailing" must be refused).
    (
        "(reader.at == text.len()).then_some(value)",
        "Some(value)",
    ),
    # 2. Stop decoding \\uXXXX escapes (the unicode test must see the escape text, not é世).
    (
        "b'u' => out.push(self.hex_escape()?),",
        "b'u' => {\n                        self.hex4()?;\n                        out.push('?');\n                    }",
    ),
]


def main() -> int:
    original = JSON.read_text()
    before = hashlib.sha256(original.encode()).hexdigest()
    mutant = original
    for old, new in MUTATIONS:
        if old not in mutant:
            print(f"the reader moved; fix this script: {old!r}", file=sys.stderr)
            return 2
        mutant = mutant.replace(old, new)
    JSON.write_text(mutant)
    try:
        run = subprocess.run(
            ["cargo", "test", "--offline", "json::tests"],
            capture_output=True,
            text=True,
        )
        print(run.stdout[-6000:])
        print(run.stderr[-6000:], file=sys.stderr)
        failed = [line for line in run.stdout.splitlines() if line.startswith("test result:")]
        print(f"counterfactual exit={run.returncode} ({failed[-1] if failed else 'no result line'})")
    finally:
        JSON.write_text(original)
    after = hashlib.sha256(JSON.read_text().encode()).hexdigest()
    print(f"restored: {after == before} ({before})")
    return 0 if run.returncode != 0 and after == before else 1


if __name__ == "__main__":
    raise SystemExit(main())
