#!/usr/bin/env bash
# In-game check for Oritech slices 1 to 5: the Basic Generator, the Pulverizer,
# the Powered Furnace multiblock (controller + one machine core), the Industrial
# Chiller (a second multiblock, with a fluid tank), the Lava Generator (a
# fluid-burning generator with its own ten-target energy push) and the machine
# addon system (the six default addon blocks and their scan).
#
# STATUS: 0.2.14 (SDK package, pin 0a875da). The run needs a game the launcher
# built with this package: `ports/tools/launcher-game.sh build oritech`, then
# `eval "$(ports/tools/launcher-game.sh env oritech)"` and this script in a
# build-slot lease. The 0.1.0 and 0.2.x runs are the historical evidence for
# the slice-1 behaviour; every run of this file is the oracle for all slices.
# 0.2.14 replaces serde_json with the package's own reader (src/json.rs), so a
# run of this file is also that reader's regression test: every carried recipe
# family below is parsed through it.
#
# The arena is the Overworld at y=100 (a stone floor, clear air above), so every
# block read and right click is in the player's own dimension (`ctl tp` is
# reliable there; cross-dimension `execute in ... run tp` left the player in the
# void in early runs of this campaign).
#
# Requirements:
#   GAME=<horizon-client built by the launcher WITH this package>   (required)
#   CONTROL_GAME=<the same build WITHOUT it>                        (optional)
#   CTL=<horizon-ctl>                                             (default: horizon-ctl on PATH)
#
# Default run:
#  - a generator with no fuel makes no FE for 40 ticks (control);
#  - a pulverizer with a bone and no power keeps 0 FE, 0 progress and its bone
#    (control: the machine idles without power);
#  - a coal right click starts the burn: 32 FE a tick and the stack shrinks;
#  - a primed pulverizer finishes bone -> 6 bone meal in exactly 100 ticks and
#    spends exactly 3,200 FE (32 x 100), the jar's own numbers;
#  - the adjacent pulverizer takes the generator's FE (API 142 move) and turns
#    its bone into 6 bone meal;
#  - stone is refused as fuel and as an input, and is not placed on either
#    machine (the click is claimed);
#  - a spectator's right click inserts nothing (the game-mode check runs before
#    the slot write);
#  - the machines keep their FE/burn/output across `save` + quit + relaunch;
#  - a powered furnace with no core does nothing even with ore and 5,000 FE in
#    it (the assembly gate's counterfactual);
#  - a machine core in the hand is placed one block above the controller by a
#    right click, and the empty-hand click assembles the multiblock;
#  - 320 FE advances a smelt exactly ten ticks, and iron ore becomes an iron
#    ingot for exactly 3,200 FE: the jar's 0.5 speed turns a 200-tick recipe
#    into 100 ticks, and both a faster and a slower machine fail the assertion;
#  - breaking the core disassembles the machine mid-smelt, keeps its progress
#    and spends nothing until it is assembled again with another core;
#  - the seven machine cores and the furnace block place and read back, and
#    `oritech:machine_core_8` is refused;
#  - an industrial chiller with no core freezes nothing even with water and 5,000
#    FE (the second multiblock's assembly gate);
#  - a bucket of water enters its tank through the block's own declared
#    `horizon:fluids` handler (the reply names what was accepted), its core
#    assembles it one block to the north, 320 FE advances the craft exactly ten
#    ticks, and water becomes exactly 3 ice for exactly 6,400 FE with the tank
#    emptied;
#  - lava becomes exactly 2 obsidian on a second chiller (the second carried
#    recipe), and `oritech:still_steam` is refused: the jar's snow recipe is the
#    recorded gap;
#  - `/oritech fluid` answers a block without a tank (the furnace) with its
#    refusal, so the declaration is per block;
#  - a lava generator with 100 mB of lava and no core burns nothing; its core
#    arrives above it by a right click, and the tank's lava then leaves it at
#    load time while exactly 120 ticks make exactly 7,680 FE (64 a tick);
#  - the lava generator pushes that FE to an adjacent pulverizer, which spends
#    it on a bone (the ten-target output, proven by the consumer's own craft);
#  - an unassembled lava generator pushes nothing even with 5,000 FE stored, and
#    re-assembling it hands that FE to the same neighbour (the output gate);
#  - a second lava generator with an empty tank makes no FE for 200 ticks, and
#    one filled with water makes none either (the wrong-fluid control);
#  - `oritech:still_sheol_fire` is refused: the jar's second lavagen recipe is
#    the recorded fluid gap;
#  - every carried loot table rolls its block into the inventory, and a table
#    that does not exist gives nothing;
#  - all six default addon blocks place and read back, and an undeclared addon id
#    is refused;
#  - a speed addon in the pulverizer's (1,0,0) slot finishes a bone in exactly 67
#    ticks for exactly 3,886 FE (additive: speed 2/3, efficiency 1.2), while the
#    same machine without it takes 100 ticks and 3,200 (the differential);
#  - an efficiency addon costs exactly 2,700 FE for the same craft; a capacitor
#    reads 2,025,000 FE of capacity and holds a 25,000 and a 1,000,000 write; a
#    charged burst addon crafts in exactly 13 ticks for 416 FE; a processing addon
#    turns two bones into twelve bone meal in one completion for 4,800 FE;
#  - the furnace, the chiller and the lava generator run their own addon slots:
#    exactly 3,886 FE, 7,772 FE and 7,590 FE for the same craft/burn;
#  - a Control Unit Addon in the pulverizer's (1,0,0) slot with a redstone block
#    beside it gates the whole tick: 40 ticks leave `progress 0`, `recipe none`,
#    the bone still in the slot and the buffer untouched at exactly 10,000 FE,
#    and the readout says `redstone disabled`; removing the signal lets the same
#    machine, addon and bone finish for exactly 3,200 FE with `redstone enabled`,
#    so the addon with no signal is not a gate (the differential);
#  - every carried crafting recipe crafts in a real table: the steel/electrum/
#    raw-silicon alloys, the magnetic coil and the motor, the three plating
#    blocks, the five machines, three machine cores and the Control Unit Addon,
#    each asserting its output id and count, with the grid consumed afterwards
#    (survival), and the generator's grid one nickel short crafting nothing;
#  - raw silicon smelts into silicon in a powered furnace in exactly 30 ticks for
#    exactly 960 FE (the jar's `cookingtime: 60` at the 0.5 speed);
#  - a foundry with a valid pair and 5,000 FE but no core makes nothing; a core in
#    the hand fills each of its three core positions and the empty-hand click
#    assembles it with the three cores' average quality;
#  - two iron ingots are no alloy: 60 ticks leave `progress 0`, `recipe none`,
#    the buffer at exactly 5,000 FE and both irons in their slots (the
#    wrong-pair differential);
#  - one coal dust and one iron, in the slots *opposite* the file's order, become
#    exactly one steel ingot in exactly 26 ticks for exactly 3,328 FE with both
#    input slots emptied and the buffer at exactly 0;
#  - the other three carried alloys (adamant, electrum, netherite) each craft
#    their declared output for exactly their own `time x 128` FE;
#  - the foundry block's own crafting recipe crafts in a real table.
#
# CONTROL=1 (with CONTROL_GAME set) launches the package-less build first, where
# every id must be unknown: that is the differential run, and without it the
# scenario proves only that the ids exist.
#
# SELFTEST=1 flips the exact-FE assertion of each machine leg (it must FAIL),
# proving the scenario can fail; `SELFTEST_COOLER=1` does the same for the
# chiller's leg on its own, `SELFTEST_LAVA=1` for the lava generator's,
# `SELFTEST_ADDONS=1` for the speed addon's, `SELFTEST_REDSTONE=1` for the
# control unit's gate, `SELFTEST_CRAFT=1` for the coil's crafted count and
# `SELFTEST_ASSEMBLER=1` for the assembler's output count, `SELFTEST_CENTRIFUGE=1`
# for the centrifuge's carbon-fibre count, `SELFTEST_FOUNDRY=1` for the foundry's
# steel count, and `SELFTEST_CENTRIFUGE_ROOM=1` for the blocked-output phase of
# 0.2.13 (it expects a craft whose result cannot be installed to have run).
set -Eeuo pipefail
export LC_ALL=C

CLONE=${CLONE:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/oritech
CONTROL=${CONTROL:-0}
CONTROL_GAME=${CONTROL_GAME:-}

CTL=${CTL:-horizon-ctl}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/oritech-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=OritechE2E
CONTROL_WORLD=OritechControl

GENERATOR=oritech:basic_generator_block
PULVERIZER=oritech:pulverizer_block
FURNACE=oritech:powered_furnace_block
COOLER=oritech:cooler_block
LAVA=oritech:lava_generator_block
# The jar's numbers (`FINDINGS.md`).
GENERATOR_RATE=32
ROOM_CAPACITY=25000
RECIPE_TICKS=100
PRIMED=10000
SPENT=$((GENERATOR_RATE * RECIPE_TICKS))
# Slice 2: `OritechConfig.FurnaceConfig` (10,000 FE, 32 FE a tick, 0.5 speed).
FURNACE_CAPACITY=10000
FURNACE_RATE=32
FURNACE_TICKS=100
FURNACE_CORE_TIER=2
# Slice 3: `OritechConfig.processingMachines.coolerData` (50,000 FE, 32 FE a
# tick, speed 1) and the jar's `time: 200` / one bucket a craft.
COOLER_CAPACITY=50000
COOLER_RATE=32
COOLER_TICKS=200
COOLER_TANK=4000
COOLER_CORE_TIER=3
# Slice 4: `OritechConfig.generators.lavaGeneratorData` (100,000 FE, no insert,
# 512 extract, 64 FE a tick) and the jar's `lavagen/lava.json` (100 mB, time 120).
LAVA_CAPACITY=100000
LAVA_RATE=64
LAVA_TICKS=120
LAVA_TANK=4000
LAVA_CORE_TIER=2
# Slice 5: the six default addon blocks (`BlockContent.MACHINE_*_ADDON`) and
# their *additive* effects — `OritechConfig.additiveAddons` ships `true`, so the
# speed addon's 0.5 is added as a bonus and inverted back to 2/3, and its 1.2
# efficiency to 1.2. The figures are the integer results of the jar's formulas:
#   speed addon:      ceil(100 * 2/3) = 67 ticks, ceil(32 * 1.2 * 3/2) = 58 FE a tick
#   efficiency addon: 100 ticks,            ceil(32 * (1/1.2))      = 27 FE a tick
#   processing addon: 100 ticks,            ceil(32 * 1.5)          = 48 FE a tick, 2 per craft
#   burst addon:      ceil(100 / 8) = 13 ticks, 32 FE a tick (the bonus cancels)
#   capacitor:        +2,000,000 FE and +2,000 insert
#   furnace:          the same 67 ticks and 58 a tick (its 0.5 and /2 cancel)
#   chiller:          ceil(200 * 2/3) = 134 ticks at 58 a tick
#   lava generator:   (int)(120 * 2/3 / 1.2) = 66 ticks at (int)(64 * 1.2 * 3/2) = 115
SPEED_ADDON=oritech:machine_speed_addon
EFFICIENCY_ADDON=oritech:machine_efficiency_addon
PROCESSING_ADDON=oritech:machine_processing_addon
CAPACITOR_ADDON=oritech:machine_capacitor_addon
BURST_ADDON=oritech:machine_burst_addon
ULTIMATE_ADDON=oritech:machine_ultimate_addon
# Slice 6: the Control Unit Addon and the pulverizer buffer the gate is primed
# with. 10,000 FE is more than the whole 3,200-FE craft, so a machine that ticked
# while gated would visibly spend it.
REDSTONE_ADDON=oritech:machine_redstone_addon
REDSTONE_PRIMED=10000
# The whole base craft: 100 ticks x 32 FE. (Phase D reassigns SPENT with the
# measured tick count, so the gate leg keeps its own exact figure.)
REDSTONE_SPENT=$((GENERATOR_RATE * RECIPE_TICKS))
ADDON_TICKS=67
ADDON_RATE=58
ADDON_SPENT=$((ADDON_TICKS * ADDON_RATE))
EFFICIENCY_RATE=27
EFFICIENCY_SPENT=$((RECIPE_TICKS * EFFICIENCY_RATE))
PROCESSING_RATE=48
PROCESSING_SPENT=$((RECIPE_TICKS * PROCESSING_RATE))
# Slice 7: the material chain. `oritech:silicon` comes from the jar's
# `silicon_from_smelting_raw_silicon.json`: `cookingtime: 60`, and the powered
# furnace's 0.5 speed makes it 30 ticks at 32 FE a tick.
SILICON_TICKS=30
SILICON_SPENT=$((SILICON_TICKS * FURNACE_RATE))
BURST_TICKS=13
BURST_SPENT=$((BURST_TICKS * GENERATOR_RATE))
# Slice 8: `OritechConfig.processingMachines.assemblerData` (50,000 FE, 1,024
# insert, 128 FE a tick) and the jar's `assembler/motor.json` (48 ticks; one
# nickel + one steel + two coils -> two motors). The whole craft is 48 x 128 =
# 6,144 FE.
ASSEMBLER=oritech:assembler_block
ASSEMBLER_CAPACITY=50000
ASSEMBLER_RATE=128
ASSEMBLER_TICKS=48
ASSEMBLER_SPENT=$((ASSEMBLER_TICKS * ASSEMBLER_RATE))
ASSEMBLER_LEFT=$((10 * ASSEMBLER_RATE))
# The three core tiers the phase assembles with: 1, 2, 3, so the readout's
# average quality is 2 — a single core would say 2 as well, so the *list* is
# what proves all three were recorded.
ASSEMBLER_CORES="1 2 3"
ASSEMBLER_CORE_LIST="oritech:machine_core_1+oritech:machine_core_2+oritech:machine_core_3"
# Slice 9: `OritechConfig.processingMachines.centrifugeData` (10,000 FE, 512
# insert, 64 FE a tick, an eight-bucket tank) and the jar's own files:
#   centrifuge/carbon.json: 50 ticks  x 64 FE = exactly 3,200 FE
#   centrifuge/fluid/clay.json: 100 ticks x 64 FE = exactly 6,400 FE, 250 mB water
CENTRIFUGE=oritech:centrifuge_block
CENTRIFUGE_CAPACITY=10000
CENTRIFUGE_RATE=64
CENTRIFUGE_TICKS=50
CENTRIFUGE_SPENT=$((CENTRIFUGE_TICKS * CENTRIFUGE_RATE))
CENTRIFUGE_FLUID_TICKS=100
CENTRIFUGE_FLUID_SPENT=$((CENTRIFUGE_FLUID_TICKS * CENTRIFUGE_RATE))
CENTRIFUGE_TANK=8000
CENTRIFUGE_CORE_TIER=2
FLUID_ADDON=oritech:machine_fluid_addon
# Slice 10: `OritechConfig.processingMachines.foundryData` (50,000 FE, 1,024
# insert, 128 FE a tick) and the jar's own foundry files:
#   foundry/alloy/steel.json:    26 ticks x 128 FE = exactly 3,328 FE
#   the other three:             80 ticks x 128 FE = exactly 10,240 FE
FOUNDRY=oritech:foundry_block
FOUNDRY_CAPACITY=50000
FOUNDRY_RATE=128
FOUNDRY_TICKS=26
FOUNDRY_SPENT=$((FOUNDRY_TICKS * FOUNDRY_RATE))
FOUNDRY_ALLOY_TICKS=80
FOUNDRY_ALLOY_SPENT=$((FOUNDRY_ALLOY_TICKS * FOUNDRY_RATE))
# The three core tiers the phase assembles with, as the assembler's are: 1, 2, 3,
# so the readout's average quality is 2 and the *list* proves all three.
FOUNDRY_CORES="1 2 3"
FOUNDRY_CORE_LIST="oritech:machine_core_1+oritech:machine_core_2+oritech:machine_core_3"
CAPACITOR_CAPACITY=$((ROOM_CAPACITY + 2000000))
COOLER_ADDON_TICKS=134
COOLER_ADDON_SPENT=$((COOLER_ADDON_TICKS * ADDON_RATE))
LAVA_ADDON_TICKS=66
LAVA_ADDON_RATE=115
LAVA_ADDON_SPENT=$((LAVA_ADDON_TICKS * LAVA_ADDON_RATE))

command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; exit 1; }
command -v "$CTL" >/dev/null || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -n "$GAME" ] || {
    echo "GAME is required: point it at a horizon-client the launcher built with this package" >&2
    echo "  (horizon-launcher native-build $PORT --instance <i> --trust-native-code)" >&2
    exit 1
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -z "$CONTROL_GAME" ] || [ -x "$CONTROL_GAME" ] \
    || { echo "CONTROL_GAME is set but not executable: $CONTROL_GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}/oritech-home.XXXXXX")
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "${E2E_TMP:-/tmp/opencode}" "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 80 >&2 || true; exit 1; }
on_error() { echo "FAIL: line $1: '$2' exited $?" >&2; ctl logs --lines 40 >&2 || true; }
trap 'on_error "$LINENO" "$BASH_COMMAND"' ERR

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD] [GAME]
    local world=${1:-$WORLD} game=${2:-$GAME}
    rm -f "$SOCK"
    cd "$SCRATCH"
    echo "  launch: $game" >&2
    # `--offscreen`, not `--headless`: slice 7 opens a crafting table and reads
    # its slots, and at this pin a menu's widget layout needs the renderer —
    # with `--headless`, `ctl screen` times out on an open menu (measured; the
    # same note is in powah's e2e).
    "$CTL" --socket "$SOCK" launch \
        --game "$game" --world "$world" --seed 20261105 --creative --offscreen --wait \
        --resources "$RES" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }
