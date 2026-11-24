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

## Slice 3: the Industrial Chiller and the mod's fluid recipes

Read for behaviour only (the clone was shallow at the same tag `v1.2.13` /
`4069b6dd14b05dfe0c80126618739b64054630ea`, deleted after these findings):

- `common/src/main/java/rearth/oritech/block/entity/processing/CoolerBlockEntity.java`
- `common/src/main/java/rearth/oritech/block/blocks/processing/CoolerBlock.java`
- `common/src/main/java/rearth/oritech/block/base/entity/MachineBlockEntity.java`
  (`serverTick`, `calculateEnergyUsage`, `craftItem`, `checkCraftingFinished`, `canOutputRecipe`)
- `common/src/main/java/rearth/oritech/block/entity/processing/CentrifugeBlockEntity.java`
  (`recipeInputMatchesTank`, the shared tank check)
- `common/src/main/java/rearth/oritech/util/FluidIngredient.java` (the `amount` default)
- `common/src/main/java/rearth/oritech/api/fluid/containers/SimpleFluidStorage.java` (the save shape)
- `common/src/main/java/rearth/oritech/init/OritechConfig.java` (`coolerData`)
- `common/src/main/java/rearth/oritech/init/recipes/OritechRecipe.java` + `FluidIngredient`
- the jar's `data/oritech/recipe/cooler/{ice,obsidian,snow}.json`

### Industrial Chiller (`oritech:cooler_block`, "Industrial Chiller")

- Buffer `BasicMachineConfig(b, "coolerData", 50_000, 32 * 8, 0, 32)`: capacity **50,000 FE**, insert
  **256**, extraction none, `energyPerTick` **32**.
- `calculateEnergyUsage` = `energyPerTick * efficiency * (1 / speed)`; with no addon both are 1, so
  **32 FE a working tick**. `checkCraftingFinished` = `progress >= time * speed`, so the jar's
  `time: 200` is **200 ticks**.
- **Cold biome**: `getSpeedMultiplier`/`getEfficiencyMultiplier` are halved in a biome of
  `TagContent.CONVENTIONAL_COLD` — twice as fast *and* the same 32 FE a tick, i.e. half the FE for the
  same craft. The pin has no biome read for a mod: recorded as `blocked(no-biome-tag-read)`.
- `fluidStorage = SimpleFluidStorage(4 * bucketAmount())`: **one 4,000 mB tank**. Upstream saves it
  with `writeNbt(nbt, "")` (a `FluidStack` codec compound under `fluid`); this port saves it in the
  engine's `fluids` list shape so the declared handler can be filled.
- Recipe type `oritech:cooler`; `getRecipe()` asks the table for recipes the (empty) item inventory
  matches, which for this machine is only the ingredient-less ones, then filters by
  `recipeInputMatchesTank` — the tank must hold that fluid and at least the recipe's amount.
  `FluidIngredient.CODEC` defaults `amount` to one bucket.
