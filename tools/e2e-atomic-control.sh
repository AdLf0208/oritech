#!/usr/bin/env bash
# Focused slice-12 differential: the package-less build only.
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
# Slice 11: `OritechConfig.processingMachines.fragmentForgeData` (50,000 FE,
# 2,048 insert, 256 FE a tick) and the jar's `grinder/ore/iron.json`:
# `#minecraft:iron_ores` -> 2 raw iron + 1 raw nickel in 40 ticks, 10,240 FE.
# The raw nickel smelts in the powered furnace: `cookingtime: 200` at 0.5 speed
# = 100 ticks x 32 FE = exactly 3,200 FE for one `oritech:nickel_ingot`.
GRINDER=oritech:fragment_forge_block
GRINDER_CAPACITY=50000
GRINDER_RATE=256
GRINDER_TICKS=40
GRINDER_SPENT=$((GRINDER_TICKS * GRINDER_RATE))
GRINDER_CORES="1 2 3 4 5 6 7"
GRINDER_CORE_LIST="oritech:machine_core_1+oritech:machine_core_2+oritech:machine_core_3+oritech:machine_core_4+oritech:machine_core_5+oritech:machine_core_6+oritech:machine_core_7"
NICKEL_TICKS=100
NICKEL_SPENT=$((NICKEL_TICKS * FURNACE_RATE))
# Slice 12: `OritechConfig.processingMachines.atomicForgeData` (1,024 default, 0
# insert, 0 extract, 1,024 FE per tick) and the jar's `atomicforge/wafer.json`:
# `c:carbon_fibre` + 2 x `c:silicon` -> 1 silicon wafer in 5 ticks. Upstream
# resizes the storage to `rate x time`; the package declares that active 5,120.
# `hasEnoughEnergy()` needs the buffer FULL, so 5,119 must never craft.
ATOMIC=oritech:atomic_forge_block
ATOMIC_DEFAULT_CAPACITY=1024
ATOMIC_RATE=1024
ATOMIC_TICKS=5
ATOMIC_CHARGE=$((ATOMIC_RATE * ATOMIC_TICKS))
ATOMIC_SHORT=$((ATOMIC_CHARGE - 1))
# The forge has eight core *positions* but the mod ships only seven core
# *types*, so position eight reuses a tier-2 core (upstream does the same).
ATOMIC_CORES="1 2 3 4 5 6 7 2"
ATOMIC_CORE_LIST="oritech:machine_core_1+oritech:machine_core_2+oritech:machine_core_3+oritech:machine_core_4+oritech:machine_core_5+oritech:machine_core_6+oritech:machine_core_7+oritech:machine_core_2"

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

block_name() { ctl block "$1" "$2" "$3" 2>/dev/null | jq -r '.name // .block // empty' || true; }
# `ctl command "setblock …"` occasionally answers "Could not set the block" at
# this pin (measured: 8 times in one long run). Retry until the block actually
# reads back, so a transient harness failure cannot be mistaken for a lost id.
setblock_checked() { # setblock_checked X Y Z ID
    local x=$1 y=$2 z=$3 id=$4 i last
    for i in $(seq 1 6); do
        ctl command "setblock $x $y $z $id" >/dev/null 2>&1 || true
        ctl wait 4
        last=$(block_name "$x" "$y" "$z")
        [ "$last" = "$id" ] && return 0
    done
    fail "setblock $id at $x,$y,$z never took (last: $last)"
}
# A mod block entity keeps its slots in the mod's own view, which `ctl container` (the
# engine's vanilla container map) does not reach; the machine's own `/oritech info` line
# is the readout, so these two helpers parse it (`read_info` sets FUEL/INPUT/OUT0/OUT1).
machine_holds() { # machine_holds X Y Z ITEM
    local line field
    line=$(info "$1" "$2" "$3")
    read_info "$line"
    for field in FUEL INPUT OUT0 OUT1 OUT2; do
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
    # Slice 11: the Fragment Forge block and the raw nickel it produces.
    ctl command "setblock 20 100 18 $GRINDER" >/dev/null 2>&1 || true
    ctl command "item replace entity @s hotbar.8 with oritech:raw_nickel 1" >/dev/null 2>&1 || true
    # Slice 12: the Atomic Forge and the wafer it makes. Without this package
    # both ids must stay unknown.
    ctl command "setblock 28 100 20 $ATOMIC" >/dev/null 2>&1 || true
    ctl command "item replace entity @s hotbar.9 with oritech:silicon_wafer 1" >/dev/null 2>&1 || true
    ctl wait 4
    local furnace core chiller lava addon capacitor control plating motor assembler adamant
    local centrifuge fluidaddon coaldust carbonfibre foundry steel rawnickel grinder
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
    rawnickel=$(ctl inventory | jq -r '[.hotbar[], .main[] | select(.item == "oritech:raw_nickel")] | length')
    grinder=$(block_name 20 100 18)
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
    [ "$rawnickel" = 0 ] || fail "the control game holds $rawnickel oritech:raw_nickel, so that item id was known"
    local atomic wafer
    name=$(block_name 28 100 20)
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name at the Atomic Forge, so the block id was known"
    wafer=$(ctl inventory | jq -r '[.hotbar[], .main[] | select(.item == "oritech:silicon_wafer")] | length')
    [ "$wafer" = 0 ] || fail "the control game holds $wafer oritech:silicon_wafer, so that item id was known"
    [ "$grinder" = "minecraft:air" ] || fail "the control game holds $grinder, so the fragment forge id was known"
    echo "  without this package: no generator item, no motor, no adamant ingot, no coal dust, no carbon fibre, no steel ingot and no raw nickel, and the generator, furnace, core, chiller, lava generator, addon, control unit, plating, assembler, centrifuge, fluid addon, foundry and fragment forge blocks all stayed $name"
    ctl quit >/dev/null 2>&1 || true
}


control_leg

echo "PASS: the package-less build refused every Atomic Forge and silicon wafer id"