# A mod block entity keeps its slots in the mod's own view, which `ctl container` (the
# engine's vanilla container map) does not reach; the machine's own `/oritech info` line
# is the readout, so these two helpers parse it (`read_info` sets FUEL/INPUT/OUT0/OUT1).
machine_holds() { # machine_holds X Y Z ITEM
    local line field
    line=$(info "$1" "$2" "$3")
    read_info "$line"
    for field in FUEL INPUT OUT0 OUT1; do
        case "${!field}" in
            "$4 x"*|"$4") return 0 ;;
        esac
    done
    return 1
}
has() { # has X Y Z ITEM COUNT
    machine_holds "$1" "$2" "$3" "$4" || fail "expected $5 x $4 at $1,$2,$3: $(info "$1" "$2" "$3")"
    case "$5" in
        1) ;;
        *) fail "has only checks one item here, not $5" ;;
    esac
    echo "  has($1,$2,$3) $4: ok"
}
has_not() { # has_not X Y Z ITEM
    if machine_holds "$1" "$2" "$3" "$4"; then
        fail "did not expect $4 at $1,$2,$3: $(info "$1" "$2" "$3")"
    fi
}

# The count in a slot field (`minecraft:bone_meal x12` -> 12; `empty` -> 0).
count_in() { # count_in "field"
    local n
    n=$(sed -n 's/.* x\([0-9][0-9]*\)$/\1/p' <<<"$1")
    echo "${n:-0}"
}

# Empty a machine's buffer and wait until its own readout says 0, so a following
# feed cannot be worked on before the buffer is reset: the addon legs prime an
# exact amount and a stray tick would make the arithmetic unreadable.
drain() { # drain X Y Z
    local line
    say "oritech energy $1 $2 $3 0"
    for _ in $(seq 1 12); do
        ctl wait 4
        line=$(info "$1" "$2" "$3")
        read_info "$line"
        if [ "${ENERGY:-x}" = 0 ]; then
            return 0
        fi
    done
    fail "the buffer at $1,$2,$3 never read 0: $line"
}

# A stone floor at y=99 with clear air above, in the Overworld, and the player
# standing on it at (0.5, 100, 4.5).
arena() {
    ctl tp 0.5 101 4.5 >/dev/null
    ctl wait 20
    ctl command "fill -8 99 -8 34 99 24 minecraft:stone" >/dev/null
    ctl command "fill -8 100 -8 34 110 24 minecraft:air" >/dev/null
    ctl command "kill @e[type=!player]" >/dev/null
    ctl tp 0.5 100 4.5 >/dev/null
    ctl wait 20
    local y feet floor screen
    y=$(ctl state | jq -r '.player.position[1] // 0')
    screen=$(ctl status | jq -r '.screen // empty')
    feet=$(block_name 0 100 4)
    floor=$(block_name 0 99 4)
    echo "  arena: y=$y screen=$screen floor(0,99,4)=$floor feet(0,100,4)=$feet" >&2
    [ "$screen" = "playing" ] || fail "the game is on screen '$screen', not playing"
    awk -v y="$y" 'BEGIN { exit !(y >= 99.9) }' || fail "the player is not on the arena floor (y=$y)"
    [ "$feet" = "minecraft:air" ] || fail "the arena volume is not clear ($feet at 0,100,4)"
    [ "$floor" = "minecraft:stone" ] || fail "the arena floor is missing ($floor at 0,99,4)"
}

# `/oritech info X Y Z`, retried until a *new* line for that position arrives
# (chat replies lag the send by ~1-2 s in this environment).
info() { # info X Y Z
    local x=$1 y=$2 z=$3 before out line
    before=$(ctl messages 24 | jq -r '.chat[]' | grep -F "at [$x, $y, $z]:" || true)
    say "oritech info $x $y $z"
    for _ in $(seq 1 30); do
        ctl wait 4 >/dev/null
        out=$(ctl messages 24)
        line=$(jq -r '.chat[]' <<<"$out" | grep -F "at [$x, $y, $z]:" | head -1 || true)
        if [ -n "$line" ] && [ "$line" != "$before" ]; then
            echo "  info($x,$y,$z): $line" >&2
            echo "$line"
            return
        fi
    done
    fail "no fresh /oritech info reply for $x,$y,$z: $(jq -c '{chat,action_bar}' <<<"$out")"
}

# `/oritech fluid X Y Z FLUID AMOUNT`: fill a machine's tank through the engine's
# own fluid handler. Answers the reply line, retried until a *new* one arrives
# (the chat read lags the send by ~1-2 s here).
fluid_put() { # fluid_put X Y Z FLUID AMOUNT
    local x=$1 y=$2 z=$3 fluid=$4 amount=$5 before out line
    before=$(ctl messages 24 | jq -r '.chat[]' | grep -F "at [$x, $y, $z]: tank" || true)
    say "oritech fluid $x $y $z $amount $fluid"
    for _ in $(seq 1 30); do
        ctl wait 4 >/dev/null
        out=$(ctl messages 24)
        line=$(jq -r '.chat[]' <<<"$out" | grep -F "at [$x, $y, $z]: tank" | head -1 || true)
        if [ -n "$line" ] && [ "$line" != "$before" ]; then
            echo "  fluid($x,$y,$z): $line" >&2
            echo "$line"
            return
        fi
    done
    fail "no fresh /oritech fluid reply for $x,$y,$z: $(jq -c '{chat}' <<<"$out")"
}

# `/oritech slot X Y Z SLOT COUNT ITEM`: write one slot of a machine's inventory
# (the harness route into an *output* slot: the right click fills input slots only,
# and `ctl container` cannot see a mod block entity). COUNT 0 empties it. Answers
# the reply line, retried until a *new* one arrives.
slot_put() { # slot_put X Y Z SLOT COUNT ITEM
    local x=$1 y=$2 z=$3 slot=$4 count=$5 item=$6 before out line
    before=$(ctl messages 24 | jq -r '.chat[]' | grep -F "at [$x, $y, $z]: slot " || true)
    say "oritech slot $x $y $z $slot $count $item"
    for _ in $(seq 1 30); do
        ctl wait 4 >/dev/null
        out=$(ctl messages 24)
        line=$(jq -r '.chat[]' <<<"$out" | grep -F "at [$x, $y, $z]: slot " | head -1 || true)
        if [ -n "$line" ] && [ "$line" != "$before" ]; then
            echo "  slot($x,$y,$z): $line" >&2
            echo "$line"
            return
        fi
    done
    fail "no fresh /oritech slot reply for $x,$y,$z: $(jq -c '{chat}' <<<"$out")"
}

# feed X Y Z ITEM: stand beside the machine and right-click it with ITEM in hand (the
# player's own insertion path; the machine keeps its slots in the mod's view).
feed() { # feed X Y Z ITEM
    ctl command "execute in minecraft:overworld run tp @s $1.5 100 $(( $3 - 2 )).5" >/dev/null
    ctl wait 6
    ctl command "item replace entity @s hotbar.0 with $4 1" >/dev/null
    ctl wait 4
    ctl select 1 >/dev/null
    ctl wait 3
    ctl look-at-block "$1" "$2" "$3" north >/dev/null
    ctl wait 2
    ctl interact "$1" "$2" "$3" >/dev/null 2>&1 || true
    ctl wait 4
}

# feed_south X Y Z ITEM: the same click from the machine's other side, for a machine
# whose north addon slot holds a block (the centrifuge's Fluid Addon) that would sit
# between the player and the face `feed` looks at.
feed_south() { # feed_south X Y Z ITEM
    ctl command "execute in minecraft:overworld run tp @s $1.5 100 $(( $3 + 2 )).5" >/dev/null
    ctl wait 6
    ctl command "item replace entity @s hotbar.0 with $4 1" >/dev/null
    ctl wait 4
    ctl select 1 >/dev/null
    ctl wait 3
    ctl look-at-block "$1" "$2" "$3" south >/dev/null
    ctl wait 2
    ctl interact "$1" "$2" "$3" >/dev/null 2>&1 || true
    ctl wait 4
}

# Parses one info line into shell fields (ENERGY, CAPACITY, BURN, MAXBURN, FUEL,
# PROGRESS, RECIPE, INPUT, OUT0, OUT1, T, KIND) and checks that all of them
# parsed. Sets the caller's variables through `eval`.
read_info() { # read_info "line"
    local assignments
    assignments=$(python3 "$PORT/tools/parse_info.py" "$1") || fail "unparsed info line: $1"
    eval "$assignments"
    [ "${ok:-0}" = 1 ] || fail "unparsed info line: $1"
}

expect() { # expect FIELD VALUE
    local field=$1 want=$2 got
    got=${!field}
    [ "$got" = "$want" ] || fail "$field is '$got', expected '$want'"
}

# The differential leg: the same ids in a build WITHOUT this package. Without
# it a passing run proves only that the ids exist, not that this package brought
# them.
control_leg() {
    if [ -z "$CONTROL_GAME" ]; then
        echo "  CONTROL=1 but no CONTROL_GAME: skipping the differential leg" >&2
        return
    fi
    echo "== control: a game built without this package =="
    launch "$CONTROL_WORLD" "$CONTROL_GAME"
    arena
    ctl command "item replace entity @s hotbar.0 with $GENERATOR 1" >/dev/null || true
    ctl wait 4
    ctl inventory | jq -e --arg item "$GENERATOR" \
        '[.hotbar[], .main[] | select(.item == $item)] | length == 0' >/dev/null \
        || fail "the control game has the generator item, so it was not a package-less build"
    ctl command "setblock 0 100 0 $GENERATOR" >/dev/null || true
    ctl wait 4
    local name
    name=$(block_name 0 100 0)
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the block id was known"
    # Slice 2's ids too: without this package neither the controller nor a core
    # may exist, so the furnace leg cannot pass by accident.
    ctl command "setblock 8 100 0 $FURNACE" >/dev/null 2>&1 || true
    ctl command "setblock 8 101 0 oritech:machine_core_2" >/dev/null 2>&1 || true
    ctl command "setblock 12 100 0 $COOLER" >/dev/null 2>&1 || true
    ctl command "setblock 12 100 -1 oritech:machine_core_2" >/dev/null 2>&1 || true
    ctl command "setblock 20 100 6 $LAVA" >/dev/null 2>&1 || true
    ctl command "setblock 5 100 0 $SPEED_ADDON" >/dev/null 2>&1 || true
    ctl command "setblock 5 100 1 $CAPACITOR_ADDON" >/dev/null 2>&1 || true
    ctl command "setblock 5 100 2 $REDSTONE_ADDON" >/dev/null 2>&1 || true
    # Slice 7: the components are items and a plain block, so the control game
    # must refuse both the item and the plating block.
    ctl command "item replace entity @s hotbar.3 with oritech:motor 1" >/dev/null 2>&1 || true
    ctl command "setblock 5 100 3 oritech:machine_plating_block" >/dev/null 2>&1 || true
    # Slice 8: the assembler block and the adamant ingot its recipe takes.
    ctl command "setblock 0 100 12 $ASSEMBLER" >/dev/null 2>&1 || true
    ctl command "item replace entity @s hotbar.4 with oritech:adamant_ingot 1" >/dev/null 2>&1 || true
    # Slice 9: the centrifuge, the Fluid Addon, and the two items of the chain.
    ctl command "setblock 8 100 10 $CENTRIFUGE" >/dev/null 2>&1 || true
    ctl command "setblock 5 100 4 $FLUID_ADDON" >/dev/null 2>&1 || true
    ctl command "item replace entity @s hotbar.5 with oritech:coal_dust 1" >/dev/null 2>&1 || true
    ctl command "item replace entity @s hotbar.6 with oritech:carbon_fibre_strands 1" >/dev/null 2>&1 || true
    # Slice 10: the foundry block and the steel it makes.
    ctl command "setblock 4 100 14 $FOUNDRY" >/dev/null 2>&1 || true
    ctl command "item replace entity @s hotbar.7 with oritech:steel_ingot 1" >/dev/null 2>&1 || true
    ctl wait 4
    local furnace core chiller lava addon capacitor control plating motor assembler adamant
    local centrifuge fluidaddon coaldust carbonfibre foundry steel
    furnace=$(block_name 8 100 0)
    core=$(block_name 8 101 0)
    chiller=$(block_name 12 100 0)
    lava=$(block_name 20 100 6)
    addon=$(block_name 5 100 0)
    capacitor=$(block_name 5 100 1)
    control=$(block_name 5 100 2)
    plating=$(block_name 5 100 3)
    motor=$(ctl inventory | jq -r '[.hotbar[], .main[] | select(.item == "oritech:motor")] | length')
    assembler=$(block_name 0 100 12)
    adamant=$(ctl inventory | jq -r '[.hotbar[], .main[] | select(.item == "oritech:adamant_ingot")] | length')
    centrifuge=$(block_name 8 100 10)
    fluidaddon=$(block_name 5 100 4)
    coaldust=$(ctl inventory | jq -r '[.hotbar[], .main[] | select(.item == "oritech:coal_dust")] | length')
    carbonfibre=$(ctl inventory | jq -r '[.hotbar[], .main[] | select(.item == "oritech:carbon_fibre_strands")] | length')
    foundry=$(block_name 4 100 14)
    steel=$(ctl inventory | jq -r '[.hotbar[], .main[] | select(.item == "oritech:steel_ingot")] | length')
    [ "$furnace" = "minecraft:air" ] || fail "the control game holds $furnace, so the furnace id was known"
    [ "$core" = "minecraft:air" ] || fail "the control game holds $core, so the core id was known"
    [ "$chiller" = "minecraft:air" ] || fail "the control game holds $chiller, so the chiller id was known"
    [ "$lava" = "minecraft:air" ] || fail "the control game holds $lava, so the lava generator id was known"
    [ "$addon" = "minecraft:air" ] || fail "the control game holds $addon, so the speed addon id was known"
    [ "$capacitor" = "minecraft:air" ] || fail "the control game holds $capacitor, so the capacitor addon id was known"
    [ "$control" = "minecraft:air" ] || fail "the control game holds $control, so the control unit addon id was known"
    [ "$plating" = "minecraft:air" ] || fail "the control game holds $plating, so the plating id was known"
    [ "$motor" = 0 ] || fail "the control game holds $motor oritech:motor, so the component id was known"
    [ "$assembler" = "minecraft:air" ] || fail "the control game holds $assembler, so the assembler id was known"
    [ "$adamant" = 0 ] || fail "the control game holds $adamant oritech:adamant_ingot, so that item id was known"
    [ "$centrifuge" = "minecraft:air" ] || fail "the control game holds $centrifuge, so the centrifuge id was known"
    [ "$fluidaddon" = "minecraft:air" ] || fail "the control game holds $fluidaddon, so the fluid addon id was known"
    [ "$coaldust" = 0 ] || fail "the control game holds $coaldust oritech:coal_dust, so that item id was known"
    [ "$carbonfibre" = 0 ] || fail "the control game holds $carbonfibre oritech:carbon_fibre_strands, so that item id was known"
    [ "$foundry" = "minecraft:air" ] || fail "the control game holds $foundry, so the foundry id was known"
    [ "$steel" = 0 ] || fail "the control game holds $steel oritech:steel_ingot, so that item id was known"
    echo "  without this package: no generator item, no motor, no adamant ingot, no coal dust, no carbon fibre and no steel ingot, and the generator, furnace, core, chiller, lava generator, addon, control unit, plating, assembler, centrifuge, fluid addon and foundry blocks all stayed $name"
    ctl quit >/dev/null 2>&1 || true
}

echo "== launch (Overworld arena at y=100, the launcher-built game) =="
if [ "$CONTROL" = 1 ]; then control_leg; fi
launch
arena

echo "== place the generator and a pulverizer out of its reach =="
ctl command "setblock 0 100 0 $GENERATOR" >/dev/null
ctl command "setblock 4 100 0 $PULVERIZER" >/dev/null
ctl wait 6
[ "$(block_name 0 100 0)" = "$GENERATOR" ] || fail "the generator did not place"
[ "$(block_name 4 100 0)" = "$PULVERIZER" ] || fail "the pulverizer did not place"
echo "  block(0,100,0) = $GENERATOR"
echo "  block(4,100,0) = $PULVERIZER"

echo "== A: a generator with no fuel makes no FE =="
line=$(info 0 100 0)
read_info "$line"
expect KIND generator
expect ENERGY 0
expect BURN 0
ctl wait 40
line=$(info 0 100 0)
read_info "$line"
expect ENERGY 0
echo "  after 40 ticks: $ENERGY/$CAPACITY FE, burning $BURN ticks"

echo "== B: a pulverizer with a bone and no power does nothing (control) =="
feed 4 100 0 minecraft:bone
line=$(info 4 100 0)
read_info "$line"
expect KIND pulverizer
expect ENERGY 0
expect PROGRESS 0
expect INPUT "minecraft:bone x1"
[ "$RECIPE" = "oritech:pulverizer/bone" ] || fail "the idle machine's recipe is '$RECIPE'"
ctl wait 120
line=$(info 4 100 0)
read_info "$line"
expect ENERGY 0
expect PROGRESS 0
expect INPUT "minecraft:bone x1"
has_not 4 100 0 minecraft:bone_meal
echo "  the unpowered machine kept its bone and made nothing"

echo "== C: a coal right click starts the burn at 32 FE a tick =="
ctl give minecraft:coal 5 >/dev/null
ctl wait 3
ctl select minecraft:coal >/dev/null
ctl wait 3
ctl look-at-block 0 100 0 up >/dev/null
ctl wait 2
echo "  click: $(ctl interact 0 100 0 | jq -c '{ok, block: .block.block, inventory_change}')"
ctl wait 40
line=$(info 0 100 0)
read_info "$line"
[ "$BURN" -gt 0 ] || fail "the coal click did not start the burn: $line"
[ "$FUEL" = "minecraft:coal x4" ] || fail "the fuel stack is '$FUEL', expected 4 coal left"
[ "$ENERGY" -ge 400 ] || fail "burning made only $ENERGY FE in 40 ticks"
echo "  generator alone: $ENERGY FE, burning $BURN/$MAXBURN ticks, $FUEL"

