# Oritech for Horizon — slices 1 to 6

The **Basic Generator**, the **Pulverizer**, the **Powered Furnace** multiblock, the **Industrial
Chiller**, the **Lava Generator**, the **machine addon system** and the **Control Unit Addon** of
[Oritech 1.2.13](https://github.com/Rearth/Oritech) (CC0-1.0), rewritten in Rust for Horizon at the
campaign pin `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (mod API 0.2, 26.3).

One flat `.hmod` carries both halves — `src/lib.rs` (the one `impl Hooks`), the five block-entity
Parts and the rules in `src/runtime.rs`, beside the carried `assets/` and `data/` (blocks, items,
models, textures, recipes, loot, lang, tags).

## What it does

- **Basic Generator** (`oritech:basic_generator_block`) — burns any fuel the
  game's fuel table knows, making **32 FE a tick** while it burns, into a
  **50,000 FE** buffer that accepts no inserts and gives up to **256 FE** a
  call. The buffer pauses the burn when it has no room for a full tick, as
  upstream's `canFitEnergy` does. One coal is 1,600 burning ticks.
- **Pulverizer** (`oritech:pulverizer_block`) — a **25,000 FE** machine that
  spends **32 FE a tick** while it works and turns one input into a recipe's
  results in the recipe's `time` ticks. The recipes are the jar's own
  `oritech:pulverizer` files: bone → 6 bone meal, blaze rod → 3 blaze powder,
  dripstone block → 4 pointed dripstone, glowstone → 3 glowstone dust,
  `#minecraft:coal_ores` → 2 coal, `#minecraft:diamond_ores` → 1 diamond,
  `#minecraft:lapis_ores` → 6 lapis lazuli (all 100 ticks).
- **The energy link** — a fuelled generator pushes its FE to the machines
  beside it (`move_energy`, 256 FE a call), so a machine next to a burner runs
  and one with no source idles with its input untouched.
- **Powered Furnace** (`oritech:powered_furnace_block`) — the mod's first
  multiblock: a controller plus one of the seven machine cores **one block
  above** it. Assembled, it smelts any vanilla smelting recipe at twice a
  vanilla furnace's speed (**100 ticks** for a 200-tick recipe) for **32 FE a
  tick** into a **10,000 FE** buffer. Breaking the core disassembles it and
  keeps the operation's progress; an unassembled machine does nothing.
- **Industrial Chiller** (`oritech:cooler_block`) — the mod's first fluid
  machine: a controller plus a core **one block to the north**, and one
  **four-bucket tank** a pipe (or `/oritech fluid`) fills. A bucket of
  `minecraft:water` becomes **3 ice**, a bucket of `minecraft:lava` **2
  obsidian**, in **200 ticks** for exactly **6,400 FE** (32 FE a tick) into a
  **50,000 FE** buffer. The tank is a real `horizon:fluids` handler, so another
  mod's pipe can fill it.
- **Lava Generator** (`oritech:lava_generator_block`) — the first generator
  that burns a fluid: a controller plus a core **one block above**, a
  **four-bucket tank** a bucket (or `/oritech fluid`) fills, and **100 mB of
  `minecraft:lava`** burnt over **120 ticks** for exactly **7,680 FE** (64 FE a
  tick) into a **100,000 FE** buffer that accepts no inserts. It pushes that FE
  to upstream's ten targets — the four sides, each repeated one block up, the
  block below and two above — so a machine beside it runs.
- **The six default addons** (`oritech:machine_{speed,efficiency,ultimate,processing,capacitor,burst}_addon`)
  — each machine scans its own `getAddonSlots` positions every tick: the speed addon makes a craft
  1.5× faster for 20% more FE, the efficiency addon makes it cheaper, the Synergy Matrix both, the
  Auxiliary Processing Chamber runs one extra craft a completion, the capacitor adds 2,000,000 FE of
  buffer and 2,000 insert, and a charged burst addon runs a machine eight times as fast for a while.
  Additive stacking, the jar's shipped default.
- **The Control Unit Addon** (`oritech:machine_redstone_addon`) — the seventh addon and the only one
  that is not a number: with a redstone signal at the addon, its machine's whole tick is gated — no
  recipe resolved, no progress, no FE spent, no energy pushed — and the readout says `redstone
  disabled`; remove the signal and the same operation carries on. Upstream's `disabledViaRedstone`,
  saved under its own `oritech.redstone` key.
- **Using it** — right-click a generator with fuel, or a pulverizer or furnace
  with a matching item, to insert it; right-click a multiblock with a machine
  core in the hand to place the core, and with an empty hand to assemble it and
  read it; right-click a machine with an addon block in the hand to put it in
  the machine's first open slot.
  `/oritech info <x> <y> <z>`, `/oritech energy <x> <y> <z> <amount>`
  and `/oritech fluid <x> <y> <z> <amount> <fluid>` (op) read, prime and fill a
  machine.

## What it does not do (yet)

No GUI, no GeckoLib models (the machines are drawn as a full cube of their own
texture), no crafting recipe for the blocks (they need Oritech components that
are not in these slices — use `/give`), no special addons (the Control Unit
Addon's four comparator modes, fluid, inventory proxy, acceptor, extender, combi,
quarry), no energy or item pipes, no comparator output, no cold-biome speed bonus
on the chiller
(the pin has no biome read), no bucket input *slots* on the lava generator (a
bucket in the hand fills it), no other multiblock (assembler, atomic forge,
foundry, refinery, centrifuge), no augment stations, no accelerator ring or
reactor, no ore processing chain and no ores. `notes.md` lists every gate and
deviation; `FINDINGS.md` has the upstream behaviour and the jar numbers.

## Building

```sh
python3 tools/carry_content.py              # assets/ + data/ from the ATM10 jar (deterministic)
cargo test --offline                       # the rules: 91 tests (runtime + glue)
cargo clippy --all-targets --offline -- -D warnings
cargo fmt --check                          # NOT --all: that checks the fetched SDK
cargo horizon-mod build                    # target/horizon-mod/oritech.hmod
cargo horizon-mod check oritech.hmod
cargo horizon-mod fits oritech.hmod --build 0a875da
cargo horizon-mod fits oritech.hmod --build 0b14f8a   # a different build: the pin run is a tautology
```

`cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` (gitignored,
never packed); the tree here already carries a copy of the campaign cache. The
package's one registry dependency is `serde_json` (the carried rules module
parses the recipe JSON); `cargo horizon-mod build` packs no `Cargo.lock`, while
the launcher's `native-build` needs one — it is generated from the launcher's
cargo home for the in-game leg and removed afterwards (see `notes.md`).

The in-game leg needs a game the launcher built with this package (`../tools/launcher-game.sh build
oritech`), because a native `.hmod` cannot load on an unmodded build. `tools/e2e.sh` is the scenario
and its assertions, driven with `GAME=` (see the file's header). `SELFTEST=1`, `SELFTEST_COOLER=1`,
`SELFTEST_LAVA=1`, `SELFTEST_ADDONS=1` and `SELFTEST_REDSTONE=1` must fail.

Every build/test step goes through `../tools/build-slot.sh` in the campaign tree (`BUILD_LOCAL=1`,
since the far box has no copy of this port). Pin `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`, mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`.

Licence: CC0-1.0 (`LICENSE`, `LICENSE-NOTICE.md`).
