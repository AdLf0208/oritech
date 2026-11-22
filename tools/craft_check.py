#!/usr/bin/env python3
"""Craft every recipe slice 7 carries in a real crafting table.

Called by `tools/e2e.sh` with the open game's control socket and the position of
a crafting table the phase placed. The harness numbers a crafting table's grid
100-108 row-major and its result slot 200, so each case below gives exactly one
item per grid cell the recipe's pattern names, places it, and asserts:

1. the result slot holds the recipe's **declared output id and count**;
2. taking it puts exactly that count in the inventory;
3. **every ingredient's inventory count fell by exactly what the recipe used** —
   the assertion that the ingredients were consumed, in survival;
4. the generator's differential: the same grid with one nickel replaced by a
   stick leaves the result slot empty.

`SELFTEST_CRAFT=1` expects the magnetic coil one item too high (5, not 4), so the
run MUST fail on that case: that is the counterfactual the e2e keeps.

The table is replaced before every case (the harness's `setblock` keeps a block
entity of the same id otherwise, and a stale grid would leak into the next case),
the player is put in survival for the craft and back in creative at the end, and
`interact` is retried from 2.5 blocks away because a menu whose face the eye
cannot see never opens — the failure that reads as `screen` timing out.
"""
import json
import os
import subprocess
import sys

GRID = range(100, 109)
RESULT = 200

# The jar's patterns (`data/oritech/recipe/…`), one item per named cell.
STEEL = {"minecraft:iron_ingot": [100, 101], "minecraft:coal": [103, 104]}
ELECTRUM = {"minecraft:gold_ingot": [100, 101], "minecraft:redstone": [103, 104]}
RAW_SILICON = {"minecraft:quartz": [100, 101], "minecraft:sand": [103, 104]}
COIL = {
    "oritech:nickel_ingot": [100, 101, 102, 106, 107, 108],
    "oritech:steel_ingot": [103, 104, 105],
}
MOTOR = {
    "oritech:nickel_ingot": [101],
    "oritech:steel_ingot": [103, 105, 106, 108],
    "oritech:magnetic_coil": [104, 107],
}
MACHINE_PLATING = {
    "oritech:steel_ingot": [101, 103, 105, 107],
    "minecraft:stone": [100, 102, 106, 108],
    "minecraft:copper_ingot": [104],
}
IRON_PLATING = {
    "oritech:steel_ingot": [101, 103, 105, 107],
    "minecraft:stone": [100, 102, 106, 108],
    "minecraft:iron_ingot": [104],
}
NICKEL_PLATING = {
    "oritech:steel_ingot": [101, 103, 105, 107],
    "minecraft:stone": [100, 102, 106, 108],
    "oritech:nickel_ingot": [104],
}
GENERATOR = {
    "oritech:nickel_ingot": [100, 101, 102, 103, 105],
    "oritech:magnetic_coil": [106, 108],
    "minecraft:copper_ingot": [104],
    "minecraft:furnace": [107],
}
PULVERIZER = {
    "minecraft:iron_ingot": [100, 101, 102, 103, 105],
    "oritech:motor": [106, 108],
    "oritech:nickel_ingot": [104],
    "minecraft:copper_block": [107],
}
FURNACE = {
    "minecraft:copper_ingot": [100, 101, 102],
    "oritech:silicon": [103, 105],
    "oritech:magnetic_coil": [106, 108],
    "oritech:electrum_ingot": [104],
    "minecraft:furnace": [107],
}
COOLER = {
    "minecraft:iron_ingot": [100, 101, 102, 103, 105],
    "oritech:motor": [104],
    "minecraft:ice": [106, 108],
    "minecraft:cauldron": [107],
}
LAVA_GENERATOR = {
    "oritech:electrum_ingot": [100, 101, 102, 103, 105],
    "oritech:magnetic_coil": [104],
    "oritech:machine_plating_block": [106, 108],
    "oritech:basic_generator_block": [107],
}
CORE_1 = {
    "minecraft:oak_planks": [100, 101, 102, 103, 105, 106, 107, 108],
    "minecraft:crafting_table": [104],
}
CORE_2 = {
    "minecraft:copper_ingot": [100, 101, 102, 103, 105, 106, 107, 108],
    "minecraft:lapis_lazuli": [104],
}
CORE_3 = {
    "oritech:nickel_ingot": [100, 101, 102, 103, 105, 106, 107, 108],
    "minecraft:redstone": [104],
}
CONTROL_UNIT = {
    "minecraft:redstone": [100, 101, 102, 103, 105],
    "minecraft:repeater": [106, 108],
    "minecraft:comparator": [104],
    "oritech:machine_plating_block": [107],
}

