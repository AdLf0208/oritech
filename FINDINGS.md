# Oritech 1.2.13 — findings (upstream behaviour, slice 1)

Read from the public repository `https://github.com/Rearth/Oritech` at tag
`v1.2.13` (commit `4069b6dd14b05dfe0c80126618739b64054630ea`, 2026-10-05), and
from the ATM10 jar `oritech-neoforge-1.21.1-1.2.13.jar`
(SHA-256 `32600bf5b08fcf6d4a49e557f90ee45b238098170777872c8919d9fdb0a1b828`,
5,184 entries, `license = "CC0-1.0"` in `META-INF/neoforge.mods.toml`). The
shallow clone was deleted after these findings were written; this is a
clean-room rewrite — no Java was copied or translated and no jar was
decompiled. Every number below is quoted from the files named beside it.

## Jar inventory (measured, `unzip -l`)

| Count | What |
|---:|---|
| 5,184 | entries |
| 212 | blockstates (`assets/oritech/blockstates/`) |
| 236 | vanilla block models (`assets/oritech/models/block/`) |
| 23 | GeckoLib block models (`assets/oritech/geo/block/models/*.geo.json`) |
| 294 | item models (`assets/oritech/models/item/`) |
| 552 | recipe files under `data/oritech/recipe/` (see below) |
| 1 | lang file, `assets/oritech/lang/en_us.json` |

The blockstates of the **machines** point at a placeholder vanilla model and
carry no `facing` variants, because GeckoLib draws the real machine:

```
assets/oritech/blockstates/basic_generator_block.json
{"variants": {"": {"model": "oritech:block/machine_speed_addon"}}}
assets/oritech/blockstates/pulverizer_block.json
{"variants": {"": {"model": "oritech:block/machine_speed_addon"}}}
```

The real machine models are `assets/oritech/geo/block/models/basic_generator_block.geo.json`
and `pulverizer_block.geo.json` (GeckoLib `.geo`, not a vanilla model), with
their textures at `assets/oritech/textures/block/models/basic_generator_block.png`
(128×128) and `pulverizer_block.png` (64×64).

## Recipe inventory (`data/oritech/recipe/<type>/`)

| Type | Files | Shape |
|---|---:|---|
| `crafting/` | 182 | vanilla shaped/shapeless |
| `pulverizer/` | 124 | `{type, ingredients:[…], results:[{count,id}], time}` |
| `grinder/` | 114 | same shape |
| `centrifuge/` | 65 | same shape |
| `assembler/` | 57 | same shape |
| `foundry/` | 29 | same shape |
| `augment/` | 29 | — |
| `refinery/` | 22 | — |
| `atomicforge/` | 19 | — |
| `mixing/` | 12 | — |
| `particle/` | 11 | — |
| `fuelgen/` | 11 | generator fuels |
| `deepdrill/` | 11 | — |
| `crushing/` | 7 | — |
| `biogen/` | 5 | — |

Pulverizer times: 111 recipes at `time: 100`, 7 at 120, 2 at 140, 4 at 150.
Example (`data/oritech/recipe/pulverizer/bone.json`): bone → 6 bone meal, 100
ticks. `coalore.json`: `#minecraft:coal_ores` → 2 coal, 100 ticks.

## Slice 1: the Basic Generator and the Pulverizer

Both classes read in `common/src/main/java/rearth/oritech/`:

- `block/entity/generators/BasicGeneratorEntity.java`
- `block/base/entity/UpgradableGeneratorBlockEntity.java`
- `block/base/entity/UpgradableMachineBlockEntity.java`
- `block/base/entity/MachineBlockEntity.java`
- `block/entity/processing/PulverizerBlockEntity.java`
- `init/OritechConfig.java`
- `init/BlockContent.java`, `init/BlockEntitiesContent.java`

### Basic Generator (`oritech:basic_generator_block`)

- Buffer: `BasicMachineConfig(b, "basicGeneratorData", 50_000, 0, 32 * 8, 32)`
  → capacity **50,000 FE**, `maxEnergyInsertion` **0**, `maxEnergyExtraction`
  **256 FE/t**, `energyPerTick` **32** (`OritechConfig.Generators`).
- Tick (`UpgradableGeneratorBlockEntity.serverTick`): with no burn left and room
  for a whole production tick, consume one input item and set
  `progress = FuelRegistry.get(item)` (`BasicGeneratorEntity.tryConsumeInput`);
  each tick while `progress > 0` and the buffer can fit the production, add
  `energyPerTick * (1 / speedMultiplier)` = **32 FE**, decrement `progress`.