echo "== D: bone -> 6 bone meal in exactly 100 ticks for exactly 3,200 FE =="
say "oritech energy 4 100 0 $PRIMED"
ctl wait 8
line=$(info 4 100 0)
read_info "$line"
# The machine is already working by the time the reply lands: what it spent so
# far must be exactly 32 FE per tick of progress, and the rest of the operation
# must cost exactly 32 x the ticks left.
[ "$((PRIMED - ENERGY))" -eq "$((GENERATOR_RATE * PROGRESS))" ] \
    || fail "spent $((PRIMED - ENERGY)) FE for $PROGRESS ticks of progress"
E_START=$ENERGY
P_START=$PROGRESS
[ "$P_START" -lt "$RECIPE_TICKS" ] || fail "the operation finished before the first read"
ctl wait 130
line=$(info 4 100 0)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect OUT0 "minecraft:bone_meal x6"
SPENT=$((E_START - ENERGY))
WANT=$((GENERATOR_RATE * (RECIPE_TICKS - P_START)))
if [ "$SELFTEST" = 1 ]; then
    echo "  SELFTEST: expecting $((WANT + GENERATOR_RATE)) FE spent (this must fail)"
    WANT=$((WANT + GENERATOR_RATE))
fi
[ "$SPENT" -eq "$WANT" ] \
    || fail "$((RECIPE_TICKS - P_START)) ticks spent $SPENT FE, expected $WANT"
echo "  $((RECIPE_TICKS - P_START)) ticks spent exactly $SPENT FE ($E_START -> $ENERGY), output $OUT0"

echo "== E: the adjacent pulverizer runs on the generator's FE =="
ctl command "setblock 1 100 0 $PULVERIZER" >/dev/null
ctl wait 10
line=$(info 1 100 0)
read_info "$line"
[ "$ENERGY" -gt 0 ] || fail "the adjacent pulverizer got no FE from the generator: $line"
echo "  the generator moved $ENERGY FE into the adjacent pulverizer"
feed 1 100 0 minecraft:bone
line=$(info 1 100 0)
read_info "$line"
expect INPUT "minecraft:bone x1"
[ "$RECIPE" = "oritech:pulverizer/bone" ] || fail "the machine found recipe '$RECIPE'"
ctl wait 140
line=$(info 1 100 0)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect OUT0 "minecraft:bone_meal x6"
echo "  the powered machine finished: $OUT0"

echo "== F: stone is neither fuel nor a pulverizer input =="
ctl give minecraft:stone 2 >/dev/null
ctl wait 3
ctl select minecraft:stone >/dev/null
ctl wait 3
ctl look-at-block 0 100 0 up >/dev/null
ctl wait 2
echo "  generator click: $(ctl interact 0 100 0 | jq -c '{ok, block: .block.block}')"
ctl wait 6
has_not 0 100 0 minecraft:stone
[ "$(block_name 0 101 0)" = "minecraft:air" ] \
    || fail "stone was placed on the generator instead of being claimed"
ctl look-at-block 1 100 0 up >/dev/null
ctl wait 2
echo "  pulverizer click: $(ctl interact 1 100 0 | jq -c '{ok, block: .block.block}')"
ctl wait 6
has_not 1 100 0 minecraft:stone
[ "$(block_name 1 101 0)" = "minecraft:air" ] \
    || fail "stone was placed on the pulverizer instead of being claimed"
echo "  stone refused by both machines (no item taken, no block placed)"

echo "== F2: a spectator's right click inserts nothing =="
line=$(info 0 100 0)
read_info "$line"
FUEL_BEFORE=$FUEL
ctl command "gamemode spectator" >/dev/null
ctl wait 6
ctl give minecraft:coal 3 >/dev/null
ctl wait 3
ctl select minecraft:coal >/dev/null
ctl wait 3
ctl look-at-block 0 100 0 up >/dev/null
ctl wait 2
ctl interact 0 100 0 >/dev/null 2>&1 || true
ctl wait 20
line=$(info 0 100 0)
read_info "$line"
[ "$FUEL" = "$FUEL_BEFORE" ] \
    || fail "a spectator's click changed the generator's fuel ($FUEL_BEFORE -> $FUEL)"
ctl command "gamemode creative" >/dev/null
ctl wait 6
echo "  as a spectator: fuel still $FUEL"

echo "== G: the machines survive save + quit + relaunch =="
line=$(info 0 100 0)
read_info "$line"
BEFORE_BURN=$BURN
echo "  before: $line"
ctl save >/dev/null
ctl wait 5
ctl quit >/dev/null 2>&1 || true
sleep 3
launch
ctl tp 0.5 100 4.5 >/dev/null
ctl wait 20
line=$(info 0 100 0)
read_info "$line"
[ "$BURN" -gt 0 ] || fail "the generator lost its burn across a relaunch: $line"
[ "$FUEL" = "minecraft:coal x4" ] || fail "the generator lost its fuel across a relaunch: $line"
echo "  after relaunch: $ENERGY/$CAPACITY FE, burning $BURN/$MAXBURN ticks (was $BEFORE_BURN), $FUEL"
echo "  (the buffer reads 0 because the adjacent pulverizer drains it every tick)"
line=$(info 4 100 0)
read_info "$line"
expect OUT0 "minecraft:bone_meal x6"
line=$(info 4 100 0)
read_info "$line"
expect OUT0 "minecraft:bone_meal x6"
line=$(info 1 100 0)
read_info "$line"
expect OUT0 "minecraft:bone_meal x6"

echo "== H: a powered furnace with no core smelts nothing (the assembly gate) =="
ctl command "setblock 8 100 0 $FURNACE" >/dev/null
ctl wait 6
[ "$(block_name 8 100 0)" = "$FURNACE" ] || fail "the powered furnace did not place"
line=$(info 8 100 0)
read_info "$line"
expect KIND furnace
expect ASSEMBLED no
expect CORE empty
expect QUALITY 0
say "oritech energy 8 100 0 5000"
ctl wait 8
line=$(info 8 100 0)
read_info "$line"
expect ENERGY 5000
ctl wait 60
line=$(info 8 100 0)
read_info "$line"
expect ENERGY 5000
expect PROGRESS 0
echo "  without a core: $ENERGY/$CAPACITY FE after 60 ticks, progress $PROGRESS, core $CORE"

echo "== H2: the same furnace with a valid recipe and FE still refuses while unassembled =="
feed 8 100 0 minecraft:iron_ore
line=$(info 8 100 0)
read_info "$line"
expect ASSEMBLED no
expect INPUT "minecraft:iron_ore x1"
# `recipe` is the machine's *current* recipe, which only a tick records, so an
# unassembled machine shows none and the recipe is asserted mid-smelt in J2.
ctl wait 60
line=$(info 8 100 0)
read_info "$line"
expect ENERGY 5000
expect PROGRESS 0
expect OUT0 empty
has_not 8 100 0 minecraft:iron_ingot
echo "  unassembled, with ore and 5,000 FE: still progress $PROGRESS, $ENERGY FE, no ingot"

echo "== I: a machine core in the hand goes above the controller; the empty-hand click assembles =="
ctl give "oritech:machine_core_$FURNACE_CORE_TIER" 1 >/dev/null
ctl wait 3
ctl select "oritech:machine_core_$FURNACE_CORE_TIER" >/dev/null
ctl wait 3
ctl command "execute in minecraft:overworld run tp @s 8.5 100 -2.5" >/dev/null
ctl wait 6
ctl look-at-block 8 100 0 north >/dev/null
ctl wait 2
echo "  core click: $(ctl interact 8 100 0 | jq -c '{ok, block: .block.block}')"
ctl wait 8
CORE_BLOCK=$(block_name 8 101 0)
[ "$CORE_BLOCK" = "oritech:machine_core_$FURNACE_CORE_TIER" ] \
    || fail "the core click left $CORE_BLOCK above the controller, not a machine core"
echo "  block(8,101,0) = $CORE_BLOCK"
# The hand is emptied so the next click is the empty-hand route upstream uses
# for `initMultiblock` (a creative hand keeps the core when it places one).
ctl command "clear @s" >/dev/null
ctl wait 6
ctl look-at-block 8 100 0 north >/dev/null
ctl wait 2
ctl interact 8 100 0 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 8 100 0)
read_info "$line"
expect ASSEMBLED yes
expect CORE "oritech:machine_core_$FURNACE_CORE_TIER"
expect QUALITY "$FURNACE_CORE_TIER"
echo "  assembled: core $CORE quality $QUALITY, progress $PROGRESS"

echo "== J: iron ore -> iron ingot for exactly 3,200 FE (100 ticks x 32 FE) =="
# J1: the ore fed while unassembled is smelted now that the machine is one
# piece; the buffer was set to exactly 5,000 FE in H and nothing else drains it.
ctl wait 140
line=$(info 8 100 0)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect OUT0 "minecraft:iron_ingot x1"
expect ENERGY 1800
echo "  5000 -> $ENERGY FE, exactly $((5000 - ENERGY)) for one ingot, out0 $OUT0"

# J2: the duration, exactly. The buffer is emptied, the ore is fed, and the
# machine is given exactly ten ticks' worth of FE (10 x 32): it must advance
# exactly ten ticks and hold there starved. Then it gets exactly the FE the rest
# of the smelt needs (32 x the ticks left): a machine that finished in fewer
# ticks would leave FE in the buffer, and one that needed more would starve with
# 32 FE left and never produce. Neither can pass by accident, and no readout race
# is involved.
say "oritech energy 8 100 0 0"
ctl wait 6
feed 8 100 0 minecraft:iron_ore
line=$(info 8 100 0)
read_info "$line"
expect ENERGY 0
expect INPUT "minecraft:iron_ore x1"
P_ZERO=$PROGRESS
[ "$P_ZERO" -lt "$FURNACE_TICKS" ] || fail "the smelt finished before the measurement"
say "oritech energy 8 100 0 $((FURNACE_RATE * 10))"
ctl wait 40
line=$(info 8 100 0)
read_info "$line"
expect ENERGY 0
[ "$PROGRESS" -eq "$((P_ZERO + 10))" ] \
    || fail "320 FE should be exactly 10 ticks of work: progress $P_ZERO -> $PROGRESS"
P_START=$PROGRESS
[ "$P_START" -lt "$FURNACE_TICKS" ] || fail "the smelt finished in ten ticks"
echo "  320 FE advanced the smelt exactly 10 ticks ($P_ZERO -> $P_START of $FURNACE_TICKS)"
NEEDED=$((FURNACE_RATE * (FURNACE_TICKS - P_START)))
say "oritech energy 8 100 0 $NEEDED"
ctl wait 8
line=$(info 8 100 0)
read_info "$line"
[ "$RECIPE" = "minecraft:iron_ingot_from_smelting_iron_ore" ] \
    || fail "the working furnace's recipe is '$RECIPE'"
ctl wait 140
line=$(info 8 100 0)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect OUT0 "minecraft:iron_ingot x2"
WANT_LEFT=0
# Its own switch: `SELFTEST=1` flips the slice-1 assertion first (and stops
# there), so the furnace's exact-FE assertion is proven fail-capable separately.
if [ "${SELFTEST_FURNACE:-0}" = 1 ] || [ "$SELFTEST" = 1 ]; then
    echo "  SELFTEST: expecting $FURNACE_RATE FE left in the buffer (this must fail)"
    WANT_LEFT=$FURNACE_RATE
fi
[ "$ENERGY" -eq "$WANT_LEFT" ] \
    || fail "$((FURNACE_TICKS - P_START)) ticks needed exactly $NEEDED FE, so the buffer should hold $WANT_LEFT, not $ENERGY"
echo "  from progress $P_START: $((FURNACE_TICKS - P_START)) ticks spent exactly $NEEDED FE, buffer $ENERGY, out0 $OUT0"

echo "== L: breaking the core disassembles the machine and keeps the progress =="
feed 8 100 0 minecraft:iron_ore
say "oritech energy 8 100 0 $FURNACE_CAPACITY"
ctl wait 20
line=$(info 8 100 0)
read_info "$line"
[ "$PROGRESS" -gt 0 ] || fail "the third ore made no progress: $line"
ctl command "setblock 8 101 0 minecraft:air" >/dev/null
ctl wait 10
line=$(info 8 100 0)
read_info "$line"
expect ASSEMBLED no
expect CORE empty
# The freeze is measured from *after* the machine has noticed the core is gone:
# between the first read and the break it was still assembled, so it was right to
# keep working then.
FROZEN_PROGRESS=$PROGRESS
FROZEN_ENERGY=$ENERGY
ctl wait 60
line=$(info 8 100 0)
read_info "$line"
expect ENERGY "$FROZEN_ENERGY"
expect PROGRESS "$FROZEN_PROGRESS"
echo "  core broken: assembled $ASSEMBLED, progress held at $PROGRESS for 60 ticks, $ENERGY FE unspent"
# The core goes back: the same machine assembles again and resumes the smelt.
ctl command "setblock 8 101 0 oritech:machine_core_4" >/dev/null
ctl wait 8
# An empty hand is the assembly click; a held item would go into the input slot.
ctl command "clear @s" >/dev/null
ctl wait 6
ctl look-at-block 8 100 0 north >/dev/null
ctl wait 2
ctl interact 8 100 0 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 8 100 0)
read_info "$line"
expect ASSEMBLED yes
expect CORE oritech:machine_core_4
expect QUALITY 4
ctl wait 140
line=$(info 8 100 0)
read_info "$line"
expect PROGRESS 0
expect OUT0 "minecraft:iron_ingot x3"
echo "  reassembled with core 4 and finished the smelt it had started: out0 $OUT0"

echo "== M: every new block places and reads back; an unknown id is refused =="
for tier in 1 2 3 4 5 6 7; do
    ctl command "setblock 16 104 $tier oritech:machine_core_$tier" >/dev/null
done
ctl wait 8
for tier in 1 2 3 4 5 6 7; do
    GOT=$(block_name 16 104 "$tier")
    [ "$GOT" = "oritech:machine_core_$tier" ] || fail "core $tier placed as '$GOT'"
done
ctl command "setblock 16 104 9 oritech:machine_core_8" >/dev/null 2>&1 || true
ctl wait 4
[ "$(block_name 16 104 9)" = "minecraft:air" ] \
    || fail "oritech:machine_core_8 was accepted as a block"
ctl give "$FURNACE" 1 >/dev/null
ctl wait 4
ctl inventory | jq -e --arg item "$FURNACE" \
    '[.hotbar[], .main[] | select(.item == $item)] | length > 0' >/dev/null \
    || fail "the powered furnace item could not be given"
echo "  7 cores placed and read back, machine_core_8 refused, the furnace item given"

echo "== N: a tag-form pulverizer recipe crafts (the 26.3 ingredient form) =="
# The jar's seven recipes wrote their ingredient as `{"item": …}`/`{"tag": …}`,
# which 26.3's loader refuses; they now carry the bare form. `bone` (a bare item)
# is phase D; this is the `#minecraft:coal_ores` tag, end to end.
ctl command "setblock 20 100 0 $PULVERIZER" >/dev/null
ctl wait 6
[ "$(block_name 20 100 0)" = "$PULVERIZER" ] || fail "the tag-recipe pulverizer did not place"
feed 20 100 0 minecraft:coal_ore
line=$(info 20 100 0)
read_info "$line"
expect INPUT "minecraft:coal_ore x1"
[ "$RECIPE" = "oritech:pulverizer/coalore" ] \
    || fail "the coal ore found recipe '$RECIPE', so the tag form did not load"
say "oritech energy 20 100 0 $PRIMED"
ctl wait 140
line=$(info 20 100 0)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect OUT0 "minecraft:coal x2"
echo "  #minecraft:coal_ores -> $OUT0, the input consumed"

echo "== O: every carried pulverizer recipe crafts and consumes its input =="
# The parity rule: each carried recipe is crafted the way it declares, with the
# output id and count asserted *and* the input consumed. One machine each, so no
# recipe can borrow another's slot.
RECIPE_INPUTS=(minecraft:bone minecraft:blaze_rod minecraft:dripstone_block \
    minecraft:glowstone minecraft:coal_ore minecraft:diamond_ore minecraft:lapis_ore \
    minecraft:coal)
RECIPE_EXPECT=("minecraft:bone_meal x6" "minecraft:blaze_powder x3" \
    "minecraft:pointed_dripstone x4" "minecraft:glowstone_dust x3" "minecraft:coal x2" \
    "minecraft:diamond x1" "minecraft:lapis_lazuli x6" "oritech:coal_dust x1")
RECIPE_IDS=(bone blaze dripstone glowstoneore coalore diamondore lapisore coal)
for i in "${!RECIPE_INPUTS[@]}"; do
    ctl command "setblock $((28 + i * 2)) 100 0 $PULVERIZER" >/dev/null
done
ctl wait 8
for i in "${!RECIPE_INPUTS[@]}"; do
    feed $((28 + i * 2)) 100 0 "${RECIPE_INPUTS[$i]}"
    say "oritech energy $((28 + i * 2)) 100 0 $PRIMED"
done
ctl wait 160
for i in "${!RECIPE_INPUTS[@]}"; do
    X=$((28 + i * 2))
    line=$(info "$X" 100 0)
    read_info "$line"
    expect PROGRESS 0
    expect INPUT empty
    [ "$OUT0" = "${RECIPE_EXPECT[$i]}" ] \
        || fail "recipe ${RECIPE_IDS[$i]}: out0 is '$OUT0', expected '${RECIPE_EXPECT[$i]}'"
    echo "  oritech:pulverizer/${RECIPE_IDS[$i]}: ${RECIPE_INPUTS[$i]} -> $OUT0"
done