CASES = [
    {"label": "steel ingot: 2 iron + 2 coal", "place": STEEL,
     "want": ("oritech:steel_ingot", 1)},
    {"label": "electrum ingot: 2 gold + 2 redstone", "place": ELECTRUM,
     "want": ("oritech:electrum_ingot", 1)},
    {"label": "raw silicon: 2 quartz + 2 sand", "place": RAW_SILICON,
     "want": ("oritech:raw_silicon", 3)},
    {"label": "magnetic coil: 6 nickel + 3 steel", "place": COIL,
     "want": ("oritech:magnetic_coil", 4), "selftest": 5},
    {"label": "motor: 1 nickel + 4 steel + 2 coils", "place": MOTOR,
     "want": ("oritech:motor", 1)},
    {"label": "copper plating: 4 steel + 4 stone + 1 copper", "place": MACHINE_PLATING,
     "want": ("oritech:machine_plating_block", 2)},
    {"label": "iron plating: 4 steel + 4 stone + 1 iron", "place": IRON_PLATING,
     "want": ("oritech:iron_plating_block", 2)},
    {"label": "nickel plating: 4 steel + 4 stone + 1 nickel", "place": NICKEL_PLATING,
     "want": ("oritech:nickel_plating_block", 2)},
    {"label": "the Basic Generator: 5 nickel + 2 coils + 1 copper + 1 furnace",
     "place": GENERATOR, "want": ("oritech:basic_generator_block", 1)},
    {"label": "the Pulverizer: 5 iron + 2 motors + 1 nickel + 1 copper block",
     "place": PULVERIZER, "want": ("oritech:pulverizer_block", 1)},
    {"label": "the Powered Furnace: 3 copper + 2 silicon + 2 coils + 1 electrum",
     "place": FURNACE, "want": ("oritech:powered_furnace_block", 1)},
    {"label": "the Industrial Chiller: 5 iron + 1 motor + 2 ice + 1 cauldron",
     "place": COOLER, "want": ("oritech:cooler_block", 1)},
    {"label": "the Lava Generator: 5 electrum + 1 coil + 2 plating + 1 generator",
     "place": LAVA_GENERATOR, "want": ("oritech:lava_generator_block", 1)},
    {"label": "machine core 1: 8 planks + 1 crafting table", "place": CORE_1,
     "want": ("oritech:machine_core_1", 1)},
    {"label": "machine core 2: 8 copper + 1 lapis", "place": CORE_2,
     "want": ("oritech:machine_core_2", 1)},
    {"label": "machine core 3: 8 nickel + 1 redstone (the alt recipe)", "place": CORE_3,
     "want": ("oritech:machine_core_3", 1)},
    {"label": "the Control Unit Addon: 5 redstone + 2 repeaters + 1 comparator + 1 plating",
     "place": CONTROL_UNIT, "want": ("oritech:machine_redstone_addon", 1)},
]

# The generator's grid one nickel short: a stick where the fifth nickel belongs.
# A recipe that ignored its ingredients would still craft here.
CONTROL = {
    "oritech:nickel_ingot": [100, 101, 102, 103],
    "minecraft:stick": [105],
    "oritech:magnetic_coil": [106, 108],
    "minecraft:copper_ingot": [104],
    "minecraft:furnace": [107],
}