- A burn therefore lasts the vanilla fuel's burn time (coal 1600 ticks) and
  makes 32 FE/t; at 50,000 FE the buffer stops the burn (`canFitEnergy`) until
  something drains it.
- Fuel rule: an item whose fuel value is > 0; a `BucketItem` is refused
  (`BasicGeneratorEntity.tryConsumeInput` returns early for one).
- Output: `outputEnergy()` every tick pushes energy to the six neighbours in
  `getOutputTargets` (above, below, east, south, west, north), each through the
  buffer's `maxExtract`.
- Inventory: one slot (fuel), `InventorySlotAssignment(0, 1, 1, 0)`.
- Block: `Properties.ofFullCopy(Blocks.IRON_BLOCK).noOcclusion()`
  (hardness 5.0, blast resistance 6.0, iron sounds, stone-tier tool needed,
  no occlusion).

### Pulverizer (`oritech:pulverizer_block`)

- Buffer: `BasicMachineConfig(b, "pulverizerData", 25_000, 32 * 8, 0, 32)`
  → capacity **25,000 FE**, `maxEnergyInsertion` **256 FE/t**, extraction 0,
  `energyPerTick` **32** (`OritechConfig.ProcessingMachines`).
- Recipe type `oritech:pulverizer` (`PulverizerBlockEntity.getOwnRecipeType`),
  JSON `{type, ingredients, results, time}`; `time` is in ticks.
- Tick (`MachineBlockEntity.serverTick`): with a matching recipe and room for
  its results, if the buffer holds `calculateEnergyUsage()` =
  `energyPerTick * efficiency(1) * (1 / speed(1))` = **32 FE**, spend it,
  `progress += 1`, and on `progress >= recipe.getTime() * speedMultiplier(1)`
  add the results to the output slots, remove one of the inputs, reset
  progress. A starved machine keeps its progress; a missing recipe or blocked
  output resets it to 0.
- Slots: `InventorySlotAssignment(0, 1, 1, 2)` → input 0, outputs 1 and 2.
- Extras not in slice 1: `combineSmallDusts` (9 small dusts → 1 dust, a crafting
  recipe lookup at completion) and the 1-damage `stepOn` when working.
- Block: same `IRON_BLOCK` copy as the generator.

### Values carried into the port

| Value | Source | Used for |
|---|---|---|
| `GENERATOR_ENERGY_PER_TICK = 32` | `OritechConfig.generators.basicGeneratorData` | FE produced per burning tick |
| `GENERATOR_CAPACITY = 50_000` | same | the buffer |
| `GENERATOR_MAX_EXTRACT = 256` | same (`32 * 8`) | the most one extraction moves |
| `MACHINE_ENERGY_PER_TICK = 32` | `pulverizerData.energyPerTick` | FE spent per working tick |
| `PULVERIZER_CAPACITY = 25_000` | `pulverizerData.energyCapacity` | its buffer |
| `PULVERIZER_MAX_INSERT = 256` | `pulverizerData.maxEnergyInsertion` (`32 * 8`) | the most one insert moves |
| `MACHINE_SPEED = 1`, `MACHINE_EFFICIENCY = 1` | `BaseAddonData.DEFAULT_ADDON_DATA` (no addons) | no multiplier in slice 1 |
| recipe `time` | each `data/oritech/recipe/pulverizer/*.json` | 100 ticks for every recipe carried |

## Not read / not ported (gaps)

- GeckoLib models and animations (`geo/`, `animations/`), the machine screens
  (`client/init/ModScreens`, `client/screen/**`), the addon system
  (`machine_*_addon`, `addon_indicator_block`), multiblocks (the accelerator
  ring, reactor, assembler, atomic forge, foundry, refinery, centrifuge, cooler,
  powered furnace — all `MultiblockMachineEntity`, which needs an `assembled`
  state and its core/frame assembly), the augment system
  (`advanced_augment_station`, `simple_augment_station`), the pipe network
  (`energy_pipe`, `item_pipe`, `fluid_pipe`, superconductors), ore processing
  chains (`grinder`, `centrifuge`, `foundry`, `refinery`, `atomicforge`),
  worldgen (`resource_node_*`, ores), and the jetpack/augment equipment.