- `craftItem` → `processCraftInstance`: refuse when `heldStacks[0].count + result.count > 64`
  (upstream compares against a literal 64, not the item's stack size); otherwise extract exactly the
  recipe's millibuckets and grow the one result.
- Slots `InventorySlotAssignment(0, 0, 0, 1)`, `getInventorySize()` 1: **one output, no item input**.
- `getCorePositions` = `Vec3i(0, 0, -1)`: the one core is **one block north**, not above (the
  furnace's is above), and `getAddonSlots` = (0, 0, -2). `MultiblockMachineEntity` supplies the
  assembled gate, as for the furnace.

### Values carried into the port

| Value | Source | Used for |
|---|---|---|
| `COOLER_CAPACITY = 50_000` | `coolerData.energyCapacity` | the buffer |
| `COOLER_MAX_INSERT = 256` | `coolerData.maxEnergyInsertion` (`32 * 8`) | the most one insert moves |
| `COOLER_ENERGY_PER_TICK = 32` | `coolerData.energyPerTick` | the usage |
| `COOLER_TANK = 4_000` | `SimpleFluidStorage(4 * bucketAmount())` | the one tank |
| `COOLER_CORE_OFFSET = [0, 0, -1]` | `getCorePositions` | where the core goes |
| `COOLER_OUTPUT_LIMIT = 64` | `processCraftInstance` | the output rule |
| `FluidInput.amount` default 1,000 | `FluidIngredient.CODEC` | millibuckets a craft takes |

### Not read / not ported (still gaps)

The GUI/screens and GeckoLib models, the addon blocks and their `extraChambers` repeat, the cold-biome
bonus (no biome read at the pin), the mod's own fluids (`oritech:still_steam`, `still_diesel`, …), the
tank's item/fluid pipe network, and every machine family whose recipes need the ore-processing items or
those fluids (assembler, centrifuge, foundry, refinery, atomic forge, grinder, bio/fuel/lava
generators, deep drill, reactor, accelerator, augment stations).

## Slice 5: the machine addon system

Read for behaviour only (the clone was shallow at the same tag `v1.2.13` /
`4069b6dd14b05dfe0c80126618739b64054630ea`, deleted after these findings):

- `common/src/main/java/rearth/oritech/util/MachineAddonController.java` (`getAllAddons`, `gatherAddonStats`, `BaseAddonData`)
- `common/src/main/java/rearth/oritech/block/blocks/addons/MachineAddonBlock.java` (`AddonSettings`, `setPlacedBy`)
- `common/src/main/java/rearth/oritech/block/base/entity/UpgradableMachineBlockEntity.java` (burst, `getSpeedMultiplier` / `getEfficiencyMultiplier`, the chamber loop)
- `common/src/main/java/rearth/oritech/block/base/entity/UpgradableGeneratorBlockEntity.java` (`tryConsumeInput`'s burn time, `produceEnergy`)
- `common/src/main/java/rearth/oritech/block/base/entity/MachineBlockEntity.java` (`calculateEnergyUsage`, `checkCraftingFinished`, `craftItem`)
- `common/src/main/java/rearth/oritech/block/entity/processing/{Pulverizer,Cooler,PoweredFurnace}BlockEntity.java` and `block/entity/generators/LavaGeneratorEntity.java` (`getAddonSlots`, the furnace's own speed/usage)
- `common/src/main/java/rearth/oritech/init/{OritechConfig,OritechStartupConfig,BlockContent}.java` (the config defaults and the six blocks)
- `common/src/main/resources/assets/oritech/{blockstates,models,textures}` for the six addons

### The six default addons (`BlockContent`)

| Block | `AddonSettings` | Speed | Efficiency | Other |
|---|---|---|---|---|
| `machine_speed_addon` | 0.5 / 1.2 | × | × | — |
| `machine_efficiency_addon` | 1 / 0.8 | — | × | — |
| `machine_ultimate_addon` | 0.25 / 1.1 | × | × | — |
| `machine_processing_addon` | 1 / 1.5, `withChambers(1)` | — | × | +1 chamber |
| `machine_capacitor_addon` | `withAddedCapacity(2_000_000).withAddedInsert(2_000)` | — | — | buffer |
| `machine_burst_addon` | `withBurstTicks(240)` | burst | burst | 240 ticks |

`OritechConfig.additiveAddons` **defaults to `true`**; `gatherAddonStats` then adds `1 - setting` and
inverts, giving 2/3 speed and 1.2 efficiency for a speed addon (not 0.5/1.2). The other addon kinds
(acceptor, fluid, redstone, proxy, extender, combi, quarry, hunter, yield, silk touch, steam boiler, crop
filter) carry machine-family-specific behaviour this port does not ship.

### How the effects reach the tick

- usage `energyPerTick * efficiency * (1/speed)` (the furnace adds `(1/0.5)/2`); a whole-FE buffer loses
  `ceil` of it each tick;
- processing duration `progress >= time * speed`; generator burn `(int)(time * speed / efficiency)` and
  output `energyPerTick * efficiency / speed` (the float is truncated by the long buffer);
- burst: `remainingBurstTicks` +1 a server tick to `maxBurstTicks`, −2 per working tick, floored at
  `−max`; bonus `1/8` while positive, `1.2` while negative, `1` otherwise, applied to speed *and*
  efficiency (so the usage is unchanged); a generator never calls `useEnergy`, so its burst never drains;
- chambers repeat `craftItem` while another input and the output room remain.

### Not read / not ported (still gaps)

The GUI/screens and GeckoLib models, the `ADDON_USED` blockstate and the addon block entities, the
addon extender traversal (`layeredExtenders`), the other special addon kinds, and (until slice 6) the
comparator/redstone surface of the redstone addon. `ports/_shared/sdk/0a875da/surface.json` does have
`has_analog_output` (hook 274), `analog_output` (hook 8) and the redstone `signal` read (hook 60); slice
6 ports the input-control gate and reads the live signal with `LevelRef::has_neighbor_signal`, and the
comparator half stays `blocked(no-addon-mode-control)` — see the slice-6 section below.


## Slice 6: the Control Unit Addon (`oritech:machine_redstone_addon`)

Read for behaviour only, at the same tag **`v1.2.13`** / commit
`4069b6dd14b05dfe0c80126618739b64054630ea` (the repo is a local shallow clone in the
session's scratch directory; the jar in `ports/_shared/licence-cache/` supplied the
carried model, texture, lang and loot):

- `common/src/main/java/rearth/oritech/block/entity/addons/RedstoneAddonBlockEntity.java`
  (`RedstoneMode`, `activeMode`, `monitoredSlot`, `currentOutput`, `setRedstonePowered`,
  `getComparatorOutput`, the two saved keys `slot`/`mode`)
- `common/src/main/java/rearth/oritech/block/blocks/addons/RedstoneAddonBlock.java`
  (`neighborChanged` → `world.hasNeighborSignal(pos)` → `setRedstonePowered`,
  `updateShape`'s `POWERED` state, `isSignalSource`, `hasAnalogOutputSignal`,
  `getAnalogOutputSignal`, `useWithoutItem` opening the screen)
- `common/src/main/java/rearth/oritech/block/base/entity/MachineBlockEntity.java`
  (`serverTick`'s `if (!isActive(state) || disabledViaRedstone) return;`,
  `onRedstoneEvent`, `disabledViaRedstone`, `nbt.putBoolean("oritech.redstone", …)`,
  the four `getComparator*` figures)
- `common/src/main/java/rearth/oritech/block/base/entity/UpgradableMachineBlockEntity.java`
  (`serverTick` → `super` then `addBurstTicks()`, `receivedRedstoneSignal`,
  `currentRedstoneEffect`)
- `common/src/main/java/rearth/oritech/block/base/entity/UpgradableGeneratorBlockEntity.java`
  (the same early return, before `tryConsumeInput` **and** `outputEnergy`)
- `common/src/main/java/rearth/oritech/init/BlockContent.java:261`
  (`new RedstoneAddonBlock(…, AddonSettings.getDefaultSettings()…)`)

### What the jar says

| Fact | Evidence |
|---|---|
| a fresh addon is in input-control mode | `activeMode = RedstoneMode.INPUT_CONTROL` |
| a neighbour change pushes the signal to the controller | `neighborChanged` → `setRedstonePowered(world.hasNeighborSignal(pos))` |
| only input control reacts; the other four set `currentOutput` | `setRedstonePowered` returns early when `activeMode != INPUT_CONTROL` |
| the machine saves the flag | `oritech.redstone`, a boolean |
| a gated machine does nothing at all | the tick's first line, before the recipe lookup and `useEnergy` |
| a gated generator pushes nothing | `outputEnergy()` is after the same return |
| the burst still recharges | `UpgradableMachineBlockEntity.serverTick` calls `addBurstTicks()` after `super.serverTick` |
| the addon answers a comparator itself | `getAnalogOutputSignal` → `ComparatorOutputProvider.getComparatorOutput`; a controller answers 0 |
| the control unit has neutral settings | `AddonSettings.getDefaultSettings()` |

### Ported, blocked, not ported

- ported: the block, the neutral settings, `INPUT_CONTROL`'s gate, the saved flag, the readout field.
- `blocked(no-addon-mode-control)`: `OUTPUT_POWER`, `OUTPUT_SLOT`, `OUTPUT_PROGRESS`,
  `OUTPUT_ACTIVE` and the addon's comparator output — they need a control (a screen) to select a mode.
- not ported: the addon's screen (`RedstoneAddonScreen`, `RedstoneAddonScreenHandler`), the two
  network payloads (`RedstoneAddonClientUpdate`/`ServerUpdate`) and the `POWERED` blockstate's
  `_on`/`_powered` models — no GUI and no per-state model in this port.

## Slice 8: the Assembler (`oritech:assembler_block`)

Read for behaviour only (the clone was shallow at the same tag `v1.2.13` /
`4069b6dd14b05dfe0c80126618739b64054630ea`, deleted after these findings):

- `common/src/main/java/rearth/oritech/block/entity/processing/AssemblerBlockEntity.java`
- `common/src/main/java/rearth/oritech/block/blocks/processing/AssemblerBlock.java`
- `common/src/main/java/rearth/oritech/block/base/entity/MachineBlockEntity.java`
  (`serverTick`, `craftItem`, `canOutputRecipe`/`canAddToSlot`, `findLowestMatchingSlot`)
- `common/src/main/java/rearth/oritech/init/recipes/OritechRecipe.java` (`matches` → `complexMatch`)
- `common/src/main/java/rearth/oritech/util/MultiblockMachineController.java`
  (`getNextMissingCore`, `initMultiblock`, `onCoreBroken`, `addMultiblockToNbt`)
- `common/src/main/java/rearth/oritech/init/OritechConfig.java` (`assemblerData`)
- the jar's `data/oritech/recipe/assembler/*.json` (57 files) and
  `data/oritech/recipe/crafting/{assembler,assembleralt,alloy/adamant}.json`

### Values carried into the port

| Value | Source | Used for |
|---|---|---|
| `ASSEMBLER_CAPACITY = 50_000` | `assemblerData.energyCapacity` | the buffer |
| `ASSEMBLER_MAX_INSERT = 1_024` | `assemblerData.maxEnergyInsertion` (`128 * 8`) | the most one insert moves |
| `ASSEMBLER_ENERGY_PER_TICK = 128` | `assemblerData.energyPerTick` | the usage formula |
| four inputs, one output | `InventorySlotAssignment(0, 4, 4, 1)`, `getInventorySize` 5 | the container |
| cores at (0,0,1), (0,1,0), (0,1,1) | `getCorePositions` | `ASSEMBLER_CORE_OFFSETS` |
| addon slots (0,0,-1), (0,0,2), (1,0,0) | `getAddonSlots` | the first three-slot machine |
| a recipe matches as a multiset | `OritechRecipe.complexMatch` | `runtime::craft_plan` |
| one item per ingredient, rotating start | `craftItem` `startOffset` | the same `craft_plan` |
| an empty slot wins the click | `findLowestMatchingSlot` | `runtime::insert_slot` |
| quality = the average core tier | `initMultiblock`'s `sumCoreQuality / size` | `Assembly::quality` |

### Ported, blocked, not ported

- ported: the block, its three-core assembly, the `oritech:assembler` recipe type and the five carried
  recipes, the four-slot click, the one-output rule, the addon scan on three slots, the readout.
- not ported: the machine's screen (`AssemblerScreen`/`AssemblerScreenHandler`), the `ENCHANTED_HIT`
  particles in `useEnergy`, the `SETUP`/`WORKING` GeckoLib animations, the `facing`-rotated core and
  addon offsets (no `facing` state), the `ADDON_USED` state — the same gaps as slices 1–7.
- the other 52 `oritech:assembler` files: plastic plates (13), `compat/geore` (29), an unported result
  family (7), `#c:fuels/bio` (2), `#c:leathers` (1) — `notes.md` → *Slice 8* → *The recipe set*.

## Slice 11: the Fragment Forge ("grinder")

Read for behaviour only, at the same tag **`v1.2.13`** / commit
`4069b6dd14b05dfe0c80126618739b64054630ea` (shallow clone in this session's scratch directory,
deleted after these findings; the jar in `ports/_shared/licence-cache/` supplied the carried
texture, item models, lang and loot). Files read:

- `common/src/main/java/rearth/oritech/block/entity/processing/FragmentForgeBlockEntity.java`
  (the recipe type, output doubling with the yield addon, `getSlotAssignments`, `getCorePositions`,
  `getAddonSlots`, the `byproductAddon` saved key, `combineSmallDusts` on completion),
- `common/src/main/java/rearth/oritech/init/OritechConfig.java:348,361,529-542`
  (`ProcessingMachines.fragmentForgeData` = `BasicMachineConfig(b, …, 50_000, 256 * 8, 0, 256)`),
- `common/src/main/java/rearth/oritech/block/base/entity/MachineBlockEntity.java:130-221`
  (`calculateEnergyUsage` = `energyPerTick × efficiency × 1/speed`, `getCraftingResults`,
  `craftItem`'s result-per-slot and rotating input drain, `checkCraftingFinished` = `time × speed`,
  `canOutputRecipe` / `canAddToSlot`),
- `common/src/main/java/rearth/oritech/init/BlockContent.java:135` (`FRAGMENT_FORGE_BLOCK`),
- the jar's `data/oritech/recipe/grinder/**` (all 114 files), `recipe/nickel_ingot_from_smelting_raw_nickel.json`,
  `assets/oritech/lang/en_us.json` and the block/item assets named in `CONTENT.md`.

The brief's `UsageConfig`/`StorageConfig` names are **not** classes in v1.2.13: the four numbers live
in the reusable `OritechConfig.BasicMachineConfig` (capacity / insertion / extraction / energyPerTick),
which is what `fragmentForgeData` instantiates. This port reads those numbers from there, not from
the class names in the brief.

| Fact | Upstream | This port |
|---|---|---|
| capacity / insert / extract / FE per tick | 50,000 / 2,048 (`256*8`) / 0 / 256 | `GRINDER_*` |
| slots | `InventorySlotAssignment(0, 1, 1, 3)`: 1 in, 3 out, inventory 4 | container 4/1/3 |
| seven cores | (0,1,0), (0,0,1), (0,1,1), (1,0,1), (1,1,1), (1,0,0), (1,1,0) | `GRINDER_CORE_OFFSETS` |
| five addon slots, yield addon doubles byproducts (index > 0) | `getAdditionalStatFromAddon` / `getCraftingResults` | not ported: no yield addon; byproducts are the recipe's own results |
| `combineSmallDusts` on craft completion | `FragmentForgeBlockEntity.craftItem` | not ported: no small-dust family here |
| `grinder/ore/iron.json` | `#minecraft:iron_ores` → 2 raw iron + 1 `oritech:raw_nickel`, `time: 40` | carried; 40 × 256 = 10,240 FE |
| nickel smelting | `minecraft:smelting`, `cookingtime: 200` | furnace at 0.5 speed: 100 ticks, 3,200 FE |

Ported: the block, its seven-core assembly, the `oritech:grinder` recipe type and the 17 files whose
result and every ingredient resolve here, the one-input/three-output container, the 256 FE/t tick,
the exact consume-then-grant rule and the readout. Not ported: the yield addon and its doubled
byproducts, the five addon slots (the port's addon scan tops out at three), `combineSmallDusts`, the
machine's screen/GeckoLib model, and the 97 recipes that need items, fluids or `c:` tags this
package does not have. `notes.md` → *Slice 11* has the per-family closure table.


## Slice 12: the Atomic Forge (`oritech:atomic_forge_block`)

Read for behaviour only at the same tag **`v1.2.13`** / commit
`4069b6dd14b05dfe0c80126618739b64054630ea`. The two files the brief named as authority:

- `common/src/main/java/rearth/oritech/init/OritechConfig.java` — `ProcessingMachines`:
  `atomicForgeData = new BasicMachineConfig(b, "atomicForgeData", 1024, 0, 0, 1024)`, i.e. capacity
  **1,024 FE**, max insert **0**, max extract **0**, `energyPerTick` **1,024**. (The brief's
  `UsageConfig` / `StorageConfig` names are not classes in this version; `BasicMachineConfig` is.)
- `common/src/main/java/rearth/oritech/block/entity/processing/AtomicForgeBlockEntity.java` —
  the charge gate and the shape:
  - `getRecipe()`: when a recipe matches, `energyStorage.setCapacity(energyPerTick × recipe.time)`;
    without one, capacity 1 and amount 0. For `wafer.json` (`time: 5`) the active capacity is
    **5,120 FE**.
  - `hasEnoughEnergy()`: `capacity > 10 && amount >= capacity` — the buffer must be **full**, not
    merely hold a tick's cost. `canProceed()` adds `super.canProceed()` (output room).
  - `useEnergy()`: `amount = 0` — a craft spends the whole buffer.
  - `checkCraftingFinished()`: `progress > 0` — one working tick completes the operation.
  - `getSlotAssignments()`: `InventorySlotAssignment(0, 3, 3, 1)` — 3 inputs, 1 output, 4 slots.
  - `getCorePositions()`: eight offsets in a ring at y = 0 — (1,0,1), (1,0,0), (1,0,-1), (0,0,1),
    (0,0,-1), (-1,0,1), (-1,0,0), (-1,0,-1). Eight positions but only seven `machine_core_N` types.
  - `getAddonSlots()`: empty — this machine has no addon positions upstream.
  - `getOwnRecipeType()`: `RecipeContent.ATOMIC_FORGE` (`oritech:atomic_forge`).
- the jar's `data/oritech/recipe/atomicforge/**` (all 19 files) and
  `data/oritech/recipe/crafting/atomicforge.json` (the block's own recipe: plastic, enderic compound,
  flux gate, duratium — none carried).

Only `wafer.json` is resolvable here; the other 18 are `not ported(inert family)`, one reason per
family in `notes.md` → *Slice 12*. The wafer's only downstream consumer is `advcomputer.json`, whose
`processing_unit` needs `#c:plates/plastic` — the blocked refinery/oil chain. No use is invented.