echo "== P: a chiller with no core freezes nothing, even with a bucket of water and 5,000 FE =="
ctl command "setblock 12 100 0 $COOLER" >/dev/null
ctl wait 6
[ "$(block_name 12 100 0)" = "$COOLER" ] || fail "the chiller did not place"
line=$(info 12 100 0)
read_info "$line"
expect KIND cooler
expect ENERGY 0
expect ASSEMBLED no
expect CORE empty
expect TANK_FLUID empty
say "oritech energy 12 100 0 5000"
ctl wait 8
# The tank is filled through the engine's own `horizon:fluids` handler, which is
# built from the tanks this block *declared*: `accepted 1000` is the declaration
# working, and a block without one accepts nothing (phase U).
line=$(fluid_put 12 100 0 minecraft:water 1000)
read_info "$line"
expect KIND fluid
expect TANK_FLUID minecraft:water
expect TANK_AMOUNT 1000
expect TANK_CAP "$COOLER_TANK"
expect ASKED 1000
expect ACCEPTED 1000
ctl wait 60
line=$(info 12 100 0)
read_info "$line"
expect ENERGY 5000
expect PROGRESS 0
expect TANK_AMOUNT 1000
expect OUT0 empty
has_not 12 100 0 minecraft:ice
echo "  unassembled, with water and 5,000 FE: still progress $PROGRESS, $ENERGY FE, tank $TANK_FLUID $TANK_AMOUNT, no ice"

echo "== Q: a machine core to the north assembles the chiller; the empty-hand click gates it =="
ctl give "oritech:machine_core_$COOLER_CORE_TIER" 1 >/dev/null
ctl wait 3
ctl select "oritech:machine_core_$COOLER_CORE_TIER" >/dev/null
ctl wait 3
ctl command "execute in minecraft:overworld run tp @s 12.5 100 2.5" >/dev/null
ctl wait 6
ctl look-at-block 12 100 0 south >/dev/null
ctl wait 2
echo "  core click: $(ctl interact 12 100 0 | jq -c '{ok, block: .block.block}')"
ctl wait 8
COOLER_CORE_BLOCK=$(block_name 12 100 -1)
[ "$COOLER_CORE_BLOCK" = "oritech:machine_core_$COOLER_CORE_TIER" ] \
    || fail "the core click left $COOLER_CORE_BLOCK north of the chiller, not a machine core"
echo "  block(12,100,-1) = $COOLER_CORE_BLOCK (the chiller's core is north, not above)"
ctl command "clear @s" >/dev/null
ctl wait 6
ctl look-at-block 12 100 0 south >/dev/null
ctl wait 2
ctl interact 12 100 0 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 12 100 0)
read_info "$line"
expect ASSEMBLED yes
expect CORE "oritech:machine_core_$COOLER_CORE_TIER"
expect QUALITY "$COOLER_CORE_TIER"
echo "  assembled: core $CORE quality $QUALITY, progress $PROGRESS"

echo "== R: 320 FE advances the craft exactly ten ticks =="
# The assembly started the craft against the 5,000 FE: take the buffer to zero and
# record where the progress stands, then pay for exactly ten ticks.
say "oritech energy 12 100 0 0"
ctl wait 8
line=$(info 12 100 0)
read_info "$line"
expect ENERGY 0
P_ZERO=$PROGRESS
[ "$P_ZERO" -lt "$COOLER_TICKS" ] || fail "the craft finished before the measurement"
say "oritech energy 12 100 0 $((COOLER_RATE * 10))"
ctl wait 40
line=$(info 12 100 0)
read_info "$line"
expect ENERGY 0
[ "$PROGRESS" -eq "$((P_ZERO + 10))" ] \
    || fail "320 FE should be exactly 10 ticks of work: progress $P_ZERO -> $PROGRESS"
P_START=$PROGRESS
[ "$P_START" -lt "$COOLER_TICKS" ] || fail "the craft finished in ten ticks"
expect TANK_AMOUNT 1000
echo "  320 FE advanced the craft exactly 10 ticks ($P_ZERO -> $P_START of $COOLER_TICKS), tank still $TANK_AMOUNT mB"

echo "== S: a bucket of water -> exactly 3 ice for exactly 6,400 FE, tank empty =="
NEEDED=$((COOLER_RATE * (COOLER_TICKS - P_START)))
say "oritech energy 12 100 0 $NEEDED"
ctl wait 8
line=$(info 12 100 0)
read_info "$line"
[ "$RECIPE" = "oritech:cooler/ice" ] || fail "the working chiller's recipe is '$RECIPE'"
ctl wait 200
line=$(info 12 100 0)
read_info "$line"
expect PROGRESS 0
expect OUT0 "minecraft:ice x3"
expect TANK_FLUID empty
expect TANK_AMOUNT 0
WANT_LEFT=0
# Its own switch, like the furnace's: the slice-3 exact-FE assertion is proven
# fail-capable on its own.
if [ "${SELFTEST_COOLER:-0}" = 1 ] || [ "$SELFTEST" = 1 ]; then
    echo "  SELFTEST: expecting $COOLER_RATE FE left in the buffer (this must fail)"
    WANT_LEFT=$COOLER_RATE
fi
[ "$ENERGY" -eq "$WANT_LEFT" ] \
    || fail "$((COOLER_TICKS - P_START)) ticks needed exactly $NEEDED FE, so the buffer should hold $WANT_LEFT, not $ENERGY"
echo "  from progress $P_START: $((COOLER_TICKS - P_START)) ticks spent exactly $NEEDED FE, buffer $ENERGY, out $OUT0, tank empty"

echo "== T: a bucket of lava -> exactly 2 obsidian (the second carried recipe) =="
ctl command "setblock 16 100 0 $COOLER" >/dev/null
ctl command "setblock 16 100 -1 oritech:machine_core_2" >/dev/null
ctl wait 8
[ "$(block_name 16 100 0)" = "$COOLER" ] || fail "the second chiller did not place"
ctl command "execute in minecraft:overworld run tp @s 16.5 100 2.5" >/dev/null
ctl wait 6
ctl command "clear @s" >/dev/null
ctl wait 4
ctl look-at-block 16 100 0 south >/dev/null
ctl wait 2
ctl interact 16 100 0 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 16 100 0)
read_info "$line"
expect ASSEMBLED yes
expect CORE oritech:machine_core_2
line=$(fluid_put 16 100 0 minecraft:lava 1000)
read_info "$line"
expect TANK_FLUID minecraft:lava
expect ACCEPTED 1000
say "oritech energy 16 100 0 $((COOLER_RATE * COOLER_TICKS))"
ctl wait 8
line=$(info 16 100 0)
read_info "$line"
[ "$RECIPE" = "oritech:cooler/obsidian" ] || fail "the lava chiller's recipe is '$RECIPE'"
ctl wait 240
line=$(info 16 100 0)
read_info "$line"
expect PROGRESS 0
expect OUT0 "minecraft:obsidian x2"
expect TANK_FLUID empty
expect ENERGY 0
echo "  lava -> $OUT0 for exactly $((COOLER_RATE * COOLER_TICKS)) FE, tank empty"

echo "== U: the tank is per block, and a fluid this pack does not have is refused =="
# The furnace has an item input and an FE buffer but no tank, so `/oritech fluid`
# answers its refusal instead of filling anything: the declaration is per block.
say "oritech fluid 8 100 0 1000 minecraft:water"
ctl wait 10
ctl messages 24 | jq -r '.chat[]' | grep -F "[8, 100, 0] has no Oritech fluid tank." >/dev/null \
    || fail "the furnace accepted a fluid: $(ctl messages 6 | jq -c '.chat')"
echo "  the furnace has no tank: /oritech fluid answered '[8, 100, 0] has no Oritech fluid tank.'"
# `snow.json` takes `oritech:still_steam`, a fluid this package does not declare
# (it is not carried: a recipe that can never match is dead data). The engine
# cannot even name it, so nothing is accepted — the measured gap.
line=$(fluid_put 12 100 0 oritech:still_steam 1000)
read_info "$line"
expect ACCEPTED 0
expect TANK_FLUID empty
echo "  oritech:still_steam: accepted $ACCEPTED of $ASKED mB, tank $TANK_FLUID (the jar's snow recipe is recorded as a gap)"

echo "== V: the chiller's and the lava generator's blocks and items place and read back =="
ctl give "$COOLER" 1 >/dev/null
ctl give "$LAVA" 1 >/dev/null
ctl wait 4
for block in "$COOLER" "$LAVA"; do
    ctl inventory | jq -e --arg item "$block" \
        '[.hotbar[], .main[] | select(.item == $item)] | length > 0' >/dev/null \
        || fail "the $block item could not be given"
done
echo "  block(12,100,0) = $COOLER, block(16,100,0) = $COOLER, block(0,100,6) = $LAVA, and both items are in the inventory"

echo "== X: a lava generator places, takes 100 mB of lava through its declared tank, and assembles =="
ctl command "setblock 0 100 6 $LAVA" >/dev/null
ctl wait 6
[ "$(block_name 0 100 6)" = "$LAVA" ] || fail "the lava generator did not place"
line=$(info 0 100 6)
read_info "$line"
expect KIND lava
expect ENERGY 0
expect CAPACITY "$LAVA_CAPACITY"
expect ASSEMBLED no
expect CORE empty
expect BURN 0
expect TANK_FLUID empty
# The tank is filled through the block's own declared handler: `accepted 100` is
# the `Registrar::block_fluids` declaration working, not a command writing NBT.
line=$(fluid_put 0 100 6 minecraft:lava 100)
read_info "$line"
expect TANK_FLUID minecraft:lava
expect TANK_AMOUNT 100
expect TANK_CAP "$LAVA_TANK"
expect ASKED 100
expect ACCEPTED 100
ctl wait 40
line=$(info 0 100 6)
read_info "$line"
expect ENERGY 0
expect BURN 0
expect TANK_AMOUNT 100
echo "  unassembled, with 100 mB of lava: still $ENERGY FE, burn $BURN, tank $TANK_FLUID $TANK_AMOUNT"
# The core goes above the controller (`LavaGeneratorEntity.getCorePositions` is
# (0, 1, 0), the furnace's offset).
ctl give "oritech:machine_core_$LAVA_CORE_TIER" 1 >/dev/null
ctl wait 3
ctl select "oritech:machine_core_$LAVA_CORE_TIER" >/dev/null
ctl wait 3
ctl command "execute in minecraft:overworld run tp @s 0.5 100 4.5" >/dev/null
ctl wait 6
ctl look-at-block 0 100 6 north >/dev/null
ctl wait 2
echo "  core click: $(ctl interact 0 100 6 | jq -c '{ok, block: .block.block}')"
ctl wait 8
CORE_BLOCK=$(block_name 0 101 6)
[ "$CORE_BLOCK" = "oritech:machine_core_$LAVA_CORE_TIER" ] \
    || fail "the core click left $CORE_BLOCK above the lava generator, not a machine core"
echo "  block(0,101,6) = $CORE_BLOCK"
ctl command "clear @s" >/dev/null
ctl wait 6
ctl look-at-block 0 100 6 north >/dev/null
ctl wait 2
ctl interact 0 100 6 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 0 100 6)
read_info "$line"
expect ASSEMBLED yes
expect CORE "oritech:machine_core_$LAVA_CORE_TIER"
expect QUALITY "$LAVA_CORE_TIER"
echo "  assembled: core $CORE quality $QUALITY, burning $BURN/$MAXBURN"

echo "== Y: a 100 mB lava burn is 120 ticks of exactly 64 FE each =="
line=$(info 0 100 6)
read_info "$line"
[ "$MAXBURN" -eq "$LAVA_TICKS" ] \
    || fail "the burn's length is $MAXBURN ticks, expected $LAVA_TICKS (the jar's time: 120)"
[ "$RECIPE" = "oritech:lavagen/lava" ] || fail "the burning machine's recipe is '$RECIPE'"
expect TANK_AMOUNT 0
# The fluid left the tank when the burn loaded, so every tick since then made
# exactly 64 FE: the buffer is 64 x (120 - the burn left).
BURNED=$((LAVA_TICKS - BURN))
[ "$ENERGY" -eq "$((LAVA_RATE * BURNED))" ] \
    || fail "$BURNED burning ticks hold $ENERGY FE, expected exactly $((LAVA_RATE * BURNED))"
echo "  burning $BURN/$MAXBURN: $BURNED ticks made exactly $ENERGY FE, tank emptied at load"
ctl wait 150
line=$(info 0 100 6)
read_info "$line"
expect BURN 0
expect RECIPE none
WANT=$((LAVA_RATE * LAVA_TICKS))
# Its own switch, like the furnace's and the chiller's: the lava leg's exact-FE
# assertion is proven fail-capable on its own.
if [ "${SELFTEST_LAVA:-0}" = 1 ] || [ "$SELFTEST" = 1 ]; then
    echo "  SELFTEST: expecting $((WANT + LAVA_RATE)) FE in the buffer (this must fail)"
    WANT=$((WANT + LAVA_RATE))
fi
[ "$ENERGY" -eq "$WANT" ] \
    || fail "a $LAVA_TICKS-tick lava burn makes exactly $((LAVA_RATE * LAVA_TICKS)) FE, so the buffer should hold $WANT, not $ENERGY"
echo "  one lava burn: exactly $ENERGY FE in the buffer, tank empty, no recipe left"

echo "== Y1: the jar's second lavagen recipe names a fluid this pack does not have =="
line=$(fluid_put 0 100 6 oritech:still_sheol_fire 1000)
read_info "$line"
expect ACCEPTED 0
expect TANK_FLUID empty
echo "  oritech:still_sheol_fire: accepted $ACCEPTED of $ASKED mB, tank $TANK_FLUID (the jar's sheolfire recipe is recorded as a gap)"

echo "== Y2: the ten-target push: an adjacent pulverizer receives the whole burn =="
ctl command "setblock 0 100 7 $PULVERIZER" >/dev/null
ctl wait 60
line=$(info 0 100 6)
read_info "$line"
expect ENERGY 0
line=$(info 0 100 7)
read_info "$line"
expect KIND pulverizer
[ "$ENERGY" -eq "$((LAVA_RATE * LAVA_TICKS))" ] \
    || fail "the adjacent pulverizer holds $ENERGY FE, expected the whole burn's $((LAVA_RATE * LAVA_TICKS))"
echo "  the lava generator pushed its $ENERGY FE one block south into the pulverizer"

echo "== Y3: the consumer spends the lava generator's FE on a craft =="
# The pulverizer stands directly south of the generator, so it is fed from its
# own south face (the generator blocks the north approach).
ctl command "execute in minecraft:overworld run tp @s 0.5 100 9.5" >/dev/null
ctl wait 6
ctl command "item replace entity @s hotbar.0 with minecraft:bone 1" >/dev/null
ctl wait 4
ctl select 1 >/dev/null
ctl wait 3
ctl look-at-block 0 100 7 south >/dev/null
ctl wait 2
ctl interact 0 100 7 >/dev/null 2>&1 || true
ctl wait 6
line=$(info 0 100 7)
read_info "$line"
expect INPUT "minecraft:bone x1"
[ "$RECIPE" = "oritech:pulverizer/bone" ] || fail "the fed pulverizer found recipe '$RECIPE'"
ctl wait 140
line=$(info 0 100 7)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect OUT0 "minecraft:bone_meal x6"
LEFT=$((LAVA_RATE * LAVA_TICKS - GENERATOR_RATE * RECIPE_TICKS))
[ "$ENERGY" -eq "$LEFT" ] \
    || fail "the consumer spent $((LAVA_RATE * LAVA_TICKS - ENERGY)) FE on the bone, expected exactly $((GENERATOR_RATE * RECIPE_TICKS)): $ENERGY left, expected $LEFT"
echo "  the consumer turned the lava generator's FE into $OUT0: $((LAVA_RATE * LAVA_TICKS)) -> $ENERGY FE"

echo "== Y4: an unassembled lava generator pushes nothing; re-assembly hands its FE over =="
# The gate on `outputEnergy`: upstream's `serverTick` returns at `isActive`
# before it, so a controller whose core is gone keeps its FE. Same buffer, same
# neighbour, only the assembly differs — that is the differential.
ctl command "setblock 0 101 6 minecraft:air" >/dev/null
ctl wait 10
line=$(info 0 100 6)
read_info "$line"
expect ASSEMBLED no
say "oritech energy 0 100 6 5000"
ctl wait 30
line=$(info 0 100 6)
read_info "$line"
expect ENERGY 5000
line=$(info 0 100 7)
read_info "$line"
PULVERIZER_BEFORE=$ENERGY
[ "$PULVERIZER_BEFORE" -eq "$((LAVA_RATE * LAVA_TICKS - GENERATOR_RATE * RECIPE_TICKS))" ] \
    || fail "the consumer's buffer moved while the generator was unassembled: $ENERGY"
echo "  unassembled with 5,000 FE: the buffer kept all of it and the pulverizer stayed at $PULVERIZER_BEFORE"
ctl command "setblock 0 101 6 oritech:machine_core_2" >/dev/null
ctl wait 8
ctl command "execute in minecraft:overworld run tp @s 0.5 100 4.5" >/dev/null
ctl wait 6
ctl command "clear @s" >/dev/null
ctl wait 4
ctl look-at-block 0 100 6 north >/dev/null
ctl wait 2
ctl interact 0 100 6 >/dev/null 2>&1 || true
ctl wait 40
line=$(info 0 100 6)
read_info "$line"
expect ASSEMBLED yes
expect ENERGY 0
line=$(info 0 100 7)
read_info "$line"
[ "$ENERGY" -eq "$((PULVERIZER_BEFORE + 5000))" ] \
    || fail "re-assembled, the generator's 5,000 FE should reach the pulverizer: it holds $ENERGY, expected $((PULVERIZER_BEFORE + 5000))"
