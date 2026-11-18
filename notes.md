# Oritech — port notes (slice 1: the Basic Generator and the Pulverizer)

Ported from **Oritech 1.2.13** (`oritech-neoforge-1.21.1-1.2.13.jar`, SHA-256
`32600bf5b08fcf6d4a49e557f90ee45b238098170777872c8919d9fdb0a1b828`,
5,184 entries) for Minecraft Java 1.21.1 / NeoForge. The jar's own
`META-INF/neoforge.mods.toml` declares **`license = "CC0-1.0"`** — a
public-domain dedication, **licence rank 0**, no obligations. (The source map's
CC-BY-4.0 is stale; the jar wins, ARCHITECTURE.md §8.) Local-only until the
maintainer approves publishing.

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`**, mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6).

Status: **ported-partial** — slice 1 (the fuel generator, the pulverizer, their
energy link, the recipe reader and the content carry) is implemented and
verified in game. The gap list below is the rest of the mod.

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
