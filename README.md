# Oritech for Horizon — slices 1 to 11

The **Basic Generator**, the **Pulverizer**, the **Powered Furnace** multiblock, the **Industrial
Chiller**, the **Lava Generator**, the **machine addon system**, the **Control Unit Addon**, the
**material and component chain**, the **Assembler** and the **Centrifuge** of
[Oritech 1.2.13](https://github.com/Rearth/Oritech) (CC0-1.0), rewritten in Rust for Horizon at the
campaign pin `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (mod API 0.2, 26.3).

One flat `.hmod` carries both halves — `src/lib.rs` (the one `impl Hooks`), the seven block-entity
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
- **The material chain** — **steel**, **electrum** and **raw silicon/silicon** from vanilla items;
  the **magnetic coil** (3 nickel + 3 steel → 4) and the **motor** (1 nickel + 4 steel + 2 coils);
  the three **plating blocks** (4 steel + 4 stone + 1 copper/iron/nickel → 2). With them, **every one
  of the five machines, three machine cores and the Control Unit Addon is craftable at a crafting
  table** — the generator from 5 nickel + 2 coils + 1 copper + 1 furnace, the pulverizer from 5 iron +
  2 motors + 1 nickel + 1 copper block, and so on — and **raw silicon smelts into silicon** in any
  furnace. The seven items and the three blocks are declared items with the jar's own models and
  textures, in the **Oritech Components** and **Oritech Machines** creative tabs, with the
  conventional `c:` item tags their recipes name.
- **Assembler** (`oritech:assembler_block`) — the mod's four-input machine: a controller plus **three
  cores** (east, above, and both) that matches an `oritech:assembler` recipe's ingredients **in any
  slot order**, spends **128 FE a tick** from a **50,000 FE** buffer, and grants the recipe's result
  into its one output slot. The jar's own files: **one nickel + one steel + two coils → two motors in
  48 ticks for exactly 6,144 FE**, steel + two nickel + copper → six coils, and three vanilla
  assemblies (four blaze powder → a blaze rod, gunpowder + blaze powder + two coal → four fire
  charges, a log + three glowstone → a shroomlight). A slot holding a spare pays one item per
  ingredient and keeps the rest. The machine itself is craftable at a table from copper, crafters,
  **adamant** (2 nickel + 2 diamond), two motors and any plating.
- **Centrifuge** (`oritech:centrifuge_block`) — the mod's separator: a controller plus a core **one
  block above**, **one input and two outputs**, and a **10,000 FE** buffer taking **512** an insert and
  spending **64 FE a tick**. The jar's `centrifuge/carbon.json` turns one **coal dust** (which the
  pulverizer now makes from any `#minecraft:coals`) into one **carbon fibre strand** in **50 ticks for
  exactly 3,200 FE**, and eight strands + redstone craft the third **machine core**. In its two addon
  slots, the eighth addon, the **Fluid Addon**, switches the machine to the jar's
  `oritech:centrifuge_fluid` table: **250 mB of water + dirt → one clay in 100 ticks for exactly 6,400
  FE**, from its own **eight-bucket tank** (a real `horizon:fluids` handler). Without the addon the
  water and dirt make nothing. A craft whose result cannot be installed — the output slot already
  holding a full stack of it — **takes nothing**: no coal dust, no dirt, no millibuckets of water and
  no FE, the rule every machine in this mod follows (`canOutputRecipe` before `craftItem`).
- **Foundry** (`oritech:foundry_block`) — the mod's alloying multiblock: a controller plus **three
  cores** at (0,0,1), (0,1,0) and (0,1,1), **two input slots and one output**, a **50,000 FE** buffer
  taking **1,024** an insert and spending **128 FE a tick**. Its `oritech:foundry` recipes match their
  two ingredients **in any slot order**: one iron + one coal dust → one **steel** in **26 ticks for
  exactly 3,328 FE**, a gold + a redstone → one **electrum**, a diamond + a nickel → one **adamant**,
  and a gold + a netherite scrap → one **netherite ingot**, the last three in **80 ticks for exactly
  10,240 FE** each. Two iron ingots make no alloy at all, and the foundry itself crafts at a table
  from a cauldron, a motor, three copper and two electrum.
- **Fragment Forge** (`oritech:fragment_forge_block`) — the last item-only processing machine and
  the jar's "grinder": a controller plus **seven cores** around it, **one input and three outputs**, a
  **50,000 FE** buffer taking **2,048** an insert and spending **256 FE a tick**. An `oritech:grinder`
  recipe matches its one input and puts each result in its own output slot: one `minecraft:iron_ore`
  becomes **2 raw iron + 1 raw nickel in 40 ticks for exactly 10,240 FE**. The raw nickel then smelts
  in the powered furnace into **one nickel ingot in 100 ticks for 3,200 FE** — a survival route to
  nickel with no worldgen. A result without a free slot takes nothing and spends nothing, as on every
  machine here.
- **Using it** — right-click a generator with fuel, or a pulverizer or furnace
  with a matching item, to insert it; right-click a multiblock with a machine
  core in the hand to place the core, and with an empty hand to assemble it and
  read it; right-click a machine with an addon block in the hand to put it in
  the machine's first open slot.
  `/oritech info <x> <y> <z>`, `/oritech energy <x> <y> <z> <amount>`,
  `/oritech fluid <x> <y> <z> <amount> <fluid>` and `/oritech slot <x> <y> <z>
  <slot> <count> <item>` (op) read, prime, fill and write one slot of a
  machine — the last one is the only route into an *output* slot, since the
  right click fills an input slot only.

## What it does not do (yet)

No GUI, no GeckoLib models (the machines are drawn as a full cube of their own
texture), no **ore worldgen** (the pin cannot register a content package's configured features;
the grinder makes raw nickel from iron ore and the powered furnace smelts it, while platinum still
has no source), no special addons (the Control Unit
Addon's four comparator modes, inventory proxy, acceptor, extender, combi,
quarry), no energy or item pipes, no comparator output, no cold-biome speed bonus
on the chiller
(the pin has no biome read), no bucket input *slots* on the lava generator (a
bucket in the hand fills it), no other multiblock (atomic forge,
refinery), no augment stations, no accelerator ring or
reactor, no rest of the ore-processing chain and no other ores — and the **refinery is blocked** outright: its 22
recipes all take or make a fluid this package cannot declare, `#c:naphtha` among them. `notes.md` lists every gate and
deviation; `FINDINGS.md` has the upstream behaviour and the jar numbers.

## Building

```sh
python3 tools/carry_content.py              # assets/ + data/ from the ATM10 jar (deterministic)
cargo test --offline                       # the rules and the carried data: 158 tests
cargo clippy --all-targets --offline -- -D warnings
cargo fmt --check                          # NOT --all: that checks the fetched SDK
cargo horizon-mod build                    # target/horizon-mod/oritech.hmod
cargo horizon-mod check oritech.hmod
cargo horizon-mod fits oritech.hmod --build 0a875da
cargo horizon-mod fits oritech.hmod --build 0b14f8a   # a different build: the pin run is a tautology
```

`cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` (gitignored,
never packed); the tree here already carries a copy of the campaign cache. The
SDK's path dependency is the package's **only** dependency: the registry refuses
a registry crate (R9), so the recipe JSON is read by the package's own small
reader in `src/json.rs` and no `Cargo.lock` travels inside the archive.

The in-game leg needs a game the launcher built with this package (`../tools/launcher-game.sh build
oritech`), because a native `.hmod` cannot load on an unmodded build. `tools/e2e.sh` is the scenario
and its assertions, driven with `GAME=` (see the file's header). `SELFTEST=1`, `SELFTEST_COOLER=1`,
`SELFTEST_LAVA=1`, `SELFTEST_ADDONS=1`, `SELFTEST_REDSTONE=1` and `SELFTEST_CRAFT=1` must fail.

Every build/test step goes through `../tools/build-slot.sh` in the campaign tree; compile steps
forward automatically to the second box (`AdLf-PC`), which now has a working `cargo-horizon-mod`
CLI. Pin `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`, mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`.

Licence: CC0-1.0 (`LICENSE`, `LICENSE-NOTICE.md`).