echo "  reassembled: the 5,000 FE left the generator, the pulverizer rose $PULVERIZER_BEFORE -> $ENERGY"

echo "== Z: an empty tank makes no FE for 200 ticks (the empty-tank control) =="
ctl command "setblock 4 100 6 $LAVA" >/dev/null
ctl command "setblock 4 101 6 oritech:machine_core_2" >/dev/null
ctl wait 8
[ "$(block_name 4 100 6)" = "$LAVA" ] || fail "the second lava generator did not place"
ctl command "execute in minecraft:overworld run tp @s 4.5 100 4.5" >/dev/null
ctl wait 6
ctl command "clear @s" >/dev/null
ctl wait 4
ctl look-at-block 4 100 6 north >/dev/null
ctl wait 2
ctl interact 4 100 6 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 4 100 6)
read_info "$line"
expect ASSEMBLED yes
expect CORE oritech:machine_core_2
ctl wait 200
line=$(info 4 100 6)
read_info "$line"
expect ENERGY 0
expect BURN 0
expect TANK_FLUID empty
expect RECIPE none
echo "  assembled with an empty tank: $ENERGY FE after 200 ticks, burn $BURN, tank $TANK_FLUID"

echo "== Z2: a tank of water makes no FE (the wrong-fluid control) =="
# Upstream refuses water at the tank's own insert; this pin's declaration has no
# fluid filter (PLAN-slice4.md, deviation 2), so water enters and the machine
# still produces nothing because no lava recipe matches it.
line=$(fluid_put 4 100 6 minecraft:water 1000)
read_info "$line"
expect TANK_FLUID minecraft:water
expect TANK_AMOUNT 1000
expect ACCEPTED 1000
ctl wait 200
line=$(info 4 100 6)
read_info "$line"
expect ENERGY 0
expect BURN 0
expect RECIPE none
expect TANK_FLUID minecraft:water
expect TANK_AMOUNT 1000
echo "  a tank of water: $ENERGY FE after 200 ticks, still $TANK_FLUID $TANK_AMOUNT, no recipe"

echo "== AA: the eight default addon blocks place and read back =="
# The eight `BlockContent.MACHINE_*_ADDON` blocks are ordinary blocks and items in
# this package; an addon id it does not declare must be refused.
for addon in "$SPEED_ADDON" "$EFFICIENCY_ADDON" "$PROCESSING_ADDON" \
    "$CAPACITOR_ADDON" "$BURST_ADDON" "$ULTIMATE_ADDON" "$REDSTONE_ADDON" \
    "$FLUID_ADDON"; do
    ctl command "setblock 20 100 16 $addon" >/dev/null
    ctl wait 3
    [ "$(block_name 20 100 16)" = "$addon" ] \
        || fail "$addon did not place (it reads $(block_name 20 100 16))"
    ctl command "setblock 20 100 16 minecraft:air" >/dev/null
done
ctl command "setblock 21 100 16 oritech:machine_speed_addon_2" >/dev/null 2>&1 || true
ctl wait 3
[ "$(block_name 21 100 16)" = "minecraft:air" ] \
    || fail "an addon id this package does not declare placed"
echo "  all eight addon blocks placed and read back; an undeclared addon id was refused"

echo "== AB: a speed addon finishes a bone in exactly 67 ticks for exactly 3,886 FE =="
# The pulverizer at (4,100,0) has one addon slot at its (1,0,0). Prime exactly the
# full additive cost: a cheaper tick would leave FE behind, a dearer one would
# starve, and a 100-tick duration would not finish in the time given.
ctl command "setblock 5 100 0 $SPEED_ADDON" >/dev/null
# Empty the buffer first so the machine starves the bone while it is fed: priming
# after the feed is then the only FE the operation can spend.
drain 4 100 0
feed 4 100 0 minecraft:bone
line=$(info 4 100 0)
read_info "$line"
expect ADDONS speed
expect INPUT "minecraft:bone x1"
expect ENERGY 0
BEFORE_OUT=$(count_in "$OUT0")
PRIME=$ADDON_SPENT
if [ "${SELFTEST_ADDONS:-0}" = 1 ]; then
    echo "  SELFTEST_ADDONS: priming one tick too much (this must fail)"
    PRIME=$((ADDON_SPENT + ADDON_RATE))
fi
say "oritech energy 4 100 0 $PRIME"
ctl wait 90
line=$(info 4 100 0)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect ENERGY 0
[ "$(count_in "$OUT0")" -eq "$((BEFORE_OUT + 6))" ] \
    || fail "the speed-addon craft produced '$OUT0', expected 6 more bone meal: $line"
echo "  67 ticks at 58 FE (2/3 speed, 1.2 efficiency) finished for exactly $ADDON_SPENT FE, $OUT0"

echo "== AC: without the addon the same machine takes 100 ticks and 3,200 FE =="
# The same machine, the same bone, only the addon removed: the differential.
ctl command "setblock 5 100 0 minecraft:air" >/dev/null
drain 4 100 0
feed 4 100 0 minecraft:bone
line=$(info 4 100 0)
read_info "$line"
expect ADDONS none
expect ENERGY 0
BEFORE_OUT=$(count_in "$OUT0")
# Phase D reused `SPENT` for its measured value, so the base cost is written out
# here: 100 ticks x 32 FE.
BASE_SPENT=$((GENERATOR_RATE * RECIPE_TICKS))
say "oritech energy 4 100 0 $BASE_SPENT"
ctl wait 130
line=$(info 4 100 0)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect ENERGY 0
[ "$(count_in "$OUT0")" -eq "$((BEFORE_OUT + 6))" ] \
    || fail "the addon-free craft produced '$OUT0', expected 6 more bone meal: $line"
echo "  without the addon: exactly $BASE_SPENT FE, $OUT0 (the speed-addon leg is not a constant)"

echo "== AD: an efficiency addon makes the same craft cost exactly 2,700 FE =="
ctl command "setblock 5 100 0 $EFFICIENCY_ADDON" >/dev/null
drain 4 100 0
feed 4 100 0 minecraft:bone
line=$(info 4 100 0)
read_info "$line"
expect ADDONS efficiency
expect ENERGY 0
BEFORE_OUT=$(count_in "$OUT0")
say "oritech energy 4 100 0 $EFFICIENCY_SPENT"
ctl wait 130
line=$(info 4 100 0)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect ENERGY 0
[ "$(count_in "$OUT0")" -eq "$((BEFORE_OUT + 6))" ] \
    || fail "the efficiency-addon craft produced '$OUT0': $line"
echo "  100 ticks at 27 FE (1 / 1.2 efficiency) cost exactly $EFFICIENCY_SPENT FE"

echo "== AE: a capacitor addon raises the buffer to 2,025,000 FE and stops clamping =="
ctl command "setblock 5 100 0 $CAPACITOR_ADDON" >/dev/null
ctl wait 4
line=$(info 4 100 0)
read_info "$line"
expect ADDONS capacitor
expect CAPACITY "$CAPACITOR_CAPACITY"
say "oritech energy 4 100 0 1000000"
ctl wait 6
line=$(info 4 100 0)
read_info "$line"
expect ENERGY 1000000
expect CAPACITY "$CAPACITOR_CAPACITY"
say "oritech energy 4 100 0 $ROOM_CAPACITY"
ctl wait 6
line=$(info 4 100 0)
read_info "$line"
expect ENERGY "$ROOM_CAPACITY"
expect CAPACITY "$CAPACITOR_CAPACITY"
# Removing it clamps the buffer back: the same rule upstream's
# `updateEnergyContainer` applies when an addon goes.
say "oritech energy 4 100 0 $ROOM_CAPACITY"
ctl wait 4
ctl command "setblock 5 100 0 minecraft:air" >/dev/null
ctl wait 4
line=$(info 4 100 0)
read_info "$line"
expect ADDONS none
expect CAPACITY "$ROOM_CAPACITY"
echo "  capacitor: 25,000/2,025,000 and 1,000,000/2,025,000 held; without it 25,000 again"

echo "== AF: a charged burst addon crafts a bone in exactly 13 ticks for 416 FE =="
ctl command "setblock 5 100 0 $BURST_ADDON" >/dev/null
ctl wait 250
line=$(info 4 100 0)
read_info "$line"
expect ADDONS burst
expect BURST 240
expect BURST_MAX 240
drain 4 100 0
feed 4 100 0 minecraft:bone
line=$(info 4 100 0)
read_info "$line"
expect ENERGY 0
BEFORE_OUT=$(count_in "$OUT0")
say "oritech energy 4 100 0 $BURST_SPENT"
ctl wait 40
line=$(info 4 100 0)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect ENERGY 0
[ "$(count_in "$OUT0")" -eq "$((BEFORE_OUT + 6))" ] \
    || fail "the burst craft produced '$OUT0' in 40 ticks, so the burst was inert: $line"
echo "  burst 240/240: 13 ticks at 32 FE (the bonus cancels) finished for $BURST_SPENT FE"
ctl command "setblock 5 100 0 minecraft:air" >/dev/null
ctl wait 4
line=$(info 4 100 0)
read_info "$line"
expect ADDONS none
[ -z "$BURST" ] || fail "the burst counter is still printed without a burst addon: $line"
echo "  without the burst addon the counter is neither printed nor kept"

echo "== AG: a processing addon crafts two bones per completion for 4,800 FE =="
ctl command "setblock 5 100 0 $PROCESSING_ADDON" >/dev/null
drain 4 100 0
feed 4 100 0 minecraft:bone
feed 4 100 0 minecraft:bone
line=$(info 4 100 0)
read_info "$line"
expect ADDONS processing
expect INPUT "minecraft:bone x2"
expect ENERGY 0
BEFORE_OUT=$(count_in "$OUT0")
say "oritech energy 4 100 0 $PROCESSING_SPENT"
ctl wait 130
line=$(info 4 100 0)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect ENERGY 0
[ "$(count_in "$OUT0")" -eq "$((BEFORE_OUT + 12))" ] \
    || fail "one completion with a chamber consumed one input, not two: $line"
echo "  two bones -> 12 bone meal in one completion for exactly $PROCESSING_SPENT FE (1.5 efficiency)"
ctl command "setblock 5 100 0 minecraft:air" >/dev/null

echo "== AH: the furnace's speed addon smelts in exactly 67 ticks for 3,886 FE =="
# The furnace's own slot is (0,-1,0): one block below the controller, in the
# floor the arena laid.
ctl command "setblock 8 99 0 $SPEED_ADDON" >/dev/null
drain 8 100 0
feed 8 100 0 minecraft:iron_ore
line=$(info 8 100 0)
read_info "$line"
expect ADDONS speed
expect INPUT "minecraft:iron_ore x1"
expect ENERGY 0
BEFORE_OUT=$(count_in "$OUT0")
say "oritech energy 8 100 0 $ADDON_SPENT"
ctl wait 100
line=$(info 8 100 0)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect ENERGY 0
[ "$(count_in "$OUT0")" -eq "$((BEFORE_OUT + 1))" ] \
    || fail "the furnace's speed-addon smelt produced '$OUT0': $line"
echo "  furnace: 200 x 2/3 x 0.5 = 67 ticks, 58 FE a tick = exactly $ADDON_SPENT FE, $OUT0"

echo "== AI: the chiller's speed addon freezes in exactly 134 ticks for 7,772 FE =="
# The chiller's slot is (0,0,-2).
ctl command "setblock 12 100 -2 $SPEED_ADDON" >/dev/null
drain 12 100 0
line=$(fluid_put 12 100 0 minecraft:water 1000)
read_info "$line"
expect ACCEPTED 1000
line=$(info 12 100 0)
read_info "$line"
expect ADDONS speed
expect ENERGY 0
BEFORE_OUT=$(count_in "$OUT0")
say "oritech energy 12 100 0 $COOLER_ADDON_SPENT"
ctl wait 200
line=$(info 12 100 0)
read_info "$line"
expect PROGRESS 0
expect ENERGY 0
expect TANK_AMOUNT 0
[ "$(count_in "$OUT0")" -eq "$((BEFORE_OUT + 3))" ] \
    || fail "the chiller's speed-addon craft produced '$OUT0': $line"
echo "  chiller: 200 x 2/3 = 134 ticks at 58 FE = exactly $COOLER_ADDON_SPENT FE, $OUT0"

echo "== AJ: the lava generator's speed addon burns 66 ticks at 115 FE =="
ctl command "setblock 20 100 8 $LAVA" >/dev/null
ctl command "setblock 20 101 8 oritech:machine_core_2" >/dev/null
ctl wait 8
ctl command "execute in minecraft:overworld run tp @s 20.5 100 6.5" >/dev/null
ctl wait 6
ctl command "clear @s" >/dev/null
ctl wait 4
ctl look-at-block 20 100 8 north >/dev/null
ctl wait 2
ctl interact 20 100 8 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 20 100 8)
read_info "$line"
expect ASSEMBLED yes
ctl command "setblock 21 100 8 $SPEED_ADDON" >/dev/null
line=$(fluid_put 20 100 8 minecraft:lava 100)
read_info "$line"
expect ACCEPTED 100
ctl wait 110
line=$(info 20 100 8)
read_info "$line"
expect ADDONS speed
expect BURN 0
expect ENERGY "$LAVA_ADDON_SPENT"
expect TANK_FLUID empty
echo "  lava: (int)(120 x 2/3 / 1.2) = 66 ticks at (int)(64 x 1.2 x 3/2) = 115 FE = $LAVA_ADDON_SPENT FE"

echo "== AK: a powered Control Unit Addon gates the machine's whole tick =="
# A fresh pulverizer at (24,100,0) — a position no other phase uses, cleared first
# so its block entity is new: `setblock` of the *same* block keeps the old entity,
# and phase N's coal would then block the output slot. Its one addon slot is
# (1,0,0), and the Control Unit Addon there reads the vanilla signal at its own
# position — upstream's `RedstoneAddonBlock.neighborChanged` →
# `world.hasNeighborSignal(pos)` — which the machine re-reads every tick. The
# redstone block is read back by name, so the signal is recorded, not assumed.
ctl command "setblock 24 100 0 minecraft:air" >/dev/null
ctl command "setblock 25 100 0 minecraft:air" >/dev/null
ctl command "setblock 26 100 0 minecraft:air" >/dev/null
ctl wait 4
ctl command "setblock 24 100 0 $PULVERIZER" >/dev/null
ctl command "setblock 25 100 0 $REDSTONE_ADDON" >/dev/null
ctl command "setblock 26 100 0 minecraft:redstone_block" >/dev/null
ctl wait 8
[ "$(block_name 24 100 0)" = "$PULVERIZER" ] || fail "the gated pulverizer did not place"
[ "$(block_name 25 100 0)" = "$REDSTONE_ADDON" ] || fail "the control unit addon did not place"
SIGNAL=$(block_name 26 100 0)
echo "  block(24,100,0) = $(block_name 24 100 0), block(25,100,0) = $(block_name 25 100 0), block(26,100,0) = $SIGNAL"
[ "$SIGNAL" = "minecraft:redstone_block" ] || fail "the signal source at 26,100,0 reads '$SIGNAL'"
drain 24 100 0
feed 24 100 0 minecraft:bone
line=$(info 24 100 0)
read_info "$line"
expect ADDONS redstone
expect REDSTONE disabled
expect INPUT "minecraft:bone x1"
expect OUT0 empty
expect ENERGY 0
expect PROGRESS 0
expect RECIPE none
# Prime more than the whole craft: a machine that ticked while gated spends 32 FE
# a tick, so the figure below is what makes the gate visible.
say "oritech energy 24 100 0 $REDSTONE_PRIMED"
ctl wait 40
line=$(info 24 100 0)
read_info "$line"
expect ADDONS redstone
expect REDSTONE disabled
expect RECIPE none
expect PROGRESS 0
expect INPUT "minecraft:bone x1"
BEFORE_OUT=$(count_in "$OUT0")
if [ "${SELFTEST_REDSTONE:-0}" = 1 ]; then
    echo "  SELFTEST_REDSTONE: expecting a gated machine to have spent a tick (this must fail)"
    expect ENERGY "$((REDSTONE_PRIMED - GENERATOR_RATE))"
else
    expect ENERGY "$REDSTONE_PRIMED"
fi
echo "  powered: progress $PROGRESS recipe $RECIPE after 40 ticks, buffer still $ENERGY FE, input $INPUT, addons $ADDONS, redstone $REDSTONE"

echo "== AL: removing the signal releases the same machine, addon and bone =="
# The differential: the same pulverizer, the same Control Unit Addon, the same
# bone and the same buffer — only the signal goes. A gated machine would never
# finish; one that was never gated would have finished during AK.
ctl command "setblock 26 100 0 minecraft:air" >/dev/null
ctl wait 120
line=$(info 24 100 0)
read_info "$line"
expect ADDONS redstone
expect REDSTONE enabled
expect PROGRESS 0
expect INPUT empty
expect ENERGY "$((REDSTONE_PRIMED - REDSTONE_SPENT))"
[ "$(count_in "$OUT0")" -eq "$((BEFORE_OUT + 6))" ] \
    || fail "the released craft produced '$OUT0', expected 6 more bone meal: $line"
echo "  signal removed: the same addon let the bone finish in $RECIPE_TICKS ticks for exactly $REDSTONE_SPENT FE, buffer $ENERGY, out0 $OUT0"
ctl command "setblock 25 100 0 minecraft:air" >/dev/null
ctl command "setblock 24 100 0 minecraft:air" >/dev/null

