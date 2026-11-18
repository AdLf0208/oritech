#!/usr/bin/env python3
"""Carry Oritech slice 1's content out of the ATM10 jar into the package root.

Every carried file is copied byte for byte from
`../_shared/licence-cache/oritech-neoforge-1.21.1-1.2.13.jar`; every authored
file is written here from values read out of that jar or the upstream source
(`FINDINGS.md`), and `CONTENT.md` says which is which. The output is
deterministic: two runs give the same bytes.

    python3 tools/carry_content.py           # write assets/ + data/
    python3 tools/carry_content.py --check   # exit 1 if either differs
"""
from __future__ import annotations

import json
import sys
import zipfile
from pathlib import Path

PORT = Path(__file__).resolve().parents[1]
ROOT = PORT.parent
JAR = ROOT / "_shared/licence-cache/oritech-neoforge-1.21.1-1.2.13.jar"
CONTENT = PORT
NS = "oritech"

# --- carried from the jar, byte for byte -------------------------------------

CARRIED = [
    # The machines' own textures: the jar's GeckoLib `.geo` models are the look,
    # and these are their base textures. They are mapped onto a full cube below
    # because the pin has no GeckoLib renderer.
    "assets/oritech/textures/block/models/basic_generator_block.png",
    "assets/oritech/textures/block/models/pulverizer_block.png",
    # Pulverizer recipes whose input and output both exist in this slice: the
    # jar's own JSON, unchanged (the custom-recipe table keeps it as written).
    "data/oritech/recipe/pulverizer/bone.json",
    "data/oritech/recipe/pulverizer/blaze.json",
    "data/oritech/recipe/pulverizer/dripstone.json",
    "data/oritech/recipe/pulverizer/glowstoneore.json",
    "data/oritech/recipe/pulverizer/coalore.json",
    "data/oritech/recipe/pulverizer/diamondore.json",
    "data/oritech/recipe/pulverizer/lapisore.json",
]

# --- authored files ----------------------------------------------------------

BLOCKS = {
    "basic_generator_block": {
        "name": "Basic Generator",
        "texture": "oritech:block/models/basic_generator_block",
    },
    "pulverizer_block": {
        "name": "Pulverizer",
        "texture": "oritech:block/models/pulverizer_block",
    },
}

def authored() -> dict[str, str]:
    """The files written from the jar's facts, keyed by their path in the package."""
    files: dict[str, str] = {}

    def put(path: str, value) -> None:
        text = value if isinstance(value, str) else json.dumps(value, indent=2, sort_keys=True) + "\n"
        files[path] = text

    lang = {
        f"block.{NS}.{block}": data["name"] for block, data in BLOCKS.items()
    }
    # Upstream's `ItemGroups.MACHINE_GROUP` title key.
    lang["itemgroup.oritech.machines"] = "Oritech Machines"
    put(f"assets/{NS}/lang/en_us.json", lang)

    for block, data in BLOCKS.items():
        # The jar's blockstate is a GeckoLib placeholder (`machine_speed_addon`);
        # a full cube of the machine's own texture is the closest a renderer
        # without GeckoLib can show. CONTENT.md records the deviation.
        put(
            f"assets/{NS}/blockstates/{block}.json",
            {"variants": {"": {"model": f"{NS}:block/{block}"}}},
        )
        put(
            f"assets/{NS}/models/block/{block}.json",
            {"parent": "minecraft:block/cube_all", "textures": {"all": data["texture"]}},
        )
        put(f"assets/{NS}/models/item/{block}.json", {"parent": f"{NS}:block/{block}"})
        put(
            f"assets/{NS}/items/{block}.json",
            {"model": {"type": "minecraft:model", "model": f"{NS}:item/{block}"}},
        )
        # `BlockContent`: `Properties.ofFullCopy(Blocks.IRON_BLOCK).noOcclusion()`.
        put(
            f"data/{NS}/horizon/block/{block}.json",
            {
                "template": "minecraft:iron_block",
                "properties": {"occludes": False},
                "item": True,
            },
        )
        # 26.3's singular typed condition: the 1.21.1 plural array evaluates
        # false here and the block would drop nothing.
        put(
            f"data/{NS}/loot_table/blocks/{block}.json",
            {
                "type": "minecraft:block",
                "pools": [
                    {
                        "bonus_rolls": 0.0,
                        "condition": {"type": "minecraft:survives_explosion"},
                        "entries": [
                            {"type": "minecraft:item", "name": f"{NS}:{block}"}
                        ],
                        "rolls": 1.0,
                    }
                ],
                "random_sequence": f"{NS}:blocks/{block}",
            },
        )

    # Upstream's four creative tabs; this slice carries the machines tab only.
    put(
        f"data/{NS}/horizon/creative_tab/machines.json",
        {
            "icon": f"{NS}:basic_generator_block",
            "items": [f"{NS}:basic_generator_block", f"{NS}:pulverizer_block"],
            "title": "itemgroup.oritech.machines",
        },
    )
    # `Blocks.IRON_BLOCK`'s copy: a pickaxe and a stone tier, as the template
    # requires; a mod block is not in vanilla's tags by itself.
    for tag in ("mineable/pickaxe", "needs_stone_tool"):
        put(
            f"data/minecraft/tags/block/{tag}.json",
            {"values": [f"{NS}:basic_generator_block", f"{NS}:pulverizer_block"]},
        )
    return files


def expected() -> dict[str, bytes]:
    out: dict[str, bytes] = {}
    with zipfile.ZipFile(JAR) as jar:
        for name in CARRIED:
            out[name] = jar.read(name)
    for path, text in authored().items():
        out[path] = text.encode()
    return out


def main(argv: list[str]) -> int:
    if not JAR.is_file():
        print(f"missing jar: {JAR}", file=sys.stderr)
        return 1
    files = expected()
    check = "--check" in argv
    stale = []
    for path, data in sorted(files.items()):
        target = CONTENT / path
        if check:
            if not target.is_file() or target.read_bytes() != data:
                stale.append(path)
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    if check:
        for path in stale:
            print(f"differs: {path}")
        print(f"{len(files)} files, {len(stale)} differ")
        return 1 if stale else 0
    print(f"wrote {len(files)} files under {CONTENT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
