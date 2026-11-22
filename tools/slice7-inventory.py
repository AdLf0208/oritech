#!/usr/bin/env python3
"""Slice 7's recipe inventory: which of the jar's recipes this package can carry.

Read-only over `ports/_shared/licence-cache/oritech-neoforge-1.21.1-1.2.13.jar`.
The rule is the one `notes.md` → *Slice 7* states: a recipe is carried only when
its **result and every ingredient resolve** — a `minecraft:` id the engine has, an
item this package declares (`assets/oritech/items/*.json`), or a tag the package
ships with resolvable values — and none of its ingredients is an alternative
list (26.3's loader drops such a file).

    python3 tools/slice7-inventory.py             # every resolvable recipe, by type
    python3 tools/slice7-inventory.py --closure   # ... and the count the record cites

The result is the measurement behind `notes.md`'s recipe table: 20 of the jar's
965 recipe files resolve, the 18 carried ones plus `core2alt` and
`pulverizeralt`, which the slice keeps out as duplicates of a carried recipe.
"""
from __future__ import annotations

import json
import pathlib
import sys
import zipfile
from collections import Counter

PORT = pathlib.Path(__file__).resolve().parents[1]
JAR = PORT.parent / "_shared/licence-cache/oritech-neoforge-1.21.1-1.2.13.jar"


def declared() -> set[str]:
    """Every item id this package declares (an item definition in `assets/`)."""
    return {f"oritech:{path.stem}" for path in (PORT / "assets/oritech/items").glob("*.json")}


def tags() -> dict[str, list[str]]:
    """Every item tag this package ships, by `namespace:path`."""
    out: dict[str, list[str]] = {}
    for path in (PORT / "data").rglob("tags/item/**/*.json"):
        parts = path.relative_to(PORT / "data").parts
        values = json.loads(path.read_text())["values"]
        out[f"{parts[0]}:{'/'.join(parts[3:])[:-5]}"] = values
    return out


def resolvable(recipe: dict, items: set[str], shipped: dict[str, list[str]]) -> bool:
    def item(value) -> bool:
        return isinstance(value, str) and (value.startswith("minecraft:") or value in items)

    def tag(name: str) -> bool:
        if name.startswith("minecraft:"):
            return True
        return name in shipped and all(item(v) for v in shipped[name])

    def entry(value) -> bool:
        if isinstance(value, str):
            return tag(value[1:]) if value.startswith("#") else item(value)
        if isinstance(value, dict):
            return item(value["item"]) if "item" in value else tag(value["tag"])
        return False

    result = recipe.get("result")
    rid = result.get("id") if isinstance(result, dict) else None
    if not rid or not item(rid):
        return False
    ingredients = list(recipe.get("key", {}).values()) + list(recipe.get("ingredients", []))
    if "ingredient" in recipe:
        ingredients.append(recipe["ingredient"])
    if not ingredients or any(isinstance(i, list) for i in ingredients):
        return False
    return all(entry(i) for i in ingredients)


def main(argv: list[str]) -> int:
    items, shipped = declared(), tags()
    with zipfile.ZipFile(JAR) as jar:
        names = [n for n in jar.namelist() if n.startswith("data/oritech/recipe/") and n.endswith(".json")]
        hits = []
        for name in names:
            recipe = json.loads(jar.read(name))
            if resolvable(recipe, items, shipped):
                hits.append((recipe.get("type", "?"), name.split("/recipe/")[1]))
    for kind, name in sorted(hits):
        print(f"{kind:32s} {name}")
    print(f"\n{len(hits)} of {len(names)} recipe files resolve")
    if "--closure" in argv:
        print("by type:", dict(Counter(kind for kind, _ in hits)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