def main():
    sock, ctl, x, y, z = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]
    selftest = os.environ.get("SELFTEST_CRAFT") == "1"

    def run(*args):
        out = subprocess.run([ctl, "--socket", sock, "--timeout", "30", *args],
                             text=True, capture_output=True)
        try:
            return json.loads(out.stdout)
        except json.JSONDecodeError:
            raise SystemExit(f"ctl {' '.join(args)}: {out.stdout}{out.stderr}")

    def wait(ticks=2):
        run("wait", str(ticks))

    def counts():
        return run("inventory").get("counts", {})

    def slots():
        screen = run("screen")
        if "slots" not in screen:
            raise SystemExit(f"no menu is open: {screen}")
        return screen["slots"]

    def slot_of(number):
        return next((s for s in slots() if s["slot"] == number), None)

    def find(item):
        for slot in slots():
            if slot.get("item") == item and slot["slot"] < 100:
                return slot["slot"]
        raise SystemExit(f"missing ingredient {item} in the inventory")

    def empty_slot():
        for slot in slots():
            if not slot.get("item") and slot["slot"] < 100:
                return slot["slot"]
        raise SystemExit("no empty inventory slot to take the result into")

    def setup(given):
        """Open a fresh crafting table with exactly `given` in the inventory.

        The interact is retried from 2.5 blocks away and facing the table: with
        the player beside it the eye sits above the block, `interact` answers
        "no face … is visible", no menu opens, and every later click lands in the
        inventory — the failure that reads as `screen` timing out.
        """
        try:
            run("close")
        except SystemExit:
            pass
        wait(3)
        run("setblock", x, y, z, "minecraft:air")
        run("setblock", x, y, z, "minecraft:crafting_table")
        run("command", "gamemode survival @s")
        run("command", "clear @s")
        wait(2)
        for item, count in given:
            run("give", item, str(count))
        wait(3)
        screen = {}
        for _ in range(4):
            run("command", f"tp @s {float(x) - 2.5} {y} {float(z) + 0.5}")
            run("look-at-block", x, y, z)
            wait(2)
            run("interact", x, y, z)
            wait(4)
            screen = run("screen")
            if any(slot["slot"] == RESULT for slot in screen.get("slots", [])):
                return
        raise SystemExit(f"could not open the crafting table at {x},{y},{z}: {screen}")

    def place(item, targets):
        """Place one ITEM into each target slot: one pickup per grid cell."""
        remaining = list(targets)
        while remaining:
            source = find(item)
            stack = slot_of(source) or {}
            per_pickup = max(1, int(stack.get("count") or 1))
            run("click-slot", str(source))
            wait(1)
            for target in remaining[:per_pickup]:
                run("click-slot", str(target), "right")
                wait(1)
            del remaining[:per_pickup]
            run("click-slot", str(source))
            wait(2)

    def grid_state():
        return [{"slot": s["slot"], "item": s.get("item")}
                for s in slots() if s["slot"] in GRID]

    failures = []

    def fail(message):
        failures.append(message)
        print(f"FAIL: {message}", file=sys.stderr)

    for index, case in enumerate(CASES):
        want_item, want_count = case["want"]
        given = {item: len(targets) for item, targets in case["place"].items()}
        setup(sorted(given.items()))
        before = counts()
        for item, targets in case["place"].items():
            place(item, targets)

        result = slot_of(RESULT)
        got = result.get("item") if result else None
        got_count = result.get("count") if result else None
        expected = case.get("selftest", want_count) if selftest else want_count
        print(f"{case['label']} -> {json.dumps(result)}")
        if got != want_item or got_count != expected:
            fail(f"{case['label']}: expected {expected} {want_item}, got {got} x{got_count}; "
                 f"grid {json.dumps(grid_state())}")
            if selftest and case.get("selftest"):
                break
            run("close")
            continue
        if selftest and case.get("selftest"):
            fail(f"SELFTEST_CRAFT: the coil crafted {got_count}, the count this run expected "
                 "— the assertion is not alive")
            break

        # Take the result, then assert the inventory moved by exactly the recipe.
        run("click-slot", str(RESULT))
        wait(2)
        run("click-slot", str(empty_slot()))
        wait(3)
        after = counts()
        made = after.get(want_item, 0) - before.get(want_item, 0)
        if made != want_count:
            fail(f"{case['label']}: the inventory gained {made} {want_item}, not {want_count}")
        for item, used in given.items():
            left = before.get(item, 0) - after.get(item, 0)
            if left != used:
                fail(f"{case['label']}: {item} fell by {left}, not the {used} it used")
        run("close")
        wait(2)

    # The differential: the generator's grid one nickel short.
    given = {item: len(targets) for item, targets in CONTROL.items()}
    setup(sorted(given.items()))
    for item, targets in CONTROL.items():
        place(item, targets)
    result = slot_of(RESULT)
    print(f"the generator's grid one nickel short -> {json.dumps(result)}")
    if result and result.get("item"):
        fail(f"the incomplete generator grid crafted {result['item']} x{result.get('count')}")
    run("close")
    wait(2)
    run("command", "gamemode creative @s")

    if failures:
        raise SystemExit(1)
    print(f"recipe PASS: all {len(CASES)} carried recipes crafted in a real table with their "
          "declared output id and count, every ingredient consumed, and the generator's grid "
          "one nickel short crafting nothing")


if __name__ == "__main__":
    main()