echo "== AM: every carried recipe crafts in a real table, ingredients consumed =="
# Slice 7 is data: items, three plating blocks, conventional tags and the jar's
# own crafting recipes. `tools/craft_check.py` puts one ingredient in every cell
# the pattern names, asserts the result's id and count, takes it and asserts each
# ingredient's inventory count fell by exactly what the recipe used, and fills the
# generator's pattern one nickel short as the differential: that grid must craft
# nothing. It puts the player in survival for the craft and back in creative.
ctl command "setblock 32 100 0 minecraft:crafting_table" >/dev/null
ctl wait 6
[ "$(block_name 32 100 0)" = "minecraft:crafting_table" ] || fail "the crafting table did not place"
SELFTEST_CRAFT="${SELFTEST_CRAFT:-0}" python3 "$PORT/tools/craft_check.py" "$SOCK" "$CTL" 32 100 0 \
    || fail "the carried recipes did not all craft"
echo "  the crafting table's craft pass is above"
ctl command "setblock 32 100 0 minecraft:air" >/dev/null
ctl wait 4

echo "== AN: raw silicon smelts into silicon in the powered furnace =="
# The one cooking recipe this slice carries: the jar's `cookingtime: 60` at the
# powered furnace's 0.5 speed is 30 ticks, 32 FE a tick = exactly 960 FE. A
# recipe the table did not load would starve at 960 with the input still there.
# A fresh controller at (28,100,0) with no addon slot filled, so the arithmetic
# is the base one (phase AH leaves a speed addon on the first furnace).
ctl command "setblock 28 100 0 $FURNACE" >/dev/null
ctl command "setblock 28 101 0 minecraft:air" >/dev/null
ctl wait 8
ctl command "clear @s" >/dev/null
ctl give "oritech:machine_core_$FURNACE_CORE_TIER" 1 >/dev/null
ctl wait 3
ctl select "oritech:machine_core_$FURNACE_CORE_TIER" >/dev/null
ctl wait 3
ctl command "execute in minecraft:overworld run tp @s 28.5 100 -2.5" >/dev/null
ctl wait 6
ctl look-at-block 28 100 0 north >/dev/null
ctl wait 2
echo "  core click: $(ctl interact 28 100 0 | jq -c '{ok, block: .block.block}')"
ctl wait 8
CORE_BLOCK=$(block_name 28 101 0)
[ "$CORE_BLOCK" = "oritech:machine_core_$FURNACE_CORE_TIER" ] \
    || fail "the core click left $CORE_BLOCK above the new controller"
ctl command "clear @s" >/dev/null
ctl wait 6
ctl look-at-block 28 100 0 north >/dev/null
ctl wait 2
ctl interact 28 100 0 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 28 100 0)
read_info "$line"
expect ASSEMBLED yes
feed 28 100 0 oritech:raw_silicon
line=$(info 28 100 0)
read_info "$line"
expect INPUT "oritech:raw_silicon x1"
expect OUT0 empty
say "oritech energy 28 100 0 $SILICON_SPENT"
ctl wait 45
line=$(info 28 100 0)
read_info "$line"
expect PROGRESS 0
expect INPUT empty
expect ENERGY 0
expect OUT0 "oritech:silicon x1"
echo "  raw silicon smelted in exactly $SILICON_TICKS ticks for exactly $SILICON_SPENT FE: out0 $OUT0"
ctl command "setblock 28 101 0 minecraft:air" >/dev/null
ctl command "setblock 28 100 0 minecraft:air" >/dev/null

echo "== AO: an unassembled assembler with a full grid and 1,280 FE does nothing =="
# Slice 8. `AssemblerBlockEntity` is a `MultiblockMachineEntity`: three cores at
# (0,0,1), (0,1,0) and (0,1,1), and `isActive(ASSEMBLED)` gates the whole tick.
# The grid is the jar's `assembler/motor.json`, whose four ingredients are one
# nickel, one steel and two coils; the click's `findLowestMatchingSlot` fills the
# first empty slot, so four feeds land in four slots.
ctl command "setblock 0 100 12 minecraft:air" >/dev/null
ctl command "setblock 0 100 13 minecraft:air" >/dev/null
ctl command "setblock 0 101 12 minecraft:air" >/dev/null
ctl command "setblock 0 101 13 minecraft:air" >/dev/null
ctl wait 4
ctl command "setblock 0 100 12 $ASSEMBLER" >/dev/null
ctl wait 8
[ "$(block_name 0 100 12)" = "$ASSEMBLER" ] || fail "the assembler did not place"
line=$(info 0 100 12)
read_info "$line"
expect KIND assembler
expect ASSEMBLED no
expect ENERGY 0
expect CAPACITY "$ASSEMBLER_CAPACITY"
drain 0 100 12
feed 0 100 12 oritech:nickel_ingot
feed 0 100 12 oritech:steel_ingot
feed 0 100 12 oritech:magnetic_coil
feed 0 100 12 oritech:magnetic_coil
line=$(info 0 100 12)
read_info "$line"
expect IN0 "oritech:nickel_ingot x1"
expect IN1 "oritech:steel_ingot x1"
expect IN2 "oritech:magnetic_coil x1"
expect IN3 "oritech:magnetic_coil x1"
expect OUT0 empty
expect PROGRESS 0
echo "  four feeds, four slots: $IN0, $IN1, $IN2, $IN3"
say "oritech energy 0 100 12 $ASSEMBLER_LEFT"
ctl wait 40
line=$(info 0 100 12)
read_info "$line"
expect ASSEMBLED no
expect PROGRESS 0
expect RECIPE none
expect ENERGY "$ASSEMBLER_LEFT"
expect OUT0 empty
expect IN0 "oritech:nickel_ingot x1"
echo "  unassembled, with a full grid and $ENERGY FE: progress $PROGRESS, recipe $RECIPE, nothing made"

echo "== AP: three cores assemble it, and 48 ticks craft two motors for exactly 6,144 FE =="
# `getNextMissingCore` puts each held core in the next air position of the three,
# and the empty-hand click assembles once all three are cores.
ctl command "clear @s" >/dev/null
ctl wait 4
for tier in $ASSEMBLER_CORES; do
    ctl give "oritech:machine_core_$tier" 1 >/dev/null
    ctl wait 3
    ctl select "oritech:machine_core_$tier" >/dev/null
    ctl wait 2
    ctl command "execute in minecraft:overworld run tp @s 0.5 100 10.5" >/dev/null
    ctl wait 4
    ctl look-at-block 0 100 12 north >/dev/null
    ctl wait 2
    ctl interact 0 100 12 >/dev/null 2>&1 || true
    ctl wait 8
done
for offset in "0 100 13" "0 101 12" "0 101 13"; do
    read -r cx cy cz <<<"$offset"
    core_name=$(block_name "$cx" "$cy" "$cz")
    case "$core_name" in
        oritech:machine_core_*) ;;
        *) fail "the core clicks left $core_name at $cx,$cy,$cz, not a machine core" ;;
    esac
done
ctl command "clear @s" >/dev/null
ctl wait 6
ctl command "execute in minecraft:overworld run tp @s 0.5 100 10.5" >/dev/null
ctl wait 4
ctl look-at-block 0 100 12 north >/dev/null
ctl wait 2
echo "  assemble click: $(ctl interact 0 100 12 | jq -c '{ok, block: .block.block}')"
ctl wait 10
line=$(info 0 100 12)
read_info "$line"
expect ASSEMBLED yes
expect CORE "$ASSEMBLER_CORE_LIST"
expect QUALITY 2
expect IN0 "oritech:nickel_ingot x1"
# The 1,280 FE primed while it was unassembled is exactly ten of its ticks, so a
# machine that ticked while gated would already have spent it.
ctl wait 40
line=$(info 0 100 12)
read_info "$line"
expect PROGRESS 10
expect ENERGY 0
expect RECIPE oritech:assembler/motor
echo "  assembled with cores $CORE (average quality $QUALITY): 1,280 FE advanced the craft exactly 10 of $ASSEMBLER_TICKS ticks"
# The other 38 ticks, at 128 FE each.
say "oritech energy 0 100 12 $((ASSEMBLER_SPENT - ASSEMBLER_LEFT))"
ctl wait 70
line=$(info 0 100 12)
read_info "$line"
expect PROGRESS 0
expect RECIPE none
expect ENERGY 0
expect IN0 empty
expect IN1 empty
expect IN2 empty
expect IN3 empty
if [ "${SELFTEST_ASSEMBLER:-0}" = 1 ]; then
    echo "  SELFTEST_ASSEMBLER: expecting three motors where the recipe makes two (this must fail)"
    expect OUT0 "oritech:motor x3"
else
    expect OUT0 "oritech:motor x2"
fi
echo "  one motor craft: $OUT0 from four items in $ASSEMBLER_TICKS ticks for exactly $ASSEMBLER_SPENT FE, every input slot emptied"

echo "== AQ: the machine spends exactly the craft and leaves the rest of its buffer =="
# The same grid again, primed with 1,280 FE more than the craft needs: a machine
# that drained its buffer, or that kept spending while idle, fails this figure.
feed 0 100 12 oritech:nickel_ingot
feed 0 100 12 oritech:steel_ingot
feed 0 100 12 oritech:magnetic_coil
feed 0 100 12 oritech:magnetic_coil
line=$(info 0 100 12)
read_info "$line"
expect IN3 "oritech:magnetic_coil x1"
expect OUT0 "oritech:motor x2"
say "oritech energy 0 100 12 $((ASSEMBLER_SPENT + ASSEMBLER_LEFT))"
ctl wait 70
line=$(info 0 100 12)
read_info "$line"
expect PROGRESS 0
expect ENERGY "$ASSEMBLER_LEFT"
expect OUT0 "oritech:motor x4"
expect IN0 empty
expect IN1 empty
expect IN2 empty
expect IN3 empty
echo "  a second craft added two motors and spent exactly $ASSEMBLER_SPENT FE: buffer $ENERGY, out0 $OUT0"

echo "== AR: one coil short, the same machine makes nothing and spends nothing =="
# The differential: the same assembler, output room in the same slot, 10,000 FE
# — only the fourth ingredient is missing. `complexMatch` needs every ingredient,
# so no recipe resolves and `serverTick` returns before `useEnergy`.
feed 0 100 12 oritech:nickel_ingot
feed 0 100 12 oritech:steel_ingot
feed 0 100 12 oritech:magnetic_coil
line=$(info 0 100 12)
read_info "$line"
expect IN0 "oritech:nickel_ingot x1"
expect IN1 "oritech:steel_ingot x1"
expect IN2 "oritech:magnetic_coil x1"
expect IN3 empty
expect OUT0 "oritech:motor x4"
say "oritech energy 0 100 12 10000"
ctl wait 40
line=$(info 0 100 12)
read_info "$line"
expect PROGRESS 0
expect RECIPE none
expect ENERGY 10000
expect OUT0 "oritech:motor x4"
expect IN2 "oritech:magnetic_coil x1"
echo "  three of four ingredients: progress $PROGRESS, recipe $RECIPE, buffer still $ENERGY FE, out0 $OUT0"

echo "== AS: a slot holding two steel pays for one ingredient and keeps the spare =="
# The brief's grid: one nickel, *two* steel and two coils for four slots. The
# five items only fit because the second steel arrives when no slot is empty, so
# `findLowestMatchingSlot` merges it into the steel stack; the craft then takes
# one item out of each slot it names and leaves the spare behind. A fresh
# controller, because `setblock` of the same block keeps the old entity.
ctl command "setblock 0 100 12 minecraft:air" >/dev/null
ctl wait 4
ctl command "setblock 0 100 12 $ASSEMBLER" >/dev/null
ctl wait 8
ctl command "clear @s" >/dev/null
ctl wait 4
ctl command "execute in minecraft:overworld run tp @s 0.5 100 10.5" >/dev/null
ctl wait 4
ctl look-at-block 0 100 12 north >/dev/null
ctl wait 2
ctl interact 0 100 12 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 0 100 12)
read_info "$line"
expect ASSEMBLED yes
expect OUT0 empty
drain 0 100 12
feed 0 100 12 oritech:nickel_ingot
feed 0 100 12 oritech:steel_ingot
feed 0 100 12 oritech:magnetic_coil
feed 0 100 12 oritech:magnetic_coil
feed 0 100 12 oritech:steel_ingot
line=$(info 0 100 12)
read_info "$line"
expect IN0 "oritech:nickel_ingot x1"
expect IN1 "oritech:steel_ingot x2"
expect IN2 "oritech:magnetic_coil x1"
expect IN3 "oritech:magnetic_coil x1"
say "oritech energy 0 100 12 $ASSEMBLER_SPENT"
ctl wait 70
line=$(info 0 100 12)
read_info "$line"
expect PROGRESS 0
expect ENERGY 0
expect OUT0 "oritech:motor x2"
expect IN0 empty
expect IN1 "oritech:steel_ingot x1"
expect IN2 empty
expect IN3 empty
echo "  five items in four slots: $OUT0, the steel slot kept its spare ($IN1), and the craft cost exactly $ASSEMBLER_SPENT FE"

echo "== AT: the other four carried assembler recipes craft too =="
# Every carried recipe is crafted the way it declares: a fresh controller each
# time (the same block id keeps its entity, so the block is cleared first) with
# the same three cores, its own ingredients, and exactly `time x 128` FE.
assemble_case() { # assemble_case X Y Z TICKS RESULT COUNT ITEM...
    local x=$1 y=$2 z=$3 ticks=$4 result=$5 count=$6
    shift 6
    ctl command "setblock $x $y $z minecraft:air" >/dev/null
    ctl wait 4
    ctl command "setblock $x $y $z $ASSEMBLER" >/dev/null
    ctl wait 8
    [ "$(block_name "$x" "$y" "$z")" = "$ASSEMBLER" ] || fail "the assembler at $x,$y,$z did not place"
    ctl command "clear @s" >/dev/null
    ctl wait 4
    ctl command "execute in minecraft:overworld run tp @s $x.5 100 $((z - 2)).5" >/dev/null
    ctl wait 4
    ctl look-at-block "$x" "$y" "$z" north >/dev/null
    ctl wait 2
    ctl interact "$x" "$y" "$z" >/dev/null 2>&1 || true
    ctl wait 10
    local line
    line=$(info "$x" "$y" "$z")
    read_info "$line"
    expect ASSEMBLED yes
    expect OUT0 empty
    drain "$x" "$y" "$z"
    for item in "$@"; do feed "$x" "$y" "$z" "$item"; done
    say "oritech energy $x $y $z $((ticks * ASSEMBLER_RATE))"
    ctl wait $((ticks + 40))
    line=$(info "$x" "$y" "$z")
    read_info "$line"
    expect PROGRESS 0
    expect ENERGY 0
    expect IN0 empty
    expect IN1 empty
    expect IN2 empty
    expect IN3 empty
    expect OUT0 "$result x$count"
    echo "  $result x$count in exactly $ticks ticks for exactly $((ticks * ASSEMBLER_RATE)) FE"
}
# The three cores at (4,100,13), (4,101,12) and (4,101,13) serve every case.
ctl command "setblock 4 100 12 minecraft:air" >/dev/null
ctl command "setblock 4 100 13 minecraft:air" >/dev/null
ctl command "setblock 4 101 12 minecraft:air" >/dev/null
ctl command "setblock 4 101 13 minecraft:air" >/dev/null
ctl wait 4
ctl command "setblock 4 100 12 $ASSEMBLER" >/dev/null
ctl wait 8
ctl command "clear @s" >/dev/null
ctl wait 4
for tier in $ASSEMBLER_CORES; do
    ctl give "oritech:machine_core_$tier" 1 >/dev/null
    ctl wait 3
    ctl select "oritech:machine_core_$tier" >/dev/null
    ctl wait 2
    ctl command "execute in minecraft:overworld run tp @s 4.5 100 10.5" >/dev/null
    ctl wait 4
    ctl look-at-block 4 100 12 north >/dev/null
    ctl wait 2
    ctl interact 4 100 12 >/dev/null 2>&1 || true
    ctl wait 8
done
# `assembler/magnet`: steel + two nickel + copper -> six coils, 48 ticks.
assemble_case 4 100 12 48 oritech:magnetic_coil 6 \
    oritech:steel_ingot oritech:nickel_ingot oritech:nickel_ingot minecraft:copper_ingot
# `assembler/blazerod`: four blaze powder -> one blaze rod, 96 ticks.
assemble_case 4 100 12 96 minecraft:blaze_rod 1 \
    minecraft:blaze_powder minecraft:blaze_powder minecraft:blaze_powder minecraft:blaze_powder
# `assembler/fireball`: gunpowder + blaze powder + two coal -> four fire charges.
assemble_case 4 100 12 96 minecraft:fire_charge 4 \
    minecraft:gunpowder minecraft:blaze_powder minecraft:coal minecraft:coal
# `assembler/shroomlight`: a log + three glowstone -> one shroomlight.
assemble_case 4 100 12 96 minecraft:shroomlight 1 \
    minecraft:oak_log minecraft:glowstone minecraft:glowstone minecraft:glowstone
ctl command "setblock 4 100 12 minecraft:air" >/dev/null
ctl command "setblock 4 100 13 minecraft:air" >/dev/null
ctl command "setblock 4 101 12 minecraft:air" >/dev/null
ctl command "setblock 4 101 13 minecraft:air" >/dev/null
ctl command "setblock 0 100 12 minecraft:air" >/dev/null
ctl command "setblock 0 100 13 minecraft:air" >/dev/null
ctl command "setblock 0 101 12 minecraft:air" >/dev/null
ctl command "setblock 0 101 13 minecraft:air" >/dev/null
ctl wait 4

