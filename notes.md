# Oritech — port notes (slice 1: the Basic Generator and the Pulverizer; slice 2: the Powered Furnace; slice 3: the Industrial Chiller; slice 4: the Lava Generator)

Ported from **Oritech 1.2.13** (`oritech-neoforge-1.21.1-1.2.13.jar`, SHA-256
`32600bf5b08fcf6d4a49e557f90ee45b238098170777872c8919d9fdb0a1b828`,
5,184 entries) for Minecraft Java 1.21.1 / NeoForge. The jar's own
`META-INF/neoforge.mods.toml` declares **`license = "CC0-1.0"`** — a
public-domain dedication, **licence rank 0**, no obligations. (The source map's
CC-BY-4.0 is stale; the jar wins, ARCHITECTURE.md §8.) Local-only until the
maintainer approves publishing.

Pin **`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`**, mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`** (schema 6, `package_format` 6) — the
campaign pin from 2026-11-21; slice 1 and the 0.2.1 review ran at `0b14f8a`, and the
0.2.2 package declares the new pin and reports `0b14f8a  fits` (see *Slice 2*).

Status: **ported-partial** — slice 1 (the fuel generator, the pulverizer, their
energy link, the recipe reader and the content carry), slice 2 (the Powered
Furnace multiblock and the seven machine cores), slice 3 (the Industrial
Chiller, the mod's first fluid machine) and slice 4 (the Lava Generator, a
fluid-burning generator) are implemented and verified in game. The gap list
below is the rest of the mod.

**Why this mod now:** it is the rank-0 `content`/`deep` core mod
`precheck-mod.py oritech` offers (`DISPATCHABLE`, jar licence CC0-1.0, no
unported content dependency, no worldgen). Brief:
`ports/_shared/queue/brief-oritech.md`.

## Class + tier and the approach

- **Class:** `content`; **tier:** `deep` (`ports/ledger.csv`,
  `ports/_shared/source-map/atm10-sources.csv`).
- **Approach** (`horizon/docs/modpacks.md`, "What packs need and what runs
  today"): a `deep` mod whose smallest honest slice is the **machines** shape —
  a block entity per machine, a server tick, a declared energy capability, a
  container, a custom recipe type — plus the carried block assets/data. The
  `deep` parts (GeckoLib models, the accelerator ring, the reactor) are gaps.
- **Carry-over vs agent port:** the two machine textures, seven pulverizer
  recipes, the lang names and the loot entries are carried from the jar; the
  behaviour, the block entities, the recipe reader and the commands are the
  agent port ([`CONTENT.md`](CONTENT.md) lists every file).

## What the slice does (the jar's own numbers)

| Behaviour | Upstream file | This port |
|---|---|---|
| generator buffer | `OritechConfig.BasicMachineConfig(50_000, 0, 32*8, 32)` | API 142 buffer: capacity 50,000 FE, insert 0, extract 256 FE/call |
| generator production | `UpgradableGeneratorBlockEntity.produceEnergy` | **32 FE** a burning tick |
| generator fuel | `FuelRegistry.get(item)` | the engine's `minecraft:cooking_fuel` (`Providers::burn_time`), so modded fuels work |
| generator burn | `progress = burnTime`, one per tick | same; a coal is 1,600 burning ticks |
| buffer-full rule | `canFitEnergy` (room for a whole tick) | same: at 49,984 FE the burn pauses and keeps its remaining progress |
| generator output | `outputEnergy` to the six neighbours | queued, then `native_move_energy` at `level_tick_post` (256 FE/call) |
| pulverizer buffer | `BasicMachineConfig(25_000, 32*8, 0, 32)` | capacity 25,000 FE, insert 256 FE/call, extract 0 |
| pulverizer cost | `MachineBlockEntity.calculateEnergyUsage` (32 x 1 / 1) | **32 FE** a working tick |
| pulverizer recipe | `oritech:pulverizer` (`ingredients`, `results`, `time`) | the pin's custom-recipe table, read by `runtime.rs`, ingredients resolved by `native_recipe_ingredient` |
| pulverizer duration | `progress >= time * speed` (speed 1) | `time` ticks, from the file (100 for every carried recipe) |
| pulverizer outputs | `canOutputRecipe` + `craftItem` | two output slots, each result must fit its own slot; one input is consumed on completion |
| slots | `InventorySlotAssignment(0,1,1,0)` / `(0,1,1,2)` | generator: fuel 0 (insert-only); pulverizer: input 0, outputs 1-2 |
| block | `Properties.ofFullCopy(Blocks.IRON_BLOCK).noOcclusion()` | content block on the `minecraft:iron_block` template, `occludes: false` |

## Hooks (all stamped; no pending hook is named)

| Hook (API) | Helper | What it does |
|---|---|---|
| `block_entity` (15/129) | `block_entity.rs` | both machines get a block entity |
| `block_entity_ticks` (15) | `block_entity_ticks.rs` | both tick on the server |
| `block_entity_container` (16) | `block_entity_container.rs` | the fuel slot / input + two outputs, all faces |
| `block_entity_energy` (142) | `block_entity_energy.rs` | the two FE buffers (`Capabilities.Energy.BLOCK`) |
| `block_entity_tick` (17) | `block_entity_tick.rs` | the generator's burn and the pulverizer's recipe/energy tick |
| `level_tick_post` (23) | `level_tick_post.rs` | the queued generator → adjacent pulverizer energy move |
| `right_click_block` (21) | `right_click_block.rs` | insert fuel/input, or read the status (no GUI) |
| `register_commands` (8) | `register_commands.rs` | `/oritech info`, `/oritech energy` |
| `perform_command` (8) | `perform_command.rs` | their bodies |
| `world_lifecycle` (44) | `world_lifecycle.rs` | drop the recipe cache and the push queue per world |
| `content_root` (raw target) | `embedded_content.rs` | write the bundle's own content out and load it |

11 injections (10 named hook points + the raw `content_root` target), 2 modules,
**0 unstamped**.

## Pin facts that shaped this slice

1. **FE is a real capability at this pin.** API 142 declares
   `Capabilities.Energy.BLOCK` with the amount in the block entity's int
   `energy`, and ships `native_energy`, `_insert`, `_extract` and
   `native_move_energy` (`game/machines/native_energy.rs`). Cross-block exchange
   therefore needs no invention.
2. **A block entity is out of the world during its own tick**
   (`native_energy_at` refuses it), so the generator's `outputEnergy` is queued
   and moved at `level_tick_post`, where both blocks are in the world — the same
   pattern `actuallyadditions`, `powah` and `enderio` use.
3. **Custom recipe types load from a content mod**, which is what makes the
   embedded `.hmod` the natural home for the seven `oritech:pulverizer` files
   (`native_recipes_of_type`, API 36).
4. **`c:…` tags do not exist in 26.3's data**: `#c:cobblestones` (the jar's
   gravel recipe) resolves to nothing, so that recipe is not carried. The
   vanilla `#minecraft:*_ores` tags do exist and are carried.
5. **26.3 loot needs the singular `condition`**; the jar's 1.21.1 `conditions`
   array would evaluate false and the machines would drop nothing.

### API mapping (upstream → this port)