- The two machines' own crafting recipes need `oritech:motor`,
  `oritech:magnetic_coil`, `c:ingots/nickel`, `c:storage_blocks/copper` and
  `c:player_workstations/furnaces` — none exist in this slice or in 26.3, so
  they are not carried (recorded as a gap in `notes.md`).
- `PulverizerBlock.stepOn` damage and `combineSmallDusts` are not ported.

## Slice 2: the Powered Furnace and the machine cores

Read for behaviour only (the clone was shallow at the same tag
`v1.2.13` / `4069b6dd14b05dfe0c80126618739b64054630ea`, deleted after these findings):

- `common/src/main/java/rearth/oritech/block/entity/processing/PoweredFurnaceBlockEntity.java`
- `common/src/main/java/rearth/oritech/block/blocks/processing/PoweredFurnaceBlock.java`
- `common/src/main/java/rearth/oritech/block/blocks/processing/MachineCoreBlock.java`
- `common/src/main/java/rearth/oritech/block/base/block/MultiblockMachine.java`
- `common/src/main/java/rearth/oritech/block/base/entity/MultiblockMachineEntity.java`
- `common/src/main/java/rearth/oritech/util/MultiblockMachineController.java`
- `common/src/main/java/rearth/oritech/init/OritechConfig.java` (`FurnaceConfig`, `ProcessingMachines`)
- `common/src/main/java/rearth/oritech/init/BlockContent.java` (`POWERED_FURNACE_BLOCK`, `MACHINE_CORE_1..7`)

### Powered Furnace (`oritech:powered_furnace_block`)

- Buffer `FurnaceConfig`: capacity **10,000 FE**, `maxEnergyInsertion` `32 * 8` = **256**, extraction
  none, `energyPerTick` **32**, `speedMultiplier` **0.5** ("relative to vanilla").
- Usage per working tick: `calculateEnergyUsage` = `energyPerTick * efficiency(1) * (1 / 0.5) / 2` =
  **32 FE**; duration: `progress >= cookingTime * speedMultiplier` → a vanilla 200-tick smelt takes
  **100 ticks** here.
- The recipe is the **vanilla smelting** table (`getRecipeFor(RecipeType.SMELTING, …)`) — its own
  `getOwnRecipeType()` returns `ASSEMBLER` and is unused ("not used in this special case").
- Slots `InventorySlotAssignment(0, 1, 1, 1)`, inventory size 2: input 0, output 1.
- Completion (`craftFurnaceItem`): shrink the input, then add the result to the output, repeated
  `extraChambers()` times (0 without a processing addon).
- A multiblock: `getCorePositions()` = one core at **(0, 1, 0)**; the machine only ticks while
  `MultiblockMachine.ASSEMBLED` is set (`isActive`), and has a `LIT` state with light level 15.
- `MultiblockMachine` (block): the empty-hand click runs `initMultiblock`; the held-item path
  (`tryPlaceNextCore`) puts a `MachineCoreBlock` into the next missing core position.
- `MultiblockMachineController.onCoreBroken`: clears `ASSEMBLED` and the connected cores (the progress
  is not reset); `onControllerBroken` releases its cores.
- `MachineCoreBlock`: `USED` boolean state, invisible when used, quality = the tier (1..7);
  `MachineCoreEntity` stores the controller position.

### Values carried into the port

| Value | Source | Used for |
|---|---|---|
| `FURNACE_CAPACITY = 10_000` | `furnaceData.energyCapacity` | the buffer |
| `FURNACE_MAX_INSERT = 256` | `furnaceData.maxEnergyInsertion` | the most one insert moves |
| `FURNACE_ENERGY_PER_TICK = 32` | `furnaceData.energyPerTick` | the usage formula |
| `FURNACE_SPEED = 0.5` | `furnaceData.speedMultiplier` | the duration |
| `CORE_OFFSET = [0, 1, 0]` | `getCorePositions` | where the core goes |
| `CORES = machine_core_1..7`, quality = tier | `BlockContent` / `MachineCoreBlock` | assembly |

### Not read / not ported (still gaps)

The frames and expandable multis (`MultiblockFrameInteractionEntity`, `ExpandableMultiblockEnergyStorage`),
the addon blocks and their scan (`MachineAddonController`), the other multiblock machines, the
accelerator/reactor, the GUI/screens, GeckoLib models and the pipe network.