echo "== CA: an unassembled centrifuge with coal dust and 5,000 FE makes nothing =="
# Slice 9. `CentrifugeBlockEntity` is a `MultiblockMachineEntity` with one core at
# (0, 1, 0); `isActive(ASSEMBLED)` gates its whole tick. The input is the item the
# jar's own `centrifuge/carbon.json` names (`c:dusts/coal`, the pulverizer's coal
# dust), so a machine that ticked while unassembled would already be spending.
ctl command "setblock 8 100 10 minecraft:air" >/dev/null
ctl command "setblock 8 101 10 minecraft:air" >/dev/null
ctl command "setblock 8 100 9 minecraft:air" >/dev/null
ctl command "setblock 8 100 11 minecraft:air" >/dev/null
ctl wait 4
ctl command "setblock 8 100 10 $CENTRIFUGE" >/dev/null
ctl wait 8
[ "$(block_name 8 100 10)" = "$CENTRIFUGE" ] || fail "the centrifuge did not place"
line=$(info 8 100 10)
read_info "$line"
expect KIND centrifuge
expect ENERGY 0
expect CAPACITY "$CENTRIFUGE_CAPACITY"
expect ASSEMBLED no
expect CORE empty
expect TANK_FLUID empty
expect INPUT empty
expect OUT0 empty
expect OUT1 empty
drain 8 100 10
feed 8 100 10 oritech:coal_dust
line=$(info 8 100 10)
read_info "$line"
expect INPUT "oritech:coal_dust x1"
expect OUT0 empty
say "oritech energy 8 100 10 5000"
ctl wait 60
line=$(info 8 100 10)
read_info "$line"
expect ASSEMBLED no
expect PROGRESS 0
expect RECIPE none
expect ENERGY 5000
expect INPUT "oritech:coal_dust x1"
expect OUT0 empty
echo "  unassembled, with coal dust and 5,000 FE: progress $PROGRESS, recipe $RECIPE, $ENERGY FE kept, no carbon fibre"

echo "== CB: the core above assembles it, and 50 ticks make one carbon fibre for exactly 3,200 FE =="
ctl give "oritech:machine_core_$CENTRIFUGE_CORE_TIER" 1 >/dev/null
ctl wait 3
ctl select "oritech:machine_core_$CENTRIFUGE_CORE_TIER" >/dev/null
ctl wait 3
ctl command "execute in minecraft:overworld run tp @s 8.5 100 8.5" >/dev/null
ctl wait 6
ctl look-at-block 8 100 10 north >/dev/null
ctl wait 2
echo "  core click: $(ctl interact 8 100 10 | jq -c '{ok, block: .block.block}')"
ctl wait 8
CORE_BLOCK=$(block_name 8 101 10)
[ "$CORE_BLOCK" = "oritech:machine_core_$CENTRIFUGE_CORE_TIER" ] \
    || fail "the core click left $CORE_BLOCK above the centrifuge, not a machine core"
echo "  block(8,101,10) = $CORE_BLOCK (the centrifuge's core is above, the furnace's offset)"
# Empty the buffer before the empty-hand click: the assembly starts the craft at
# once, and a starved machine spends nothing, so priming after it is the only FE
# the operation can use.
say "oritech energy 8 100 10 0"
ctl wait 8
ctl command "clear @s" >/dev/null
ctl wait 6
ctl look-at-block 8 100 10 north >/dev/null
ctl wait 2
ctl interact 8 100 10 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 8 100 10)
read_info "$line"
expect ASSEMBLED yes
expect CORE "oritech:machine_core_$CENTRIFUGE_CORE_TIER"
expect QUALITY "$CENTRIFUGE_CORE_TIER"
expect INPUT "oritech:coal_dust x1"
expect ENERGY 0
ctl wait 40
line=$(info 8 100 10)
read_info "$line"
expect ASSEMBLED yes
expect ENERGY 0
expect PROGRESS 0
[ "$RECIPE" = "oritech:centrifuge/carbon" ] \
    || fail "the assembled centrifuge resolved '$RECIPE', not the jar's carbon recipe"
echo "  assembled with core $CORE quality $QUALITY: the starved machine spent nothing and holds $RECIPE"
say "oritech energy 8 100 10 $CENTRIFUGE_SPENT"
ctl wait 100
line=$(info 8 100 10)
read_info "$line"
expect PROGRESS 0
expect RECIPE none
expect ENERGY 0
expect INPUT empty
expect OUT1 empty
expect TANK_FLUID empty
if [ "${SELFTEST_CENTRIFUGE:-0}" = 1 ]; then
    echo "  SELFTEST_CENTRIFUGE: expecting two carbon fibre where the recipe makes one (this must fail)"
    expect OUT0 "oritech:carbon_fibre_strands x2"
else
    expect OUT0 "oritech:carbon_fibre_strands x1"
fi
echo "  one carbon craft: $OUT0 from one coal dust in exactly $CENTRIFUGE_TICKS ticks for exactly $CENTRIFUGE_SPENT FE, input emptied"

echo "== CC: a Fluid Addon plus only 100 mB of water is the wrong ratio: no clay, no FE =="
# The fluid table is gated behind the Fluid Addon (`hasFluidAddon`); `clay.json`
# needs 250 mB of water, so a tank holding 100 mB must not resolve the recipe and
# `serverTick` returns before `useEnergy`. The addon sits in the (0,0,-1) slot, so
# the clicks come from the south, whose slot is empty.
ctl command "setblock 12 100 10 minecraft:air" >/dev/null
ctl command "setblock 12 101 10 oritech:machine_core_2" >/dev/null
ctl command "setblock 12 100 9 $FLUID_ADDON" >/dev/null
ctl wait 4
ctl command "setblock 12 100 10 $CENTRIFUGE" >/dev/null
ctl wait 8
[ "$(block_name 12 100 10)" = "$CENTRIFUGE" ] || fail "the fluid centrifuge did not place"
[ "$(block_name 12 100 9)" = "$FLUID_ADDON" ] || fail "the Fluid Addon did not place in the north slot"
ctl command "clear @s" >/dev/null
ctl wait 4
ctl command "execute in minecraft:overworld run tp @s 12.5 100 12.5" >/dev/null
ctl wait 6
ctl look-at-block 12 100 10 south >/dev/null
ctl wait 2
ctl interact 12 100 10 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 12 100 10)
read_info "$line"
expect ASSEMBLED yes
expect CORE oritech:machine_core_2
expect ADDONS fluid
line=$(fluid_put 12 100 10 minecraft:water 100)
read_info "$line"
expect TANK_FLUID minecraft:water
expect TANK_AMOUNT 100
expect TANK_CAP "$CENTRIFUGE_TANK"
expect ACCEPTED 100
feed_south 12 100 10 minecraft:dirt
line=$(info 12 100 10)
read_info "$line"
expect INPUT "minecraft:dirt x1"
say "oritech energy 12 100 10 10000"
ctl wait 50
line=$(info 12 100 10)
read_info "$line"
expect PROGRESS 0
expect RECIPE none
expect ENERGY 10000
expect INPUT "minecraft:dirt x1"
expect TANK_AMOUNT 100
expect OUT0 empty
echo "  a tank of 100 mB (the recipe needs 250): progress $PROGRESS, recipe $RECIPE, buffer untouched at $ENERGY FE, dirt kept"

echo "== CD: 250 mB of water and 100 ticks make one clay for exactly 6,400 FE, tank emptied =="
line=$(fluid_put 12 100 10 minecraft:water 150)
read_info "$line"
expect TANK_AMOUNT 250
# The recipe resolves the moment the tank reaches 250 and the machine (which has
# 10,000 FE from CC) starts at once: take the buffer to zero and measure from
# where the craft stands, the chiller's phase-R pattern.
say "oritech energy 12 100 10 0"
ctl wait 8
line=$(info 12 100 10)
read_info "$line"
expect ENERGY 0
[ "$RECIPE" = "oritech:centrifuge/fluid/clay" ] \
    || fail "the charged tank resolved '$RECIPE', not the jar's clay recipe"
P_ZERO=$PROGRESS
[ "$P_ZERO" -lt "$CENTRIFUGE_FLUID_TICKS" ] || fail "the clay craft finished before the measurement"
say "oritech energy 12 100 10 $((CENTRIFUGE_RATE * 10))"
ctl wait 40
line=$(info 12 100 10)
read_info "$line"
expect ENERGY 0
[ "$PROGRESS" -eq "$((P_ZERO + 10))" ] \
    || fail "320 FE should be exactly 10 ticks of work: progress $P_ZERO -> $PROGRESS"
P_START=$PROGRESS
expect TANK_AMOUNT 250
echo "  320 FE advanced the fluid craft exactly 10 ticks ($P_ZERO -> $P_START of $CENTRIFUGE_FLUID_TICKS), tank still $TANK_AMOUNT mB"
NEEDED=$((CENTRIFUGE_RATE * (CENTRIFUGE_FLUID_TICKS - P_START)))
say "oritech energy 12 100 10 $NEEDED"
ctl wait $((CENTRIFUGE_FLUID_TICKS - P_START + 60))
line=$(info 12 100 10)
read_info "$line"
expect PROGRESS 0
expect RECIPE none
expect ENERGY 0
expect INPUT empty
expect TANK_FLUID empty
expect TANK_AMOUNT 0
expect OUT0 "minecraft:clay x1"
echo "  from progress $P_START: $((CENTRIFUGE_FLUID_TICKS - P_START)) ticks spent exactly $NEEDED FE, $OUT0, tank emptied"

echo "== CE: without the Fluid Addon the same tank and dirt make no clay =="
# The differential for the addon gate: a third centrifuge, assembled, a full
# 250 mB of water in its tank, dirt in its slot and 10,000 FE — but no Fluid Addon,
# so `getOwnRecipeType()` stays the item table and dirt names no recipe there.
ctl command "setblock 16 100 10 minecraft:air" >/dev/null
ctl command "setblock 16 101 10 oritech:machine_core_2" >/dev/null
ctl command "setblock 16 100 9 minecraft:air" >/dev/null
ctl wait 4
ctl command "setblock 16 100 10 $CENTRIFUGE" >/dev/null
ctl wait 8
[ "$(block_name 16 100 10)" = "$CENTRIFUGE" ] || fail "the third centrifuge did not place"
ctl command "clear @s" >/dev/null
ctl wait 4
ctl command "execute in minecraft:overworld run tp @s 16.5 100 8.5" >/dev/null
ctl wait 6
ctl look-at-block 16 100 10 north >/dev/null
ctl wait 2
ctl interact 16 100 10 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 16 100 10)
read_info "$line"
expect ASSEMBLED yes
expect ADDONS none
line=$(fluid_put 16 100 10 minecraft:water 250)
read_info "$line"
expect TANK_AMOUNT 250
feed 16 100 10 minecraft:dirt
line=$(info 16 100 10)
read_info "$line"
expect INPUT "minecraft:dirt x1"
say "oritech energy 16 100 10 10000"
ctl wait 60
line=$(info 16 100 10)
read_info "$line"
expect PROGRESS 0
expect RECIPE none
expect ENERGY 10000
expect INPUT "minecraft:dirt x1"
expect TANK_AMOUNT 250
expect OUT0 empty
echo "  no Fluid Addon: progress $PROGRESS, recipe $RECIPE, buffer still $ENERGY FE, dirt and water kept, no clay"
ctl command "setblock 8 100 10 minecraft:air" >/dev/null
ctl command "setblock 8 101 10 minecraft:air" >/dev/null
ctl command "setblock 12 100 10 minecraft:air" >/dev/null
ctl command "setblock 12 101 10 minecraft:air" >/dev/null
ctl command "setblock 12 100 9 minecraft:air" >/dev/null
ctl command "setblock 16 100 10 minecraft:air" >/dev/null
ctl command "setblock 16 101 10 minecraft:air" >/dev/null
ctl wait 4

echo "== CF: a full output slot blocks the craft: no coal dust and no FE are spent =="
# 0.2.13, the review fix. `MachineBlockEntity.canOutputRecipe` gates the whole
# tick, so with `out0` already holding a full stack of the result the machine must
# consume nothing and spend nothing. The output slot is filled by the harness
# control `/oritech slot` — the right click fills input slots only, and
# `ctl container` cannot see a mod block entity — and the readout is re-read after
# the write, so the setup itself is asserted.
ctl command "setblock 30 100 10 minecraft:air" >/dev/null
ctl command "setblock 30 101 10 minecraft:air" >/dev/null
ctl command "setblock 30 100 9 minecraft:air" >/dev/null
ctl wait 4
ctl command "setblock 30 101 10 oritech:machine_core_2" >/dev/null
ctl command "setblock 30 100 10 $CENTRIFUGE" >/dev/null
ctl wait 8
[ "$(block_name 30 100 10)" = "$CENTRIFUGE" ] || fail "the blocked centrifuge did not place"
ctl command "clear @s" >/dev/null
ctl wait 4
ctl command "execute in minecraft:overworld run tp @s 30.5 100 8.5" >/dev/null
ctl wait 6
ctl look-at-block 30 100 10 north >/dev/null
ctl wait 2
ctl interact 30 100 10 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 30 100 10)
read_info "$line"
expect ASSEMBLED yes
expect CORE oritech:machine_core_2
line=$(slot_put 30 100 10 1 64 oritech:carbon_fibre_strands)
read_info "$line"
expect SLOT 1
expect HELD "oritech:carbon_fibre_strands x64"
feed 30 100 10 oritech:coal_dust
line=$(info 30 100 10)
read_info "$line"
expect INPUT "oritech:coal_dust x1"
expect OUT0 "oritech:carbon_fibre_strands x64"
say "oritech energy 30 100 10 5000"
ctl wait 60
line=$(info 30 100 10)
read_info "$line"
expect ENERGY 5000
expect INPUT "oritech:coal_dust x1"
expect OUT1 empty
expect PROGRESS 0
expect RECIPE none
if [ "${SELFTEST_CENTRIFUGE_ROOM:-0}" = 1 ]; then
    echo "  SELFTEST_CENTRIFUGE_ROOM: expecting a full output slot to have taken the craft anyway (this must fail)"
    expect OUT0 "oritech:carbon_fibre_strands x1"
else
    expect OUT0 "oritech:carbon_fibre_strands x64"
fi
echo "  a full out0: after 60 ticks (the craft is 50) the input is still $INPUT, out0 still $OUT0, the buffer still $ENERGY FE, progress $PROGRESS"

echo "== CG: with room again the same recipe completes for exactly 3,200 FE =="
# The differential: the same machine, the same coal dust and the same 5,000 FE —
# only the output room changes.
line=$(slot_put 30 100 10 1 0 minecraft:air)
read_info "$line"
expect SLOT 1
expect HELD empty
ctl wait 80
line=$(info 30 100 10)
read_info "$line"
expect INPUT empty
expect OUT0 "oritech:carbon_fibre_strands x1"
expect OUT1 empty
expect PROGRESS 0
expect RECIPE none
expect ENERGY 1800
echo "  emptied out0: the coal dust became $OUT0 in exactly $CENTRIFUGE_TICKS ticks, buffer 5000 -> $ENERGY FE (exactly $CENTRIFUGE_SPENT)"

echo "== CH: a full tank and a full output slot: the fluid craft takes neither the dirt nor a millibucket =="
# The fluid half of the same rule: the tank is an input, so a full one changes
# nothing — the *output slot* is what cannot be installed, and the craft must take
# nothing from the tank either.
ctl command "setblock 33 100 10 minecraft:air" >/dev/null
ctl command "setblock 33 101 10 minecraft:air" >/dev/null
ctl command "setblock 33 100 9 minecraft:air" >/dev/null
ctl wait 4
ctl command "setblock 33 101 10 oritech:machine_core_2" >/dev/null
ctl command "setblock 33 100 9 $FLUID_ADDON" >/dev/null
ctl command "setblock 33 100 10 $CENTRIFUGE" >/dev/null
ctl wait 8
[ "$(block_name 33 100 10)" = "$CENTRIFUGE" ] || fail "the fluid blocked centrifuge did not place"
[ "$(block_name 33 100 9)" = "$FLUID_ADDON" ] || fail "the Fluid Addon did not place in the north slot"
ctl command "clear @s" >/dev/null
ctl wait 4
ctl command "execute in minecraft:overworld run tp @s 33.5 100 12.5" >/dev/null
ctl wait 6
ctl look-at-block 33 100 10 south >/dev/null
ctl wait 2
ctl interact 33 100 10 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 33 100 10)
read_info "$line"
expect ASSEMBLED yes
expect CORE oritech:machine_core_2
expect ADDONS fluid
line=$(fluid_put 33 100 10 minecraft:water "$CENTRIFUGE_TANK")
read_info "$line"
expect TANK_FLUID minecraft:water
expect TANK_AMOUNT "$CENTRIFUGE_TANK"
expect ACCEPTED "$CENTRIFUGE_TANK"
line=$(slot_put 33 100 10 1 64 minecraft:clay)
read_info "$line"
expect HELD "minecraft:clay x64"
feed_south 33 100 10 minecraft:dirt
line=$(info 33 100 10)
read_info "$line"
expect INPUT "minecraft:dirt x1"
expect OUT0 "minecraft:clay x64"
say "oritech energy 33 100 10 10000"
ctl wait 120
line=$(info 33 100 10)
read_info "$line"
expect ENERGY 10000
expect INPUT "minecraft:dirt x1"
expect TANK_AMOUNT "$CENTRIFUGE_TANK"
expect OUT0 "minecraft:clay x64"
expect PROGRESS 0
expect RECIPE none
echo "  a full $CENTRIFUGE_TANK mB tank and a full out0: after 120 ticks (the craft is 100) the dirt is still $INPUT, the tank still $TANK_AMOUNT mB, the buffer still $ENERGY FE"

echo "== CI: with room again the clay completes for exactly 6,400 FE and the tank drains 250 mB =="
line=$(slot_put 33 100 10 1 0 minecraft:air)
read_info "$line"
expect HELD empty
ctl wait 160
line=$(info 33 100 10)
read_info "$line"
expect INPUT empty
expect OUT0 "minecraft:clay x1"
expect TANK_AMOUNT 7750
expect RECIPE none
expect ENERGY 3600
echo "  emptied out0: $OUT0 for exactly $CENTRIFUGE_FLUID_SPENT FE, the tank $CENTRIFUGE_TANK -> $TANK_AMOUNT mB (250 drained)"
ctl command "setblock 30 100 10 minecraft:air" >/dev/null
ctl command "setblock 30 101 10 minecraft:air" >/dev/null
ctl command "setblock 33 100 10 minecraft:air" >/dev/null
ctl command "setblock 33 101 10 minecraft:air" >/dev/null
ctl command "setblock 33 100 9 minecraft:air" >/dev/null
ctl wait 4

