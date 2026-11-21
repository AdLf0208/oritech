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
    "assets/oritech/textures/block/models/powered_furnace_block.png",
]

# The pulverizer recipes whose input and output both exist in this slice are the
# jar's own files, with one change: a 1.21.1 recipe writes its ingredient as an
# object (`{"item": …}` / `{"tag": …}`), and 26.3's recipe loader rejects that
# form ("invalid ingredient (unresolved tag or malformed input)") — the file is
# then dropped and the recipe can never craft. They are emitted in the bare form
# 26.3 takes (`"minecraft:bone"` / `"#minecraft:coal_ores"`); everything else
# (results, time, id) is the jar's. See notes.md, "Carry defect fixed".
REWRITTEN_RECIPES = [
    "data/oritech/recipe/pulverizer/bone.json",
    "data/oritech/recipe/pulverizer/blaze.json",
    "data/oritech/recipe/pulverizer/dripstone.json",
    "data/oritech/recipe/pulverizer/glowstoneore.json",
    "data/oritech/recipe/pulverizer/coalore.json",
    "data/oritech/recipe/pulverizer/diamondore.json",
    "data/oritech/recipe/pulverizer/lapisore.json",
]

# The seven machine cores are plain vanilla models in the jar (a `cube_all` of
# their own texture, one blockstate variant each), so their blockstate, block
# model, item model and texture travel byte for byte. `machine_core_hidden` is
# upstream's worldgen plating, not part of the multiblock; it is not carried.
CORE_ASSETS = [
    f"assets/oritech/{kind}/{path}"
    for core in range(1, 8)
    for kind, path in (
        ("blockstates", f"machine_core_{core}.json"),
        ("models/block", f"machine_core_{core}.json"),
        ("models/item", f"machine_core_{core}.json"),
        ("textures/block", f"machine_core_{core}.png"),
    )
]
CARRIED += CORE_ASSETS

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
    # Slice 2: the multiblock machine and the cores it assembles from. The
    # furnace's jar blockstate is the same GeckoLib placeholder as the others, so
    # its model is authored; the cores' assets are carried (`carry_assets`).
    "powered_furnace_block": {
        "name": "Powered Furnace",
        "texture": "oritech:block/models/powered_furnace_block",
    },
}
CORE_NAMES = [
    "Primitive Machine Core",
    "Basic Machine Core",
    "Improved Machine Core",
    "Advanced Machine Core",
    "Elite Machine Core",
    "Ultra Machine Core",
    "Ultimate Machine Core",
]
for tier, name in enumerate(CORE_NAMES, start=1):
    BLOCKS[f"machine_core_{tier}"] = {"name": name, "carry_assets": True}

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
        if not data.get("carry_assets"):
            # The jar's blockstate is a GeckoLib placeholder (`machine_speed_addon`);
            # a full cube of the machine's own texture is the closest a renderer
            # without GeckoLib can show. CONTENT.md records the deviation.
            put(
                f"assets/{NS}/blockstates/{block}.json",
                {"variants": {"": {"model": f"{NS}:block/{block}"}}},
            )
            put(
                f"assets/{NS}/models/block/{block}.json",
                {
                    "parent": "minecraft:block/cube_all",
                    "textures": {"all": data["texture"]},
                },
            )
            put(f"assets/{NS}/models/item/{block}.json", {"parent": f"{NS}:block/{block}"})
        # 26.3's item definition names the item model, carried or authored.
        put(
            f"assets/{NS}/items/{block}.json",
            {"model": {"type": "minecraft:model", "model": f"{NS}:item/{block}"}},
        )
        # `BlockContent`: `Properties.ofFullCopy(Blocks.IRON_BLOCK).noOcclusion()`
        # for the machines and the cores alike.
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
            "items": [f"{NS}:{block}" for block in BLOCKS],
            "title": "itemgroup.oritech.machines",
        },
    )
    # `Blocks.IRON_BLOCK`'s copy: a pickaxe and a stone tier, as the template
    # requires; a mod block is not in vanilla's tags by itself.
    for tag in ("mineable/pickaxe", "needs_stone_tool"):
        put(
            f"data/minecraft/tags/block/{tag}.json",
            {"values": [f"{NS}:{block}" for block in BLOCKS]},
        )
    return files


def expected() -> dict[str, bytes]:
    out: dict[str, bytes] = {}
    with zipfile.ZipFile(JAR) as jar:
        for name in CARRIED:
            out[name] = jar.read(name)
        for name in REWRITTEN_RECIPES:
            out[name] = bare_ingredients(jar.read(name))
    for path, text in authored().items():
        out[path] = text.encode()
    return out


def bare_ingredients(raw: bytes) -> bytes:
    """The jar's recipe with every ingredient in the bare form 26.3 loads.

    A 1.21.1 file writes `{"item": "minecraft:bone"}` or
    `{"tag": "minecraft:coal_ores"}`; 26.3's loader refuses those and drops the
    recipe, so they become `"minecraft:bone"` and `"#minecraft:coal_ores"`. The
    rest of the file (type, results, time) is untouched.
    """
    recipe = json.loads(raw)

    def bare(ingredient):
        if isinstance(ingredient, list):
            return [bare(entry) for entry in ingredient]
        if isinstance(ingredient, dict):
            if "item" in ingredient:
                return ingredient["item"]
            if "tag" in ingredient:
                return f"#{ingredient['tag']}"
        return ingredient

    recipe["ingredients"] = [bare(entry) for entry in recipe["ingredients"]]
    return (json.dumps(recipe, indent=2, sort_keys=True) + "\n").encode()


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
