#!/usr/bin/env bash
# In-game check for Oritech slice 1: the Basic Generator and the Pulverizer.
#
# The arena is the Overworld at y=100 (a stone floor, clear air above), so every
# block read and right click is in the player's own dimension (`ctl tp` is
# reliable there; cross-dimension `execute in ... run tp` left the player in the
# void in early runs of this campaign).
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
#  - the machines keep their FE/burn/output across `save` + quit + relaunch.
#
# EMBEDDED=1 launches with no `--dev-mod`: the bundle carries its own content.
# CONTROL=1 (default with EMBEDDED) first launches the same binary with
# `.horizon-cache` unusable, where the ids must be unknown.
#
# SELFTEST=1 flips the exact-FE assertion (it must FAIL), proving the scenario
# can fail.
set -Eeuo pipefail
export LC_ALL=C

CLONE=${CLONE:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/oritech
EMBEDDED=${EMBEDDED:-0}
CONTROL=${CONTROL:-$EMBEDDED}
DEV_MOD=()
[ "$EMBEDDED" = 1 ] || DEV_MOD=(--dev-mod "$PORT/content")

PIN_BIN=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$PIN_BIN/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/oritech-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=OritechE2E
CONTROL_WORLD=OritechControl

GENERATOR=oritech:basic_generator_block
PULVERIZER=oritech:pulverizer_block
# The jar's numbers (`FINDINGS.md`).
GENERATOR_RATE=32
ROOM_CAPACITY=25000
RECIPE_TICKS=100
PRIMED=10000
SPENT=$((GENERATOR_RATE * RECIPE_TICKS))

command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; exit 1; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/oritech-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
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

launch() { # launch [WORLD]
    local world=${1:-$WORLD}
    rm -f "$SOCK"
    cd "$SCRATCH"
    if [ "$EMBEDDED" = 1 ]; then
        echo "  launch: no --dev-mod; the bundle carries its own content" >&2
    fi
    "$CTL" --socket "$SOCK" launch \
        --game "$GAME" --world "$world" --seed 20261105 --creative --headless --wait \
        --resources "$RES" "${DEV_MOD[@]}" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }
slots() { ctl container "$1" "$2" "$3" | jq -c '[.slots[] | select(. != null)]'; }
has() { # has X Y Z ITEM COUNT
    local got
    got=$(slots "$1" "$2" "$3")
    jq -e --arg item "$4" --argjson count "$5" \
        '[.[] | select(.item == $item) | .count] | add // 0 | . == $count' <<<"$got" >/dev/null \
        || fail "expected $5 x $4 at $1,$2,$3; got $got"
    echo "  has($1,$2,$3) $4 x$5: ok"
}
has_not() { # has_not X Y Z ITEM
    local got
    got=$(slots "$1" "$2" "$3")
    jq -e --arg item "$4" '[.[] | select(.item == $item)] | length == 0' <<<"$got" >/dev/null \
        || fail "did not expect $4 at $1,$2,$3; got $got"
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

# The control leg for EMBEDDED=1: the same binary (every hook is in it) with the
# embedded content made unavailable — `.horizon-cache` is a plain file, so the
# helper can neither compare nor write its content and falls back to `Continue`.
# Both ids must then be unknown, or the run proves nothing about the bundle.
control_leg() {
    echo "== control: the same game with no embedded content file =="
    rm -rf "$SCRATCH/.horizon-cache"
    : > "$SCRATCH/.horizon-cache"
    launch "$CONTROL_WORLD"
    arena
    ctl command "item replace entity @s hotbar.0 with $GENERATOR 1" >/dev/null || true
    ctl wait 4
    ctl inventory | jq -e --arg item "$GENERATOR" \
        '[.hotbar[], .main[] | select(.item == $item)] | length == 0' >/dev/null \
        || fail "the control game has the generator item, so the content did not come from the embedded file"
    ctl command "setblock 0 100 0 $GENERATOR" >/dev/null || true
    ctl wait 4
    local name
    name=$(block_name 0 100 0)
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the block id was known"
    [ -f "$SCRATCH/.horizon-cache" ] || fail "the control's .horizon-cache was replaced: the helper wrote its content"
    echo "  without the embedded file: no generator item, the block stayed $name"
    ctl quit >/dev/null 2>&1 || true
    rm -f "$SCRATCH/.horizon-cache"
}

# EMBEDDED=1: the bundle must have written its content out at startup, byte for
# byte the delivered content mod.
embedded_content_check() {
    local cache=$SCRATCH/.horizon-cache/oritech-embedded-content.hmod
    [ -f "$cache" ] || fail "the bundle did not write its embedded content to $cache"
    cmp -s "$cache" "$PORT/content/oritech.hmod" \
        || fail "the embedded content differs from content/oritech.hmod"
    echo "  embedded content: $(wc -c < "$cache" | tr -d ' ') B at $cache, identical to content/oritech.hmod"
}

echo "== launch (Overworld arena at y=100, content + native bundle) =="
if [ "$CONTROL" = 1 ]; then control_leg; fi
launch
if [ "$EMBEDDED" = 1 ]; then embedded_content_check; fi
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
ctl container 4 100 0 minecraft:bone 1 >/dev/null
ctl wait 6
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
ctl container 1 100 0 minecraft:bone 1 >/dev/null
ctl wait 6
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
has 4 100 0 minecraft:bone_meal 6
line=$(info 4 100 0)
read_info "$line"
expect OUT0 "minecraft:bone_meal x6"
line=$(info 1 100 0)
read_info "$line"
expect OUT0 "minecraft:bone_meal x6"

echo "PASS: an unfuelled generator made no FE, a pulverizer with no power kept its bone, one coal click started the burn at 32 FE a tick, bone became exactly 6 bone meal in exactly 100 ticks for exactly 3,200 FE, the adjacent pulverizer ran on the generator's FE, stone was refused by both machines, and the machines survived a relaunch"