echo "== FA: an unassembled foundry with a valid pair and 5,000 FE makes nothing =="
# Slice 10. `FoundryBlockEntity` is a `MultiblockMachineEntity` with three core
# positions; `isActive(ASSEMBLED)` gates its whole tick. The pair is the jar's
# own `foundry/alloy/steel.json` (`c:ingots/iron` + `c:dusts/coal`), so a machine
# that ticked while unassembled would already be spending.
ctl command "setblock 4 100 14 minecraft:air" >/dev/null
ctl command "setblock 4 100 15 minecraft:air" >/dev/null
ctl command "setblock 4 101 14 minecraft:air" >/dev/null
ctl command "setblock 4 101 15 minecraft:air" >/dev/null
ctl wait 4
ctl command "setblock 4 100 14 $FOUNDRY" >/dev/null
ctl wait 8
[ "$(block_name 4 100 14)" = "$FOUNDRY" ] || fail "the foundry did not place"
line=$(info 4 100 14)
read_info "$line"
expect KIND foundry
expect ENERGY 0
expect CAPACITY "$FOUNDRY_CAPACITY"
expect ASSEMBLED no
expect CORE empty
expect IN0 empty
expect IN1 empty
expect OUT0 empty
drain 4 100 14
feed 4 100 14 minecraft:iron_ingot
feed 4 100 14 oritech:coal_dust
line=$(info 4 100 14)
read_info "$line"
expect IN0 "minecraft:iron_ingot x1"
expect IN1 "oritech:coal_dust x1"
say "oritech energy 4 100 14 5000"
ctl wait 60
line=$(info 4 100 14)
read_info "$line"
expect ASSEMBLED no
expect PROGRESS 0
expect RECIPE none
expect ENERGY 5000
expect IN0 "minecraft:iron_ingot x1"
expect IN1 "oritech:coal_dust x1"
expect OUT0 empty
echo "  unassembled, with iron and coal dust and 5,000 FE: progress $PROGRESS, recipe $RECIPE, $ENERGY FE kept, no steel"

echo "== FB: three cores assemble it, and its quality is their average =="
# `getNextMissingCore` fills (0,0,1), (0,1,0) then (0,1,1): a core in the hand
# goes into the next empty position, and an empty-hand click assembles only when
# all three are there (`initMultiblock`).
ctl command "clear @s" >/dev/null
ctl wait 4
for tier in $FOUNDRY_CORES; do
    ctl give "oritech:machine_core_$tier" 1 >/dev/null
    ctl wait 3
    ctl select "oritech:machine_core_$tier" >/dev/null
    ctl wait 2
    ctl command "execute in minecraft:overworld run tp @s 4.5 100 12.5" >/dev/null
    ctl wait 4
    ctl look-at-block 4 100 14 north >/dev/null
    ctl wait 2
    ctl interact 4 100 14 >/dev/null 2>&1 || true
    ctl wait 8
done
[ "$(block_name 4 100 15)" = "oritech:machine_core_1" ] || fail "the first core did not land at 4,100,15"
[ "$(block_name 4 101 14)" = "oritech:machine_core_2" ] || fail "the second core did not land at 4,101,14"
[ "$(block_name 4 101 15)" = "oritech:machine_core_3" ] || fail "the third core did not land at 4,101,15"
ctl command "clear @s" >/dev/null
ctl wait 4
ctl interact 4 100 14 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 4 100 14)
read_info "$line"
expect ASSEMBLED yes
expect CORE "$FOUNDRY_CORE_LIST"
expect QUALITY 2
echo "  assembled with $CORE (quality $QUALITY)"

echo "== FC: two iron ingots are no alloy — the wrong-pair differential =="
# A fresh controller (the same block id keeps its block entity, so it is cleared
# first) reassembled from the same three cores. Both irons are inputs of a
# carried alloy, so both clicks are accepted; the pair is not one.
ctl command "setblock 4 100 14 minecraft:air" >/dev/null
ctl wait 4
ctl command "setblock 4 100 14 $FOUNDRY" >/dev/null
ctl wait 8
ctl command "clear @s" >/dev/null
ctl wait 4
ctl command "execute in minecraft:overworld run tp @s 4.5 100 12.5" >/dev/null
ctl wait 4
ctl look-at-block 4 100 14 north >/dev/null
ctl wait 2
ctl interact 4 100 14 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 4 100 14)
read_info "$line"
expect ASSEMBLED yes
drain 4 100 14
feed 4 100 14 minecraft:iron_ingot
feed 4 100 14 minecraft:iron_ingot
line=$(info 4 100 14)
read_info "$line"
expect IN0 "minecraft:iron_ingot x1"
expect IN1 "minecraft:iron_ingot x1"
say "oritech energy 4 100 14 5000"
ctl wait 60
line=$(info 4 100 14)
read_info "$line"
expect PROGRESS 0
expect RECIPE none
expect ENERGY 5000
expect IN0 "minecraft:iron_ingot x1"
expect IN1 "minecraft:iron_ingot x1"
expect OUT0 empty
echo "  two iron ingots: progress $PROGRESS, recipe $RECIPE, buffer untouched at $ENERGY FE, no steel"

echo "== FD: the steel alloy — 26 ticks, 3,328 FE, the pair in either slot =="
ctl command "setblock 4 100 14 minecraft:air" >/dev/null
ctl wait 4
ctl command "setblock 4 100 14 $FOUNDRY" >/dev/null
ctl wait 8
ctl command "clear @s" >/dev/null
ctl wait 4
ctl command "execute in minecraft:overworld run tp @s 4.5 100 12.5" >/dev/null
ctl wait 4
ctl look-at-block 4 100 14 north >/dev/null
ctl wait 2
ctl interact 4 100 14 >/dev/null 2>&1 || true
ctl wait 10
line=$(info 4 100 14)
read_info "$line"
expect ASSEMBLED yes
drain 4 100 14
# The file is `[c:ingots/iron, c:dusts/coal]`; feed them the other way round, so
# a slot-bound matcher would find no recipe.
feed 4 100 14 oritech:coal_dust
feed 4 100 14 minecraft:iron_ingot
line=$(info 4 100 14)
read_info "$line"
expect IN0 "oritech:coal_dust x1"
expect IN1 "minecraft:iron_ingot x1"
say "oritech energy 4 100 14 $FOUNDRY_SPENT"
ctl wait 8
line=$(info 4 100 14)
read_info "$line"
expect RECIPE oritech:foundry/alloy/steel
echo "  primed: recipe $RECIPE, progress $PROGRESS of $FOUNDRY_TICKS"
ctl wait $((FOUNDRY_TICKS + 40))
line=$(info 4 100 14)
read_info "$line"
expect PROGRESS 0
expect ENERGY 0
expect RECIPE none
expect IN0 empty
expect IN1 empty
if [ "${SELFTEST_FOUNDRY:-0}" = 1 ]; then
    echo "  SELFTEST_FOUNDRY: expecting two steel where the recipe makes one (this must fail)"
    expect OUT0 "oritech:steel_ingot x2"
fi
expect OUT0 "oritech:steel_ingot x1"
echo "  one steel in exactly $FOUNDRY_TICKS ticks for exactly $FOUNDRY_SPENT FE, both inputs emptied"

echo "== FE: the other three carried alloys each craft their declared output =="
# Every carried recipe is fed the way it declares, with a fresh controller each
# time (the same block id keeps its entity) and exactly `time x 128` FE.
foundry_case() { # foundry_case X Y Z TICKS RESULT COUNT RECIPE ITEM...
    local x=$1 y=$2 z=$3 ticks=$4 result=$5 count=$6 recipe=$7
    shift 7
    ctl command "setblock $x $y $z minecraft:air" >/dev/null
    ctl wait 4
    ctl command "setblock $x $y $z $FOUNDRY" >/dev/null
    ctl wait 8
    [ "$(block_name "$x" "$y" "$z")" = "$FOUNDRY" ] || fail "the foundry at $x,$y,$z did not place"
    ctl command "clear @s" >/dev/null
    ctl wait 4
    ctl command "execute in minecraft:overworld run tp @s $x.5 100 $((z - 2)).5" >/dev/null
    ctl wait 4
    ctl look-at-block "$x" "$y" "$z" north >/dev/null
    ctl wait 2
    ctl interact "$x" "$y" "$z" >/dev/null 2>&1 || true
    ctl wait 10
    local line
    line=$(info "$x" "$y" "$z")
    read_info "$line"
    expect ASSEMBLED yes
    expect OUT0 empty
    drain "$x" "$y" "$z"
    for item in "$@"; do feed "$x" "$y" "$z" "$item"; done
    say "oritech energy $x $y $z $((ticks * FOUNDRY_RATE))"
    ctl wait 8
    line=$(info "$x" "$y" "$z")
    read_info "$line"
    expect RECIPE "$recipe"
    ctl wait $((ticks + 40))
    line=$(info "$x" "$y" "$z")
    read_info "$line"
    expect PROGRESS 0
    expect RECIPE none
    expect ENERGY 0
    expect IN0 empty
    expect IN1 empty
    expect OUT0 "$result x$count"
    echo "  $result x$count in exactly $ticks ticks for exactly $((ticks * FOUNDRY_RATE)) FE"
}
# `foundry/alloy/adamant`: a diamond + a nickel -> one adamant ingot, 80 ticks.
foundry_case 4 100 14 80 oritech:adamant_ingot 1 oritech:foundry/alloy/adamant \
    minecraft:diamond oritech:nickel_ingot
# `foundry/alloy/electrum`: a gold + a redstone -> one electrum ingot, 80 ticks.
foundry_case 4 100 14 80 oritech:electrum_ingot 1 oritech:foundry/alloy/electrum \
    minecraft:gold_ingot minecraft:redstone
# `foundry/alloy/netherite`: a gold + a netherite scrap -> one netherite ingot, 80 ticks.
foundry_case 4 100 14 80 minecraft:netherite_ingot 1 oritech:foundry/alloy/netherite \
    minecraft:gold_ingot minecraft:netherite_scrap
ctl command "setblock 4 100 14 minecraft:air" >/dev/null
ctl command "setblock 4 100 15 minecraft:air" >/dev/null
ctl command "setblock 4 101 14 minecraft:air" >/dev/null
ctl command "setblock 4 101 15 minecraft:air" >/dev/null
ctl wait 4

echo "== W: every carried loot table rolls its own block =="
# The carried `data/oritech/loot_table/blocks/*.json` are the jar's self-drop,
# rewritten into 26.3's singular `condition`. Rolling each table directly asserts
# the table loads and its entry resolves; a missing table answers an error and the
# item never arrives.
ctl command "clear @s" >/dev/null
ctl wait 4
for block in "$GENERATOR" "$PULVERIZER" "$FURNACE" "$COOLER" "$LAVA" "$ASSEMBLER" \
    "$CENTRIFUGE" "$FOUNDRY" \
    oritech:machine_core_1 oritech:machine_core_2 oritech:machine_core_3 \
    oritech:machine_core_4 oritech:machine_core_5 oritech:machine_core_6 \
    oritech:machine_core_7 \
    "$SPEED_ADDON" "$EFFICIENCY_ADDON" "$PROCESSING_ADDON" "$CAPACITOR_ADDON" \
    "$BURST_ADDON" "$ULTIMATE_ADDON" "$REDSTONE_ADDON" "$FLUID_ADDON" \
    oritech:machine_plating_block oritech:iron_plating_block \
    oritech:nickel_plating_block; do
    ctl command "loot give @s loot oritech:blocks/${block#oritech:}" >/dev/null
done
ctl wait 8
INV=$(ctl inventory)
for block in "$GENERATOR" "$PULVERIZER" "$FURNACE" "$COOLER" "$LAVA" "$ASSEMBLER" \
    "$CENTRIFUGE" "$FOUNDRY" \
    oritech:machine_core_1 oritech:machine_core_7 \
    "$SPEED_ADDON" "$EFFICIENCY_ADDON" "$PROCESSING_ADDON" "$CAPACITOR_ADDON" \
    "$BURST_ADDON" "$ULTIMATE_ADDON" "$REDSTONE_ADDON" "$FLUID_ADDON" \
    oritech:machine_plating_block oritech:iron_plating_block \
    oritech:nickel_plating_block; do
    jq -e --arg item "$block" \
        '[.hotbar[], .main[] | select(.item == $item)] | length > 0' <<<"$INV" >/dev/null \
        || fail "loot table oritech:blocks/${block#oritech:} gave nothing: $(jq -c '{hotbar, main}' <<<"$INV")"
done
echo "  every carried loot table rolled its block into the inventory"
# A table that does not exist must give nothing: the assertion above is not
# satisfied by any `/loot` call, only by a table that resolves.
BEFORE=$(jq '[.hotbar[], .main[]] | length' <<<"$INV")
ctl command "loot give @s loot oritech:blocks/no_such_block" >/dev/null 2>&1 || true
ctl wait 6
AFTER=$(ctl inventory | jq '[.hotbar[], .main[]] | length')
[ "$BEFORE" = "$AFTER" ] || fail "an unknown loot table changed the inventory ($BEFORE -> $AFTER)"
echo "  an unknown loot table gave nothing (inventory entries $BEFORE -> $AFTER)"

echo "PASS: an unfuelled generator made no FE, a pulverizer with no power kept its bone, one coal click started the burn at 32 FE a tick, bone became exactly 6 bone meal in exactly 100 ticks for exactly 3,200 FE, the adjacent pulverizer ran on the generator's FE, stone was refused by both machines, the machines survived a relaunch, a powered furnace with no core smelted nothing even with ore and 5,000 FE in it, its core arrived by a right click and assembly gated it, 320 FE advanced a smelt exactly ten ticks, iron ore became an iron ingot for exactly 3,200 FE, breaking the core disassembled the machine mid-smelt and kept its progress and reassembly resumed it, every new block placed and read back, every one of the seven carried pulverizer recipes crafted with its input consumed, an industrial chiller with no core froze nothing even with a bucket of water and 5,000 FE, its core assembled it one block to the north, a bucket of water entered the tank through the block's own declared fluid handler, 320 FE advanced the freeze exactly ten ticks, water became exactly 3 ice for exactly 6,400 FE with the tank emptied, lava became exactly 2 obsidian, a mod fluid was refused, only the chiller had a tank, a lava generator with an empty tank made no FE and one with no core burned the lava in its tank for nothing, its core assembled it one block above, 100 mB of lava entered its tank through the block's own declared fluid handler and left it when the burn loaded, a 120-tick burn made exactly 7,680 FE at exactly 64 a tick, its ten-target push handed that whole 7,680 FE to an adjacent pulverizer which spent it on a bone, an unassembled one pushed nothing until its core came back, a tank of water made no FE, a fluid this pack does not have was refused, all seven default addon blocks placed and read back and an undeclared one was refused, a speed addon made the pulverizer finish a bone in exactly 67 ticks for exactly 3,886 FE while the same machine without it took 100 ticks and 3,200, an efficiency addon made the same craft cost exactly 2,700 FE, a capacitor addon raised the buffer to 2,025,000 FE and held both a 25,000 and a 1,000,000 write before clamping back, a charged burst addon crafted in exactly 13 ticks for 416 FE, a processing addon turned two bones into twelve bone meal in one completion for 4,800 FE, the furnace, chiller and lava generator each ran their own addon slot for exactly 3,886, 7,772 and 7,590 FE, all seven addon blocks placed and read back, a Control Unit Addon beside a redstone block held a pulverizer's whole tick for 40 ticks and left its bone, its recipe and exactly 10,000 FE untouched, removing the signal let the same machine finish the bone for exactly 3,200 FE, all nineteen carried crafting recipes crafted in a real table with their ingredients consumed and the generator's grid one nickel short crafting nothing, raw silicon smelted into silicon in exactly 30 ticks for exactly 960 FE, an assembler with a full motor grid and 1,280 FE but no core crafted nothing, its three cores assembled it and exactly 6,144 FE made two motors in 48 ticks with every input slot emptied, a second craft spent exactly 6,144 and left the extra 1,280 in its buffer, one coil short it made and spent nothing, a slot holding two steel paid for one ingredient and kept the spare, and all five carried assembler recipes crafted their declared output for their own exact cost, and every carried loot table rolled its own block, an unassembled centrifuge with coal dust and 5,000 FE made no carbon fibre and spent nothing, its core above assembled it and 50 ticks turned one coal dust into exactly one carbon fibre strand for exactly 3,200 FE with the input emptied, a Fluid Addon and a tank of only 100 mB of water made no clay and spent no FE (the recipe needs 250), the same machine with 250 mB made exactly one clay in 100 ticks for exactly 6,400 FE and emptied its tank, and the same water and dirt without the Fluid Addon made nothing, a foundry with a valid pair and 5,000 FE but no core made nothing, its three cores assembled it with their average quality, two iron ingots made no alloy and spent no FE, one coal dust and one iron in the slots opposite the file's order became exactly one steel ingot in exactly 26 ticks for exactly 3,328 FE with both inputs emptied, and its adamant, electrum and netherite alloys each crafted their declared output for exactly 10,240 FE, a centrifuge whose output slot was already full of the result neither took its coal dust nor spent a single FE in 60 ticks and the same machine with the slot emptied again made one carbon fibre for exactly 3,200 FE, a full 8,000 mB tank and a full clay slot took neither the dirt nor a millibucket and the same machine with the slot emptied made one clay and drained exactly 250 mB, and every carried loot table rolled its own block"