| Upstream | Here |
|---|---|
| `BasicGeneratorEntity.tryConsumeInput` | `support::tick_generator` (fuel pick) + `runtime::Generator::load_fuel` |
| `UpgradableGeneratorBlockEntity.serverTick` / `produceEnergy` | `runtime::Generator::tick` |
| `UpgradableGeneratorBlockEntity.canFitEnergy` | `runtime::Generator::can_fit` |
| `UpgradableGeneratorBlockEntity.outputEnergy` + `getOutputTargets` | `support::queue_push` + `support::on_level_tick_post` (`native_move_energy`) |
| `MachineBlockEntity.serverTick` / `calculateEnergyUsage` | `runtime::Pulverizer::tick` |
| `MachineBlockEntity.canOutputRecipe` + `canAddToSlot` | `runtime::can_output` / `runtime::output_fits` |
| `MachineBlockEntity.craftItem` | `support::finish_operation` / `support::grow_output` |
| `PulverizerBlockEntity.getOwnRecipeType` (`oritech:pulverizer`) | `support::find_recipe` + `native_recipes_of_type` |
| `OritechRecipe` (`ingredients`, `results`, `time`) | `runtime::PulverizerRecipe::parse` |
| `OritechConfig.BasicMachineConfig` | the `const`s at the top of `runtime.rs` |
| `DynamicEnergyStorage` (long `oritech.machine_energy`) | API 142 buffer (int `energy`) |
| `InventorySlotAssignment` | `support::on_container` (inputs/outputs per block) |
| `ItemGroups.MACHINE_GROUP` | `content/data/oritech/horizon/creative_tab/machines.json` |
| `saveAdditional` / `loadAdditional` | `support::{generator,pulverizer}_state` / `store_*` (upstream's key names) |

### Preserved quirks

- The generator stops one tick short of a full buffer and keeps the rest of the
  fuel (`canFitEnergy`), so one coal alone fills only to 49,984 FE.
- The pulverizer keeps its progress while it has no FE and resets it when the
  recipe changes or the output is blocked.
- An operation consumes its input only when it completes.

## Command (harness addition)

`/oritech info <x> <y> <z>` (no permission level) and `/oritech energy <x> <y>
<z> <amount>` (op level 2) print/prime a machine:

```
Basic Generator at [0, 100, 0]: 1440/50000 FE, burning 1555/1600 ticks, fuel minecraft:coal x4, t=406
Pulverizer at [4, 100, 0]: 6800/25000 FE, progress 0 recipe none, input empty, out0 minecraft:bone_meal x6, out1 empty, t=563
```

Upstream has no commands of its own (its GUI and config are the only controls);
these exist so a player without the GUI, and the harness, can read the buffers.
`t=` is the world game time, added so a stale chat reply cannot be mistaken for
a fresh one. Precedent: `/powah info|set`, `/charginggadgets info|energy`.

## Content (`.hmod`)

`content/` is generated from the jar by [`tools/carry_content.py`](tools/carry_content.py)
(deterministic, `--check`); [`CONTENT.md`](CONTENT.md) lists every carried and
every written file and why. 26 entries: both blockstates, block and item models,
26.3 item definitions, two machine textures, `en_us.json`, two loot tables, seven
pulverizer recipes, two block declarations, the machines creative tab and the
pickaxe / stone-tool tags.

## Embedded content (AE2 pattern)

The registry stores **one file per project**, so a port with both halves could
not be listed faithfully: an `.hnmod` is code-only and an `.hmod` is content.
This bundle carries its content, as the merged AE2 entry does:

- **`mixins/oritech/data/oritech-content.hmod`** is the built
  `content/oritech.hmod`, byte for byte (19,622 B, `7a189137…deabf`).
  `tools/package.py` packs `data/**` as `mixins/<id>/data/…` with fixed zip
  timestamps and refuses a copy that is not byte-identical to
  `content/oritech.hmod`.
- **`mixins/oritech/embedded_content.rs`** (`embedded-content` injection,
  `target = "horizon_client::game::mods::content_root"`, HEAD, cancellable)
  compiles the file in with `native_data!`, writes it to
  `.horizon-cache/oritech-embedded-content.hmod` when it differs, appends it to
  the dev list and returns the merged `content_root`. The recursion stops at its
  own `dev.contains(&file)` guard, so the vanilla body runs once. An unwritable
  cache logs a warning and continues with no content instead of failing.

Verified in game with **only the bundle** (`EMBEDDED=1`, no `--dev-mod`): the
whole scenario passes, the written file is byte-identical to
`content/oritech.hmod`, and the control leg (same binary, `.horizon-cache`
replaced by a plain file) leaves the block id unknown and gives no item.

## Verification

Every staged build/test/e2e ran through `ports/tools/build-slot.sh` (the package
step's compile forwarded to the second machine); `CARGO_INCREMENTAL=0`, no bare
cargo, no `--target-dir`. Evidence: `ports/_shared/evidence/oritech/`.

| Step | Evidence | Result |
|---|---|---|
| runtime TDD (red) | `test-red.log` | **0 passed, 17 failed** — every body `todo!()` |
| runtime tests (green) | `test-green.log` | **17 passed**, 0 failed |
| fmt | `fmt.log` | `cargo fmt --all -- --check` and `rustfmt --check mixins/oritech/*.rs`, both exit 0 |
| clippy | `clippy.log` | `--all-targets --locked --offline -- -D warnings`, exit 0, `Finished` |
| stage | `ladder/stage.log` | `stage_changed: true`, 2 modules, 11 injections |
| staged check | `ladder/check.log`, `horizon-check.log` | `cargo_exit: 0`, **0 warnings** |
| embedded tests | `ladder/test.log`, `horizon-test.log` | `test result: ok. **17 passed**; 0 failed; 1240 filtered out` |
| fast build | `ladder/build.log`, `horizon-build.log` | `cargo_exit: 0`, **0 warnings** |
| e2e | `e2e.log` | **PASS**, `EXIT:0` |
| fail-capability | `e2e-selftest.log` | `SELFTEST=1` → **EXIT:1** (`89 ticks spent 2848 FE, expected 2880`) |
| e2e, one file only (`EMBEDDED=1`, no `--dev-mod`) | `e2e-embedded.log` | **PASS**, `EXIT:0`; the control leg shows the ids unknown without the embedded file |
| embed fail-capability | `e2e-embedded-selftest.log` | `EMBEDDED=1 SELFTEST=1` → **EXIT:1** |
| log checker | `check-e2e-logs.py oritech --strict` | 4 logs, 4 ok, 0 bad |
| `.hmod` build/check | `package.log` | 19,622 B, `7a189137…deabf`, 26 entries, “no code: content only” |
| `.hnmod` (content embedded) | `package.log` | 35,047 B, `60c38261…fa93c`, 16 entries incl. `mixins/oritech/data/oritech-content.hmod` |
| `cargo horizon-mod check --kind native-mod` | `package.log` | exit 0, `native-mod … for 26.3` |
| reproducibility | `package.log` | two consecutive `tools/package.sh` runs: identical SHA-256 for both artifacts |

### In-game transcript (quoted from `e2e.log`)

```
  info(0,100,0): Basic Generator at [0, 100, 0]: 0/50000 FE, burning 0/0 ticks, fuel empty, t=142
  info(4,100,0): Pulverizer at [4, 100, 0]: 0/25000 FE, progress 0 recipe oritech:pulverizer/bone, input minecraft:bone x1, out0 empty, out1 empty, t=337
  the unpowered machine kept its bone and made nothing
  click: {"ok":true,"block":"oritech:basic_generator_block","inventory_change":{}}
  generator alone: 1440 FE, burning 1555/1600 ticks, minecraft:coal x4
  info(4,100,0): Pulverizer at [4, 100, 0]: 9648/25000 FE, progress 11 recipe oritech:pulverizer/bone, input minecraft:bone x1, out0 empty, out1 empty, t=424
  info(4,100,0): Pulverizer at [4, 100, 0]: 6800/25000 FE, progress 0 recipe none, input empty, out0 minecraft:bone_meal x6, out1 empty, t=563
  89 ticks spent exactly 2848 FE (9648 -> 6800), output minecraft:bone_meal x6
  the generator moved 3584 FE into the adjacent pulverizer
  the powered machine finished: minecraft:bone_meal x6
  after relaunch: 0/50000 FE, burning 1049/1600 ticks (was 1160), minecraft:coal x4
  (the buffer reads 0 because the adjacent pulverizer drains it every tick)
  has(4,100,0) minecraft:bone_meal x6: ok
PASS: an unfuelled generator made no FE, a pulverizer with no power kept its bone, one coal click started the burn at 32 FE a tick, bone became exactly 6 bone meal in exactly 100 ticks for exactly 3,200 FE, the adjacent pulverizer ran on the generator's FE, stone was refused by both machines, and the machines survived a relaunch
EXIT:0
```

The run is fail-capable four ways: `SELFTEST=1` flips the energy-spent
expectation and exits 1 (both legs); A, B and F are controls that must not move
a number (no fuel, no power, a non-fuel/non-recipe item); D and E assert exact
energy arithmetic (32 FE x ticks) rather than "some FE appeared"; and the
embedded run's control leg shows the same ids unknown without the bundled
content.

The 100-tick leg: the first read lands 11 ticks into the operation
(`9648 = 10000 - 11 x 32`), the rest costs `89 x 32 = 2,848`, so the whole
operation is `100 x 32 = 3,200 FE` — the jar's `time: 100` and its 32 FE/t.

## Deviations and gaps (slice 1)

1. **The energy buffer's key is the API's `energy` (int), not upstream's
   `oritech.machine_energy` (long).** API 142 reads and writes the int `energy`;
   every value here fits, and the buffer is a real `Capabilities.Energy.BLOCK`
   to other mods.
2. **Progress is saved with the recipe id** (`oritech.machine_recipe`).
   Upstream does not save `currentRecipe`, so its first tick after a reload
   resets `progress`; here a reload continues the operation. The visible rule
   (a different input restarts the count) is upstream's.
3. **No GUI/screen.** Upstream opens a machine screen with an energy bar, a
   progress arrow and addon slots; this slice inserts by right-click or
   automation and reads state with `/oritech info`. The menu, the screen, its
   textures and shift-click routing are deferred.
4. **No GeckoLib rendering.** The machines are drawn as a full cube of their own
   texture (the jar's blockstate is a GeckoLib placeholder). No animations, no
   `lit` state, no particles (`PulverizerBlock`'s `stepOn` damage included).
5. **The machines have no crafting recipes.** They need Oritech components
   (`motor`, `magnetic_coil`, nickel/copper tags) that are not in this slice;
   carrying the recipes would ship uncraftable ones. Gained with `/give` here.
6. **117 of the 124 pulverizer recipes are not carried**: they name Oritech
   dusts/clumps/gems (the ore-processing chain) that this slice does not add.
   The seven carried ones are the vanilla-in/vanilla-out recipes.
7. **No dust auto-combine** (`PulverizerBlockEntity.combineSmallDusts`: 9 small
   dusts + a crafting lookup → 1 dust) and no second-output recipes in this
   slice (none of the carried recipes has one; the slot and the rule exist).
8. **The energy link is adjacency.** Upstream pushes to its six neighbours
   through `getOutputTargets`; here the same six neighbours are walked at
   `level_tick_post`. No energy pipe, cable or power pole (the mod's network is
   a gap; ARCHITECTURE.md §3 records that the pin has no network/graph API).
9. **Redstone control and addons do nothing.** The basic generator has no addon
   slots upstream; the pulverizer's one addon slot, the addon items, the speed
   and efficiency multipliers and the burst ticks are all fixed at 1 here
   (`BaseAddonData.DEFAULT_ADDON_DATA`).
10. **No comparator output** for the buffers; the block has no `has_analog_output`
    hook declared, so a comparator reads the container, not the FE.
11. **Breaking a machine loses its FE and its slots' contents are the vanilla
    container drop** — the loot tables are the jar's plain self-drop (upstream
    keeps energy in the block entity too).
12. **Bucket fuels are accepted here.** Upstream's `tryConsumeInput` refuses a
    `BucketItem`; this port burns any item the engine's fuel table knows, and
    honours the crafting remainder (a lava bucket leaves a bucket). A
    deliberate, recorded difference.
13. **Not carried:** the powered furnace (a multiblock machine that needs its
    core assembled), the assembler, atomic forge, foundry, refinery, centrifuge,
    cooler, the accelerator ring, the reactor, the augment stations, the jetpack
    and equipment, the resource-node worldgen and the ores. All later slices.

## Campaign gates at this touch

- `python3 ports/tools/precheck-mod.py oritech` → exit 0, `DISPATCHABLE`, jar
  licence `CC0-1.0` (rank 0).
- `python3 ports/tools/check-e2e-logs.py oritech --strict` → 4/4 logs OK.
- `python3 ports/tools/check-skills.py --notes ports/oritech` → the `Skills:`
  line below.
- `python3 ports/tools/upstream_ready.py oritech` → every gate green except the
  ledger status, which the orchestrator records (`--include-blocked` shows the
  rest). This port does not write `ports/ledger.csv`.
- Pin watch at this touch: `check-pin-updates.py` reported the pin **5,056
  commits** behind main with `+2` hooks (`world_hold`, `world_warp`) and `+3`
  helpers (`force_chunk`, …), none of which this slice needs. Stayed on the pin.

## Skills

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

`cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test` were
run before hand-back. `#[allow(private_interfaces, reason = "the injected bundle
is this crate's own module")]` marks the injected module's own private
`ModBlockEntity`/`Container`/`Screen` parameters (the same exception
`charginggadgets`, `botanypots` and `powah` record), and
`#[allow(dead_code, reason = "pure API: the glue builds the state from NBT, the
tests use the empty one")]` marks the two `new()` constructors.

## Provenance (upstream analysis)

Repository `https://github.com/Rearth/Oritech`, tag **`v1.2.13`**, commit
**`4069b6dd14b05dfe0c80126618739b64054630ea`** (2026-10-05, "Add 1.2.13
changelog"); shallow clone deleted after the findings were written. Files read
and behaviour extracted: [`FINDINGS.md`](FINDINGS.md). The jar stays as licence
evidence at `ports/_shared/licence-cache/oritech-neoforge-1.21.1-1.2.13.jar`.
Clean-room rewrite: no Java was copied or translated and no jar was decompiled.

## Packaged artifacts

| File | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `content/oritech.hmod` | 19,622 B | `7a189137fcef9bf92c6771c28edd0c3964a29a1eb1fc7b3d5c01971e41cdeabf` | 26 |
| `oritech.hnmod` | 35,047 B | `60c382616c8c0c91476303148a341f0814517087f8cdf37a9c82fb4b5bffa93c` | 16 |

The `.hnmod` carries the content mod as
`mixins/oritech/data/oritech-content.hmod` (19,622 B, the same SHA-256), so it
is the single artifact a registry entry needs.

### Port version (reviewer rule, playbook §1.7)

The registry version is the **port's** version, not the original mod's: the
manifest, `horizon-mixin.toml` `[mod]` and the artifact name all say **0.1.0**,
not the jar's 1.2.13.

## Review repairs (PR #671)

- **[P2] `runtime.rs::output_fits(None, …)` returned true for any result.** The
  parser accepts result counts 1..=99 while `grow_output` clamps to the item's
  stack limit, so a 99-item result into an empty 64-stack slot lost 35 items and
  still consumed the input. `output_fits` now rejects an empty-slot result larger
  than one stack (`result.count <= max_stack`), and
  `an_empty_output_slot_rejects_a_result_larger_than_one_stack` is the
  regression test. `cargo test` and the staged ladder are re-run below.
- *"The entry original link `[oritech](Rearth/Oritech)` is relative"* — the
  entry links `https://github.com/Rearth/Oritech` and credits Rearth.
- Staging (`oritech.hnmod` not in the registry release/staging) is the
  orchestrator's (horizon-mods#686).

### Verification after the repairs

- Artifact: `oritech.hnmod`, 42,476 B, SHA-256 `1b21ad66b549cb217ec654b47edf4faaa90380b521aeab26ede30f768f3ae614` (rebuilt twice, identical bytes; `tools/package.py`; the embedded content equals `content/oritech.hmod`).
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --locked --offline`: clean, 18 passed; the embedded run (`python3 tools/horizon.py test --horizon ports/_shared/pins/horizon-1d63e0a`) is 18 passed, 0 failed (`target/horizon-test.log`).
- `build-slot.sh e2e --job … --port ports/oritech -- tools/e2e.sh`: PASS, log `ports/_shared/evidence/oritech/e2e-repair.log` ends `EXIT:0` (`check-e2e-logs.py oritech --strict` exit 0).
- `python3 ports/tools/upstream_ready.py oritech` and `check-skills.py --notes ports/oritech`: exit 0.

## Migration to the SDK format (0.2.0, 2026-11-18)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70, PR #17992) retired the `.hnmod`
container; the registry closed every `.hnmod` submission on 2026-11-18 with one reason
(*"republish this as a native `.hmod` with the SDK from the next Nightly"*). This port's
former submission, **PR #883** (`oritech` 0.1.0), was closed by that purge. **0.2.0** is
the same slice republished as one flat SDK `.hmod`; **`src/runtime.rs` and its 18 tests are
unchanged, byte for byte**. The changelog text is at the end of this section.

- **Pin:** SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (2026-11-18,
  `package_format` 6, `content_api` "0.2", rust 1.99.1), mixins
  **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.78, `cargo-horizon-mod`
  from `horizon-rust/horizon@main` (0.2.x). The SDK is cached in-tree at
  `ports/_shared/sdk/0b14f8a/` and was copied into `.horizon/sdk/` (gitignored, never
  packed), so every step below ran offline.
- **Version:** the port's own **0.2.0** (the retired bundle was 0.1.0; a resubmission is
  always a new version). The mod's identity is `horizon-mod.toml`, not Cargo.
- **Class + tier** unchanged: class **content**, tier **deep** (`ports/ledger.csv`), still
  delivered as the **machines** shape this slice needs (two block entities, a declared
  container and a declared FE buffer, one recipe type, one energy link). No re-tiering, no
  new feature.
- **Licence** unchanged: **CC0-1.0**, rank 0, read from the jar's own
  `META-INF/neoforge.mods.toml` (`audit_licences.py --mod oritech --fetch` → `matches True`).
  Local-only until the maintainer approves publishing.

**Done in this order** (the retired tree was gone before the first SDK build): the old
layout moved to `ports/_shared/evidence/oritech/retired-hnmod-0.1.0/`, `assets/` + `data/`
copied to the package root from `content/`, `src/runtime.rs` copied with `cp` and proved
with `cmp` + `sha256sum`, `cargo test` run on the new crate **before any glue existed**
(18 passed, `evidence/oritech/sdk-runtime-first.log`), the glue's tests written first and
run **red** against `todo!()` bodies (7 failed, `sdk-glue-red.log`), and only then the glue
implemented.

### File map (old → new)

| 0.1.0 path | Now | Evidence |
|---|---|---|
| `mixins/oritech/runtime.rs` | `src/runtime.rs`, **byte-identical** (SHA-256 `0451000b2c153b1be1bb6750096a8cbf71b9c49de956ad7bc7195333c770661e`, `cp` then `cmp`, and the same hash for `unzip -p oritech.hmod src/runtime.rs`) | `retired-hnmod-0.1.0/mixins/oritech/runtime.rs` |
| `mixins/oritech/support.rs` (glue, `oritech_state` module) | `src/support.rs` (same glue, typed): declarations, saved state, both ticks, the recipe cache, the energy move, the right click, the command bodies | `retired-hnmod-0.1.0/mixins/oritech/support.rs` |
| `mixins/oritech/horizon-mixin.toml` (2 `[[module]]`, 11 `[[injection]]`) | no plan: `Registrar` declarations + typed `Hooks`/Part methods | `retired-hnmod-0.1.0/mixins/` |
| 11 helper `.rs` files (`block_entity`, `block_entity_ticks`, `block_entity_container`, `block_entity_energy`, `block_entity_tick`, `level_tick_post`, `right_click_block`, `register_commands`, `perform_command`, `world_lifecycle`, `embedded_content`) | methods of `impl Hooks`, of the two Parts, or `Registrar` calls; no per-hook files | `retired-hnmod-0.1.0/mixins/oritech/` |
| `mixins/oritech/data/oritech-content.hmod` + `embedded_content.rs` (the AE2 write-out) | **gone**: code and content are one package, so there is nothing to embed | `retired-hnmod-0.1.0/mixins/` |
| `content/horizon-mod.toml` + `content/assets/` + `content/data/` | the package root: `horizon-mod.toml`, `assets/`, `data/` — all 25 carried files byte-identical (`sha256sum` diff empty; `tools/carry_content.py --check` → `25 files, 0 differ`) | `retired-hnmod-0.1.0/content/` |
| `content/oritech.hmod` (19,622 B) | not needed: the content is inside the one package | `retired-hnmod-0.1.0/content/oritech.hmod` |
| `oritech.hnmod` (42,476 B, embeds the content) | `oritech.hmod` (**56,147 B**, native + content) | `retired-hnmod-0.1.0/oritech.hnmod` |
| `compatibility.json`, `rust-toolchain.toml` | `[native]` in `horizon-mod.toml`; the pinned SDK decides the toolchain | `retired-hnmod-0.1.0/` |
| `tools/horizon.py`, `tools/package.py`, `tools/package.sh`, `tools/add-notices.py` | **gone** (staging and `.hnmod` packaging belong to the retired format) | `retired-hnmod-0.1.0/tools/` |
| `Cargo.lock` (the old crate's) | deleted; the new crate resolves **its own** lock, which **ships** (trap 1 below) | — |

**Kept and updated:** `tools/carry_content.py` (now writes `assets/` + `data/` at the package
root and no longer writes a manifest — the root `horizon-mod.toml` is the only one),
`tools/parse_info.py`, `tools/e2e.sh` (launcher-built game; see the gap list), `FINDINGS.md`,
`CONTENT.md`, `PLAN.md`, `notes.md`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md` and
`AGENTS.md` (both rewritten for the flat layout — trap 6).

### Manifest mapping

- identity: `id = "oritech"` (also the resource namespace), `name`, `version = "0.2.0"`
  (the port's version, not the jar's 1.2.13), `authors = ["Rearth", "AdLf0208"]`,
  `license = "CC0-1.0"` in `Cargo.toml`.
- `api = "0.2"` (exact), `side = "both"` (single player's integrated server belongs to the
  client installation, so a LAN host and single player need it),
  `capabilities = ["registries"]` (the one shared value; this package really does declare two
  blocks and their block entity types).
- `[native] game = "26.3"`, `horizon = "0b14f8a…"`, `mixins = "e59da90…"` replace
  `[compatibility] horizon_revision` and the bundle's mixins revision (`5a64af93…` →
  `e59da909…`).
- `[mod] priority = 100` is gone; nothing in this slice needs ordering
  (`#[listener(priority = …)]` is the new lever).

### Helper → hook table (every injection and every module, each with a home)

| 0.1.0 `[[injection]]` / module | 0.2.0 | Mapping |
|---|---|---|
| `[[module]] runtime` (`runtime.rs`, injected as `crate::game::oritech`) | `pub mod runtime;` in `src/lib.rs` | an ordinary Rust module now; the file is byte-identical |
| `[[module]] state` (`support.rs`, `crate::game::oritech_state`) | `mod support;` | the glue is an ordinary private module; no `//!`/`#![…]` in it, so no `E_MODULE` |
| `block_entity` (15) | `BlockEntityPart::block_entity` on `GeneratorPart` / `PulverizerPart`, attached to the types declared by `Registrar::block_entity_type(…).blocks(…)` | the type owns the id now, so the method answers an empty compound only |
| `block_entity_ticks` (15) | `BlockEntityPart::block_entity_ticks` → `event.ticks = true` | unchanged |
| `block_entity_container` (16) | **no hook**: `Registrar::block_container(&block, support::container(block))` in `Hooks::register` | a declaration keyed by the block; the retired hook's shared `&mut Option<ModContainer>` is gone, so a foreign block cannot be touched |
| `block_entity_energy` (142) | **no hook**: `Registrar::block_energy(&block, support::battery(block))` | same numbers: generator 50,000 / 0 in / 256 out; pulverizer 25,000 / 256 in / 0 out |
| `block_entity_tick` (17) | `BlockEntityPart::block_entity_tick` | `event.pos`, `event.entity.nbt`/`.items`; the Part answers `Unchanged` when nothing moved (trap 4) |
| `level_tick_post` (23) | `Hooks::level_tick_post` → `support::on_level_tick_post(host, event.level)` | `native_move_energy` → `move_energy` over the level's `ENERGY` handles, in one transaction per side |
| `right_click_block` (21) | `Hooks::right_click_block` → `support::right_click` | the pre-block, cancellable route; `canceled = true` is `Flow::Cancel` + `InteractionResult::Success` |
| `register_commands` (8) | `Hooks::register_commands` (`event.commands`) | the same two usages, typed (`<pos:block_pos>`, `<amount:integer>`) |
| `perform_command` (8, a raw line) | `Hooks::run_mod_command` (`event.command.literals` + typed `argument(…)`) | `Flow::Cancel` = handled; the readout text is byte-identical (tested) |
| `world_lifecycle` (44) | `Hooks::world_lifecycle` → `support::reset_world_state()` | drops the recipe cache and the push queue |
| `embedded-content` (`content_root` HEAD, cancellable) | **none — an explicit gap closed by the format** | one `.hmod` carries `assets/` + `data/`; the cache write, the `native_data!` blob and its control leg have no purpose |

`uses.json` in the package is the audit of that table: **394 surface entries**, 8 hooks
(`block_entity`, `block_entity_ticks`, `block_entity_tick`, `level_tick_post`,
`right_click_block`, `register_commands`, `run_mod_command`, `world_lifecycle`), 3 Part
methods, the two `DefaultCapabilitiesRegistrar` declarations, `fn:move_energy`,
`EnergyHandler::{amount,capacity,insert,extract}`, the host handles
(`block_entity`, `item_slot`/`set_item_slot`/`merge_data`/`set_changed`, `recipes_of_type`,
`recipe_ingredient`, `item_burn_time`, `max_stack_size`, `item_crafting_remainder`), both
macros and the `horizon:energy` + `registries` capabilities
(`evidence/oritech/sdk-uses-summary.log`).

`runtime.rs` is untouched: `Generator`, `Pulverizer`, `PulverizerRecipe`, `can_output`,
`output_fits`, `ingredient_json`, `clamp_energy`, `is_fuel`, `Work`, `Step`, `RecipeResult`,
`RecipeError` and the two capacities are called exactly as before; the glue only converts
between the SDK's types and theirs.

### The ladder (measured 2026-11-18, every step in a `build-slot.sh` lease)

`build-slot.sh` still speaks the retired tool, so it is used as the **slot guard only**
(`BUILD_LOCAL=1`, steps `test|lint|fmt|build|check`); there is no staging step and no staged
tree — the crate is compiled in place against `.horizon/sdk/`. `CARGO_INCREMENTAL=0`, no
`--target-dir`. Logs in [`../_shared/evidence/oritech/`](../../_shared/evidence/oritech/).

| Step | Command | Result |
|---|---|---|
| runtime carried, before glue | `cargo test --offline` with only `pub mod runtime;` | **18 passed**, 0 failed (`sdk-runtime-first.log`) |
| glue red | same, glue bodies `todo!()` | **7 failed**, 18 passed (`sdk-glue-red.log`) |
| tests | `cargo test --offline` | **25 passed** (18 `runtime::`, 7 `support::`), 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`sdk-clippy.log`) |
| format | `cargo fmt --check` (**not** `--all`) | clean, empty diff (`sdk-fmt.log`) |
| package | `cargo horizon-mod build` | `Built ./target/horizon-mod/oritech.hmod` (`sdk-build.log`) |
| same package twice | build again after deleting the artifact | **56,147 B**, SHA-256 `0d4e2d05b07b16746a04f15d5a64135829bfba115e37303896ffa5eaddb3f1bc` **both times** (`sdk-build-second.log`, `sdk-sha256.txt`) |
| registry check | `cargo horizon-mod check oritech.hmod` | `oritech 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)` / `56147 bytes … unsigned`, **exit 0** (`sdk-check.log`) |
| fits the pinned build | `cargo horizon-mod fits oritech.hmod --build nightly` | `Oritech:` / `0b14f8a  made for`, **exit 0** (`sdk-check.log`) |
| `runtime.rs` untouched | `unzip -p oritech.hmod src/runtime.rs \| sha256sum` | `0451000b…661e` = `src/runtime.rs` = the retired file (`sdk-runtime-sha256.txt`) |
| content carry unchanged | `python3 tools/carry_content.py --check` | `25 files, 0 differ` (`sdk-content-carry.log`) |

The archive is **35 entries**: `horizon-mod.toml`, `Cargo.toml`, `Cargo.lock`, `LICENSE`,
`LICENSE-NOTICE.md`, `README.md`, `src/{lib,runtime,support}.rs`, `uses.json`, `assets/…`
(11) and `data/…` (14). `notes.md`, `FINDINGS.md`, `CONTENT.md`, `PLAN.md`, `AGENTS.md` and
`tools/` are **not** packed (the tool packs a root-document whitelist), so the package ships
no working notes; `LICENSE` and `LICENSE-NOTICE.md` are inside it.

### Traps this migration hit (new to this port; the next one must know)

1. **A registry dependency makes the lock a package file, not a leak.** The carried
   `runtime.rs` parses the recipe JSON with `serde_json`, so the crate needs
   `serde_json = "1"` and `cargo horizon-mod build` **packs `Cargo.lock`** — the opposite of
   `imfast`'s trap 5. Here the lock *is* this package's record of its pins (resolved
   `serde_json` 1.0.154 from this box's cache) and belongs inside it, exactly as
   `securitycraft` recorded for `sha2`. A future builder additionally needs
   `horizon-launcher native-lock .` on the machine that builds the game (gap 2).
2. **A block-entity tick's payload edits are discarded by `Unchanged`.** `event.reply`
   defaults to `Changed` and `*event.reply = Unchanged` throws away **this Part's** `nbt` and
   `items` writes. A machine that answers `Unchanged` while its energy moved would silently
   lose the tick. Both Parts compare the state before/after (plus a resize and a fuel
   change) and answer `Unchanged` only when nothing moved, so an idle machine stops dirtying
   its chunk without ever dropping a write.
3. **`host.levels()` is not on `Host` alone** (`E0599: no method named 'levels'`): it needs
   the `ServicesHost` trait in scope. In a tick the simpler and correct read is
   `host.level(host.dimension())` — a block-entity tick always runs in the level in place —
   which is what `support::in_place_level` does.
4. **`host.transaction(…)` needs `HostTransaction` imported** (`E0599` with the misleading
   hint `transaction_open`). One `use mod_api::HostTransaction;` fixes it.
5. **`max_stack_size` takes an `ItemKey`, not an id.** The pure `can_output` asks for a max
   stack *by item id* (recipe results name ids), so the glue resolves it:
   `Identifier::parse(id)` → `host.item_key(&id)` → `host.max_stack_size(key)`, 64 when
   unknown. `ItemResource::of(&stack)` is the same shape for the click's extract.
6. **A root document that still describes `mixins/` ships inside the package.** `build`
   packs `README.md`; the migrated one is part of the change, not a nicety. `AGENTS.md` is
   not packed but was rewritten too, because it told the next reader to run `tools/horizon.py`.
7. **The recipe table is a published-resources read.** `host.recipes_of_type("oritech:pulverizer")`
   returns `Arc<[Recipe]>` whose `json` is the file as written; the pure parser still reads
   that text, and the ingredient is resolved from its JSON via
   `host.recipe_ingredient(&json.to_string())`. The 40-tick cache from 0.1.0 is kept (a
   `/reload` lands within two seconds) and cleared at `world_lifecycle`.
8. **`Ingredient` is cacheable but only testable with a host.** It is plain data
   (`HashSet`, shapes, a rule), so it lives in the `static Mutex<Option<Vec<Resolved>>>`; its
   `test(host, stack)` needs the host at match time and must not be called per candidate
   without one.
9. **A Part is per block-entity type, not per block id.** The retired tick read `entity.id`
   (the block id) to pick a machine; now each type has its own Part, so the tick never
   branches on an id. Only the readout needs the block id, and it reads
   `level.block(host, pos).id(host)`.
10. **The dropped click needs `InteractionResult::Success`, not just `Flow::Cancel`.** A bare
    cancel answers the default `Pass`, which would let the held block's own use run; the
    retired `canceled = true` ended the click. `Success` is the same end the 0.1.0 hook had.
11. **A creative hand gives for free through `ItemPlace`.** `PlayerHand.extract` already
    gives without losing any for a creative player, so the retired `if !game.creative` check
    is gone; the insert and the take share one transaction, so a refused extract leaves the
    slot unchanged.
12. **The retired `setChanged` fake disappears.** `merge_data` (in `/oritech energy`) marks
    the chunk changed, and a `Changed` tick payload is the write-back; `set_changed` is still
    called after a click write.
13. **`cargo horizon-mod check` with no file compiles wasm** (`can't find crate for 'core'`).
    The package check is `check <file.hmod>`; `cargo test`/`cargo clippy` are the native
    gate.
14. **`cargo fmt --all` reports the fetched SDK.** `cargo fmt --check` only. `src/runtime.rs`
    was never formatted by `cargo fmt`: `rustfmt --edition 2024 src/lib.rs src/support.rs`
    was run explicitly and the runtime hash re-checked afterwards.
15. **No `rust-toolchain.toml`.** The package deliberately has none; the pinned SDK (rust
    1.99.1 declared) and the launcher decide, and this crate compiled here with rustc 1.97.0,
    edition 2024.
16. **The retired e2e cannot run here.** It needs a game built by the launcher with this
    package (`horizon-launcher native-build … --trust-native-code`) and no launcher is
    installed, so **no 0.2.0 in-game leg is claimed** (gap 1).

### Behaviour that changed with the format (deliberate, and why)

- **The embedded-content control leg is gone** (no gap: the format carries the content). The
  retired `EMBEDDED=1` leg proved the bundle wrote its own `.hmod` out; there is no such file
  any more, and the same ids load from `data/` inside this package.
- **Usage messages for a malformed command now come from the dispatcher.** The retired
  `perform_command` printed `usage: /oritech info <x> <y> <z>` itself; `run_mod_command`
  receives typed arguments, and vanilla answers a wrong arity. The readout lines themselves
  are byte-identical and tested (`the_info_lines_are_the_ones_the_e2e_asserts`,
  `a_working_pulverizer_prints_its_recipe_and_input`).
- **`tools/e2e.sh` now launches a launcher-built game.** `GAME=` is required and there is no
  `--dev-mod`; `CONTROL_GAME=` (with `CONTROL=1`) is the differential leg — a build without
  this package, where both ids must be unknown. `SELFTEST=1` still flips the exact-FE
  assertion, so the scenario is still fail-capable.

### Gaps after the migration

1. **No in-game re-run in this environment.** No launcher and therefore no `0b14f8a` client:
   `tools/e2e.sh` needs `GAME=<launcher-built horizon-client>`. The four 0.1.0 runs
   (`evidence/oritech/e2e.log`, `e2e-selftest.log`, `e2e-embedded.log`,
   `e2e-embedded-selftest.log`) are the **historical** evidence for a `runtime.rs` that did
   not change; what changed is the transport of the same calls, not the rules.
2. **A registry dependency needs the launcher's lock step.** The machine that builds the
   game must run `horizon-launcher native-lock .` for `serde_json` (the shipped `Cargo.lock`
   records the resolution; the launcher re-resolves against its own crates). Recorded, not
   worked around.
3. **The block entity's type id and the block id differ** (`oritech:basic_generator` vs
   `oritech:basic_generator_block`). Any later code that looks a machine up by type must not
   assume the block id; the readout uses the block.
4. Everything in *Deviations and gaps (slice 1)* still stands: no GUI/screen, no GeckoLib
   rendering, no crafting recipes for the machines, 117 of 124 pulverizer recipes, no
   addons/redstone/comparator, and the multiblock machines, the accelerator and the ores are
   later slices.

`## Upstream defects fixed`: none found in this migration. The 0.1.0 repair (a recipe
result larger than one stack into an empty slot) was a port defect, already fixed and
covered by `an_empty_output_slot_rejects_a_result_larger_than_one_stack`.

### Changelog for the resubmission (supersedes #883)

> Supersedes **#883**, closed on 2026-11-18 because Horizon retired the `.hnmod` container
> (#16941; PR #17992 cleans up the old files). 0.2.0 is the flat SDK package: `api = "0.2"`,
> `[native]` pinned to nightly `0b14f8acfd97fa2c750e42795249dcc981178ca4` and mixins
> `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, launcher 0.2.78, for 26.3. The eleven old
> injections became eight typed hooks (`block_entity`, `block_entity_ticks`,
> `block_entity_tick`, `level_tick_post`, `right_click_block`, `register_commands`,
> `run_mod_command`, `world_lifecycle`) plus `Registrar::block_container` and
> `Registrar::block_energy` declarations and two block-entity Parts; the content half now
> travels in the same package instead of an embedded `.hmod`; `src/runtime.rs` and its 18
> tests are unchanged, byte for byte. Package: 35 entries, **56,147 B**, sha256
> `0d4e2d05b07b16746a04f15d5a64135829bfba115e37303896ffa5eaddb3f1bc`; `check` and `fits`
> exit 0; two builds give the same bytes. No 0.2.0 in-game run: the launcher is not
> installed here (gap 1).

Skills: superpowers/using-superpowers,writing-plans,test-driven-development,verification-before-completion; BeautifulRust/rust-best-practices

## Review fixes (2026-11-18) — registry source review #1218

Skills: superpowers/using-superpowers,systematic-debugging,test-driven-development,verification-before-completion; BeautifulRust/rust-best-practices

File: `var/review-all/1218-oritech.md`. Package **0.2.1**, `oritech.hmod` 54,808 B, sha256
`de762c4efd93b034561223d5ddb18b9c8fa44765ccb332069fe7fa12b5fc6d73`.

1. **`src/support.rs:297` the pulverizer ignored the SDK's refusal to set a count to zero, so the
   last input returned to its slot after the output was granted and a powered machine produced
   from the same item for ever.** `finish_operation` now consumes the input **before** granting
   the output: `runtime::after_consuming(count)` is `None` for the last item (the slot stays
   empty, which is how the SDK represents an empty slot), `Some(left)` otherwise; a count the
   stack refuses aborts before any output. Pinned by
   `consuming_the_last_input_empties_the_slot_instead_of_keeping_it`.
2. **`src/support.rs:460` the right-click hook ran for spectators.** The acting player's game mode
   is now checked before the click is claimed or written: a spectator's click returns
   `Flow::Continue` and changes nothing. Pinned in game by phase **F2** (a spectator's coal click
   leaves the generator's fuel stack unchanged). `cargo test`: **26 passed**.

**Found by the in-game leg (fixed here, not in the reviewer's list).** The carried
`data/oritech/recipe/pulverizer/*.json` declare `oritech:pulverizer`, but the port never registered
a serializer for it, so the game refused every file at load (*"unregistered recipe serializer
oritech:pulverizer"*) — a hard startup crash, so the 0.2.0 package could not run. `register` now
declares the type and an object-preserving raw serializer; the port keeps reading the JSON with its
own matcher. The e2e feeds the machines by the player's right click (a mod block entity's slots are
not in the engine's vanilla container map, so `ctl container` cannot reach them) and reads the
`/oritech info` line, which already reports `input`/`out0`/`out1`. In-game: `PASS` + `E2E EXIT:0`
(`evidence/oritech-e2e/`), control `EXIT:1`; phase **F2** is the spectator.

## Slice 2 — the Powered Furnace multiblock and the machine cores (0.2.2, 2026-11-21)

**Why this family:** `notes.md`'s gap list names it first among the unported machines — gap **13**:
"Not carried: the powered furnace (a multiblock machine that needs its core assembled), the assembler,
atomic forge, foundry, refinery, centrifuge, cooler, the accelerator ring, the reactor, the augment
stations, the jetpack and equipment, the resource-node worldgen and the ores." `FINDINGS.md` lists the
same machine among the `MultiblockMachineEntity` family. It is the **system**, not one machine: the
core block, the assembled gate, the core-position lookup and "a broken core disassembles" are what the
foundry, refinery, atomic forge, deep drill and reactor all reuse. It also closes the hole slice 1
left: a generator can now drive a machine that turns ore into ingots.

**Class + tier** unchanged: class `content`, tier `deep` (`ports/ledger.csv`), delivered in the
**machines** shape this slice needs (a block entity, a declared container and FE buffer, a multiblock
assembly step, one recipe family read from the vanilla `minecraft:smelting` table). No renderer, no
GUI, no energy network — nothing beyond the tier.

### The jar's numbers (upstream `rearth/oritech` v1.2.13, commit `4069b6d`)

| Value | Upstream | This port |
|---|---|---|
| capacity 10,000 FE | `OritechConfig.FurnaceConfig.energyCapacity` (`init/OritechConfig.java:497`) | `FURNACE_CAPACITY` |
| insert 256 FE/t (`32 * 8`) | same, `:498` | `FURNACE_MAX_INSERT` |
| 32 FE a working tick | same, `energyPerTick` `:499`, and `calculateEnergyUsage` = `energyPerTick * efficiency(1) * (1/0.5) / 2` | `furnace_energy_usage()` |
| ×0.5 speed (half a vanilla furnace's time) | same, `speedMultiplier` `:500`; `PoweredFurnaceBlockEntity.getSpeedMultiplier` | `FURNACE_SPEED` |
| duration = `cookingTime * speed` (200 → **100** ticks) | `PoweredFurnaceBlockEntity.furnaceCraftingFinished` | `furnace_ticks()` |
| one core at offset **(0, 1, 0)** | `PoweredFurnaceBlockEntity.getCorePositions` | `CORE_OFFSET` |
| slots `(0, 1, 1, 1)`, inventory 2 | `getSlotAssignments` / `getInventorySize` | container: input 0, output 1 |
| seven cores, quality = the tier | `BlockContent.MACHINE_CORE_1..7` (`MachineCoreBlock(…, n)`) | `CORES` / `core_quality()` |
| a broken core disassembles, progress kept | `MultiblockMachineController.onCoreBroken` | the tick's core re-check |
| an unassembled machine does nothing | `PoweredFurnaceBlockEntity.serverTick` → `isActive(state)` | the tick's assembled gate |
| the recipe is vanilla smelting's first match | `getRecipeFor(RecipeType.SMELTING, …)` | `SmeltingRecipe::parse` + the engine's ingredient test |

### What was added

- `src/runtime.rs`: `FURNACE_*`, `CORE_OFFSET`, `CORES`, `core_quality`, `furnace_ticks`,
  `furnace_energy_usage`, `Furnace`, `SmeltingRecipe`/`SmeltError`. `Pulverizer::tick` and
  `Furnace::tick` now share one private `advance` step, so the two machines cannot drift apart; the
  pulverizer's own tests and behaviour are unchanged.
- `src/support.rs`: the furnace's container and buffer declarations, `FurnaceSaved`
  (`connectedCores` + `coreQuality` + the assembled flag), `tick_furnace` (assembly re-check, the
  smelting lookup, the exact consume-then-grant), `place_core`/`try_assemble` on the right click, the
  readout line, the third buffer in `/oritech energy`, and one cached recipe lookup for both tables.
- `src/lib.rs`: `FurnacePart` and the third block-entity type.
- Content: `oritech:powered_furnace_block` and `oritech:machine_core_1..7` — blockstates, models,
  item models, textures, 26.3 item definitions, block declarations, loot, lang, the machines tab and
  the pickaxe/stone-tool tags. `assets/` + `data/` are now **81 files** (`tools/carry_content.py
  --check` → `81 files, 0 differ`).
- `tools/{parse_info.py,e2e.sh}` extended; `PLAN-slice2.md` is the slice's plan; `VALIDATION.md` is
  the ladder.

### Deviations (slice 2)

1. **Assembly state is the block entity's saved compound, not a blockstate.** Upstream keeps
   `machine_assembled` on the block and `core_used` on the core (which also renders the core
   invisible). The pin can declare both as `state_properties`, but this slice keeps them in
   `oritech.multiblock.assembled` on the controller and re-checks the core block each tick instead:
   the core is visible either way here (no GeckoLib renderer), and one source of truth cannot
   disagree with itself. The core keeps upstream's `connectedCores`/`coreQuality` keys, so the
   saved shape is upstream's.
2. **The `lit` state is not declared.** `PoweredFurnaceBlock` has `LIT` and light level 15; with a
   `cube_all` model and no light support in the carried declaration the state has no renderer to
   drive — the same recorded gap as slice 1's `facing`.
3. **Container and buffer exist while unassembled.** The declaration API is per block; upstream
   registers the capabilities only after `initMultiblock`. The machine does nothing until assembled
   (the tick's gate, proven in phase H2), but its declared buffer can be filled by
   `/oritech energy` or a cable.
4. **The core block is not clickable.** Upstream forwards a click on a used core to its controller
   (through `MachineCoreEntity`); here the core is a plain block and the controller is the click
   target. Nothing depends on it.
5. **Extra chambers/addons are not ported** (gap 9 stands): one item is smelted per completion
   (`getBaseAddonData().extraChambers()` is 0 without a processing addon).
6. **`coreQuality` is stored and shown, not used**: upstream uses it to widen the addon scan, and
   addons are not ported.

### Verification (0.2.2)

Every Rust and package step ran in a `build-slot.sh` lease (`BUILD_LOCAL=1`); the launcher build is
outside the slot (it has its own global lock and its own cache). Logs in
[`../_shared/evidence/oritech/`](../../_shared/evidence/oritech/) and
[`../_shared/evidence/oritech-e2e/`](../../_shared/evidence/oritech-e2e/).

| Step | Command | Result |
|---|---|---|
| runtime + glue tests (red) | `cargo test --offline`, new bodies `todo!()` | **26 passed, 7 failed** (`slice2-test-red.log`, every failure `panicked at src/runtime.rs:…: not yet implemented`) |
| glue tests (red) | same, glue stubs | **35 passed, 3 failed** (`slice2-glue-red.log`) |
| tests (green) | `cargo test --offline` | **38 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo fmt --check` + `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`sdk-clippy.log`) |
| package | `cargo horizon-mod build` (no `Cargo.lock` in the tree) | **80,939 B**, sha256 `9af384e8…b037eefc`, **90 entries**, no `Cargo.lock` (`slice2-build.log`) |
| same package twice | delete the artifact, rebuild | `cmp` **identical**, same sha256 both times |
| registry check | `cargo horizon-mod check oritech.hmod` | `oritech 0.2.2 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)` / `80939 bytes … unsigned`, exit 0 |
| fits the pin | `cargo horizon-mod fits oritech.hmod --build 0a875da` | `0a875da  made for`, exit 0 |
| fits **another** build | `cargo horizon-mod fits oritech.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (the previous pin, so the check is not a tautology) |
| package gate | `python3 ports/tools/verify-sdk-package.py oritech` | **1/1** |
| e2e | `launcher-game.sh build oritech`, then `build-slot.sh run --step e2e` | **PASS** + `EXIT:0` (`e2e-slice2.log`) |
| e2e control (whole scenario on the package-less build) | `GAME=$CONTROL_GAME …` | **EXIT:1** — `the generator did not place` (`e2e-slice2-control.log`) |
| e2e in-run differential | `CONTROL=1` | the control game has no generator item and the generator, **furnace and core** ids stay `minecraft:air` (`e2e-slice2-differential.log`) |
| fail-capability, slice 1 | `SELFTEST=1` | **EXIT:1** (`89 ticks spent 2848 FE, expected 2880`) |
| fail-capability, furnace | `SELFTEST_FURNACE=1` | **EXIT:1** (`90 ticks needed exactly 2880 FE, so the buffer should hold 32, not 0`) |
| log checker | `check-e2e-logs.py oritech-e2e --strict` | 7 logs, 7 ok, 0 bad |

The furnace leg is fail-capable four ways: the no-core phase (a machine with ore and 5,000 FE does
nothing), the ten-tick arithmetic (320 FE must advance exactly ten ticks), the exact-FE completion
(a faster machine leaves FE behind; a slower one starves with 32 FE left and makes nothing), and
`SELFTEST_FURNACE=1`, which flips the last one and exits 1.

### In-game transcript (quoted from `e2e-slice2.log`)

```
  without a core: 5000/10000 FE after 60 ticks, progress 0, core empty
  unassembled, with ore and 5,000 FE: still progress 0, 5000 FE, no ingot
  core click: {"ok":true,"block":"oritech:powered_furnace_block"}
  block(8,101,0) = oritech:machine_core_2
  info(8,100,0): Powered Furnace at [8, 100, 0]: 3880/10000 FE, assembled yes core oritech:machine_core_2 quality 2, progress 35 recipe minecraft:iron_ingot_from_smelting_iron_ore, input minecraft:iron_ore x1, out empty, t=1577
  assembled: core oritech:machine_core_2 quality 2, progress 35
  info(8,100,0): Powered Furnace at [8, 100, 0]: 1800/10000 FE, assembled yes core oritech:machine_core_2 quality 2, progress 0 recipe none, input empty, out minecraft:iron_ingot x1, t=1728
  5000 -> 1800 FE, exactly 3200 for one ingot, out0 minecraft:iron_ingot x1
  320 FE advanced the smelt exactly 10 ticks (0 -> 10 of 100)
  from progress 10: 90 ticks spent exactly 2880 FE, buffer 0, out0 minecraft:iron_ingot x2
  info(8,100,0): Powered Furnace at [8, 100, 0]: 9008/10000 FE, assembled no core empty quality 0, progress 31 recipe minecraft:iron_ingot_from_smelting_iron_ore, input minecraft:iron_ore x1, out minecraft:iron_ingot x2, t=2189
  core broken: assembled no, progress held at 31 for 60 ticks, 9008 FE unspent
  info(8,100,0): Powered Furnace at [8, 100, 0]: 6800/10000 FE, assembled yes core oritech:machine_core_4 quality 4, progress 0 recipe none, input empty, out minecraft:iron_ingot x3, t=2403
  reassembled with core 4 and finished the smelt it had started: out0 minecraft:iron_ingot x3
  7 cores placed and read back, machine_core_8 refused, the furnace item given
  oritech:pulverizer/bone: minecraft:bone -> minecraft:bone_meal x6
  oritech:pulverizer/blaze: minecraft:blaze_rod -> minecraft:blaze_powder x3
  oritech:pulverizer/dripstone: minecraft:dripstone_block -> minecraft:pointed_dripstone x4
  oritech:pulverizer/glowstoneore: minecraft:glowstone -> minecraft:glowstone_dust x3
  oritech:pulverizer/coalore: minecraft:coal_ore -> minecraft:coal x2
  oritech:pulverizer/diamondore: minecraft:diamond_ore -> minecraft:diamond x1
  oritech:pulverizer/lapisore: minecraft:lapis_ore -> minecraft:lapis_lazuli x6
PASS: … a powered furnace with no core smelted nothing even with ore and 5,000 FE in it, its core
arrived by a right click and assembly gated it, 320 FE advanced a smelt exactly ten ticks, iron ore
became an iron ingot for exactly 3,200 FE, breaking the core disassembled the machine mid-smelt and
kept its progress and reassembly resumed it, every new block placed and read back, every one of the
seven carried pulverizer recipes crafted with its input consumed
EXIT:0
```

### What slice 2 covered, and by which phase

(The machine-checked table for all three slices is under *Slice 3* → *Parity coverage*.)

| Family | Where |
|---|---|
| `oritech:powered_furnace_block` block + item | phase M (given + read back), H (placed), I (clicked) |
| `oritech:machine_core_1..7` blocks + items | phase M places all seven and reads each back; `machine_core_8` is refused |
| multiblock assembly (controller + core) | phase I: a core in the hand is placed above by a right click, the empty-hand click assembles, the readout is `assembled yes core … quality 2` |
| disassembly on a broken core | phase L: progress frozen for 60 ticks with FE unspent, then core 4 re-assembles and the smelt finishes |
| vanilla smelting recipe family | phase J: `minecraft:iron_ingot_from_smelting_iron_ore` (200-tick recipe) → 1 ingot in exactly 100 ticks for exactly 3,200 FE; the table is the game's own, so any smelting recipe runs |
| the furnace's FE buffer (10,000 / 256 / 0) | `H`/`J` readouts and `/oritech energy` writes; declaration pinned by a unit test |
| `oritech:pulverizer/bone` | phase D (100 ticks, 3,200 FE) and O |
| the other six carried pulverizer recipes | phase O: each crafted with its declared input, output id and count asserted, input consumed |
| the remaining 117 pulverizer recipes | not carried: they need the ore-processing items (dusts/clumps/gems) the port does not carry (gap 6) |
| GUI/screen, GeckoLib render, addons, energy pipes, the other multiblocks | not ported; recorded gaps |

### Carry defect fixed: the object-form ingredient

- **What it was.** The seven carried `data/oritech/recipe/pulverizer/*.json` were copied byte for byte
  from the jar, and a 1.21.1 recipe writes its ingredient as an object: `{"item": "minecraft:bone"}`,
  `{"tag": "minecraft:coal_ores"}`.
- **Why 26.3 rejects it.** The recipe loader reads ingredients in the bare form — an item id or
  `#tag`, or a list of them — and an object is refused at load (*"invalid ingredient (unresolved tag
  or malformed input)"*), so the file is dropped and the recipe can never match. The port's own cache
  reads the table, so the recipe was silently absent: the machine showed `recipe none`.
- **What changed.** `tools/carry_content.py` now rewrites the ingredient list only
  (`{"item": …}` → `"minecraft:…"`, `{"tag": …}` → `"#minecraft:…"`); results, time and type stay the
  jar's bytes-as-JSON, and the change is deterministic (`--check` → `81 files, 0 differ`).
- **What a user notices.** The seven recipes exist again: phase O crafts every one — bone → 6 bone
  meal, blaze rod → 3 blaze powder, dripstone → 4 pointed dripstone, glowstone → 3 glowstone dust,
  `#coal_ores` → 2 coal, `#diamond_ores` → 1 diamond, `#lapis_ores` → 6 lapis — and the input slot is
  empty afterwards. Phase N is the tag form end to end.
- Upstream report: not worth one (the jar targets 1.21.1, where the object form is valid; this is a
  data-pack format difference, not a mod defect).

### The in-game leg's own prerequisite

`cargo horizon-mod build` packs whatever `Cargo.lock` exists, and the gate requires none, so the tree
carries no lock; but `horizon-launcher native-build` needs one. The measured sequence is:

```sh
cd ports/oritech
CARGO_HOME=$HOME/.cache/horizon-e2e/root/data/horizon-launcher/cargo \
    cargo generate-lockfile --offline          # the game's own pins (syn 3.0.6, unicode-ident 1.0.26)
ports/tools/launcher-game.sh build oritech     # native-build + export the game and the control game
rm -f Cargo.lock                               # keep it out of the package
ports/tools/launcher-game.sh build oritech     # (not needed again; the game is cached)
eval "$(ports/tools/launcher-game.sh env oritech)"
ports/tools/build-slot.sh run --job W1-oritech --port ports/oritech --step e2e -- bash tools/e2e.sh
```

`horizon-launcher native-lock .` also writes a lock, but at this pin it resolved `unicode-ident
1.0.27` while the prebuilt game pins `1.0.26`, and `native-build` refuses the mismatch; generating the
lock from the launcher's own cargo home is what worked. The launcher has a **global** build lock
(`A native build or cache cleanup is running`), so a build can wait behind another port's.

`## Upstream defects fixed`: none found in this slice. The ingredient form was a carry mistake in this
port (a 1.21.1 file into a 26.3 pack), fixed here and covered in game.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

## Slice 3 — the Industrial Chiller (`oritech:cooler_block`), the mod's fluid machine (0.2.5, 2026-11-21)

**Why this family**, from `notes.md`'s own gap list — *Deviations and gaps (slice 1)*, gap **13**,
verbatim: "Not carried: the powered furnace (a multiblock machine that needs its core assembled), the
assembler, atomic forge, foundry, refinery, centrifuge, cooler, the accelerator ring, the reactor, the
augment stations, the jetpack and equipment, the resource-node worldgen and the ores. All later
slices."

Every other machine named there (`assembler`, `centrifuge`, `atomic forge`, `foundry`, `refinery`,
`grinder`, the generators) consumes Oritech dusts, clumps, gems, components or mod fluids that this
port does not carry, so its recipes could never match — dead data, which the campaign forbids shipping.
The **cooler** is the one machine family whose whole process runs on vanilla content: `minecraft:water`
→ 3 `minecraft:ice` and `minecraft:lava` → 2 `minecraft:obsidian`, both of them fluids the engine's
fluid handler knows. It is also the first **fluid** machine, and its core sits at a *different* offset
from the furnace's, so it generalises the multiblock assembly slice 2 built instead of duplicating it.
Class + tier unchanged: class `content`, tier `deep` (`ports/ledger.csv`), delivered in the machines
shape (a block entity, a declared container, a declared FE buffer, a declared **fluid tank**, a recipe
family, a multiblock assembly). No renderer, no GUI, no dimension, no accelerator.

### The jar's numbers (upstream `Rearth/Oritech` v1.2.13, commit `4069b6d`)

| Value | Upstream | This port |
|---|---|---|
| capacity 50,000 FE | `OritechConfig.processingMachines.coolerData` (`init/OritechConfig.java:360`) | `COOLER_CAPACITY` |
| insert 256 FE/t (`32 * 8`) | same | `COOLER_MAX_INSERT` |
| 32 FE a working tick | same `energyPerTick`; `calculateEnergyUsage` = 32 × efficiency(1) × (1/speed(1)) | `cooler_energy_usage()` |
| 200 ticks a craft | the jar's `data/oritech/recipe/cooler/*.json` `time: 200`; `checkCraftingFinished` = `time * speed` | `cooler_ticks(time)` |
| tank 4 buckets | `CoolerBlockEntity.fluidStorage = SimpleFluidStorage(4 * bucketAmount())` | `COOLER_TANK = 4000` |
| one bucket a craft | `FluidIngredient`'s `amount` defaults to `FluidStack.bucketAmount()` | `FluidInput::amount` |
| core at **(0, 0, -1)** | `CoolerBlockEntity.getCorePositions` | `COOLER_CORE_OFFSET` |
| slots `(0, 0, 0, 1)`, inventory 1 | `getSlotAssignments` / `getInventorySize` | container: one output, no item input |
| output rule `count + result <= 64` | `CoolerBlockEntity.processCraftInstance` | `cooler_output_fits` |
| cold biome: speed ×0.5 **and** efficiency ×0.5 | `getSpeedMultiplier` / `getEfficiencyMultiplier` | `blocked(no-biome-tag-read)` — see deviations |

### What was added

- `src/runtime.rs`: `COOLER_*`, `cooler_energy_usage`, `cooler_ticks`, `FluidInput` + `tank_matches`
  (`recipeInputMatchesTank`), `cooler_output_fits`, `after_draining`, `CoolerRecipe::parse` /
  `CoolerError`, and `Cooler` — which advances through the **same** private `advance` step as the
  pulverizer and the furnace, so three machines cannot drift apart. Ten new unit tests.
- `src/support.rs`: the cooler's container/buffer/**fluid-tank** arms, `Assembly` (the multiblock
  facts both machines now share), `CoolerSaved` with its tank in the game's own `fluids` shape,
  `tick_cooler` (assembly re-check, the tank's recipe, the 64-item rule, the drain-then-grant), the
  cooler recipe cache (no `Ingredient`: a cooler recipe has no item input), `core_offset_for`,
  `/oritech fluid`, `set_energy`'s cooler arm and the readout line. Its own eight tests pin the
  declarations, the saved shape and the exact lines.
- `src/lib.rs`: `CoolerPart`, the fourth block-entity type, `Registrar::block_fluids` for the tank,
  the `oritech:cooler` recipe type + raw serializer, and `/oritech fluid <pos> <amount> <fluid>`.
- Content: `oritech:cooler_block` (blockstate, block and item models, its own texture, 26.3 item
  definition, block declaration, loot, lang `Industrial Chiller`, the machines tab, the pickaxe and
  stone-tool tags) and the jar's two cooler recipes whose fluid and result both exist here.
  `assets/` + `data/` are now **90 files** (`tools/carry_content.py --check` → `90 files, 0 differ`).
- `tools/parse_info.py` learned the chiller's and the fluid reply's shapes; `tools/e2e.sh` gained
  phases P–W; `PLAN-slice3.md` is the slice's plan; `VALIDATION.md` is the ladder.

### Deviations (slice 3)

1. **The cold-biome bonus is not ported** — `blocked(no-biome-tag-read)`. Upstream halves the speed
   *and* the efficiency in a biome of `TagContent.CONVENTIONAL_COLD`, which leaves the usage at 32
   FE/tick and halves the duration (a 200-tick craft becomes 100 ticks for 3,200 FE). The pin's mod
   API has no biome read at all (no `biome` surface item in `sdk.json`'s 749 hooks), so this port
   always runs the warm figures. The semantic difference is stated in `runtime::cooler_energy_usage`.
2. **The tank is saved in the game's `fluids` shape, not upstream's `fluid` tag.** Upstream writes
   `fluidStorage.writeNbt(nbt, "")` (a `FluidStack` codec compound under the key `fluid`); API 54's
   fluid handler reads and writes a **list** under `fluids`, with `Fluid` + `Amount` per entry. The
   port uses the engine's shape, exactly as slice 1 used the API's `energy` int instead of
   `oritech.machine_energy` — otherwise the declared tank would be invisible to the engine and to
   every other mod's pipe.
3. **A fluid tag input can never match** — `blocked(no-fluid-tag-lookup)`. `tank_matches` refuses a
   `#tag` input rather than treating it as "any fluid", because the pin's mod API has no fluid-tag
   query. Only the jar's `snow.json` uses one (`#c:steam`).
4. **`snow.json` is not carried**: its input is `oritech:still_steam`, a fluid this package does not
   declare, so the recipe could never match. Recorded as `blocked(fluid-whitelist: oritech:still_steam)`
   and *measured* in game (phase U: the engine refuses the id, `accepted 0`).
5. **`/oritech fluid` is a harness control, not an upstream command.** Upstream fills the tank from its
   GUI or a pipe; there is no bucket route into a mod block entity at this pin, so the harness needs a
   way in. It goes through the engine's own `horizon:fluids` handler for the block, which means the
   reply is evidence of the *declaration*: a block that declared no tank accepts nothing.
6. **No GUI, no `lit` state, no GeckoLib render, no addons** — the same recorded gaps as slices 1–2.
   The chiller's addon slot at (0, 0, -2) and its `extraChambers` repeat are not ported (0 extra
   chambers without an addon).

### Verification (0.2.5)

Every Rust, package and in-game step ran in a `build-slot.sh` lease (`BUILD_LOCAL=1`,
`CARGO_INCREMENTAL=0`, no `--target-dir`); the launcher build is outside a slot (its own global lock).

| Step | Command | Result |
|---|---|---|
| runtime + glue red | `cargo test --offline`, the ten new bodies `todo!()` | **38 passed, 10 failed** — every failure `panicked at src/runtime.rs:…: not yet implemented` (`slice3-test-red.log`) |
| glue fail-capability (counterfactual) | nine targeted mutations of the declarations/readout, then the tests | **48 passed, 8 failed**, one per new glue test; the file restored byte-identically afterwards (`slice3-glue-red-counterfactual.log`, `tools/slice3-glue-counterfactual.py`) |
| tests (green) | `cargo test --offline` | **56 passed**, 0 failed (`sdk-test.log`) |
| lints + format | `cargo fmt --check` + `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`sdk-clippy.log`) |
| package | `cargo horizon-mod build` twice, the artifact deleted between | **94,227 B**, sha256 `89d0ae6f…2045ef3b`, `cmp` identical, **99 entries**, no packed `Cargo.lock` (`sdk-build.log`) |
| registry check | `cargo horizon-mod check oritech.hmod` | `oritech 0.2.5 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, exit 0 (`sdk-check.log`) |
| fits the pin | `cargo horizon-mod fits oritech.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`sdk-fits.log`) |
| fits **another** build | `… --build 0b14f8a` | `0b14f8a  fits`, exit 0 (the previous pin, so the check is not a tautology) |
| package gate | `python3 ports/tools/verify-sdk-package.py oritech` | **1/1** |
| content carry | `python3 tools/carry_content.py --check` | `90 files, 0 differ` (`slice3-carry-check.log`) |
| e2e | `launcher-game.sh build oritech` (manifest 0.2.5 ⇒ a real recompile), then `build-slot.sh run --step e2e` | **PASS** + `EXIT:0` (`e2e-slice3.log`) |
| e2e differential | `CONTROL=1` (the package-less build first) | **PASS** + `EXIT:0`; without this package the generator item is absent and the generator, furnace, core and **chiller** ids stay `minecraft:air` (`e2e-slice3-differential.log`) |
| e2e whole scenario on the package-less build | `GAME=$CONTROL_GAME` | **EXIT:1** — `the generator did not place` (`e2e-slice3-control.log`) |
| fail-capability, slice 1 | `SELFTEST=1` | **EXIT:1** — `89 ticks spent 2848 FE, expected 2880` (`e2e-slice3-selftest.log`) |
| fail-capability, chiller | `SELFTEST_COOLER=1` | **EXIT:1** — `148 ticks needed exactly 4736 FE, so the buffer should hold 32, not 0` (`e2e-slice3-selftest-cooler.log`) |
| log checker | `check-e2e-logs.py oritech-e2e --strict` | 12 logs, 12 ok, 0 bad |

The chiller leg is fail-capable four ways: the no-core phase (water and 5,000 FE, nothing happens), the
ten-tick arithmetic (320 FE must advance exactly ten ticks), the exact-FE completion (144 ticks must
cost exactly 4,608 FE and leave 0; a slower machine starves and makes nothing, a faster one leaves FE
behind), and `SELFTEST_COOLER=1`, which flips the last one and exits 1. Phase U adds two more: the
furnace has no tank (per-block declaration) and a fluid the pack does not have is refused.

### In-game transcript (quoted from `e2e-slice3.log`)

```
  info(12,100,0): Industrial Chiller at [12, 100, 0]: 0/50000 FE, assembled no core empty quality 0, progress 0 recipe none, tank empty 0/4000 mB, out empty, t=3359
  fluid(12,100,0): oritech:cooler_block at [12, 100, 0]: tank minecraft:water 1000/4000 mB (asked 1000, accepted 1000)
  info(12,100,0): Industrial Chiller at [12, 100, 0]: 5000/50000 FE, assembled no core empty quality 0, progress 0 recipe none, tank minecraft:water 1000/4000 mB, out empty, t=3458
  unassembled, with water and 5,000 FE: still progress 0, 5000 FE, tank minecraft:water 1000, no ice
  core click: {"ok":true,"block":"oritech:cooler_block"}
  block(12,100,-1) = oritech:machine_core_3 (the chiller's core is north, not above)
  info(12,100,0): Industrial Chiller at [12, 100, 0]: 3848/50000 FE, assembled yes core oritech:machine_core_3 quality 3, progress 36 recipe oritech:cooler/ice, tank minecraft:water 1000/4000 mB, out empty, t=3570
  info(12,100,0): Industrial Chiller at [12, 100, 0]: 0/50000 FE, assembled yes core oritech:machine_core_3 quality 3, progress 46 recipe oritech:cooler/ice, tank minecraft:water 1000/4000 mB, out empty, t=3591
  320 FE advanced the craft exactly 10 ticks (46 -> 56 of 200), tank still 1000 mB
  info(12,100,0): Industrial Chiller at [12, 100, 0]: 0/50000 FE, assembled yes core oritech:machine_core_3 quality 3, progress 0 recipe none, tank empty 0/4000 mB, out minecraft:ice x3, t=3869
  from progress 56: 144 ticks spent exactly 4608 FE, buffer 0, out minecraft:ice x3, tank empty
  fluid(16,100,0): oritech:cooler_block at [16, 100, 0]: tank minecraft:lava 1000/4000 mB (asked 1000, accepted 1000)
  info(16,100,0): Industrial Chiller at [16, 100, 0]: 5952/50000 FE, assembled yes core oritech:machine_core_2 quality 2, progress 14 recipe oritech:cooler/obsidian, tank minecraft:lava 1000/4000 mB, out empty, t=3991
  info(16,100,0): Industrial Chiller at [16, 100, 0]: 0/50000 FE, assembled yes core oritech:machine_core_2 quality 2, progress 0 recipe none, tank empty 0/4000 mB, out minecraft:obsidian x2, t=4246
  lava -> minecraft:obsidian x2 for exactly 6400 FE, tank empty
  the furnace has no tank: /oritech fluid answered '[8, 100, 0] has no Oritech fluid tank.'
  fluid(12,100,0): oritech:cooler_block at [12, 100, 0]: tank empty 0/4000 mB (asked 1000, accepted 0)
  oritech:still_steam: accepted 0 of 1000 mB, tank empty (the jar's snow recipe is recorded as a gap)
  every carried loot table rolled its block into the inventory
  an unknown loot table gave nothing (inventory entries 36 -> 36)
PASS: … an industrial chiller with no core froze nothing even with a bucket of water and 5,000 FE, its core assembled it one block to the north, a bucket of water entered the tank through the block's own declared fluid handler, 320 FE advanced the freeze exactly ten ticks, water became exactly 3 ice for exactly 6,400 FE with the tank emptied, lava became exactly 2 obsidian, a mod fluid was refused, only the chiller had a tank, and every carried loot table rolled its own block
EXIT:0
```

The craft's arithmetic: the first read lands 56 ticks into the operation, so the rest is
`200 - 56 = 144` ticks and `144 × 32 = 4,608` FE — the whole craft is `200 × 32 = 6,400` FE, the jar's
`time: 200` at its own 32 FE a tick. The lava craft starts from progress 0 and runs the full 200 ticks
for exactly 6,400 FE.

### The fluid path that worked, and the one that does not exist

- **`Host::fluid_insert` is not in the mod API at this pin.** The game has `native_fluid_insert`
  (`game/machines/native_fluids.rs`), but the SDK's `Host` has no such method; what a mod can reach is
  the **capability**: `level.capability(&mut *host, &FLUIDS, pos, Some(face))` →
  `ResourceHandler<FluidResource>` → `insert_any`, which is exactly what `rangedpumps` drains through
  and what this port fills through. The engine builds that handler from
  `Registrar::block_fluids` — so the `accepted 1000` in phase P is the declaration working end to end.
- **`fluid_insert` refuses every fluid but `minecraft:water`/`minecraft:lava`** (HANDOVER §9f), and
  this port needs neither more nor less: its two recipes take exactly those two.
- **A `word` command argument stops at `:`** (`horizon-command`'s `is_allowed_in_unquoted` allows only
  alphanumerics and `_ - . +`), so `/oritech fluid` takes its fluid as a greedy `text` **last**:
  `<pos:block_pos> <amount:integer> <fluid:text>`. A `word` variant fails the parse with
  *"Expected whitespace to end one argument"* — measured, and the first e2e run died on it.

### Parity coverage (the mod's families, slices 1–3)

The machine-checked form (`python3 ports/tools/check-coverage.py oritech`): one row per selector, the
specific ones above the wildcard, so a carried-and-tested entry outranks the family it belongs to.

| Family | Status | Evidence / reason |
|---|---|---|
| `oritech:basic_generator_block` | tested | A (no fuel, no FE), C (coal click, 32 FE a tick), F (stone refused), F2 (spectator), G (relaunch) |
| `oritech:pulverizer_block` | tested | B (no power, keeps its bone), D (100 ticks, 3,200 FE), E (the adjacent generator drives it), N (tag recipe) |
| `oritech:powered_furnace_block` | tested | H (no core), H2 (unassembled), I (core + assembly), J (100 ticks, 3,200 FE), L (core broken, progress kept) |
| `oritech:cooler_block` | tested | P (no core), Q (core north), R (10 ticks), S (3 ice, exactly 6,400 FE, tank emptied), T (lava), V (item given) |
| `oritech:lava_generator_block` | tested | X (place, 100 mB through the declared tank, core above, assembled), Y (120 ticks, exactly 7,680 FE at 64 a tick, tank emptied at load), Y2 (the ten-target push hands it all to a pulverizer), Y3 (that pulverizer spends it), Y4 (unassembled, 5,000 FE stay; re-assembled, they move), Z (empty tank, 0 FE for 200 ticks), Z2 (a tank of water, 0 FE), V (item given) |
| `oritech:machine_core_1` | tested | M places and reads it back; Q assembles the chiller with core 3 |
| `oritech:machine_core_2` | tested | M; I assembles the furnace with core 2; T assembles the second chiller |
| `oritech:machine_core_3` | tested | M; Q assembles the chiller with core 3 |
| `oritech:machine_core_4` | tested | M; L reassembles the furnace with core 4 |
| `oritech:machine_core_5` | tested | M |
| `oritech:machine_core_6` | tested | M |
| `oritech:machine_core_7` | tested | M |
| `oritech:blocks/basic_generator_block` | tested | W: the carried loot table rolls the generator into the inventory |
| `oritech:blocks/pulverizer_block` | tested | W |
| `oritech:blocks/powered_furnace_block` | tested | W |
| `oritech:blocks/cooler_block` | tested | W |
| `oritech:blocks/lava_generator_block` | tested | X places it; W rolls its carried loot table into the inventory |
| `oritech:blocks/machine_core_1` | tested | W |
| `oritech:blocks/machine_core_2` | tested | W |
| `oritech:blocks/machine_core_3` | tested | W |
| `oritech:blocks/machine_core_4` | tested | W |
| `oritech:blocks/machine_core_5` | tested | W |
| `oritech:blocks/machine_core_6` | tested | W |
| `oritech:blocks/machine_core_7` | tested | W |
| `multiblock` | tested | furnace I/L (core above), chiller Q (core north) |
| `block_entity` | tested | every machine read through `/oritech info` after a relaunch (G) and across a core break (L); the chiller's and the lava generator's saved shapes are unit-tested (`the_*_s_state_reads_back_from_the_compound_it_wrote`) rather than relaunched in the leg |
| `oritech:pulverizer/bone` | tested | D (100 ticks, 3,200 FE) and O |
| `oritech:pulverizer/blaze` | tested | O: crafted, output id and count asserted, input consumed |
| `oritech:pulverizer/dripstone` | tested | O |
| `oritech:pulverizer/glowstoneore` | tested | O |
| `oritech:pulverizer/coalore` | tested | N (tag form) and O |
| `oritech:pulverizer/diamondore` | tested | O |
| `oritech:pulverizer/lapisore` | tested | O |
| `oritech:cooler/ice` | tested | R (10 ticks) and S (3 ice, exactly 6,400 FE, tank emptied) |
| `oritech:cooler/obsidian` | tested | T (2 obsidian, exactly 6,400 FE) |
| `oritech:cooler/snow` | blocked(fluid-whitelist: oritech:still_steam) | U measures the refusal (`accepted 0`); the fluid does not exist in this pack |
| `oritech:lavagen/lava` | tested | X (100 mB through the tank), Y (a 120-tick burn: exactly 7,680 FE at 64 a tick, the tank emptied at load) |
| `oritech:lavagen/sheolfire` | blocked(fluid-whitelist: oritech:still_sheol_fire) | Y1 measures the refusal (`accepted 0`); the fluid does not exist in this pack |
| `recipe:oritech:pulverizer` | not ported | the other 117 need Oritech dusts/clumps/gems this port does not carry (gap 6) |
| `recipe:oritech:cooler` | not ported | `compat/pneumaticcraft/plastic` needs PneumaticCraft (gap 4) and `snow` is blocked above |
| `recipe:oritech:lava_generator` | not ported | `sheolfire` is blocked above; `lava` is carried and tested |
| `recipe:*` | not ported | assembler, centrifuge, foundry, refinery, atomic forge, grinder, bio/fuel generators, reactor, particle, laser, deep drill, `minecraft:` smelting/blasting/crafting and every `compat/` family: each needs item, fluid or block chains this port does not carry |
| `tag:block` | not ported | 14 block tags (`frame_support`, `laser_accelerated`, …) belong to families outside this slice |
| `tag:item` | not ported | 21 item tags (`biomatter`, `feeder_blacklist`, …) — same |
| `tag:entity_type` | not ported | `spawner_blacklist` — the spawner family is not ported |
| `worldgen:*` | not ported | oil springs, the four ores and their placed features: worldgen is the campaign's last tier and the pin's mod worldgen is inert (HANDOVER §9d item 19) |
| `data:*` | not ported | the NeoForge biome modifiers belong to the worldgen families above |
| `command` | tested | `/oritech info` (every phase), `/oritech energy` (D, H, R, S, T), `/oritech fluid` (P, T, U) |
| `*` | not ported | everything else in the jar — the GUI, the screens, GeckoLib models, the addons, the pipes and energy network, the remaining multiblocks (assembler, atomic forge, foundry, refinery, centrifuge, cooler's reactor kin, the accelerator ring, the reactor), the augment stations, the jetpack/equipment, the resource-node worldgen and the ores. Named one by one in the gap lists above |

`## Upstream defects fixed` (slice 3): none found. The `word`-argument limit is a pin property, not a
mod defect, and the snow recipe is a coverable gap rather than a broken behaviour.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

## Slice 4 — the Lava Generator (`oritech:lava_generator_block`), the fluid-burning generator (0.2.6, 2026-11-21)

**Why this family**, from `notes.md`'s own gap list — *Deviations and gaps (slice 1)*, gap **13**,
verbatim: "Not carried: the powered furnace (a multiblock machine that needs its core assembled), the
assembler, atomic forge, foundry, refinery, centrifuge, cooler, the accelerator ring, the reactor, the
augment stations, the jetpack and equipment, the resource-node worldgen and the ores. All later
slices." The slice-3 report's `## Next slice` named the lava generator, and it is the next machine whose
whole process runs on vanilla content: the jar's `data/oritech/recipe/lavagen/lava.json` burns **100 mB
of `minecraft:lava`** for **120 ticks**, and its only other recipe (`sheolfire.json`) takes
`oritech:still_sheol_fire`, which this package does not declare — the same measured gap as the chiller's
`snow.json`, recorded rather than shipped as dead data.

It is also the first machine that **makes** FE from a fluid rather than spending it, so it reuses
slice 1's generator tick (`progress` = burning ticks left, `canFitEnergy` gate) and slice 3's declared
tank, `tank_matches` and `/oritech fluid`, and it generalises the multiblock assembly slice 2 built with
the furnace's `(0, 1, 0)` core offset. Class + tier unchanged: class `content`, tier `deep`
(`ports/ledger.csv`), delivered in the machines shape (a block entity, a declared FE buffer, a declared
fluid tank, one recipe family, a multiblock assembly, the ten-target energy push). No renderer, no GUI,
no dimension, no accelerator.

### The jar's numbers (upstream `Rearth/Oritech` v1.2.13, commit `4069b6d`)

| Value | Upstream | This port |
|---|---|---|
| capacity 100,000 FE | `OritechConfig.generators.lavaGeneratorData` (`init/OritechConfig.java:385`) | `LAVA_CAPACITY` |
| insert 0 / extract 512 FE (`64 * 8`) | same `BasicMachineConfig(…, 100_000, 0, 64 * 8, 64)` | `battery(LAVA)` |
| 64 FE a burning tick | same `energyPerTick`; `calculateEnergyUsage` = `energyPerTick / speed(1)` | `lava_energy_usage()` |
| 120 ticks a burn | `data/oritech/recipe/lavagen/lava.json` `time: 120`; `progress = recipeTime` | `lavagen_ticks(time)` |
| 100 mB a burn | the same file's `fluidInput.amount` | `FluidInput::amount` |
| tank 4 buckets | `FluidMultiblockGeneratorBlockEntity.fluidStorage = new SimpleFluidStorage(4 * bucketAmount())` | `LAVA_TANK = 4000` |
| core at **(0, 1, 0)** | `LavaGeneratorEntity.getCorePositions` | `LAVA_CORE_OFFSET` (the furnace's, not the chiller's) |
| 10 output targets | `LavaGeneratorEntity.getOutputTargets` | `LAVA_OUTPUT_TARGETS` / `lava_output_targets()` |
| slots `(0, 1, 1, 1)`, inventory 2 (bucket in, empty out) | `getSlotAssignments` / `getInventorySize` | **not declared** — the bucket path is the engine's `fluid_util` interaction (deviation 1) |
| the tank refuses water at insert | the `fluidStorage.insert` override | `blocked(no-tank-filter)` — the declaration has no fluid filter (deviation 2) |
| the burn loads its recipe only with room | `UpgradableGeneratorBlockEntity.serverTick`'s `progress == 0 && canFitEnergy()` | `LavaGenerator::wants_recipe` |

### What was added

- `src/runtime.rs`: `LAVA_*`, `EnergyTarget`, `lava_energy_usage`, `lavagen_ticks`, `LavaGenerator`
  (energy / progress / max_burn / recipe, `room`, `can_fit`, `wants_recipe`, `load_recipe`, `tick`),
  `LavaGenRecipe::parse` / `LavaGenError`, `LAVA_OUTPUT_TARGETS` / `lava_output_targets`, and one
  shared `parse_fluid_input` (the cooler's parser now calls it too, so the two files cannot drift).
  Ten new tests.
- `src/support.rs`: `LAVA`/`LAVA_TYPE`/`LAVA_RECIPE`, the `battery` and tank arms, `tank_capacity`,
  `LavaSaved` + `lava_state`/`store_lava`, `tick_lava_generator` (assembly gate, the tank's recipe,
  drain-then-burn, and `outputEnergy` behind the gate), the `oritech:lava_generator` cache, `push_plan` (the basic generator's 6 faces and
  the lava generator's 10 targets, **the capability is now the filter** instead of a pulverizer-only
  block check), `direction`, the bucket right click through `fluid_util::interact_with_fluid_handler`,
  the `set_energy`/`set_fluid`/`info_line` arms, and the `lava_line` readout. `tank_state`,
  `store_tank` and `tank_text` take the block's capacity now, so the chiller and the lava generator
  share one shape. Seven new tests.
- `src/lib.rs`: `LavaPart`, the fifth block-entity type, the `oritech:lava_generator` recipe type and
  its raw serializer.
- Content: `oritech:lava_generator_block` (blockstate, block and item models, its own texture, 26.3
  item definition, block declaration, loot, lang `Lava Generator`, the machines tab, the pickaxe and
  stone-tool tags) and the jar's `lavagen/lava.json`. `assets/` + `data/` are now **98 files**
  (`tools/carry_content.py --check` → `98 files, 0 differ`).
- `tools/parse_info.py` learned the lava generator's line; `tools/e2e.sh` gained phases X, Y, Y1, Y2,
  Y3, Y4, Z, Z2; `tools/slice4-glue-counterfactual.py` is the glue's fail-capability; `PLAN-slice4.md` is
  the slice's plan; `VALIDATION.md` is the ladder.

### The command surface

`/oritech info` now answers the lava generator, and `/oritech energy`/`/oritech fluid` reach its
100,000 FE buffer and its 4,000 mB tank (the tank's capacity is read from the block, so the chiller's
line still says `…/4000 mB`):

```
Lava Generator at [0, 100, 6]: 2816/100000 FE, assembled yes core oritech:machine_core_2 quality 2, burning 76/120 ticks recipe oritech:lavagen/lava, tank empty 0/4000 mB, t=4355
oritech:lava_generator_block at [0, 100, 6]: tank minecraft:lava 100/4000 mB (asked 100, accepted 100)
```

### Deviations (slice 4)

1. **The two bucket slots are not declared.** Upstream drains slot 0 into the tank on the server tick
   and moves the empty container to slot 1. This port has no GUI and no item automation, so an inert
   input slot would be dead data; the player's path is the engine's own
   `fluid_util::interact_with_fluid_handler` on the right click (a held lava bucket fills the tank and
   is replaced by the bucket), which is the same action for the player. Nothing depends on the slots.
2. **The tank's water refusal cannot be expressed** — `blocked(no-tank-filter)`. `ModFluidTanks`
   declares capacity, faces and which tanks fill/drain, with no fluid filter, so water can enter the
   tank; it then matches no lava recipe and the machine makes 0 FE, which is exactly what phase Z2
   asserts. Upstream returns 0 from `insert` for water instead.
3. **`sheolfire.json` is not carried** — `blocked(fluid-whitelist: oritech:still_sheol_fire)`. The
   fluid is the mod's own and this package does not declare it, so the recipe could never match; phase
   Y1 measures the engine refusing the id (`accepted 0`), as slice 3 did for `snow.json`.
4. **The energy push is capability-driven.** Slice 1 pushed the basic generator's FE only into a block
   whose id is `oritech:pulverizer_block`; upstream's `outputEnergy` builds a cache on each target and
   moves into whatever `ENERGY` handler answers. `push_plan` now names only the targets and the
   generator's own extraction rate, and the capability lookup is the filter, so the basic generator
   also charges a furnace or a chiller beside it. The lava generator walks its own ten upstream
   targets (including the `(0, 2, 0)` and the four `+1` positions), which is why its FE reaches a
   consumer the basic generator would not.
5. **No GUI, no `lit` state, no GeckoLib render, no addons, no redstone** — the same recorded gaps as
   slices 1–3. The lava generator's two addon slots at (1, 0, 0) and (1, 1, 0) are not ported (no
   addon system).

### Verification (0.2.6)

Every Rust, package and in-game step ran in a `build-slot.sh` lease (`BUILD_LOCAL=1`,
`CARGO_INCREMENTAL=0`, no `--target-dir`); the launcher build is outside a slot (its own global lock).

| Step | Command | Result |
|---|---|---|
| runtime red | `cargo test --offline`, the new bodies `todo!()` | **57 passed, 9 failed** — every failure `panicked at src/runtime.rs:…: not yet implemented` (`slice4-test-red.log`; the tenth new test, `the_tank_must_hold_a_lava_recipes_own_fluid_and_amount`, pins the shared `tank_matches` and passed at once — reported as such) |
| glue red | same, the new declarations/readout/saved-state bodies `todo!()` | **68 passed, 5 failed** (`slice4-glue-red.log`) |
| glue fail-capability (counterfactual) | six targeted mutations of the declarations/saved state/push/readout, then the tests | **68 passed, 5 failed**, one per new glue test; the file restored byte-identically afterwards (`slice4-glue-red-counterfactual.log`, `tools/slice4-glue-counterfactual.py`) |
| tests (green) | `cargo test --offline` | **73 passed**, 0 failed (`sdk-test.log`) |
| lints + format | `cargo fmt --check` + `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`sdk-clippy.log`) |
| package | `cargo horizon-mod build` twice, `Cargo.lock` removed before each and the artifact deleted between | **105,595 B**, sha256 `a42999768aba2e3083422a006c28da3154b93256bc98d6fd691cf1c607f06cac`, `cmp` identical, **107 entries**, no packed `Cargo.lock` (`sdk-build.log`, `sdk-build-second.log`, `sdk-sha256.txt`) |
| registry check | `cargo horizon-mod check oritech.hmod` | `oritech 0.2.6 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)` / `105595 bytes … unsigned`, exit 0 (`sdk-check.log`) |
| fits the pin | `cargo horizon-mod fits oritech.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`sdk-fits.log`, on the delivered file, sha `a4299976…`) |
| fits **another** build | `… --build 0b14f8a` | `0b14f8a  fits`, exit 0 (the previous pin, so the check is not a tautology) |
| package gate | `python3 ports/tools/verify-sdk-package.py oritech` | **1/1** |
| content carry | `python3 tools/carry_content.py --check` | `98 files, 0 differ` |
| launcher build | `ports/tools/launcher-game.sh build oritech` (manifest 0.2.6 ⇒ a real recompile) | `Compiling the mods against the prebuilt game`; `strings liboritech.so` shows `Lava Generator at `, `oritech:lava_generator` and the bucket refusal |
| e2e | `build-slot.sh run --step e2e -- bash ports/oritech/tools/e2e.sh` | **PASS** + `EXIT:0` (`e2e-slice4.log`) |
| e2e differential | `CONTROL=1` (the package-less build first) | **PASS** + `EXIT:0`; without this package the generator item is absent and the generator, furnace, core, chiller and **lava generator** ids all stay `minecraft:air` (`e2e-slice4-differential.log`) |
| e2e whole scenario on the package-less build | `GAME=$CONTROL_GAME` | **EXIT:1** — `the generator did not place` (`e2e-slice4-control.log`) |
| fail-capability, slice 1 | `SELFTEST=1` | **EXIT:1** — `89 ticks spent 2848 FE, expected 2880` (`e2e-slice4-selftest.log`) |
| fail-capability, lava generator | `SELFTEST_LAVA=1` | **EXIT:1** — `a 120-tick lava burn makes exactly 7680 FE, so the buffer should hold 7744, not 7680` (`e2e-slice4-selftest-lava.log`) |
| log checker | `check-e2e-logs.py oritech-e2e --strict` | 17 logs, 17 ok, 0 bad |
| coverage | `check-coverage.py oritech` | exit 0, `missing_families=0 missing=0` |
| skills | `check-skills.py --notes ports/oritech` | PASS |

The lava leg is fail-capable six ways: the **no-core** phase (100 mB of lava and no assembly does
nothing), the **per-tick rate** (the buffer must be exactly 64 × the ticks burned), the **exact burn**
(120 ticks must leave exactly 7,680 FE; a slower machine leaves less, a faster one more), the
**empty-tank** control (0 FE for 200 ticks) and the **wrong-fluid** control (a tank of water makes 0 FE),
plus `SELFTEST_LAVA=1`, which flips the exact-FE figure and exits 1, and **Y4** is a
within-scenario differential for the output gate. Y2/Y3 add a conservation
assertion the machine cannot fake: the generator ends at 0 FE and the adjacent pulverizer holds exactly
7,680, then spends exactly 3,200 of it on a bone.

### In-game transcript (quoted from `e2e-slice4.log`)

```
  info(0,100,6): Lava Generator at [0, 100, 6]: 0/100000 FE, assembled no core empty quality 0, burning 0/0 ticks recipe none, tank empty 0/4000 mB, t=4301
  fluid(0,100,6): oritech:lava_generator_block at [0, 100, 6]: tank minecraft:lava 100/4000 mB (asked 100, accepted 100)
  unassembled, with 100 mB of lava: still 0 FE, burn 0, tank minecraft:lava 100
  core click: {"ok":true,"block":"oritech:lava_generator_block"}
  block(0,101,6) = oritech:machine_core_2
  info(0,100,6): Lava Generator at [0, 100, 6]: 2240/100000 FE, assembled yes core oritech:machine_core_2 quality 2, burning 85/120 ticks recipe oritech:lavagen/lava, tank empty 0/4000 mB, t=4470
  info(0,100,6): Lava Generator at [0, 100, 6]: 2816/100000 FE, assembled yes core oritech:machine_core_2 quality 2, burning 76/120 ticks recipe oritech:lavagen/lava, tank empty 0/4000 mB, t=4479
  burning 76/120: 44 ticks made exactly 2816 FE, tank emptied at load
  info(0,100,6): Lava Generator at [0, 100, 6]: 7680/100000 FE, assembled yes core oritech:machine_core_2 quality 2, burning 0/120 ticks recipe none, tank empty 0/4000 mB, t=4639
  one lava burn: exactly 7680 FE in the buffer, tank empty, no recipe left
  fluid(0,100,6): oritech:lava_generator_block at [0, 100, 6]: tank empty 0/4000 mB (asked 1000, accepted 0)
  oritech:still_sheol_fire: accepted 0 of 1000 mB, tank empty (the jar's sheolfire recipe is recorded as a gap)
  info(0,100,7): Pulverizer at [0, 100, 7]: 7680/25000 FE, progress 0 recipe none, input empty, out0 empty, out1 empty, t=4727
  the lava generator pushed its 7680 FE one block south into the pulverizer
  info(0,100,7): Pulverizer at [0, 100, 7]: 4480/25000 FE, progress 0 recipe none, input empty, out0 minecraft:bone_meal x6, out1 empty, t=4948
  the consumer turned the lava generator's FE into minecraft:bone_meal x6: 7680 -> 4480 FE
  info(0,100,6): Lava Generator at [0, 100, 6]: 5000/100000 FE, assembled no core empty quality 0, burning 0/120 ticks recipe none, tank empty 0/4000 mB, t=5013
  info(0,100,7): Pulverizer at [0, 100, 7]: 4480/25000 FE, progress 0 recipe none, input empty, out0 minecraft:bone_meal x6, out1 empty, t=5021
  unassembled with 5,000 FE: the buffer kept all of it and the pulverizer stayed at 4480
  info(0,100,6): Lava Generator at [0, 100, 6]: 0/100000 FE, assembled yes core oritech:machine_core_2 quality 2, burning 0/120 ticks recipe none, tank empty 0/4000 mB, t=5127
  info(0,100,7): Pulverizer at [0, 100, 7]: 9480/25000 FE, progress 0 recipe none, input empty, out0 minecraft:bone_meal x6, out1 empty, t=5140
  reassembled: the 5,000 FE left the generator, the pulverizer rose 4480 -> 9480
  info(4,100,6): Lava Generator at [4, 100, 6]: 0/100000 FE, assembled yes core oritech:machine_core_2 quality 2, burning 0/0 ticks recipe none, tank empty 0/4000 mB, t=5209
  assembled with an empty tank: 0 FE after 200 ticks, burn 0, tank empty
  fluid(4,100,6): oritech:lava_generator_block at [4, 100, 6]: tank minecraft:water 1000/4000 mB (asked 1000, accepted 1000)
  info(4,100,6): Lava Generator at [4, 100, 6]: 0/100000 FE, assembled yes core oritech:machine_core_2 quality 2, burning 0/0 ticks recipe none, tank minecraft:water 1000/4000 mB, t=5432
  a tank of water: 0 FE after 200 ticks, still minecraft:water 1000, no recipe
```

The burn's arithmetic: 120 ticks × 64 FE = **7,680 FE**, the jar's `time: 120` at its own
`energyPerTick: 64`; the mid-read lands 44 ticks in and sees exactly `44 × 64 = 2,816`. The tank's
100 mB leaves at load time (`consumeFluidRecipeInput`), so the tank reads empty from the first tick
while the burn still has ticks left — upstream's own order.

`## Upstream defects fixed` (slice 4): none found. The water-refusal and the bucket slots are
recorded deviations (the pin cannot express the filter, and the port has no GUI), not defects; the
`sheolfire` recipe is a coverable gap, not a broken behaviour.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices
