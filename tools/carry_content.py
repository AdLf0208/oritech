#!/usr/bin/env python3
"""Carry Oritech's content out of the ATM10 jar into the package root.

Slices 1 to 6 carried the machines, the cores and the seven addons; slice 7 adds
the material and component chain (seven items, three plating blocks, the tags
their recipes name and the eighteen recipes themselves).

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
    # Slice 3's chiller: same shape again (`blockstates/cooler_block.json` names
    # the `machine_speed_addon` placeholder, the look is the `.geo` model).
    "assets/oritech/textures/block/models/cooler_block.png",
    # Slice 4's lava generator: the same placeholder blockstate and `.geo` model.
    "assets/oritech/textures/block/models/lava_generator_block.png",
    # Slice 8's assembler: the same placeholder blockstate (it names the
    # `machine_speed_addon` model) and the same kind of `.geo` model.
    "assets/oritech/textures/block/models/assembler_block.png",
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
    # Slice 3: the chiller's two recipes whose fluid and result both exist here
    # (water -> ice, lava -> obsidian). The third, `snow.json`, takes
    # `oritech:still_steam`, a fluid this package does not declare, so it is not
    # carried: a recipe that can never match is dead data (notes.md, gap).
    "data/oritech/recipe/cooler/ice.json",
    "data/oritech/recipe/cooler/obsidian.json",
    # Slice 4: the lava generator's one recipe this package can run — 100 mB of
    # `minecraft:lava` for 120 ticks. Its `sheolfire.json` takes
    # `oritech:still_sheol_fire`, another fluid this package does not declare, so
    # it is the same recorded gap as `snow.json`.
    "data/oritech/recipe/lavagen/lava.json",
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

# Slice 5: the six default addon blocks. Their blockstate is authored (the jar's
# has 24 `face`/`facing`/`addon_used` variants, states this port's declaration
# does not model), but the block model, the item model and the `_off` texture are
# the jar's own, and every model names `machine_particle_texture` as its
# particle texture.
ADDON_NAMES = {
    "machine_speed_addon": "Speed Addon",
    "machine_efficiency_addon": "Efficiency Addon",
    "machine_processing_addon": "Auxiliary Processing Chamber Addon",
    "machine_capacitor_addon": "Capacitor Addon",
    "machine_burst_addon": "Burst Addon",
    "machine_ultimate_addon": "Synergy Matrix Addon",
    # Slice 6: the Control Unit Addon (`block.oritech.machine_redstone_addon` is
    # "Control Unit Addon" in the jar's lang). Its settings are the neutral
    # default: it is a control, not a multiplier.
    "machine_redstone_addon": "Control Unit Addon",
}
ADDON_ASSETS = [
    f"assets/oritech/{kind}/{path}"
    for block in ADDON_NAMES
    for kind, path in (
        ("models/block", f"{block}.json"),
        ("models/item", f"{block}.json"),
        ("textures/block", f"{block}_off.png"),
    )
] + ["assets/oritech/textures/block/machine_particle_texture.png"]
CARRIED += ADDON_ASSETS

# --- slice 7: the material and component chain --------------------------------
#
# The seven item components and the three plating blocks. Item models and item
# textures are the jar's own; the plating blocks' model names their `particle`
# texture (the jar's blockstate also carries `athena:ctm` keys this pin has no
# loader for, so the blockstate is authored and only the particle texture is
# needed for a `cube_all`).
ITEMS = {
    # id: (lang name, vanilla template the declaration copies)
    "steel_ingot": ("Steel Ingot", "minecraft:iron_ingot"),
    "nickel_ingot": ("Nickel Ingot", "minecraft:iron_ingot"),
    "electrum_ingot": ("Electrum Ingot", "minecraft:iron_ingot"),
    "raw_silicon": ("Raw Silicon", "minecraft:iron_ingot"),
    "silicon": ("Silicon", "minecraft:iron_ingot"),
    "magnetic_coil": ("Magnetic Coil", "minecraft:iron_ingot"),
    "motor": ("Motor", "minecraft:iron_ingot"),
    # Slice 8: the assembler's own recipe takes one, so the item slice 7 left out
    # (nothing consumed it then) is carried and craftable now.
    "adamant_ingot": ("Adamant Ingot", "minecraft:iron_ingot"),
}
PLATING = {
    # The jar's lang names them "… Reinforced Plating"; upstream copies
    # `Blocks.IRON_BLOCK` for them, the same copy the machines use.
    "machine_plating_block": "Copper Reinforced Plating",
    "iron_plating_block": "Iron Reinforced Plating",
    "nickel_plating_block": "Nickel Reinforced Plating",
}
ITEM_ASSETS = [
    f"assets/oritech/{kind}/{path}"
    for item in ITEMS
    for kind, path in (
        ("models/item", f"{item}.json"),
        ("textures/item", f"{item}.png"),
    )
]
PLATING_ASSETS = [
    f"assets/oritech/{kind}/{path}"
    for block in PLATING
    for kind, path in (
        ("models/block", f"{block}.json"),
        ("models/item", f"{block}.json"),
        ("textures/block", f"{block}/particle.png"),
    )
]
CARRIED += ITEM_ASSETS + PLATING_ASSETS

# Slice 7's recipes: the jar's own files, rewritten into 26.3's bare ingredient
# form. Nothing else about them changes (results, counts, patterns, times).
#
# The rule for taking one: its result and every ingredient must resolve — a
# `minecraft:` id the engine has, an item this package declares above, or a tag
# this package ships with resolvable values. The report's table lists what was
# left and why; `CONTENT.md` repeats it.
CHAIN_RECIPES = [
    # The alloys, from vanilla metal + coal/redstone/quartz/sand.
    "data/oritech/recipe/crafting/alloy/steel.json",
    "data/oritech/recipe/crafting/alloy/electrum.json",
    "data/oritech/recipe/crafting/alloy/rawsilicon.json",
    # The two components: the coil from nickel + steel, the motor from both.
    "data/oritech/recipe/crafting/magnet.json",
    "data/oritech/recipe/motor/motorcraft.json",
    # The three plating blocks the `oritech:plating` tag holds.
    "data/oritech/recipe/crafting/plating.json",
    "data/oritech/recipe/crafting/iron.json",
    "data/oritech/recipe/crafting/nickel.json",
    # The five machines this port ships, and the three machine cores whose
    # ingredients resolve (`core3`'s carbon fibre belongs to the centrifuge, so
    # its `core3alt` — nickel + redstone — is the one carried).
    "data/oritech/recipe/crafting/basicgen.json",
    "data/oritech/recipe/crafting/pulverizer.json",
    "data/oritech/recipe/crafting/electricfurnace.json",
    "data/oritech/recipe/crafting/cooler.json",
    "data/oritech/recipe/crafting/lavagen.json",
    "data/oritech/recipe/crafting/core1.json",
    "data/oritech/recipe/crafting/core2.json",
    "data/oritech/recipe/crafting/core3alt.json",
    # The seventh addon, whose four ingredients all resolve here (the other six
    # want plastic plates, energite or flux gates this package does not carry).
    "data/oritech/recipe/crafting/addon/redstone.json",
    # Raw silicon smelts into silicon in any furnace (60 ticks; the powered
    # furnace's 0.5 speed makes it 30).
    "data/oritech/recipe/silicon_from_smelting_raw_silicon.json",
]

# Slice 8's recipes: the five `oritech:assembler` files whose ingredients *and*
# result resolve here (see `notes.md` for the other 52), the assembler block's
# own craft (the jar's alt, which takes any `#oritech:plating`), and the adamant
# ingot that craft needs. Every one is the jar's file with the bare-ingredient
# rewrite only.
ASSEMBLER_RECIPES = [
    # The two component recipes this slice is about: two motors from a nickel, a
    # steel and two coils; six coils from steel, two nickel and copper.
    "data/oritech/recipe/assembler/motor.json",
    "data/oritech/recipe/assembler/magnet.json",
    # Three vanilla-only assemblies, free once the machine exists.
    "data/oritech/recipe/assembler/blazerod.json",
    "data/oritech/recipe/assembler/fireball.json",
    "data/oritech/recipe/assembler/shroomlight.json",
    # The machine itself, and its one non-plating-only ingredient.
    "data/oritech/recipe/crafting/assembleralt.json",
    "data/oritech/recipe/crafting/alloy/adamant.json",
]
REWRITTEN_RECIPES += ASSEMBLER_RECIPES
REWRITTEN_RECIPES += CHAIN_RECIPES

# Slice 7's tags. The jar's own `c:ingots/*` files list items this package does
# not declare (its steel tag repeats `steel_ingot` twice and names
# `biosteel_ingot`), so they are authored from the declarations above instead of
# copied. The vanilla-only tags are the conventional (`c:`) names NeoForge
# generates for vanilla items; 26.3 ships no `c:` namespace, so a recipe naming
# one resolves to nothing without these files.
CONVENTION_TAGS = {
    "c:ingots/steel": ["oritech:steel_ingot"],
    "c:ingots/nickel": ["oritech:nickel_ingot"],
    "c:ingots/electrum": ["oritech:electrum_ingot"],
    "c:ingots/iron": ["minecraft:iron_ingot"],
    "c:ingots/copper": ["minecraft:copper_ingot"],
    "c:ingots/gold": ["minecraft:gold_ingot"],
    "c:dusts/redstone": ["minecraft:redstone"],
    "c:dusts/quartz": ["minecraft:quartz"],
    "c:gems/diamond": ["minecraft:diamond"],
    "c:gems/lapis": ["minecraft:lapis_lazuli"],
    "c:silicon": ["oritech:silicon"],
    "c:storage_blocks/copper": ["minecraft:copper_block"],
    "c:player_workstations/furnaces": [
        "minecraft:furnace",
        "minecraft:blast_furnace",
        "minecraft:smoker",
    ],
    # The jar groups the plating blocks so the lava generator takes any of them;
    # its fourth member (carbon plating) needs the centrifuge's carbon fibre and
    # is not carried.
    "oritech:plating": [
        "oritech:machine_plating_block",
        "oritech:iron_plating_block",
        "oritech:nickel_plating_block",
    ],
}


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
    # Slice 3: the Industrial Chiller (upstream's `tooltip.oritech.cooler_block`
    # sits beside the block's name in the jar's lang).
    "cooler_block": {
        "name": "Industrial Chiller",
        "texture": "oritech:block/models/cooler_block",
    },
    # Slice 4: the Lava Generator (`block.oritech.lava_generator_block` is
    # "Lava Generator" in the jar's lang).
    "lava_generator_block": {
        "name": "Lava Generator",
        "texture": "oritech:block/models/lava_generator_block",
    },
    # Slice 8: the Assembler (`block.oritech.assembler_block` is "Assembler" in
    # the jar's lang). Its jar blockstate names the `machine_speed_addon` model
    # and its item model is `builtin/entity`, so both are authored here.
    "assembler_block": {
        "name": "Assembler",
        "texture": "oritech:block/models/assembler_block",
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
# Slice 7: the three plating blocks carry their model and their `particle`
# texture; their blockstate is authored (the jar's also carries `athena:ctm`
# keys this pin has no loader for), and their declaration copies the same iron
# block upstream copies.
for block, name in PLATING.items():
    BLOCKS[block] = {"name": name, "carry_model": True, "solid": True}
# Slice 5: the addon blocks carry their models and textures; only the blockstate
# is authored, because the jar's has states this port does not declare.
for block, name in ADDON_NAMES.items():
    BLOCKS[block] = {"name": name, "carry_model": True}

def authored() -> dict[str, str]:
    """The files written from the jar's facts, keyed by their path in the package."""
    files: dict[str, str] = {}

    def put(path: str, value) -> None:
        text = value if isinstance(value, str) else json.dumps(value, indent=2, sort_keys=True) + "\n"
        files[path] = text

    lang = {
        f"block.{NS}.{block}": data["name"] for block, data in BLOCKS.items()
    }
    lang |= {f"item.{NS}.{item}": name for item, (name, _) in ITEMS.items()}
    # Upstream's `ItemGroups.MACHINE_GROUP` title key.
    lang["itemgroup.oritech.machines"] = "Oritech Machines"
    # Slice 7: `ItemGroups.COMPONENT_GROUP`.
    lang["itemgroup.oritech.components"] = "Oritech Components"
    put(f"assets/{NS}/lang/en_us.json", lang)

    for block, data in BLOCKS.items():
        # The blockstate is authored unless the jar's own is usable as it stands
        # (the cores: one variant, vanilla states only).
        if not data.get("carry_assets"):
            put(
                f"assets/{NS}/blockstates/{block}.json",
                {"variants": {"": {"model": f"{NS}:block/{block}"}}},
            )
        # The block model and item model are carried for the cores and the six
        # addons; the machines' GeckoLib placeholder is replaced by a full cube.
        if not data.get("carry_assets") and not data.get("carry_model"):
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
        # for the machines and the cores alike; the plating blocks are full
        # opaque cubes, so only they keep the copy's occlusion.
        declaration = {"template": "minecraft:iron_block", "item": True}
        if not data.get("solid"):
            declaration["properties"] = {"occludes": False}
        put(f"data/{NS}/horizon/block/{block}.json", declaration)
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
    # Slice 7: the components tab (`ItemGroups.COMPONENT_GROUP`).
    put(
        f"data/{NS}/horizon/creative_tab/components.json",
        {
            "icon": f"{NS}:motor",
            "items": [f"{NS}:{item}" for item in ITEMS],
            "title": "itemgroup.oritech.components",
        },
    )
    # Slice 7: the items themselves — 26.3's item definition plus the
    # declaration that registers them (the vanilla template each copies).
    for item, (_, template) in ITEMS.items():
        put(
            f"assets/{NS}/items/{item}.json",
            {"model": {"type": "minecraft:model", "model": f"{NS}:item/{item}"}},
        )
        put(f"data/{NS}/horizon/item/{item}.json", {"template": template})
    # Slice 7: the tags the carried recipes name. `minecraft:` tags are the
    # engine's own; `c:` has no namespace at 26.3, so every conventional name a
    # carried recipe uses is declared here.
    for tag, values in CONVENTION_TAGS.items():
        namespace, name = tag.split(":", 1)
        put(f"data/{namespace}/tags/item/{name}.json", {"values": values})
    # `BlockContent`'s iron-block copy: a pickaxe and a stone tier, as the
    # template requires; a mod block is not in vanilla's tags by itself.
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
    shapes a file can use are a shaped recipe's `key` map, a shapeless recipe's
    `ingredients` list, a cooking recipe's single `ingredient`, and the mod's own
    machine recipes' `ingredients` list. A *list* of alternative ingredients is
    refused by 26.3's loader, so the script refuses it too rather than shipping a
    file the game drops. Everything else (type, results, counts, patterns, time)
    is the jar's.
    """
    recipe = json.loads(raw)

    def bare(ingredient):
        if isinstance(ingredient, dict):
            if "item" in ingredient:
                return ingredient["item"]
            if "tag" in ingredient:
                return f"#{ingredient['tag']}"
        if isinstance(ingredient, list):
            raise SystemExit(
                f"an alternative-list ingredient is fatal to 26.3's loader: {ingredient}"
            )
        return ingredient

    if "key" in recipe:
        recipe["key"] = {symbol: bare(entry) for symbol, entry in recipe["key"].items()}
    if "ingredients" in recipe:
        recipe["ingredients"] = [bare(entry) for entry in recipe["ingredients"]]
    if "ingredient" in recipe:
        recipe["ingredient"] = bare(recipe["ingredient"])
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
