#!/usr/bin/env bash
# In-game check for Oritech slices 1 to 4: the Basic Generator, the Pulverizer,
# the Powered Furnace multiblock (controller + one machine core), the
# Industrial Chiller (a second multiblock, with a fluid tank) and the Lava
# Generator (a fluid-burning generator with its own ten-target energy push).
#
# STATUS: 0.2.3 (SDK package, pin 0a875da). The run needs a game the launcher
# built with this package: `ports/tools/launcher-game.sh build oritech`, then
# `eval "$(ports/tools/launcher-game.sh env oritech)"` and this script in a
# build-slot lease. The 0.1.0 and 0.2.x runs are the historical evidence for
# the slice-1 behaviour; every run of this file is the oracle for all slices.
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
#    that does not exist gives nothing.
#
# CONTROL=1 (with CONTROL_GAME set) launches the package-less build first, where
# every id must be unknown: that is the differential run, and without it the
# scenario proves only that the ids exist.
#
# SELFTEST=1 flips the exact-FE assertion of each machine leg (it must FAIL),
# proving the scenario can fail; `SELFTEST_COOLER=1` does the same for the
# chiller's leg on its own, `SELFTEST_LAVA=1` for the lava generator's.
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
    "$CTL" --socket "$SOCK" launch \
        --game "$game" --world "$world" --seed 20261105 --creative --headless --wait \
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

# A stone floor at y=99 with clear air above, in the Overworld, and the player
# standing on it at (0.5, 100, 4.5).
arena() {
    ctl tp 0.5 101 4.5 >/dev/null
    ctl wait 20
    ctl command "fill -8 99 -8 24 99 24 minecraft:stone" >/dev/null
    ctl command "fill -8 100 -8 24 110 24 minecraft:air" >/dev/null
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
    ctl wait 4
    local furnace core chiller lava
    furnace=$(block_name 8 100 0)
    core=$(block_name 8 101 0)
    chiller=$(block_name 12 100 0)
    lava=$(block_name 20 100 6)
    [ "$furnace" = "minecraft:air" ] || fail "the control game holds $furnace, so the furnace id was known"
    [ "$core" = "minecraft:air" ] || fail "the control game holds $core, so the core id was known"
    [ "$chiller" = "minecraft:air" ] || fail "the control game holds $chiller, so the chiller id was known"
    [ "$lava" = "minecraft:air" ] || fail "the control game holds $lava, so the lava generator id was known"
    echo "  without this package: no generator item, and the generator, furnace, core, chiller and lava generator blocks all stayed $name"
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
    minecraft:glowstone minecraft:coal_ore minecraft:diamond_ore minecraft:lapis_ore)
RECIPE_EXPECT=("minecraft:bone_meal x6" "minecraft:blaze_powder x3" \
    "minecraft:pointed_dripstone x4" "minecraft:glowstone_dust x3" "minecraft:coal x2" \
    "minecraft:diamond x1" "minecraft:lapis_lazuli x6")
RECIPE_IDS=(bone blaze dripstone glowstoneore coalore diamondore lapisore)
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

echo "== W: every carried loot table rolls its own block =="
# The ten carried `data/oritech/loot_table/blocks/*.json` are the jar's self-drop,
# rewritten into 26.3's singular `condition`. Rolling each table directly asserts
# the table loads and its entry resolves; a missing table answers an error and the
# item never arrives.
ctl command "clear @s" >/dev/null
ctl wait 4
for block in "$GENERATOR" "$PULVERIZER" "$FURNACE" "$COOLER" "$LAVA" \
    oritech:machine_core_1 oritech:machine_core_2 oritech:machine_core_3 \
    oritech:machine_core_4 oritech:machine_core_5 oritech:machine_core_6 \
    oritech:machine_core_7; do
    ctl command "loot give @s loot oritech:blocks/${block#oritech:}" >/dev/null
done
ctl wait 8
INV=$(ctl inventory)
for block in "$GENERATOR" "$PULVERIZER" "$FURNACE" "$COOLER" "$LAVA" \
    oritech:machine_core_1 oritech:machine_core_7; do
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

echo "PASS: an unfuelled generator made no FE, a pulverizer with no power kept its bone, one coal click started the burn at 32 FE a tick, bone became exactly 6 bone meal in exactly 100 ticks for exactly 3,200 FE, the adjacent pulverizer ran on the generator's FE, stone was refused by both machines, the machines survived a relaunch, a powered furnace with no core smelted nothing even with ore and 5,000 FE in it, its core arrived by a right click and assembly gated it, 320 FE advanced a smelt exactly ten ticks, iron ore became an iron ingot for exactly 3,200 FE, breaking the core disassembled the machine mid-smelt and kept its progress and reassembly resumed it, every new block placed and read back, every one of the seven carried pulverizer recipes crafted with its input consumed, an industrial chiller with no core froze nothing even with a bucket of water and 5,000 FE, its core assembled it one block to the north, a bucket of water entered the tank through the block's own declared fluid handler, 320 FE advanced the freeze exactly ten ticks, water became exactly 3 ice for exactly 6,400 FE with the tank emptied, lava became exactly 2 obsidian, a mod fluid was refused, only the chiller had a tank, a lava generator with an empty tank made no FE and one with no core burned the lava in its tank for nothing, its core assembled it one block above, 100 mB of lava entered its tank through the block's own declared fluid handler and left it when the burn loaded, a 120-tick burn made exactly 7,680 FE at exactly 64 a tick, its ten-target push handed that whole 7,680 FE to an adjacent pulverizer which spent it on a bone, an unassembled one pushed nothing until its core came back, a tank of water made no FE, a fluid this pack does not have was refused, and every carried loot table rolled its own block"
